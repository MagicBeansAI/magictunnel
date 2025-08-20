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
pub mod allowlist_data;
pub mod audit;
pub mod change_integration;
pub mod change_tracker;
pub mod config;
pub mod emergency;
pub mod middleware;
pub mod rbac;
pub mod sanitization;
pub mod statistics;
pub mod audit_log;

// Re-export specific types to avoid conflicts
pub use allowlist_types::{AllowlistConfig, AllowlistContext, AllowlistAction, AllowlistRule, AllowlistPattern};

pub use allowlist_data::{
    AllowlistData, AllowlistDecision, RuleSource, DecisionAuditTrail, 
    ToolWithAllowlistStatus, PatternTestRequest, PatternTestResponse,
    AllowlistMetadata, AllowlistPatterns, PatternRule, ExplicitRules,
    AllowlistSummary, TestPattern, PatternScope, 
    RealTimePatternTestRequest, RealTimePatternTestResponse, PatternToolTestResult,
    PatternEvaluationStep, RealTimePatternTestSummary
};
pub use allowlist::{AllowlistService};
pub use audit::{AuditConfig, AuditService, AuditEntry, AuditEventType, AuditUser, AuditRequest, AuditTool, AuditResource, AuditSecurity, AuditOutcome, AuditError, AuditQueryFilters};
pub use config::SecurityConfig;
pub use middleware::{SecurityMiddleware, SecurityContext, SecurityRequest, SecurityUser, SecurityTool, SecurityResult, extract_security_user};
pub use rbac::{RbacConfig, RbacService, PermissionContext};
pub use sanitization::{SanitizationConfig, SanitizationService};
pub use statistics::{SecurityServiceStatistics, HealthMonitor, ServiceHealth, HealthStatus, AllowlistStatistics, RbacStatistics, AuditStatistics, SanitizationStatistics};
pub use emergency::{EmergencyLockdownManager, EmergencyLockdownConfig, EmergencyLockdownState, EmergencyLockdownResult, EmergencyLockdownStatistics};
pub use change_tracker::{ConfigurationChangeTracker, ChangeTrackerConfig, ConfigurationChange, ChangeType, ChangeOperation, ChangeUser, ChangeTarget, ChangeDiff, ChangeImpact, ChangeValidation, ChangeListener, ChangeTrackingStatistics};
pub use change_integration::{ChangeTrackingIntegration, AllowlistChangeListener, ConfigFileWatcher};
pub use audit_log::AuditLogger;