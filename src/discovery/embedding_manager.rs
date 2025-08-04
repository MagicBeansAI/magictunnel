//! Embedding Manager Module
//!
//! This module handles dynamic embedding lifecycle management including:
//! - Adding/removing embeddings when tools are enabled/disabled
//! - Detecting capability changes and updating embeddings
//! - Merging new dynamic embeddings into persistent storage
//! - Preventing overwrites of user-configured settings

use crate::discovery::semantic::{SemanticSearchService, ToolMetadata};
use crate::discovery::enhancement::ToolEnhancementPipeline;
use crate::error::{ProxyError, Result};
use crate::registry::service::RegistryService;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, mpsc};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::path::PathBuf;

/// Status of embedding operations
#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddingStatus {
    /// Embedding is up to date
    UpToDate,
    /// Embedding needs to be created
    NeedsCreation,
    /// Embedding needs to be updated
    NeedsUpdate,
    /// Embedding should be removed
    ShouldRemove,
}

/// Embedding operation result
#[derive(Debug, Clone)]
pub struct EmbeddingOperation {
    /// Tool name
    pub tool_name: String,
    /// Operation status
    pub status: EmbeddingStatus,
    /// Reason for the operation
    pub reason: String,
    /// Success flag
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Embedding change summary
#[derive(Debug, Clone)]
pub struct EmbeddingChangeSummary {
    /// Total tools processed
    pub total_processed: usize,
    /// Number of embeddings created
    pub created: usize,
    /// Number of embeddings updated
    pub updated: usize,
    /// Number of embeddings removed
    pub removed: usize,
    /// Number of operations that failed
    pub failed: usize,
    /// List of all operations performed
    pub operations: Vec<EmbeddingOperation>,
    /// Processing duration in milliseconds
    pub duration_ms: u64,
}

/// Configuration for the embedding manager
#[derive(Debug, Clone)]
pub struct EmbeddingManagerConfig {
    /// How often to check for changes (in seconds)
    pub check_interval_seconds: u64,
    /// Whether to automatically save after changes
    pub auto_save: bool,
    /// Maximum number of operations to batch together
    pub batch_size: usize,
    /// Whether to run change detection in background
    pub background_monitoring: bool,
    /// Whether to preserve user disabled settings during external MCP updates
    pub preserve_user_settings: bool,
    /// Whether to enable file watching for hot-reload
    pub enable_hot_reload: bool,
}

impl Default for EmbeddingManagerConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 300, // 5 minutes
            auto_save: true,
            batch_size: 10,
            background_monitoring: true,
            preserve_user_settings: true,
            enable_hot_reload: true, // Enable by default
        }
    }
}

/// Embedding lifecycle manager
pub struct EmbeddingManager {
    /// Registry service for accessing tools
    registry: Arc<RegistryService>,
    
    /// Semantic search service for managing embeddings
    semantic_search: Arc<SemanticSearchService>,
    
    /// Configuration
    config: EmbeddingManagerConfig,
    
    /// Last known state of tools for change detection
    last_known_state: Arc<RwLock<HashMap<String, (String, bool, bool)>>>, // name -> (content_hash, enabled, hidden)
    
    /// User-configured disabled tools (to preserve during external MCP updates)
    user_disabled_tools: Arc<RwLock<HashSet<String>>>,
    
    /// Background task handle
    background_task_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    
    /// File watcher handle for hot-reload
    _file_watcher: Arc<RwLock<Option<Box<dyn Watcher + Send + Sync>>>>,
    
    /// Tool enhancement service for sampling/elicitation pipeline
    enhancement_service: Option<Arc<ToolEnhancementPipeline>>,
}

impl EmbeddingManager {
    /// Create a new embedding manager
    pub fn new(
        registry: Arc<RegistryService>,
        semantic_search: Arc<SemanticSearchService>,
        config: EmbeddingManagerConfig,
    ) -> Self {
        Self {
            registry,
            semantic_search,
            config,
            last_known_state: Arc::new(RwLock::new(HashMap::new())),
            user_disabled_tools: Arc::new(RwLock::new(HashSet::new())),
            background_task_handle: Arc::new(RwLock::new(None)),
            _file_watcher: Arc::new(RwLock::new(None)),
            enhancement_service: None,
        }
    }
    
    /// Create a new embedding manager with enhancement service
    pub fn new_with_enhancement(
        registry: Arc<RegistryService>,
        semantic_search: Arc<SemanticSearchService>,
        config: EmbeddingManagerConfig,
        enhancement_service: Arc<ToolEnhancementPipeline>,
    ) -> Self {
        Self {
            registry,
            semantic_search,
            config,
            last_known_state: Arc::new(RwLock::new(HashMap::new())),
            user_disabled_tools: Arc::new(RwLock::new(HashSet::new())),
            background_task_handle: Arc::new(RwLock::new(None)),
            _file_watcher: Arc::new(RwLock::new(None)),
            enhancement_service: Some(enhancement_service),
        }
    }
    
    /// Initialize the embedding manager
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing embedding manager");
        
        // Perform initial embedding sync
        let summary = self.sync_embeddings().await?;
        info!("Initial embedding sync completed: {} created, {} updated, {} removed", 
              summary.created, summary.updated, summary.removed);
        
        // Start background monitoring if enabled
        if self.config.background_monitoring {
            self.start_background_monitoring().await;
        }
        
        // Start file watching for hot-reload if enabled
        if self.config.enable_hot_reload {
            if let Err(e) = self.start_file_watching().await {
                warn!("Failed to start file watching for embeddings hot-reload: {}", e);
                info!("Falling back to timer-based monitoring only");
            }
        }
        
        Ok(())
    }
    
    /// Start background monitoring for tool changes
    async fn start_background_monitoring(&self) {
        let registry = Arc::clone(&self.registry);
        let semantic_search = Arc::clone(&self.semantic_search);
        let last_known_state = Arc::clone(&self.last_known_state);
        let user_disabled_tools = Arc::clone(&self.user_disabled_tools);
        let config = self.config.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.check_interval_seconds));
            
            loop {
                interval.tick().await;
                
                debug!("Running background embedding sync check");
                
                let manager = EmbeddingManager {
                    registry: Arc::clone(&registry),
                    semantic_search: Arc::clone(&semantic_search),
                    config: config.clone(),
                    last_known_state: Arc::clone(&last_known_state),
                    user_disabled_tools: Arc::clone(&user_disabled_tools),
                    background_task_handle: Arc::new(RwLock::new(None)), // Avoid circular reference
                    _file_watcher: Arc::new(RwLock::new(None)),
                    enhancement_service: None, // No enhancement service in background task
                };
                
                match manager.sync_embeddings().await {
                    Ok(summary) => {
                        if summary.created + summary.updated + summary.removed > 0 {
                            info!("Background embedding sync: {} created, {} updated, {} removed", 
                                  summary.created, summary.updated, summary.removed);
                        }
                    }
                    Err(e) => {
                        error!("Background embedding sync failed: {}", e);
                    }
                }
            }
        });
        
        let mut task_handle = self.background_task_handle.write().await;
        *task_handle = Some(handle);
        
        info!("Started background embedding monitoring (interval: {}s)", self.config.check_interval_seconds);
    }
    
    /// Start file watching for embeddings hot-reload
    async fn start_file_watching(&self) -> Result<()> {
        info!("Starting file watching for embeddings hot-reload");
        
        // Create a channel for file events
        let (tx, mut rx) = mpsc::unbounded_channel();
        
        // Clone necessary data for the event handler
        let semantic_search = Arc::clone(&self.semantic_search);
        let registry = Arc::clone(&self.registry);
        let last_known_state = Arc::clone(&self.last_known_state);
        let user_disabled_tools = Arc::clone(&self.user_disabled_tools);
        let config = self.config.clone();
        
        // Create the file watcher
        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    // Filter for relevant events on embedding files
                    if let EventKind::Modify(_) = event.kind {
                        for path in &event.paths {
                            let path_str = path.to_string_lossy();
                            if path_str.contains("tool_embeddings.bin") || 
                               path_str.contains("tool_metadata.json") || 
                               path_str.contains("content_hashes.json") {
                                info!("🔄 Embedding file changed: {}", path_str);
                                if let Err(e) = tx.send(path.clone()) {
                                    error!("Failed to send file change event: {}", e);
                                }
                                break; // Only send one event per file change batch
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("File watcher error: {}", e);
                }
            }
        }).map_err(|e| ProxyError::validation(format!("Failed to create file watcher: {}", e)))?;
        
        // Watch the embeddings directory
        let embeddings_dir = PathBuf::from("./data/embeddings");
        if embeddings_dir.exists() {
            watcher.watch(&embeddings_dir, RecursiveMode::NonRecursive)
                .map_err(|e| ProxyError::validation(format!("Failed to watch embeddings directory: {}", e)))?;
            info!("👀 Watching directory: {}", embeddings_dir.display());
        } else {
            warn!("Embeddings directory not found: {}", embeddings_dir.display());
        }
        
        // Store the watcher (keep it alive)
        {
            let mut file_watcher = self._file_watcher.write().await;
            *file_watcher = Some(Box::new(watcher));
        }
        
        // Spawn task to handle file change events
        tokio::spawn(async move {
            let mut debounce_timer: Option<tokio::time::Instant> = None;
            const DEBOUNCE_DURATION: std::time::Duration = std::time::Duration::from_millis(500);
            
            while let Some(changed_path) = rx.recv().await {
                // Debounce rapid file changes
                let now = tokio::time::Instant::now();
                if let Some(last_change) = debounce_timer {
                    if now.duration_since(last_change) < DEBOUNCE_DURATION {
                        continue; // Skip this change, too soon
                    }
                }
                debounce_timer = Some(now);
                
                info!("🔥 Hot-reloading embeddings due to file change: {}", changed_path.display());
                
                // Reload embeddings in the semantic search service
                match semantic_search.reload_embeddings().await {
                    Ok(()) => {
                        info!("✅ Successfully hot-reloaded embeddings");
                        
                        // Also trigger a sync to update any tool changes
                        let manager = EmbeddingManager {
                            registry: Arc::clone(&registry),
                            semantic_search: Arc::clone(&semantic_search),
                            config: config.clone(),
                            last_known_state: Arc::clone(&last_known_state),
                            user_disabled_tools: Arc::clone(&user_disabled_tools),
                            background_task_handle: Arc::new(RwLock::new(None)),
                            _file_watcher: Arc::new(RwLock::new(None)),
                            enhancement_service: None, // No enhancement service in file watcher task
                        };
                        
                        if let Err(e) = manager.sync_embeddings().await {
                            warn!("Failed to sync embeddings after hot-reload: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("❌ Failed to hot-reload embeddings: {}", e);
                    }
                }
                
                // Small delay to avoid hammering the system
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
        
        info!("🔥 Hot-reload file watching started for embeddings");
        Ok(())
    }
    
    /// Synchronize embeddings with current tool state
    pub async fn sync_embeddings(&self) -> Result<EmbeddingChangeSummary> {
        let start_time = SystemTime::now();
        info!("Starting embedding synchronization");
        
        // Get current tool state from registry
        let current_tools = self.get_current_tool_state().await;
        let mut last_state = self.last_known_state.write().await;
        
        // Detect changes
        let mut operations = Vec::new();
        let mut created = 0;
        let mut updated = 0;
        let mut removed = 0;
        let mut failed = 0;
        
        // Check for new or updated tools
        for (tool_name, (current_hash, enabled, hidden)) in &current_tools {
            let operation_status = if let Some((last_hash, last_enabled, last_hidden)) = last_state.get(tool_name) {
                // Tool exists, check for changes
                if current_hash != last_hash {
                    EmbeddingStatus::NeedsUpdate
                } else if enabled != last_enabled || hidden != last_hidden {
                    // State changed but content didn't - update metadata only
                    EmbeddingStatus::NeedsUpdate
                } else {
                    EmbeddingStatus::UpToDate
                }
            } else {
                // New tool
                EmbeddingStatus::NeedsCreation
            };
            
            if operation_status != EmbeddingStatus::UpToDate {
                let result = self.handle_tool_embedding(
                    tool_name,
                    operation_status.clone(),
                    *enabled,
                    *hidden,
                ).await;
                
                let success = result.is_ok();
                if !success {
                    failed += 1;
                } else {
                    match operation_status {
                        EmbeddingStatus::NeedsCreation => created += 1,
                        EmbeddingStatus::NeedsUpdate => updated += 1,
                        _ => {}
                    }
                }
                
                operations.push(EmbeddingOperation {
                    tool_name: tool_name.clone(),
                    status: operation_status,
                    reason: self.get_operation_reason(tool_name, &last_state, &current_tools),
                    success,
                    error: result.err().map(|e| e.to_string()),
                });
            }
        }
        
        // Check for removed tools
        let current_tool_names: HashSet<_> = current_tools.keys().collect();
        let removed_tools: Vec<_> = last_state.keys()
            .filter(|name| !current_tool_names.contains(name))
            .cloned()
            .collect();
        
        for tool_name in removed_tools {
            let result = self.remove_tool_embedding(&tool_name).await;
            let success = result.is_ok();
            
            if success {
                removed += 1;
            } else {
                failed += 1;
            }
            
            operations.push(EmbeddingOperation {
                tool_name: tool_name.clone(),
                status: EmbeddingStatus::ShouldRemove,
                reason: "Tool no longer exists in registry".to_string(),
                success,
                error: result.err().map(|e| e.to_string()),
            });
        }
        
        // Update last known state
        *last_state = current_tools;
        drop(last_state);
        
        // Auto-save if configured
        if self.config.auto_save && (created + updated + removed > 0) {
            if let Err(e) = self.semantic_search.save_embeddings().await {
                error!("Failed to auto-save embeddings: {}", e);
            }
        }
        
        let duration = start_time.elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;
        
        let summary = EmbeddingChangeSummary {
            total_processed: operations.len(),
            created,
            updated,
            removed,
            failed,
            operations,
            duration_ms: duration,
        };
        
        info!("Embedding synchronization completed in {}ms: {} created, {} updated, {} removed, {} failed",
              duration, created, updated, removed, failed);
        
        Ok(summary)
    }
    
    /// Get current tool state from registry (using enhanced tools if available)
    async fn get_current_tool_state(&self) -> HashMap<String, (String, bool, bool)> {
        let mut tool_state = HashMap::new();
        
        // Get enhanced tools if enhancement service is available, otherwise base tools
        let tools = if let Some(enhancement_service) = &self.enhancement_service {
            info!("🔄 Getting enhanced tools for embedding state calculation");
            match enhancement_service.get_enhanced_tools().await {
                Ok(enhanced_tools) => {
                    let mut tools = Vec::new();
                    for (name, enhanced_tool) in enhanced_tools {
                        let mut tool_def = enhanced_tool.base.clone();
                        
                        // Use enhanced description if available
                        if let Some(enhanced_desc) = &enhanced_tool.sampling_enhanced_description {
                            tool_def.description = enhanced_desc.clone();
                        }
                        
                        tools.push((name, tool_def));
                    }
                    tools
                }
                Err(e) => {
                    warn!("Failed to get enhanced tools for embedding state: {}. Using base tools.", e);
                    self.registry.get_enabled_tools().into_iter().collect()
                }
            }
        } else {
            debug!("🔧 Using base tools for embedding state (no enhancement service)");
            self.registry.get_enabled_tools().into_iter().collect()
        };
        
        for (tool_name, tool_def) in tools {
            // Skip smart_discovery_tool itself to avoid recursion
            if tool_name == "smart_discovery_tool" || tool_name == "smart_tool_discovery" {
                continue;
            }
            
            let content_hash = self.semantic_search.generate_content_hash(&tool_def);
            tool_state.insert(tool_name, (content_hash, tool_def.enabled, tool_def.hidden));
        }
        
        tool_state
    }
    
    /// Handle embedding operation for a specific tool (using enhanced tools if available)
    async fn handle_tool_embedding(
        &self,
        tool_name: &str,
        status: EmbeddingStatus,
        enabled: bool,
        hidden: bool,
    ) -> Result<()> {
        // Get the tool definition (potentially enhanced)
        let tool_def = if let Some(enhancement_service) = &self.enhancement_service {
            // Try to get enhanced tool first
            match enhancement_service.get_enhanced_tools().await {
                Ok(enhanced_tools) => {
                    if let Some(enhanced_tool) = enhanced_tools.get(tool_name) {
                        debug!("✨ Using enhanced tool definition for embedding: {}", tool_name);
                        let mut tool_def = enhanced_tool.base.clone();
                        
                        // Use enhanced description if available
                        if let Some(enhanced_desc) = &enhanced_tool.sampling_enhanced_description {
                            info!("🎯 Using sampling-enhanced description for embedding: {}", tool_name);
                            tool_def.description = enhanced_desc.clone();
                        }
                        
                        tool_def
                    } else {
                        // Fallback to base tool
                        let arc_tool = self.registry.get_tool(tool_name)
                            .ok_or_else(|| ProxyError::validation(format!("Tool '{}' not found", tool_name)))?;
                        (*arc_tool).clone()
                    }
                }
                Err(e) => {
                    warn!("Failed to get enhanced tools for embedding: {}. Using base tool.", e);
                    let arc_tool = self.registry.get_tool(tool_name)
                        .ok_or_else(|| ProxyError::validation(format!("Tool '{}' not found", tool_name)))?;
                    (*arc_tool).clone()
                }
            }
        } else {
            // Use base tool
            let arc_tool = self.registry.get_tool(tool_name)
                .ok_or_else(|| ProxyError::validation(format!("Tool '{}' not found", tool_name)))?;
            (*arc_tool).clone()
        };
        
        // Check if this is an external MCP tool that user has disabled
        if self.config.preserve_user_settings {
            let user_disabled = self.user_disabled_tools.read().await;
            if user_disabled.contains(tool_name) && enabled {
                debug!("Preserving user disabled setting for tool: {}", tool_name);
                return Ok(()); // Skip embedding creation for user-disabled tools
            }
        }
        
        // Create the text to embed (combine name and description)
        let embedding_text = format!("{}: {}", tool_def.name, tool_def.description);
        
        // Generate embedding
        let embedding = self.semantic_search.generate_embedding(&embedding_text).await?;
        
        // Create metadata
        let metadata = ToolMetadata {
            name: tool_name.to_string(),
            description: tool_def.description.clone(),
            enabled,
            hidden,
            content_hash: self.semantic_search.generate_content_hash(&tool_def),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            embedding_dims: embedding.len(),
        };
        
        // Store the embedding
        let mut storage = self.semantic_search.storage.write().await;
        storage.add_tool_embedding(tool_name.to_string(), embedding, metadata);
        
        debug!("Handled embedding for tool '{}' with status: {:?}", tool_name, status);
        Ok(())
    }
    
    /// Remove tool embedding
    async fn remove_tool_embedding(&self, tool_name: &str) -> Result<()> {
        let mut storage = self.semantic_search.storage.write().await;
        storage.remove_tool_embedding(tool_name);
        debug!("Removed embedding for tool: {}", tool_name);
        Ok(())
    }
    
    /// Get reason for an operation
    fn get_operation_reason(
        &self,
        tool_name: &str,
        last_state: &HashMap<String, (String, bool, bool)>,
        current_state: &HashMap<String, (String, bool, bool)>,
    ) -> String {
        if let Some((last_hash, last_enabled, last_hidden)) = last_state.get(tool_name) {
            if let Some((current_hash, current_enabled, current_hidden)) = current_state.get(tool_name) {
                let mut reasons = Vec::new();
                
                if last_hash != current_hash {
                    reasons.push("content changed");
                }
                if last_enabled != current_enabled {
                    reasons.push(if *current_enabled { "enabled" } else { "disabled" });
                }
                if last_hidden != current_hidden {
                    reasons.push(if *current_hidden { "hidden" } else { "made visible" });
                }
                
                if reasons.is_empty() {
                    "no changes detected".to_string()
                } else {
                    reasons.join(", ")
                }
            } else {
                "tool removed".to_string()
            }
        } else {
            "new tool".to_string()
        }
    }
    
    /// Mark a tool as user-disabled to preserve the setting
    pub async fn mark_user_disabled(&self, tool_name: &str) {
        let mut user_disabled = self.user_disabled_tools.write().await;
        user_disabled.insert(tool_name.to_string());
        info!("Marked tool '{}' as user-disabled (will be preserved during external updates)", tool_name);
    }
    
    /// Remove user-disabled marking
    pub async fn unmark_user_disabled(&self, tool_name: &str) {
        let mut user_disabled = self.user_disabled_tools.write().await;
        user_disabled.remove(tool_name);
        info!("Removed user-disabled marking for tool '{}'", tool_name);
    }
    
    /// Check if a tool is marked as user-disabled
    pub async fn is_user_disabled(&self, tool_name: &str) -> bool {
        let user_disabled = self.user_disabled_tools.read().await;
        user_disabled.contains(tool_name)
    }
    
    /// Get statistics about the embedding manager
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        let last_state = self.last_known_state.read().await;
        let user_disabled = self.user_disabled_tools.read().await;
        
        stats.insert("last_known_tools".to_string(), serde_json::Value::Number(last_state.len().into()));
        stats.insert("user_disabled_tools".to_string(), serde_json::Value::Number(user_disabled.len().into()));
        stats.insert("background_monitoring".to_string(), serde_json::Value::Bool(self.config.background_monitoring));
        stats.insert("check_interval_seconds".to_string(), serde_json::Value::Number(self.config.check_interval_seconds.into()));
        stats.insert("auto_save".to_string(), serde_json::Value::Bool(self.config.auto_save));
        stats.insert("preserve_user_settings".to_string(), serde_json::Value::Bool(self.config.preserve_user_settings));
        
        // Get semantic search stats
        let semantic_stats = self.semantic_search.get_stats().await;
        for (key, value) in semantic_stats {
            stats.insert(format!("semantic_{}", key), value);
        }
        
        stats
    }
    
    /// Force a manual sync (useful for external triggers)
    pub async fn force_sync(&self) -> Result<EmbeddingChangeSummary> {
        info!("Forcing manual embedding sync");
        self.sync_embeddings().await
    }
    
    /// Shutdown the embedding manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down embedding manager");
        
        // Stop background task
        let mut task_handle = self.background_task_handle.write().await;
        if let Some(handle) = task_handle.take() {
            handle.abort();
            info!("Stopped background embedding monitoring");
        }
        
        // Save embeddings if needed
        if self.config.auto_save {
            self.semantic_search.save_embeddings().await?;
            info!("Saved embeddings during shutdown");
        }
        
        Ok(())
    }
}

impl Drop for EmbeddingManager {
    fn drop(&mut self) {
        // Note: We can't call async methods in Drop, so this is just for cleanup
        debug!("Embedding manager dropped");
    }
}