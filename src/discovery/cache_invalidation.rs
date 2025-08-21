//! Cache Invalidation Strategy for Permission-Based Discovery
//!
//! This module implements intelligent cache invalidation strategies including
//! TTL-based expiration, event-driven updates, and background cleanup tasks.

use crate::discovery::permission_cache::{PermissionCacheManager, PermissionIndex, UserId, ToolId};
use ahash::{AHashMap, AHashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, broadcast, mpsc};
use tracing::{debug, info, warn, error};
use serde::{Deserialize, Serialize};

/// Events that trigger cache invalidation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheInvalidationEvent {
    /// User's permissions changed
    UserPermissionsChanged {
        user_id: UserId,
        old_roles: Vec<String>,
        new_roles: Vec<String>,
    },
    
    /// Tool's permission requirements changed
    ToolPermissionsChanged {
        tool_id: ToolId,
        old_requirements: Vec<String>,
        new_requirements: Vec<String>,
    },
    
    /// Role definition changed (affects all users with that role)
    RoleDefinitionChanged {
        role_id: String,
        old_permissions: Vec<String>,
        new_permissions: Vec<String>,
    },
    
    /// Global allowlist rule changed
    AllowlistRuleChanged {
        rule_id: String,
        affected_tools: Vec<ToolId>,
        action: String, // "added", "removed", "modified"
    },
    
    /// Emergency cache clear (security incident)
    EmergencyCacheClear {
        reason: String,
        affected_users: Option<Vec<UserId>>,
    },
    
    /// Periodic TTL cleanup
    TtlCleanup {
        expired_users: Vec<UserId>,
        cleanup_time_ms: u64,
    },
}

/// Configuration for cache invalidation strategy
#[derive(Debug, Clone)]
pub struct CacheInvalidationConfig {
    /// Default TTL for user caches
    pub default_user_cache_ttl: Duration,
    
    /// TTL for high-privilege users (shorter for security)
    pub admin_cache_ttl: Duration,
    
    /// How often to run background cleanup
    pub cleanup_interval: Duration,
    
    /// How long to keep invalidation events for audit
    pub event_history_retention: Duration,
    
    /// Maximum number of concurrent invalidation operations
    pub max_concurrent_invalidations: usize,
    
    /// Whether to use intelligent cache warming
    pub enable_cache_warming: bool,
    
    /// Whether to use predictive invalidation
    pub enable_predictive_invalidation: bool,
}

impl Default for CacheInvalidationConfig {
    fn default() -> Self {
        Self {
            default_user_cache_ttl: Duration::from_secs(300), // 5 minutes
            admin_cache_ttl: Duration::from_secs(60),         // 1 minute for admins
            cleanup_interval: Duration::from_secs(30),        // Every 30 seconds
            event_history_retention: Duration::from_secs(3600), // 1 hour
            max_concurrent_invalidations: 10,
            enable_cache_warming: true,
            enable_predictive_invalidation: false, // Experimental
        }
    }
}

/// Statistics for cache invalidation operations
#[derive(Debug, Clone, Default)]
pub struct InvalidationStats {
    /// Total invalidations triggered
    pub total_invalidations: u64,
    
    /// Invalidations by event type
    pub invalidations_by_type: AHashMap<String, u64>,
    
    /// TTL-based expirations
    pub ttl_expirations: u64,
    
    /// Event-driven invalidations
    pub event_driven_invalidations: u64,
    
    /// Emergency cache clears
    pub emergency_clears: u64,
    
    /// Average invalidation time (ms)
    pub avg_invalidation_time_ms: f64,
    
    /// Cache warming operations
    pub cache_warming_operations: u64,
    
    /// Predictive invalidations (if enabled)
    pub predictive_invalidations: u64,
}

/// Intelligent cache invalidation manager
pub struct CacheInvalidationManager {
    /// Reference to permission cache manager
    cache_manager: Arc<PermissionCacheManager>,
    
    /// Configuration for invalidation strategy
    config: CacheInvalidationConfig,
    
    /// Event broadcaster for real-time invalidation
    event_sender: broadcast::Sender<CacheInvalidationEvent>,
    
    /// Command channel for background tasks
    command_sender: mpsc::UnboundedSender<InvalidationCommand>,
    
    /// Statistics tracking
    stats: Arc<RwLock<InvalidationStats>>,
    
    /// Event history for audit and analysis
    event_history: Arc<RwLock<Vec<(Instant, CacheInvalidationEvent)>>>,
    
    /// Active background tasks
    background_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
}

/// Commands for background invalidation tasks
#[derive(Debug)]
enum InvalidationCommand {
    /// Force immediate TTL cleanup
    ForceCleanup,
    
    /// Warm cache for specific user
    WarmUserCache { user_id: UserId },
    
    /// Warm cache for specific tools
    WarmToolCache { tool_ids: Vec<ToolId> },
    
    /// Update invalidation statistics
    UpdateStats { event_type: String, duration: Duration },
    
    /// Shutdown background tasks
    Shutdown,
}

impl CacheInvalidationManager {
    /// Create a new cache invalidation manager
    pub fn new(
        cache_manager: Arc<PermissionCacheManager>,
        config: CacheInvalidationConfig,
    ) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        let (command_sender, command_receiver) = mpsc::unbounded_channel();
        
        let manager = Self {
            cache_manager: Arc::clone(&cache_manager),
            config,
            event_sender,
            command_sender,
            stats: Arc::new(RwLock::new(InvalidationStats::default())),
            event_history: Arc::new(RwLock::new(Vec::new())),
            background_tasks: Arc::new(RwLock::new(Vec::new())),
        };
        
        // Start background tasks
        manager.start_background_tasks(command_receiver);
        
        manager
    }
    
    /// Start background invalidation tasks
    fn start_background_tasks(&self, mut command_receiver: mpsc::UnboundedReceiver<InvalidationCommand>) {
        // Task 1: Periodic TTL cleanup
        let cleanup_task = {
            let cache_manager = Arc::clone(&self.cache_manager);
            let command_sender = self.command_sender.clone();
            let config = self.config.clone();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(config.cleanup_interval);
                
                loop {
                    interval.tick().await;
                    
                    debug!("Starting periodic TTL cleanup");
                    let start_time = Instant::now();
                    
                    // Perform TTL cleanup
                    let expired_users = cache_manager.cleanup_expired_caches_async().await;
                    
                    if !expired_users.is_empty() {
                        info!("TTL cleanup removed {} expired user caches", expired_users.len());
                        
                        // Update statistics
                        let duration = start_time.elapsed();
                        let _ = command_sender.send(InvalidationCommand::UpdateStats {
                            event_type: "ttl_cleanup".to_string(),
                            duration,
                        });
                    }
                }
            })
        };
        
        // Task 2: Command processor
        let command_task = {
            let cache_manager = Arc::clone(&self.cache_manager);
            let stats = Arc::clone(&self.stats);
            
            tokio::spawn(async move {
                while let Some(command) = command_receiver.recv().await {
                    match command {
                        InvalidationCommand::ForceCleanup => {
                            debug!("Force cleanup requested");
                            let _expired = cache_manager.cleanup_expired_caches_async().await;
                        }
                        
                        InvalidationCommand::WarmUserCache { user_id } => {
                            debug!("Warming cache for user: {}", user_id);
                            // In a real implementation, you'd pre-populate the cache
                            // For now, this is a placeholder
                        }
                        
                        InvalidationCommand::WarmToolCache { tool_ids: _ } => {
                            debug!("Warming tool caches");
                            // Placeholder for tool cache warming
                        }
                        
                        InvalidationCommand::UpdateStats { event_type, duration } => {
                            let mut stats_guard = stats.write().await;
                            stats_guard.total_invalidations += 1;
                            *stats_guard.invalidations_by_type.entry(event_type).or_insert(0) += 1;
                            
                            // Update rolling average
                            let duration_ms = duration.as_millis() as f64;
                            if stats_guard.total_invalidations == 1 {
                                stats_guard.avg_invalidation_time_ms = duration_ms;
                            } else {
                                stats_guard.avg_invalidation_time_ms = 
                                    (stats_guard.avg_invalidation_time_ms * (stats_guard.total_invalidations - 1) as f64 + duration_ms)
                                    / stats_guard.total_invalidations as f64;
                            }
                        }
                        
                        InvalidationCommand::Shutdown => {
                            info!("Shutting down cache invalidation background tasks");
                            break;
                        }
                    }
                }
            })
        };
        
        // Store task handles
        tokio::spawn(async move {
            let tasks = vec![cleanup_task, command_task];
            // In a real implementation, you'd store these for graceful shutdown
            futures_util::future::join_all(tasks).await;
        });
    }
    
    /// Subscribe to invalidation events
    pub fn subscribe(&self) -> broadcast::Receiver<CacheInvalidationEvent> {
        self.event_sender.subscribe()
    }
    
    /// Trigger a cache invalidation event
    pub async fn invalidate(&self, event: CacheInvalidationEvent) -> Result<(), String> {
        let start_time = Instant::now();
        
        // Add to event history
        {
            let mut history = self.event_history.write().await;
            history.push((start_time, event.clone()));
            
            // Cleanup old events
            let cutoff = start_time - self.config.event_history_retention;
            history.retain(|(timestamp, _)| *timestamp > cutoff);
        }
        
        // Process the invalidation event
        match &event {
            CacheInvalidationEvent::UserPermissionsChanged { user_id, .. } => {
                info!("Invalidating cache for user: {}", user_id);
                self.cache_manager.invalidate_user_cache(user_id).await;
                
                // Optionally warm the cache immediately
                if self.config.enable_cache_warming {
                    let _ = self.command_sender.send(InvalidationCommand::WarmUserCache {
                        user_id: user_id.clone(),
                    });
                }
            }
            
            CacheInvalidationEvent::ToolPermissionsChanged { tool_id, .. } => {
                info!("Tool permissions changed for: {}", tool_id);
                // Invalidate all user caches since tool requirements changed
                self.cache_manager.invalidate_all_user_caches().await;
            }
            
            CacheInvalidationEvent::RoleDefinitionChanged { role_id, .. } => {
                info!("Role definition changed: {}", role_id);
                // Find all users with this role and invalidate their caches
                self.cache_manager.invalidate_caches_by_role(role_id).await;
            }
            
            CacheInvalidationEvent::AllowlistRuleChanged { affected_tools, .. } => {
                info!("Allowlist rule changed affecting {} tools", affected_tools.len());
                // Invalidate all user caches since allowlist affects everyone
                self.cache_manager.invalidate_all_user_caches().await;
            }
            
            CacheInvalidationEvent::EmergencyCacheClear { reason, affected_users } => {
                warn!("Emergency cache clear triggered: {}", reason);
                
                if let Some(users) = affected_users {
                    for user_id in users {
                        self.cache_manager.invalidate_user_cache(user_id).await;
                    }
                } else {
                    self.cache_manager.emergency_clear_all_caches().await;
                }
                
                let mut stats = self.stats.write().await;
                stats.emergency_clears += 1;
            }
            
            CacheInvalidationEvent::TtlCleanup { expired_users, .. } => {
                debug!("TTL cleanup expired {} user caches", expired_users.len());
                let mut stats = self.stats.write().await;
                stats.ttl_expirations += expired_users.len() as u64;
            }
        }
        
        // Broadcast the event to subscribers
        if let Err(e) = self.event_sender.send(event.clone()) {
            warn!("Failed to broadcast invalidation event: {}", e);
        }
        
        // Update statistics
        let duration = start_time.elapsed();
        let event_type = match event {
            CacheInvalidationEvent::UserPermissionsChanged { .. } => "user_permissions",
            CacheInvalidationEvent::ToolPermissionsChanged { .. } => "tool_permissions",
            CacheInvalidationEvent::RoleDefinitionChanged { .. } => "role_definition",
            CacheInvalidationEvent::AllowlistRuleChanged { .. } => "allowlist_rule",
            CacheInvalidationEvent::EmergencyCacheClear { .. } => "emergency_clear",
            CacheInvalidationEvent::TtlCleanup { .. } => "ttl_cleanup",
        }.to_string();
        
        let _ = self.command_sender.send(InvalidationCommand::UpdateStats {
            event_type,
            duration,
        });
        
        Ok(())
    }
    
    /// Force immediate cache cleanup
    pub async fn force_cleanup(&self) {
        let _ = self.command_sender.send(InvalidationCommand::ForceCleanup);
    }
    
    /// Get invalidation statistics
    pub async fn get_stats(&self) -> InvalidationStats {
        self.stats.read().await.clone()
    }
    
    /// Get recent invalidation events for audit
    pub async fn get_recent_events(&self, limit: usize) -> Vec<(Instant, CacheInvalidationEvent)> {
        let history = self.event_history.read().await;
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Predict upcoming invalidations (experimental)
    pub async fn predict_invalidations(&self) -> Vec<CacheInvalidationEvent> {
        if !self.config.enable_predictive_invalidation {
            return Vec::new();
        }
        
        // This is a placeholder for ML-based prediction
        // In a real implementation, you'd analyze patterns in event history
        // to predict when invalidations might be needed
        
        let history = self.event_history.read().await;
        let recent_events: Vec<_> = history.iter()
            .rev()
            .take(10)
            .collect();
        
        // Simple pattern detection (placeholder)
        let mut predictions = Vec::new();
        
        // If we see frequent user permission changes, predict more
        let user_changes = recent_events.iter()
            .filter(|(_, event)| matches!(event, CacheInvalidationEvent::UserPermissionsChanged { .. }))
            .count();
        
        if user_changes > 3 {
            // Predict that we might need to warm some caches
            debug!("Predicting increased cache warming needs due to frequent user changes");
        }
        
        predictions
    }
    
    /// Graceful shutdown of background tasks
    pub async fn shutdown(&self) {
        info!("Shutting down cache invalidation manager");
        let _ = self.command_sender.send(InvalidationCommand::Shutdown);
        
        // Wait for background tasks to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Helper trait for cache managers to implement invalidation methods
#[async_trait::async_trait]
pub trait CacheInvalidation {
    /// Invalidate cache for a specific user
    async fn invalidate_user_cache(&self, user_id: &UserId);
    
    /// Invalidate all user caches
    async fn invalidate_all_user_caches(&self);
    
    /// Invalidate caches for users with a specific role
    async fn invalidate_caches_by_role(&self, role_id: &str);
    
    /// Emergency clear all caches
    async fn emergency_clear_all_caches(&self);
    
    /// Cleanup expired caches and return list of expired user IDs
    async fn cleanup_expired_caches(&self) -> Vec<UserId>;
}

#[async_trait::async_trait]
impl CacheInvalidation for PermissionCacheManager {
    async fn invalidate_user_cache(&self, user_id: &UserId) {
        self.invalidate_user_cache(user_id).await;
    }
    
    async fn invalidate_all_user_caches(&self) {
        self.invalidate_all_user_caches().await;
    }
    
    async fn invalidate_caches_by_role(&self, role_id: &str) {
        self.invalidate_caches_by_role(role_id).await;
    }
    
    async fn emergency_clear_all_caches(&self) {
        self.emergency_clear_all_caches().await;
    }
    
    async fn cleanup_expired_caches(&self) -> Vec<UserId> {
        self.cleanup_expired_caches_async().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::permission_cache::PermissionCacheConfig;
    
    #[tokio::test]
    async fn test_cache_invalidation_manager_creation() {
        let cache_config = PermissionCacheConfig::default();
        let cache_manager = Arc::new(PermissionCacheManager::new(cache_config));
        
        let invalidation_config = CacheInvalidationConfig::default();
        let manager = CacheInvalidationManager::new(cache_manager, invalidation_config);
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_invalidations, 0);
    }
    
    #[tokio::test]
    async fn test_user_permission_invalidation() {
        let cache_config = PermissionCacheConfig::default();
        let cache_manager = Arc::new(PermissionCacheManager::new(cache_config));
        
        let invalidation_config = CacheInvalidationConfig::default();
        let manager = CacheInvalidationManager::new(cache_manager, invalidation_config);
        
        let event = CacheInvalidationEvent::UserPermissionsChanged {
            user_id: "test_user".to_string(),
            old_roles: vec!["user".to_string()],
            new_roles: vec!["admin".to_string()],
        };
        
        let result = manager.invalidate(event).await;
        assert!(result.is_ok());
        
        // Allow some time for background processing
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_invalidations, 1);
    }
    
    #[tokio::test]
    async fn test_emergency_cache_clear() {
        let cache_config = PermissionCacheConfig::default();
        let cache_manager = Arc::new(PermissionCacheManager::new(cache_config));
        
        let invalidation_config = CacheInvalidationConfig::default();
        let manager = CacheInvalidationManager::new(cache_manager, invalidation_config);
        
        let event = CacheInvalidationEvent::EmergencyCacheClear {
            reason: "Security breach detected".to_string(),
            affected_users: None,
        };
        
        let result = manager.invalidate(event).await;
        assert!(result.is_ok());
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.emergency_clears, 1);
    }
}