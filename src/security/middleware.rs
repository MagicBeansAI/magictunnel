//! Security middleware for MagicTunnel MCP server
//!
//! Integrates all security components (allowlisting, sanitization, RBAC, policies, audit)
//! into a unified middleware system for the MCP server.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use serde_json;
use chrono::Utc;
use crate::error::{Result, ProxyError};

use super::{
    SecurityConfig, AllowlistService, SanitizationService, RbacService, AuditService,
    AllowlistContext, PermissionContext,
    AuditEntry, AuditEventType, AuditUser, AuditRequest,
    AuditTool, AuditResource, AuditSecurity, AuditOutcome, AuditError
};
use super::emergency::EmergencyLockdownManager;

/// Security middleware for MCP requests
pub struct SecurityMiddleware {
    config: SecurityConfig,
    allowlist_service: Option<AllowlistService>,
    sanitization_service: Option<SanitizationService>,
    rbac_service: Option<Arc<RwLock<RbacService>>>,
    audit_service: Option<Arc<AuditService>>,
    emergency_manager: Option<Arc<EmergencyLockdownManager>>,
}

/// Context for security evaluation
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// User information from authentication
    pub user: Option<SecurityUser>,
    /// Request information
    pub request: SecurityRequest,
    /// Tool information (if applicable)
    pub tool: Option<SecurityTool>,
    /// Resource information (if applicable)
    pub resource: Option<SecurityResource>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// User information for security context
#[derive(Debug, Clone)]
pub struct SecurityUser {
    /// User ID
    pub id: Option<String>,
    /// User roles
    pub roles: Vec<String>,
    /// User permissions
    pub permissions: Vec<String>,
    /// API key name (if using API key auth)
    pub api_key_name: Option<String>,
    /// Authentication method
    pub auth_method: String,
}

/// Request information for security context
#[derive(Debug, Clone)]
pub struct SecurityRequest {
    /// Request ID
    pub id: String,
    /// Request method
    pub method: String,
    /// Request path
    pub path: String,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body
    pub body: Option<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Tool information for security context
#[derive(Debug, Clone)]
pub struct SecurityTool {
    /// Tool name
    pub name: String,
    /// Tool parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Tool source/capability
    pub source: Option<String>,
}

/// Resource information for security context
#[derive(Debug, Clone)]
pub struct SecurityResource {
    /// Resource URI
    pub uri: String,
    /// Resource type
    pub resource_type: String,
    /// Operation being performed
    pub operation: String,
}

/// Result of security evaluation
#[derive(Debug, Clone)]
pub struct SecurityResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Whether the request should be blocked
    pub blocked: bool,
    /// Whether approval is required
    pub requires_approval: bool,
    /// Whether the request was modified
    pub modified: bool,
    /// Modified request data (if any)
    pub modified_data: Option<serde_json::Value>,
    /// Reason for the decision
    pub reason: String,
    /// Security events that occurred
    pub events: Vec<SecurityEvent>,
    /// HTTP status code to return (if blocked)
    pub status_code: Option<u16>,
    /// Error message (if blocked)
    pub error_message: Option<String>,
}

/// Security event for logging
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    /// Event type
    pub event_type: String,
    /// Event source (allowlist, sanitization, etc.)
    pub source: String,
    /// Event message
    pub message: String,
    /// Event severity
    pub severity: SecuritySeverity,
    /// Additional details
    pub details: HashMap<String, serde_json::Value>,
}

/// Security event severity
#[derive(Debug, Clone)]
pub enum SecuritySeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl SecurityMiddleware {
    /// Create a new security middleware
    pub async fn new(config: SecurityConfig) -> Result<Self> {
        let mut middleware = Self {
            config: config.clone(),
            allowlist_service: None,
            sanitization_service: None,
            rbac_service: None,
            audit_service: None,
            emergency_manager: None,
        };
        
        // Initialize allowlist service (unified: regular or ultra-fast based on config)
        if let Some(allowlist_config) = &config.allowlist {
            if allowlist_config.enabled {
                let service = AllowlistService::new(allowlist_config.clone())
                    .map_err(|e| ProxyError::config(format!("Failed to initialize allowlist service: {}", e)))?;
                
                middleware.allowlist_service = Some(service);
                info!("Allowlist service initialized (high-performance)");
            }
        }
        
        // Initialize sanitization service
        if let Some(sanitization_config) = &config.sanitization {
            if sanitization_config.enabled {
                let service = SanitizationService::new(sanitization_config.clone())
                    .map_err(|e| ProxyError::config(format!("Failed to initialize sanitization service: {}", e)))?;
                middleware.sanitization_service = Some(service);
                info!("Sanitization service initialized");
            }
        }
        
        // Initialize RBAC service
        if let Some(rbac_config) = &config.rbac {
            if rbac_config.enabled {
                let service = RbacService::new(rbac_config.clone())
                    .map_err(|e| ProxyError::config(format!("Failed to initialize RBAC service: {}", e)))?;
                middleware.rbac_service = Some(Arc::new(RwLock::new(service)));
                info!("RBAC service initialized");
            }
        }
        
        
        // Initialize audit service
        if let Some(audit_config) = &config.audit {
            if audit_config.enabled {
                let service = AuditService::new(audit_config.clone()).await
                    .map_err(|e| ProxyError::config(format!("Failed to initialize audit service: {}", e)))?;
                middleware.audit_service = Some(Arc::new(service));
                info!("Audit service initialized");
            }
        }
        
        // Initialize emergency lockdown manager
        if let Some(emergency_config) = &config.emergency_lockdown {
            if emergency_config.enabled {
                let manager = EmergencyLockdownManager::new(emergency_config.clone()).await
                    .map_err(|e| ProxyError::config(format!("Failed to initialize emergency lockdown manager: {}", e)))?;
                middleware.emergency_manager = Some(Arc::new(manager));
                info!("Emergency lockdown manager initialized");
            }
        }
        
        info!("Security middleware initialized with {} active services", middleware.active_service_count());
        Ok(middleware)
    }
    
    /// Evaluate security for a request
    pub async fn evaluate_security(
        &self,
        context: &SecurityContext,
    ) -> SecurityResult {
        if !self.config.enabled {
            return SecurityResult {
                allowed: true,
                blocked: false,
                requires_approval: false,
                modified: false,
                modified_data: None,
                reason: "Security disabled".to_string(),
                events: vec![],
                status_code: None,
                error_message: None,
            };
        }
        
        // 1. Emergency Lockdown Check (ABSOLUTE PRIORITY - checked before anything else)
        if let Some(ref emergency_manager) = self.emergency_manager {
            if emergency_manager.is_lockdown_active() {
                // Increment blocked request counter
                let blocked_count = emergency_manager.increment_blocked_requests();
                
                info!("Request blocked due to emergency lockdown (#{} blocked)", blocked_count);
                
                return SecurityResult {
                    allowed: false,
                    blocked: true,
                    requires_approval: false,
                    modified: false,
                    modified_data: None,
                    reason: "Emergency lockdown is active - all requests are blocked".to_string(),
                    events: vec![SecurityEvent {
                        event_type: "emergency_lockdown".to_string(),
                        source: "emergency".to_string(),
                        message: format!("Request blocked by emergency lockdown (#{} requests blocked in this session)", blocked_count),
                        severity: SecuritySeverity::Critical,
                        details: HashMap::new(),
                    }],
                    status_code: Some(503), // Service Unavailable
                    error_message: Some("System is currently under emergency lockdown. All requests are temporarily blocked.".to_string()),
                };
            }
        }
        
        let mut result = SecurityResult {
            allowed: true,
            blocked: false,
            requires_approval: false,
            modified: false,
            modified_data: None,
            reason: "Security evaluation passed".to_string(),
            events: vec![],
            status_code: None,
            error_message: None,
        };
        
        debug!("Evaluating security for request: {}", context.request.id);
        
        
        // 2. RBAC Permission Check
        if let Some(rbac_service) = &self.rbac_service {
            let permission_context = self.build_permission_context(context);
            let rbac = rbac_service.read().await;
            
            // Check required permissions based on request type
            let required_permissions = self.determine_required_permissions(context);
            
            for permission in &required_permissions {
                let permission_result = rbac.check_permission(permission, &permission_context);
                
                if !permission_result.granted {
                    result.blocked = true;
                    result.allowed = false;
                    result.reason = permission_result.reason;
                    result.status_code = Some(403);
                    result.error_message = Some(format!("Insufficient permissions: {}", permission));
                    self.add_security_event(&mut result, "rbac", &format!("Permission denied: {}", permission), SecuritySeverity::Error);
                    return self.finalize_result(result, context).await;
                }
            }
            
            if !required_permissions.is_empty() {
                self.add_security_event(&mut result, "rbac", "Permissions verified", SecuritySeverity::Info);
            }
        }
        
        // 4. Tool Allowlisting
        if let Some(tool) = &context.tool {
            if let Some(allowlist_service) = &self.allowlist_service {
                let allowlist_context = self.build_allowlist_context(context);
                let allowlist_result = allowlist_service.check_tool_access(
                    &tool.name,
                    &tool.parameters,
                    &allowlist_context,
                );
                
                if !allowlist_result.allowed {
                    result.blocked = true;
                    result.allowed = false;
                    result.reason = allowlist_result.reason.to_string();
                    result.status_code = Some(403);
                    result.error_message = Some("Tool access denied by allowlist".to_string());
                    self.add_security_event(&mut result, "allowlist", "Tool blocked by allowlist", SecuritySeverity::Error);
                    return self.finalize_result(result, context).await;
                }
                
                if allowlist_result.requires_approval {
                    result.requires_approval = true;
                    result.reason = allowlist_result.reason.to_string();
                    self.add_security_event(&mut result, "allowlist", "Tool requires approval", SecuritySeverity::Warning);
                    return self.finalize_result(result, context).await;
                }
                
                self.add_security_event(&mut result, "allowlist", "Tool access allowed", SecuritySeverity::Info);
            }
        }
        
        // 5. Resource Allowlisting
        if let Some(resource) = &context.resource {
            if let Some(allowlist_service) = &self.allowlist_service {
                let allowlist_context = self.build_allowlist_context(context);
                let allowlist_result = allowlist_service.check_resource_access(
                    &resource.uri,
                    &allowlist_context,
                );
                
                if !allowlist_result.allowed {
                    result.blocked = true;
                    result.allowed = false;
                    result.reason = allowlist_result.reason.to_string();
                    result.status_code = Some(403);
                    result.error_message = Some("Resource access denied by allowlist".to_string());
                    self.add_security_event(&mut result, "allowlist", "Resource blocked by allowlist", SecuritySeverity::Error);
                    return self.finalize_result(result, context).await;
                }
                
                self.add_security_event(&mut result, "allowlist", "Resource access allowed", SecuritySeverity::Info);
            }
        }
        
        // 6. Request Sanitization
        if let Some(sanitization_service) = &self.sanitization_service {
            if let Some(body) = &context.request.body {
                let mut request_data = serde_json::from_str(body).unwrap_or(serde_json::json!({}));
                let tool_name = context.tool.as_ref().map(|t| t.name.as_str());
                
                let sanitization_result = sanitization_service.sanitize_request(&mut request_data, tool_name);
                
                if sanitization_result.should_block {
                    result.blocked = true;
                    result.allowed = false;
                    result.reason = "Request blocked by sanitization policy".to_string();
                    result.status_code = Some(400);
                    result.error_message = Some("Request contains prohibited content".to_string());
                    self.add_security_event(&mut result, "sanitization", "Request blocked by sanitization", SecuritySeverity::Error);
                    return self.finalize_result(result, context).await;
                }
                
                if sanitization_result.requires_approval {
                    result.requires_approval = true;
                    result.reason = "Request requires approval due to sanitization policy".to_string();
                    self.add_security_event(&mut result, "sanitization", "Request requires approval", SecuritySeverity::Warning);
                    return self.finalize_result(result, context).await;
                }
                
                if sanitization_result.modified {
                    result.modified = true;
                    result.modified_data = Some(request_data);
                    self.add_security_event(&mut result, "sanitization", "Request sanitized", SecuritySeverity::Info);
                }
            }
        }
        
        self.finalize_result(result, context).await
    }
    
    
    /// Build permission context from security context
    fn build_permission_context(&self, context: &SecurityContext) -> PermissionContext {
        PermissionContext {
            user_id: context.user.as_ref().and_then(|u| u.id.clone()),
            user_roles: context.user.as_ref().map(|u| u.roles.clone()).unwrap_or_default(),
            api_key_name: context.user.as_ref().and_then(|u| u.api_key_name.clone()),
            resource: context.resource.as_ref().map(|r| r.uri.clone()),
            action: context.resource.as_ref().map(|r| r.operation.clone()),
            client_ip: context.request.client_ip.clone(),
            timestamp: context.request.timestamp,
            metadata: context.metadata.clone(),
        }
    }
    
    /// Build allowlist context from security context
    fn build_allowlist_context(&self, context: &SecurityContext) -> AllowlistContext {
        AllowlistContext {
            user_id: context.user.as_ref().and_then(|u| u.id.clone()),
            user_roles: context.user.as_ref().map(|u| u.roles.clone()).unwrap_or_default(),
            api_key_name: context.user.as_ref().and_then(|u| u.api_key_name.clone()),
            permissions: context.user.as_ref().map(|u| u.permissions.clone()).unwrap_or_default(),
            source: context.tool.as_ref().and_then(|t| t.source.clone()),
            client_ip: context.request.client_ip.clone(),
        }
    }
    
    /// Determine required permissions based on request
    fn determine_required_permissions(&self, context: &SecurityContext) -> Vec<String> {
        let mut permissions = Vec::new();
        
        // Base permissions based on request type
        match context.request.method.as_str() {
            "GET" => permissions.push("read".to_string()),
            "POST" | "PUT" | "PATCH" => permissions.push("write".to_string()),
            "DELETE" => {
                permissions.push("write".to_string());
                permissions.push("admin".to_string());
            }
            _ => permissions.push("read".to_string()),
        }
        
        // Tool-specific permissions
        if let Some(tool) = &context.tool {
            permissions.push(format!("tool:{}", tool.name));
            
            // Add category-based permissions
            if tool.name.contains("database") || tool.name.contains("sql") {
                permissions.push("database".to_string());
            }
            if tool.name.contains("network") || tool.name.contains("ping") || tool.name.contains("traceroute") {
                permissions.push("network".to_string());
            }
            if tool.name.contains("file") || tool.name.contains("fs") {
                permissions.push("filesystem".to_string());
            }
        }
        
        // Resource-specific permissions
        if let Some(resource) = &context.resource {
            permissions.push(format!("resource:{}", resource.resource_type));
        }
        
        permissions
    }
    
    /// Add security event to result
    fn add_security_event(
        &self,
        result: &mut SecurityResult,
        source: &str,
        message: &str,
        severity: SecuritySeverity,
    ) {
        result.events.push(SecurityEvent {
            event_type: "security_check".to_string(),
            source: source.to_string(),
            message: message.to_string(),
            severity,
            details: HashMap::new(),
        });
    }
    
    /// Finalize security result and log audit event
    async fn finalize_result(
        &self,
        result: SecurityResult,
        context: &SecurityContext,
    ) -> SecurityResult {
        // Log audit event
        if let Some(audit_service) = &self.audit_service {
            let audit_entry = self.build_audit_entry(&result, context).await;
            if let Err(e) = audit_service.log_event(audit_entry).await {
                error!("Failed to log audit event: {}", e);
            }
        }
        
        // Log security events
        for event in &result.events {
            match event.severity {
                SecuritySeverity::Info => info!("Security [{}]: {}", event.source, event.message),
                SecuritySeverity::Warning => warn!("Security [{}]: {}", event.source, event.message),
                SecuritySeverity::Error => error!("Security [{}]: {}", event.source, event.message),
                SecuritySeverity::Critical => error!("Security [{}] CRITICAL: {}", event.source, event.message),
            }
        }
        
        result
    }
    
    /// Build audit entry from security result and context
    async fn build_audit_entry(&self, result: &SecurityResult, context: &SecurityContext) -> AuditEntry {
        let event_type = if result.blocked {
            AuditEventType::SecurityViolation
        } else if context.tool.is_some() {
            AuditEventType::ToolExecution
        } else if context.resource.is_some() {
            AuditEventType::ResourceAccess
        } else {
            AuditEventType::Authorization
        };
        
        let outcome = if result.blocked {
            AuditOutcome::Blocked
        } else if result.requires_approval {
            AuditOutcome::PendingApproval
        } else if result.allowed {
            AuditOutcome::Success
        } else {
            AuditOutcome::Failure
        };
        
        AuditEntry {
            id: AuditEntry::generate_id(),
            timestamp: Utc::now(),
            event_type,
            user: context.user.as_ref().map(|u| AuditUser {
                id: u.id.clone(),
                name: None,
                roles: u.roles.clone(),
                api_key_name: u.api_key_name.clone(),
                auth_method: u.auth_method.clone(),
            }),
            request: Some(AuditRequest {
                id: Some(context.request.id.clone()),
                method: context.request.method.clone(),
                path: context.request.path.clone(),
                client_ip: context.request.client_ip.clone(),
                user_agent: context.request.user_agent.clone(),
                headers: context.request.headers.clone(),
                body: context.request.body.clone(),
                size: context.request.body.as_ref().map(|b| b.len()).unwrap_or(0),
            }),
            response: None, // Will be filled in by response handler
            tool: context.tool.as_ref().map(|t| AuditTool {
                name: t.name.clone(),
                parameters: Some(t.parameters.clone()),
                result: None,
                execution_time_ms: None,
                success: result.allowed,
            }),
            resource: context.resource.as_ref().map(|r| AuditResource {
                uri: r.uri.clone(),
                resource_type: r.resource_type.clone(),
                operation: r.operation.clone(),
            }),
            security: AuditSecurity {
                authenticated: context.user.is_some(),
                authorized: result.allowed,
                permissions_checked: self.determine_required_permissions(context),
                policies_applied: result.events.iter().map(|e| e.source.clone()).collect(),
                content_sanitized: result.modified,
                approval_required: result.requires_approval,
            },
            metadata: context.metadata.clone(),
            outcome,
            error: if result.blocked {
                Some(AuditError {
                    code: "SECURITY_VIOLATION".to_string(),
                    message: result.reason.clone(),
                    details: result.error_message.clone(),
                    stack_trace: None,
                })
            } else {
                None
            },
        }
    }
    
    /// Get number of active services
    fn active_service_count(&self) -> usize {
        let mut count = 0;
        if self.allowlist_service.is_some() { count += 1; }
        if self.sanitization_service.is_some() { count += 1; }
        if self.rbac_service.is_some() { count += 1; }
        if self.audit_service.is_some() { count += 1; }
        if self.emergency_manager.is_some() { count += 1; }
        count
    }
    
    /// Check if security is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.config.has_any_enabled()
    }
    
    /// Check if emergency lockdown is currently active
    pub fn is_emergency_lockdown_active(&self) -> bool {
        self.emergency_manager
            .as_ref()
            .map_or(false, |manager| manager.is_lockdown_active())
    }
    
    /// Get emergency lockdown manager reference (for external access)
    pub fn get_emergency_manager(&self) -> Option<Arc<EmergencyLockdownManager>> {
        self.emergency_manager.as_ref().map(Arc::clone)
    }
}

/// Helper function to extract user from authentication context
pub fn extract_security_user(auth_context: Option<&crate::auth::AuthenticationResult>) -> Option<SecurityUser> {
    auth_context.map(|ctx| {
        let (auth_method, api_key_name) = match ctx {
            crate::auth::AuthenticationResult::ApiKey(key_entry) => {
                ("api_key".to_string(), Some(key_entry.name.clone()))
            }
            crate::auth::AuthenticationResult::OAuth(_) => {
                ("oauth".to_string(), None)
            }
            crate::auth::AuthenticationResult::Jwt(_) => {
                ("jwt".to_string(), None)
            }
            crate::auth::AuthenticationResult::ServiceAccount(sa_result) => {
                ("service_account".to_string(), sa_result.user_info.name.clone())
            }
            crate::auth::AuthenticationResult::DeviceCode(device_result) => {
                ("device_code".to_string(), device_result.user_info.as_ref().and_then(|info| info.name.clone()))
            }
        };
        
        SecurityUser {
            id: Some(ctx.get_user_id()),
            roles: vec![], // TODO: Extract roles from auth result if available
            permissions: ctx.get_permissions(),
            api_key_name,
            auth_method,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::RbacConfig;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_security_middleware_disabled() {
        let config = SecurityConfig {
            enabled: false,
            ..Default::default()
        };
        
        let middleware = SecurityMiddleware::new(config).await.unwrap();
        
        let context = SecurityContext {
            user: None,
            request: SecurityRequest {
                id: "test-123".to_string(),
                method: "GET".to_string(),
                path: "/test".to_string(),
                client_ip: None,
                user_agent: None,
                headers: HashMap::new(),
                body: None,
                timestamp: Utc::now(),
            },
            tool: None,
            resource: None,
            metadata: HashMap::new(),
        };
        
        let result = middleware.evaluate_security(&context).await;
        assert!(result.allowed);
        assert!(!result.blocked);
    }
    
    #[tokio::test]
    async fn test_security_middleware_rbac() {
        let mut config = SecurityConfig::default();
        config.enabled = true;
        config.rbac = Some(RbacConfig {
            enabled: true,
            ..Default::default()
        });
        
        let middleware = SecurityMiddleware::new(config).await.unwrap();
        
        let context = SecurityContext {
            user: Some(SecurityUser {
                id: Some("test-user".to_string()),
                roles: vec!["user".to_string()],
                permissions: vec!["read".to_string()],
                api_key_name: None,
                auth_method: "jwt".to_string(),
            }),
            request: SecurityRequest {
                id: "test-123".to_string(),
                method: "GET".to_string(),
                path: "/test".to_string(),
                client_ip: None,
                user_agent: None,
                headers: HashMap::new(),
                body: None,
                timestamp: Utc::now(),
            },
            tool: None,
            resource: None,
            metadata: HashMap::new(),
        };
        
        let result = middleware.evaluate_security(&context).await;
        assert!(result.allowed);
        assert!(!result.blocked);
    }
}