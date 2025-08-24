//! Security configuration for MagicTunnel
//!
//! Unified configuration structure for all security components

use serde::{Deserialize, Serialize};
use super::{AllowlistConfig, SanitizationConfig, RbacConfig, AuditConfig, EmergencyLockdownConfig, PolicyEngineConfig, ThreatDetectionConfig};

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
    /// Centralized audit system configuration
    pub audit: Option<AuditConfig>,
    /// Emergency lockdown configuration
    pub emergency_lockdown: Option<EmergencyLockdownConfig>,
    /// Policy Engine configuration
    pub policy_engine: Option<PolicyEngineConfig>,
    /// Threat Detection Engine configuration
    pub threat_detection: Option<ThreatDetectionConfig>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: false,                    // Enterprise security features remain opt-in
            allowlist: None,                   // Enterprise allowlisting opt-in
            sanitization: None,                // Enterprise sanitization opt-in
            rbac: None,                        // Enterprise RBAC opt-in
            audit: None,                       // Enterprise audit system opt-in
            emergency_lockdown: None,          // Enterprise emergency lockdown opt-in
            policy_engine: None,               // Production service - opt-in only
            threat_detection: None,            // Production service - opt-in only
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
            audit: Some(AuditConfig {
                enabled: true,
                ..Default::default()
            }),
            emergency_lockdown: Some(EmergencyLockdownConfig {
                enabled: true,
                ..Default::default()
            }),
            // Alpha services - disabled in secure defaults until tested
            policy_engine: Some(PolicyEngineConfig {
                enabled: false,    // Disabled for production
                ..Default::default()
            }),
            threat_detection: Some(ThreatDetectionConfig {
                enabled: false,    // Disabled for production
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
        self.audit.as_ref().map_or(false, |c| c.enabled) ||
        self.emergency_lockdown.as_ref().map_or(false, |c| c.enabled) ||
        self.policy_engine.as_ref().map_or(false, |c| c.enabled) ||
        self.threat_detection.as_ref().map_or(false, |c| c.enabled)
    }
    
    /// Get production security services status
    pub fn get_production_security_services(&self) -> Vec<(String, bool, String)> {
        let mut services = Vec::new();
        
        if let Some(policy_engine) = &self.policy_engine {
            services.push((
                "Policy Engine".to_string(),
                policy_engine.enabled,
                policy_engine.get_status_description(),
            ));
        }
        
        if let Some(threat_detection) = &self.threat_detection {
            services.push((
                "Threat Detection".to_string(),
                threat_detection.enabled,
                threat_detection.get_status_description(),
            ));
        }
        
        services
    }
}

