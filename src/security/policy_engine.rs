//! Policy Engine configuration and service for MagicTunnel
//!
//! Provides business rule evaluation layer for security policies.

use serde::{Deserialize, Serialize};

/// Policy Engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEngineConfig {
    /// Whether Policy Engine is enabled
    pub enabled: bool,
}

impl Default for PolicyEngineConfig {
    fn default() -> Self {
        Self {
            enabled: true,                           // Enabled by default
        }
    }
}

/// Policy action enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyAction {
    /// Allow the request
    Allow,
    /// Deny the request
    Deny,
    /// Require additional approval
    RequireApproval,
    /// Log and allow (monitoring mode)
    LogAndAllow,
}

impl PolicyEngineConfig {
    /// Validate the policy engine configuration
    pub fn validate(&self) -> Result<(), String> {
        // Configuration validation
        Ok(())
    }
    
    /// Get service status description
    pub fn get_status_description(&self) -> String {
        if !self.enabled {
            "Disabled".to_string()
        } else {
            "Enabled".to_string()
        }
    }
}

/// Policy Engine Service Statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEngineStatistics {
    /// Total policy evaluations
    pub total_evaluations: u64,
    /// Successful evaluations
    pub successful_evaluations: u64,
    /// Failed evaluations
    pub failed_evaluations: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Average evaluation time in milliseconds
    pub avg_evaluation_time_ms: f64,
    /// Active policies count
    pub active_policies_count: usize,
    /// Service status
    pub service_status: String,
}

impl Default for PolicyEngineStatistics {
    fn default() -> Self {
        Self {
            total_evaluations: 0,
            successful_evaluations: 0,
            failed_evaluations: 0,
            cache_hits: 0,
            cache_misses: 0,
            avg_evaluation_time_ms: 0.0,
            active_policies_count: 0,
            service_status: "Not Initialized".to_string(),
        }
    }
}