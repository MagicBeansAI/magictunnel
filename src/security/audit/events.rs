//! Structured Audit Event Schemas
//! 
//! This module defines consistent event schemas for all audit events across
//! MagicTunnel components. All events follow the same structure with:
//! - Standard metadata (timestamp, component, user, session)
//! - Event-specific payloads with structured data
//! - Consistent serialization for storage and analysis

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Core audit event structure
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    /// Unique event identifier
    pub id: String,
    
    /// Event timestamp (UTC)
    pub timestamp: DateTime<Utc>,
    
    /// Event type classification
    pub event_type: AuditEventType,
    
    /// Component that generated the event
    pub component: String,
    
    /// Human-readable message
    pub message: String,
    
    /// Standard metadata
    pub metadata: AuditMetadata,
    
    /// Event-specific structured payload
    pub payload: Value,
    
    /// Event severity level
    pub severity: AuditSeverity,
    
    /// Optional correlation ID for related events
    pub correlation_id: Option<String>,
}

/// Standard audit event types
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // Authentication & Authorization
    Authentication,
    Authorization,
    TokenRefresh,
    SessionCreated,
    SessionDestroyed,
    
    // OAuth 2.1 Flow Events
    OauthFlow,
    OauthDiscovery,
    OauthRegistration,
    OauthAuthorization,
    OauthTokenExchange,
    OauthTokenUsage,
    OauthForwarded,
    
    // Tool & MCP Events
    ToolExecution,
    McpConnection,
    McpDisconnection,
    SmartDiscovery,
    CapabilityRefresh,
    ResourceAccess,
    
    // Security Events
    SecurityViolation,
    AllowlistCheck,
    RbacCheck,
    Sanitization,
    EmergencyLockdown,
    
    // Administrative Events
    AdminAction,
    ConfigChange,
    ServiceStart,
    ServiceStop,
    
    // System Events
    SystemHealth,
    PerformanceMetric,
    ErrorOccurred,
    
    // Custom events for extensibility
    Custom(String),
}

/// Event severity levels
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

// Custom deserialization for backward compatibility with old PascalCase format
impl<'de> Deserialize<'de> for AuditEventType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        
        // Try current snake_case format first by manual mapping
        match s.as_str() {
            // Current snake_case format
            "authentication" => return Ok(AuditEventType::Authentication),
            "authorization" => return Ok(AuditEventType::Authorization),
            "token_refresh" => return Ok(AuditEventType::TokenRefresh),
            "session_created" => return Ok(AuditEventType::SessionCreated),
            "session_destroyed" => return Ok(AuditEventType::SessionDestroyed),
            "oauth_flow" => return Ok(AuditEventType::OauthFlow),
            "oauth_discovery" => return Ok(AuditEventType::OauthDiscovery),
            "oauth_registration" => return Ok(AuditEventType::OauthRegistration),
            "oauth_authorization" => return Ok(AuditEventType::OauthAuthorization),
            "oauth_token_exchange" => return Ok(AuditEventType::OauthTokenExchange),
            "oauth_token_usage" => return Ok(AuditEventType::OauthTokenUsage),
            "oauth_forwarded" => return Ok(AuditEventType::OauthForwarded),
            "tool_execution" => return Ok(AuditEventType::ToolExecution),
            "mcp_connection" => return Ok(AuditEventType::McpConnection),
            "mcp_disconnection" => return Ok(AuditEventType::McpDisconnection),
            "smart_discovery" => return Ok(AuditEventType::SmartDiscovery),
            "capability_refresh" => return Ok(AuditEventType::CapabilityRefresh),
            "resource_access" => return Ok(AuditEventType::ResourceAccess),
            "security_violation" => return Ok(AuditEventType::SecurityViolation),
            "allowlist_check" => return Ok(AuditEventType::AllowlistCheck),
            "rbac_check" => return Ok(AuditEventType::RbacCheck),
            "sanitization" => return Ok(AuditEventType::Sanitization),
            "emergency_lockdown" => return Ok(AuditEventType::EmergencyLockdown),
            "admin_action" => return Ok(AuditEventType::AdminAction),
            "config_change" => return Ok(AuditEventType::ConfigChange),
            "service_start" => return Ok(AuditEventType::ServiceStart),
            "service_stop" => return Ok(AuditEventType::ServiceStop),
            "system_health" => return Ok(AuditEventType::SystemHealth),
            "performance_metric" => return Ok(AuditEventType::PerformanceMetric),
            "error_occurred" => return Ok(AuditEventType::ErrorOccurred),
            _ => {}
        }
        
        // Handle backward compatibility for old PascalCase format
        match s.as_str() {
            // Authentication & Authorization
            "Authentication" => Ok(AuditEventType::Authentication),
            "Authorization" => Ok(AuditEventType::Authorization), 
            "TokenRefresh" => Ok(AuditEventType::TokenRefresh),
            "SessionCreated" => Ok(AuditEventType::SessionCreated),
            "SessionDestroyed" => Ok(AuditEventType::SessionDestroyed),
            
            // OAuth 2.1 Flow Events  
            "OauthFlow" => Ok(AuditEventType::OauthFlow),
            "OauthDiscovery" => Ok(AuditEventType::OauthDiscovery),
            "OauthRegistration" => Ok(AuditEventType::OauthRegistration),
            "OauthAuthorization" => Ok(AuditEventType::OauthAuthorization),
            "OauthTokenExchange" => Ok(AuditEventType::OauthTokenExchange),
            "OauthTokenUsage" => Ok(AuditEventType::OauthTokenUsage),
            "OauthForwarded" => Ok(AuditEventType::OauthForwarded),
            
            // Tool & MCP Events
            "ToolExecution" => Ok(AuditEventType::ToolExecution),
            "McpConnection" => Ok(AuditEventType::McpConnection),
            "McpDisconnection" => Ok(AuditEventType::McpDisconnection),
            "SmartDiscovery" => Ok(AuditEventType::SmartDiscovery),
            "CapabilityRefresh" => Ok(AuditEventType::CapabilityRefresh),
            "ResourceAccess" => Ok(AuditEventType::ResourceAccess),
            
            // Security Events
            "SecurityViolation" => Ok(AuditEventType::SecurityViolation),
            "AllowlistCheck" => Ok(AuditEventType::AllowlistCheck),
            "RbacCheck" => Ok(AuditEventType::RbacCheck),
            "Sanitization" => Ok(AuditEventType::Sanitization),
            "EmergencyLockdown" => Ok(AuditEventType::EmergencyLockdown),
            
            // Administrative Events
            "AdminAction" => Ok(AuditEventType::AdminAction),
            "ConfigChange" => Ok(AuditEventType::ConfigChange),
            "ServiceStart" => Ok(AuditEventType::ServiceStart),
            "ServiceStop" => Ok(AuditEventType::ServiceStop),
            
            // System Events
            "SystemHealth" => Ok(AuditEventType::SystemHealth),
            "PerformanceMetric" => Ok(AuditEventType::PerformanceMetric),
            "ErrorOccurred" => Ok(AuditEventType::ErrorOccurred),
            
            // Fallback to Custom variant for unknown types
            _ => Ok(AuditEventType::Custom(s)),
        }
    }
}

impl<'de> Deserialize<'de> for AuditSeverity {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        
        // Try current lowercase format first
        match s.as_str() {
            "debug" => return Ok(AuditSeverity::Debug),
            "info" => return Ok(AuditSeverity::Info),
            "warning" => return Ok(AuditSeverity::Warning),
            "error" => return Ok(AuditSeverity::Error),
            "critical" => return Ok(AuditSeverity::Critical),
            _ => {}
        }
        
        // Handle backward compatibility for old PascalCase format
        match s.as_str() {
            "Debug" => Ok(AuditSeverity::Debug),
            "Info" => Ok(AuditSeverity::Info),
            "Warning" => Ok(AuditSeverity::Warning),
            "Error" => Ok(AuditSeverity::Error),
            "Critical" => Ok(AuditSeverity::Critical),
            // Default to Info for unknown severities
            _ => Ok(AuditSeverity::Info),
        }
    }
}

/// Standard metadata present in all audit events
#[derive(Debug, Clone, Serialize)]
pub struct AuditMetadata {
    /// User identifier (if available)
    pub user_id: Option<String>,
    
    /// Session identifier
    pub session_id: Option<String>,
    
    /// Request identifier for tracing
    pub request_id: Option<String>,
    
    /// Source IP address
    pub source_ip: Option<String>,
    
    /// User agent or client information
    pub user_agent: Option<String>,
    
    /// Tenant identifier (for multi-tenant systems)
    pub tenant_id: Option<String>,
    
    /// Environment (dev, staging, prod)
    pub environment: Option<String>,
    
    /// Service version
    pub version: Option<String>,
    
    /// Additional custom metadata
    #[serde(default)]
    pub custom: HashMap<String, Value>,
}

impl AuditEvent {
    /// Create a new audit event with minimal information
    pub fn new(event_type: AuditEventType, component: String, message: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            component,
            message,
            metadata: AuditMetadata::default(),
            payload: json!({}),
            severity: AuditSeverity::Info,
            correlation_id: None,
        }
    }
    
    /// Create audit event from structured data
    pub fn from_structured(event_type: AuditEventType, component: String, data: Value) -> Self {
        let message = data.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Structured audit event")
            .to_string();
            
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            component,
            message,
            metadata: AuditMetadata::default(),
            payload: data,
            severity: AuditSeverity::Info,
            correlation_id: None,
        }
    }
    
    /// Add metadata to the event
    pub fn add_metadata(&mut self, key: &str, value: Value) {
        self.metadata.custom.insert(key.to_string(), value);
    }
    
    /// Set event severity
    pub fn with_severity(mut self, severity: AuditSeverity) -> Self {
        self.severity = severity;
        self
    }
    
    /// Set correlation ID for event grouping
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
    
    /// Set user context
    pub fn with_user(mut self, user_id: String) -> Self {
        self.metadata.user_id = Some(user_id);
        self
    }
    
    /// Set session context
    pub fn with_session(mut self, session_id: String) -> Self {
        self.metadata.session_id = Some(session_id);
        self
    }
    
    /// Set request context
    pub fn with_request(mut self, request_id: String) -> Self {
        self.metadata.request_id = Some(request_id);
        self
    }
    
    /// Set network context
    pub fn with_network(mut self, source_ip: String, user_agent: Option<String>) -> Self {
        self.metadata.source_ip = Some(source_ip);
        self.metadata.user_agent = user_agent;
        self
    }
    
    /// Set tenant context (for multi-tenant systems)
    pub fn with_tenant(mut self, tenant_id: String) -> Self {
        self.metadata.tenant_id = Some(tenant_id);
        self
    }
    
    /// Set structured payload
    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }
    
    /// Add metadata key-value pair (convenience method for backward compatibility)
    pub fn with_metadata(mut self, key: &str, value: Value) -> Self {
        // Add to the payload for now - in a full migration, this could be added to metadata
        if let Some(obj) = self.payload.as_object_mut() {
            obj.insert(key.to_string(), value);
        } else {
            let mut new_payload = serde_json::Map::new();
            new_payload.insert(key.to_string(), value);
            self.payload = Value::Object(new_payload);
        }
        self
    }
    
    /// Generate a new unique ID (legacy compatibility method)
    pub fn generate_id() -> String {
        Uuid::new_v4().to_string()
    }
}

impl Default for AuditMetadata {
    fn default() -> Self {
        Self {
            user_id: None,
            session_id: None,
            request_id: None,
            source_ip: None,
            user_agent: None,
            tenant_id: None,
            environment: Some("production".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            custom: HashMap::new(),
        }
    }
}

// Custom deserializer for backward compatibility with old audit log format
impl<'de> Deserialize<'de> for AuditMetadata {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let mut metadata = AuditMetadata::default();
        
        if let Value::Object(obj) = value {
            // Handle user_id field
            if let Some(user_id) = obj.get("user_id") {
                if let Some(user_id_str) = user_id.as_str() {
                    metadata.user_id = Some(user_id_str.to_string());
                }
            }
            
            // Handle session_id field
            if let Some(session_id) = obj.get("session_id") {
                if let Some(session_id_str) = session_id.as_str() {
                    metadata.session_id = Some(session_id_str.to_string());
                }
            }
            
            // Handle request_id field
            if let Some(request_id) = obj.get("request_id") {
                if let Some(request_id_str) = request_id.as_str() {
                    metadata.request_id = Some(request_id_str.to_string());
                }
            }
            
            // Handle source_ip field (backward compatibility: also check for "ip")
            if let Some(source_ip) = obj.get("source_ip") {
                if let Some(source_ip_str) = source_ip.as_str() {
                    metadata.source_ip = Some(source_ip_str.to_string());
                }
            } else if let Some(ip) = obj.get("ip") {
                // Backward compatibility: old format used "ip" instead of "source_ip"
                if let Some(ip_str) = ip.as_str() {
                    metadata.source_ip = Some(ip_str.to_string());
                }
            }
            
            // Handle user_agent field
            if let Some(user_agent) = obj.get("user_agent") {
                if let Some(user_agent_str) = user_agent.as_str() {
                    metadata.user_agent = Some(user_agent_str.to_string());
                }
            }
            
            // Handle tenant_id field
            if let Some(tenant_id) = obj.get("tenant_id") {
                if let Some(tenant_id_str) = tenant_id.as_str() {
                    metadata.tenant_id = Some(tenant_id_str.to_string());
                }
            }
            
            // Handle environment field
            if let Some(environment) = obj.get("environment") {
                if let Some(environment_str) = environment.as_str() {
                    metadata.environment = Some(environment_str.to_string());
                }
            }
            
            // Handle version field
            if let Some(version) = obj.get("version") {
                if let Some(version_str) = version.as_str() {
                    metadata.version = Some(version_str.to_string());
                }
            }
            
            // Handle custom field (optional, defaults to empty HashMap if not present)
            if let Some(custom) = obj.get("custom") {
                if let Value::Object(custom_obj) = custom {
                    for (key, value) in custom_obj {
                        metadata.custom.insert(key.clone(), value.clone());
                    }
                }
            }
            
            // For backward compatibility, add any unknown fields to custom
            let known_fields = ["user_id", "session_id", "request_id", "source_ip", "ip", 
                              "user_agent", "tenant_id", "environment", "version", "custom"];
            for (key, value) in obj {
                if !known_fields.contains(&key.as_str()) {
                    metadata.custom.insert(key.clone(), value.clone());
                }
            }
        }
        
        Ok(metadata)
    }
}

// Custom deserializer for the entire AuditEvent to handle backward compatibility
impl<'de> Deserialize<'de> for AuditEvent {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        
        if let Value::Object(obj) = value {
            // Extract required fields with fallbacks for old format
            
            // ID - generate if missing (old format)
            let id = obj.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| "")
                .to_string();
            let id = if id.is_empty() { Uuid::new_v4().to_string() } else { id };
            
            // Timestamp - required field
            let timestamp_str = obj.get("timestamp")
                .and_then(|v| v.as_str())
                .ok_or_else(|| serde::de::Error::missing_field("timestamp"))?;
            let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
                .map_err(|e| serde::de::Error::custom(format!("Invalid timestamp format: {}", e)))?
                .with_timezone(&Utc);
            
            // Event type - required field
            let event_type_value = obj.get("event_type")
                .ok_or_else(|| serde::de::Error::missing_field("event_type"))?;
            let event_type: AuditEventType = serde_json::from_value(event_type_value.clone())
                .map_err(|e| serde::de::Error::custom(format!("Invalid event_type: {}", e)))?;
            
            // Component - required field
            let component = obj.get("component")
                .and_then(|v| v.as_str())
                .ok_or_else(|| serde::de::Error::missing_field("component"))?
                .to_string();
            
            // Message - required field
            let message = obj.get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| serde::de::Error::missing_field("message"))?
                .to_string();
            
            // Metadata - handle old and new formats manually
            let metadata = if let Some(metadata_value) = obj.get("metadata") {
                if let Value::Object(metadata_obj) = metadata_value {
                    let mut metadata = AuditMetadata::default();
                    
                    // Extract known fields
                    if let Some(user_id) = metadata_obj.get("user_id").and_then(|v| v.as_str()) {
                        metadata.user_id = Some(user_id.to_string());
                    }
                    if let Some(session_id) = metadata_obj.get("session_id").and_then(|v| v.as_str()) {
                        metadata.session_id = Some(session_id.to_string());
                    }
                    if let Some(request_id) = metadata_obj.get("request_id").and_then(|v| v.as_str()) {
                        metadata.request_id = Some(request_id.to_string());
                    }
                    // Handle both "source_ip" and old "ip" field
                    if let Some(source_ip) = metadata_obj.get("source_ip").and_then(|v| v.as_str()) {
                        metadata.source_ip = Some(source_ip.to_string());
                    } else if let Some(ip) = metadata_obj.get("ip").and_then(|v| v.as_str()) {
                        metadata.source_ip = Some(ip.to_string());
                    }
                    if let Some(user_agent) = metadata_obj.get("user_agent").and_then(|v| v.as_str()) {
                        metadata.user_agent = Some(user_agent.to_string());
                    }
                    if let Some(tenant_id) = metadata_obj.get("tenant_id").and_then(|v| v.as_str()) {
                        metadata.tenant_id = Some(tenant_id.to_string());
                    }
                    if let Some(environment) = metadata_obj.get("environment").and_then(|v| v.as_str()) {
                        metadata.environment = Some(environment.to_string());
                    }
                    if let Some(version) = metadata_obj.get("version").and_then(|v| v.as_str()) {
                        metadata.version = Some(version.to_string());
                    }
                    
                    // Handle custom field if present
                    if let Some(custom_value) = metadata_obj.get("custom") {
                        if let Value::Object(custom_obj) = custom_value {
                            for (key, value) in custom_obj {
                                metadata.custom.insert(key.clone(), value.clone());
                            }
                        }
                    }
                    
                    // For backward compatibility, add any unknown fields to custom
                    let known_fields = ["user_id", "session_id", "request_id", "source_ip", "ip", 
                                      "user_agent", "tenant_id", "environment", "version", "custom"];
                    for (key, value) in metadata_obj {
                        if !known_fields.contains(&key.as_str()) {
                            metadata.custom.insert(key.clone(), value.clone());
                        }
                    }
                    
                    metadata
                } else {
                    AuditMetadata::default()
                }
            } else {
                AuditMetadata::default()
            };
            
            // Payload - use entire object as payload for old format, or extract payload field for new format
            let payload = if obj.contains_key("payload") {
                obj.get("payload").unwrap().clone()
            } else {
                // For old format, use the metadata as payload for backward compatibility
                obj.get("metadata").cloned().unwrap_or(json!({}))
            };
            
            // Severity - required field
            let severity_value = obj.get("severity")
                .ok_or_else(|| serde::de::Error::missing_field("severity"))?;
            let severity: AuditSeverity = serde_json::from_value(severity_value.clone())
                .map_err(|e| serde::de::Error::custom(format!("Invalid severity: {}", e)))?;
            
            // Correlation ID - optional field
            let correlation_id = obj.get("correlation_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            Ok(AuditEvent {
                id,
                timestamp,
                event_type,
                component,
                message,
                metadata,
                payload,
                severity,
                correlation_id,
            })
        } else {
            Err(serde::de::Error::custom("Expected object for AuditEvent"))
        }
    }
}

/// Specialized event builders for common scenarios
pub struct EventBuilders;

impl EventBuilders {
    /// Authentication event
    pub fn authentication(component: &str, user_id: &str, success: bool, method: &str) -> AuditEvent {
        AuditEvent::new(
            AuditEventType::Authentication,
            component.to_string(),
            format!("Authentication {} for user {} using {}", 
                   if success { "successful" } else { "failed" }, user_id, method)
        )
        .with_user(user_id.to_string())
        .with_severity(if success { AuditSeverity::Info } else { AuditSeverity::Warning })
        .with_payload(json!({
            "success": success,
            "method": method,
            "user_id": user_id
        }))
    }
    
    /// OAuth flow event
    pub fn oauth_flow(component: &str, server_name: &str, flow_type: &str, success: bool) -> AuditEvent {
        AuditEvent::new(
            AuditEventType::OauthAuthorization,
            component.to_string(),
            format!("OAuth {} {} for server {}", 
                   flow_type, 
                   if success { "successful" } else { "failed" }, 
                   server_name)
        )
        .with_severity(if success { AuditSeverity::Info } else { AuditSeverity::Error })
        .with_payload(json!({
            "server_name": server_name,
            "flow_type": flow_type,
            "success": success
        }))
    }
    
    /// Tool execution event
    pub fn tool_execution(component: &str, tool_name: &str, user_id: Option<&str>, success: bool) -> AuditEvent {
        AuditEvent::new(
            AuditEventType::ToolExecution,
            component.to_string(),
            format!("Tool {} execution {}", 
                   tool_name, 
                   if success { "successful" } else { "failed" })
        )
        .with_severity(if success { AuditSeverity::Info } else { AuditSeverity::Warning })
        .with_payload(json!({
            "tool_name": tool_name,
            "success": success,
            "user_id": user_id
        }))
    }
    
    /// Security violation event
    pub fn security_violation(component: &str, violation_type: &str, details: &str, severity: AuditSeverity) -> AuditEvent {
        AuditEvent::new(
            AuditEventType::SecurityViolation,
            component.to_string(),
            format!("Security violation: {} - {}", violation_type, details)
        )
        .with_severity(severity)
        .with_payload(json!({
            "violation_type": violation_type,
            "details": details
        }))
    }
    
    /// Administrative action event
    pub fn admin_action(component: &str, admin_user: &str, action: &str, target: &str) -> AuditEvent {
        AuditEvent::new(
            AuditEventType::AdminAction,
            component.to_string(),
            format!("Admin {} performed {} on {}", admin_user, action, target)
        )
        .with_user(admin_user.to_string())
        .with_severity(AuditSeverity::Warning) // Admin actions always get attention
        .with_payload(json!({
            "admin_user": admin_user,
            "action": action,
            "target": target
        }))
    }
    
    /// System health event
    pub fn system_health(component: &str, metric: &str, value: f64, threshold: Option<f64>) -> AuditEvent {
        let severity = if let Some(t) = threshold {
            if value > t { AuditSeverity::Warning } else { AuditSeverity::Info }
        } else {
            AuditSeverity::Info
        };
        
        AuditEvent::new(
            AuditEventType::PerformanceMetric,
            component.to_string(),
            format!("System metric {}: {}", metric, value)
        )
        .with_severity(severity)
        .with_payload(json!({
            "metric": metric,
            "value": value,
            "threshold": threshold
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(
            AuditEventType::Authentication,
            "mcp_server".to_string(),
            "User login successful".to_string()
        );
        
        assert_eq!(event.event_type, AuditEventType::Authentication);
        assert_eq!(event.component, "mcp_server");
        assert_eq!(event.severity, AuditSeverity::Info);
        assert!(!event.id.is_empty());
    }
    
    #[test]
    fn test_event_builders() {
        let auth_event = EventBuilders::authentication("auth_service", "user123", true, "oauth");
        assert_eq!(auth_event.event_type, AuditEventType::Authentication);
        assert_eq!(auth_event.metadata.user_id, Some("user123".to_string()));
        assert_eq!(auth_event.severity, AuditSeverity::Info);
        
        let violation_event = EventBuilders::security_violation(
            "allowlist", 
            "denied_tool", 
            "Tool not in allowlist", 
            AuditSeverity::Error
        );
        assert_eq!(violation_event.event_type, AuditEventType::SecurityViolation);
        assert_eq!(violation_event.severity, AuditSeverity::Error);
    }
    
    #[test]
    fn test_event_metadata() {
        let mut event = AuditEvent::new(
            AuditEventType::ToolExecution,
            "smart_discovery".to_string(),
            "Tool executed".to_string()
        );
        
        event.add_metadata("execution_time", json!(1.5));
        event.add_metadata("parameters", json!({"param1": "value1"}));
        
        assert_eq!(event.metadata.custom.len(), 2);
        assert_eq!(event.metadata.custom["execution_time"], json!(1.5));
    }
    
    #[test]
    fn test_event_chaining() {
        let event = AuditEvent::new(
            AuditEventType::OauthAuthorization,
            "oauth_manager".to_string(),
            "OAuth flow completed".to_string()
        )
        .with_user("user123".to_string())
        .with_session("session456".to_string())
        .with_severity(AuditSeverity::Info)
        .with_correlation_id("flow789".to_string());
        
        assert_eq!(event.metadata.user_id, Some("user123".to_string()));
        assert_eq!(event.metadata.session_id, Some("session456".to_string()));
        assert_eq!(event.severity, AuditSeverity::Info);
        assert_eq!(event.correlation_id, Some("flow789".to_string()));
    }
    
    #[test]
    fn test_backward_compatibility_deserialization() {
        // Test that old PascalCase format can be deserialized
        let old_event_type_json = r#""Authentication""#;
        let parsed_event_type: AuditEventType = serde_json::from_str(old_event_type_json).unwrap();
        assert_eq!(parsed_event_type, AuditEventType::Authentication);
        
        let old_severity_json = r#""Info""#;
        let parsed_severity: AuditSeverity = serde_json::from_str(old_severity_json).unwrap();
        assert_eq!(parsed_severity, AuditSeverity::Info);
        
        // Test that new snake_case/lowercase format still works
        let new_event_type_json = r#""authentication""#;
        let parsed_new_event_type: AuditEventType = serde_json::from_str(new_event_type_json).unwrap();
        assert_eq!(parsed_new_event_type, AuditEventType::Authentication);
        
        let new_severity_json = r#""info""#;
        let parsed_new_severity: AuditSeverity = serde_json::from_str(new_severity_json).unwrap();
        assert_eq!(parsed_new_severity, AuditSeverity::Info);
        
        // Test that both formats parse to the same result
        assert_eq!(parsed_event_type, parsed_new_event_type);
        assert_eq!(parsed_severity, parsed_new_severity);
        
        // Test more old PascalCase formats
        let test_cases = vec![
            ("SecurityViolation", AuditEventType::SecurityViolation),
            ("ToolExecution", AuditEventType::ToolExecution),
            ("OauthFlow", AuditEventType::OauthFlow),
            ("AdminAction", AuditEventType::AdminAction),
        ];
        
        for (old_format, expected) in test_cases {
            let json_str = format!(r#""{}""#, old_format);
            let parsed: AuditEventType = serde_json::from_str(&json_str).unwrap();
            assert_eq!(parsed, expected, "Failed to parse old format: {}", old_format);
        }
        
        // Test old severity formats
        let severity_cases = vec![
            ("Debug", AuditSeverity::Debug),
            ("Warning", AuditSeverity::Warning),
            ("Error", AuditSeverity::Error),
            ("Critical", AuditSeverity::Critical),
        ];
        
        for (old_format, expected) in severity_cases {
            let json_str = format!(r#""{}""#, old_format);
            let parsed: AuditSeverity = serde_json::from_str(&json_str).unwrap();
            assert_eq!(parsed, expected, "Failed to parse old severity format: {}", old_format);
        }
    }
    
    #[test]
    fn test_full_event_backward_compatibility() {
        // Test a complete old-format audit event (from actual logs)
        let old_format_json = r#"{
            "timestamp": "2025-08-22T07:08:16.000Z",
            "event_type": "SecurityViolation",
            "component": "allowlist_service",
            "message": "Tool execution denied by allowlist",
            "metadata": {
                "tool_name": "dangerous_tool",
                "user_id": "test_user_123"
            },
            "severity": "Warning",
            "correlation_id": "test_violation_004"
        }"#;
        
        // This should parse without error
        let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(old_format_json);
        assert!(parsed.is_ok(), "Failed to parse old format audit event");
        
        // Extract and test the enum fields specifically
        let event_obj = parsed.unwrap();
        let event_type_str = event_obj["event_type"].as_str().unwrap();
        let severity_str = event_obj["severity"].as_str().unwrap();
        
        let event_type: AuditEventType = serde_json::from_str(&format!(r#""{}""#, event_type_str)).unwrap();
        let severity: AuditSeverity = serde_json::from_str(&format!(r#""{}""#, severity_str)).unwrap();
        
        assert_eq!(event_type, AuditEventType::SecurityViolation);
        assert_eq!(severity, AuditSeverity::Warning);
    }
}