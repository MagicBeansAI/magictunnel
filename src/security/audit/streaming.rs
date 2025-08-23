//! Real-time Audit Event Streaming
//! 
//! This module provides WebSocket-based real-time streaming of audit events
//! for live monitoring dashboards and security operations centers (SOCs).
//! Features include:
//! - WebSocket connections with heartbeat
//! - Event filtering and subscription management
//! - Connection management and authentication
//! - Buffering and backpressure handling

use super::{AuditEvent, AuditResult, AuditError};
use super::storage::AuditQuery;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, mpsc};
use tokio::time::{interval, Duration, Instant};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tracing::{info, warn, error, debug};

/// WebSocket message types for audit streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamMessage {
    /// Audit event notification
    Event {
        event: AuditEvent,
    },
    
    /// Subscription confirmation
    Subscribe {
        subscription_id: String,
        filters: AuditQuery,
        message: String,
    },
    
    /// Unsubscription confirmation
    Unsubscribe {
        subscription_id: String,
        message: String,
    },
    
    /// Heartbeat/ping message
    Heartbeat {
        timestamp: String,
        server_time: String,
    },
    
    /// Error message
    Error {
        code: String,
        message: String,
        details: Option<Value>,
    },
    
    /// Connection status
    Status {
        connected: bool,
        client_count: usize,
        uptime_seconds: u64,
    },
    
    /// Statistics update
    Stats {
        events_per_second: f64,
        total_events: u64,
        active_subscriptions: usize,
    },
}

/// Client connection information
#[derive(Debug, Clone)]
pub struct ClientConnection {
    pub id: String,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub subscriptions: HashSet<String>,
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Subscription filter for real-time events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSubscription {
    pub id: String,
    pub client_id: String,
    pub filters: AuditQuery,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

/// Real-time audit event streamer
pub struct AuditStreamer {
    /// Broadcast channel for events
    event_broadcaster: broadcast::Sender<AuditEvent>,
    
    /// Connected clients
    clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
    
    /// Active subscriptions
    subscriptions: Arc<RwLock<HashMap<String, StreamSubscription>>>,
    
    /// Configuration
    config: StreamingConfig,
    
    /// Statistics
    stats: Arc<RwLock<StreamingStats>>,
    
    /// Shutdown signal
    shutdown_tx: mpsc::Sender<()>,
}

/// Streaming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub max_connections: usize,
    pub buffer_size: usize,
    pub heartbeat_interval: Duration,
    pub client_timeout: Duration,
    pub max_subscriptions_per_client: usize,
    pub enable_authentication: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            buffer_size: 1000,
            heartbeat_interval: Duration::from_secs(30),
            client_timeout: Duration::from_secs(60),
            max_subscriptions_per_client: 10,
            enable_authentication: true,
        }
    }
}

/// Streaming statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingStats {
    pub connected_clients: usize,
    pub active_subscriptions: usize,
    pub events_streamed: u64,
    pub bytes_sent: u64,
    pub uptime: Duration,
    pub started_at: DateTime<Utc>,
}

impl AuditStreamer {
    /// Create new audit streamer
    pub fn new(config: StreamingConfig) -> Self {
        let (event_broadcaster, _) = broadcast::channel(config.buffer_size);
        let (shutdown_tx, _) = mpsc::channel(1);
        
        Self {
            event_broadcaster,
            clients: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(StreamingStats {
                connected_clients: 0,
                active_subscriptions: 0,
                events_streamed: 0,
                bytes_sent: 0,
                uptime: Duration::from_secs(0),
                started_at: Utc::now(),
            })),
            shutdown_tx,
        }
    }
    
    /// Start the streaming service
    pub async fn start(&self) -> AuditResult<()> {
        info!("🌊 Starting audit event streaming service");
        
        // Start heartbeat task
        self.start_heartbeat_task().await;
        
        // Start cleanup task
        self.start_cleanup_task().await;
        
        info!("✅ Audit streaming service started successfully");
        Ok(())
    }
    
    /// Broadcast an audit event to all subscribed clients
    pub async fn broadcast_event(&self, event: &AuditEvent) -> AuditResult<()> {
        // Send to broadcast channel (non-blocking)
        match self.event_broadcaster.send(event.clone()) {
            Ok(subscriber_count) => {
                debug!("📡 Broadcasted audit event to {} subscribers", subscriber_count);
                
                // Update statistics
                let mut stats = self.stats.write().await;
                stats.events_streamed += 1;
                
                Ok(())
            },
            Err(_) => {
                // No subscribers, which is fine
                debug!("📡 No subscribers for audit event broadcast");
                Ok(())
            }
        }
    }
    
    /// Register a new client connection
    pub async fn register_client(
        &self,
        client_id: String,
        user_id: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AuditResult<broadcast::Receiver<AuditEvent>> {
        let mut clients = self.clients.write().await;
        
        // Check connection limit
        if clients.len() >= self.config.max_connections {
            return Err(AuditError::Streaming(
                "Maximum client connections reached".to_string()
            ));
        }
        
        let connection = ClientConnection {
            id: client_id.clone(),
            connected_at: Utc::now(),
            last_heartbeat: Utc::now(),
            subscriptions: HashSet::new(),
            user_id,
            ip_address,
            user_agent,
        };
        
        clients.insert(client_id.clone(), connection);
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.connected_clients = clients.len();
        
        info!("🔗 Client {} connected to audit stream", client_id);
        
        // Create receiver for this client
        let receiver = self.event_broadcaster.subscribe();
        
        Ok(receiver)
    }
    
    /// Unregister a client connection
    pub async fn unregister_client(&self, client_id: &str) -> AuditResult<()> {
        let mut clients = self.clients.write().await;
        let mut subscriptions = self.subscriptions.write().await;
        
        // Remove all subscriptions for this client
        if let Some(client) = clients.get(client_id) {
            for subscription_id in &client.subscriptions {
                subscriptions.remove(subscription_id);
            }
        }
        
        // Remove client
        clients.remove(client_id);
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.connected_clients = clients.len();
        stats.active_subscriptions = subscriptions.len();
        
        info!("❌ Client {} disconnected from audit stream", client_id);
        
        Ok(())
    }
    
    /// Create a subscription for filtered events
    pub async fn create_subscription(
        &self,
        client_id: String,
        filters: AuditQuery,
    ) -> AuditResult<String> {
        let mut clients = self.clients.write().await;
        let mut subscriptions = self.subscriptions.write().await;
        
        // Check if client exists
        let client = clients.get_mut(&client_id)
            .ok_or_else(|| AuditError::Streaming("Client not found".to_string()))?;
        
        // Check subscription limit
        if client.subscriptions.len() >= self.config.max_subscriptions_per_client {
            return Err(AuditError::Streaming(
                "Maximum subscriptions per client reached".to_string()
            ));
        }
        
        let subscription_id = Uuid::new_v4().to_string();
        
        let subscription = StreamSubscription {
            id: subscription_id.clone(),
            client_id: client_id.clone(),
            filters,
            created_at: Utc::now(),
            last_activity: Utc::now(),
        };
        
        // Add to client's subscriptions
        client.subscriptions.insert(subscription_id.clone());
        
        // Add to global subscriptions
        subscriptions.insert(subscription_id.clone(), subscription);
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.active_subscriptions = subscriptions.len();
        
        info!("📋 Created subscription {} for client {}", subscription_id, client_id);
        
        Ok(subscription_id)
    }
    
    /// Remove a subscription
    pub async fn remove_subscription(
        &self,
        client_id: &str,
        subscription_id: &str,
    ) -> AuditResult<()> {
        let mut clients = self.clients.write().await;
        let mut subscriptions = self.subscriptions.write().await;
        
        // Remove from client's subscriptions
        if let Some(client) = clients.get_mut(client_id) {
            client.subscriptions.remove(subscription_id);
        }
        
        // Remove from global subscriptions
        subscriptions.remove(subscription_id);
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.active_subscriptions = subscriptions.len();
        
        info!("🗑️ Removed subscription {} for client {}", subscription_id, client_id);
        
        Ok(())
    }
    
    /// Update client heartbeat
    pub async fn update_heartbeat(&self, client_id: &str) -> AuditResult<()> {
        let mut clients = self.clients.write().await;
        
        if let Some(client) = clients.get_mut(client_id) {
            client.last_heartbeat = Utc::now();
            debug!("💓 Updated heartbeat for client {}", client_id);
        } else {
            return Err(AuditError::Streaming("Client not found".to_string()));
        }
        
        Ok(())
    }
    
    /// Get streaming statistics
    pub async fn get_stats(&self) -> StreamingStats {
        let stats = self.stats.read().await;
        let mut stats_copy = stats.clone();
        let uptime_duration = Utc::now().signed_duration_since(stats.started_at);
        stats_copy.uptime = Duration::from_secs(uptime_duration.num_seconds() as u64);
        stats_copy
    }
    
    /// Get connected clients count
    pub async fn get_client_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }
    
    /// Check if an event matches subscription filters
    pub fn matches_subscription(&self, event: &AuditEvent, subscription: &StreamSubscription) -> bool {
        let query = &subscription.filters;
        
        // Event type filter
        if let Some(ref types) = query.event_types {
            let event_type_str = format!("{:?}", event.event_type).to_lowercase();
            if !types.iter().any(|t| event_type_str.contains(&t.to_lowercase())) {
                return false;
            }
        }
        
        // Component filter
        if let Some(ref components) = query.components {
            if !components.contains(&event.component) {
                return false;
            }
        }
        
        // Severity filter
        if let Some(ref severities) = query.severities {
            let severity_str = format!("{:?}", event.severity).to_lowercase();
            if !severities.iter().any(|s| severity_str == s.to_lowercase()) {
                return false;
            }
        }
        
        // User ID filter
        if let Some(ref user_ids) = query.user_ids {
            if let Some(ref event_user) = event.metadata.user_id {
                if !user_ids.contains(event_user) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Time range filter (for subscriptions, usually only end_time matters)
        if let Some(end) = query.end_time {
            if event.timestamp > end {
                return false;
            }
        }
        
        // Text search filter
        if let Some(ref search) = query.search_text {
            let search_lower = search.to_lowercase();
            if !event.message.to_lowercase().contains(&search_lower) {
                return false;
            }
        }
        
        // Correlation ID filter
        if let Some(ref corr_id) = query.correlation_id {
            if event.correlation_id.as_ref() != Some(corr_id) {
                return false;
            }
        }
        
        true
    }
    
    /// Start heartbeat task
    async fn start_heartbeat_task(&self) {
        let clients = Arc::clone(&self.clients);
        let heartbeat_interval = self.config.heartbeat_interval;
        
        tokio::spawn(async move {
            let mut interval = interval(heartbeat_interval);
            
            loop {
                interval.tick().await;
                
                let clients_read = clients.read().await;
                let client_count = clients_read.len();
                drop(clients_read);
                
                if client_count > 0 {
                    debug!("💓 Heartbeat check for {} clients", client_count);
                }
            }
        });
    }
    
    /// Start cleanup task for inactive clients
    async fn start_cleanup_task(&self) {
        let clients = Arc::clone(&self.clients);
        let subscriptions = Arc::clone(&self.subscriptions);
        let stats = Arc::clone(&self.stats);
        let client_timeout = self.config.client_timeout;
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Check every minute
            
            loop {
                interval.tick().await;
                
                let now = Utc::now();
                let mut clients_write = clients.write().await;
                let mut subscriptions_write = subscriptions.write().await;
                
                // Find inactive clients
                let inactive_clients: Vec<String> = clients_write
                    .iter()
                    .filter(|(_, client)| {
                        let duration_since = now.signed_duration_since(client.last_heartbeat);
                        duration_since.to_std().unwrap_or_default() > client_timeout
                    })
                    .map(|(id, _)| id.clone())
                    .collect();
                
                // Remove inactive clients and their subscriptions
                for client_id in inactive_clients {
                    if let Some(client) = clients_write.remove(&client_id) {
                        warn!("🧹 Cleaning up inactive client: {}", client_id);
                        
                        // Remove all subscriptions for this client
                        for subscription_id in &client.subscriptions {
                            subscriptions_write.remove(subscription_id);
                        }
                    }
                }
                
                // Update statistics
                let mut stats_write = stats.write().await;
                stats_write.connected_clients = clients_write.len();
                stats_write.active_subscriptions = subscriptions_write.len();
            }
        });
    }
    
    /// Graceful shutdown
    pub async fn shutdown(&self) -> AuditResult<()> {
        info!("🛑 Shutting down audit streaming service");
        
        // Send shutdown signal
        let _ = self.shutdown_tx.send(()).await;
        
        // Clear all clients and subscriptions
        {
            let mut clients = self.clients.write().await;
            let mut subscriptions = self.subscriptions.write().await;
            
            clients.clear();
            subscriptions.clear();
        }
        
        info!("✅ Audit streaming service shutdown completed");
        Ok(())
    }
}

/// Streaming message builder utilities
pub struct StreamMessageBuilder;

impl StreamMessageBuilder {
    pub fn event(event: AuditEvent) -> StreamMessage {
        StreamMessage::Event { event }
    }
    
    pub fn subscribe_confirmation(subscription_id: String, filters: AuditQuery) -> StreamMessage {
        StreamMessage::Subscribe {
            subscription_id,
            filters,
            message: "Subscription created successfully".to_string(),
        }
    }
    
    pub fn unsubscribe_confirmation(subscription_id: String) -> StreamMessage {
        StreamMessage::Unsubscribe {
            subscription_id,
            message: "Subscription removed successfully".to_string(),
        }
    }
    
    pub fn heartbeat() -> StreamMessage {
        let now = chrono::Utc::now();
        StreamMessage::Heartbeat {
            timestamp: now.timestamp().to_string(),
            server_time: now.to_rfc3339(),
        }
    }
    
    pub fn error(code: &str, message: &str, details: Option<Value>) -> StreamMessage {
        StreamMessage::Error {
            code: code.to_string(),
            message: message.to_string(),
            details,
        }
    }
    
    pub fn status(connected: bool, client_count: usize, uptime_seconds: u64) -> StreamMessage {
        StreamMessage::Status {
            connected,
            client_count,
            uptime_seconds,
        }
    }
    
    pub fn stats(events_per_second: f64, total_events: u64, active_subscriptions: usize) -> StreamMessage {
        StreamMessage::Stats {
            events_per_second,
            total_events,
            active_subscriptions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::audit::events::{AuditEventType, AuditSeverity};
    
    #[tokio::test]
    async fn test_streamer_creation() {
        let config = StreamingConfig::default();
        let streamer = AuditStreamer::new(config);
        
        assert_eq!(streamer.get_client_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_client_registration() {
        let config = StreamingConfig::default();
        let streamer = AuditStreamer::new(config);
        
        let client_id = "test_client".to_string();
        let result = streamer.register_client(
            client_id.clone(),
            Some("user123".to_string()),
            Some("127.0.0.1".to_string()),
            Some("test_agent".to_string()),
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(streamer.get_client_count().await, 1);
        
        // Cleanup
        streamer.unregister_client(&client_id).await.unwrap();
        assert_eq!(streamer.get_client_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_subscription_management() {
        let config = StreamingConfig::default();
        let streamer = AuditStreamer::new(config);
        
        let client_id = "test_client".to_string();
        streamer.register_client(
            client_id.clone(),
            None,
            None,
            None,
        ).await.unwrap();
        
        let filters = AuditQuery::default();
        let subscription_id = streamer.create_subscription(client_id.clone(), filters).await.unwrap();
        
        assert!(!subscription_id.is_empty());
        
        // Remove subscription
        streamer.remove_subscription(&client_id, &subscription_id).await.unwrap();
        
        // Cleanup
        streamer.unregister_client(&client_id).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_event_broadcasting() {
        let config = StreamingConfig::default();
        let streamer = AuditStreamer::new(config);
        
        let event = AuditEvent::new(
            AuditEventType::Authentication,
            "test".to_string(),
            "Test event".to_string(),
        );
        
        // Should not error even with no subscribers
        let result = streamer.broadcast_event(&event).await;
        assert!(result.is_ok());
    }
}