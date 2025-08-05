//! Security configuration for MagicTunnel
//!
//! Unified configuration structure for all security components

use serde::{Deserialize, Serialize};
use super::{AllowlistConfig, SanitizationConfig, RbacConfig, PolicyConfig, AuditConfig};

/// Comprehensive security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Whether security features are enabled globally
    pub enabled: bool,
    /// Tool allowlisting configuration
    pub allowlist: Option<AllowlistConfig>,
    /// Request sanitization configuration
    pub sanitization: Option<SanitizationConfig>,
    /// Role-based access control configuration
    pub rbac: Option<RbacConfig>,
    /// Organization-wide policy configuration
    pub policies: Option<PolicyConfig>,
    /// Audit logging configuration
    pub audit: Option<AuditConfig>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: false,                    // Enterprise security features remain opt-in
            allowlist: None,                   // Enterprise allowlisting opt-in
            sanitization: None,                // Enterprise sanitization opt-in
            rbac: None,                        // Enterprise RBAC opt-in
            policies: None,                    // Enterprise policies opt-in
            audit: None,                       // Enterprise audit logging opt-in
        }
    }
}

impl SecurityConfig {

    /// Create a secure default configuration
    pub fn secure_defaults() -> Self {
        Self {
            enabled: true,
            allowlist: Some(AllowlistConfig {
                enabled: true,
                default_action: super::AllowlistAction::Deny,
                ..Default::default()
            }),
            sanitization: Some(SanitizationConfig {
                enabled: true,
                ..Default::default()
            }),
            rbac: Some(RbacConfig {
                enabled: true,
                ..Default::default()
            }),
            policies: Some(PolicyConfig {
                enabled: true,
                ..Default::default()
            }),
            audit: Some(AuditConfig {
                enabled: true,
                ..Default::default()
            }),
        }
    }
    
    /// Check if any security feature is enabled
    pub fn has_any_enabled(&self) -> bool {
        if !self.enabled {
            return false;
        }
        
        self.allowlist.as_ref().map_or(false, |c| c.enabled) ||
        self.sanitization.as_ref().map_or(false, |c| c.enabled) ||
        self.rbac.as_ref().map_or(false, |c| c.enabled) ||
        self.policies.as_ref().map_or(false, |c| c.enabled) ||
        self.audit.as_ref().map_or(false, |c| c.enabled) ||
        false // MCP 2025-06-18 security removed - see TODO.md
    }
}

