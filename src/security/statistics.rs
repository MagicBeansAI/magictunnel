//! Security service statistics and health monitoring
//!
//! This module provides common types and interfaces for monitoring
//! the health and performance of all security services.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ============================================================================
// Common Health and Status Types
// ============================================================================

/// Health status of a security service
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Error,
    Disabled,
}

/// Detailed health information for a security service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Overall health status
    pub status: HealthStatus,
    /// Whether the service is currently healthy (operational)
    pub is_healthy: bool,
    /// Last health check timestamp
    pub last_checked: DateTime<Utc>,
    /// Current error message (if any)
    pub error_message: Option<String>,
    /// Service uptime in seconds
    pub uptime_seconds: u64,
    /// Performance metrics
    pub performance: PerformanceMetrics,
}

/// Performance metrics common to all services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Requests per second
    pub requests_per_second: f64,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
}

// ============================================================================
// Service-Specific Statistics Types
// ============================================================================

/// Statistics for the allowlist service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistStatistics {
    /// Service health information
    pub health: ServiceHealth,
    /// Total number of allowlist rules
    pub total_rules: u32,
    /// Number of active (enabled) rules
    pub active_rules: u32,
    /// Total requests processed
    pub total_requests: u64,
    /// Requests that were allowed
    pub allowed_requests: u64,
    /// Requests that were blocked
    pub blocked_requests: u64,
    /// Requests that required approval
    pub approval_required_requests: u64,
    /// Top matched rules
    pub top_matched_rules: Vec<RuleMatch>,
    /// Request patterns by hour (last 24 hours)
    pub hourly_patterns: Vec<HourlyMetric>,
}

/// Statistics for the RBAC service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacStatistics {
    /// Service health information
    pub health: ServiceHealth,
    /// Total number of roles
    pub total_roles: u32,
    /// Total number of users
    pub total_users: u32,
    /// Total number of permissions
    pub total_permissions: u32,
    /// Currently active sessions
    pub active_sessions: u32,
    /// Total authentication attempts
    pub total_auth_attempts: u64,
    /// Successful authentications
    pub successful_auth: u64,
    /// Failed authentications
    pub failed_auth: u64,
    /// Permission evaluation metrics
    pub permission_evaluations: u64,
    /// Top active roles
    pub top_roles: Vec<RoleUsage>,
}

/// Statistics for the audit service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    /// Service health information
    pub health: ServiceHealth,
    /// Total audit entries
    #[serde(rename = "totalEntries")]
    pub total_entries: u64,
    /// Entries added today
    #[serde(rename = "entries_today")]
    pub entries_today: u64,
    /// Security-related events
    #[serde(rename = "security_events")]
    pub security_events: u64,
    /// Violations detected (mapped to violations_today for now)
    #[serde(rename = "violations")]
    pub violations_today: u64,
    /// Critical violations (requiring immediate attention)
    #[serde(rename = "critical_violations")]
    pub critical_violations: u64,
    /// Storage size in bytes
    #[serde(rename = "storage_size_bytes")]
    pub storage_size_bytes: u64,
    /// Average entries per day (last 30 days)
    #[serde(rename = "avg_entries_per_day")]
    pub avg_entries_per_day: f64,
    /// Top event types
    #[serde(rename = "eventTypes")]
    pub top_event_types: Vec<EventTypeCount>,
    /// Auth events (computed field)
    #[serde(rename = "authEvents")]
    pub auth_events: u64,
    /// Failed auth attempts (computed field)
    #[serde(rename = "failedAuth")]
    pub failed_auth: u64,
    /// Unique users (computed field)
    #[serde(rename = "uniqueUsers")]
    pub unique_users: u64,
}

impl Default for AuditStatistics {
    fn default() -> Self {
        Self {
            health: ServiceHealth::default(),
            total_entries: 0,
            entries_today: 0,
            security_events: 0,
            violations_today: 0,
            critical_violations: 0,
            storage_size_bytes: 0,
            avg_entries_per_day: 0.0,
            top_event_types: Vec::new(),
            auth_events: 0,
            failed_auth: 0,
            unique_users: 0,
        }
    }
}

/// Statistics for the sanitization service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationStatistics {
    /// Service health information
    pub health: ServiceHealth,
    /// Total sanitization policies
    #[serde(rename = "totalPolicies")]
    pub total_policies: u32,
    /// Active policies
    #[serde(rename = "activePolicies")]
    pub active_policies: u32,
    /// Total requests processed
    #[serde(rename = "totalRequests")]
    pub total_requests: u64,
    /// Requests that were sanitized
    #[serde(rename = "sanitizedRequests")]
    pub sanitized_requests: u64,
    /// Requests that were blocked
    #[serde(rename = "blockedRequests")]
    pub blocked_requests: u64,
    /// Alerts generated
    #[serde(rename = "alertsGenerated")]
    pub alerts_generated: u64,
    /// Secrets detected and blocked
    #[serde(rename = "secretsDetected")]
    pub secrets_detected: u64,
    /// Policy effectiveness (detection rate)
    #[serde(rename = "detectionRate")]
    pub detection_rate: f64,
    /// Top triggered policies
    #[serde(rename = "topPolicies")]
    pub top_policies: Vec<PolicyTrigger>,
}

impl Default for SanitizationStatistics {
    fn default() -> Self {
        Self {
            health: ServiceHealth::default(),
            total_policies: 0,
            active_policies: 0,
            total_requests: 0,
            sanitized_requests: 0,
            blocked_requests: 0,
            alerts_generated: 0,
            secrets_detected: 0,
            detection_rate: 0.0,
            top_policies: Vec::new(),
        }
    }
}


// ============================================================================
// Supporting Types
// ============================================================================

/// Rule match information for allowlist statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMatch {
    /// Rule name/pattern
    pub rule_name: String,
    /// Number of times this rule was matched
    pub match_count: u64,
    /// Action taken (allow/deny/require_approval)
    pub action: String,
    /// Last time this rule was matched
    pub last_matched: DateTime<Utc>,
}

/// Hourly metric for time-based analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyMetric {
    /// Hour timestamp
    pub hour: DateTime<Utc>,
    /// Number of requests in this hour
    pub request_count: u64,
    /// Number of blocked requests
    pub blocked_count: u64,
    /// Number of allowed requests
    pub allowed_count: u64,
}

/// Role usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleUsage {
    /// Role name
    pub role_name: String,
    /// Number of users with this role
    pub user_count: u32,
    /// Number of active sessions with this role
    pub active_sessions: u32,
    /// Last time this role was used
    pub last_used: DateTime<Utc>,
}

/// Event type count for audit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTypeCount {
    /// Event type name
    #[serde(rename = "type")]
    pub event_type: String,
    /// Number of events of this type
    pub count: u64,
    /// Percentage of total events
    pub percentage: f64,
}

/// Policy trigger information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyTrigger {
    /// Policy name
    pub policy_name: String,
    /// Policy type
    pub policy_type: String,
    /// Number of times triggered
    pub trigger_count: u64,
    /// Action taken (sanitize/block/warn/log)
    pub action: String,
    /// Effectiveness rate (0.0 - 1.0)
    pub effectiveness_rate: f64,
}


// ============================================================================
// Common Traits and Interfaces
// ============================================================================

/// Trait that all security services must implement for statistics
pub trait SecurityServiceStatistics {
    /// The specific statistics type for this service
    type Statistics;
    
    /// Get current service statistics
    async fn get_statistics(&self) -> Self::Statistics;
    
    /// Get current service health
    async fn get_health(&self) -> ServiceHealth;
    
    /// Reset statistics (for testing/maintenance)
    async fn reset_statistics(&self) -> Result<(), Box<dyn std::error::Error>>;
}

/// Trait for services that can be monitored for health
pub trait HealthMonitor {
    /// Check if the service is healthy
    async fn is_healthy(&self) -> bool;
    
    /// Get detailed health information
    async fn health_check(&self) -> ServiceHealth;
    
    /// Get service uptime in seconds
    fn get_uptime(&self) -> u64;
}

// ============================================================================
// Utility Functions
// ============================================================================

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            avg_response_time_ms: 0.0,
            requests_per_second: 0.0,
            error_rate: 0.0,
            memory_usage_bytes: 0,
        }
    }
}

impl Default for ServiceHealth {
    fn default() -> Self {
        Self {
            status: HealthStatus::Disabled,
            is_healthy: false,
            last_checked: Utc::now(),
            error_message: None,
            uptime_seconds: 0,
            performance: PerformanceMetrics::default(),
        }
    }
}

impl ServiceHealth {
    /// Create a healthy service health instance
    pub fn healthy(uptime_seconds: u64) -> Self {
        Self {
            status: HealthStatus::Healthy,
            is_healthy: true,
            last_checked: Utc::now(),
            error_message: None,
            uptime_seconds,
            performance: PerformanceMetrics::default(),
        }
    }
    
    /// Create an error service health instance
    pub fn error(error_message: String) -> Self {
        Self {
            status: HealthStatus::Error,
            is_healthy: false,
            last_checked: Utc::now(),
            error_message: Some(error_message),
            uptime_seconds: 0,
            performance: PerformanceMetrics::default(),
        }
    }
    
    /// Create a disabled service health instance
    pub fn disabled() -> Self {
        Self {
            status: HealthStatus::Disabled,
            is_healthy: false,
            last_checked: Utc::now(),
            error_message: None,
            uptime_seconds: 0,
            performance: PerformanceMetrics::default(),
        }
    }
}

/// Calculate percentage safely
pub fn safe_percentage(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

/// Calculate rate safely
pub fn safe_rate(count: u64, duration_seconds: u64) -> f64 {
    if duration_seconds == 0 {
        0.0
    } else {
        count as f64 / duration_seconds as f64
    }
}