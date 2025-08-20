//! OAuth and Security Audit Logging
//! 
//! This module provides comprehensive audit logging for OAuth flows,
//! security events, and MCP server interactions.

use serde_json::{json, Value};
use chrono::{DateTime, Utc};
use tracing::{info, warn, error};
use std::collections::HashMap;

/// OAuth and security audit logger
pub struct AuditLogger {
    /// Enable audit logging
    enabled: bool,
    /// Log structured events to separate audit log
    structured_logging: bool,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new(enabled: bool, structured_logging: bool) -> Self {
        Self {
            enabled,
            structured_logging,
        }
    }

    /// Log OAuth discovery attempt
    pub async fn log_oauth_discovery_attempt(&self, server_name: &str, base_url: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_discovery_attempt",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "base_url": base_url,
            "action": "discovery_start"
        });

        self.log_audit_event("OAuth Discovery Attempt", &event).await;
        info!("🔍 [AUDIT] OAuth discovery attempt for server: {}", server_name);
    }

    /// Log OAuth server metadata discovery success
    pub async fn log_oauth_metadata_discovered(&self, server_name: &str, metadata: &Value) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_metadata_discovered",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "metadata": metadata,
            "action": "discovery_success"
        });

        self.log_audit_event("OAuth Metadata Discovery", &event).await;
        info!("✅ [AUDIT] OAuth metadata discovered for server: {}", server_name);
    }

    /// Log dynamic OAuth registration attempt
    pub async fn log_oauth_registration_attempt(&self, server_name: &str, client_name: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_registration_attempt",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "client_name": client_name,
            "action": "registration_start"
        });

        self.log_audit_event("OAuth Dynamic Registration Attempt", &event).await;
        info!("🔐 [AUDIT] OAuth dynamic registration attempt for server: {}", server_name);
    }

    /// Log OAuth registration failure
    pub async fn log_oauth_registration_failure(&self, server_name: &str, error: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_registration_failure",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "error": error,
            "action": "registration_failed",
            "severity": "error"
        });

        self.log_audit_event("OAuth Registration Failure", &event).await;
        error!("❌ [AUDIT] OAuth registration failed for server: {} - {}", server_name, error);
    }

    /// Log successful OAuth registration
    pub async fn log_oauth_registration_success(&self, server_name: &str, client_id: &str, scopes: &[String]) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_registration_success",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "client_id": client_id,
            "granted_scopes": scopes,
            "action": "registration_success"
        });

        self.log_audit_event("OAuth Registration Success", &event).await;
        info!("✅ [AUDIT] OAuth registration successful for server: {} (client_id: {})", server_name, client_id);
    }

    /// Log OAuth authorization flow start
    pub async fn log_oauth_authorization_start(&self, server_name: &str, auth_url: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_authorization_start",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "authorization_url": auth_url,
            "action": "authorization_start",
            "user_interaction": "browser_opened"
        });

        self.log_audit_event("OAuth Authorization Start", &event).await;
        info!("🌐 [AUDIT] OAuth authorization started for server: {}", server_name);
    }

    /// Log OAuth authorization success
    pub async fn log_oauth_authorization_success(&self, server_name: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_authorization_success",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "action": "authorization_success"
        });

        self.log_audit_event("OAuth Authorization Success", &event).await;
        info!("✅ [AUDIT] OAuth authorization successful for server: {}", server_name);
    }

    /// Log OAuth authorization failure
    pub async fn log_oauth_authorization_failure(&self, server_name: &str, error: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_authorization_failure",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "error": error,
            "action": "authorization_failed",
            "severity": "error"
        });

        self.log_audit_event("OAuth Authorization Failure", &event).await;
        error!("❌ [AUDIT] OAuth authorization failed for server: {} - {}", server_name, error);
    }

    /// Log OAuth token exchange attempt
    pub async fn log_oauth_token_exchange_attempt(&self, server_name: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_token_exchange_attempt",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "action": "token_exchange_start"
        });

        self.log_audit_event("OAuth Token Exchange Attempt", &event).await;
        info!("🔄 [AUDIT] OAuth token exchange attempt for server: {}", server_name);
    }

    /// Log OAuth token exchange failure
    pub async fn log_oauth_token_exchange_failure(&self, server_name: &str, error: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_token_exchange_failure",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "error": error,
            "action": "token_exchange_failed",
            "severity": "error"
        });

        self.log_audit_event("OAuth Token Exchange Failure", &event).await;
        error!("❌ [AUDIT] OAuth token exchange failed for server: {} - {}", server_name, error);
    }

    /// Log successful OAuth token exchange
    pub async fn log_oauth_token_exchange_success(&self, server_name: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_token_exchange_success",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "action": "token_exchange_success"
        });

        self.log_audit_event("OAuth Token Exchange Success", &event).await;
        info!("✅ [AUDIT] OAuth token exchange successful for server: {}", server_name);
    }

    /// Log OAuth details forwarded to client
    pub async fn log_oauth_forwarded_to_client(&self, server_name: &str, oauth_details: &Value) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_forwarded_to_client",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "oauth_termination_here": false,
            "action": "oauth_delegated_to_client",
            "client_receives": {
                "authorization_endpoint": oauth_details.get("authorization_endpoint"),
                "token_endpoint": oauth_details.get("token_endpoint"),
                "client_id": oauth_details.get("client_id"),
                "required_scopes": oauth_details.get("required_scopes")
            }
        });

        self.log_audit_event("OAuth Forwarded to Client", &event).await;
        info!("📤 [AUDIT] OAuth details forwarded to client for server: {}", server_name);
    }

    /// Log OAuth token usage for MCP API call
    pub async fn log_oauth_token_usage(&self, server_name: &str, method: &str, endpoint: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_token_usage",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "method": method,
            "endpoint": endpoint,
            "action": "api_call_authenticated"
        });

        self.log_audit_event("OAuth Token Usage", &event).await;
        info!("🔐 [AUDIT] OAuth token used for API call: {} {} on server: {}", method, endpoint, server_name);
    }

    /// Log OAuth token refresh attempt
    pub async fn log_oauth_token_refresh(&self, server_name: &str, success: bool, error: Option<&str>) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "oauth_token_refresh",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "success": success,
            "error": error,
            "action": if success { "token_refresh_success" } else { "token_refresh_failed" },
            "severity": if success { "info" } else { "warning" }
        });

        self.log_audit_event("OAuth Token Refresh", &event).await;
        
        if success {
            info!("✅ [AUDIT] OAuth token refresh successful for server: {}", server_name);
        } else {
            warn!("⚠️ [AUDIT] OAuth token refresh failed for server: {} - {}", server_name, error.unwrap_or("unknown"));
        }
    }

    /// Log security policy violation
    pub async fn log_security_policy_violation(&self, server_name: &str, violation_type: &str, details: &str) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "security_policy_violation",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "violation_type": violation_type,
            "details": details,
            "action": "policy_violation_detected",
            "severity": "warning"
        });

        self.log_audit_event("Security Policy Violation", &event).await;
        warn!("⚠️ [AUDIT] Security policy violation for server: {} - {} ({})", server_name, violation_type, details);
    }

    /// Log credential storage event
    pub async fn log_credential_storage(&self, server_name: &str, storage_type: &str, success: bool) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "credential_storage",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "storage_type": storage_type,
            "success": success,
            "action": if success { "credentials_stored" } else { "credentials_storage_failed" }
        });

        self.log_audit_event("Credential Storage", &event).await;
        
        if success {
            info!("💾 [AUDIT] Credentials stored for server: {} (type: {})", server_name, storage_type);
        } else {
            error!("❌ [AUDIT] Failed to store credentials for server: {} (type: {})", server_name, storage_type);
        }
    }

    /// Log MCP connection establishment
    pub async fn log_mcp_connection_established(&self, server_name: &str, connection_type: &str, authenticated: bool) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "mcp_connection_established",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "connection_type": connection_type,
            "authenticated": authenticated,
            "action": "connection_established"
        });

        self.log_audit_event("MCP Connection Established", &event).await;
        info!("🔗 [AUDIT] MCP connection established for server: {} (type: {}, auth: {})", 
              server_name, connection_type, authenticated);
    }

    /// Log MCP tool execution with OAuth context
    pub async fn log_mcp_tool_execution(&self, server_name: &str, tool_name: &str, authenticated: bool, success: bool) {
        if !self.enabled {
            return;
        }

        let event = json!({
            "event": "mcp_tool_execution",
            "timestamp": Utc::now().to_rfc3339(),
            "server_name": server_name,
            "tool_name": tool_name,
            "authenticated": authenticated,
            "success": success,
            "action": "tool_executed"
        });

        self.log_audit_event("MCP Tool Execution", &event).await;
        
        if success {
            info!("🔧 [AUDIT] Tool executed: {} on server: {} (auth: {})", tool_name, server_name, authenticated);
        } else {
            error!("❌ [AUDIT] Tool execution failed: {} on server: {} (auth: {})", tool_name, server_name, authenticated);
        }
    }

    /// Internal method to log audit events
    async fn log_audit_event(&self, event_type: &str, event_data: &Value) {
        if self.structured_logging {
            // Log as structured JSON for audit systems
            info!(target: "audit", "{}: {}", event_type, event_data);
        } else {
            // Log as human-readable format
            info!("[AUDIT] {}: {}", event_type, event_data);
        }
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(true, true)
    }
}