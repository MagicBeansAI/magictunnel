use actix_web::HttpRequest;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info};

use crate::config::{TlsConfig, TlsMode};
use crate::error::{ProxyError, Result};
use super::{ProxyHeaders, TrustedProxyValidator};

/// Auto-detection engine for determining TLS mode based on request patterns
#[derive(Debug)]
pub struct TlsAutoDetector {
    /// Current detected mode
    detected_mode: Arc<RwLock<TlsMode>>,
    /// Configuration for auto-detection
    config: AutoDetectionConfig,
    /// Statistics for detection decisions
    stats: Arc<RwLock<DetectionStats>>,
    /// Trusted proxy validator
    trusted_proxy_validator: Option<TrustedProxyValidator>,
}

/// Configuration for auto-detection behavior
#[derive(Debug, Clone)]
pub struct AutoDetectionConfig {
    /// Minimum number of requests before making a detection decision
    pub min_requests: u32,
    /// Percentage of requests that must have proxy headers to detect proxy mode
    pub proxy_threshold: f64,
    /// Time window for collecting detection samples
    pub detection_window: Duration,
    /// Whether to enable adaptive detection (changes mode based on patterns)
    pub adaptive_detection: bool,
    /// Headers to check for proxy detection
    pub detection_headers: Vec<String>,
    /// Fallback mode when detection is inconclusive
    pub fallback_mode: TlsMode,
}

/// Statistics for auto-detection decisions
#[derive(Debug, Clone)]
pub struct DetectionStats {
    /// Total requests analyzed
    pub total_requests: u64,
    /// Requests with proxy headers
    pub proxy_requests: u64,
    /// Requests without proxy headers
    pub direct_requests: u64,
    /// Last detection decision time
    pub last_detection: Option<Instant>,
    /// Detection confidence (0.0 to 1.0)
    pub confidence: f64,
    /// History of recent requests for pattern analysis
    pub recent_requests: Vec<RequestSample>,
}

/// Sample of a request for pattern analysis
#[derive(Debug, Clone)]
pub struct RequestSample {
    /// Timestamp of the request
    pub timestamp: Instant,
    /// Whether the request had proxy headers
    pub has_proxy_headers: bool,
    /// Client IP (if available)
    pub client_ip: Option<std::net::IpAddr>,
    /// User agent (for pattern detection)
    pub user_agent: Option<String>,
}

impl Default for AutoDetectionConfig {
    fn default() -> Self {
        Self {
            min_requests: 10,
            proxy_threshold: 0.8, // 80% of requests must have proxy headers
            detection_window: Duration::from_secs(300), // 5 minutes
            adaptive_detection: true,
            detection_headers: vec![
                "X-Forwarded-Proto".to_string(),
                "X-Forwarded-For".to_string(),
                "X-Real-IP".to_string(),
                "X-Forwarded-Host".to_string(),
            ],
            fallback_mode: TlsMode::Application,
        }
    }
}

impl Default for DetectionStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            proxy_requests: 0,
            direct_requests: 0,
            last_detection: None,
            confidence: 0.0,
            recent_requests: Vec::new(),
        }
    }
}

impl TlsAutoDetector {
    /// Create a new auto-detector
    pub fn new(tls_config: &TlsConfig) -> Result<Self> {
        let config = AutoDetectionConfig {
            detection_headers: tls_config.auto_detect_headers.clone(),
            fallback_mode: tls_config.fallback_mode.clone(),
            ..Default::default()
        };
        
        let trusted_proxy_validator = if !tls_config.trusted_proxies.is_empty() {
            Some(TrustedProxyValidator::new(&tls_config.trusted_proxies)?)
        } else {
            None
        };
        
        Ok(Self {
            detected_mode: Arc::new(RwLock::new(config.fallback_mode.clone())),
            config,
            stats: Arc::new(RwLock::new(DetectionStats::default())),
            trusted_proxy_validator,
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(tls_config: &TlsConfig, detection_config: AutoDetectionConfig) -> Result<Self> {
        let trusted_proxy_validator = if !tls_config.trusted_proxies.is_empty() {
            Some(TrustedProxyValidator::new(&tls_config.trusted_proxies)?)
        } else {
            None
        };
        
        Ok(Self {
            detected_mode: Arc::new(RwLock::new(detection_config.fallback_mode.clone())),
            config: detection_config,
            stats: Arc::new(RwLock::new(DetectionStats::default())),
            trusted_proxy_validator,
        })
    }
    
    /// Analyze a request and update detection state
    pub fn analyze_request(&self, req: &HttpRequest) -> Result<TlsMode> {
        let proxy_headers = ProxyHeaders::from_request(req);
        let has_proxy_headers = proxy_headers.is_proxied;
        
        // Create request sample
        let sample = RequestSample {
            timestamp: Instant::now(),
            has_proxy_headers,
            client_ip: proxy_headers.client_ip,
            user_agent: req.headers()
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
        };
        
        // Update statistics
        self.update_stats(sample)?;
        
        // Perform detection if we have enough data
        if self.should_perform_detection()? {
            self.perform_detection()?;
        }
        
        // Return current detected mode
        let detected_mode = self.detected_mode.read()
            .map_err(|e| ProxyError::config(format!("Failed to read detected mode: {}", e)))?
            .clone();
        
        debug!(
            "Auto-detection result: mode={:?}, has_proxy_headers={}, confidence={}",
            detected_mode,
            has_proxy_headers,
            self.get_confidence()?
        );
        
        Ok(detected_mode)
    }
    
    /// Get the current detected mode
    pub fn get_detected_mode(&self) -> Result<TlsMode> {
        self.detected_mode.read()
            .map_err(|e| ProxyError::config(format!("Failed to read detected mode: {}", e)))
            .map(|mode| mode.clone())
    }
    
    /// Get detection statistics
    pub fn get_stats(&self) -> Result<DetectionStats> {
        self.stats.read()
            .map_err(|e| ProxyError::config(format!("Failed to read stats: {}", e)))
            .map(|stats| stats.clone())
    }
    
    /// Get detection confidence (0.0 to 1.0)
    pub fn get_confidence(&self) -> Result<f64> {
        Ok(self.get_stats()?.confidence)
    }
    
    /// Force a detection decision
    pub fn force_detection(&self) -> Result<TlsMode> {
        self.perform_detection()?;
        self.get_detected_mode()
    }
    
    /// Reset detection state
    pub fn reset(&self) -> Result<()> {
        let mut stats = self.stats.write()
            .map_err(|e| ProxyError::config(format!("Failed to write stats: {}", e)))?;
        *stats = DetectionStats::default();
        
        let mut detected_mode = self.detected_mode.write()
            .map_err(|e| ProxyError::config(format!("Failed to write detected mode: {}", e)))?;
        *detected_mode = self.config.fallback_mode.clone();
        
        info!("Auto-detection state reset to fallback mode: {:?}", self.config.fallback_mode);
        Ok(())
    }
    
    /// Update statistics with a new request sample
    fn update_stats(&self, sample: RequestSample) -> Result<()> {
        let mut stats = self.stats.write()
            .map_err(|e| ProxyError::config(format!("Failed to write stats: {}", e)))?;
        
        stats.total_requests += 1;
        if sample.has_proxy_headers {
            stats.proxy_requests += 1;
        } else {
            stats.direct_requests += 1;
        }
        
        // Add to recent requests (keep only within detection window)
        stats.recent_requests.push(sample);
        let cutoff = Instant::now() - self.config.detection_window;
        stats.recent_requests.retain(|req| req.timestamp > cutoff);
        
        Ok(())
    }
    
    /// Check if we should perform detection
    fn should_perform_detection(&self) -> Result<bool> {
        let stats = self.stats.read()
            .map_err(|e| ProxyError::config(format!("Failed to read stats: {}", e)))?;
        
        // Check if we have minimum requests
        if stats.total_requests < self.config.min_requests as u64 {
            return Ok(false);
        }
        
        // Check if adaptive detection is enabled and we haven't detected recently
        if self.config.adaptive_detection {
            if let Some(last_detection) = stats.last_detection {
                let time_since_detection = Instant::now() - last_detection;
                if time_since_detection < self.config.detection_window / 2 {
                    return Ok(false); // Don't detect too frequently
                }
            }
        }
        
        Ok(true)
    }
    
    /// Perform the actual detection logic
    fn perform_detection(&self) -> Result<()> {
        let mut stats = self.stats.write()
            .map_err(|e| ProxyError::config(format!("Failed to write stats: {}", e)))?;
        
        // Calculate proxy ratio from recent requests
        let recent_proxy_count = stats.recent_requests.iter()
            .filter(|req| req.has_proxy_headers)
            .count();
        let recent_total = stats.recent_requests.len();
        
        let proxy_ratio = if recent_total > 0 {
            recent_proxy_count as f64 / recent_total as f64
        } else {
            0.0
        };
        
        // Determine mode based on proxy ratio
        let new_mode = if proxy_ratio >= self.config.proxy_threshold {
            TlsMode::BehindProxy
        } else if proxy_ratio <= (1.0 - self.config.proxy_threshold) {
            TlsMode::Application
        } else {
            // Inconclusive - use fallback
            self.config.fallback_mode.clone()
        };
        
        // Calculate confidence
        let confidence = if proxy_ratio >= self.config.proxy_threshold || proxy_ratio <= (1.0 - self.config.proxy_threshold) {
            (proxy_ratio - 0.5).abs() * 2.0 // Scale to 0.0-1.0
        } else {
            0.5 // Low confidence for inconclusive results
        };
        
        // Update detected mode
        let mut detected_mode = self.detected_mode.write()
            .map_err(|e| ProxyError::config(format!("Failed to write detected mode: {}", e)))?;
        
        let mode_changed = *detected_mode != new_mode;
        *detected_mode = new_mode.clone();
        
        // Update stats
        stats.last_detection = Some(Instant::now());
        stats.confidence = confidence;
        
        if mode_changed {
            info!(
                "Auto-detection mode changed to {:?} (confidence: {:.2}, proxy_ratio: {:.2})",
                new_mode, confidence, proxy_ratio
            );
        } else {
            debug!(
                "Auto-detection confirmed mode {:?} (confidence: {:.2}, proxy_ratio: {:.2})",
                new_mode, confidence, proxy_ratio
            );
        }
        
        Ok(())
    }
}

/// Utility functions for auto-detection
pub struct AutoDetectionUtils;

impl AutoDetectionUtils {
    /// Check if a request appears to be from a reverse proxy
    pub fn appears_proxied(req: &HttpRequest, detection_headers: &[String]) -> bool {
        let headers = req.headers();
        
        for header_name in detection_headers {
            if headers.contains_key(header_name.as_str()) {
                return true;
            }
        }
        
        false
    }
    
    /// Analyze request patterns for proxy detection
    pub fn analyze_request_pattern(req: &HttpRequest) -> RequestPattern {
        let headers = req.headers();
        let proxy_headers = ProxyHeaders::from_request(req);
        
        RequestPattern {
            has_forwarded_headers: proxy_headers.is_proxied,
            has_real_ip: headers.contains_key("x-real-ip"),
            has_forwarded_proto: headers.contains_key("x-forwarded-proto"),
            has_forwarded_for: headers.contains_key("x-forwarded-for"),
            has_forwarded_host: headers.contains_key("x-forwarded-host"),
            connection_info: req.connection_info().clone(),
        }
    }
}

/// Pattern analysis result for a request
#[derive(Debug, Clone)]
pub struct RequestPattern {
    /// Whether any forwarded headers are present
    pub has_forwarded_headers: bool,
    /// Whether X-Real-IP header is present
    pub has_real_ip: bool,
    /// Whether X-Forwarded-Proto header is present
    pub has_forwarded_proto: bool,
    /// Whether X-Forwarded-For header is present
    pub has_forwarded_for: bool,
    /// Whether X-Forwarded-Host header is present
    pub has_forwarded_host: bool,
    /// Connection information
    pub connection_info: actix_web::dev::ConnectionInfo,
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    
    fn create_test_tls_config() -> TlsConfig {
        TlsConfig {
            mode: TlsMode::Auto,
            cert_file: None,
            key_file: None,
            ca_file: None,
            behind_proxy: false,
            trusted_proxies: vec!["127.0.0.1/32".to_string()],
            min_tls_version: "1.2".to_string(),
            cipher_suites: None,
            hsts_enabled: false,
            hsts_max_age: 0,
            hsts_include_subdomains: false,
            hsts_preload: false,
            require_forwarded_proto: false,
            require_forwarded_for: false,
            auto_detect_headers: vec![
                "X-Forwarded-Proto".to_string(),
                "X-Forwarded-For".to_string(),
            ],
            fallback_mode: TlsMode::Application,
        }
    }
    
    #[test]
    fn test_auto_detector_creation() {
        let tls_config = create_test_tls_config();
        let detector = TlsAutoDetector::new(&tls_config).unwrap();
        
        assert_eq!(detector.get_detected_mode().unwrap(), TlsMode::Application);
        assert_eq!(detector.get_confidence().unwrap(), 0.0);
    }
    
    #[test]
    fn test_proxy_detection() {
        let detection_headers = vec!["X-Forwarded-Proto".to_string()];
        
        // Request with proxy headers
        let proxy_req = TestRequest::default()
            .insert_header(("x-forwarded-proto", "https"))
            .to_http_request();
        
        assert!(AutoDetectionUtils::appears_proxied(&proxy_req, &detection_headers));
        
        // Request without proxy headers
        let direct_req = TestRequest::default().to_http_request();
        assert!(!AutoDetectionUtils::appears_proxied(&direct_req, &detection_headers));
    }
}
