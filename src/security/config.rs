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
    /// MCP 2025-06-18 specific security enhancements
    pub mcp_2025_security: Option<Mcp2025SecurityConfig>,
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
            mcp_2025_security: Some(Mcp2025SecurityConfig::default()), // ✅ MCP protocol security always enabled
        }
    }
}

impl SecurityConfig {
    /// Verify that MCP 2025-06-18 security is enabled by default
    pub fn has_mcp_security(&self) -> bool {
        self.mcp_2025_security.is_some()
    }

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
            mcp_2025_security: Some(Mcp2025SecurityConfig {
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
        self.mcp_2025_security.as_ref().map_or(false, |c| c.enabled)
    }
}

/// MCP 2025-06-18 specific security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mcp2025SecurityConfig {
    /// Enable MCP 2025-06-18 security enhancements
    pub enabled: bool,
    /// Enhanced user consent flows for sampling/elicitation operations
    pub enhanced_consent: McpConsentConfig,
    /// Granular permission controls for MCP capabilities
    pub capability_permissions: McpCapabilityPermissionsConfig,
    /// Tool execution approval workflows
    pub tool_approval: McpToolApprovalConfig,
    /// OAuth 2.1 security enhancements
    pub oauth_21_enhancements: McpOAuth21Config,
    /// Resource Indicators security settings
    pub resource_indicators: McpResourceIndicatorsConfig,
}

/// MCP user consent configuration for sampling/elicitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConsentConfig {
    /// Require explicit consent for sampling operations
    pub require_sampling_consent: bool,
    /// Require explicit consent for elicitation operations  
    pub require_elicitation_consent: bool,
    /// Consent timeout in seconds
    pub consent_timeout_seconds: u64,
    /// Enable consent audit logging
    pub audit_consent_decisions: bool,
    /// Minimum consent level required (none, basic, explicit, informed)
    pub minimum_consent_level: ConsentLevel,
}

/// Granular permission controls for MCP capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapabilityPermissionsConfig {
    /// Require explicit permission for sampling capability
    pub sampling_permission_required: bool,
    /// Require explicit permission for elicitation capability
    pub elicitation_permission_required: bool,
    /// Require explicit permission for roots capability
    pub roots_permission_required: bool,
    /// Default permission behavior (allow, deny, ask)
    pub default_permission_behavior: PermissionBehavior,
    /// Enable permission caching
    pub cache_permissions: bool,
    /// Permission cache TTL in seconds
    pub permission_cache_ttl_seconds: u64,
}

/// Tool execution approval workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolApprovalConfig {
    /// Enable tool execution approval workflow
    pub enabled: bool,
    /// Require approval for high-risk tools
    pub require_approval_for_high_risk: bool,
    /// Approval timeout in seconds
    pub approval_timeout_seconds: u64,
    /// Enable multi-step approval for critical operations
    pub enable_multi_step_approval: bool,
    /// Tools that always require approval (regex patterns)
    pub always_require_approval: Vec<String>,
    /// Tools that never require approval (regex patterns)
    pub never_require_approval: Vec<String>,
    /// Approval notification methods
    pub notification_methods: Vec<ApprovalNotificationMethod>,
}

/// OAuth 2.1 security enhancements configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpOAuth21Config {
    /// Enforce OAuth 2.1 compliance
    pub enforce_oauth_21: bool,
    /// Require PKCE for all flows
    pub require_pkce: bool,
    /// Validate Resource Indicators
    pub validate_resource_indicators: bool,
    /// Enable token binding
    pub enable_token_binding: bool,
    /// Maximum token lifetime in seconds
    pub max_token_lifetime_seconds: u64,
    /// Enable token rotation
    pub enable_token_rotation: bool,
}

/// Resource Indicators security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceIndicatorsConfig {
    /// Enforce Resource Indicators validation
    pub enforce_validation: bool,
    /// Require explicit resource specification
    pub require_explicit_resources: bool,
    /// Enable resource scope validation
    pub validate_resource_scopes: bool,
    /// Maximum number of resources per request
    pub max_resources_per_request: usize,
    /// Enable cross-resource access controls
    pub enable_cross_resource_controls: bool,
}

/// Consent levels for MCP operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsentLevel {
    /// No consent required
    None,
    /// Basic acknowledgment
    Basic,
    /// Explicit opt-in required
    Explicit,
    /// Informed consent with full disclosure
    Informed,
}

/// Permission behavior for capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionBehavior {
    /// Allow by default
    Allow,
    /// Deny by default
    Deny,
    /// Ask user for permission
    Ask,
}

/// Approval notification methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalNotificationMethod {
    /// Email notification
    Email,
    /// Slack notification
    Slack,
    /// Webhook notification
    Webhook,
    /// In-app notification
    InApp,
}

impl Default for Mcp2025SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enhanced_consent: McpConsentConfig::default(),
            capability_permissions: McpCapabilityPermissionsConfig::default(),
            tool_approval: McpToolApprovalConfig::default(),
            oauth_21_enhancements: McpOAuth21Config::default(),
            resource_indicators: McpResourceIndicatorsConfig::default(),
        }
    }
}

impl Default for McpConsentConfig {
    fn default() -> Self {
        Self {
            require_sampling_consent: true,
            require_elicitation_consent: true,
            consent_timeout_seconds: 300, // 5 minutes
            audit_consent_decisions: true,
            minimum_consent_level: ConsentLevel::Explicit,
        }
    }
}

impl Default for McpCapabilityPermissionsConfig {
    fn default() -> Self {
        Self {
            sampling_permission_required: true,
            elicitation_permission_required: true,
            roots_permission_required: false,
            default_permission_behavior: PermissionBehavior::Ask,
            cache_permissions: true,
            permission_cache_ttl_seconds: 3600, // 1 hour
        }
    }
}

impl Default for McpToolApprovalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_approval_for_high_risk: true,
            approval_timeout_seconds: 1800, // 30 minutes
            enable_multi_step_approval: false,
            always_require_approval: vec![
                ".*delete.*".to_string(),
                ".*remove.*".to_string(),
                ".*destroy.*".to_string(),
            ],
            never_require_approval: vec![
                "get.*".to_string(),
                "list.*".to_string(),
                "read.*".to_string(),
            ],
            notification_methods: vec![ApprovalNotificationMethod::InApp],
        }
    }
}

impl Default for McpOAuth21Config {
    fn default() -> Self {
        Self {
            enforce_oauth_21: true,
            require_pkce: true,
            validate_resource_indicators: true,
            enable_token_binding: false, // Optional advanced feature
            max_token_lifetime_seconds: 3600, // 1 hour
            enable_token_rotation: true,
        }
    }
}

impl Default for McpResourceIndicatorsConfig {
    fn default() -> Self {
        Self {
            enforce_validation: true,
            require_explicit_resources: false, // For backward compatibility
            validate_resource_scopes: true,
            max_resources_per_request: 10,
            enable_cross_resource_controls: true,
        }
    }
}