use actix_web::HttpRequest;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::{ProxyError, Result};
use crate::tls::ProxyHeaders;

/// Security audit event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    /// Authentication attempt
    AuthenticationAttempt,
    /// Authentication failure
    AuthenticationFailure,
    /// Authorization failure
    AuthorizationFailure,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// DDoS attack detected
    DdosDetected,
    /// Suspicious activity detected
    SuspiciousActivity,
    /// TLS handshake failure
    TlsHandshakeFailure,
    /// Certificate validation failure
    CertificateValidationFailure,
    /// Security header violation
    SecurityHeaderViolation,
    /// Proxy header manipulation detected
    ProxyHeaderManipulation,
    /// Unusual request pattern
    UnusualRequestPattern,
    /// Security policy violation
    SecurityPolicyViolation,
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Event ID
    pub id: String,
    /// Event type
    pub event_type: SecurityEventType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Client IP address
    pub client_ip: Option<IpAddr>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request path
    pub request_path: Option<String>,
    /// HTTP method
    pub http_method: Option<String>,
    /// Event severity
    pub severity: SecuritySeverity,
    /// Event message
    pub message: String,
    /// Additional event data
    pub data: Value,
    /// Request headers (filtered for security)
    pub headers: HashMap<String, String>,
    /// Geographic information (if available)
    pub geo_info: Option<GeoInfo>,
}

/// Security event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SecuritySeverity {
    /// Low severity - informational
    Low,
    /// Medium severity - warning
    Medium,
    /// High severity - error
    High,
    /// Critical severity - immediate attention required
    Critical,
}

/// Geographic information for security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoInfo {
    /// Country code
    pub country: Option<String>,
    /// City
    pub city: Option<String>,
    /// ISP/Organization
    pub organization: Option<String>,
}

/// Security audit configuration
#[derive(Debug, Clone)]
pub struct SecurityAuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Log authentication events
    pub log_auth_events: bool,
    /// Log rate limiting events
    pub log_rate_limit_events: bool,
    /// Log DDoS events
    pub log_ddos_events: bool,
    /// Log TLS events
    pub log_tls_events: bool,
    /// Log suspicious activity
    pub log_suspicious_activity: bool,
    /// Maximum events to keep in memory
    pub max_events_in_memory: usize,
    /// Event retention period in days
    pub retention_days: u32,
    /// Enable real-time alerting
    pub enable_alerting: bool,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

/// Alert thresholds for security events
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// Failed authentication attempts per minute
    pub failed_auth_per_minute: u32,
    /// Rate limit violations per minute
    pub rate_limit_violations_per_minute: u32,
    /// DDoS events per hour
    pub ddos_events_per_hour: u32,
    /// Suspicious activities per hour
    pub suspicious_activities_per_hour: u32,
}

impl Default for SecurityAuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_auth_events: true,
            log_rate_limit_events: true,
            log_ddos_events: true,
            log_tls_events: true,
            log_suspicious_activity: true,
            max_events_in_memory: 10000,
            retention_days: 90,
            enable_alerting: true,
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            failed_auth_per_minute: 10,
            rate_limit_violations_per_minute: 50,
            ddos_events_per_hour: 5,
            suspicious_activities_per_hour: 20,
        }
    }
}

/// Security audit logger
pub struct SecurityAuditLogger {
    config: SecurityAuditConfig,
    /// In-memory event storage
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    /// Event statistics
    stats: Arc<RwLock<SecurityAuditStats>>,
}

/// Security audit statistics
#[derive(Debug, Clone)]
pub struct SecurityAuditStats {
    /// Total events logged
    pub total_events: u64,
    /// Events by type
    pub events_by_type: HashMap<String, u64>,
    /// Events by severity
    pub events_by_severity: HashMap<String, u64>,
    /// Recent event rate (events per minute)
    pub recent_event_rate: f64,
    /// Last event timestamp
    pub last_event_time: Option<DateTime<Utc>>,
}

impl SecurityAuditLogger {
    /// Create a new security audit logger
    pub fn new(config: SecurityAuditConfig) -> Self {
        Self {
            config,
            events: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(SecurityAuditStats::default())),
        }
    }
    
    /// Log a security event
    pub fn log_event(&self, event: SecurityEvent) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // Log to tracing based on severity
        match event.severity {
            SecuritySeverity::Low => info!("Security Event: {}", serde_json::to_string(&event).unwrap_or_default()),
            SecuritySeverity::Medium => warn!("Security Event: {}", serde_json::to_string(&event).unwrap_or_default()),
            SecuritySeverity::High => error!("Security Event: {}", serde_json::to_string(&event).unwrap_or_default()),
            SecuritySeverity::Critical => error!("CRITICAL Security Event: {}", serde_json::to_string(&event).unwrap_or_default()),
        }
        
        // Store in memory
        {
            let mut events = self.events.write()
                .map_err(|e| ProxyError::config(format!("Failed to acquire events lock: {}", e)))?;
            
            events.push(event.clone());
            
            // Maintain maximum events in memory
            if events.len() > self.config.max_events_in_memory {
                events.remove(0);
            }
        }
        
        // Update statistics
        self.update_stats(&event)?;
        
        // Check alert thresholds
        if self.config.enable_alerting {
            self.check_alert_thresholds(&event)?;
        }
        
        Ok(())
    }
    
    /// Log authentication attempt
    pub fn log_auth_attempt(&self, req: &HttpRequest, success: bool, user_id: Option<&str>) -> Result<()> {
        if !self.config.log_auth_events {
            return Ok(());
        }
        
        let event_type = if success {
            SecurityEventType::AuthenticationAttempt
        } else {
            SecurityEventType::AuthenticationFailure
        };
        
        let severity = if success {
            SecuritySeverity::Low
        } else {
            SecuritySeverity::Medium
        };
        
        let event = self.create_event_from_request(
            event_type,
            severity,
            if success { "Authentication successful" } else { "Authentication failed" },
            req,
            json!({
                "success": success,
                "user_id": user_id,
                "auth_method": "api_key" // TODO: Make this configurable
            }),
        )?;
        
        self.log_event(event)
    }
    
    /// Log rate limit exceeded
    pub fn log_rate_limit_exceeded(&self, req: &HttpRequest, limit_type: &str) -> Result<()> {
        if !self.config.log_rate_limit_events {
            return Ok(());
        }
        
        let event = self.create_event_from_request(
            SecurityEventType::RateLimitExceeded,
            SecuritySeverity::Medium,
            &format!("Rate limit exceeded: {}", limit_type),
            req,
            json!({
                "limit_type": limit_type,
                "endpoint": req.path()
            }),
        )?;
        
        self.log_event(event)
    }
    
    /// Log DDoS detection
    pub fn log_ddos_detected(&self, req: &HttpRequest, requests_per_second: u32) -> Result<()> {
        if !self.config.log_ddos_events {
            return Ok(());
        }
        
        let event = self.create_event_from_request(
            SecurityEventType::DdosDetected,
            SecuritySeverity::High,
            &format!("DDoS attack detected: {} requests/second", requests_per_second),
            req,
            json!({
                "requests_per_second": requests_per_second,
                "detection_time": Utc::now()
            }),
        )?;
        
        self.log_event(event)
    }
    
    /// Log suspicious activity
    pub fn log_suspicious_activity(&self, req: &HttpRequest, activity_type: &str, details: Value) -> Result<()> {
        if !self.config.log_suspicious_activity {
            return Ok(());
        }
        
        let event = self.create_event_from_request(
            SecurityEventType::SuspiciousActivity,
            SecuritySeverity::Medium,
            &format!("Suspicious activity detected: {}", activity_type),
            req,
            json!({
                "activity_type": activity_type,
                "details": details
            }),
        )?;
        
        self.log_event(event)
    }
    
    /// Log TLS handshake failure
    pub fn log_tls_failure(&self, client_ip: Option<IpAddr>, error: &str) -> Result<()> {
        if !self.config.log_tls_events {
            return Ok(());
        }
        
        let event = SecurityEvent {
            id: Uuid::new_v4().to_string(),
            event_type: SecurityEventType::TlsHandshakeFailure,
            timestamp: Utc::now(),
            client_ip,
            user_agent: None,
            request_path: None,
            http_method: None,
            severity: SecuritySeverity::Medium,
            message: format!("TLS handshake failure: {}", error),
            data: json!({
                "error": error,
                "tls_version": "unknown"
            }),
            headers: HashMap::new(),
            geo_info: None,
        };
        
        self.log_event(event)
    }
    
    /// Create security event from HTTP request
    fn create_event_from_request(
        &self,
        event_type: SecurityEventType,
        severity: SecuritySeverity,
        message: &str,
        req: &HttpRequest,
        data: Value,
    ) -> Result<SecurityEvent> {
        let proxy_headers = ProxyHeaders::from_request(req);
        let client_ip = proxy_headers.client_ip.or_else(|| {
            req.connection_info().peer_addr()
                .and_then(|addr| IpAddr::from_str(addr).ok())
        });
        
        let user_agent = req.headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        // Filter sensitive headers
        let mut filtered_headers = HashMap::new();
        for (name, value) in req.headers() {
            let name_str = name.as_str().to_lowercase();
            if !name_str.contains("authorization") && !name_str.contains("cookie") && !name_str.contains("token") {
                if let Ok(value_str) = value.to_str() {
                    filtered_headers.insert(name.as_str().to_string(), value_str.to_string());
                }
            }
        }
        
        Ok(SecurityEvent {
            id: Uuid::new_v4().to_string(),
            event_type,
            timestamp: Utc::now(),
            client_ip,
            user_agent,
            request_path: Some(req.path().to_string()),
            http_method: Some(req.method().to_string()),
            severity,
            message: message.to_string(),
            data,
            headers: filtered_headers,
            geo_info: None, // TODO: Implement GeoIP lookup
        })
    }
    
    /// Update statistics
    fn update_stats(&self, event: &SecurityEvent) -> Result<()> {
        let mut stats = self.stats.write()
            .map_err(|e| ProxyError::config(format!("Failed to acquire stats lock: {}", e)))?;
        
        stats.total_events += 1;
        
        // Update events by type
        let type_key = format!("{:?}", event.event_type);
        *stats.events_by_type.entry(type_key).or_insert(0) += 1;
        
        // Update events by severity
        let severity_key = format!("{:?}", event.severity);
        *stats.events_by_severity.entry(severity_key).or_insert(0) += 1;
        
        stats.last_event_time = Some(event.timestamp);
        
        Ok(())
    }
    
    /// Check alert thresholds
    fn check_alert_thresholds(&self, event: &SecurityEvent) -> Result<()> {
        // TODO: Implement threshold checking logic
        // This would typically involve:
        // 1. Counting recent events of specific types
        // 2. Comparing against thresholds
        // 3. Triggering alerts if thresholds are exceeded
        
        match event.event_type {
            SecurityEventType::AuthenticationFailure => {
                // Check failed auth threshold
            }
            SecurityEventType::RateLimitExceeded => {
                // Check rate limit violation threshold
            }
            SecurityEventType::DdosDetected => {
                // Check DDoS event threshold
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Get recent events
    pub fn get_recent_events(&self, limit: usize) -> Result<Vec<SecurityEvent>> {
        let events = self.events.read()
            .map_err(|e| ProxyError::config(format!("Failed to acquire events lock: {}", e)))?;
        
        let start_index = if events.len() > limit {
            events.len() - limit
        } else {
            0
        };
        
        Ok(events[start_index..].to_vec())
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> Result<SecurityAuditStats> {
        let stats = self.stats.read()
            .map_err(|e| ProxyError::config(format!("Failed to acquire stats lock: {}", e)))?;
        Ok(stats.clone())
    }
    
    /// Clear old events (based on retention policy)
    pub fn cleanup_old_events(&self) -> Result<()> {
        let cutoff_time = Utc::now() - chrono::Duration::days(self.config.retention_days as i64);
        
        let mut events = self.events.write()
            .map_err(|e| ProxyError::config(format!("Failed to acquire events lock: {}", e)))?;
        
        events.retain(|event| event.timestamp > cutoff_time);
        
        Ok(())
    }
}

impl Default for SecurityAuditStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            events_by_type: HashMap::new(),
            events_by_severity: HashMap::new(),
            recent_event_rate: 0.0,
            last_event_time: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    
    #[test]
    fn test_security_event_creation() {
        let config = SecurityAuditConfig::default();
        let logger = SecurityAuditLogger::new(config);
        
        let req = TestRequest::default()
            .insert_header(("user-agent", "test-agent"))
            .to_http_request();
        
        let event = logger.create_event_from_request(
            SecurityEventType::AuthenticationFailure,
            SecuritySeverity::Medium,
            "Test event",
            &req,
            json!({"test": "data"}),
        ).unwrap();
        
        assert_eq!(event.event_type, SecurityEventType::AuthenticationFailure);
        assert_eq!(event.severity, SecuritySeverity::Medium);
        assert_eq!(event.message, "Test event");
    }
    
    #[test]
    fn test_event_logging() {
        let config = SecurityAuditConfig::default();
        let logger = SecurityAuditLogger::new(config);
        
        let req = TestRequest::default().to_http_request();
        
        // Test authentication logging
        assert!(logger.log_auth_attempt(&req, false, Some("test_user")).is_ok());
        
        // Test rate limit logging
        assert!(logger.log_rate_limit_exceeded(&req, "per_ip").is_ok());
        
        // Test DDoS logging
        assert!(logger.log_ddos_detected(&req, 150).is_ok());
        
        // Check that events were logged
        let events = logger.get_recent_events(10).unwrap();
        assert_eq!(events.len(), 3);
    }
}
