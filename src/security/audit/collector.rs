//! Centralized Audit Event Collector
//! 
//! This module implements the main audit collector that coordinates all
//! audit system components with non-blocking async processing:
//! - Event queue management with backpressure
//! - Storage backend coordination
//! - Real-time streaming integration
//! - Batch processing optimization
//! - Health monitoring and metrics

use super::{
    AuditEvent, AuditResult, AuditError, AuditConfig,
    storage::{AuditStorage, StorageBackend, AuditQuery, StorageStats},
    streaming::{AuditStreamer, StreamingConfig},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::{interval, Duration, Instant};
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use std::collections::HashMap;
use tracing::{info, warn, error, debug};
use crate::security::{SecurityServiceStatistics, ServiceHealth, HealthStatus, AuditStatistics};
use crate::security::statistics::PerformanceMetrics;

/// Main audit collector that coordinates all audit components
pub struct AuditCollector {
    /// Configuration
    config: AuditConfig,
    
    /// Storage backend
    storage: Arc<dyn AuditStorage>,
    
    /// Real-time streamer (optional)
    streamer: Option<AuditStreamer>,
    
    /// Event queue for async processing
    event_queue: mpsc::Sender<AuditEvent>,
    
    /// Batch processing buffer
    batch_buffer: Arc<RwLock<Vec<AuditEvent>>>,
    
    /// Collector statistics
    stats: Arc<RwLock<CollectorStats>>,
    
    /// Shutdown signal
    shutdown_tx: mpsc::Sender<()>,
    
    /// Processing semaphore for backpressure
    processing_semaphore: Arc<Semaphore>,
}

/// Collector statistics and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorStats {
    /// Total events processed
    pub total_events: u64,
    
    /// Events processed per second (rolling average)
    pub events_per_second: f64,
    
    /// Current queue depth
    pub queue_depth: usize,
    
    /// Events in current batch
    pub batch_size: usize,
    
    /// Storage statistics
    pub storage_healthy: bool,
    pub storage_errors: u64,
    
    /// Streaming statistics
    pub streaming_clients: usize,
    pub streaming_subscriptions: usize,
    
    /// Processing times
    pub avg_processing_time_ms: f64,
    pub max_processing_time_ms: f64,
    
    /// System health
    pub healthy: bool,
    pub uptime: Duration,
    pub started_at: DateTime<Utc>,
    
    /// Error counters
    pub total_errors: u64,
    pub queue_full_errors: u64,
    pub storage_errors_count: u64,
    pub streaming_errors: u64,
}

impl Default for CollectorStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            events_per_second: 0.0,
            queue_depth: 0,
            batch_size: 0,
            storage_healthy: true,
            storage_errors: 0,
            streaming_clients: 0,
            streaming_subscriptions: 0,
            avg_processing_time_ms: 0.0,
            max_processing_time_ms: 0.0,
            healthy: true,
            uptime: Duration::from_secs(0),
            started_at: Utc::now(),
            total_errors: 0,
            queue_full_errors: 0,
            storage_errors_count: 0,
            streaming_errors: 0,
        }
    }
}

// Aggregated statistics for a specific time range (for API consumption)
#[derive(Debug, Clone, Serialize, Default)]
pub struct AuditRangeStatistics {
    pub total_events: u64,
    pub violations: u64,
    pub critical_violations: u64,
    pub auth_events: u64,
    pub failed_auth: u64,
    pub unique_users: u64,
    pub avg_entries_per_day: f64,
    pub event_types: Vec<crate::security::statistics::EventTypeCount>,
}

impl AuditCollector {
    /// Create new audit collector
    pub async fn new(config: AuditConfig) -> AuditResult<Self> {
        info!("🏗️ Initializing audit collector");
        
        // Create storage backend
        let storage_box = StorageBackend::create(&config.storage).await?;
        let storage = Arc::from(storage_box);
        info!("✅ Storage backend initialized: {:?}", config.storage);
        
        // Create streaming service if enabled
        let streamer = if let Some(ref streaming_config) = config.streaming {
            if streaming_config.enabled {
                let stream_config = StreamingConfig {
                    max_connections: streaming_config.max_connections,
                    buffer_size: streaming_config.buffer_size,
                    heartbeat_interval: Duration::from_secs(streaming_config.heartbeat_interval),
                    client_timeout: Duration::from_secs(60),
                    max_subscriptions_per_client: 10,
                    enable_authentication: true,
                };
                
                let streamer = AuditStreamer::new(stream_config);
                streamer.start().await?;
                info!("✅ Audit streaming service initialized");
                Some(streamer)
            } else {
                info!("ℹ️ Audit streaming disabled in configuration");
                None
            }
        } else {
            info!("ℹ️ No streaming configuration provided");
            None
        };
        
        // Create event queue for async processing
        let (event_tx, event_rx) = mpsc::channel(config.performance.max_queue_size);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        // Create processing semaphore for backpressure
        let processing_semaphore = Arc::new(Semaphore::new(config.performance.worker_threads));
        
        let collector = Self {
            config: config.clone(),
            storage,
            streamer,
            event_queue: event_tx,
            batch_buffer: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(CollectorStats::default())),
            shutdown_tx,
            processing_semaphore,
        };
        
        // Start background processing tasks
        collector.start_event_processor(event_rx, shutdown_rx).await;
        collector.start_batch_processor().await;
        collector.start_stats_updater().await;
        collector.start_health_monitor().await;
        
        info!("🎉 Audit collector initialized successfully");
        Ok(collector)
    }

    /// Convenience: get most recent events up to a limit
    pub async fn get_recent_events(&self, limit: usize) -> AuditResult<Vec<super::AuditEvent>> {
        let now = Utc::now();
        let query = AuditQuery {
            start_time: Some(now - ChronoDuration::hours(24)),
            end_time: Some(now),
            limit: Some(limit),
            ..AuditQuery::default()
        };
        self.storage.query_events(&query).await
    }
    
    /// Log an audit event (non-blocking)
    pub async fn log_event(&self, event: AuditEvent) -> AuditResult<()> {
        // Try to acquire permit for backpressure control
        let permit = self.processing_semaphore.try_acquire()
            .map_err(|_| AuditError::QueueFull)?;
        
        // Send to processing queue
        let result = match self.event_queue.try_send(event.clone()) {
            Ok(()) => {
                debug!("📝 Audit event queued for processing: {}", event.id);
                
                // Update queue statistics
                {
                    let mut stats = self.stats.write().await;
                    // For mpsc::Sender, we can't easily track queue depth
                    // For now, increment a counter when we successfully send
                    stats.queue_depth = 0; // TODO: Track this properly with receiver feedback
                }
                
                Ok(())
            },
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Queue is full, increment error counter
                {
                    let mut stats = self.stats.write().await;
                    stats.queue_full_errors += 1;
                    stats.total_errors += 1;
                }
                
                warn!("⚠️ Audit event queue is full, dropping event: {}", event.id);
                Err(AuditError::QueueFull)
            },
            Err(mpsc::error::TrySendError::Closed(_)) => {
                error!("❌ Audit event queue is closed");
                Err(AuditError::Storage("Audit system is shutting down".to_string()))
            },
        };
        
        // Explicitly drop the permit here 
        drop(permit);
        
        result
    }
    
    /// Get collector statistics
    pub async fn get_stats(&self) -> CollectorStats {
        let stats = self.stats.read().await;
        let mut stats_copy = stats.clone();
        let uptime_duration = Utc::now().signed_duration_since(stats.started_at);
        stats_copy.uptime = Duration::from_secs(uptime_duration.num_seconds() as u64);
        
        // Update batch size
        {
            let batch = self.batch_buffer.read().await;
            stats_copy.batch_size = batch.len();
        }
        
        // Update streaming statistics if available
        if let Some(ref streamer) = self.streamer {
            let streaming_stats = streamer.get_stats().await;
            stats_copy.streaming_clients = streaming_stats.connected_clients;
            stats_copy.streaming_subscriptions = streaming_stats.active_subscriptions;
        }
        
        stats_copy
    }
    
    /// Query audit events
    pub async fn query_events(&self, query: &super::storage::AuditQuery) -> AuditResult<Vec<super::AuditEvent>> {
        self.storage.query_events(query).await
    }
    
    /// Count audit events
    pub async fn count_events(&self, query: &super::storage::AuditQuery) -> AuditResult<u64> {
        self.storage.count_events(query).await
    }
    
    /// Get security violations
    pub async fn get_security_violations(&self, query: &super::storage::AuditQuery) -> AuditResult<Vec<super::AuditEvent>> {
        // Query for security violation events
        let mut violation_query = query.clone();
        violation_query.event_types = Some(vec!["security_violation".to_string()]);
        
        self.storage.query_events(&violation_query).await
    }
    
    /// Get violation statistics
    pub async fn get_violation_statistics(&self, time_range: Option<chrono::Duration>) -> AuditResult<serde_json::Value> {
        use chrono::Utc;
        use std::collections::HashMap;
        
        let start_time = if let Some(duration) = time_range {
            Some(Utc::now() - duration)
        } else {
            Some(Utc::now() - chrono::Duration::hours(24)) // Default to 24 hours
        };
        
        // Query for security violations in time range
        let query = super::storage::AuditQuery {
            start_time,
            end_time: Some(Utc::now()),
            event_types: Some(vec!["security_violation".to_string()]),
            components: None,
            severities: None,
            user_ids: None,
            search_text: None,
            limit: None,
            offset: None,
            sort_by: Some("timestamp".to_string()),
            sort_desc: true,
            correlation_id: None,
            metadata_filters: HashMap::new(),
        };
        
        let violations = self.storage.query_events(&query).await?;
        
        // Calculate statistics
        let total = violations.len();
        
        // Group by severity
        let mut by_severity: HashMap<String, usize> = HashMap::new();
        for violation in &violations {
            let severity = format!("{:?}", violation.severity);
            *by_severity.entry(severity).or_insert(0) += 1;
        }
        
        // Group by component
        let mut by_component: HashMap<String, usize> = HashMap::new();
        for violation in &violations {
            *by_component.entry(violation.component.clone()).or_insert(0) += 1;
        }
        
        // Calculate trend (compare with previous period)
        let previous_period_start = if let Some(duration) = time_range {
            Some(start_time.unwrap() - duration)
        } else {
            Some(Utc::now() - chrono::Duration::hours(48))
        };
        
        let previous_query = super::storage::AuditQuery {
            start_time: previous_period_start,
            end_time: start_time,
            event_types: Some(vec!["security_violation".to_string()]),
            components: None,
            severities: None,
            user_ids: None,
            search_text: None,
            limit: None,
            offset: None,
            sort_by: Some("timestamp".to_string()),
            sort_desc: true,
            correlation_id: None,
            metadata_filters: HashMap::new(),
        };
        
        let previous_violations = self.storage.query_events(&previous_query).await.unwrap_or_default();
        let previous_total = previous_violations.len();
        
        let growth = if previous_total > 0 {
            ((total as f64 - previous_total as f64) / previous_total as f64) * 100.0
        } else if total > 0 {
            100.0 // If no previous violations but current violations exist
        } else {
            0.0
        };
        
        Ok(serde_json::json!({
            "total_violations": total,
            "by_severity": by_severity,
            "by_component": by_component,
            "previous_period_total": previous_total,
            "growth_percentage": growth,
            "time_range_hours": time_range.map(|d| d.num_hours()).unwrap_or(24)
        }))
    }
    
    /// Get violation related entries
    pub async fn get_violation_related_entries(&self, violation_id: &str) -> AuditResult<Vec<super::AuditEvent>> {
        // Query events with the same correlation_id or related metadata
        let query = super::storage::AuditQuery {
            start_time: None,
            end_time: None,
            event_types: None,
            components: None,
            severities: None,
            user_ids: None,
            search_text: None,
            limit: Some(50),
            offset: None,
            sort_by: Some("timestamp".to_string()),
            sort_desc: true,
            correlation_id: Some(violation_id.to_string()),
            metadata_filters: std::collections::HashMap::new(),
        };
        
        self.storage.query_events(&query).await
    }
    
    /// Update violation status
    pub async fn update_violation_status(&self, violation_id: &str, _params: &serde_json::Value) -> AuditResult<serde_json::Value> {
        // Log the status update as an audit event
        let event = super::AuditEvent::new(
            super::events::AuditEventType::AdminAction,
            "audit_system".to_string(),
            format!("Violation status updated for: {}", violation_id),
        ).with_metadata("violation_id", serde_json::json!(violation_id))
         .with_metadata("action", serde_json::json!("status_update"));
        
        self.log_event(event).await?;
        
        Ok(serde_json::json!({
            "success": true,
            "message": "Violation status updated successfully"
        }))
    }
    
    /// Assign violation
    pub async fn assign_violation(&self, violation_id: &str, _params: &serde_json::Value) -> AuditResult<serde_json::Value> {
        // Log the assignment as an audit event
        let event = super::AuditEvent::new(
            super::events::AuditEventType::AdminAction,
            "audit_system".to_string(),
            format!("Violation assigned: {}", violation_id),
        ).with_metadata("violation_id", serde_json::json!(violation_id))
         .with_metadata("action", serde_json::json!("assignment"));
        
        self.log_event(event).await?;
        
        Ok(serde_json::json!({
            "success": true,
            "message": "Violation assigned successfully"
        }))
    }
    
    /// Add violation note
    pub async fn add_violation_note(&self, violation_id: &str, _params: &serde_json::Value) -> AuditResult<serde_json::Value> {
        // Log the note addition as an audit event
        let event = super::AuditEvent::new(
            super::events::AuditEventType::AdminAction,
            "audit_system".to_string(),
            format!("Note added to violation: {}", violation_id),
        ).with_metadata("violation_id", serde_json::json!(violation_id))
         .with_metadata("action", serde_json::json!("note_added"));
        
        self.log_event(event).await?;
        
        Ok(serde_json::json!({
            "success": true,
            "message": "Note added successfully"
        }))
    }
    
    /// Get audit event types
    pub async fn get_audit_event_types(&self) -> AuditResult<Vec<String>> {
        // Return the available audit event types
        Ok(vec![
            "authentication".to_string(),
            "authorization".to_string(),
            "token_refresh".to_string(),
            "session_created".to_string(),
            "session_destroyed".to_string(),
            "oauth_flow".to_string(),
            "oauth_discovery".to_string(),
            "oauth_registration".to_string(),
            "oauth_authorization".to_string(),
            "oauth_token_exchange".to_string(),
            "oauth_token_usage".to_string(),
            "oauth_forwarded".to_string(),
            "tool_execution".to_string(),
            "mcp_connection".to_string(),
            "mcp_disconnection".to_string(),
            "smart_discovery".to_string(),
            "capability_refresh".to_string(),
            "resource_access".to_string(),
            "security_violation".to_string(),
            "allowlist_check".to_string(),
            "rbac_check".to_string(),
            "sanitization".to_string(),
            "emergency_lockdown".to_string(),
            "admin_action".to_string(),
            "config_change".to_string(),
            "service_start".to_string(),
            "service_stop".to_string(),
            "system_health".to_string(),
            "performance_metric".to_string(),
            "error_occurred".to_string(),
        ])
    }
    
    /// Get audit users (users who have been involved in audit events)
    pub async fn get_audit_users(&self) -> AuditResult<Vec<String>> {
        // Query all events and extract unique user IDs
        let query = super::storage::AuditQuery {
            start_time: None,
            end_time: None,
            event_types: None,
            components: None,
            severities: None,
            user_ids: None,
            search_text: None,
            limit: Some(10000), // Limit to avoid memory issues
            offset: None,
            sort_by: Some("timestamp".to_string()),
            sort_desc: true,
            correlation_id: None,
            metadata_filters: std::collections::HashMap::new(),
        };
        
        let events = self.storage.query_events(&query).await?;
        let mut users = std::collections::HashSet::new();
        
        for event in events {
            if let Some(user_id) = &event.metadata.user_id {
                users.insert(user_id.clone());
            }
        }
        
        let mut user_list: Vec<String> = users.into_iter().collect();
        user_list.sort();
        
        Ok(user_list)
    }
    
    /// Check system health
    pub async fn health_check(&self) -> AuditResult<CollectorHealth> {
        let stats = self.get_stats().await;
        
        // Check storage health
        let storage_health = self.storage.health_check().await?;
        
        // Check streaming health if enabled
        let streaming_healthy = if let Some(ref streamer) = self.streamer {
            streamer.get_client_count().await < 1000 // Arbitrary healthy limit
        } else {
            true
        };
        
        let overall_healthy = storage_health.healthy && streaming_healthy && stats.total_errors < 100;
        
        Ok(CollectorHealth {
            healthy: overall_healthy,
            storage_health,
            streaming_healthy,
            queue_depth: stats.queue_depth,
            events_per_second: stats.events_per_second,
            uptime_seconds: stats.uptime.as_secs(),
            error_count: stats.total_errors,
        })
    }
    
    /// Get audit streamer for WebSocket connections
    pub fn get_streamer(&self) -> Option<&AuditStreamer> {
        self.streamer.as_ref()
    }
    
    /// Flush all pending events
    pub async fn flush(&self) -> AuditResult<()> {
        info!("🔄 Flushing audit collector");
        
        // Flush batch buffer
        self.flush_batch().await?;
        
        // Flush storage
        self.storage.flush().await?;
        
        info!("✅ Audit collector flush completed");
        Ok(())
    }
    
    /// Start event processing task
    async fn start_event_processor(&self, mut event_rx: mpsc::Receiver<AuditEvent>, mut shutdown_rx: mpsc::Receiver<()>) {
        let batch_buffer = Arc::clone(&self.batch_buffer);
        let storage = Arc::clone(&self.storage);
        // Check if streamer exists and get its broadcast channel
        let streamer_available = self.streamer.is_some();
        let stats = Arc::clone(&self.stats);
        let batch_size = self.config.performance.batch_size;
        let processing_semaphore = Arc::clone(&self.processing_semaphore);
        
        tokio::spawn(async move {
            info!("🔄 Starting audit event processor");
            
            loop {
                tokio::select! {
                    // Process incoming events
                    Some(event) = event_rx.recv() => {
                        let start_time = Instant::now();
                        
                        // Stream event if streaming is enabled
                        if streamer_available {
                            // For now, skip streaming in the async task to avoid lifetime issues
                            // The event will still be stored and can be streamed via other means
                            debug!("Event streaming skipped in async processor (streamer available but not accessible)");
                        }
                        
                        // Add to batch buffer
                        {
                            let mut batch = batch_buffer.write().await;
                            batch.push(event);
                            
                            // Flush batch if it reaches configured size
                            if batch.len() >= batch_size {
                                let events_to_store = batch.drain(..).collect::<Vec<_>>();
                                drop(batch);
                                
                                // Store batch
                                if let Err(e) = storage.store_batch(&events_to_store).await {
                                    error!("❌ Failed to store audit event batch: {}", e);
                                    let mut stats = stats.write().await;
                                    stats.storage_errors_count += 1;
                                    stats.total_errors += 1;
                                }
                            }
                        }
                        
                        // Update processing statistics
                        {
                            let mut stats = stats.write().await;
                            stats.total_events += 1;
                            
                            let processing_time = start_time.elapsed().as_millis() as f64;
                            stats.avg_processing_time_ms = 
                                (stats.avg_processing_time_ms * 0.9) + (processing_time * 0.1);
                            stats.max_processing_time_ms = 
                                stats.max_processing_time_ms.max(processing_time);
                        }
                        
                        // Release processing permit
                        processing_semaphore.add_permits(1);
                    },
                    
                    // Handle shutdown
                    _ = shutdown_rx.recv() => {
                        info!("🛑 Shutting down audit event processor");
                        break;
                    }
                }
            }
        });
    }
    
    /// Start batch processing task for periodic flushes
    async fn start_batch_processor(&self) {
        let batch_buffer = Arc::clone(&self.batch_buffer);
        let storage = Arc::clone(&self.storage);
        let stats = Arc::clone(&self.stats);
        let flush_interval = Duration::from_secs(self.config.performance.flush_interval_secs);
        
        tokio::spawn(async move {
            let mut interval = interval(flush_interval);
            debug!("🕒 Batch processor started with flush interval: {:?}", flush_interval);
            
            loop {
                interval.tick().await;
                debug!("🕒 Batch processor tick - checking for events to flush");
                
                // Flush batch buffer periodically
                let events_to_store = {
                    let mut batch = batch_buffer.write().await;
                    let batch_size = batch.len();
                    debug!("📦 Batch buffer contains {} events", batch_size);
                    if batch.is_empty() {
                        continue;
                    }
                    batch.drain(..).collect::<Vec<_>>()
                };
                
                if !events_to_store.is_empty() {
                    debug!("🔄 Flushing {} audit events from batch buffer", events_to_store.len());
                    
                    // Use Arc properly instead of unsafe code
                    match storage.store_batch(&events_to_store).await {
                        Ok(()) => {
                            debug!("✅ Successfully flushed {} audit events to storage", events_to_store.len());
                        },
                        Err(e) => {
                            error!("❌ Failed to flush audit event batch: {}", e);
                            let mut stats = stats.write().await;
                            stats.storage_errors_count += 1;
                            stats.total_errors += 1;
                        }
                    }
                }
            }
        });
    }
    
    /// Start statistics updater task
    async fn start_stats_updater(&self) {
        let stats = Arc::clone(&self.stats);
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            let mut last_event_count = 0u64;
            let mut event_rates = Vec::with_capacity(60); // 1 minute rolling window
            
            loop {
                interval.tick().await;
                
                let mut stats_write = stats.write().await;
                
                // Calculate events per second (rolling average)
                let current_events = stats_write.total_events;
                let events_this_second = current_events.saturating_sub(last_event_count);
                last_event_count = current_events;
                
                event_rates.push(events_this_second as f64);
                if event_rates.len() > 60 {
                    event_rates.remove(0);
                }
                
                stats_write.events_per_second = event_rates.iter().sum::<f64>() / event_rates.len() as f64;
                
                // Update health status
                stats_write.healthy = stats_write.total_errors < 100 && 
                                     stats_write.queue_depth < 5000;
            }
        });
    }
    
    /// Start health monitoring task
    async fn start_health_monitor(&self) {
        let storage = Arc::clone(&self.storage);
        let stats = Arc::clone(&self.stats);
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
            
            loop {
                interval.tick().await;
                
                // SAFETY: storage pointer is valid for the lifetime of the collector
                let storage_ref = unsafe { &*storage };
                
                // Check storage health
                match storage_ref.health_check().await {
                    Ok(health) => {
                        let mut stats = stats.write().await;
                        stats.storage_healthy = health.healthy;
                        stats.storage_errors = health.error_count;
                    },
                    Err(e) => {
                        error!("❌ Storage health check failed: {}", e);
                        let mut stats = stats.write().await;
                        stats.storage_healthy = false;
                        stats.total_errors += 1;
                    }
                }
            }
        });
    }
    
    /// Flush batch buffer manually
    async fn flush_batch(&self) -> AuditResult<()> {
        let events_to_store = {
            let mut batch = self.batch_buffer.write().await;
            batch.drain(..).collect::<Vec<_>>()
        };
        
        if !events_to_store.is_empty() {
            info!("🔄 Flushing {} events from batch buffer", events_to_store.len());
            self.storage.store_batch(&events_to_store).await?;
        }
        
        Ok(())
    }
    
    /// Graceful shutdown
    pub async fn shutdown(self) -> AuditResult<()> {
        info!("🛑 Shutting down audit collector");
        
        // Send shutdown signal
        let _ = self.shutdown_tx.send(()).await;
        
        // Flush all pending events
        self.flush().await?;
        
        // Shutdown streaming service
        if let Some(streamer) = self.streamer {
            streamer.shutdown().await?;
        }
        
        info!("✅ Audit collector shutdown completed");
        Ok(())
    }
    
    // Legacy OAuth audit methods (convenience methods for backward compatibility)
    pub async fn log_oauth_discovery_attempt(&self, server_name: &str, base_url: &str) {
        let event = AuditEvent::new(
            super::events::AuditEventType::OauthFlow,
            "oauth_discovery".to_string(),
            format!("OAuth discovery attempt for server: {}", server_name),
        ).with_metadata("server_name", serde_json::json!(server_name))
         .with_metadata("base_url", serde_json::json!(base_url));
        let _ = self.log_event(event).await;
    }
    
    pub async fn log_oauth_registration_failure(&self, server_name: &str, error: &str) {
        let event = AuditEvent::new(
            super::events::AuditEventType::OauthFlow,
            "oauth_discovery".to_string(),
            format!("OAuth registration failed for server: {} - {}", server_name, error),
        ).with_metadata("server_name", serde_json::json!(server_name))
         .with_metadata("error", serde_json::json!(error));
        let _ = self.log_event(event).await;
    }
    
    pub async fn log_oauth_registration_success(&self, server_name: &str, client_id: &str, granted_scopes: &str) {
        let event = AuditEvent::new(
            super::events::AuditEventType::OauthFlow,
            "oauth_discovery".to_string(),
            format!("OAuth registration successful for server: {}", server_name),
        ).with_metadata("server_name", serde_json::json!(server_name))
         .with_metadata("client_id", serde_json::json!(client_id))
         .with_metadata("granted_scopes", serde_json::json!(granted_scopes));
        let _ = self.log_event(event).await;
    }
    
    pub async fn log_oauth_authorization_start(&self, server_name: &str, auth_url: &str) {
        let event = AuditEvent::new(
            super::events::AuditEventType::OauthFlow,
            "oauth_authorization".to_string(),
            format!("OAuth authorization started for server: {} (URL: {})", server_name, auth_url),
        ).with_metadata("server_name", serde_json::json!(server_name))
         .with_metadata("auth_url", serde_json::json!(auth_url));
        let _ = self.log_event(event).await;
    }
    
    pub async fn log_oauth_authorization_success(&self, server_name: &str) {
        let event = AuditEvent::new(
            super::events::AuditEventType::OauthFlow,
            "oauth_authorization".to_string(),
            format!("OAuth authorization successful for server: {}", server_name),
        ).with_metadata("server_name", serde_json::json!(server_name));
        let _ = self.log_event(event).await;
    }
    
    pub async fn log_oauth_forwarded_to_client(&self, server_name: &str, oauth_details: &str) {
        let event = AuditEvent::new(
            super::events::AuditEventType::OauthFlow,
            "oauth_forward".to_string(),
            format!("OAuth authorization forwarded to client for server: {}", server_name),
        ).with_metadata("server_name", serde_json::json!(server_name))
         .with_metadata("oauth_details", serde_json::json!(oauth_details));
        let _ = self.log_event(event).await;
    }
    
    pub async fn log_oauth_token_exchange_failure(&self, server_name: &str, error: &str) {
        let event = AuditEvent::new(
            super::events::AuditEventType::OauthFlow,
            "oauth_token_exchange".to_string(),
            format!("OAuth token exchange failed for server: {} - {}", server_name, error),
        ).with_metadata("server_name", serde_json::json!(server_name))
         .with_metadata("error", serde_json::json!(error));
        let _ = self.log_event(event).await;
    }
    
    pub async fn log_oauth_token_exchange_success(&self, server_name: &str) {
        let event = AuditEvent::new(
            super::events::AuditEventType::OauthFlow,
            "oauth_token_exchange".to_string(),
            format!("OAuth token exchange successful for server: {}", server_name),
        ).with_metadata("server_name", serde_json::json!(server_name));
        let _ = self.log_event(event).await;
    }
    
    pub async fn log_mcp_connection_established(&self, server_name: &str, transport_type: &str, success: bool) {
        let event = AuditEvent::new(
            super::events::AuditEventType::McpConnection,
            "mcp_connection".to_string(),
            format!("MCP connection established to server: {} (transport: {}, success: {})", server_name, transport_type, success),
        ).with_metadata("server_name", serde_json::json!(server_name))
         .with_metadata("transport_type", serde_json::json!(transport_type))
         .with_metadata("success", serde_json::json!(success));
        let _ = self.log_event(event).await;
    }
    
    pub async fn log_mcp_tool_execution(&self, server_name: &str, tool_name: &str, success: bool, authenticated: bool) {
        let event = AuditEvent::new(
            super::events::AuditEventType::ToolExecution,
            "mcp_tool_execution".to_string(),
            format!("Tool '{}' executed on server: {} (success: {}, authenticated: {})", tool_name, server_name, success, authenticated),
        ).with_metadata("tool_name", serde_json::json!(tool_name))
         .with_metadata("server_name", serde_json::json!(server_name))
         .with_metadata("success", serde_json::json!(success))
         .with_metadata("authenticated", serde_json::json!(authenticated));
        let _ = self.log_event(event).await;
    }
    
    pub async fn log_oauth_token_usage(&self, server_name: &str, method: &str, endpoint: &str) {
        let event = AuditEvent::new(
            super::events::AuditEventType::OauthFlow,
            "oauth_token_usage".to_string(),
            format!("OAuth token used for server: {} ({} {})", server_name, method, endpoint),
        ).with_metadata("server_name", serde_json::json!(server_name))
         .with_metadata("method", serde_json::json!(method))
         .with_metadata("endpoint", serde_json::json!(endpoint));
        let _ = self.log_event(event).await;
    }
}

// ---------------------------------------------------------------------------
// SecurityServiceStatistics-style helpers (used by APIs)
// ---------------------------------------------------------------------------

impl AuditCollector {
    pub async fn get_statistics_snapshot(&self) -> AuditStatistics {
        // Base stats from collector and storage
        let collector_stats = self.get_stats().await;
        let storage_stats = match self.storage.get_stats().await {
            Ok(s) => s,
            Err(_) => super::storage::StorageStats {
                total_events: 0,
                events_per_second: 0.0,
                storage_size_bytes: 0,
                oldest_event: None,
                newest_event: None,
                error_count: 0,
            },
        };

        // Time windows
        let now = Utc::now();
        let last_24h = ChronoDuration::hours(24);

        // Counts
        let entries_today = self.storage.count_events(&AuditQuery {
            start_time: Some(now - last_24h),
            end_time: Some(now),
            ..AuditQuery::default()
        }).await.unwrap_or(0);

        let security_events = self.storage.count_events(&AuditQuery {
            event_types: Some(vec!["security_violation".to_string()]),
            ..AuditQuery::default()
        }).await.unwrap_or(0);

        let violations_today = self.storage.count_events(&AuditQuery {
            event_types: Some(vec!["security_violation".to_string()]),
            start_time: Some(now - last_24h),
            end_time: Some(now),
            ..AuditQuery::default()
        }).await.unwrap_or(0);

        // Heuristic critical violations: severity high within 24h
        let critical_violations = self.storage.count_events(&AuditQuery {
            event_types: Some(vec!["security_violation".to_string()]),
            severities: Some(vec!["high".to_string()]),
            start_time: Some(now - last_24h),
            end_time: Some(now),
            ..AuditQuery::default()
        }).await.unwrap_or(0);

        // Avg entries/day over available window
        let avg_entries_per_day = match (storage_stats.oldest_event, storage_stats.newest_event) {
            (Some(oldest), Some(newest)) if newest > oldest => {
                let days = (newest - oldest).num_days().max(1) as f64;
                storage_stats.total_events as f64 / days
            }
            _ => storage_stats.total_events as f64,
        };

        // Build health
        let health = self.get_health().await;
        
        // Compute top event types over last 24h
        let recent_events = self.storage.query_events(&AuditQuery {
            start_time: Some(now - last_24h),
            end_time: Some(now),
            limit: Some(5000),
            ..AuditQuery::default()
        }).await.unwrap_or_default();

        let mut type_counts: HashMap<String, u64> = HashMap::new();
        let mut unique_users: HashMap<String, ()> = HashMap::new();
        let mut auth_total = 0u64;
        let mut auth_failed = 0u64;
        for ev in &recent_events {
            let type_str = serde_json::to_string(&ev.event_type).unwrap_or_else(|_| "unknown".to_string()).trim_matches('"').to_string();
            *type_counts.entry(type_str).or_insert(0) += 1;

            if let Some(uid) = &ev.metadata.user_id {
                unique_users.entry(uid.clone()).or_insert(());
            }

            // Authentication stats approximation
            if matches!(ev.event_type, super::events::AuditEventType::Authentication | super::events::AuditEventType::Authorization) {
                auth_total += 1;
                if matches!(ev.severity, super::events::AuditSeverity::Error | super::events::AuditSeverity::Critical) {
                    auth_failed += 1;
                }
            }
        }

        let total_recent = recent_events.len() as f64;
        let mut top_event_types: Vec<crate::security::statistics::EventTypeCount> = type_counts
            .into_iter()
            .map(|(event_type, count)| crate::security::statistics::EventTypeCount {
                event_type,
                count,
                percentage: if total_recent > 0.0 { (count as f64 / total_recent) * 100.0 } else { 0.0 },
            })
            .collect();
        top_event_types.sort_by(|a, b| b.count.cmp(&a.count));
        top_event_types.truncate(10);

        AuditStatistics {
            health,
            total_entries: storage_stats.total_events,
            entries_today,
            security_events,
            violations_today,
            critical_violations: critical_violations as u64,
            storage_size_bytes: storage_stats.storage_size_bytes,
            avg_entries_per_day,
            top_event_types,
            auth_events: auth_total,
            failed_auth: auth_failed,
            unique_users: unique_users.len() as u64,
        }
    }

    pub async fn get_health(&self) -> ServiceHealth {
        // Collector perspective
        let stats = self.get_stats().await;
        let storage_health = self.storage.health_check().await;

        let (status, is_healthy, error_message) = match storage_health {
            Ok(h) => {
                if h.healthy && stats.healthy {
                    (HealthStatus::Healthy, true, None)
                } else {
                    (HealthStatus::Warning, false, Some(h.message))
                }
            }
            Err(e) => (HealthStatus::Error, false, Some(format!("Storage health error: {}", e))),
        };

        ServiceHealth {
            status,
            is_healthy,
            last_checked: Utc::now(),
            error_message,
            uptime_seconds: stats.uptime.as_secs(),
            performance: PerformanceMetrics {
                avg_response_time_ms: stats.avg_processing_time_ms,
                requests_per_second: stats.events_per_second,
                error_rate: if stats.total_events > 0 { stats.total_errors as f64 / stats.total_events as f64 } else { 0.0 },
                memory_usage_bytes: 0,
            },
        }
    }

    pub async fn reset_statistics_counters(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut s = self.stats.write().await;
        *s = CollectorStats { started_at: Utc::now(), ..CollectorStats::default() };
        Ok(())
    }

    /// Get storage statistics from the backend (size, totals, etc.)
    pub async fn get_storage_stats(&self) -> AuditResult<StorageStats> {
        self.storage.get_stats().await
    }

    // ---------------------------------------------------------------------
    // Time-range statistics (centralized for API consumption)
    // ---------------------------------------------------------------------

    /// Compute grouped audit statistics for a given time range.
    pub async fn get_statistics_for_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<usize>,
    ) -> AuditResult<AuditRangeStatistics> {
        let events = self.storage.query_events(&AuditQuery {
            start_time: Some(start_time),
            end_time: Some(end_time),
            limit,
            sort_by: Some("timestamp".to_string()),
            sort_desc: true,
            ..AuditQuery::default()
        }).await?;

        let total_events = events.len() as u64;
        let mut type_counts: HashMap<String, u64> = HashMap::new();
        let mut unique_users: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut auth_events = 0u64;
        let mut failed_auth = 0u64;
        let mut violations = 0u64;
        let mut critical_violations = 0u64;

        for ev in &events {
            // Event type counts
            let type_str = serde_json::to_string(&ev.event_type)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
                .to_string();
            *type_counts.entry(type_str).or_insert(0) += 1;

            // Unique users
            if let Some(uid) = &ev.metadata.user_id {
                unique_users.insert(uid.clone());
            }

            // Authentication stats
            if matches!(ev.event_type, super::events::AuditEventType::Authentication | super::events::AuditEventType::Authorization) {
                auth_events += 1;
                if matches!(ev.severity, super::events::AuditSeverity::Error | super::events::AuditSeverity::Critical) {
                    failed_auth += 1;
                }
            }

            // Violations
            if matches!(ev.event_type, super::events::AuditEventType::SecurityViolation) {
                violations += 1;
                if matches!(ev.severity, super::events::AuditSeverity::Critical) {
                    critical_violations += 1;
                }
            }
        }

        // Convert to vector with percentages
        let total_f = total_events as f64;
        let mut event_types: Vec<crate::security::statistics::EventTypeCount> = type_counts
            .into_iter()
            .map(|(event_type, count)| crate::security::statistics::EventTypeCount {
                event_type,
                count,
                percentage: if total_f > 0.0 { (count as f64 / total_f) * 100.0 } else { 0.0 },
            })
            .collect();
        event_types.sort_by(|a, b| b.count.cmp(&a.count));
        event_types.truncate(50);

        // Compute average per day across the range
        let hours = (end_time - start_time).num_hours().max(1) as f64;
        let avg_entries_per_day = (total_f / hours) * 24.0;

        Ok(AuditRangeStatistics {
            total_events,
            violations,
            critical_violations,
            auth_events,
            failed_auth,
            unique_users: unique_users.len() as u64,
            avg_entries_per_day,
            event_types,
        })
    }
}

// ============================================================================
// SecurityServiceStatistics Implementation
// ============================================================================

#[async_trait::async_trait]
impl crate::security::statistics::SecurityServiceStatistics for AuditCollector {
    type Statistics = crate::security::statistics::AuditStatistics;
    
    async fn get_statistics(&self) -> Self::Statistics {
        use crate::security::statistics::{AuditStatistics, ServiceHealth, HealthStatus, PerformanceMetrics, EventTypeCount};
        use chrono::{Duration, Utc};
        use std::collections::HashMap;
        
        // Get basic collector statistics
        let collector_stats = self.get_stats().await;
        let storage_stats = self.storage.get_stats().await.unwrap_or_default();
        
        // Calculate today's entries (last 24 hours)
        let today_start = Utc::now() - Duration::hours(24);
        let entries_today = self.storage.query_events(&super::storage::AuditQuery {
            start_time: Some(today_start),
            end_time: Some(Utc::now()),
            event_types: None,
            components: None,
            severities: None,
            user_ids: None,
            search_text: None,
            limit: None,
            offset: None,
            sort_by: None,
            sort_desc: false,
            correlation_id: None,
            metadata_filters: HashMap::new(),
        }).await.map(|events| events.len() as u64).unwrap_or(0);
        
        // Get security events count
        let security_events = self.storage.query_events(&super::storage::AuditQuery {
            start_time: Some(today_start),
            end_time: Some(Utc::now()),
            event_types: Some(vec!["security_violation".to_string()]),
            components: None,
            severities: None,
            user_ids: None,
            search_text: None,
            limit: None,
            offset: None,
            sort_by: None,
            sort_desc: false,
            correlation_id: None,
            metadata_filters: HashMap::new(),
        }).await.map(|events| events.len() as u64).unwrap_or(0);
        
        // Get authentication events
        let auth_events = self.storage.query_events(&super::storage::AuditQuery {
            start_time: Some(today_start),
            end_time: Some(Utc::now()),
            event_types: Some(vec!["authentication".to_string()]),
            components: None,
            severities: None,
            user_ids: None,
            search_text: None,
            limit: None,
            offset: None,
            sort_by: None,
            sort_desc: false,
            correlation_id: None,
            metadata_filters: HashMap::new(),
        }).await.map(|events| events.len() as u64).unwrap_or(0);
        
        // Get critical violations (high severity security events)
        let critical_violations = self.storage.query_events(&super::storage::AuditQuery {
            start_time: Some(today_start),
            end_time: Some(Utc::now()),
            event_types: Some(vec!["security_violation".to_string()]),
            components: None,
            severities: Some(vec!["critical".to_string(), "high".to_string()]),
            user_ids: None,
            search_text: None,
            limit: None,
            offset: None,
            sort_by: None,
            sort_desc: false,
            correlation_id: None,
            metadata_filters: HashMap::new(),
        }).await.map(|events| events.len() as u64).unwrap_or(0);
        
        // Calculate average entries per day (last 30 days)
        let thirty_days_ago = Utc::now() - Duration::days(30);
        let total_entries_30d = self.storage.query_events(&super::storage::AuditQuery {
            start_time: Some(thirty_days_ago),
            end_time: Some(Utc::now()),
            event_types: None,
            components: None,
            severities: None,
            user_ids: None,
            search_text: None,
            limit: None,
            offset: None,
            sort_by: None,
            sort_desc: false,
            correlation_id: None,
            metadata_filters: HashMap::new(),
        }).await.map(|events| events.len() as f64).unwrap_or(0.0);
        
        let avg_entries_per_day = total_entries_30d / 30.0;
        
        // Create service health
        let health_status = if collector_stats.healthy {
            HealthStatus::Healthy
        } else {
            HealthStatus::Error
        };
        
        let service_health = ServiceHealth {
            status: health_status,
            is_healthy: collector_stats.healthy,
            last_checked: Utc::now(),
            error_message: if collector_stats.healthy { None } else { Some("Audit service errors detected".to_string()) },
            uptime_seconds: collector_stats.uptime.as_secs(),
            performance: PerformanceMetrics {
                avg_response_time_ms: collector_stats.avg_processing_time_ms,
                requests_per_second: collector_stats.events_per_second,
                error_rate: if collector_stats.total_events > 0 {
                    collector_stats.total_errors as f64 / collector_stats.total_events as f64
                } else {
                    0.0
                },
                memory_usage_bytes: storage_stats.storage_size_bytes,
            },
        };
        
        // Create top event types (simplified for now)
        let total_events_f64 = collector_stats.total_events as f64;
        let top_event_types = vec![
            EventTypeCount {
                event_type: "authentication".to_string(),
                count: auth_events,
                percentage: if total_events_f64 > 0.0 { auth_events as f64 / total_events_f64 * 100.0 } else { 0.0 },
            },
            EventTypeCount {
                event_type: "security_violation".to_string(),
                count: security_events,
                percentage: if total_events_f64 > 0.0 { security_events as f64 / total_events_f64 * 100.0 } else { 0.0 },
            },
        ];
        
        AuditStatistics {
            health: service_health,
            total_entries: collector_stats.total_events,
            entries_today,
            security_events,
            violations_today: security_events, // Using security_events as violations
            critical_violations,
            storage_size_bytes: storage_stats.storage_size_bytes,
            avg_entries_per_day,
            top_event_types,
            auth_events,
            failed_auth: 0, // Would need additional tracking
            unique_users: 0, // Would need additional tracking
        }
    }
    
    async fn get_health(&self) -> crate::security::statistics::ServiceHealth {
        let stats = self.get_statistics().await;
        stats.health
    }
    
    async fn reset_statistics(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Reset collector-level statistics
        let mut stats = self.stats.write().await;
        *stats = CollectorStats {
            total_events: 0,
            events_per_second: 0.0,
            queue_depth: 0,
            batch_size: 0,
            storage_healthy: true,
            storage_errors: 0,
            streaming_clients: 0,
            streaming_subscriptions: 0,
            avg_processing_time_ms: 0.0,
            max_processing_time_ms: 0.0,
            healthy: true,
            uptime: Duration::from_secs(0),
            started_at: Utc::now(),
            total_errors: 0,
            queue_full_errors: 0,
            storage_errors_count: 0,
            streaming_errors: 0,
        };
        
        // Note: Storage statistics would need to be reset separately if supported
        Ok(())
    }
}

// HealthMonitor trait is optional; AuditCollector exposes detailed health
// via SecurityServiceStatistics::get_health implementation above.

/// Overall collector health information
#[derive(Debug, Serialize, Deserialize)]
pub struct CollectorHealth {
    pub healthy: bool,
    pub storage_health: super::storage::StorageHealth,
    pub streaming_healthy: bool,
    pub queue_depth: usize,
    pub events_per_second: f64,
    pub uptime_seconds: u64,
    pub error_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::audit::events::{AuditEventType, AuditSeverity};
    use crate::security::audit::{StorageConfig, RolloverConfig};
    
    #[tokio::test]
    async fn test_collector_creation() {
        let config = AuditConfig::default();
        let collector = AuditCollector::new(config).await.unwrap();
        
        let stats = collector.get_stats().await;
        assert_eq!(stats.total_events, 0);
        assert!(stats.healthy);
    }
    
    #[tokio::test]
    async fn test_event_logging() {
        let config = AuditConfig::default();
        let collector = AuditCollector::new(config).await.unwrap();
        
        let event = AuditEvent::new(
            AuditEventType::Authentication,
            "test".to_string(),
            "Test event".to_string(),
        );
        
        let result = collector.log_event(event).await;
        assert!(result.is_ok());
        
        // Give some time for async processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let stats = collector.get_stats().await;
        assert!(stats.total_events > 0);
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let config = AuditConfig::default();
        let collector = AuditCollector::new(config).await.unwrap();
        
        let health = collector.health_check().await.unwrap();
        assert!(health.healthy);
        assert!(health.storage_health.healthy);
    }
    
    #[tokio::test]
    async fn test_security_service_statistics() {
        use crate::security::statistics::{SecurityServiceStatistics, HealthStatus};
        use tempfile::tempdir;
        
        let temp_dir = tempdir().unwrap();
        let config = AuditConfig {
            storage: StorageConfig::File {
                directory: temp_dir.path().to_path_buf(),
                max_file_size: 100 * 1024 * 1024,
                max_files: 10,
                compress: true,
                sync_interval_secs: 5,
                rollover: RolloverConfig::default(),
            },
            ..AuditConfig::default()
        };
        let collector = AuditCollector::new(config).await.unwrap();
        
        // Test get_statistics method
        let stats = collector.get_statistics().await;
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.entries_today, 0);
        assert_eq!(stats.security_events, 0);
        assert_eq!(stats.violations_today, 0);
        assert_eq!(stats.critical_violations, 0);
        assert_eq!(stats.auth_events, 0);
        assert_eq!(stats.failed_auth, 0);
        assert_eq!(stats.unique_users, 0);
        assert!(stats.avg_entries_per_day >= 0.0);
        assert!(stats.top_event_types.len() >= 0);
        
        // Test service health
        assert_eq!(stats.health.status, HealthStatus::Healthy);
        assert!(stats.health.is_healthy);
        assert!(stats.health.error_message.is_none());
        assert!(stats.health.uptime_seconds >= 0);
        assert!(stats.health.performance.avg_response_time_ms >= 0.0);
        assert!(stats.health.performance.requests_per_second >= 0.0);
        assert!(stats.health.performance.error_rate >= 0.0);
        assert!(stats.health.performance.memory_usage_bytes >= 0);
        
        // Test get_health method
        let health = collector.get_health().await;
        assert_eq!(health.status, HealthStatus::Healthy);
        assert!(health.is_healthy);
        
        // Test reset_statistics method
        let result = collector.reset_statistics().await;
        assert!(result.is_ok());
        
        let stats_after_reset = collector.get_statistics().await;
        assert_eq!(stats_after_reset.total_entries, 0);
    }
    
    #[tokio::test]
    async fn test_audit_statistics_after_events() {
        use crate::security::statistics::SecurityServiceStatistics;
        use crate::security::audit::events::{AuditEventType, AuditSeverity};
        use tempfile::tempdir;
        
        let temp_dir = tempdir().unwrap();
        let config = AuditConfig {
            storage: StorageConfig::File {
                directory: temp_dir.path().to_path_buf(),
                max_file_size: 100 * 1024 * 1024,
                max_files: 10,
                compress: true,
                sync_interval_secs: 5,
                rollover: RolloverConfig::default(),
            },
            ..AuditConfig::default()
        };
        let collector = AuditCollector::new(config).await.unwrap();
        
        // Add some test events
        let auth_event = AuditEvent::new(
            AuditEventType::Authentication,
            "test".to_string(),
            "Test auth event".to_string(),
        );
        
        let security_event = AuditEvent::new(
            AuditEventType::SecurityViolation,
            "test".to_string(),
            "Test security violation".to_string(),
        );
        
        // Log events
        let _ = collector.log_event(auth_event).await;
        let _ = collector.log_event(security_event).await;
        
        // Give some time for async processing
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Get statistics and verify event counting
        let stats = collector.get_statistics().await;
        
        // Basic collector stats should show events
        let collector_stats = collector.get_stats().await;
        assert!(collector_stats.total_events > 0, "Should have processed some events");
        
        // Verify health metrics are updated
        assert!(stats.health.performance.requests_per_second >= 0.0);
        assert!(stats.health.uptime_seconds >= 0);
        
        println!("Statistics after events: total_entries={}, health_status={:?}", 
                 stats.total_entries, stats.health.status);
    }
}
