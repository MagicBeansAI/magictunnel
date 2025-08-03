//! MCP Progress Tracking System
//!
//! Provides comprehensive progress tracking for long-running operations according to MCP 2025-06-18 specification
//! with detailed progress updates, status monitoring, and completion tracking.

use crate::error::{ProxyError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock, oneshot};
use tracing::{debug, info, warn, error};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Progress tracking manager for long-running operations
pub struct ProgressTracker {
    /// Active progress sessions
    active_sessions: Arc<RwLock<HashMap<String, ProgressSession>>>,
    /// Progress event broadcaster
    event_sender: broadcast::Sender<ProgressEvent>,
    /// Configuration
    config: ProgressConfig,
}

/// Configuration for progress tracking system
#[derive(Debug, Clone)]
pub struct ProgressConfig {
    /// Enable progress tracking
    pub enabled: bool,
    /// Maximum number of concurrent progress sessions
    pub max_concurrent_sessions: usize,
    /// Progress update interval (milliseconds)
    pub update_interval_ms: u64,
    /// Session timeout (seconds)
    pub session_timeout_seconds: u64,
    /// Enable detailed progress events
    pub enable_detailed_events: bool,
    /// Cleanup interval for expired sessions (seconds)
    pub cleanup_interval_seconds: u64,
    /// Progress granularity level
    pub granularity_level: ProgressGranularity,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_sessions: 1000,
            update_interval_ms: 500, // 500ms updates
            session_timeout_seconds: 3600, // 1 hour
            enable_detailed_events: true,
            cleanup_interval_seconds: 300, // 5 minutes
            granularity_level: ProgressGranularity::Detailed,
        }
    }
}

/// Progress granularity levels
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressGranularity {
    /// Basic progress (start, complete, error)
    Basic,
    /// Standard progress with percentage updates
    Standard,
    /// Detailed progress with step-by-step tracking
    Detailed,
    /// Verbose progress with sub-operation tracking
    Verbose,
}

/// Progress session for tracking a long-running operation
#[derive(Debug)]
pub struct ProgressSession {
    /// Unique session ID
    pub id: String,
    /// Operation ID this session tracks
    pub operation_id: String,
    /// Session creation time
    pub created_at: DateTime<Utc>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
    /// Session expiry time
    pub expires_at: DateTime<Utc>,
    /// Current progress state
    pub state: ProgressState,
    /// Progress metadata
    pub metadata: HashMap<String, Value>,
    /// Sub-operations for detailed tracking
    pub sub_operations: Vec<SubOperation>,
    /// Progress history
    pub history: Vec<ProgressSnapshot>,
    /// Update sender for real-time updates
    pub update_sender: Option<oneshot::Sender<ProgressUpdate>>,
}

/// Current state of a progress session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressState {
    /// Operation is initializing
    Initializing,
    /// Operation is in progress
    InProgress {
        /// Current progress percentage (0-100)
        percentage: f64,
        /// Current step description
        current_step: String,
        /// Total steps (if known)
        total_steps: Option<u32>,
        /// Current step number
        current_step_number: Option<u32>,
        /// Estimated time remaining (seconds)
        eta_seconds: Option<u64>,
    },
    /// Operation is paused
    Paused {
        /// Pause reason
        reason: String,
        /// Progress at pause time
        paused_at_percentage: f64,
    },
    /// Operation completed successfully
    Completed {
        /// Completion time
        completed_at: DateTime<Utc>,
        /// Final result summary
        result_summary: Option<String>,
        /// Total duration
        duration_seconds: u64,
    },
    /// Operation failed
    Failed {
        /// Failure time
        failed_at: DateTime<Utc>,
        /// Error message
        error_message: String,
        /// Progress at failure time
        failed_at_percentage: f64,
        /// Error code
        error_code: Option<String>,
    },
    /// Operation was cancelled
    Cancelled {
        /// Cancellation time
        cancelled_at: DateTime<Utc>,
        /// Progress at cancellation time
        cancelled_at_percentage: f64,
        /// Cancellation reason
        reason: String,
    },
}

/// Sub-operation for detailed progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubOperation {
    /// Sub-operation ID
    pub id: String,
    /// Sub-operation name
    pub name: String,
    /// Sub-operation state
    pub state: SubOperationState,
    /// Progress percentage for this sub-operation
    pub percentage: f64,
    /// Start time
    pub started_at: Option<DateTime<Utc>>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
    /// Sub-operation metadata
    pub metadata: HashMap<String, Value>,
}

/// State of a sub-operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubOperationState {
    /// Not started yet
    Pending,
    /// Currently executing
    Active,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed { error: String },
    /// Skipped
    Skipped { reason: String },
}

/// Progress snapshot for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressSnapshot {
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
    /// Progress state at this time
    pub state: ProgressState,
    /// Additional context
    pub context: HashMap<String, Value>,
}

/// Progress event for notifications
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    /// Session ID
    pub session_id: String,
    /// Operation ID
    pub operation_id: String,
    /// Event type
    pub event_type: ProgressEventType,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event data
    pub data: HashMap<String, Value>,
}

/// Types of progress events
#[derive(Debug, Clone)]
pub enum ProgressEventType {
    /// Session created
    SessionCreated,
    /// Progress updated
    ProgressUpdated,
    /// State changed
    StateChanged,
    /// Sub-operation started
    SubOperationStarted,
    /// Sub-operation completed
    SubOperationCompleted,
    /// Session completed
    SessionCompleted,
    /// Session failed
    SessionFailed,
    /// Session cancelled
    SessionCancelled,
    /// Session paused
    SessionPaused,
    /// Session resumed
    SessionResumed,
    /// Session expired
    SessionExpired,
}

/// Progress update structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    /// Session ID
    pub session_id: String,
    /// New progress state
    pub state: ProgressState,
    /// Sub-operation updates
    pub sub_operation_updates: Vec<SubOperationUpdate>,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

/// Sub-operation update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubOperationUpdate {
    /// Sub-operation ID
    pub id: String,
    /// New state
    pub state: SubOperationState,
    /// New progress percentage
    pub percentage: f64,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

/// Progress statistics
#[derive(Debug, Serialize)]
pub struct ProgressStats {
    /// Total active sessions
    pub active_sessions: usize,
    /// Sessions by state
    pub sessions_by_state: HashMap<String, usize>,
    /// Average session duration
    pub avg_session_duration_seconds: f64,
    /// Total completed sessions
    pub total_completed: usize,
    /// Total failed sessions
    pub total_failed: usize,
    /// Total cancelled sessions
    pub total_cancelled: usize,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(config: ProgressConfig) -> Self {
        let (sender, _) = broadcast::channel(1000);
        
        let tracker = Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            event_sender: sender,
            config,
        };

        // Start cleanup task
        tracker.start_cleanup_task();
        
        tracker
    }

    /// Create a new progress session
    pub async fn create_session(
        &self,
        operation_id: String,
        metadata: HashMap<String, Value>,
    ) -> Result<String> {
        if !self.config.enabled {
            return Err(ProxyError::mcp("Progress tracking is disabled"));
        }

        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let operation_id_clone = operation_id.clone();
        
        // Check session limit
        {
            let sessions = self.active_sessions.read().await;
            if sessions.len() >= self.config.max_concurrent_sessions {
                return Err(ProxyError::mcp("Maximum concurrent progress sessions reached"));
            }
        }

        let session = ProgressSession {
            id: session_id.clone(),
            operation_id: operation_id.clone(),
            created_at: now,
            updated_at: now,
            expires_at: now + chrono::Duration::seconds(self.config.session_timeout_seconds as i64),
            state: ProgressState::Initializing,
            metadata: metadata.clone(),
            sub_operations: Vec::new(),
            history: vec![ProgressSnapshot {
                timestamp: now,
                state: ProgressState::Initializing,
                context: metadata.clone(),
            }],
            update_sender: None,
        };

        // Store session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        // Send creation event
        self.send_event(ProgressEvent {
            session_id: session_id.clone(),
            operation_id,
            event_type: ProgressEventType::SessionCreated,
            timestamp: now,
            data: metadata,
        }).await;

        info!("Created progress session: {} for operation: {}", session_id, operation_id_clone);
        Ok(session_id)
    }

    /// Update progress for a session
    pub async fn update_progress(
        &self,
        session_id: &str,
        state: ProgressState,
        sub_operation_updates: Vec<SubOperationUpdate>,
        metadata: HashMap<String, Value>,
    ) -> Result<()> {
        let now = Utc::now();
        let mut session_updated = false;
        let mut operation_id = String::new();

        // Update session
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                // Update session state
                let old_state = session.state.clone();
                session.state = state.clone();
                session.updated_at = now;
                session.metadata.extend(metadata.clone());
                operation_id = session.operation_id.clone();

                // Add to history
                session.history.push(ProgressSnapshot {
                    timestamp: now,
                    state: state.clone(),
                    context: metadata.clone(),
                });

                // Update sub-operations
                for update in &sub_operation_updates {
                    if let Some(sub_op) = session.sub_operations.iter_mut().find(|s| s.id == update.id) {
                        sub_op.state = update.state.clone();
                        sub_op.percentage = update.percentage;
                        sub_op.metadata.extend(update.metadata.clone());
                        
                        // Update timestamps based on state
                        match &update.state {
                            SubOperationState::Active => {
                                if sub_op.started_at.is_none() {
                                    sub_op.started_at = Some(now);
                                }
                            }
                            SubOperationState::Completed | SubOperationState::Failed { .. } | SubOperationState::Skipped { .. } => {
                                if sub_op.ended_at.is_none() {
                                    sub_op.ended_at = Some(now);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                session_updated = true;

                // Send state change event if state changed
                if std::mem::discriminant(&old_state) != std::mem::discriminant(&state) {
                    self.send_event(ProgressEvent {
                        session_id: session_id.to_string(),
                        operation_id: session.operation_id.clone(),
                        event_type: ProgressEventType::StateChanged,
                        timestamp: now,
                        data: json!({
                            "old_state": old_state,
                            "new_state": state,
                        }).as_object().unwrap().clone().into_iter().collect(),
                    }).await;
                }
            }
        }

        if !session_updated {
            return Err(ProxyError::mcp("Progress session not found"));
        }

        // Send progress update event
        self.send_event(ProgressEvent {
            session_id: session_id.to_string(),
            operation_id,
            event_type: ProgressEventType::ProgressUpdated,
            timestamp: now,
            data: metadata,
        }).await;

        debug!("Updated progress for session: {}", session_id);
        Ok(())
    }

    /// Add sub-operations to a session
    pub async fn add_sub_operations(
        &self,
        session_id: &str,
        sub_operations: Vec<SubOperation>,
    ) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.sub_operations.extend(sub_operations);
            session.updated_at = Utc::now();
            Ok(())
        } else {
            Err(ProxyError::mcp("Progress session not found"))
        }
    }

    /// Start a sub-operation
    pub async fn start_sub_operation(
        &self,
        session_id: &str,
        sub_operation_id: &str,
    ) -> Result<()> {
        let now = Utc::now();
        let mut operation_id = String::new();

        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                if let Some(sub_op) = session.sub_operations.iter_mut().find(|s| s.id == sub_operation_id) {
                    sub_op.state = SubOperationState::Active;
                    sub_op.started_at = Some(now);
                    operation_id = session.operation_id.clone();
                } else {
                    return Err(ProxyError::mcp("Sub-operation not found"));
                }
            } else {
                return Err(ProxyError::mcp("Progress session not found"));
            }
        }

        // Send sub-operation started event
        self.send_event(ProgressEvent {
            session_id: session_id.to_string(),
            operation_id,
            event_type: ProgressEventType::SubOperationStarted,
            timestamp: now,
            data: json!({
                "sub_operation_id": sub_operation_id
            }).as_object().unwrap().clone().into_iter().collect(),
        }).await;

        Ok(())
    }

    /// Complete a sub-operation
    pub async fn complete_sub_operation(
        &self,
        session_id: &str,
        sub_operation_id: &str,
        result_metadata: HashMap<String, Value>,
    ) -> Result<()> {
        let now = Utc::now();
        let mut operation_id = String::new();

        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                if let Some(sub_op) = session.sub_operations.iter_mut().find(|s| s.id == sub_operation_id) {
                    sub_op.state = SubOperationState::Completed;
                    sub_op.percentage = 100.0;
                    sub_op.ended_at = Some(now);
                    sub_op.metadata.extend(result_metadata.clone());
                    operation_id = session.operation_id.clone();
                } else {
                    return Err(ProxyError::mcp("Sub-operation not found"));
                }
            } else {
                return Err(ProxyError::mcp("Progress session not found"));
            }
        }

        // Send sub-operation completed event
        self.send_event(ProgressEvent {
            session_id: session_id.to_string(),
            operation_id,
            event_type: ProgressEventType::SubOperationCompleted,
            timestamp: now,
            data: result_metadata,
        }).await;

        Ok(())
    }

    /// Complete a progress session
    pub async fn complete_session(
        &self,
        session_id: &str,
        result_summary: Option<String>,
    ) -> Result<()> {
        let now = Utc::now();
        let mut operation_id = String::new();
        let mut duration_seconds = 0u64;

        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                duration_seconds = (now - session.created_at).num_seconds() as u64;
                session.state = ProgressState::Completed {
                    completed_at: now,
                    result_summary: result_summary.clone(),
                    duration_seconds,
                };
                session.updated_at = now;
                operation_id = session.operation_id.clone();

                // Add final snapshot
                session.history.push(ProgressSnapshot {
                    timestamp: now,
                    state: session.state.clone(),
                    context: json!({
                        "duration_seconds": duration_seconds,
                        "result_summary": result_summary
                    }).as_object().unwrap().clone().into_iter().collect(),
                });
            } else {
                return Err(ProxyError::mcp("Progress session not found"));
            }
        }

        // Send completion event
        self.send_event(ProgressEvent {
            session_id: session_id.to_string(),
            operation_id,
            event_type: ProgressEventType::SessionCompleted,
            timestamp: now,
            data: json!({
                "duration_seconds": duration_seconds,
                "result_summary": result_summary
            }).as_object().unwrap().clone().into_iter().collect(),
        }).await;

        info!("Completed progress session: {} in {} seconds", session_id, duration_seconds);
        Ok(())
    }

    /// Fail a progress session
    pub async fn fail_session(
        &self,
        session_id: &str,
        error_message: String,
        error_code: Option<String>,
    ) -> Result<()> {
        let now = Utc::now();
        let mut operation_id = String::new();
        let mut failed_at_percentage = 0.0;

        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                // Get current progress percentage
                failed_at_percentage = match &session.state {
                    ProgressState::InProgress { percentage, .. } => *percentage,
                    _ => 0.0,
                };

                session.state = ProgressState::Failed {
                    failed_at: now,
                    error_message: error_message.clone(),
                    failed_at_percentage,
                    error_code: error_code.clone(),
                };
                session.updated_at = now;
                operation_id = session.operation_id.clone();

                // Add failure snapshot
                session.history.push(ProgressSnapshot {
                    timestamp: now,
                    state: session.state.clone(),
                    context: json!({
                        "error_message": error_message,
                        "error_code": error_code,
                        "failed_at_percentage": failed_at_percentage
                    }).as_object().unwrap().clone().into_iter().collect(),
                });
            } else {
                return Err(ProxyError::mcp("Progress session not found"));
            }
        }

        // Send failure event
        self.send_event(ProgressEvent {
            session_id: session_id.to_string(),
            operation_id,
            event_type: ProgressEventType::SessionFailed,
            timestamp: now,
            data: json!({
                "error_message": error_message,
                "error_code": error_code,
                "failed_at_percentage": failed_at_percentage
            }).as_object().unwrap().clone().into_iter().collect(),
        }).await;

        warn!("Failed progress session: {} - {}", session_id, error_message);
        Ok(())
    }

    /// Cancel a progress session
    pub async fn cancel_session(
        &self,
        session_id: &str,
        reason: String,
    ) -> Result<()> {
        let now = Utc::now();
        let mut operation_id = String::new();
        let mut cancelled_at_percentage = 0.0;

        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                // Get current progress percentage
                cancelled_at_percentage = match &session.state {
                    ProgressState::InProgress { percentage, .. } => *percentage,
                    _ => 0.0,
                };

                session.state = ProgressState::Cancelled {
                    cancelled_at: now,
                    cancelled_at_percentage,
                    reason: reason.clone(),
                };
                session.updated_at = now;
                operation_id = session.operation_id.clone();

                // Add cancellation snapshot
                session.history.push(ProgressSnapshot {
                    timestamp: now,
                    state: session.state.clone(),
                    context: json!({
                        "reason": reason,
                        "cancelled_at_percentage": cancelled_at_percentage
                    }).as_object().unwrap().clone().into_iter().collect(),
                });
            } else {
                return Err(ProxyError::mcp("Progress session not found"));
            }
        }

        // Send cancellation event
        self.send_event(ProgressEvent {
            session_id: session_id.to_string(),
            operation_id,
            event_type: ProgressEventType::SessionCancelled,
            timestamp: now,
            data: json!({
                "reason": reason,
                "cancelled_at_percentage": cancelled_at_percentage
            }).as_object().unwrap().clone().into_iter().collect(),
        }).await;

        info!("Cancelled progress session: {} - {}", session_id, reason);
        Ok(())
    }

    /// Get progress session
    pub async fn get_session(&self, session_id: &str) -> Option<ProgressSession> {
        let sessions = self.active_sessions.read().await;
        sessions.get(session_id).map(|session| ProgressSession {
            id: session.id.clone(),
            operation_id: session.operation_id.clone(),
            created_at: session.created_at,
            updated_at: session.updated_at,
            expires_at: session.expires_at,
            state: session.state.clone(),
            metadata: session.metadata.clone(),
            sub_operations: session.sub_operations.clone(),
            history: session.history.clone(),
            update_sender: None, // Don't clone the sender
        })
    }

    /// Remove a completed session
    pub async fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;
        if sessions.remove(session_id).is_some() {
            debug!("Removed progress session: {}", session_id);
            Ok(())
        } else {
            Err(ProxyError::mcp("Progress session not found"))
        }
    }

    /// Subscribe to progress events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<ProgressEvent> {
        self.event_sender.subscribe()
    }

    /// Send a progress event
    async fn send_event(&self, event: ProgressEvent) {
        if self.config.enable_detailed_events {
            if let Err(e) = self.event_sender.send(event) {
                debug!("No subscribers for progress event: {}", e);
            }
        }
    }

    /// Start the cleanup task for expired sessions
    fn start_cleanup_task(&self) {
        let sessions = Arc::clone(&self.active_sessions);
        let event_sender = self.event_sender.clone();
        let cleanup_interval = Duration::from_secs(self.config.cleanup_interval_seconds);
        let enable_events = self.config.enable_detailed_events;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            
            loop {
                interval.tick().await;
                
                let expired_sessions = {
                    let mut sessions_guard = sessions.write().await;
                    let mut expired = Vec::new();
                    let now = Utc::now();
                    
                    let to_remove: Vec<String> = sessions_guard
                        .iter()
                        .filter(|(_, session)| now > session.expires_at)
                        .map(|(id, _)| id.clone())
                        .collect();
                    
                    for session_id in &to_remove {
                        if let Some(session) = sessions_guard.remove(session_id) {
                            expired.push((session_id.clone(), session));
                        }
                    }
                    
                    expired
                };

                // Send expiry events
                for (session_id, session) in expired_sessions {
                    debug!("Cleaning up expired progress session: {}", session_id);
                    
                    if enable_events {
                        let event = ProgressEvent {
                            session_id,
                            operation_id: session.operation_id,
                            event_type: ProgressEventType::SessionExpired,
                            timestamp: Utc::now(),
                            data: HashMap::new(),
                        };
                        
                        if let Err(e) = event_sender.send(event) {
                            debug!("No subscribers for cleanup event: {}", e);
                        }
                    }
                }
            }
        });
    }

    /// Get progress statistics
    pub async fn get_stats(&self) -> ProgressStats {
        let sessions = self.active_sessions.read().await;
        
        let mut sessions_by_state = HashMap::new();
        let mut total_duration = 0u64;
        let mut completed_count = 0;
        let mut failed_count = 0;
        let mut cancelled_count = 0;

        for session in sessions.values() {
            // Count by state
            let state_name = match &session.state {
                ProgressState::Initializing => "initializing",
                ProgressState::InProgress { .. } => "in_progress",
                ProgressState::Paused { .. } => "paused",
                ProgressState::Completed { .. } => {
                    completed_count += 1;
                    "completed"
                }
                ProgressState::Failed { .. } => {
                    failed_count += 1;
                    "failed"
                }
                ProgressState::Cancelled { .. } => {
                    cancelled_count += 1;
                    "cancelled"
                }
            };
            
            *sessions_by_state.entry(state_name.to_string()).or_insert(0) += 1;

            // Calculate duration for completed sessions
            if let ProgressState::Completed { duration_seconds, .. } = &session.state {
                total_duration += duration_seconds;
            }
        }

        let avg_duration = if completed_count > 0 {
            total_duration as f64 / completed_count as f64
        } else {
            0.0
        };

        ProgressStats {
            active_sessions: sessions.len(),
            sessions_by_state,
            avg_session_duration_seconds: avg_duration,
            total_completed: completed_count,
            total_failed: failed_count,
            total_cancelled: cancelled_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_session_creation() {
        let config = ProgressConfig::default();
        let tracker = ProgressTracker::new(config);
        
        let session_id = tracker.create_session(
            "test_operation".to_string(),
            HashMap::new(),
        ).await.unwrap();
        
        assert!(!session_id.is_empty());
        
        let session = tracker.get_session(&session_id).await.unwrap();
        assert_eq!(session.operation_id, "test_operation");
        assert!(matches!(session.state, ProgressState::Initializing));
    }

    #[tokio::test]
    async fn test_progress_update() {
        let config = ProgressConfig::default();
        let tracker = ProgressTracker::new(config);
        
        let session_id = tracker.create_session(
            "test_operation".to_string(),
            HashMap::new(),
        ).await.unwrap();
        
        let progress_state = ProgressState::InProgress {
            percentage: 50.0,
            current_step: "Processing data".to_string(),
            total_steps: Some(10),
            current_step_number: Some(5),
            eta_seconds: Some(60),
        };
        
        tracker.update_progress(
            &session_id,
            progress_state,
            Vec::new(),
            HashMap::new(),
        ).await.unwrap();
        
        let session = tracker.get_session(&session_id).await.unwrap();
        if let ProgressState::InProgress { percentage, .. } = session.state {
            assert_eq!(percentage, 50.0);
        } else {
            panic!("Expected InProgress state");
        }
    }

    #[tokio::test]
    async fn test_session_completion() {
        let config = ProgressConfig::default();
        let tracker = ProgressTracker::new(config);
        
        let session_id = tracker.create_session(
            "test_operation".to_string(),
            HashMap::new(),
        ).await.unwrap();
        
        tracker.complete_session(
            &session_id,
            Some("Operation completed successfully".to_string()),
        ).await.unwrap();
        
        let session = tracker.get_session(&session_id).await.unwrap();
        assert!(matches!(session.state, ProgressState::Completed { .. }));
    }

    #[tokio::test]
    async fn test_sub_operations() {
        let config = ProgressConfig::default();
        let tracker = ProgressTracker::new(config);
        
        let session_id = tracker.create_session(
            "test_operation".to_string(),
            HashMap::new(),
        ).await.unwrap();
        
        let sub_ops = vec![
            SubOperation {
                id: "sub1".to_string(),
                name: "Sub-operation 1".to_string(),
                state: SubOperationState::Pending,
                percentage: 0.0,
                started_at: None,
                ended_at: None,
                metadata: HashMap::new(),
            }
        ];
        
        tracker.add_sub_operations(&session_id, sub_ops).await.unwrap();
        
        tracker.start_sub_operation(&session_id, "sub1").await.unwrap();
        
        tracker.complete_sub_operation(
            &session_id,
            "sub1",
            HashMap::new(),
        ).await.unwrap();
        
        let session = tracker.get_session(&session_id).await.unwrap();
        assert_eq!(session.sub_operations.len(), 1);
        assert!(matches!(session.sub_operations[0].state, SubOperationState::Completed));
    }

    #[tokio::test]
    async fn test_event_subscription() {
        let config = ProgressConfig::default();
        let tracker = ProgressTracker::new(config);
        let mut event_receiver = tracker.subscribe_to_events();
        
        let session_id = tracker.create_session(
            "test_operation".to_string(),
            HashMap::new(),
        ).await.unwrap();
        
        // Should receive session creation event
        let event = event_receiver.recv().await.unwrap();
        assert_eq!(event.session_id, session_id);
        assert!(matches!(event.event_type, ProgressEventType::SessionCreated));
    }
}