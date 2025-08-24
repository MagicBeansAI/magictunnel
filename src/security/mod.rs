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
pub mod policy_engine;
pub mod rbac;
pub mod sanitization;
pub mod statistics;
pub mod threat_detection;

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

// Modern centralized audit system
pub use audit::{
    AuditConfig,
    AuditCollector, 
    AuditEvent, 
    AuditEventType, 
    AuditMetadata,
    AuditStorage,
    AuditStreamer,
    initialize_audit_system,
    get_audit_collector,
    AuditService,
    AuditEntry,
    AuditOutcome,
    AuditUser,
    AuditQuery,
};
pub use audit::events::AuditSeverity;

pub use config::SecurityConfig;
pub use middleware::{SecurityMiddleware, SecurityContext, SecurityRequest, SecurityUser, SecurityTool, SecurityResult, extract_security_user};
pub use rbac::{RbacConfig, RbacService, PermissionContext};
pub use sanitization::{SanitizationConfig, SanitizationService};
pub use statistics::{SecurityServiceStatistics, HealthMonitor, ServiceHealth, HealthStatus, AllowlistStatistics, RbacStatistics, AuditStatistics, SanitizationStatistics};
pub use emergency::{EmergencyLockdownManager, EmergencyLockdownConfig, EmergencyLockdownState, EmergencyLockdownResult, EmergencyLockdownStatistics};
pub use change_tracker::{ConfigurationChangeTracker, ChangeTrackerConfig, ConfigurationChange, ChangeType, ChangeOperation, ChangeUser, ChangeTarget, ChangeDiff, ChangeImpact, ChangeValidation, ChangeListener, ChangeTrackingStatistics};
pub use change_integration::{ChangeTrackingIntegration, AllowlistChangeListener, ConfigFileWatcher};
pub use policy_engine::{PolicyEngineConfig, PolicyAction, PolicyEngineStatistics};
pub use threat_detection::{ThreatDetectionConfig, ThreatAction, ThreatSeverity, ThreatDetectionStatistics};