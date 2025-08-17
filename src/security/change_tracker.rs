//! Configuration change tracking system for MagicTunnel security
//!
//! Tracks all changes to security configurations (rules, patterns, policies)
//! with before/after diffs, user attribution, and impact analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tokio::fs;
use std::path::PathBuf;
use tracing::{info, warn, error, debug};
use std::sync::{Arc, Mutex};

/// Configuration change tracking system
pub struct ConfigurationChangeTracker {
    config: ChangeTrackerConfig,
    changes: Arc<Mutex<Vec<ConfigurationChange>>>,
    change_listeners: Arc<Mutex<Vec<Box<dyn ChangeListener + Send + Sync>>>>,
}

/// Configuration for change tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeTrackerConfig {
    /// Whether change tracking is enabled
    pub enabled: bool,
    /// Directory to store change history
    pub storage_directory: PathBuf,
    /// Maximum number of changes to keep in memory
    pub max_memory_changes: usize,
    /// Whether to persist changes to disk
    pub persist_changes: bool,
    /// Whether to track rule-level changes
    pub track_rules: bool,
    /// Whether to track pattern changes
    pub track_patterns: bool,
    /// Whether to track policy changes
    pub track_policies: bool,
    /// Whether to generate impact analysis
    pub enable_impact_analysis: bool,
    /// File extensions to monitor for changes
    pub monitored_extensions: Vec<String>,
}

/// A single configuration change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationChange {
    /// Unique change ID
    pub id: String,
    /// Timestamp of the change
    pub timestamp: DateTime<Utc>,
    /// Type of change (rule, pattern, policy, emergency, etc.)
    pub change_type: ChangeType,
    /// Operation performed (create, update, delete)
    pub operation: ChangeOperation,
    /// User who made the change
    pub user: ChangeUser,
    /// Target of the change (file, rule name, etc.)
    pub target: ChangeTarget,
    /// Before state (JSON representation)
    pub before_state: Option<serde_json::Value>,
    /// After state (JSON representation)
    pub after_state: Option<serde_json::Value>,
    /// Computed diff between before and after
    pub diff: ChangeDiff,
    /// Impact analysis of the change
    pub impact: ChangeImpact,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Validation result of the change
    pub validation: ChangeValidation,
}

/// Type of configuration change
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChangeType {
    /// Tool-level allowlist rule change
    ToolRule { tool_name: String },
    /// Server-level allowlist rule change
    ServerRule { server_name: String },
    /// Capability pattern change
    CapabilityPattern { pattern_name: String },
    /// Global pattern change
    GlobalPattern { pattern_name: String },
    /// Security policy change
    Policy { policy_name: String },
    /// Emergency lockdown configuration
    EmergencyLockdown,
    /// RBAC role/permission change
    Rbac { role_name: Option<String> },
    /// Sanitization policy change
    Sanitization { policy_name: String },
    /// Audit configuration change
    AuditConfig,
    /// File-level change (when specific rule not identified)
    File { file_path: String },
}

/// Change operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeOperation {
    Create,
    Update,
    Delete,
    Enable,
    Disable,
    Move,
    Rename,
}

/// User who made the change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeUser {
    /// User ID
    pub id: Option<String>,
    /// Username or display name
    pub name: Option<String>,
    /// Authentication method used
    pub auth_method: String,
    /// API key name (if applicable)
    pub api_key_name: Option<String>,
    /// User roles at time of change
    pub roles: Vec<String>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
}

/// Target of the configuration change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeTarget {
    /// Target type (file, rule, pattern, etc.)
    pub target_type: String,
    /// Target identifier (file path, rule name, etc.)
    pub identifier: String,
    /// Parent context (e.g., file containing the rule)
    pub parent: Option<String>,
    /// Scope of the change (tool, capability, global, etc.)
    pub scope: String,
}

/// Computed diff between before and after states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDiff {
    /// Type of diff computation used
    pub diff_type: String,
    /// Number of additions
    pub additions: usize,
    /// Number of deletions
    pub deletions: usize,
    /// Number of modifications
    pub modifications: usize,
    /// Detailed field-level changes
    pub field_changes: Vec<FieldChange>,
    /// Human-readable summary
    pub summary: String,
}

/// Individual field change within a configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    /// Field path (e.g., "rule.action", "pattern.value")
    pub field_path: String,
    /// Previous value
    pub old_value: Option<serde_json::Value>,
    /// New value
    pub new_value: Option<serde_json::Value>,
    /// Type of change
    pub change_type: String, // added, removed, modified
}

/// Impact analysis of a configuration change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeImpact {
    /// Severity of the impact (low, medium, high, critical)
    pub severity: String,
    /// Number of tools potentially affected
    pub affected_tools_count: usize,
    /// List of specifically affected tools
    pub affected_tools: Vec<String>,
    /// Number of users potentially affected
    pub affected_users_count: Option<usize>,
    /// Security implications
    pub security_implications: Vec<String>,
    /// Performance implications
    pub performance_implications: Vec<String>,
    /// Rollback difficulty assessment
    pub rollback_difficulty: String,
    /// Recommended validation steps
    pub validation_steps: Vec<String>,
    /// Related changes that might be needed
    pub related_changes: Vec<String>,
}

/// Validation result of a configuration change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeValidation {
    /// Whether the change is valid
    pub is_valid: bool,
    /// Validation errors (if any)
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Syntax validation result
    pub syntax_valid: bool,
    /// Semantic validation result
    pub semantic_valid: bool,
    /// Conflict detection result
    pub conflicts: Vec<String>,
}

/// Trait for listening to configuration changes
pub trait ChangeListener {
    /// Called before a change is applied
    fn before_change(&self, change: &ConfigurationChange) -> Result<(), Box<dyn std::error::Error>>;
    /// Called after a change is applied
    fn after_change(&self, change: &ConfigurationChange) -> Result<(), Box<dyn std::error::Error>>;
    /// Called when a change fails
    fn change_failed(&self, change: &ConfigurationChange, error: &str);
}

/// Change tracking statistics
#[derive(Debug, Serialize)]
pub struct ChangeTrackingStatistics {
    /// Total number of changes tracked
    pub total_changes: usize,
    /// Changes by type
    pub changes_by_type: HashMap<String, usize>,
    /// Changes by operation
    pub changes_by_operation: HashMap<String, usize>,
    /// Changes by user
    pub changes_by_user: HashMap<String, usize>,
    /// Recent changes (last 24 hours)
    pub recent_changes: usize,
    /// Average impact severity
    pub average_impact_severity: f64,
    /// Most active files
    pub most_changed_files: Vec<(String, usize)>,
}

impl Default for ChangeTrackerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage_directory: PathBuf::from("./security/change_history"),
            max_memory_changes: 1000,
            persist_changes: true,
            track_rules: true,
            track_patterns: true,
            track_policies: true,
            enable_impact_analysis: true,
            monitored_extensions: vec![
                "yaml".to_string(),
                "yml".to_string(), 
                "json".to_string(),
                "toml".to_string(),
            ],
        }
    }
}

impl ConfigurationChangeTracker {
    /// Create a new configuration change tracker
    pub async fn new(config: ChangeTrackerConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Create storage directory if it doesn't exist
        if config.persist_changes {
            fs::create_dir_all(&config.storage_directory).await?;
        }

        let tracker = Self {
            config,
            changes: Arc::new(Mutex::new(Vec::new())),
            change_listeners: Arc::new(Mutex::new(Vec::new())),
        };

        // Load existing changes from disk if available
        if tracker.config.persist_changes {
            if let Err(e) = tracker.load_changes_from_disk().await {
                warn!("Failed to load existing changes from disk: {}", e);
            }
        }

        info!("Configuration change tracker initialized");
        Ok(tracker)
    }

    /// Track a configuration change
    pub async fn track_change(
        &self,
        change_type: ChangeType,
        operation: ChangeOperation,
        user: ChangeUser,
        target: ChangeTarget,
        before_state: Option<serde_json::Value>,
        after_state: Option<serde_json::Value>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if !self.config.enabled {
            return Ok("tracking_disabled".to_string());
        }

        let change_id = uuid::Uuid::new_v4().to_string();
        
        // Compute diff
        let diff = self.compute_diff(&before_state, &after_state);
        
        // Perform impact analysis
        let impact = if self.config.enable_impact_analysis {
            self.analyze_impact(&change_type, &operation, &before_state, &after_state).await
        } else {
            ChangeImpact {
                severity: "unknown".to_string(),
                affected_tools_count: 0,
                affected_tools: Vec::new(),
                affected_users_count: None,
                security_implications: Vec::new(),
                performance_implications: Vec::new(),
                rollback_difficulty: "unknown".to_string(),
                validation_steps: Vec::new(),
                related_changes: Vec::new(),
            }
        };

        // Validate the change
        let validation = self.validate_change(&change_type, &after_state);

        let change = ConfigurationChange {
            id: change_id.clone(),
            timestamp: Utc::now(),
            change_type,
            operation,
            user,
            target,
            before_state,
            after_state,
            diff,
            impact,
            metadata,
            validation,
        };

        // Notify listeners before applying change
        {
            let listeners = self.change_listeners.lock().unwrap();
            for listener in listeners.iter() {
                if let Err(e) = listener.before_change(&change) {
                    error!("Change listener failed (before): {}", e);
                }
            }
        }

        // Store the change
        {
            let mut changes = self.changes.lock().unwrap();
            changes.push(change.clone());
            
            // Trim to max size
            if changes.len() > self.config.max_memory_changes {
                let excess = changes.len() - self.config.max_memory_changes;
                changes.drain(0..excess);
            }
        }

        // Persist to disk if enabled
        if self.config.persist_changes {
            if let Err(e) = self.persist_change(&change).await {
                error!("Failed to persist change to disk: {}", e);
            }
        }

        // Notify listeners after applying change
        {
            let listeners = self.change_listeners.lock().unwrap();
            for listener in listeners.iter() {
                if let Err(e) = listener.after_change(&change) {
                    error!("Change listener failed (after): {}", e);
                }
            }
        }

        info!("Tracked configuration change: {} ({})", change_id, change.diff.summary);
        Ok(change_id)
    }

    /// Get all tracked changes
    pub fn get_changes(&self) -> Vec<ConfigurationChange> {
        self.changes.lock().unwrap().clone()
    }

    /// Get changes filtered by criteria
    pub fn get_changes_filtered(
        &self,
        change_type: Option<&str>,
        operation: Option<&str>,
        user_id: Option<&str>,
        since: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Vec<ConfigurationChange> {
        let changes = self.changes.lock().unwrap();
        let mut filtered: Vec<ConfigurationChange> = changes
            .iter()
            .filter(|change| {
                if let Some(since_time) = since {
                    if change.timestamp < since_time {
                        return false;
                    }
                }
                
                if let Some(ct) = change_type {
                    let matches = match &change.change_type {
                        ChangeType::ToolRule { .. } => ct == "tool_rule",
                        ChangeType::ServerRule { .. } => ct == "server_rule",
                        ChangeType::CapabilityPattern { .. } => ct == "capability_pattern",
                        ChangeType::GlobalPattern { .. } => ct == "global_pattern",
                        ChangeType::Policy { .. } => ct == "policy",
                        ChangeType::EmergencyLockdown => ct == "emergency_lockdown",
                        ChangeType::Rbac { .. } => ct == "rbac",
                        ChangeType::Sanitization { .. } => ct == "sanitization",
                        ChangeType::AuditConfig => ct == "audit_config",
                        ChangeType::File { .. } => ct == "file",
                    };
                    if !matches {
                        return false;
                    }
                }
                
                if let Some(op) = operation {
                    let op_str = match change.operation {
                        ChangeOperation::Create => "create",
                        ChangeOperation::Update => "update",
                        ChangeOperation::Delete => "delete",
                        ChangeOperation::Enable => "enable",
                        ChangeOperation::Disable => "disable",
                        ChangeOperation::Move => "move",
                        ChangeOperation::Rename => "rename",
                    };
                    if op_str != op {
                        return false;
                    }
                }
                
                if let Some(uid) = user_id {
                    if change.user.id.as_deref() != Some(uid) {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();

        // Sort by timestamp (newest first)
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit
        if let Some(limit) = limit {
            filtered.truncate(limit);
        }

        filtered
    }

    /// Get change tracking statistics
    pub fn get_statistics(&self) -> ChangeTrackingStatistics {
        let changes = self.changes.lock().unwrap();
        let mut stats = ChangeTrackingStatistics {
            total_changes: changes.len(),
            changes_by_type: HashMap::new(),
            changes_by_operation: HashMap::new(),
            changes_by_user: HashMap::new(),
            recent_changes: 0,
            average_impact_severity: 0.0,
            most_changed_files: Vec::new(),
        };

        let twenty_four_hours_ago = Utc::now() - chrono::Duration::hours(24);
        let mut severity_scores = Vec::new();
        let mut file_changes: HashMap<String, usize> = HashMap::new();

        for change in changes.iter() {
            // Count by type
            let type_key = match &change.change_type {
                ChangeType::ToolRule { .. } => "tool_rule",
                ChangeType::ServerRule { .. } => "server_rule",
                ChangeType::CapabilityPattern { .. } => "capability_pattern",
                ChangeType::GlobalPattern { .. } => "global_pattern",
                ChangeType::Policy { .. } => "policy",
                ChangeType::EmergencyLockdown => "emergency_lockdown",
                ChangeType::Rbac { .. } => "rbac",
                ChangeType::Sanitization { .. } => "sanitization",
                ChangeType::AuditConfig => "audit_config",
                ChangeType::File { .. } => "file",
            };
            *stats.changes_by_type.entry(type_key.to_string()).or_insert(0) += 1;

            // Count by operation
            let op_key = match change.operation {
                ChangeOperation::Create => "create",
                ChangeOperation::Update => "update",
                ChangeOperation::Delete => "delete",
                ChangeOperation::Enable => "enable",
                ChangeOperation::Disable => "disable",
                ChangeOperation::Move => "move",
                ChangeOperation::Rename => "rename",
            };
            *stats.changes_by_operation.entry(op_key.to_string()).or_insert(0) += 1;

            // Count by user
            if let Some(ref user_id) = change.user.id {
                *stats.changes_by_user.entry(user_id.clone()).or_insert(0) += 1;
            }

            // Count recent changes
            if change.timestamp > twenty_four_hours_ago {
                stats.recent_changes += 1;
            }

            // Collect severity scores
            let severity_score = match change.impact.severity.as_str() {
                "low" => 1.0,
                "medium" => 2.0,
                "high" => 3.0,
                "critical" => 4.0,
                _ => 0.0,
            };
            severity_scores.push(severity_score);

            // Count file changes
            if let ChangeType::File { file_path } = &change.change_type {
                *file_changes.entry(file_path.clone()).or_insert(0) += 1;
            }
        }

        // Calculate average severity
        if !severity_scores.is_empty() {
            stats.average_impact_severity = severity_scores.iter().sum::<f64>() / severity_scores.len() as f64;
        }

        // Get most changed files
        let mut file_changes_vec: Vec<(String, usize)> = file_changes.into_iter().collect();
        file_changes_vec.sort_by(|a, b| b.1.cmp(&a.1));
        stats.most_changed_files = file_changes_vec.into_iter().take(10).collect();

        stats
    }

    /// Add a change listener
    pub fn add_listener(&self, listener: Box<dyn ChangeListener + Send + Sync>) {
        let mut listeners = self.change_listeners.lock().unwrap();
        listeners.push(listener);
    }

    /// Compute diff between before and after states
    fn compute_diff(
        &self,
        before: &Option<serde_json::Value>,
        after: &Option<serde_json::Value>,
    ) -> ChangeDiff {
        let mut field_changes = Vec::new();
        let mut additions = 0;
        let mut deletions = 0;
        let mut modifications = 0;

        match (before, after) {
            (None, None) => {
                return ChangeDiff {
                    diff_type: "no_change".to_string(),
                    additions: 0,
                    deletions: 0,
                    modifications: 0,
                    field_changes,
                    summary: "No changes detected".to_string(),
                };
            }
            (None, Some(_)) => {
                additions = 1;
                field_changes.push(FieldChange {
                    field_path: "root".to_string(),
                    old_value: None,
                    new_value: after.clone(),
                    change_type: "added".to_string(),
                });
            }
            (Some(_), None) => {
                deletions = 1;
                field_changes.push(FieldChange {
                    field_path: "root".to_string(),
                    old_value: before.clone(),
                    new_value: None,
                    change_type: "removed".to_string(),
                });
            }
            (Some(before_val), Some(after_val)) => {
                if before_val != after_val {
                    modifications = 1;
                    field_changes.push(FieldChange {
                        field_path: "root".to_string(),
                        old_value: Some(before_val.clone()),
                        new_value: Some(after_val.clone()),
                        change_type: "modified".to_string(),
                    });
                }
            }
        }

        let summary = if additions > 0 && deletions > 0 {
            format!("{} additions, {} deletions, {} modifications", additions, deletions, modifications)
        } else if additions > 0 {
            format!("{} additions", additions)
        } else if deletions > 0 {
            format!("{} deletions", deletions)
        } else if modifications > 0 {
            format!("{} modifications", modifications)
        } else {
            "No changes".to_string()
        };

        ChangeDiff {
            diff_type: "json_comparison".to_string(),
            additions,
            deletions,
            modifications,
            field_changes,
            summary,
        }
    }

    /// Analyze the impact of a configuration change
    async fn analyze_impact(
        &self,
        change_type: &ChangeType,
        operation: &ChangeOperation,
        before_state: &Option<serde_json::Value>,
        after_state: &Option<serde_json::Value>,
    ) -> ChangeImpact {
        let mut impact = ChangeImpact {
            severity: "low".to_string(),
            affected_tools_count: 0,
            affected_tools: Vec::new(),
            affected_users_count: None,
            security_implications: Vec::new(),
            performance_implications: Vec::new(),
            rollback_difficulty: "easy".to_string(),
            validation_steps: Vec::new(),
            related_changes: Vec::new(),
        };

        // Analyze based on change type
        match change_type {
            ChangeType::ToolRule { tool_name } => {
                impact.affected_tools.push(tool_name.clone());
                impact.affected_tools_count = 1;
                impact.severity = "medium".to_string();
                impact.security_implications.push("Changes tool access permissions".to_string());
                impact.validation_steps.push("Test tool functionality".to_string());
            }
            ChangeType::CapabilityPattern { .. } | ChangeType::GlobalPattern { .. } => {
                impact.severity = "high".to_string();
                impact.affected_tools_count = 999; // Patterns can affect many tools
                impact.security_implications.push("May affect multiple tools via pattern matching".to_string());
                impact.validation_steps.push("Test pattern matching behavior".to_string());
                impact.validation_steps.push("Verify no unintended tool access changes".to_string());
            }
            ChangeType::EmergencyLockdown => {
                impact.severity = "critical".to_string();
                impact.affected_tools_count = 999;
                impact.security_implications.push("Affects all tool access during lockdown".to_string());
                impact.rollback_difficulty = "immediate".to_string();
            }
            ChangeType::Policy { .. } => {
                impact.severity = "medium".to_string();
                impact.security_implications.push("Changes security policy enforcement".to_string());
            }
            _ => {
                impact.severity = "low".to_string();
            }
        }

        // Analyze based on operation
        match operation {
            ChangeOperation::Delete => {
                impact.severity = "high".to_string();
                impact.security_implications.push("Deletion may remove security controls".to_string());
                impact.rollback_difficulty = "hard".to_string();
            }
            ChangeOperation::Disable => {
                impact.security_implications.push("Disabling may reduce security coverage".to_string());
            }
            _ => {}
        }

        impact
    }

    /// Validate a configuration change
    fn validate_change(
        &self,
        change_type: &ChangeType,
        after_state: &Option<serde_json::Value>,
    ) -> ChangeValidation {
        let mut validation = ChangeValidation {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            syntax_valid: true,
            semantic_valid: true,
            conflicts: Vec::new(),
        };

        // Basic validation based on change type
        match change_type {
            ChangeType::ToolRule { tool_name } => {
                if tool_name.is_empty() {
                    validation.errors.push("Tool name cannot be empty".to_string());
                    validation.is_valid = false;
                    validation.semantic_valid = false;
                }
            }
            ChangeType::CapabilityPattern { pattern_name } | ChangeType::GlobalPattern { pattern_name } => {
                if pattern_name.is_empty() {
                    validation.errors.push("Pattern name cannot be empty".to_string());
                    validation.is_valid = false;
                    validation.semantic_valid = false;
                }
                
                // Validate pattern syntax if available
                if let Some(state) = after_state {
                    if let Some(pattern_value) = state.get("pattern").and_then(|p| p.as_str()) {
                        if let Err(_) = regex::Regex::new(pattern_value) {
                            validation.errors.push("Invalid regex pattern syntax".to_string());
                            validation.is_valid = false;
                            validation.syntax_valid = false;
                        }
                    }
                }
            }
            _ => {}
        }

        validation
    }

    /// Persist a change to disk
    async fn persist_change(&self, change: &ConfigurationChange) -> Result<(), Box<dyn std::error::Error>> {
        let filename = format!("{}.json", change.id);
        let filepath = self.config.storage_directory.join(filename);
        
        let json_content = serde_json::to_string_pretty(change)?;
        fs::write(filepath, json_content).await?;
        
        Ok(())
    }

    /// Load changes from disk
    async fn load_changes_from_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.config.storage_directory.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&self.config.storage_directory).await?;
        let mut loaded_changes = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|s| s.to_str()).flatten() == Some("json") {
                match fs::read_to_string(&path).await {
                    Ok(content) => {
                        match serde_json::from_str::<ConfigurationChange>(&content) {
                            Ok(change) => loaded_changes.push(change),
                            Err(e) => warn!("Failed to parse change file {:?}: {}", path, e),
                        }
                    }
                    Err(e) => warn!("Failed to read change file {:?}: {}", path, e),
                }
            }
        }

        // Sort by timestamp and take the most recent ones
        loaded_changes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        loaded_changes.truncate(self.config.max_memory_changes);

        {
            let mut changes = self.changes.lock().unwrap();
            *changes = loaded_changes;
        }

        info!("Loaded {} changes from disk", self.changes.lock().unwrap().len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_change_tracker_creation() {
        let temp_dir = tempdir().unwrap();
        let config = ChangeTrackerConfig {
            storage_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let tracker = ConfigurationChangeTracker::new(config).await.unwrap();
        assert_eq!(tracker.get_changes().len(), 0);
    }

    #[tokio::test]
    async fn test_track_change() {
        let temp_dir = tempdir().unwrap();
        let config = ChangeTrackerConfig {
            storage_directory: temp_dir.path().to_path_buf(),
            persist_changes: false,
            ..Default::default()
        };

        let tracker = ConfigurationChangeTracker::new(config).await.unwrap();

        let change_id = tracker.track_change(
            ChangeType::ToolRule { tool_name: "test_tool".to_string() },
            ChangeOperation::Create,
            ChangeUser {
                id: Some("test_user".to_string()),
                name: Some("Test User".to_string()),
                auth_method: "api_key".to_string(),
                api_key_name: Some("test_key".to_string()),
                roles: vec!["admin".to_string()],
                client_ip: Some("127.0.0.1".to_string()),
                user_agent: Some("test_agent".to_string()),
            },
            ChangeTarget {
                target_type: "tool_rule".to_string(),
                identifier: "test_tool".to_string(),
                parent: Some("tools.yaml".to_string()),
                scope: "tool".to_string(),
            },
            None,
            Some(serde_json::json!({"action": "allow", "enabled": true})),
            HashMap::new(),
        ).await.unwrap();

        assert!(!change_id.is_empty());
        assert_eq!(tracker.get_changes().len(), 1);

        let changes = tracker.get_changes();
        let change = &changes[0];
        assert_eq!(change.user.id, Some("test_user".to_string()));
        assert!(change.validation.is_valid);
    }

    #[tokio::test]
    async fn test_change_filtering() {
        let temp_dir = tempdir().unwrap();
        let config = ChangeTrackerConfig {
            storage_directory: temp_dir.path().to_path_buf(),
            persist_changes: false,
            ..Default::default()
        };

        let tracker = ConfigurationChangeTracker::new(config).await.unwrap();

        // Add multiple changes
        for i in 0..3 {
            tracker.track_change(
                ChangeType::ToolRule { tool_name: format!("tool_{}", i) },
                ChangeOperation::Create,
                ChangeUser {
                    id: Some(format!("user_{}", i % 2)),
                    name: None,
                    auth_method: "api_key".to_string(),
                    api_key_name: None,
                    roles: Vec::new(),
                    client_ip: None,
                    user_agent: None,
                },
                ChangeTarget {
                    target_type: "tool_rule".to_string(),
                    identifier: format!("tool_{}", i),
                    parent: None,
                    scope: "tool".to_string(),
                },
                None,
                Some(serde_json::json!({"action": "allow"})),
                HashMap::new(),
            ).await.unwrap();
        }

        // Test filtering by user
        let user_0_changes = tracker.get_changes_filtered(
            None,
            None, 
            Some("user_0"),
            None,
            None,
        );
        assert_eq!(user_0_changes.len(), 2); // user_0 and user_2 -> user_0

        // Test filtering by change type
        let tool_changes = tracker.get_changes_filtered(
            Some("tool_rule"),
            None,
            None,
            None,
            None,
        );
        assert_eq!(tool_changes.len(), 3);

        // Test limiting
        let limited_changes = tracker.get_changes_filtered(
            None,
            None,
            None,
            None,
            Some(2),
        );
        assert_eq!(limited_changes.len(), 2);
    }
}