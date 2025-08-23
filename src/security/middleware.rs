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
    SecurityConfig, AllowlistService, SanitizationService, RbacService,
    AllowlistContext, PermissionContext,
    AuditCollector, AuditEvent, AuditEventType, AuditSeverity, get_audit_collector
};
use super::emergency::EmergencyLockdownManager;

/// Security middleware for MCP requests
pub struct SecurityMiddleware {
    config: SecurityConfig,
    allowlist_service: Option<Arc<AllowlistService>>,
    sanitization_service: Option<SanitizationService>,
    rbac_service: Option<Arc<RwLock<RbacService>>>,
    audit_service: Option<Arc<AuditCollector>>,
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
    /// Create a new security middleware with a shared allowlist service
    pub async fn with_shared_allowlist(
        config: SecurityConfig, 
        shared_allowlist_service: Option<Arc<AllowlistService>>
    ) -> Result<Self> {
        let mut middleware = Self {
            config: config.clone(),
            allowlist_service: None,
            sanitization_service: None,
            rbac_service: None,
            audit_service: None,
            emergency_manager: None,
        };
        
        // Use shared allowlist service if provided
        if let Some(allowlist_config) = &config.allowlist {
            if allowlist_config.enabled {
                if let Some(shared_service) = shared_allowlist_service {
                    let instance_id = format!("{:p}", shared_service.as_ref());
                    info!("✅ Using shared allowlist service from AdvancedServices - Instance ID: {}", instance_id);
                    middleware.allowlist_service = Some(shared_service);
                } else {
                    // Fallback to creating own service
                    let service = if !allowlist_config.data_file.is_empty() {
                        info!("🔄 Creating fallback allowlist service with data file: {}", allowlist_config.data_file);
                        AllowlistService::with_data_file(
                            allowlist_config.clone(),
                            allowlist_config.data_file.clone()
                        ).map_err(|e| ProxyError::config(format!("Failed to initialize allowlist service with data file '{}': {}", allowlist_config.data_file, e)))?
                    } else {
                        info!("🔄 Creating fallback allowlist service without data file (config-only)");
                        AllowlistService::new(allowlist_config.clone())
                            .map_err(|e| ProxyError::config(format!("Failed to initialize allowlist service: {}", e)))?
                    };
                    let arc_service = Arc::new(service);
                    let instance_id = format!("{:p}", arc_service.as_ref());
                    info!("🔒 Fallback allowlist service created - Instance ID: {}", instance_id);
                    middleware.allowlist_service = Some(arc_service);
                }
                info!("✅ Allowlist service configured successfully");
            }
        }
        
        // Initialize other services
        Self::initialize_other_services(&mut middleware).await?;
        Self::finalize_middleware(middleware).await
    }

    /// Create a new security middleware (legacy method for backward compatibility)
    pub async fn new(config: SecurityConfig) -> Result<Self> {
        Self::with_shared_allowlist(config, None).await
    }
    
    /// Initialize non-allowlist services
    async fn initialize_other_services(middleware: &mut Self) -> Result<()> {
        let config = &middleware.config;
        
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
        
        
        // Initialize audit service (use centralized audit collector)
        if let Some(audit_config) = &config.audit {
            if audit_config.enabled {
                // Use the global centralized audit collector directly
                if let Some(_collector) = get_audit_collector() {
                    // Middleware will use global collector directly, no separate service needed
                    info!("✅ Security middleware connected to global audit collector");
                    middleware.audit_service = None; // We'll use global collector directly
                } else {
                    warn!("⚠️ Global audit collector not available, audit features disabled in Security middleware");
                    middleware.audit_service = None;
                }
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
        
        Ok(())
    }
    
    /// Create security middleware with all services
    async fn finalize_middleware(mut middleware: Self) -> Result<Self> {
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
        // Log audit event using global collector
        if let Some(audit_collector) = get_audit_collector() {
            debug!("🔍 Global audit collector found, building audit entry");
            let audit_entry = self.build_audit_entry(&result, context).await;
            debug!("📋 Built audit entry: event_type={:?}, component={}, message='{}'", 
                   audit_entry.event_type, audit_entry.component, audit_entry.message);
            if let Err(e) = audit_collector.log_event(audit_entry).await {
                error!("❌ Failed to log audit event: {}", e);
            } else {
                debug!("✅ Successfully logged audit event to global collector");
            }
        } else {
            warn!("⚠️  Global audit collector not found - audit event will not be logged");
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
    async fn build_audit_entry(&self, result: &SecurityResult, context: &SecurityContext) -> AuditEvent {
        let event_type = if result.blocked {
            AuditEventType::SecurityViolation
        } else if context.tool.is_some() {
            AuditEventType::ToolExecution
        } else if context.resource.is_some() {
            AuditEventType::ResourceAccess
        } else {
            AuditEventType::Authorization
        };
        
        let component = "security_middleware";
        let message = if result.blocked {
            format!("Security check blocked: {}", result.reason)
        } else if result.requires_approval {
            format!("Security check requires approval: {}", result.reason)
        } else {
            format!("Security check passed: {}", result.reason)
        };
        
        let mut event = AuditEvent::new(event_type, component.to_string(), message);
        
        // Add user information as metadata
        if let Some(user) = &context.user {
            if let Some(user_id) = &user.id {
                event = event.with_metadata("user_id", serde_json::json!(user_id));
            }
            event = event.with_metadata("user_roles", serde_json::json!(user.roles));
            event = event.with_metadata("user_permissions", serde_json::json!(user.permissions));
            if let Some(api_key) = &user.api_key_name {
                event = event.with_metadata("api_key_name", serde_json::json!(api_key));
            }
            event = event.with_metadata("auth_method", serde_json::json!(user.auth_method));
        }
        
        // Add request information as metadata
        event = event.with_metadata("request_id", serde_json::json!(context.request.id));
        event = event.with_metadata("request_method", serde_json::json!(context.request.method));
        event = event.with_metadata("request_path", serde_json::json!(context.request.path));
        if let Some(client_ip) = &context.request.client_ip {
            event = event.with_metadata("client_ip", serde_json::json!(client_ip));
        }
        if let Some(user_agent) = &context.request.user_agent {
            event = event.with_metadata("user_agent", serde_json::json!(user_agent));
        }
        event = event.with_metadata("request_headers", serde_json::json!(context.request.headers));
        if let Some(body) = &context.request.body {
            event = event.with_metadata("request_body_size", serde_json::json!(body.len()));
        }
        
        // Add tool information as metadata
        if let Some(tool) = &context.tool {
            event = event.with_metadata("tool_name", serde_json::json!(tool.name));
            event = event.with_metadata("tool_parameters", serde_json::json!(tool.parameters));
            if let Some(source) = &tool.source {
                event = event.with_metadata("tool_source", serde_json::json!(source));
            }
        }
        
        // Add resource information as metadata
        if let Some(resource) = &context.resource {
            event = event.with_metadata("resource_uri", serde_json::json!(resource.uri));
            event = event.with_metadata("resource_type", serde_json::json!(resource.resource_type));
            event = event.with_metadata("resource_operation", serde_json::json!(resource.operation));
        }
        
        // Add security result information as metadata
        event = event.with_metadata("security_allowed", serde_json::json!(result.allowed));
        event = event.with_metadata("security_blocked", serde_json::json!(result.blocked));
        event = event.with_metadata("security_requires_approval", serde_json::json!(result.requires_approval));
        event = event.with_metadata("security_modified", serde_json::json!(result.modified));
        event = event.with_metadata("security_reason", serde_json::json!(result.reason));
        
        // Add security events as metadata
        let security_events: Vec<serde_json::Value> = result.events.iter().map(|e| {
            serde_json::json!({
                "event_type": e.event_type,
                "source": e.source,
                "message": e.message,
                "severity": format!("{:?}", e.severity),
                "details": e.details
            })
        }).collect();
        event = event.with_metadata("security_events", serde_json::json!(security_events));
        
        // Add permissions checked
        let permissions_checked = self.determine_required_permissions(context);
        event = event.with_metadata("permissions_checked", serde_json::json!(permissions_checked));
        
        // Add policies applied
        let policies_applied: Vec<String> = result.events.iter().map(|e| e.source.clone()).collect();
        event = event.with_metadata("policies_applied", serde_json::json!(policies_applied));
        
        // Add error information if blocked
        if result.blocked {
            event = event.with_metadata("error_code", serde_json::json!("SECURITY_VIOLATION"));
            event = event.with_metadata("error_message", serde_json::json!(result.reason));
            if let Some(error_msg) = &result.error_message {
                event = event.with_metadata("error_details", serde_json::json!(error_msg));
            }
        }
        
        // Add context metadata
        for (key, value) in &context.metadata {
            event = event.with_metadata(&format!("context_{}", key), value.clone());
        }
        
        event
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
        
        // Extract roles based on permissions and authentication method
        let roles = match ctx {
            crate::auth::AuthenticationResult::ApiKey(key_entry) => {
                // Map API key permissions to roles
                if key_entry.permissions.contains(&"admin".to_string()) {
                    vec!["admin".to_string()]
                } else if key_entry.permissions.contains(&"write".to_string()) {
                    vec!["user".to_string(), "writer".to_string()]
                } else if key_entry.permissions.contains(&"read".to_string()) {
                    vec!["user".to_string(), "reader".to_string()]
                } else {
                    vec!["guest".to_string()]
                }
            },
            crate::auth::AuthenticationResult::OAuth(_) => {
                // OAuth users get default user role
                vec!["user".to_string(), "oauth_user".to_string()]
            },
            crate::auth::AuthenticationResult::Jwt(jwt_result) => {
                // Extract roles from JWT permissions
                if jwt_result.permissions.contains(&"admin".to_string()) {
                    vec!["admin".to_string()]
                } else if jwt_result.permissions.contains(&"write".to_string()) {
                    vec!["user".to_string(), "writer".to_string()]
                } else {
                    vec!["user".to_string(), "reader".to_string()]
                }
            },
            crate::auth::AuthenticationResult::ServiceAccount(sa_result) => {
                // Service accounts get service role plus permission-based roles
                let mut roles = vec!["service_account".to_string()];
                if sa_result.permissions.contains(&"admin".to_string()) {
                    roles.push("admin".to_string());
                } else if sa_result.permissions.contains(&"write".to_string()) {
                    roles.push("writer".to_string());
                } else {
                    roles.push("reader".to_string());
                }
                roles
            },
            crate::auth::AuthenticationResult::DeviceCode(_) => {
                // Device code users get standard user role
                vec!["user".to_string(), "device_user".to_string()]
            }
        };
        
        SecurityUser {
            id: Some(ctx.get_user_id()),
            roles,
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