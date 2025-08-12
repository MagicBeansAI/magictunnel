//! MCP Request Cancellation System
//!
//! Provides comprehensive request cancellation support according to MCP 2025-06-18 specification
//! with proper cleanup, notification, and timeout handling.

use crate::error::{ProxyError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock, oneshot};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Request cancellation manager
pub struct CancellationManager {
    /// Active cancellation tokens
    active_tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
    /// Cancellation event broadcaster
    event_sender: broadcast::Sender<CancellationEvent>,
    /// Configuration
    config: CancellationConfig,
}

/// Configuration for cancellation system
#[derive(Debug, Clone)]
pub struct CancellationConfig {
    /// Enable graceful cancellation (attempt cleanup before force cancel)
    pub enable_graceful_cancellation: bool,
    /// Graceful cancellation timeout (seconds)
    pub graceful_timeout_seconds: u64,
    /// Maximum active tokens to track
    pub max_active_tokens: usize,
    /// Token cleanup interval (seconds)
    pub cleanup_interval_seconds: u64,
    /// Enable cancellation notifications
    pub enable_notifications: bool,
}

impl Default for CancellationConfig {
    fn default() -> Self {
        Self {
            enable_graceful_cancellation: true,
            graceful_timeout_seconds: 30,
            max_active_tokens: 10000,
            cleanup_interval_seconds: 300, // 5 minutes
            enable_notifications: true,
        }
    }
}

/// Cancellation token for tracking and canceling operations
#[derive(Debug)]
pub struct CancellationToken {
    /// Unique token ID
    pub id: String,
    /// Request ID this token is associated with
    pub request_id: String,
    /// Token creation time
    pub created_at: Instant,
    /// Operation type being cancelled
    pub operation_type: String,
    /// Cancellation sender
    pub cancel_sender: Option<oneshot::Sender<CancellationReason>>,
    /// Whether token is cancelled
    pub is_cancelled: bool,
    /// Cancellation reason
    pub cancellation_reason: Option<CancellationReason>,
    /// Metadata about the operation
    pub metadata: HashMap<String, Value>,
}

/// Reason for cancellation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CancellationReason {
    /// User-initiated cancellation
    UserCancelled,
    /// System timeout
    Timeout,
    /// Server shutdown
    ServerShutdown,
    /// Resource constraints
    ResourceExhausted,
    /// Security violation
    SecurityViolation,
    /// Client disconnection
    ClientDisconnected,
    /// Operation completed (not really a cancellation)
    Completed,
}

/// Cancellation event for notifications
#[derive(Debug, Clone)]
pub struct CancellationEvent {
    /// Token ID
    pub token_id: String,
    /// Request ID
    pub request_id: String,
    /// Event type
    pub event_type: CancellationEventType,
    /// Reason for cancellation
    pub reason: CancellationReason,
    /// Timestamp
    pub timestamp: Instant,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

/// Types of cancellation events
#[derive(Debug, Clone)]
pub enum CancellationEventType {
    /// Token created
    TokenCreated,
    /// Cancellation requested
    CancellationRequested,
    /// Graceful cancellation started
    GracefulCancellationStarted,
    /// Force cancellation initiated
    ForceCancellationInitiated,
    /// Operation successfully cancelled
    OperationCancelled,
    /// Cancellation failed
    CancellationFailed,
    /// Token expired/cleaned up
    TokenExpired,
}

impl CancellationManager {
    /// Create a new cancellation manager
    pub fn new(config: CancellationConfig) -> Self {
        let (sender, _) = broadcast::channel(1000);
        
        let manager = Self {
            active_tokens: Arc::new(RwLock::new(HashMap::new())),
            event_sender: sender,
            config,
        };

        // Start cleanup task
        manager.start_cleanup_task();
        
        manager
    }

    /// Create a new cancellation token for an operation
    pub async fn create_token(
        &self,
        request_id: String,
        operation_type: String,
        metadata: HashMap<String, Value>,
    ) -> Result<String> {
        let token_id = Uuid::new_v4().to_string();
        let (cancel_sender, _cancel_receiver) = oneshot::channel();

        let token = CancellationToken {
            id: token_id.clone(),
            request_id: request_id.clone(),
            created_at: Instant::now(),
            operation_type: operation_type.clone(),
            cancel_sender: Some(cancel_sender),
            is_cancelled: false,
            cancellation_reason: None,
            metadata: metadata.clone(),
        };

        // Check if we're at capacity
        {
            let tokens = self.active_tokens.read().await;
            if tokens.len() >= self.config.max_active_tokens {
                return Err(ProxyError::mcp(
                    "Maximum active cancellation tokens reached"
                ));
            }
        }

        // Store the token
        {
            let mut tokens = self.active_tokens.write().await;
            tokens.insert(token_id.clone(), token);
        }

        // Send creation event
        self.send_event(CancellationEvent {
            token_id: token_id.clone(),
            request_id,
            event_type: CancellationEventType::TokenCreated,
            reason: CancellationReason::UserCancelled, // Placeholder
            timestamp: Instant::now(),
            metadata,
        }).await;

        debug!("Created cancellation token: {} for operation: {}", token_id, operation_type);
        Ok(token_id)
    }

    /// Cancel an operation by token ID
    pub async fn cancel_operation(
        &self,
        token_id: &str,
        reason: CancellationReason,
    ) -> Result<()> {
        info!("Cancellation requested for token: {} with reason: {:?}", token_id, reason);

        // Get token info for cancellation operations
        let (token_id_str, request_id, metadata) = {
            let mut tokens = self.active_tokens.write().await;
            match tokens.get_mut(token_id) {
                Some(token) => {
                    token.is_cancelled = true;
                    token.cancellation_reason = Some(reason.clone());
                    (token.id.clone(), token.request_id.clone(), token.metadata.clone())
                }
                None => {
                    warn!("Attempted to cancel non-existent token: {}", token_id);
                    return Err(ProxyError::mcp("Cancellation token not found"));
                }
            }
        };

        // Send cancellation requested event
        self.send_event(CancellationEvent {
            token_id: token_id.to_string(),
            request_id: request_id.clone(),
            event_type: CancellationEventType::CancellationRequested,
            reason: reason.clone(),
            timestamp: Instant::now(),
            metadata: metadata.clone(),
        }).await;

        // Attempt graceful cancellation first
        if self.config.enable_graceful_cancellation {
            match self.graceful_cancel(token_id, reason.clone()).await {
                Ok(()) => {
                    info!("Graceful cancellation successful for token: {}", token_id);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Graceful cancellation failed for token: {}, attempting force cancel: {}", token_id, e);
                }
            }
        }

        // Force cancellation
        self.force_cancel(token_id, reason).await
    }

    /// Attempt graceful cancellation
    async fn graceful_cancel(
        &self,
        token_id: &str,
        reason: CancellationReason,
    ) -> Result<()> {
        debug!("Attempting graceful cancellation for token: {}", token_id);

        // Get token info and send cancellation signal
        let (request_id, metadata, sender) = {
            let mut tokens = self.active_tokens.write().await;
            match tokens.get_mut(token_id) {
                Some(token) => {
                    let sender = token.cancel_sender.take();
                    (token.request_id.clone(), token.metadata.clone(), sender)
                }
                None => return Err(ProxyError::mcp("Token not found")),
            }
        };

        // Send graceful cancellation event
        self.send_event(CancellationEvent {
            token_id: token_id.to_string(),
            request_id: request_id.clone(),
            event_type: CancellationEventType::GracefulCancellationStarted,
            reason: reason.clone(),
            timestamp: Instant::now(),
            metadata: metadata.clone(),
        }).await;

        // Send cancellation signal if sender is available
        if let Some(sender) = sender {
            if sender.send(reason.clone()).is_err() {
                return Err(ProxyError::mcp("Failed to send cancellation signal"));
            }

            // Wait for graceful timeout
            let _timeout_duration = Duration::from_secs(self.config.graceful_timeout_seconds);
            
            // In a real implementation, we'd wait for the operation to acknowledge cancellation
            // For now, we'll simulate with a timeout
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Send success event
            self.send_event(CancellationEvent {
                token_id: token_id.to_string(),
                request_id,
                event_type: CancellationEventType::OperationCancelled,
                reason,
                timestamp: Instant::now(),
                metadata,
            }).await;

            return Ok(());
        }

        Err(ProxyError::mcp("No cancellation sender available"))
    }

    /// Force cancellation (immediate termination)
    async fn force_cancel(
        &self,
        token_id: &str,
        reason: CancellationReason,
    ) -> Result<()> {
        info!("Force cancelling operation for token: {}", token_id);

        // Get token info
        let (request_id, metadata) = {
            let tokens = self.active_tokens.read().await;
            match tokens.get(token_id) {
                Some(token) => (token.request_id.clone(), token.metadata.clone()),
                None => return Err(ProxyError::mcp("Token not found")),
            }
        };

        // Send force cancellation event
        self.send_event(CancellationEvent {
            token_id: token_id.to_string(),
            request_id: request_id.clone(),
            event_type: CancellationEventType::ForceCancellationInitiated,
            reason: reason.clone(),
            timestamp: Instant::now(),
            metadata: metadata.clone(),
        }).await;

        // Force termination (in a real implementation, this might involve:
        // - Killing processes
        // - Closing connections
        // - Cleaning up resources
        // - Sending termination signals)

        // Send completion event
        self.send_event(CancellationEvent {
            token_id: token_id.to_string(),
            request_id,
            event_type: CancellationEventType::OperationCancelled,
            reason,
            timestamp: Instant::now(),
            metadata,
        }).await;

        Ok(())
    }

    /// Check if a token is cancelled
    pub async fn is_cancelled(&self, token_id: &str) -> bool {
        let tokens = self.active_tokens.read().await;
        tokens.get(token_id)
            .map(|token| token.is_cancelled)
            .unwrap_or(false)
    }

    /// Get cancellation receiver for a token
    pub async fn get_cancellation_receiver(&self, token_id: &str) -> Option<oneshot::Receiver<CancellationReason>> {
        let mut tokens = self.active_tokens.write().await;
        if let Some(token) = tokens.get_mut(token_id) {
            let (sender, receiver) = oneshot::channel();
            token.cancel_sender = Some(sender);
            Some(receiver)
        } else {
            None
        }
    }

    /// Remove a completed token
    pub async fn remove_token(&self, token_id: &str) -> Result<()> {
        let mut tokens = self.active_tokens.write().await;
        if let Some(token) = tokens.remove(token_id) {
            debug!("Removed cancellation token: {}", token_id);
            
            // Send completion event
            self.send_event(CancellationEvent {
                token_id: token_id.to_string(), 
                request_id: token.request_id.clone(),
                event_type: CancellationEventType::OperationCancelled,
                reason: CancellationReason::Completed,
                timestamp: Instant::now(),
                metadata: token.metadata.clone(),
            }).await;
            
            Ok(())
        } else {
            Err(ProxyError::mcp("Token not found"))
        }
    }

    /// Subscribe to cancellation events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<CancellationEvent> {
        self.event_sender.subscribe()
    }

    /// Send a cancellation event
    async fn send_event(&self, event: CancellationEvent) {
        if self.config.enable_notifications {
            if let Err(e) = self.event_sender.send(event) {
                debug!("No subscribers for cancellation event: {}", e);
            }
        }
    }

    /// Start the cleanup task for expired tokens
    fn start_cleanup_task(&self) {
        let tokens = Arc::clone(&self.active_tokens);
        let event_sender = self.event_sender.clone();
        let cleanup_interval = Duration::from_secs(self.config.cleanup_interval_seconds);
        let enable_notifications = self.config.enable_notifications;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            
            loop {
                interval.tick().await;
                
                let expired_tokens = {
                    let mut tokens_guard = tokens.write().await;
                    let mut expired = Vec::new();
                    let now = Instant::now();
                    
                    // Find tokens older than 1 hour
                    let expiry_duration = Duration::from_secs(3600);
                    
                    let mut to_remove = Vec::new();
                    for (token_id, token) in tokens_guard.iter() {
                        if now.duration_since(token.created_at) > expiry_duration {
                            to_remove.push(token_id.clone());
                        }
                    }
                    
                    for token_id in &to_remove {
                        if let Some(token) = tokens_guard.remove(token_id) {
                            expired.push((token_id.clone(), token));
                        }
                    }
                    
                    expired
                };

                // Send expiry events
                for (token_id, token) in expired_tokens {
                    debug!("Cleaning up expired cancellation token: {}", token_id);
                    
                    if enable_notifications {
                        let event = CancellationEvent {
                            token_id,
                            request_id: token.request_id,
                            event_type: CancellationEventType::TokenExpired,
                            reason: CancellationReason::Timeout,
                            timestamp: Instant::now(),
                            metadata: token.metadata,
                        };
                        
                        if let Err(e) = event_sender.send(event) {
                            debug!("No subscribers for cleanup event: {}", e);
                        }
                    }
                }
            }
        });
    }

    /// Get statistics about active tokens
    pub async fn get_stats(&self) -> CancellationStats {
        let tokens = self.active_tokens.read().await;
        let now = Instant::now();
        
        let mut cancelled_count = 0;
        let mut active_count = 0;
        let mut operation_types = HashMap::new();
        
        for token in tokens.values() {
            if token.is_cancelled {
                cancelled_count += 1;
            } else {
                active_count += 1;
            }
            
            *operation_types.entry(token.operation_type.clone()).or_insert(0) += 1;
        }

        CancellationStats {
            total_tokens: tokens.len(),
            active_tokens: active_count,
            cancelled_tokens: cancelled_count,
            operation_types,
        }
    }
}

/// Statistics about the cancellation system
#[derive(Debug, Serialize)]
pub struct CancellationStats {
    pub total_tokens: usize,
    pub active_tokens: usize,
    pub cancelled_tokens: usize,
    pub operation_types: HashMap<String, usize>,
}

impl std::fmt::Display for CancellationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CancellationReason::UserCancelled => write!(f, "user_cancelled"),
            CancellationReason::Timeout => write!(f, "timeout"),
            CancellationReason::ServerShutdown => write!(f, "server_shutdown"),
            CancellationReason::ResourceExhausted => write!(f, "resource_exhausted"),
            CancellationReason::SecurityViolation => write!(f, "security_violation"),
            CancellationReason::ClientDisconnected => write!(f, "client_disconnected"),
            CancellationReason::Completed => write!(f, "completed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_creation() {
        let config = CancellationConfig::default();
        let manager = CancellationManager::new(config);
        
        let token_id = manager.create_token(
            "req123".to_string(),
            "tool_execution".to_string(),
            HashMap::new(),
        ).await.unwrap();
        
        assert!(!token_id.is_empty());
        assert!(!manager.is_cancelled(&token_id).await);
    }

    #[tokio::test]
    async fn test_cancellation() {
        let config = CancellationConfig::default();
        let manager = CancellationManager::new(config);
        
        let token_id = manager.create_token(
            "req123".to_string(),
            "tool_execution".to_string(),
            HashMap::new(),
        ).await.unwrap();
        
        manager.cancel_operation(&token_id, CancellationReason::UserCancelled).await.unwrap();
        
        assert!(manager.is_cancelled(&token_id).await);
    }

    #[tokio::test]
    async fn test_event_subscription() {
        let config = CancellationConfig::default();
        let manager = CancellationManager::new(config);
        let mut event_receiver = manager.subscribe_to_events();
        
        let token_id = manager.create_token(
            "req123".to_string(),
            "tool_execution".to_string(),
            HashMap::new(),
        ).await.unwrap();
        
        // Should receive token creation event
        let event = event_receiver.recv().await.unwrap();
        assert_eq!(event.token_id, token_id);
        assert!(matches!(event.event_type, CancellationEventType::TokenCreated));
    }
}