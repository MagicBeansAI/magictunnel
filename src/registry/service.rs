//! High-performance registry service with hot-reloading and concurrent access
//!
//! This module implements the enterprise-scale capability registry with:
//! - Sub-millisecond lookups using lock-free data structures
//! - Near-instant hot-reloading with file system watching
//! - Parallel YAML processing across CPU cores
//! - Smart caching with incremental updates

use crate::config::RegistryConfig;
use crate::error::{ProxyError, Result};
use crate::registry::types::*;
use crate::registry::loader::RegistryLoader;
use crate::mcp::notifications::McpNotificationManager;
use crate::config::hierarchical::{ConfigResolver as HierarchicalResolver, ResolvedToolConfig};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use futures_util::future;
use globset::{Glob, GlobMatcher};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

/// Callback trait for enhancement services to be notified of tool changes
#[async_trait::async_trait]
pub trait EnhancementCallback: Send + Sync {
    /// Called when tools are added, updated, or reloaded
    async fn on_tools_changed(&self, changed_tools: Vec<(String, ToolDefinition)>) -> crate::error::Result<()>;
}

/// High-performance registry service with hot-reloading
pub struct RegistryService {
    /// Lock-free atomic registry for zero-contention reads
    registry: ArcSwap<CapabilityRegistry>,

    /// Concurrent cache for fast lookups
    cache: DashMap<String, Arc<CapabilityFile>>,

    /// Compiled glob patterns for performance
    patterns: Vec<GlobMatcher>,

    /// Configuration
    config: RegistryConfig,

    /// Registry loader for enhanced format support
    loader: RegistryLoader,

    /// File modification times for incremental updates
    file_times: DashMap<PathBuf, SystemTime>,

    /// File watcher for hot-reload
    _watcher: Option<RecommendedWatcher>,

    /// Channel for file system events
    event_rx: Option<mpsc::UnboundedReceiver<Event>>,

    /// Optional notification manager for MCP list_changed notifications
    notification_manager: RwLock<Option<Arc<McpNotificationManager>>>,
    
    /// Optional enhancement callback for tool changes
    enhancement_callback: RwLock<Option<Arc<dyn EnhancementCallback>>>,
    
    /// Optional hierarchical configuration resolver for tool-level precedence
    hierarchical_resolver: RwLock<Option<Arc<HierarchicalResolver>>>,
}

/// Complete capability registry with metadata
#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    /// All capability files indexed by path
    files: HashMap<PathBuf, Arc<CapabilityFile>>,
    
    /// Tools indexed by name for fast lookup
    tools: HashMap<String, Arc<ToolDefinition>>,
    
    /// Registry metadata
    metadata: RegistryMetadata,
}

/// Registry metadata for performance tracking
#[derive(Debug, Clone)]
pub struct RegistryMetadata {
    /// Total number of capability files
    pub file_count: usize,

    /// Total number of tools
    pub tool_count: usize,

    /// Last update timestamp
    pub last_updated: Instant,

    /// Load duration in milliseconds
    pub load_duration_ms: u64,
}

impl CapabilityRegistry {
    /// Get all tools with their definitions
    pub fn get_all_tools(&self) -> Vec<(String, ToolDefinition)> {
        self.tools.iter()
            .map(|(name, tool_def)| (name.clone(), (**tool_def).clone()))
            .collect()
    }

    /// Get a tool definition by name
    pub fn get_tool(&self, name: &str) -> Option<ToolDefinition> {
        self.tools.get(name).map(|tool_def| (**tool_def).clone())
    }

    /// Get registry metadata
    pub fn metadata(&self) -> &RegistryMetadata {
        &self.metadata
    }
}

impl RegistryService {
    /// Create a new high-performance registry service
    pub async fn new(config: RegistryConfig) -> Result<Self> {
        info!("Initializing high-performance registry service");
        
        // Compile glob patterns for performance
        let patterns = Self::compile_glob_patterns(&config.paths)?;
        
        // Create initial empty registry
        let initial_registry = CapabilityRegistry {
            files: HashMap::new(),
            tools: HashMap::new(),
            metadata: RegistryMetadata {
                file_count: 0,
                tool_count: 0,
                last_updated: Instant::now(),
                load_duration_ms: 0,
            },
        };
        
        // Create registry loader with the same config
        let loader = RegistryLoader::new(config.clone());
        
        let mut service = Self {
            registry: ArcSwap::from_pointee(initial_registry),
            cache: DashMap::new(),
            patterns,
            config,
            loader,
            file_times: DashMap::new(),
            _watcher: None,
            event_rx: None,
            notification_manager: RwLock::new(None),
            enhancement_callback: RwLock::new(None),
            hierarchical_resolver: RwLock::new(None),
        };
        
        // Perform initial load (without enhancement notifications to avoid overriding cached enhancements)
        service.reload_registry_internal(false).await?;
        
        // Set up file watching if enabled
        if service.config.hot_reload {
            info!("🔧 DEBUG: Hot-reload is enabled in new() - setting up file watcher");
            service.setup_file_watcher().await?;
        } else {
            info!("🔧 DEBUG: Hot-reload is DISABLED in new() - config.hot_reload = {}", service.config.hot_reload);
        }
        
        info!("Registry service initialized successfully");
        Ok(service)
    }

    /// Create and start the registry service with hot-reload in background
    pub async fn start_with_hot_reload(config: RegistryConfig) -> Result<Arc<Self>> {
        let mut service = Self::new(config.clone()).await?;
        
        if config.hot_reload {
            info!("🔧 DEBUG: Hot-reload is enabled in start_with_hot_reload() - skipping duplicate setup (already done in new())");
            // Skip duplicate setup - already done in new()
        } else {
            info!("🔧 DEBUG: Hot-reload is DISABLED in start_with_hot_reload() - config.hot_reload = {}", config.hot_reload);
        }
        
        let service = Arc::new(service);
        
        if config.hot_reload {
            info!("🔧 DEBUG: Starting hot-reload background task");
            let service_clone = service.clone();
            
            // Start the hot-reload background task
            tokio::spawn(async move {
                info!("🔧 DEBUG: Hot-reload background task started");
                Self::run_hot_reload_loop(service_clone).await;
            });
        } else {
            info!("🔧 DEBUG: NOT starting hot-reload background task - config.hot_reload = {}", config.hot_reload);
        }
        
        Ok(service)
    }
    
    /// Run the hot-reload loop (static method to avoid ownership issues)
    async fn run_hot_reload_loop(service: Arc<Self>) {
        // We need to create a new watcher since we can't access the existing one from Arc
        let (tx, mut rx) = mpsc::unbounded_channel();
        
        let mut watcher = match RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                match res {
                    Ok(event) => {
                        if let Err(e) = tx.send(event) {
                            error!("Failed to send file system event: {}", e);
                        }
                    }
                    Err(e) => error!("File system watch error: {}", e),
                }
            },
            Config::default(),
        ) {
            Ok(watcher) => watcher,
            Err(e) => {
                error!("Failed to create file watcher for hot-reload: {}", e);
                return;
            }
        };
        
        // Watch all configured paths
        for path_str in &service.config.paths {
            let path = Path::new(path_str);
            if path.exists() {
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    error!("Failed to watch path {} for hot-reload: {}", path_str, e);
                } else {
                    info!("Hot-reload watching path: {}", path_str);
                }
            }
        }
        
        info!("Hot-reload background task started");
        
        let mut debounce_timer: Option<tokio::time::Instant> = None;
        const DEBOUNCE_DURATION: Duration = Duration::from_millis(100);

        loop {
            tokio::select! {
                // Handle file system events
                event = rx.recv() => {
                    match event {
                        Some(event) => {
                            info!("🔧 DEBUG: Hot-reload file system event: {:?}", event);

                            // Check if this affects our capability files
                            let is_relevant = Self::should_reload_for_event_static(&service.config.paths, &event);
                            info!("🔧 DEBUG: Event relevance check: {} for paths: {:?}", is_relevant, service.config.paths);
                            if is_relevant {
                                info!("🔧 DEBUG: Hot-reload detected relevant file change - setting debounce timer");
                                // Set debounce timer
                                debounce_timer = Some(tokio::time::Instant::now() + DEBOUNCE_DURATION);
                            } else {
                                info!("🔧 DEBUG: File change not relevant for hot-reload");
                            }
                        }
                        None => {
                            warn!("Hot-reload file system event channel closed");
                            break;
                        }
                    }
                }

                // Handle debounced reload
                _ = tokio::time::sleep(Duration::from_millis(50)), if debounce_timer.is_some() => {
                    if let Some(timer) = debounce_timer {
                        if tokio::time::Instant::now() >= timer {
                            info!("🔧 DEBUG: Debounce timer expired - triggering reload");
                            info!("🔄 Triggering hot-reload due to file changes");

                            // Reload the registry
                            if let Err(e) = service.reload_registry().await {
                                error!("❌ Hot-reload failed: {}", e);
                            } else {
                                info!("✅ Hot-reload completed successfully");
                            }

                            debounce_timer = None;
                        } else {
                            info!("🔧 DEBUG: Debounce timer not yet expired");
                        }
                    }
                }
            }
        }
    }
    
    /// Static version of should_reload_for_event for use in hot-reload loop
    fn should_reload_for_event_static(config_paths: &[String], event: &Event) -> bool {
        use notify::EventKind;

        // Only care about write/create/remove events
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                // Check if any affected paths are YAML files we care about
                event.paths.iter().any(|path| {
                    // Check if it's a YAML file
                    let is_yaml = if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        ext_str == "yaml" || ext_str == "yml"
                    } else {
                        false
                    };
                    
                    if !is_yaml {
                        return false;
                    }
                    
                    // Check if path is within our watched directories
                    for config_path in config_paths {
                        // Normalize the config path to absolute path for comparison
                        let config_path = if config_path.starts_with("./") {
                            // Convert relative path to absolute
                            if let Ok(current_dir) = std::env::current_dir() {
                                current_dir.join(&config_path[2..])
                            } else {
                                Path::new(config_path).to_path_buf()
                            }
                        } else {
                            Path::new(config_path).to_path_buf()
                        };
                        
                        // Canonicalize both paths for accurate comparison
                        let config_canonical = config_path.canonicalize().unwrap_or(config_path);
                        let path_canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                        
                        if path_canonical.starts_with(&config_canonical) || path_canonical == config_canonical {
                            return true;
                        }
                    }
                    false
                })
            }
            _ => false,
        }
    }
    
    /// Get a tool definition by name (sub-microsecond lookup)
    pub fn get_tool(&self, name: &str) -> Option<Arc<ToolDefinition>> {
        // First check cache for fastest access
        if let Some(file) = self.cache.get(name) {
            return file.get_tool(name).map(|t| Arc::new(t.clone()));
        }
        
        // Fallback to registry lookup
        let registry = self.registry.load();
        registry.tools.get(name).cloned()
    }
    
    /// List all available tools (visible and enabled only)
    pub fn list_tools(&self) -> Vec<String> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(name, tool_def)| {
                !self.is_tool_hidden_hierarchical(name, tool_def) && self.is_tool_enabled_hierarchical(name, tool_def)
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// List all tools including hidden ones
    pub fn list_all_tools(&self) -> Vec<String> {
        let registry = self.registry.load();
        registry.tools.keys().cloned().collect()
    }

    /// List only visible tools
    pub fn list_visible_tools(&self) -> Vec<String> {
        self.list_tools() // Same as list_tools for backward compatibility
    }

    /// List only hidden tools
    pub fn list_hidden_tools(&self) -> Vec<String> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(_, tool_def)| tool_def.is_hidden())
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// List only enabled tools (regardless of visibility)
    pub fn list_enabled_tools(&self) -> Vec<String> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(_, tool_def)| tool_def.is_enabled())
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// List only disabled tools
    pub fn list_disabled_tools(&self) -> Vec<String> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(_, tool_def)| !tool_def.is_enabled())
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// List tools that are both visible and enabled (active tools)
    pub fn list_active_tools(&self) -> Vec<String> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(_, tool_def)| !tool_def.is_hidden() && tool_def.is_enabled())
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// List tools that are enabled but hidden (discoverable tools)
    pub fn list_discoverable_tools(&self) -> Vec<String> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(_, tool_def)| tool_def.is_hidden() && tool_def.is_enabled())
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get all active tools with their definitions (visible and enabled)
    pub fn get_all_tools(&self) -> Vec<(String, ToolDefinition)> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(name, tool_def)| {
                !self.is_tool_hidden_hierarchical(name, tool_def) && self.is_tool_enabled_hierarchical(name, tool_def)
            })
            .map(|(name, tool_def)| (name.clone(), (**tool_def).clone()))
            .collect()
    }

    /// Get all tools including hidden ones with their definitions
    pub fn get_all_tools_including_hidden(&self) -> Vec<(String, ToolDefinition)> {
        let registry = self.registry.load();
        registry.tools.iter()
            .map(|(name, tool_def)| (name.clone(), (**tool_def).clone()))
            .collect()
    }

    /// Get only visible tools with their definitions
    pub fn get_visible_tools(&self) -> Vec<(String, ToolDefinition)> {
        self.get_all_tools() // Same as get_all_tools for backward compatibility
    }

    /// Get only hidden tools with their definitions
    pub fn get_hidden_tools(&self) -> Vec<(String, ToolDefinition)> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(name, tool_def)| self.is_tool_hidden_hierarchical(name, tool_def))
            .map(|(name, tool_def)| (name.clone(), (**tool_def).clone()))
            .collect()
    }

    /// Get only enabled tools with their definitions (regardless of visibility)
    pub fn get_enabled_tools(&self) -> Vec<(String, ToolDefinition)> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(name, tool_def)| self.is_tool_enabled_hierarchical(name, tool_def))
            .map(|(name, tool_def)| (name.clone(), (**tool_def).clone()))
            .collect()
    }

    /// Get only disabled tools with their definitions
    pub fn get_disabled_tools(&self) -> Vec<(String, ToolDefinition)> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(name, tool_def)| !self.is_tool_enabled_hierarchical(name, tool_def))
            .map(|(name, tool_def)| (name.clone(), (**tool_def).clone()))
            .collect()
    }

    /// Get discoverable tools with their definitions (enabled but hidden)
    pub fn get_discoverable_tools(&self) -> Vec<(String, ToolDefinition)> {
        let registry = self.registry.load();
        registry.tools.iter()
            .filter(|(name, tool_def)| {
                self.is_tool_hidden_hierarchical(name, tool_def) && self.is_tool_enabled_hierarchical(name, tool_def)
            })
            .map(|(name, tool_def)| (name.clone(), (**tool_def).clone()))
            .collect()
    }
    
    /// Get all tools with their server and capability context for allowlist processing
    /// Returns: Vec<(tool_name, tool_definition, server, capability)>
    pub fn get_all_tools_with_context(&self) -> Vec<(String, ToolDefinition, String, String)> {
        let registry = self.registry.load();
        let mut tools_with_context = Vec::new();
        
        // Iterate through all capability files to get server/capability context
        for (file_path, capability_file) in &registry.files {
            // Try to get capability name from metadata first, fallback to path extraction
            let capability_name = if let Some(ref metadata) = capability_file.metadata {
                if let Some(ref name) = metadata.name {
                    name.clone()
                } else {
                    // Fallback to filename without extension
                    file_path.file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                }
            } else {
                // Fallback to filename without extension
                file_path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            };
            
            // Extract server from directory structure, or use "internal" for capability files
            let server = if let Some(parent) = file_path.parent() {
                if let Some(parent_name) = parent.file_name() {
                    parent_name.to_string_lossy().to_string()
                } else {
                    "internal".to_string()
                }
            } else {
                "internal".to_string()
            };
            
            // Add all tools from this capability file with their context
            for tool in &capability_file.tools {
                tools_with_context.push((
                    tool.name.clone(),
                    tool.clone(),
                    server.clone(),
                    capability_name.clone(),
                ));
            }
            
            // Handle enhanced tools if present (MCP 2025-06-18 format)
            if let Some(enhanced_tools) = capability_file.get_enhanced_tools() {
                for enhanced_tool in enhanced_tools {
                    // Convert enhanced tool to regular tool definition for compatibility
                    let regular_tool: crate::registry::types::ToolDefinition = enhanced_tool.into();
                    tools_with_context.push((
                        regular_tool.name.clone(),
                        regular_tool,
                        server.clone(),
                        capability_name.clone(),
                    ));
                }
            }
        }
        
        tools_with_context
    }
    
    /// Extract server and capability from capability file path
    /// Example: "capabilities/external-mcp/filesystem.yaml" -> ("external-mcp", "filesystem")
    fn extract_server_capability_from_path(&self, file_path: &std::path::Path) -> (String, String) {
        let path_str = file_path.to_string_lossy();
        
        // Split path and find capabilities directory
        let parts: Vec<&str> = path_str.split('/').collect();
        
        // Look for the pattern: .../capabilities/{server}/{capability}.yaml
        if let Some(capabilities_idx) = parts.iter().rposition(|&part| part == "capabilities") {
            if capabilities_idx + 2 < parts.len() {
                let server = parts[capabilities_idx + 1].to_string();
                let capability_file = parts[capabilities_idx + 2];
                // Remove .yaml extension to get capability name
                let capability = capability_file.strip_suffix(".yaml")
                    .unwrap_or(capability_file)
                    .to_string();
                return (server, capability);
            }
        }
        
        // Fallback: try to extract from filename and parent directory
        if let Some(file_name) = file_path.file_stem() {
            let capability = file_name.to_string_lossy().to_string();
            if let Some(parent) = file_path.parent() {
                if let Some(server_name) = parent.file_name() {
                    return (server_name.to_string_lossy().to_string(), capability);
                }
            }
        }
        
        // Ultimate fallback
        ("unknown".to_string(), "general".to_string())
    }
    
    /// Get registry metadata for monitoring
    pub fn metadata(&self) -> RegistryMetadata {
        let registry = self.registry.load();
        registry.metadata.clone()
    }

    /// Set tool visibility by name
    pub async fn set_tool_hidden(&self, tool_name: &str, hidden: bool) -> Result<()> {
        // This is a read-only operation on the current registry
        // For persistent changes, tools need to be modified in their source files
        warn!("set_tool_hidden called for '{}' with hidden={}, but registry is read-only. Modify source capability files for persistent changes.", tool_name, hidden);
        Err(crate::error::ProxyError::registry(
            "Registry is read-only. Modify source capability files to change tool visibility.".to_string()
        ))
    }

    /// Check if a tool is hidden
    pub fn is_tool_hidden(&self, tool_name: &str) -> Option<bool> {
        self.get_tool(tool_name).map(|tool_def| tool_def.is_hidden())
    }

    /// Get visibility statistics
    pub fn visibility_stats(&self) -> (usize, usize, usize) {
        let registry = self.registry.load();
        let total = registry.tools.len();
        let hidden = registry.tools.values().filter(|tool| tool.is_hidden()).count();
        let visible = total - hidden;
        (total, visible, hidden)
    }

    /// Set tool enabled status by name
    pub async fn set_tool_enabled(&self, tool_name: &str, enabled: bool) -> Result<()> {
        // This is a read-only operation on the current registry
        // For persistent changes, tools need to be modified in their source files
        warn!("set_tool_enabled called for '{}' with enabled={}, but registry is read-only. Modify source capability files for persistent changes.", tool_name, enabled);
        Err(crate::error::ProxyError::registry(
            "Registry is read-only. Modify source capability files to change tool enabled status.".to_string()
        ))
    }

    /// Check if a tool is enabled
    pub fn is_tool_enabled(&self, tool_name: &str) -> Option<bool> {
        self.get_tool(tool_name).map(|tool_def| tool_def.is_enabled())
    }

    /// Get enabled statistics
    pub fn enabled_stats(&self) -> (usize, usize, usize) {
        let registry = self.registry.load();
        let total = registry.tools.len();
        let enabled = registry.tools.values().filter(|tool| tool.is_enabled()).count();
        let disabled = total - enabled;
        (total, enabled, disabled)
    }

    /// Get comprehensive tool statistics (visibility + enabled)
    pub fn tool_stats(&self) -> (usize, usize, usize, usize, usize, usize) {
        let registry = self.registry.load();
        let total = registry.tools.len();
        let visible = registry.tools.values().filter(|tool| !tool.is_hidden()).count();
        let hidden = total - visible;
        let enabled = registry.tools.values().filter(|tool| tool.is_enabled()).count();
        let disabled = total - enabled;
        let active = registry.tools.values().filter(|tool| !tool.is_hidden() && tool.is_enabled()).count();
        (total, visible, hidden, enabled, disabled, active)
    }

    /// Force reload the entire registry using high-performance parallel pipeline
    pub async fn reload_registry(&self) -> Result<()> {
        self.reload_registry_internal(true).await
    }
    
    /// Internal reload method with option to skip enhancement notifications
    async fn reload_registry_internal(&self, notify_enhancements: bool) -> Result<()> {
        let start_time = Instant::now();
        info!("Starting registry reload with parallel pipeline");

        // Execute the 5-phase parallel processing pipeline (force full reload)
        let new_registry = self.execute_parallel_pipeline(false).await?;

        let load_duration = start_time.elapsed();

        // Get current tools before swap to detect changes
        let old_tools: HashMap<String, ToolDefinition> = {
            let old_registry = self.registry.load();
            old_registry.tools.iter()
                .map(|(name, tool_def)| (name.clone(), (**tool_def).clone()))
                .collect()
        };

        // Atomic swap - zero downtime update
        self.registry.store(Arc::new(new_registry));

        // Update cache
        self.update_cache().await;

        // Detect changed tools and notify enhancement service
        let new_tools: HashMap<String, ToolDefinition> = {
            let current_registry = self.registry.load();
            current_registry.tools.iter()
                .map(|(name, tool_def)| (name.clone(), (**tool_def).clone()))
                .collect()
        };

        // Find tools that are new or changed
        let mut changed_tools = Vec::new();
        for (tool_name, new_tool) in &new_tools {
            if let Some(old_tool) = old_tools.get(tool_name) {
                // Check if tool changed (compare description and schema)
                if old_tool.description != new_tool.description || 
                   serde_json::to_string(&old_tool.input_schema).unwrap_or_default() != 
                   serde_json::to_string(&new_tool.input_schema).unwrap_or_default() {
                    debug!("Tool '{}' has changed, will trigger enhancement", tool_name);
                    changed_tools.push((tool_name.clone(), new_tool.clone()));
                }
            } else {
                // New tool
                debug!("Tool '{}' is new, will trigger enhancement", tool_name);
                changed_tools.push((tool_name.clone(), new_tool.clone()));
            }
        }

        info!(
            "Registry reload completed in {}ms - {} files, {} tools ({} changed)",
            load_duration.as_millis(),
            self.metadata().file_count,
            self.metadata().tool_count,
            changed_tools.len()
        );

        // Notify enhancement service of changes (async) - only if requested
        if notify_enhancements && !changed_tools.is_empty() {
            self.notify_tools_changed(changed_tools).await;
        } else if !notify_enhancements && !changed_tools.is_empty() {
            info!("Skipping enhancement notification for {} tools (initial load)", changed_tools.len());
        }

        // Send tools list_changed notification
        self.notify_tools_list_changed();

        Ok(())
    }

    /// Execute the 5-phase parallel processing pipeline
    /// Phase 1: Discovery - Find all capability files
    /// Phase 2: Loading - Read file contents in parallel
    /// Phase 3: Parsing - Parse YAML content in parallel
    /// Phase 4: Validation - Validate capability files in parallel
    /// Phase 5: Update - Build registry and update cache
    async fn execute_parallel_pipeline(&self, incremental: bool) -> Result<CapabilityRegistry> {
        let pipeline_start = Instant::now();

        // Phase 1: Discovery - Find all capability files
        let discovery_start = Instant::now();
        let file_paths = self.discover_capability_files().await?;
        let discovery_duration = discovery_start.elapsed();
        debug!("Phase 1 (Discovery): Found {} files in {:?}", file_paths.len(), discovery_duration);

        // Phase 2: Loading - Read file contents in parallel with optional incremental updates
        let loading_start = Instant::now();
        let file_contents: Vec<(PathBuf, String)> = file_paths
            .par_iter()
            .filter_map(|path| {
                // Check if file has been modified since last load (only if incremental is enabled)
                if incremental {
                    if let Ok(metadata) = std::fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            if let Some(last_modified) = self.file_times.get(path) {
                                if modified <= *last_modified {
                                    // File hasn't changed, skip loading
                                    debug!("Skipping unchanged file: {}", path.display());
                                    return None;
                                }
                            }
                            // Update modification time
                            self.file_times.insert(path.clone(), modified);
                        }
                    }
                } else {
                    // For full reload, always update modification time
                    if let Ok(metadata) = std::fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            self.file_times.insert(path.clone(), modified);
                        }
                    }
                }

                // Load file content
                match std::fs::read_to_string(path) {
                    Ok(content) => Some(Ok((path.clone(), content))),
                    Err(e) => Some(Err(ProxyError::registry(format!("Failed to read file {}: {}", path.display(), e)))),
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let loading_duration = loading_start.elapsed();
        debug!("Phase 2 (Loading): Loaded {} files in {:?} ({})",
               file_contents.len(), loading_duration,
               if incremental { "incremental" } else { "full" });

        // Phase 3: Parsing - Parse YAML content using loader (handles enhanced format)
        let parsing_start = Instant::now();
        let mut tasks = Vec::new();
        for (path, _content) in file_contents {
            let loader = &self.loader;
            let path_clone = path.clone();
            tasks.push(async move {
                let capability_file = loader.load_file(&path_clone).await?;
                Ok::<(PathBuf, CapabilityFile), ProxyError>((path_clone, capability_file))
            });
        }
        
        let parsed_files: Vec<(PathBuf, CapabilityFile)> = future::try_join_all(tasks).await?;
        let parsing_duration = parsing_start.elapsed();
        debug!("Phase 3 (Parsing): Parsed {} files in {:?}", parsed_files.len(), parsing_duration);

        // Phase 4: Validation - Validate capability files in parallel while preserving paths
        let validation_start = Instant::now();
        let validated_files: Vec<(PathBuf, CapabilityFile)> = parsed_files
            .par_iter()
            .map(|(path, capability_file)| {
                capability_file.validate()
                    .map_err(|e| ProxyError::registry(format!("Validation failed for {}: {}", path.display(), e)))?;
                Ok((path.clone(), capability_file.clone()))
            })
            .collect::<Result<Vec<_>>>()?;
        let validation_duration = validation_start.elapsed();
        debug!("Phase 4 (Validation): Validated {} files in {:?}", validated_files.len(), validation_duration);

        // Phase 5: Update - Build registry with actual file paths
        let update_start = Instant::now();
        let registry = self.build_registry_with_paths(validated_files, pipeline_start.elapsed())?;
        let update_duration = update_start.elapsed();
        debug!("Phase 5 (Update): Built registry in {:?}", update_duration);

        let total_duration = pipeline_start.elapsed();
        info!("Parallel pipeline completed in {:?} (Discovery: {:?}, Loading: {:?}, Parsing: {:?}, Validation: {:?}, Update: {:?})",
              total_duration, discovery_duration, loading_duration, parsing_duration, validation_duration, update_duration);

        Ok(registry)
    }
    
    /// Compile glob patterns for high-performance matching
    fn compile_glob_patterns(paths: &[String]) -> Result<Vec<GlobMatcher>> {
        let mut patterns = Vec::new();

        for path in paths {
            if path.contains('*') || path.contains('?') || path.contains('[') {
                let glob = Glob::new(path)
                    .map_err(|e| ProxyError::registry(format!("Invalid glob pattern '{}': {}", path, e)))?;
                patterns.push(glob.compile_matcher());
            }
        }

        Ok(patterns)
    }

    /// Discover all capability files using parallel glob matching
    async fn discover_capability_files(&self) -> Result<Vec<PathBuf>> {
        let start_time = Instant::now();
        let mut all_paths = Vec::new();

        debug!("Starting file discovery with {} paths", self.config.paths.len());

        // Process each configured path
        for path_str in &self.config.paths {
            debug!("Processing path: {}", path_str);
            let path = Path::new(path_str);

            if path.is_file() {
                // Single file
                debug!("Path is a file: {}", path_str);
                if self.is_yaml_file(path) {
                    all_paths.push(path.to_path_buf());
                }
            } else if path.is_dir() {
                // Directory - discover recursively
                debug!("Path is a directory: {}", path_str);
                let dir_paths = self.discover_directory_files(path).await?;
                all_paths.extend(dir_paths);
            } else if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
                // Glob pattern
                debug!("Path is a glob pattern: {}", path_str);
                let glob_paths = self.discover_glob_files(path_str).await?;
                debug!("Glob pattern {} found {} files", path_str, glob_paths.len());
                all_paths.extend(glob_paths);
            } else {
                warn!("Path does not exist: {}", path_str);
            }
        }

        let discovery_time = start_time.elapsed();
        debug!("File discovery completed in {}ms, found {} files", discovery_time.as_millis(), all_paths.len());

        Ok(all_paths)
    }

    /// Discover YAML files in a directory recursively
    async fn discover_directory_files(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        use walkdir::WalkDir;

        let paths: Vec<PathBuf> = WalkDir::new(dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.path().to_path_buf())
            .filter(|path| self.is_yaml_file(path))
            .collect();

        Ok(paths)
    }

    /// Discover files using compiled glob patterns for performance
    async fn discover_glob_files(&self, pattern: &str) -> Result<Vec<PathBuf>> {
        use glob::glob;

        debug!("Discovering files with glob pattern: {}", pattern);

        // Fallback to runtime glob (simplified for now)
        let paths: Vec<PathBuf> = glob(pattern)
            .map_err(|e| ProxyError::registry(format!("Glob pattern error: {}", e)))?
            .filter_map(|entry| {
                match entry {
                    Ok(path) => {
                        debug!("Glob found path: {}", path.display());
                        if self.is_yaml_file(&path) {
                            Some(path)
                        } else {
                            debug!("Path is not a YAML file: {}", path.display());
                            None
                        }
                    }
                    Err(e) => {
                        debug!("Glob error for entry: {}", e);
                        None
                    }
                }
            })
            .collect();

        debug!("Glob pattern {} matched {} YAML files", pattern, paths.len());
        Ok(paths)
    }

    /// Check if a file is a YAML file
    fn is_yaml_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            ext_str == "yaml" || ext_str == "yml"
        } else {
            false
        }
    }

    /// Load and parse a single capability file
    pub fn load_capability_file(&self, path: &Path) -> Result<CapabilityFile> {
        use std::fs;

        // Use the loader's load_file method which handles enhanced format
        let capability_file = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.loader.load_file(path))
        })?;

        // Update file modification time
        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                self.file_times.insert(path.to_path_buf(), modified);
            }
        }

        Ok(capability_file)
    }

    /// Build a new registry from capability files with their actual paths
    fn build_registry_with_paths(&self, capability_files: Vec<(PathBuf, CapabilityFile)>, load_duration: Duration) -> Result<CapabilityRegistry> {
        let mut files = HashMap::new();
        let mut tools = HashMap::new();

        for (file_path, file) in capability_files.into_iter() {
            let arc_file = Arc::new(file);

            // Index tools from this file
            for tool in &arc_file.tools {
                tools.insert(tool.name().to_string(), Arc::new(tool.clone()));
            }

            files.insert(file_path, arc_file);
        }

        let metadata = RegistryMetadata {
            file_count: files.len(),
            tool_count: tools.len(),
            last_updated: Instant::now(),
            load_duration_ms: load_duration.as_millis() as u64,
        };

        Ok(CapabilityRegistry {
            files,
            tools,
            metadata,
        })
    }

    /// Build a new registry from capability files (legacy method for backward compatibility)
    fn build_registry(&self, capability_files: Vec<CapabilityFile>, load_duration: Duration) -> Result<CapabilityRegistry> {
        // Convert to path-file pairs using incremental naming for backward compatibility
        let capability_files_with_paths: Vec<(PathBuf, CapabilityFile)> = capability_files
            .into_iter()
            .enumerate()
            .map(|(index, file)| (PathBuf::from(format!("legacy_file_{}", index)), file))
            .collect();
        
        self.build_registry_with_paths(capability_files_with_paths, load_duration)
    }

    /// Update the concurrent cache for fast lookups
    async fn update_cache(&self) {
        let registry = self.registry.load();

        // Clear old cache
        self.cache.clear();

        // Populate cache with most frequently accessed tools
        for (name, _tool_def) in &registry.tools {
            if let Some(file) = registry.files.values().find(|f| f.get_tool(name).is_some()) {
                self.cache.insert(name.clone(), file.clone());
            }
        }

        debug!("Cache updated with {} entries", self.cache.len());
    }

    /// Set up file system watcher for hot-reload
    async fn setup_file_watcher(&mut self) -> Result<()> {
        info!("🔧 DEBUG: setup_file_watcher() called - starting file watcher setup");
        let (tx, rx) = mpsc::unbounded_channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                match res {
                    Ok(event) => {
                        if let Err(e) = tx.send(event) {
                            error!("Failed to send file system event: {}", e);
                        }
                    }
                    Err(e) => error!("File system watch error: {}", e),
                }
            },
            Config::default(),
        ).map_err(|e| ProxyError::registry(format!("Failed to create file watcher: {}", e)))?;

        // Watch all configured paths
        for path_str in &self.config.paths {
            let path = Path::new(path_str);
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)
                    .map_err(|e| ProxyError::registry(format!("Failed to watch path {}: {}", path_str, e)))?;
                info!("Watching path for changes: {}", path_str);
            }
        }

        self._watcher = Some(watcher);
        self.event_rx = Some(rx);

        info!("File system watcher initialized");
        Ok(())
    }

    /// Start the hot-reload background task
    pub async fn start_hot_reload_task(mut self) -> Result<()> {
        if let Some(mut event_rx) = self.event_rx.take() {
            info!("Starting hot-reload background task");

            let mut debounce_timer: Option<tokio::time::Instant> = None;
            const DEBOUNCE_DURATION: Duration = Duration::from_millis(100);

            loop {
                tokio::select! {
                    // Handle file system events
                    event = event_rx.recv() => {
                        match event {
                            Some(event) => {
                                debug!("File system event: {:?}", event);

                                // Check if this affects our capability files
                                if self.should_reload_for_event(&event) {
                                    // Set debounce timer
                                    debounce_timer = Some(tokio::time::Instant::now() + DEBOUNCE_DURATION);
                                }
                            }
                            None => {
                                warn!("File system event channel closed");
                                break;
                            }
                        }
                    }

                    // Handle debounced reload
                    _ = sleep(Duration::from_millis(50)), if debounce_timer.is_some() => {
                        if let Some(timer) = debounce_timer {
                            if tokio::time::Instant::now() >= timer {
                                info!("Triggering hot-reload due to file changes");

                                if let Err(e) = self.reload_registry().await {
                                    error!("Hot-reload failed: {}", e);
                                } else {
                                    info!("Hot-reload completed successfully");
                                }

                                debounce_timer = None;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a file system event should trigger a reload
    fn should_reload_for_event(&self, event: &Event) -> bool {
        use notify::EventKind;

        // Only care about write/create/remove events
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                // Check if any affected paths are YAML files we care about
                event.paths.iter().any(|path| {
                    self.is_yaml_file(path) && self.is_watched_path(path)
                })
            }
            _ => false,
        }
    }

    /// Check if a path is within our watched directories
    fn is_watched_path(&self, path: &Path) -> bool {
        for config_path in &self.config.paths {
            let config_path = Path::new(config_path);
            if path.starts_with(config_path) || path == config_path {
                return true;
            }
        }
        false
    }

    /// Set notification manager for list_changed notifications
    pub fn set_notification_manager(&self, notification_manager: Arc<McpNotificationManager>) {
        if let Ok(mut manager) = self.notification_manager.write() {
            *manager = Some(notification_manager);
        }
    }

    /// Send tools list_changed notification if manager is available
    fn notify_tools_list_changed(&self) {
        if let Ok(manager_guard) = self.notification_manager.read() {
            if let Some(ref manager) = *manager_guard {
                if let Err(e) = manager.notify_tools_list_changed() {
                    warn!("Failed to send tools list_changed notification: {}", e);
                }
            }
        }
    }

    /// Set enhancement callback for tool change notifications
    pub fn set_enhancement_callback(&self, callback: Arc<dyn EnhancementCallback>) {
        if let Ok(mut cb) = self.enhancement_callback.write() {
            *cb = Some(callback);
            info!("Enhancement callback registered for tool change notifications");
        }
    }
    
    /// Set hierarchical configuration resolver for tool-level configuration precedence
    pub fn set_hierarchical_resolver(&self, resolver: Arc<HierarchicalResolver>) {
        if let Ok(mut hr) = self.hierarchical_resolver.write() {
            *hr = Some(resolver);
            info!("Hierarchical configuration resolver registered for tool-level precedence");
        }
    }
    
    /// Check if a tool should be hidden based on hierarchical configuration
    /// Falls back to tool's built-in hidden property if no hierarchical resolver is set
    pub fn is_tool_hidden_hierarchical(&self, tool_name: &str, tool_def: &ToolDefinition) -> bool {
        if let Ok(resolver_lock) = self.hierarchical_resolver.read() {
            if let Some(resolver) = resolver_lock.as_ref() {
                return resolver.resolve_hidden(Some(tool_name));
            }
        }
        tool_def.is_hidden()
    }
    
    /// Check if a tool should be enabled based on hierarchical configuration
    /// Falls back to tool's built-in enabled property if no hierarchical resolver is set
    pub fn is_tool_enabled_hierarchical(&self, tool_name: &str, tool_def: &ToolDefinition) -> bool {
        if let Ok(resolver_lock) = self.hierarchical_resolver.read() {
            if let Some(resolver) = resolver_lock.as_ref() {
                return resolver.resolve_enabled(Some(tool_name));
            }
        }
        tool_def.is_enabled()
    }
    
    /// Get resolved tool configuration based on hierarchical configuration
    /// Falls back to tool's built-in properties if no hierarchical resolver is set
    pub fn get_resolved_tool_config(&self, tool_name: &str, tool_def: &ToolDefinition) -> ResolvedToolConfig {
        if let Ok(resolver_lock) = self.hierarchical_resolver.read() {
            if let Some(resolver) = resolver_lock.as_ref() {
                return resolver.resolve_tool_config(tool_name);
            }
        }
        
        // Create a resolved config from the tool definition's built-in properties
        ResolvedToolConfig {
            name: tool_name.to_string(),
            timeout: 30,    // Default timeout (tool def doesn't have this field)
            max_retries: 3, // Default max retries (tool def doesn't have this)
            priority: 5,    // Default priority (tool def doesn't have this)
            enabled: tool_def.is_enabled(),
            hidden: tool_def.is_hidden(),
        }
    }

    /// Notify enhancement service of tool changes
    async fn notify_tools_changed(&self, changed_tools: Vec<(String, ToolDefinition)>) {
        if changed_tools.is_empty() {
            return;
        }

        // Clone the callback outside the lock to avoid holding the guard across await
        let callback = {
            if let Ok(callback_guard) = self.enhancement_callback.read() {
                callback_guard.clone()
            } else {
                None
            }
        };

        if let Some(callback) = callback {
            info!("🔄 Notifying enhancement service of {} changed tools", changed_tools.len());
            if let Err(e) = callback.on_tools_changed(changed_tools).await {
                error!("Enhancement callback failed: {}", e);
            }
        } else {
            debug!("No enhancement callback registered, skipping tool change notification");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::hierarchical::{HierarchicalConfig, ConfigResolver, GlobalConfig, McpConfig, DiscoveryConfig, ToolConfig, DefaultSettings};
    use std::collections::HashMap;
    use tempfile::TempDir;
    use std::fs;

    /// Create a mock registry service with test tool definitions
    async fn create_test_registry_service() -> Result<RegistryService> {
        let temp_dir = TempDir::new().unwrap();
        let capabilities_dir = temp_dir.path().join("capabilities");
        fs::create_dir_all(&capabilities_dir).unwrap();
        
        // Create a test capability file with multiple tools
        let test_capability = r#"
name: test_tools
description: Test tools for hierarchical configuration
tools:
  - name: "test_tool_1" 
    description: "Test tool 1"
    inputSchema:
      type: "object"
      properties:
        input:
          type: "string"
    routing:
      type: "command"
      config:
        command: "echo"
        args: []
    hidden: false
    enabled: true
  - name: "test_tool_2"
    description: "Test tool 2"
    inputSchema:
      type: "object"
      properties:
        input:
          type: "string"
    routing:
      type: "command"
      config:
        command: "echo"
        args: []
    hidden: true
    enabled: true
  - name: "test_tool_3"
    description: "Test tool 3"
    inputSchema:
      type: "object"
      properties:
        input:
          type: "string"
    routing:
      type: "command"
      config:
        command: "echo"
        args: []
    hidden: false
    enabled: false
"#;
        
        let capability_file = capabilities_dir.join("test_tools.yaml");
        fs::write(capability_file, test_capability).unwrap();
        
        let config = RegistryConfig {
            r#type: "file".to_string(),
            paths: vec![capabilities_dir.to_string_lossy().to_string()],
            hot_reload: false,
            validation: crate::config::ValidationConfig {
                strict: false,
                allow_unknown_fields: true,
            },
        };
        
        RegistryService::new(config).await
    }

    /// Create a test hierarchical configuration with tool-level overrides
    fn create_test_hierarchical_config() -> HierarchicalConfig {
        let mut tools = HashMap::new();
        
        // Override test_tool_1: make it hidden (overriding its default visible state)
        tools.insert("test_tool_1".to_string(), ToolConfig {
            overrides: Some(DefaultSettings {
                timeout: 15,
                retry_attempts: 1,
                retry_delay_ms: 500,
            }),
            routing: None,
            parameters: None,
            auth: None,
            timeout: Some(15),
            max_retries: Some(1), 
            priority: Some(8),
            enabled: Some(true),  // Keep enabled
            hidden: Some(true),   // Override to hidden
        });
        
        // Override test_tool_2: make it visible (overriding its default hidden state)
        tools.insert("test_tool_2".to_string(), ToolConfig {
            overrides: None,
            routing: None,
            parameters: None,
            auth: None,
            timeout: Some(25),
            max_retries: Some(4),
            priority: Some(2),
            enabled: Some(true),  // Keep enabled
            hidden: Some(false),  // Override to visible
        });
        
        // Override test_tool_3: enable it (overriding its default disabled state)  
        tools.insert("test_tool_3".to_string(), ToolConfig {
            overrides: None,
            routing: None,
            parameters: None,
            auth: None,
            timeout: Some(35),
            max_retries: Some(6),
            priority: Some(9),
            enabled: Some(true),  // Override to enabled
            hidden: Some(false),  // Keep visible
        });

        HierarchicalConfig {
            global: GlobalConfig {
                timeout: Some(30),    // Global timeout
                max_retries: Some(3), // Global max retries
                priority: Some(5),    // Global priority
                enabled: Some(true),  // Global enabled
                hidden: Some(true),   // Global hidden (default)
                ..Default::default()
            },
            mcp: McpConfig {
                timeout: Some(20),    // MCP timeout
                max_retries: Some(2), // MCP max retries  
                priority: Some(4),    // MCP priority
                enabled: Some(true),  // MCP enabled
                hidden: Some(false),  // MCP hidden
                ..Default::default()
            },
            discovery: DiscoveryConfig {
                smart_discovery: crate::discovery::SmartDiscoveryConfig::default(),
                tool_enhancement: crate::config::ToolEnhancementConfig::default(),
                enhancement_storage: crate::discovery::EnhancementStorageConfig::default(),
                visibility: crate::config::VisibilityConfig::default(),
                conflict_resolution: crate::routing::ConflictResolutionConfig::default(),
            },
            tools,
        }
    }

    #[tokio::test]
    async fn test_hierarchical_tool_filtering_precedence() {
        let mut registry = create_test_registry_service().await.unwrap();
        let hierarchical_config = create_test_hierarchical_config();
        let resolver = Arc::new(ConfigResolver::new(hierarchical_config));
        
        // Set the hierarchical resolver
        registry.set_hierarchical_resolver(resolver);
        
        // Test basic tool filtering with hierarchical config
        let visible_tools = registry.get_all_tools(); // Should get visible and enabled tools
        let hidden_tools = registry.get_hidden_tools();
        let enabled_tools = registry.get_enabled_tools();
        let discoverable_tools = registry.get_discoverable_tools(); // Hidden but enabled
        
        // Verify hierarchical precedence effects:
        // test_tool_1: originally visible, overridden to hidden (should be in hidden/discoverable)  
        // test_tool_2: originally hidden, overridden to visible (should be in visible)
        // test_tool_3: originally disabled, overridden to enabled and visible (should be in visible)
        
        // Check visible tools (enabled + not hidden)
        let visible_names: Vec<String> = visible_tools.iter().map(|(name, _)| name.clone()).collect();
        assert!(visible_names.contains(&"test_tool_2".to_string()), "test_tool_2 should be visible (overridden from hidden)");
        assert!(visible_names.contains(&"test_tool_3".to_string()), "test_tool_3 should be visible (overridden to enabled)");
        assert!(!visible_names.contains(&"test_tool_1".to_string()), "test_tool_1 should be hidden (overridden to hidden)");
        
        // Check hidden tools
        let hidden_names: Vec<String> = hidden_tools.iter().map(|(name, _)| name.clone()).collect();
        assert!(hidden_names.contains(&"test_tool_1".to_string()), "test_tool_1 should be hidden (overridden)");
        assert!(!hidden_names.contains(&"test_tool_2".to_string()), "test_tool_2 should not be hidden (overridden to visible)");
        assert!(!hidden_names.contains(&"test_tool_3".to_string()), "test_tool_3 should not be hidden (overridden to visible)");
        
        // Check enabled tools (regardless of visibility)
        let enabled_names: Vec<String> = enabled_tools.iter().map(|(name, _)| name.clone()).collect();
        assert!(enabled_names.contains(&"test_tool_1".to_string()), "test_tool_1 should be enabled");
        assert!(enabled_names.contains(&"test_tool_2".to_string()), "test_tool_2 should be enabled");
        assert!(enabled_names.contains(&"test_tool_3".to_string()), "test_tool_3 should be enabled (overridden)");
        
        // Check discoverable tools (hidden + enabled)  
        let discoverable_names: Vec<String> = discoverable_tools.iter().map(|(name, _)| name.clone()).collect();
        assert!(discoverable_names.contains(&"test_tool_1".to_string()), "test_tool_1 should be discoverable (hidden but enabled)");
        assert!(!discoverable_names.contains(&"test_tool_2".to_string()), "test_tool_2 should not be discoverable (visible)");
        assert!(!discoverable_names.contains(&"test_tool_3".to_string()), "test_tool_3 should not be discoverable (visible)");
        
        println!("✅ Hierarchical tool filtering precedence test passed");
    }

    #[tokio::test]
    async fn test_resolved_tool_configuration() {
        let mut registry = create_test_registry_service().await.unwrap();
        let hierarchical_config = create_test_hierarchical_config();
        let resolver = Arc::new(ConfigResolver::new(hierarchical_config));
        
        registry.set_hierarchical_resolver(resolver);
        
        // Get tool definitions to test resolved configs
        let all_tools = registry.get_all_tools_including_hidden();
        let tool_1 = all_tools.iter().find(|(name, _)| name == "test_tool_1").unwrap();
        let tool_2 = all_tools.iter().find(|(name, _)| name == "test_tool_2").unwrap();
        let tool_3 = all_tools.iter().find(|(name, _)| name == "test_tool_3").unwrap();
        
        // Test resolved configurations
        let config_1 = registry.get_resolved_tool_config("test_tool_1", &tool_1.1);
        assert_eq!(config_1.timeout, 15, "test_tool_1 should have tool-level timeout override");
        assert_eq!(config_1.max_retries, 1, "test_tool_1 should have tool-level max_retries override");
        assert_eq!(config_1.priority, 8, "test_tool_1 should have tool-level priority override");
        assert_eq!(config_1.hidden, true, "test_tool_1 should be hidden via tool-level override");
        assert_eq!(config_1.enabled, true, "test_tool_1 should be enabled");
        
        let config_2 = registry.get_resolved_tool_config("test_tool_2", &tool_2.1);
        assert_eq!(config_2.timeout, 25, "test_tool_2 should have tool-level timeout override");
        assert_eq!(config_2.max_retries, 4, "test_tool_2 should have tool-level max_retries override");
        assert_eq!(config_2.priority, 2, "test_tool_2 should have tool-level priority override");
        assert_eq!(config_2.hidden, false, "test_tool_2 should be visible via tool-level override");
        assert_eq!(config_2.enabled, true, "test_tool_2 should be enabled");
        
        let config_3 = registry.get_resolved_tool_config("test_tool_3", &tool_3.1);
        assert_eq!(config_3.timeout, 35, "test_tool_3 should have tool-level timeout override");
        assert_eq!(config_3.max_retries, 6, "test_tool_3 should have tool-level max_retries override"); 
        assert_eq!(config_3.priority, 9, "test_tool_3 should have tool-level priority override");
        assert_eq!(config_3.hidden, false, "test_tool_3 should be visible via tool-level override");
        assert_eq!(config_3.enabled, true, "test_tool_3 should be enabled via tool-level override");
        
        println!("✅ Resolved tool configuration test passed");
    }

    #[tokio::test]
    async fn test_hierarchical_fallback_to_built_in_properties() {
        let mut registry = create_test_registry_service().await.unwrap();
        
        // Test without hierarchical resolver - should fall back to tool's built-in properties
        let all_tools = registry.get_all_tools_including_hidden();
        let tool_1 = all_tools.iter().find(|(name, _)| name == "test_tool_1").unwrap();
        
        // Should use built-in properties when no hierarchical resolver
        let config_fallback = registry.get_resolved_tool_config("test_tool_1", &tool_1.1);
        assert_eq!(config_fallback.timeout, 30, "Should use default timeout when no hierarchical resolver");
        assert_eq!(config_fallback.max_retries, 3, "Should use default max_retries when no hierarchical resolver");
        assert_eq!(config_fallback.priority, 5, "Should use default priority when no hierarchical resolver");
        assert_eq!(config_fallback.hidden, false, "Should use tool's built-in hidden property (false)");
        assert_eq!(config_fallback.enabled, true, "Should use tool's built-in enabled property (true)");
        
        println!("✅ Hierarchical fallback test passed");
    }

    #[tokio::test]
    async fn test_tool_configuration_isolation() {
        let mut registry = create_test_registry_service().await.unwrap();
        let hierarchical_config = create_test_hierarchical_config();
        let resolver = Arc::new(ConfigResolver::new(hierarchical_config));
        
        registry.set_hierarchical_resolver(resolver);
        
        // Test that tool-specific configurations don't affect other tools
        let all_tools = registry.get_all_tools_including_hidden();
        let tool_1 = all_tools.iter().find(|(name, _)| name == "test_tool_1").unwrap();
        
        // test_tool_1 has specific overrides
        assert_eq!(registry.is_tool_hidden_hierarchical("test_tool_1", &tool_1.1), true);
        assert_eq!(registry.is_tool_enabled_hierarchical("test_tool_1", &tool_1.1), true);
        
        // Create a tool with no overrides - should fall back to MCP/Global levels
        assert_eq!(registry.is_tool_hidden_hierarchical("nonexistent_tool", &tool_1.1), false); // MCP level: hidden = false
        assert_eq!(registry.is_tool_enabled_hierarchical("nonexistent_tool", &tool_1.1), true);  // MCP level: enabled = true
        
        println!("✅ Tool configuration isolation test passed");
    }
}
