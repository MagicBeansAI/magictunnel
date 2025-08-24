//! Threat Detection Engine configuration and service for MagicTunnel
//!
//! Provides behavioral anomaly detection and threat monitoring.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Threat Detection Engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionConfig {
    /// Whether Threat Detection is enabled
    pub enabled: bool,
}

impl Default for ThreatDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,                           // Enabled by default
        }
    }
}

/// Threat action enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreatAction {
    /// Allow and log the threat
    LogAndAllow,
    /// Block the request
    Block,
    /// Quarantine the user/session
    Quarantine,
    /// Alert administrators
    Alert,
    /// Rate limit the source
    RateLimit,
}

/// Threat severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatSeverity {
    /// Low severity threat
    Low,
    /// Medium severity threat
    Medium,
    /// High severity threat
    High,
    /// Critical severity threat
    Critical,
}

impl ThreatDetectionConfig {
    /// Validate the threat detection configuration
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

/// Threat Detection Service Statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionStatistics {
    /// Total threat analyses performed
    pub total_analyses: u64,
    /// Threats detected by severity
    pub threats_by_severity: HashMap<String, u64>,
    /// False positives detected
    pub false_positives: u64,
    /// Rules triggered count
    pub rules_triggered: HashMap<String, u64>,
    /// Average analysis time in milliseconds
    pub avg_analysis_time_ms: f64,
    /// Active threat rules count
    pub active_rules_count: usize,
    /// Real-time monitoring status
    pub monitoring_active: bool,
    /// Service status
    pub service_status: String,
}

impl Default for ThreatDetectionStatistics {
    fn default() -> Self {
        let mut threats_by_severity = HashMap::new();
        threats_by_severity.insert("low".to_string(), 0);
        threats_by_severity.insert("medium".to_string(), 0);
        threats_by_severity.insert("high".to_string(), 0);
        threats_by_severity.insert("critical".to_string(), 0);
        
        Self {
            total_analyses: 0,
            threats_by_severity,
            false_positives: 0,
            rules_triggered: HashMap::new(),
            avg_analysis_time_ms: 0.0,
            active_rules_count: 0,
            monitoring_active: false,
            service_status: "Not Initialized".to_string(),
        }
    }
}