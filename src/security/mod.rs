//! Security module for MagicTunnel
//!
//! This module provides comprehensive security features including:
//! - Tool allowlisting and access control
//! - Request sanitization and content filtering
//! - Audit logging and security monitoring
//! - Role-based access control (RBAC)
//! - Policy engine for organization-wide security rules

pub mod allowlist;
pub mod allowlist_types;
pub mod audit;
pub mod change_integration;
pub mod change_tracker;
pub mod config;
pub mod emergency;
pub mod middleware;
pub mod pattern_loader;
pub mod rbac;
pub mod sanitization;
pub mod statistics;

// Re-export specific types to avoid conflicts
pub use allowlist_types::{AllowlistConfig, AllowlistContext, AllowlistAction, AllowlistRule, AllowlistResult, RuleLevel, AllowlistPattern, PatternRule};
pub use allowlist::{AllowlistService};
pub use audit::{AuditConfig, AuditService, AuditEntry, AuditEventType, AuditUser, AuditRequest, AuditTool, AuditResource, AuditSecurity, AuditOutcome, AuditError, AuditQueryFilters};
pub use config::SecurityConfig;
pub use middleware::{SecurityMiddleware, SecurityContext, SecurityRequest, SecurityUser, SecurityTool, SecurityResult, extract_security_user};
pub use rbac::{RbacConfig, RbacService, PermissionContext};
pub use sanitization::{SanitizationConfig, SanitizationService};
pub use statistics::{SecurityServiceStatistics, HealthMonitor, ServiceHealth, HealthStatus, AllowlistStatistics, RbacStatistics, AuditStatistics, SanitizationStatistics};
pub use emergency::{EmergencyLockdownManager, EmergencyLockdownConfig, EmergencyLockdownState, EmergencyLockdownResult, EmergencyLockdownStatistics};
pub use pattern_loader::{PatternLoader, PatternTestResults, PatternTestResult};
pub use change_tracker::{ConfigurationChangeTracker, ChangeTrackerConfig, ConfigurationChange, ChangeType, ChangeOperation, ChangeUser, ChangeTarget, ChangeDiff, ChangeImpact, ChangeValidation, ChangeListener, ChangeTrackingStatistics};
pub use change_integration::{ChangeTrackingIntegration, AllowlistChangeListener, ConfigFileWatcher};