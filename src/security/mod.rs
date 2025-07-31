//! Security module for MagicTunnel
//!
//! This module provides comprehensive security features including:
//! - Tool allowlisting and access control
//! - Request sanitization and content filtering
//! - Audit logging and security monitoring
//! - Role-based access control (RBAC)
//! - Policy engine for organization-wide security rules

pub mod allowlist;
pub mod audit;
pub mod config;
pub mod middleware;
pub mod policy;
pub mod rbac;
pub mod sanitization;

// Re-export specific types to avoid conflicts
pub use allowlist::{AllowlistConfig, AllowlistService, AllowlistContext, AllowlistResult, AllowlistAction, ToolAllowlistRule, ResourceAllowlistRule, PromptAllowlistRule, GlobalAllowlistRule, ParameterRules, RateLimit};
pub use audit::{AuditConfig, AuditService, AuditEntry, AuditEventType, AuditUser, AuditRequest, AuditResponse, AuditTool, AuditResource, AuditSecurity, AuditOutcome, AuditError, AuditQueryFilters, AuditStorage};
pub use config::SecurityConfig;
pub use middleware::{SecurityMiddleware, SecurityContext, SecurityUser, SecurityRequest, SecurityTool, SecurityResource, SecurityResult, SecurityEvent, SecuritySeverity, extract_security_user};
pub use policy::{PolicyConfig, PolicyEngine, PolicyContext, PolicyResult, PolicyCondition, PolicyAction, PolicyUser};
pub use rbac::{RbacConfig, RbacService, PermissionContext, PermissionResult, Role};
pub use sanitization::{SanitizationConfig, SanitizationService, SanitizationResult, SanitizationPolicy, SanitizationTrigger, SanitizationAction, SanitizationMethod, SecretType, HashAlgorithm, ApprovalWorkflow};