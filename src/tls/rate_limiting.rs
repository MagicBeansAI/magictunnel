use actix_web::{HttpRequest, HttpResponse};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error as ActixError;
use futures_util::future::{ok, Ready};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tracing::{debug, warn, info};

use crate::error::{ProxyError, Result};
use crate::tls::ProxyHeaders;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Global rate limit (requests per minute)
    pub global_limit: u32,
    /// Per-IP rate limit (requests per minute)
    pub per_ip_limit: u32,
    /// Per-endpoint rate limits
    pub endpoint_limits: HashMap<String, u32>,
    /// Burst allowance (requests that can exceed the rate limit temporarily)
    pub burst_allowance: u32,
    /// Time window for rate limiting (in seconds)
    pub window_seconds: u64,
    /// Enable DDoS protection
    pub ddos_protection: bool,
    /// DDoS threshold (requests per second to trigger protection)
    pub ddos_threshold: u32,
    /// Whitelist of IPs that bypass rate limiting
    pub whitelist: Vec<String>,
    /// Enable adaptive rate limiting
    pub adaptive_limiting: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut endpoint_limits = HashMap::new();
        endpoint_limits.insert("/health".to_string(), 60); // 60 requests per minute
        endpoint_limits.insert("/mcp/tools".to_string(), 30); // 30 requests per minute
        endpoint_limits.insert("/mcp/call".to_string(), 20); // 20 requests per minute
        endpoint_limits.insert("/mcp/call/stream".to_string(), 10); // 10 requests per minute
        
        Self {
            global_limit: 1000, // 1000 requests per minute globally
            per_ip_limit: 100,  // 100 requests per minute per IP
            endpoint_limits,
            burst_allowance: 10,
            window_seconds: 60,
            ddos_protection: true,
            ddos_threshold: 100, // 100 requests per second
            whitelist: vec![
                "127.0.0.1".to_string(),
                "::1".to_string(),
            ],
            adaptive_limiting: true,
        }
    }
}

/// Rate limiting statistics
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    /// Total requests processed
    pub total_requests: u64,
    /// Requests blocked by rate limiting
    pub blocked_requests: u64,
    /// Current active IPs
    pub active_ips: u32,
    /// DDoS events detected
    pub ddos_events: u32,
    /// Last reset time
    pub last_reset: Instant,
}

/// Rate limiter implementation
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    /// Global request counter
    global_counter: Arc<RwLock<RequestCounter>>,
    /// Per-IP request counters
    ip_counters: Arc<RwLock<HashMap<IpAddr, RequestCounter>>>,
    /// Per-endpoint request counters
    endpoint_counters: Arc<RwLock<HashMap<String, RequestCounter>>>,
    /// DDoS detection state
    ddos_state: Arc<RwLock<DdosState>>,
    /// Rate limiting statistics
    stats: Arc<RwLock<RateLimitStats>>,
}

/// Request counter for rate limiting
#[derive(Debug, Clone)]
struct RequestCounter {
    /// Number of requests in current window
    count: u32,
    /// Window start time
    window_start: Instant,
    /// Burst tokens available
    burst_tokens: u32,
    /// Last request time
    last_request: Instant,
}

/// DDoS detection state
#[derive(Debug)]
struct DdosState {
    /// Recent request timestamps for DDoS detection
    recent_requests: Vec<Instant>,
    /// Whether DDoS protection is currently active
    protection_active: bool,
    /// When protection was activated
    protection_start: Option<Instant>,
    /// Number of DDoS events detected
    event_count: u32,
}

impl RequestCounter {
    fn new(burst_allowance: u32) -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
            burst_tokens: burst_allowance,
            last_request: Instant::now(),
        }
    }
    
    fn reset_if_needed(&mut self, window_duration: Duration, burst_allowance: u32) {
        let now = Instant::now();
        if now.duration_since(self.window_start) >= window_duration {
            self.count = 0;
            self.window_start = now;
            self.burst_tokens = burst_allowance;
        }
    }
    
    fn can_proceed(&mut self, limit: u32, window_duration: Duration, burst_allowance: u32) -> bool {
        self.reset_if_needed(window_duration, burst_allowance);
        
        let now = Instant::now();
        self.last_request = now;
        
        // Check if within normal limit
        if self.count < limit {
            self.count += 1;
            return true;
        }
        
        // Check if burst tokens are available
        if self.burst_tokens > 0 {
            self.burst_tokens -= 1;
            self.count += 1;
            return true;
        }
        
        false
    }
}

impl DdosState {
    fn new() -> Self {
        Self {
            recent_requests: Vec::new(),
            protection_active: false,
            protection_start: None,
            event_count: 0,
        }
    }
    
    fn check_ddos(&mut self, threshold: u32) -> bool {
        let now = Instant::now();
        let one_second_ago = now - Duration::from_secs(1);
        
        // Remove old requests (older than 1 second)
        self.recent_requests.retain(|&time| time > one_second_ago);
        
        // Add current request
        self.recent_requests.push(now);
        
        // Check if threshold is exceeded
        if self.recent_requests.len() as u32 > threshold {
            if !self.protection_active {
                self.protection_active = true;
                self.protection_start = Some(now);
                self.event_count += 1;
                warn!("DDoS protection activated - {} requests in last second", self.recent_requests.len());
            }
            return true;
        }
        
        // Deactivate protection if requests drop below threshold
        if self.protection_active && self.recent_requests.len() as u32 <= threshold / 2 {
            self.protection_active = false;
            self.protection_start = None;
            info!("DDoS protection deactivated");
        }
        
        false
    }
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let burst_allowance = config.burst_allowance;
        Self {
            config,
            global_counter: Arc::new(RwLock::new(RequestCounter::new(burst_allowance))),
            ip_counters: Arc::new(RwLock::new(HashMap::new())),
            endpoint_counters: Arc::new(RwLock::new(HashMap::new())),
            ddos_state: Arc::new(RwLock::new(DdosState::new())),
            stats: Arc::new(RwLock::new(RateLimitStats {
                total_requests: 0,
                blocked_requests: 0,
                active_ips: 0,
                ddos_events: 0,
                last_reset: Instant::now(),
            })),
        }
    }
    
    /// Check if request should be allowed
    pub fn check_request(&self, req: &HttpRequest) -> Result<bool> {
        let client_ip = self.get_client_ip(req);
        let endpoint = req.path().to_string();
        let window_duration = Duration::from_secs(self.config.window_seconds);
        
        // Update stats
        {
            let mut stats = self.stats.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire stats lock: {}", e)))?;
            stats.total_requests += 1;
        }
        
        // Check whitelist
        if let Some(ip) = client_ip {
            if self.is_whitelisted(&ip) {
                debug!("Request from whitelisted IP: {}", ip);
                return Ok(true);
            }
        }
        
        // Check DDoS protection
        if self.config.ddos_protection {
            let mut ddos_state = self.ddos_state.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire DDoS state lock: {}", e)))?;
            
            if ddos_state.check_ddos(self.config.ddos_threshold) {
                self.increment_blocked_stats()?;
                return Ok(false);
            }
        }
        
        // Check global limit
        {
            let mut global_counter = self.global_counter.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire global counter lock: {}", e)))?;
            
            if !global_counter.can_proceed(self.config.global_limit, window_duration, self.config.burst_allowance) {
                debug!("Request blocked by global rate limit");
                self.increment_blocked_stats()?;
                return Ok(false);
            }
        }
        
        // Check per-IP limit
        if let Some(ip) = client_ip {
            let mut ip_counters = self.ip_counters.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire IP counters lock: {}", e)))?;
            
            let counter = ip_counters.entry(ip).or_insert_with(|| RequestCounter::new(self.config.burst_allowance));
            
            if !counter.can_proceed(self.config.per_ip_limit, window_duration, self.config.burst_allowance) {
                debug!("Request blocked by per-IP rate limit for {}", ip);
                self.increment_blocked_stats()?;
                return Ok(false);
            }
        }
        
        // Check endpoint-specific limit
        if let Some(&endpoint_limit) = self.config.endpoint_limits.get(&endpoint) {
            let mut endpoint_counters = self.endpoint_counters.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire endpoint counters lock: {}", e)))?;
            
            let counter = endpoint_counters.entry(endpoint.clone()).or_insert_with(|| RequestCounter::new(self.config.burst_allowance));
            
            if !counter.can_proceed(endpoint_limit, window_duration, self.config.burst_allowance) {
                debug!("Request blocked by endpoint rate limit for {}", endpoint);
                self.increment_blocked_stats()?;
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Get client IP address from request
    fn get_client_ip(&self, req: &HttpRequest) -> Option<IpAddr> {
        // Try to get real client IP from proxy headers
        let proxy_headers = ProxyHeaders::from_request(req);
        if let Some(client_ip) = proxy_headers.client_ip {
            return Some(client_ip);
        }
        
        // Fall back to connection info
        req.connection_info().peer_addr()
            .and_then(|addr| IpAddr::from_str(addr).ok())
    }
    
    /// Check if IP is whitelisted
    fn is_whitelisted(&self, ip: &IpAddr) -> bool {
        self.config.whitelist.iter().any(|whitelisted| {
            if let Ok(whitelisted_ip) = IpAddr::from_str(whitelisted) {
                *ip == whitelisted_ip
            } else {
                false
            }
        })
    }
    
    /// Increment blocked request statistics
    fn increment_blocked_stats(&self) -> Result<()> {
        let mut stats = self.stats.write()
            .map_err(|e| ProxyError::config(format!("Failed to acquire stats lock: {}", e)))?;
        stats.blocked_requests += 1;
        Ok(())
    }
    
    /// Get rate limiting statistics
    pub fn get_stats(&self) -> Result<RateLimitStats> {
        let stats = self.stats.read()
            .map_err(|e| ProxyError::config(format!("Failed to acquire stats lock: {}", e)))?;
        
        let mut stats_clone = stats.clone();
        
        // Update active IPs count
        if let Ok(ip_counters) = self.ip_counters.read() {
            stats_clone.active_ips = ip_counters.len() as u32;
        }
        
        // Update DDoS events count
        if let Ok(ddos_state) = self.ddos_state.read() {
            stats_clone.ddos_events = ddos_state.event_count;
        }
        
        Ok(stats_clone)
    }
    
    /// Reset rate limiting counters
    pub fn reset(&self) -> Result<()> {
        {
            let mut global_counter = self.global_counter.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire global counter lock: {}", e)))?;
            *global_counter = RequestCounter::new(self.config.burst_allowance);
        }
        
        {
            let mut ip_counters = self.ip_counters.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire IP counters lock: {}", e)))?;
            ip_counters.clear();
        }
        
        {
            let mut endpoint_counters = self.endpoint_counters.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire endpoint counters lock: {}", e)))?;
            endpoint_counters.clear();
        }
        
        {
            let mut ddos_state = self.ddos_state.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire DDoS state lock: {}", e)))?;
            *ddos_state = DdosState::new();
        }
        
        {
            let mut stats = self.stats.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire stats lock: {}", e)))?;
            stats.total_requests = 0;
            stats.blocked_requests = 0;
            stats.active_ips = 0;
            stats.ddos_events = 0;
            stats.last_reset = Instant::now();
        }
        
        info!("Rate limiting counters reset");
        Ok(())
    }
}

/// Rate limiting middleware
pub struct RateLimitMiddleware {
    rate_limiter: Arc<RateLimiter>,
}

impl RateLimitMiddleware {
    /// Create new rate limiting middleware
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            rate_limiter: Arc::new(RateLimiter::new(config)),
        }
    }
    
    /// Get rate limiter instance
    pub fn rate_limiter(&self) -> &Arc<RateLimiter> {
        &self.rate_limiter
    }
}

/// Transform implementation for Actix Web middleware
impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type InitError = ();
    type Transform = RateLimitService<S>;
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RateLimitService {
            service,
            rate_limiter: self.rate_limiter.clone(),
        })
    }
}

/// Rate limiting service
pub struct RateLimitService<S> {
    service: S,
    rate_limiter: Arc<RateLimiter>,
}

impl<S, B> Service<ServiceRequest> for RateLimitService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Future = futures_util::future::LocalBoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // For now, just pass through all requests without rate limiting
        // TODO: Implement proper rate limiting middleware
        let fut = self.service.call(req);
        Box::pin(async move {
            fut.await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    
    #[test]
    fn test_request_counter() {
        let mut counter = RequestCounter::new(5);
        let window = Duration::from_secs(60);
        
        // Should allow requests within limit
        for _ in 0..10 {
            assert!(counter.can_proceed(10, window, 5));
        }
        
        // Should allow burst requests
        for _ in 0..5 {
            assert!(counter.can_proceed(10, window, 5));
        }
        
        // Should block after burst is exhausted
        assert!(!counter.can_proceed(10, window, 5));
    }
    
    #[test]
    fn test_ddos_detection() {
        let mut ddos_state = DdosState::new();
        
        // Simulate rapid requests
        for _ in 0..150 {
            ddos_state.check_ddos(100);
        }
        
        assert!(ddos_state.protection_active);
        assert_eq!(ddos_state.event_count, 1);
    }
    
    #[test]
    fn test_whitelist() {
        let config = RateLimitConfig {
            whitelist: vec!["127.0.0.1".to_string()],
            ..Default::default()
        };
        
        let rate_limiter = RateLimiter::new(config);
        let localhost = IpAddr::from_str("127.0.0.1").unwrap();
        
        assert!(rate_limiter.is_whitelisted(&localhost));
    }
}
