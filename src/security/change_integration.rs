//! Integration layer for automatic configuration change tracking
//!
//! This module provides integration between the change tracker and various
//! security services to automatically track configuration changes.

use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use serde_json::json;
use tracing::{info, warn, error, debug};
use chrono::Utc;
use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event, EventKind};
use tokio::sync::mpsc;

use super::{
    ConfigurationChangeTracker, ChangeType, ChangeOperation, ChangeUser, ChangeTarget, ChangeListener,
    AllowlistService, AllowlistRule, AllowlistAction,
};

/// Integration service that connects change tracking with security services
pub struct ChangeTrackingIntegration {
    change_tracker: Arc<ConfigurationChangeTracker>,
    default_user: ChangeUser,
}

/// Change listener for allowlist service
pub struct AllowlistChangeListener {
    change_tracker: Arc<ConfigurationChangeTracker>,
    default_user: ChangeUser,
}

/// File watcher for configuration file changes
pub struct ConfigFileWatcher {
    change_tracker: Arc<ConfigurationChangeTracker>,
    watched_directories: Vec<PathBuf>,
    default_user: ChangeUser,
    watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
    shutdown_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
}

impl ChangeTrackingIntegration {
    /// Create new change tracking integration
    pub fn new(
        change_tracker: Arc<ConfigurationChangeTracker>,
        default_user_id: Option<String>,
    ) -> Self {
        let default_user = ChangeUser {
            id: default_user_id,
            name: Some("System".to_string()),
            auth_method: "system".to_string(),
            api_key_name: None,
            roles: vec!["system".to_string()],
            client_ip: None,
            user_agent: Some("MagicTunnel-System".to_string()),
        };

        Self {
            change_tracker,
            default_user,
        }
    }

    /// Track a tool rule change
    pub async fn track_tool_rule_change(
        &self,
        tool_name: &str,
        operation: ChangeOperation,
        before_rule: Option<&AllowlistRule>,
        after_rule: Option<&AllowlistRule>,
        user: Option<ChangeUser>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let change_type = ChangeType::ToolRule {
            tool_name: tool_name.to_string(),
        };

        let target = ChangeTarget {
            target_type: "tool_rule".to_string(),
            identifier: tool_name.to_string(),
            parent: Some("allowlist_config".to_string()),
            scope: "tool".to_string(),
        };

        let before_state = before_rule.map(|rule| json!({
            "action": match rule.action {
                AllowlistAction::Allow => "allow",
                AllowlistAction::Deny => "deny",
            },
            "reason": rule.reason,
            "enabled": rule.enabled,
            "name": rule.name,
        }));

        let after_state = after_rule.map(|rule| json!({
            "action": match rule.action {
                AllowlistAction::Allow => "allow",
                AllowlistAction::Deny => "deny",
            },
            "reason": rule.reason,
            "enabled": rule.enabled,
            "name": rule.name,
        }));

        let metadata = {
            let mut map = HashMap::new();
            map.insert("tool_name".to_string(), json!(tool_name));
            map.insert("timestamp".to_string(), json!(Utc::now()));
            map.insert("source".to_string(), json!("allowlist_service"));
            map
        };

        let user = user.unwrap_or_else(|| self.default_user.clone());

        self.change_tracker
            .track_change(
                change_type,
                operation,
                user,
                target,
                before_state,
                after_state,
                metadata,
            )
            .await
    }

    /// Track a pattern rule change
    pub async fn track_pattern_change(
        &self,
        pattern_name: &str,
        pattern_type: &str, // "capability" or "global"
        operation: ChangeOperation,
        before_state: Option<serde_json::Value>,
        after_state: Option<serde_json::Value>,
        user: Option<ChangeUser>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let change_type = match pattern_type {
            "capability" => ChangeType::CapabilityPattern {
                pattern_name: pattern_name.to_string(),
            },
            "global" => ChangeType::GlobalPattern {
                pattern_name: pattern_name.to_string(),
            },
            _ => return Err("Invalid pattern type".into()),
        };

        let target = ChangeTarget {
            target_type: format!("{}_pattern", pattern_type),
            identifier: pattern_name.to_string(),
            parent: Some(format!("{}-patterns.yaml", pattern_type)),
            scope: pattern_type.to_string(),
        };

        let metadata = {
            let mut map = HashMap::new();
            map.insert("pattern_name".to_string(), json!(pattern_name));
            map.insert("pattern_type".to_string(), json!(pattern_type));
            map.insert("timestamp".to_string(), json!(Utc::now()));
            map.insert("source".to_string(), json!("allowlist_service"));
            map
        };

        let user = user.unwrap_or_else(|| self.default_user.clone());

        self.change_tracker
            .track_change(
                change_type,
                operation,
                user,
                target,
                before_state,
                after_state,
                metadata,
            )
            .await
    }

    /// Track an emergency lockdown change
    pub async fn track_emergency_lockdown_change(
        &self,
        operation: ChangeOperation,
        before_state: Option<serde_json::Value>,
        after_state: Option<serde_json::Value>,
        user: Option<ChangeUser>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let change_type = ChangeType::EmergencyLockdown;

        let target = ChangeTarget {
            target_type: "emergency_lockdown".to_string(),
            identifier: "emergency_lockdown_state".to_string(),
            parent: Some("emergency_lockdown_config".to_string()),
            scope: "system".to_string(),
        };

        let metadata = {
            let mut map = HashMap::new();
            map.insert("component".to_string(), json!("emergency_lockdown"));
            map.insert("timestamp".to_string(), json!(Utc::now()));
            map.insert("source".to_string(), json!("emergency_manager"));
            map
        };

        let user = user.unwrap_or_else(|| self.default_user.clone());

        self.change_tracker
            .track_change(
                change_type,
                operation,
                user,
                target,
                before_state,
                after_state,
                metadata,
            )
            .await
    }

    /// Track a file-level configuration change
    pub async fn track_file_change(
        &self,
        file_path: &str,
        operation: ChangeOperation,
        before_content: Option<String>,
        after_content: Option<String>,
        user: Option<ChangeUser>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let change_type = ChangeType::File {
            file_path: file_path.to_string(),
        };

        let target = ChangeTarget {
            target_type: "file".to_string(),
            identifier: file_path.to_string(),
            parent: None,
            scope: "filesystem".to_string(),
        };

        let before_state = before_content.map(|content| json!({
            "file_path": file_path,
            "content": content,
            "size": content.len(),
        }));

        let after_state = after_content.map(|content| json!({
            "file_path": file_path,
            "content": content,
            "size": content.len(),
        }));

        let metadata = {
            let mut map = HashMap::new();
            map.insert("file_path".to_string(), json!(file_path));
            map.insert("timestamp".to_string(), json!(Utc::now()));
            map.insert("source".to_string(), json!("file_watcher"));
            
            // Add file extension and type information
            if let Some(extension) = std::path::Path::new(file_path).extension() {
                map.insert("file_extension".to_string(), json!(extension.to_string_lossy()));
            }
            
            // Determine config type based on file path
            let config_type = if file_path.contains("allowlist") {
                "allowlist"
            } else if file_path.contains("pattern") {
                "pattern"
            } else if file_path.contains("policy") {
                "policy"
            } else if file_path.contains("rbac") {
                "rbac"
            } else {
                "unknown"
            };
            map.insert("config_type".to_string(), json!(config_type));
            
            map
        };

        let user = user.unwrap_or_else(|| self.default_user.clone());

        self.change_tracker
            .track_change(
                change_type,
                operation,
                user,
                target,
                before_state,
                after_state,
                metadata,
            )
            .await
    }

    /// Get the change tracker reference
    pub fn get_tracker(&self) -> Arc<ConfigurationChangeTracker> {
        Arc::clone(&self.change_tracker)
    }

    /// Create an allowlist change listener
    pub fn create_allowlist_listener(&self) -> AllowlistChangeListener {
        AllowlistChangeListener {
            change_tracker: Arc::clone(&self.change_tracker),
            default_user: self.default_user.clone(),
        }
    }

    /// Create a file watcher for configuration directories
    pub fn create_file_watcher(&self, watched_directories: Vec<PathBuf>) -> ConfigFileWatcher {
        ConfigFileWatcher {
            change_tracker: Arc::clone(&self.change_tracker),
            watched_directories,
            default_user: self.default_user.clone(),
            watcher: Arc::new(Mutex::new(None)),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }
}

impl ChangeListener for AllowlistChangeListener {
    fn before_change(&self, change: &super::ConfigurationChange) -> Result<(), Box<dyn std::error::Error>> {
        debug!(
            "Allowlist change listener: before_change for {} ({})",
            change.target.identifier, change.id
        );

        // Pre-change validation
        match &change.change_type {
            ChangeType::ToolRule { tool_name } => {
                if tool_name.is_empty() {
                    return Err("Tool name cannot be empty".into());
                }
            }
            ChangeType::CapabilityPattern { pattern_name } | ChangeType::GlobalPattern { pattern_name } => {
                if pattern_name.is_empty() {
                    return Err("Pattern name cannot be empty".into());
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn after_change(&self, change: &super::ConfigurationChange) -> Result<(), Box<dyn std::error::Error>> {
        info!(
            "Allowlist change applied: {} - {} ({})",
            change.target.identifier,
            change.diff.summary,
            change.id
        );

        // Post-change actions (e.g., cache invalidation, notifications)
        match &change.change_type {
            ChangeType::ToolRule { .. } => {
                debug!("Tool rule change applied, may need cache invalidation");
            }
            ChangeType::CapabilityPattern { .. } | ChangeType::GlobalPattern { .. } => {
                debug!("Pattern change applied, may affect multiple tools");
            }
            ChangeType::EmergencyLockdown => {
                warn!("Emergency lockdown state changed");
            }
            _ => {}
        }

        Ok(())
    }

    fn change_failed(&self, change: &super::ConfigurationChange, error: &str) {
        error!(
            "Allowlist change failed: {} - {} (error: {})",
            change.target.identifier, change.id, error
        );

        // Handle change failure (e.g., rollback, alerts)
        match &change.change_type {
            ChangeType::EmergencyLockdown => {
                error!("Emergency lockdown change failed - this is critical!");
            }
            _ => {
                warn!("Configuration change failed, manual intervention may be required");
            }
        }
    }
}

impl ConfigFileWatcher {
    /// Start watching configuration files for changes
    pub async fn start_watching(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting configuration file watcher for {} directories", self.watched_directories.len());

        // Create channel for shutdown signaling
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        
        // Store shutdown sender for later cleanup
        {
            let mut tx_guard = self.shutdown_tx.lock().unwrap();
            *tx_guard = Some(shutdown_tx);
        }

        // Create channel for file events
        let (event_tx, mut event_rx) = mpsc::channel::<Result<Event, notify::Error>>(100);

        // Create the watcher
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Err(e) = event_tx.blocking_send(result) {
                    error!("Failed to send file event: {}", e);
                }
            },
            notify::Config::default(),
        )?;

        // Watch all specified directories
        for directory in &self.watched_directories {
            if directory.exists() && directory.is_dir() {
                info!("Watching directory: {:?}", directory);
                watcher.watch(directory, RecursiveMode::Recursive)?;
            } else {
                warn!("Directory does not exist or is not a directory: {:?}", directory);
            }
        }

        // Store watcher for later cleanup
        {
            let mut watcher_guard = self.watcher.lock().unwrap();
            *watcher_guard = Some(watcher);
        }

        // Clone Arc references for the async task
        let change_tracker = Arc::clone(&self.change_tracker);
        let default_user = self.default_user.clone();

        // Spawn async task to handle file events
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle file events
                    event_result = event_rx.recv() => {
                        match event_result {
                            Some(Ok(event)) => {
                                if let Err(e) = Self::handle_file_event(&change_tracker, &default_user, event).await {
                                    error!("Failed to handle file event: {}", e);
                                }
                            }
                            Some(Err(e)) => {
                                error!("File watcher error: {}", e);
                            }
                            None => {
                                debug!("File event channel closed");
                                break;
                            }
                        }
                    }
                    // Handle shutdown signal
                    _ = shutdown_rx.recv() => {
                        info!("File watcher shutdown requested");
                        break;
                    }
                }
            }
        });

        info!("Configuration file watcher started successfully");
        Ok(())
    }

    /// Stop watching configuration files
    pub async fn stop_watching(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Stopping configuration file watcher");
        
        // Send shutdown signal
        if let Some(shutdown_tx) = {
            let mut tx_guard = self.shutdown_tx.lock().unwrap();
            tx_guard.take()
        } {
            if let Err(e) = shutdown_tx.send(()).await {
                warn!("Failed to send shutdown signal: {}", e);
            }
        }

        // Stop the file watcher
        if let Some(mut watcher) = {
            let mut watcher_guard = self.watcher.lock().unwrap();
            watcher_guard.take()
        } {
            // Watcher will be dropped here, automatically stopping file monitoring
            drop(watcher);
            info!("File watcher stopped");
        }
        
        info!("Configuration file watcher stopped successfully");
        Ok(())
    }

    /// Handle a file system event from the watcher
    async fn handle_file_event(
        change_tracker: &Arc<ConfigurationChangeTracker>,
        default_user: &ChangeUser,
        event: Event,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Filter for file events we care about
        let paths = event.paths;
        
        for path in paths {
            // Only process files that look like configuration files
            if !Self::is_config_file(&path) {
                continue;
            }

            let file_path_str = path.to_string_lossy().to_string();
            let operation = match event.kind {
                EventKind::Create(_) => ChangeOperation::Create,
                EventKind::Modify(_) => ChangeOperation::Update,
                EventKind::Remove(_) => ChangeOperation::Delete,
                _ => continue, // Skip other event types
            };

            debug!("File event detected: {:?} on {:?}", operation, path);

            // Read file content if it exists and wasn't deleted
            let content = if path.exists() && operation != ChangeOperation::Delete {
                match tokio::fs::read_to_string(&path).await {
                    Ok(content) => Some(content),
                    Err(e) => {
                        warn!("Failed to read file {:?}: {}", path, e);
                        None
                    }
                }
            } else {
                None
            };

            // Create integration instance for tracking
            let integration = ChangeTrackingIntegration::new(
                Arc::clone(change_tracker),
                default_user.id.clone(),
            );

            // Track the file change
            if let Err(e) = integration.track_file_change(
                &file_path_str,
                operation.clone(),
                None, // We don't have before content in real-time watching
                content,
                Some(default_user.clone()),
            ).await {
                error!("Failed to track file change for {:?}: {}", path, e);
            } else {
                info!("Successfully tracked file change: {:?} {:?}", operation, path);
            }
        }

        Ok(())
    }

    /// Check if a path represents a configuration file we should track
    fn is_config_file(path: &std::path::Path) -> bool {
        // Check file extension
        if let Some(extension) = path.extension() {
            let ext_str = extension.to_string_lossy().to_lowercase();
            match ext_str.as_str() {
                "yaml" | "yml" | "toml" | "json" | "conf" | "cfg" | "ini" => return true,
                _ => {}
            }
        }

        // Check filename patterns
        if let Some(filename) = path.file_name() {
            let filename_str = filename.to_string_lossy().to_lowercase();
            if filename_str.contains("config") ||
               filename_str.contains("allowlist") ||
               filename_str.contains("policy") ||
               filename_str.contains("rbac") ||
               filename_str.contains("pattern") {
                return true;
            }
        }

        false
    }

    /// Handle a detected file change
    async fn handle_file_change(
        &self,
        file_path: &std::path::Path,
        change_type: &str, // "created", "modified", "deleted"
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_path_str = file_path.to_string_lossy().to_string();
        
        let operation = match change_type {
            "created" => ChangeOperation::Create,
            "modified" => ChangeOperation::Update,
            "deleted" => ChangeOperation::Delete,
            _ => return Ok(()),
        };

        // Read file content if it exists
        let content = if file_path.exists() && operation != ChangeOperation::Delete {
            match tokio::fs::read_to_string(file_path).await {
                Ok(content) => Some(content),
                Err(e) => {
                    warn!("Failed to read file {:?}: {}", file_path, e);
                    None
                }
            }
        } else {
            None
        };

        // Create change tracking integration
        let integration = ChangeTrackingIntegration::new(
            Arc::clone(&self.change_tracker),
            Some("file_watcher".to_string()),
        );

        // Track the file change
        integration
            .track_file_change(
                &file_path_str,
                operation,
                None, // We don't have the before content in this simple implementation
                content,
                Some(self.default_user.clone()),
            )
            .await?;

        Ok(())
    }
}

/// Helper functions for creating change users from different contexts
pub mod user_helpers {
    use super::ChangeUser;
    use crate::auth::AuthenticationResult;

    /// Create a ChangeUser from an authentication result
    pub fn from_auth_result(auth_result: &AuthenticationResult) -> ChangeUser {
        let (auth_method, api_key_name) = match auth_result {
            AuthenticationResult::ApiKey(key_entry) => {
                ("api_key".to_string(), Some(key_entry.name.clone()))
            }
            AuthenticationResult::OAuth(_) => {
                ("oauth".to_string(), None)
            }
            AuthenticationResult::Jwt(_) => {
                ("jwt".to_string(), None)
            }
            AuthenticationResult::ServiceAccount(sa_result) => {
                ("service_account".to_string(), sa_result.user_info.name.clone())
            }
            AuthenticationResult::DeviceCode(device_result) => {
                ("device_code".to_string(), device_result.user_info.as_ref().and_then(|info| info.name.clone()))
            }
        };

        // Extract roles based on permissions and authentication method
        let roles = match auth_result {
            AuthenticationResult::ApiKey(key_entry) => {
                // Map API key permissions to roles
                if key_entry.permissions.contains(&"admin".to_string()) {
                    vec!["admin".to_string()]
                } else if key_entry.permissions.contains(&"write".to_string()) {
                    vec!["user".to_string(), "writer".to_string()]
                } else if key_entry.permissions.contains(&"read".to_string()) {
                    vec!["user".to_string(), "reader".to_string()]
                } else {
                    vec!["guest".to_string()]
                }
            },
            AuthenticationResult::OAuth(_) => {
                // OAuth users get default user role
                vec!["user".to_string(), "oauth_user".to_string()]
            },
            AuthenticationResult::Jwt(jwt_result) => {
                // Extract roles from JWT permissions
                if jwt_result.permissions.contains(&"admin".to_string()) {
                    vec!["admin".to_string()]
                } else if jwt_result.permissions.contains(&"write".to_string()) {
                    vec!["user".to_string(), "writer".to_string()]
                } else {
                    vec!["user".to_string(), "reader".to_string()]
                }
            },
            AuthenticationResult::ServiceAccount(sa_result) => {
                // Service accounts get service role plus permission-based roles
                let mut roles = vec!["service_account".to_string()];
                if sa_result.permissions.contains(&"admin".to_string()) {
                    roles.push("admin".to_string());
                } else if sa_result.permissions.contains(&"write".to_string()) {
                    roles.push("writer".to_string());
                } else {
                    roles.push("reader".to_string());
                }
                roles
            },
            AuthenticationResult::DeviceCode(_) => {
                // Device code users get standard user role
                vec!["user".to_string(), "device_user".to_string()]
            }
        };
        
        ChangeUser {
            id: Some(auth_result.get_user_id()),
            name: None, // Could be extracted from auth result if available
            auth_method,
            api_key_name,
            roles,
            client_ip: None, // Would need to be passed from request context
            user_agent: None, // Would need to be passed from request context
        }
    }

    /// Create a ChangeUser for system operations
    pub fn system_user() -> ChangeUser {
        ChangeUser {
            id: Some("system".to_string()),
            name: Some("System".to_string()),
            auth_method: "system".to_string(),
            api_key_name: None,
            roles: vec!["system".to_string()],
            client_ip: None,
            user_agent: Some("MagicTunnel-System".to_string()),
        }
    }

    /// Create a ChangeUser for API operations
    pub fn api_user(user_id: Option<String>, api_key_name: Option<String>) -> ChangeUser {
        ChangeUser {
            id: user_id,
            name: None,
            auth_method: "api".to_string(),
            api_key_name,
            roles: vec!["api".to_string()],
            client_ip: None,
            user_agent: Some("MagicTunnel-API".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{ChangeTrackerConfig, AllowlistAction};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_change_integration() {
        let temp_dir = tempdir().unwrap();
        let config = ChangeTrackerConfig {
            storage_directory: temp_dir.path().to_path_buf(),
            persist_changes: false,
            ..Default::default()
        };

        let tracker = Arc::new(ConfigurationChangeTracker::new(config).await.unwrap());
        let integration = ChangeTrackingIntegration::new(tracker, Some("test_user".to_string()));

        // Test tracking a tool rule change
        let before_rule = AllowlistRule {
            action: AllowlistAction::Allow,
            reason: Some("Original rule".to_string()),
            pattern: None, // Tool-level rules don't use patterns
            enabled: true,
            name: Some("test_rule".to_string()),
        };

        let after_rule = AllowlistRule {
            action: AllowlistAction::Deny,
            reason: Some("Updated rule".to_string()),
            pattern: None, // Tool-level rules don't use patterns
            enabled: true,
            name: Some("test_rule".to_string()),
        };

        let change_id = integration
            .track_tool_rule_change(
                "test_tool",
                ChangeOperation::Update,
                Some(&before_rule),
                Some(&after_rule),
                None,
            )
            .await
            .unwrap();

        assert!(!change_id.is_empty());

        // Verify the change was tracked
        let changes = integration.get_tracker().get_changes();
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.id, change_id);
        assert!(matches!(change.change_type, ChangeType::ToolRule { .. }));
        assert!(matches!(change.operation, ChangeOperation::Update));
        assert!(change.before_state.is_some());
        assert!(change.after_state.is_some());
    }

    #[tokio::test]
    async fn test_allowlist_change_listener() {
        let temp_dir = tempdir().unwrap();
        let config = ChangeTrackerConfig {
            storage_directory: temp_dir.path().to_path_buf(),
            persist_changes: false,
            ..Default::default()
        };

        let tracker = Arc::new(ConfigurationChangeTracker::new(config).await.unwrap());
        let integration = ChangeTrackingIntegration::new(tracker, Some("test_user".to_string()));
        let listener = integration.create_allowlist_listener();

        // Create a test change
        let change = super::super::ConfigurationChange {
            id: "test_change".to_string(),
            timestamp: chrono::Utc::now(),
            change_type: ChangeType::ToolRule {
                tool_name: "test_tool".to_string(),
            },
            operation: ChangeOperation::Update,
            user: super::user_helpers::system_user(),
            target: ChangeTarget {
                target_type: "tool_rule".to_string(),
                identifier: "test_tool".to_string(),
                parent: None,
                scope: "tool".to_string(),
            },
            before_state: None,
            after_state: None,
            diff: super::super::ChangeDiff {
                diff_type: "test".to_string(),
                additions: 0,
                deletions: 0,
                modifications: 1,
                field_changes: Vec::new(),
                summary: "Test change".to_string(),
            },
            impact: super::super::ChangeImpact {
                severity: "low".to_string(),
                affected_tools_count: 1,
                affected_tools: vec!["test_tool".to_string()],
                affected_users_count: None,
                security_implications: Vec::new(),
                performance_implications: Vec::new(),
                rollback_difficulty: "easy".to_string(),
                validation_steps: Vec::new(),
                related_changes: Vec::new(),
            },
            metadata: HashMap::new(),
            validation: super::super::ChangeValidation {
                is_valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
                syntax_valid: true,
                semantic_valid: true,
                conflicts: Vec::new(),
            },
        };

        // Test listener methods
        assert!(listener.before_change(&change).is_ok());
        assert!(listener.after_change(&change).is_ok());
        
        // Test with invalid change
        let invalid_change = super::super::ConfigurationChange {
            change_type: ChangeType::ToolRule {
                tool_name: "".to_string(), // Empty tool name should fail
            },
            ..change
        };

        assert!(listener.before_change(&invalid_change).is_err());
    }

    #[tokio::test]
    async fn test_config_file_watcher_creation() {
        let temp_dir = tempdir().unwrap();
        let config = ChangeTrackerConfig {
            storage_directory: temp_dir.path().to_path_buf(),
            persist_changes: false,
            ..Default::default()
        };

        let tracker = Arc::new(ConfigurationChangeTracker::new(config).await.unwrap());
        let integration = ChangeTrackingIntegration::new(tracker, Some("test_user".to_string()));

        let watched_dirs = vec![temp_dir.path().to_path_buf()];
        let watcher = integration.create_file_watcher(watched_dirs.clone());

        // Verify watcher was created with correct configuration
        assert_eq!(watcher.watched_directories, watched_dirs);
        assert_eq!(watcher.default_user.id, Some("test_user".to_string()));
        assert_eq!(watcher.default_user.auth_method, "system");
        
        // Verify watcher and shutdown_tx are initially None
        assert!(watcher.watcher.lock().unwrap().is_none());
        assert!(watcher.shutdown_tx.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_watcher_start_stop() {
        let temp_dir = tempdir().unwrap();
        let config = ChangeTrackerConfig {
            storage_directory: temp_dir.path().to_path_buf(),
            persist_changes: false,
            ..Default::default()
        };

        let tracker = Arc::new(ConfigurationChangeTracker::new(config).await.unwrap());
        let integration = ChangeTrackingIntegration::new(tracker, Some("test_user".to_string()));

        let watched_dirs = vec![temp_dir.path().to_path_buf()];
        let watcher = integration.create_file_watcher(watched_dirs);

        // Start watching
        let result = watcher.start_watching().await;
        assert!(result.is_ok(), "Failed to start file watcher: {:?}", result);

        // Verify watcher is running
        assert!(watcher.watcher.lock().unwrap().is_some());
        assert!(watcher.shutdown_tx.lock().unwrap().is_some());

        // Stop watching
        let result = watcher.stop_watching().await;
        assert!(result.is_ok(), "Failed to stop file watcher: {:?}", result);

        // Verify watcher is stopped
        assert!(watcher.watcher.lock().unwrap().is_none());
        assert!(watcher.shutdown_tx.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_watcher_nonexistent_directory() {
        let temp_dir = tempdir().unwrap();
        let config = ChangeTrackerConfig {
            storage_directory: temp_dir.path().to_path_buf(),
            persist_changes: false,
            ..Default::default()
        };

        let tracker = Arc::new(ConfigurationChangeTracker::new(config).await.unwrap());
        let integration = ChangeTrackingIntegration::new(tracker, Some("test_user".to_string()));

        // Create watcher with non-existent directory
        let nonexistent_dir = temp_dir.path().join("nonexistent");
        let watched_dirs = vec![nonexistent_dir];
        let watcher = integration.create_file_watcher(watched_dirs);

        // Should still start successfully (just logs warning)
        let result = watcher.start_watching().await;
        assert!(result.is_ok());

        // Stop watching
        let result = watcher.stop_watching().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_config_file() {
        use std::path::Path;

        // Test config file extensions
        assert!(ConfigFileWatcher::is_config_file(Path::new("config.yaml")));
        assert!(ConfigFileWatcher::is_config_file(Path::new("settings.yml")));
        assert!(ConfigFileWatcher::is_config_file(Path::new("app.toml")));
        assert!(ConfigFileWatcher::is_config_file(Path::new("data.json")));
        assert!(ConfigFileWatcher::is_config_file(Path::new("server.conf")));
        assert!(ConfigFileWatcher::is_config_file(Path::new("app.cfg")));
        assert!(ConfigFileWatcher::is_config_file(Path::new("settings.ini")));

        // Test config file names
        assert!(ConfigFileWatcher::is_config_file(Path::new("allowlist.txt")));
        assert!(ConfigFileWatcher::is_config_file(Path::new("security-policy")));
        assert!(ConfigFileWatcher::is_config_file(Path::new("rbac-rules")));
        assert!(ConfigFileWatcher::is_config_file(Path::new("pattern-config")));
        assert!(ConfigFileWatcher::is_config_file(Path::new("magictunnel-config")));

        // Test non-config files
        assert!(!ConfigFileWatcher::is_config_file(Path::new("script.py")));
        assert!(!ConfigFileWatcher::is_config_file(Path::new("data.csv")));
        assert!(!ConfigFileWatcher::is_config_file(Path::new("image.png")));
        assert!(!ConfigFileWatcher::is_config_file(Path::new("binary")));
    }

    #[tokio::test]
    async fn test_file_change_integration() {
        let temp_dir = tempdir().unwrap();
        let config = ChangeTrackerConfig {
            storage_directory: temp_dir.path().to_path_buf(),
            persist_changes: false,
            ..Default::default()
        };

        let tracker = Arc::new(ConfigurationChangeTracker::new(config).await.unwrap());
        let integration = ChangeTrackingIntegration::new(tracker, Some("test_user".to_string()));

        // Test file change tracking
        let test_file = temp_dir.path().join("test-config.yaml");
        let test_content = "test: configuration\nvalue: 123";

        let result = integration.track_file_change(
            test_file.to_string_lossy().as_ref(),
            ChangeOperation::Create,
            None,
            Some(test_content.to_string()),
            None, // Use default user
        ).await;

        assert!(result.is_ok(), "Failed to track file change: {:?}", result);

        // Test file deletion tracking
        let result = integration.track_file_change(
            test_file.to_string_lossy().as_ref(),
            ChangeOperation::Delete,
            Some(test_content.to_string()),
            None,
            None,
        ).await;

        assert!(result.is_ok(), "Failed to track file deletion: {:?}", result);
    }

    #[tokio::test]
    async fn test_file_watcher_double_stop() {
        let temp_dir = tempdir().unwrap();
        let config = ChangeTrackerConfig {
            storage_directory: temp_dir.path().to_path_buf(),
            persist_changes: false,
            ..Default::default()
        };

        let tracker = Arc::new(ConfigurationChangeTracker::new(config).await.unwrap());
        let integration = ChangeTrackingIntegration::new(tracker, Some("test_user".to_string()));

        let watched_dirs = vec![temp_dir.path().to_path_buf()];
        let watcher = integration.create_file_watcher(watched_dirs);

        // Start and stop normally
        assert!(watcher.start_watching().await.is_ok());
        assert!(watcher.stop_watching().await.is_ok());

        // Stop again should be safe
        assert!(watcher.stop_watching().await.is_ok());
    }
}