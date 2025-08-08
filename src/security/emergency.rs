//! Emergency lockdown system for MagicTunnel
//!
//! Provides emergency lockdown capabilities to immediately block all tool requests
//! during security incidents, with persistent state management and audit logging.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use tokio::fs;
use tracing::{info, warn, error};
use std::path::PathBuf;

/// Emergency lockdown state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyLockdownState {
    /// Whether lockdown is currently active
    pub is_active: bool,
    /// When the lockdown was activated
    pub activated_at: Option<DateTime<Utc>>,
    /// Who activated the lockdown
    pub activated_by: Option<String>,
    /// Reason for the lockdown
    pub reason: Option<String>,
    /// When the lockdown was last updated
    pub last_updated: DateTime<Utc>,
    /// Number of requests blocked during this lockdown
    pub blocked_requests: u64,
    /// Lockdown session ID for tracking
    pub session_id: String,
}

/// Emergency lockdown configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyLockdownConfig {
    /// Whether emergency lockdown is enabled
    pub enabled: bool,
    /// Path to store lockdown state
    pub state_file_path: PathBuf,
    /// Whether to log all blocked requests during lockdown
    pub log_blocked_requests: bool,
    /// Authorized users who can activate/deactivate lockdown
    pub authorized_users: Vec<String>,
    /// Whether to automatically notify administrators
    pub auto_notify_admins: bool,
    /// Notification methods configuration
    pub notification_config: NotificationConfig,
}

/// Configuration for lockdown notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Whether to send email notifications
    pub email_enabled: bool,
    /// Email addresses to notify
    pub email_addresses: Vec<String>,
    /// Whether to send webhook notifications
    pub webhook_enabled: bool,
    /// Webhook URLs to notify
    pub webhook_urls: Vec<String>,
    /// Whether to log notifications to audit service
    pub audit_notifications: bool,
}

/// Result of emergency lockdown operations
#[derive(Debug, Clone)]
pub struct EmergencyLockdownResult {
    /// Whether the operation was successful
    pub success: bool,
    /// Previous state (for rollback if needed)
    pub previous_state: Option<EmergencyLockdownState>,
    /// Current state after operation
    pub current_state: EmergencyLockdownState,
    /// Operation message
    pub message: String,
    /// Any errors that occurred
    pub error: Option<String>,
}

/// Emergency lockdown manager
pub struct EmergencyLockdownManager {
    config: EmergencyLockdownConfig,
    state: Arc<Mutex<EmergencyLockdownState>>,
}

impl Default for EmergencyLockdownConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            state_file_path: PathBuf::from("./emergency_lockdown_state.json"),
            log_blocked_requests: true,
            authorized_users: vec!["admin".to_string()],
            auto_notify_admins: true,
            notification_config: NotificationConfig {
                email_enabled: false,
                email_addresses: Vec::new(),
                webhook_enabled: false,
                webhook_urls: Vec::new(),
                audit_notifications: true,
            },
        }
    }
}

impl Default for EmergencyLockdownState {
    fn default() -> Self {
        Self {
            is_active: false,
            activated_at: None,
            activated_by: None,
            reason: None,
            last_updated: Utc::now(),
            blocked_requests: 0,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl EmergencyLockdownManager {
    /// Create a new emergency lockdown manager
    pub async fn new(config: EmergencyLockdownConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Try to load existing state from file
        let state = if config.state_file_path.exists() {
            match Self::load_state_from_file(&config.state_file_path).await {
                Ok(loaded_state) => {
                    info!("Loaded existing emergency lockdown state from {:?}", config.state_file_path);
                    loaded_state
                },
                Err(e) => {
                    warn!("Failed to load emergency lockdown state from file: {}. Using default state.", e);
                    EmergencyLockdownState::default()
                }
            }
        } else {
            info!("No existing emergency lockdown state file found. Using default state.");
            EmergencyLockdownState::default()
        };

        let manager = Self {
            config,
            state: Arc::new(Mutex::new(state)),
        };

        // Save initial state to ensure file is created
        manager.save_state_to_file().await?;

        Ok(manager)
    }

    /// Check if emergency lockdown is currently active
    pub fn is_lockdown_active(&self) -> bool {
        if !self.config.enabled {
            return false;
        }
        
        self.state.lock().unwrap().is_active
    }

    /// Get current lockdown state (read-only)
    pub fn get_lockdown_state(&self) -> EmergencyLockdownState {
        self.state.lock().unwrap().clone()
    }

    /// Activate emergency lockdown
    pub async fn activate_lockdown(
        &self,
        activated_by: Option<String>,
        reason: Option<String>,
    ) -> Result<EmergencyLockdownResult, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Ok(EmergencyLockdownResult {
                success: false,
                previous_state: None,
                current_state: self.get_lockdown_state(),
                message: "Emergency lockdown is disabled in configuration".to_string(),
                error: Some("Emergency lockdown disabled".to_string()),
            });
        }

        // Check authorization
        if let Some(ref user) = activated_by {
            if !self.config.authorized_users.contains(user) {
                return Ok(EmergencyLockdownResult {
                    success: false,
                    previous_state: None,
                    current_state: self.get_lockdown_state(),
                    message: format!("User {} is not authorized to activate emergency lockdown", user),
                    error: Some("Unauthorized user".to_string()),
                });
            }
        }

        let previous_state = self.get_lockdown_state();
        
        // Don't reactivate if already active
        if previous_state.is_active {
            return Ok(EmergencyLockdownResult {
                success: false,
                previous_state: Some(previous_state.clone()),
                current_state: previous_state,
                message: "Emergency lockdown is already active".to_string(),
                error: None,
            });
        }

        // Update state
        let new_state = {
            let mut state = self.state.lock().unwrap();
            state.is_active = true;
            state.activated_at = Some(Utc::now());
            state.activated_by = activated_by.clone();
            state.reason = reason.clone();
            state.last_updated = Utc::now();
            state.blocked_requests = 0;
            state.session_id = uuid::Uuid::new_v4().to_string();
            state.clone()
        };

        // Save state to file
        if let Err(e) = self.save_state_to_file().await {
            error!("Failed to save emergency lockdown state: {}", e);
            return Ok(EmergencyLockdownResult {
                success: false,
                previous_state: Some(previous_state),
                current_state: new_state,
                message: "Failed to persist lockdown state".to_string(),
                error: Some(format!("File save error: {}", e)),
            });
        }

        // Log the activation
        info!(
            "Emergency lockdown activated by: {:?}, reason: {:?}, session: {}",
            activated_by, reason, new_state.session_id
        );

        // Send notifications if configured
        if self.config.auto_notify_admins {
            if let Err(e) = self.send_lockdown_notifications(&new_state, true).await {
                warn!("Failed to send lockdown activation notifications: {}", e);
            }
        }

        Ok(EmergencyLockdownResult {
            success: true,
            previous_state: Some(previous_state),
            current_state: new_state.clone(),
            message: "Emergency lockdown activated successfully".to_string(),
            error: None,
        })
    }

    /// Deactivate emergency lockdown
    pub async fn deactivate_lockdown(
        &self,
        deactivated_by: Option<String>,
    ) -> Result<EmergencyLockdownResult, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            return Ok(EmergencyLockdownResult {
                success: false,
                previous_state: None,
                current_state: self.get_lockdown_state(),
                message: "Emergency lockdown is disabled in configuration".to_string(),
                error: Some("Emergency lockdown disabled".to_string()),
            });
        }

        // Check authorization
        if let Some(ref user) = deactivated_by {
            if !self.config.authorized_users.contains(user) {
                return Ok(EmergencyLockdownResult {
                    success: false,
                    previous_state: None,
                    current_state: self.get_lockdown_state(),
                    message: format!("User {} is not authorized to deactivate emergency lockdown", user),
                    error: Some("Unauthorized user".to_string()),
                });
            }
        }

        let previous_state = self.get_lockdown_state();

        // Don't deactivate if already inactive
        if !previous_state.is_active {
            return Ok(EmergencyLockdownResult {
                success: false,
                previous_state: Some(previous_state.clone()),
                current_state: previous_state,
                message: "Emergency lockdown is already inactive".to_string(),
                error: None,
            });
        }

        // Update state
        let new_state = {
            let mut state = self.state.lock().unwrap();
            state.is_active = false;
            state.last_updated = Utc::now();
            state.clone()
        };

        // Save state to file
        if let Err(e) = self.save_state_to_file().await {
            error!("Failed to save emergency lockdown state: {}", e);
            return Ok(EmergencyLockdownResult {
                success: false,
                previous_state: Some(previous_state),
                current_state: new_state,
                message: "Failed to persist lockdown state".to_string(),
                error: Some(format!("File save error: {}", e)),
            });
        }

        // Log the deactivation
        info!(
            "Emergency lockdown deactivated by: {:?}, session: {}, blocked requests: {}",
            deactivated_by, new_state.session_id, new_state.blocked_requests
        );

        // Send notifications if configured
        if self.config.auto_notify_admins {
            if let Err(e) = self.send_lockdown_notifications(&new_state, false).await {
                warn!("Failed to send lockdown deactivation notifications: {}", e);
            }
        }

        let blocked_count = new_state.blocked_requests;
        Ok(EmergencyLockdownResult {
            success: true,
            previous_state: Some(previous_state),
            current_state: new_state,
            message: format!("Emergency lockdown deactivated successfully. {} requests were blocked during this session.", blocked_count),
            error: None,
        })
    }

    /// Increment blocked request counter (called when a request is blocked)
    pub fn increment_blocked_requests(&self) -> u64 {
        if !self.config.enabled {
            return 0;
        }

        let mut state = self.state.lock().unwrap();
        if state.is_active {
            state.blocked_requests += 1;
            state.last_updated = Utc::now();
            state.blocked_requests
        } else {
            0
        }
    }

    /// Get lockdown statistics
    pub fn get_lockdown_statistics(&self) -> EmergencyLockdownStatistics {
        let state = self.state.lock().unwrap().clone();
        
        let duration = if let Some(activated_at) = state.activated_at {
            if state.is_active {
                Some((Utc::now() - activated_at).num_seconds())
            } else {
                Some((state.last_updated - activated_at).num_seconds())
            }
        } else {
            None
        };

        EmergencyLockdownStatistics {
            is_active: state.is_active,
            activated_at: state.activated_at,
            activated_by: state.activated_by,
            reason: state.reason,
            duration_seconds: duration,
            blocked_requests: state.blocked_requests,
            session_id: state.session_id,
            last_updated: state.last_updated,
        }
    }

    /// Load state from file
    async fn load_state_from_file(file_path: &PathBuf) -> Result<EmergencyLockdownState, Box<dyn std::error::Error + Send + Sync>> {
        let contents = fs::read_to_string(file_path).await?;
        let state: EmergencyLockdownState = serde_json::from_str(&contents)?;
        Ok(state)
    }

    /// Save current state to file
    async fn save_state_to_file(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state = self.state.lock().unwrap().clone();
        let contents = serde_json::to_string_pretty(&state)?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = self.config.state_file_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        fs::write(&self.config.state_file_path, contents).await?;
        Ok(())
    }

    /// Send notifications about lockdown state change
    async fn send_lockdown_notifications(
        &self,
        state: &EmergencyLockdownState,
        is_activation: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let action = if is_activation { "activated" } else { "deactivated" };
        let message = format!(
            "Emergency lockdown {} by {:?} at {}. Reason: {:?}",
            action,
            state.activated_by,
            state.last_updated,
            state.reason
        );

        // Log notification (always enabled for audit trail)
        info!("Emergency lockdown notification: {}", message);

        // Email notifications
        if self.config.notification_config.email_enabled {
            for email in &self.config.notification_config.email_addresses {
                // TODO: Implement actual email sending
                info!("Would send email notification to {}: {}", email, message);
            }
        }

        // Webhook notifications  
        if self.config.notification_config.webhook_enabled {
            for webhook_url in &self.config.notification_config.webhook_urls {
                // TODO: Implement actual webhook sending
                info!("Would send webhook notification to {}: {}", webhook_url, message);
            }
        }

        Ok(())
    }
}

/// Statistics for emergency lockdown system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyLockdownStatistics {
    /// Whether lockdown is currently active
    pub is_active: bool,
    /// When the lockdown was activated
    pub activated_at: Option<DateTime<Utc>>,
    /// Who activated the lockdown
    pub activated_by: Option<String>,
    /// Reason for the lockdown
    pub reason: Option<String>,
    /// Duration of lockdown in seconds (if active or completed)
    pub duration_seconds: Option<i64>,
    /// Number of requests blocked
    pub blocked_requests: u64,
    /// Current session ID
    pub session_id: String,
    /// When the state was last updated
    pub last_updated: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_emergency_lockdown_activation() {
        let temp_dir = tempdir().unwrap();
        let state_file = temp_dir.path().join("test_emergency_state.json");
        
        let config = EmergencyLockdownConfig {
            state_file_path: state_file,
            ..Default::default()
        };

        let manager = EmergencyLockdownManager::new(config).await.unwrap();
        
        // Initially should not be active
        assert!(!manager.is_lockdown_active());

        // Activate lockdown
        let result = manager.activate_lockdown(
            Some("test_admin".to_string()),
            Some("Test emergency".to_string()),
        ).await.unwrap();

        assert!(result.success);
        assert!(manager.is_lockdown_active());
        
        let state = manager.get_lockdown_state();
        assert!(state.is_active);
        assert_eq!(state.activated_by, Some("test_admin".to_string()));
        assert_eq!(state.reason, Some("Test emergency".to_string()));
    }

    #[tokio::test]
    async fn test_emergency_lockdown_deactivation() {
        let temp_dir = tempdir().unwrap();
        let state_file = temp_dir.path().join("test_emergency_state.json");
        
        let config = EmergencyLockdownConfig {
            state_file_path: state_file,
            ..Default::default()
        };

        let manager = EmergencyLockdownManager::new(config).await.unwrap();
        
        // Activate first
        manager.activate_lockdown(
            Some("test_admin".to_string()),
            Some("Test emergency".to_string()),
        ).await.unwrap();

        assert!(manager.is_lockdown_active());

        // Now deactivate
        let result = manager.deactivate_lockdown(Some("test_admin".to_string())).await.unwrap();
        
        assert!(result.success);
        assert!(!manager.is_lockdown_active());
        
        let state = manager.get_lockdown_state();
        assert!(!state.is_active);
    }

    #[tokio::test]
    async fn test_unauthorized_user() {
        let temp_dir = tempdir().unwrap();
        let state_file = temp_dir.path().join("test_emergency_state.json");
        
        let config = EmergencyLockdownConfig {
            state_file_path: state_file,
            authorized_users: vec!["admin".to_string()],
            ..Default::default()
        };

        let manager = EmergencyLockdownManager::new(config).await.unwrap();
        
        // Try to activate with unauthorized user
        let result = manager.activate_lockdown(
            Some("unauthorized_user".to_string()),
            Some("Test emergency".to_string()),
        ).await.unwrap();

        assert!(!result.success);
        assert!(!manager.is_lockdown_active());
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_blocked_requests_counter() {
        let temp_dir = tempdir().unwrap();
        let state_file = temp_dir.path().join("test_emergency_state.json");
        
        let config = EmergencyLockdownConfig {
            state_file_path: state_file,
            ..Default::default()
        };

        let manager = EmergencyLockdownManager::new(config).await.unwrap();
        
        // Activate lockdown
        manager.activate_lockdown(
            Some("test_admin".to_string()),
            Some("Test emergency".to_string()),
        ).await.unwrap();

        // Increment blocked requests
        let count1 = manager.increment_blocked_requests();
        let count2 = manager.increment_blocked_requests();
        let count3 = manager.increment_blocked_requests();

        assert_eq!(count1, 1);
        assert_eq!(count2, 2);
        assert_eq!(count3, 3);

        let stats = manager.get_lockdown_statistics();
        assert_eq!(stats.blocked_requests, 3);
    }
}