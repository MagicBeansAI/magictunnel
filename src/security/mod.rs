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
pub use allowlist::{AllowlistConfig, AllowlistService, AllowlistContext, AllowlistAction, ToolAllowlistRule};
pub use audit::{AuditConfig, AuditService, AuditEntry, AuditEventType, AuditUser, AuditRequest, AuditTool, AuditResource, AuditSecurity, AuditOutcome, AuditError};
pub use config::SecurityConfig;
pub use middleware::{SecurityMiddleware, SecurityContext, SecurityRequest, SecurityUser, SecurityTool, SecurityResult, extract_security_user};
pub use policy::{PolicyConfig, PolicyEngine, PolicyContext, PolicyUser};
pub use rbac::{RbacConfig, RbacService, PermissionContext};
pub use sanitization::{SanitizationConfig, SanitizationService};