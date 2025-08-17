//! Tool Enhancement Service
//!
//! Implements the critical tool enhancement pipeline: base → sampling → elicitation → ranking
//! This service fixes the architectural issue where ranking was happening on base descriptions
//! instead of enhanced descriptions, significantly improving smart discovery performance.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// Analysis results for tool enhancement filtering
#[derive(Debug, Clone)]
pub struct ToolEnhancementAnalysis {
    pub total_tools: usize,
    pub regular_tools_count: usize,
    pub external_tools_count: usize,
    pub disabled_tools_count: usize,
    pub generator_enhanced_count: usize,
    pub already_enhanced: usize,
    pub needs_enhancement: usize,
}
use futures_util::future;
use sha2::{Sha256, Digest};

use crate::config::Config;
use crate::discovery::types::{
    EnhancedToolDefinition, 
    ElicitationMetadata, 
    EnhancementSource, 
    EnhancementGenerationMetadata
};
use crate::registry::types::ToolDefinition;
use crate::registry::RegistryService;
use crate::registry::service::EnhancementCallback;
use crate::mcp::tool_enhancement::ToolEnhancementService;
use crate::mcp::elicitation::ElicitationService;
use crate::mcp::request_generator::RequestGeneratorService;
use crate::discovery::enhancement_storage::EnhancementStorageService;
use crate::error::{Result, ProxyError};

/// Configuration for tool enhancement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEnhancementConfig {
    /// Enable description enhancement for tool descriptions (using LLM to improve base descriptions)
    pub enable_description_enhancement: bool,
    
    /// Enable tool enhancement for tool metadata, descriptions, and keywords
    pub enable_tool_enhancement: bool,
    
    /// Whether enhancements require human approval
    pub require_approval: bool,
    
    /// Cache enhanced tools to avoid regeneration
    pub cache_enhancements: bool,
    
    /// Timeout for enhancement operations in seconds
    pub enhancement_timeout_seconds: u64,
    
    /// Batch size for processing multiple tools
    pub batch_size: usize,
    
    /// Whether to fallback to base descriptions on enhancement failure
    pub graceful_degradation: bool,
}

impl Default for ToolEnhancementConfig {
    fn default() -> Self {
        Self {
            enable_description_enhancement: true,
            enable_tool_enhancement: true,
            require_approval: false, // Default to auto-approval for development
            cache_enhancements: true,
            enhancement_timeout_seconds: 30,
            batch_size: 10,
            graceful_degradation: true,
        }
    }
}

/// Service for enhancing tools with sampling and elicitation capabilities
pub struct ToolEnhancementPipeline {
    /// Configuration for enhancement behavior
    config: ToolEnhancementConfig,
    
    /// Registry service for accessing base tools
    registry: Arc<RegistryService>,
    
    /// Tool Enhancement service for description enhancement 
    tool_enhancement_service: Option<Arc<crate::mcp::tool_enhancement::ToolEnhancementService>>,
    
    /// Elicitation service for metadata enhancement
    elicitation_service: Option<Arc<ElicitationService>>,
    
    /// Request generator service for server-side generation
    request_generator: Option<Arc<RequestGeneratorService>>,
    
    /// Cache of enhanced tool definitions
    enhanced_cache: Arc<RwLock<HashMap<String, EnhancedToolDefinition>>>,
    
    /// Cache of enhancement failures (to avoid retrying repeatedly)
    failure_cache: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
    
    /// Optional persistent storage service for enhanced tool descriptions
    storage_service: Option<Arc<EnhancementStorageService>>,
    
    /// Elicitation configuration for authority management
    elicitation_config: Option<crate::config::ElicitationConfig>,
    
    /// Whether smart discovery is enabled (affects elicitation tool discovery behavior)
    smart_discovery_enabled: bool,
}

impl ToolEnhancementPipeline {
    /// Create a new tool enhancement service
    pub fn new(
        config: ToolEnhancementConfig,
        registry: Arc<RegistryService>,
        tool_enhancement_service: Option<Arc<crate::mcp::tool_enhancement::ToolEnhancementService>>,
        elicitation_service: Option<Arc<ElicitationService>>,
    ) -> Self {
        Self::new_with_storage(config, registry, tool_enhancement_service, elicitation_service, None, None, false)
    }
    
    /// Create a new tool enhancement service with persistent storage
    pub fn new_with_storage(
        config: ToolEnhancementConfig,
        registry: Arc<RegistryService>,
        tool_enhancement_service: Option<Arc<ToolEnhancementService>>,
        elicitation_service: Option<Arc<ElicitationService>>,
        storage_service: Option<Arc<EnhancementStorageService>>,
        elicitation_config: Option<crate::config::ElicitationConfig>,
        smart_discovery_enabled: bool,
    ) -> Self {
        let request_generator = if tool_enhancement_service.is_some() && elicitation_service.is_some() {
            Some(Arc::new(RequestGeneratorService::new(
                crate::mcp::request_generator::RequestGeneratorConfig::default(),
                tool_enhancement_service.as_ref().unwrap().clone(),
                elicitation_service.as_ref().unwrap().clone(),
            )))
        } else {
            None
        };
        info!("Initializing tool enhancement service");
        info!("  - Description enhancement: {}", config.enable_description_enhancement);
        info!("  - Tool enhancement: {}", config.enable_tool_enhancement);
        info!("  - Require approval: {}", config.require_approval);
        info!("  - Cache enhancements: {}", config.cache_enhancements);
        
        Self {
            config,
            registry,
            tool_enhancement_service,
            elicitation_service,
            request_generator,
            enhanced_cache: Arc::new(RwLock::new(HashMap::new())),
            failure_cache: Arc::new(RwLock::new(HashMap::new())),
            storage_service,
            elicitation_config,
            smart_discovery_enabled,
        }
    }
    
    /// Create from main config
    pub fn from_config(
        config: &Config,
        registry: Arc<RegistryService>,
        tool_enhancement_service: Option<Arc<crate::mcp::tool_enhancement::ToolEnhancementService>>,
        elicitation_service: Option<Arc<ElicitationService>>,
    ) -> Self {
        // Check if tool enhancement is enabled and create config accordingly
        let description_enhancement_enabled = config.sampling.as_ref().map(|s| s.enabled).unwrap_or(false);
        let tool_enhancement_enabled = config.tool_enhancement
            .as_ref()
            .map(|te| te.enabled)
            .unwrap_or(false);
            
        let enhancement_config = if tool_enhancement_enabled {
            ToolEnhancementConfig {
                enable_description_enhancement: description_enhancement_enabled, // Use global sampling config
                enable_tool_enhancement: tool_enhancement_enabled,
                ..ToolEnhancementConfig::default()
            }
        } else {
            ToolEnhancementConfig::default()
        };
        
        // Create enhancement storage service from config if present
        let storage_service = if let Some(storage_config) = &config.enhancement_storage {
            debug!("🔧 Creating enhancement storage service from config");
            match EnhancementStorageService::new(storage_config.clone()) {
                Ok(service) => {
                    info!("✅ Enhancement storage service created successfully");
                    Some(Arc::new(service))
                }
                Err(e) => {
                    warn!("Failed to create enhancement storage service: {}. Enhancement storage disabled.", e);
                    None
                }
            }
        } else {
            debug!("🔧 No enhancement storage config found - storage disabled");
            None
        };
        
        let elicitation_config = config.elicitation.clone();
        let smart_discovery_enabled = config.smart_discovery.as_ref().map(|sd| sd.enabled).unwrap_or(false);
        Self::new_with_storage(enhancement_config, registry, tool_enhancement_service, elicitation_service, storage_service, elicitation_config, smart_discovery_enabled)
    }

    /// Create from main config with injected storage service (for proper dependency injection)
    pub fn from_config_with_storage(
        config: &Config,
        registry: Arc<RegistryService>,
        tool_enhancement_service: Option<Arc<crate::mcp::tool_enhancement::ToolEnhancementService>>,
        elicitation_service: Option<Arc<ElicitationService>>,
        storage_service: Option<Arc<EnhancementStorageService>>,
    ) -> Self {
        // Check if tool enhancement is enabled and create config accordingly
        let description_enhancement_enabled = config.sampling.as_ref().map(|s| s.enabled).unwrap_or(false);
        let tool_enhancement_enabled = config.tool_enhancement
            .as_ref()
            .map(|te| te.enabled)
            .unwrap_or(false);
            
        let enhancement_config = if tool_enhancement_enabled {
            ToolEnhancementConfig {
                enable_description_enhancement: description_enhancement_enabled, // Use global sampling config
                enable_tool_enhancement: tool_enhancement_enabled,
                ..ToolEnhancementConfig::default()
            }
        } else {
            ToolEnhancementConfig::default()
        };
        
        let elicitation_config = config.elicitation.clone();
        let smart_discovery_enabled = config.smart_discovery.as_ref().map(|sd| sd.enabled).unwrap_or(false);
        
        // Use injected storage if provided, otherwise create from config (for backward compatibility)
        let final_storage_service = if let Some(injected_storage) = storage_service {
            info!("Using injected enhancement storage service");
            Some(injected_storage)
        } else if let Some(storage_config) = &config.enhancement_storage {
            debug!("🔧 Creating enhancement storage service from config (fallback for backward compatibility)");
            match EnhancementStorageService::new(storage_config.clone()) {
                Ok(service) => {
                    info!("✅ Enhancement storage service created successfully (backward compatibility)");
                    Some(Arc::new(service))
                }
                Err(e) => {
                    warn!("Failed to create enhancement storage service: {}. Enhancement storage disabled.", e);
                    None
                }
            }
        } else {
            debug!("🔧 No enhancement storage config found - storage disabled");
            None
        };
        
        Self::new_with_storage(enhancement_config, registry, tool_enhancement_service, elicitation_service, final_storage_service, elicitation_config, smart_discovery_enabled)
    }
    
    /// Check if a tool should be excluded from enhancement processing
    /// Only excludes disabled tools - all enabled tools (including system tools) can be enhanced
    fn should_exclude_tool(tool_def: &ToolDefinition) -> bool {
        !tool_def.enabled
    }
    
    /// Check if a tool is from an external MCP server (using routing type)
    fn is_external_mcp_tool(tool_def: &ToolDefinition) -> bool {
        matches!(tool_def.routing.r#type.as_str(), "external_mcp" | "websocket")
    }
    
    /// Check if a tool should be excluded from enhancement
    fn should_exclude_from_enhancement(tool_def: &ToolDefinition) -> bool {
        // Exclude smart discovery system tools (identified by routing type)
        let is_smart_discovery = tool_def.routing.r#type == "smart_discovery";
        
        debug!("🔍 Enhancement exclusion check for '{}': routing='{}', is_smart_discovery={}", 
               tool_def.name, tool_def.routing.r#type, is_smart_discovery);
        
        is_smart_discovery
    }

    /// Central method to check if tools need enhancement
    /// This is used by both pre-generation and startup flows to ensure consistent logic
    pub async fn analyze_tools_for_enhancement(
        all_tools: &HashMap<String, ToolDefinition>,
        enhancement_storage: &Option<Arc<EnhancementStorageService>>
    ) -> Result<(Vec<(String, ToolDefinition)>, ToolEnhancementAnalysis)> {
        let mut regular_tools = Vec::new();
        let mut external_tools_count = 0;
        let mut disabled_tools_count = 0;
        let mut generator_enhanced_count = 0;

        // Filter tools using centralized logic
        for (tool_name, tool_def) in all_tools {
            // Skip disabled tools
            if Self::should_exclude_tool(tool_def) {
                disabled_tools_count += 1;
                continue;
            }
            
            // Skip external MCP tools - they should get enhancements from their source
            if Self::is_external_mcp_tool(tool_def) {
                external_tools_count += 1;
                continue;
            }
            
            // Skip smart discovery system tool only
            if Self::should_exclude_from_enhancement(tool_def) {
                generator_enhanced_count += 1;
                continue;
            }
            
            // This is a regular tool that can be enhanced
            regular_tools.push((tool_name.clone(), tool_def.clone()));
        }

        // Check which regular tools are missing enhancements
        let mut tools_needing_enhancement = 0;
        let mut already_enhanced = 0;
        
        if let Some(storage) = enhancement_storage {
            debug!("✅ Enhancement storage service is available, checking {} regular tools", regular_tools.len());
            for (tool_name, _tool_def) in &regular_tools {
                debug!("🔍 Checking if tool '{}' has existing enhancement", tool_name);
                // Check if this tool has existing enhancements
                let has_enhancement = storage.load_enhanced_tool(tool_name).await
                    .map(|enhancement| enhancement.is_some())
                    .unwrap_or(false);
                if !has_enhancement {
                    debug!("❌ Tool '{}' needs enhancement", tool_name);
                    tools_needing_enhancement += 1;
                } else {
                    debug!("✅ Tool '{}' already enhanced", tool_name);
                    already_enhanced += 1;
                }
            }
        } else {
            debug!("❌ Enhancement storage service is None - cannot check for existing enhancements");
            // No storage means we can't track what's enhanced, so enhance all regular tools
            tools_needing_enhancement = regular_tools.len();
        }

        let analysis = ToolEnhancementAnalysis {
            total_tools: all_tools.len(),
            regular_tools_count: regular_tools.len(),
            external_tools_count,
            disabled_tools_count,
            generator_enhanced_count,
            already_enhanced,
            needs_enhancement: tools_needing_enhancement,
        };

        Ok((regular_tools, analysis))
    }
    
    
    /// Check if tool enhancement (keyword generation) should be used for this tool
    pub fn should_use_tool_enhancement(&self, tool: &ToolDefinition) -> bool {
        // If tool enhancement is disabled, never use tool enhancement
        if !self.config.enable_tool_enhancement {
            return false;
        }
        
        // Tool enhancement should run regardless of smart discovery state
        // It enhances tools for better semantic search even when tools are hidden behind smart discovery
        
        // Only run on enabled tools
        if !tool.enabled {
            debug!("Skipping tool enhancement for '{}' - tool is disabled", tool.name);
            return false;
        }
        
        // For non-external tools, use tool enhancement if enabled
        if !Self::is_external_mcp_tool(tool) {
            debug!("Using tool enhancement for '{}' - local tool with enhancement enabled", tool.name);
            return true;
        }
        
        // For external tools, tool enhancement can always run since it's just keyword generation
        // This is different from actual elicitation which needs authority checks
        debug!("Using tool enhancement for external tool '{}' - keyword generation is safe", tool.name);
        return true
    }
    
    
    
    /// Fast initialization - just set up service structure without heavy I/O
    pub async fn initialize(&self) -> Result<()> {
        info!("🚀 Fast initialization of tool enhancement service (deferred analysis)");
        
        // Just do basic setup without heavy analysis
        info!("✅ Tool enhancement service initialized - background analysis will run separately");
        Ok(())
    }
    
    /// Full initialization with storage loading and tool analysis (runs in background)
    pub async fn initialize_with_analysis(&self) -> Result<()> {
        info!("🔍 Starting background tool enhancement analysis and initialization");
        
        // Load enhanced tools from persistent storage if available
        if let Some(storage) = &self.storage_service {
            info!("📂 Enhancement storage service is available, attempting to load...");
            match storage.load_all_enhanced_tools().await {
                Ok(stored_tools) => {
                    let mut cache = self.enhanced_cache.write().await;
                    info!("📋 Before loading: cache has {} items", cache.len());
                    info!("📋 Storage returned {} tools", stored_tools.len());
                    
                    // Debug: log first few tool names and their enhancement status
                    for (i, (name, tool)) in stored_tools.iter().enumerate() {
                        if i < 5 {
                            info!("  📝 Tool '{}': is_enhanced={}, source={:?}", name, tool.is_enhanced(), tool.enhancement_source);
                        }
                    }
                    
                    cache.extend(stored_tools);
                    info!("📦 Loaded {} enhanced tools from persistent storage, cache now has {} items", cache.len(), cache.len());
                }
                Err(e) => {
                    warn!("Failed to load enhanced tools from storage: {}", e);
                }
            }
        } else {
            warn!("📂 No enhancement storage service available - cannot load stored tools");
        }
        
        // Get all enabled tools from registry
        let all_tools = self.registry.get_enabled_tools();
        let total_tools = all_tools.len();
        
        if total_tools == 0 {
            info!("No tools found in registry, skipping enhancement generation");
            return Ok(());
        }
        
        info!("📊 Found {} tools in registry, analyzing for missing enhancements", total_tools);
        
        // Use centralized logic to determine if enhancement is actually needed
        let tools_map: HashMap<String, ToolDefinition> = all_tools.iter().cloned().collect();
        let (regular_tools, analysis) = Self::analyze_tools_for_enhancement(&tools_map, &self.storage_service).await?;
        
        info!("🔍 Enhancement analysis completed:");
        info!("  - Total tools: {}", analysis.total_tools);
        info!("  - Regular tools (can be enhanced): {}", analysis.regular_tools_count);
        info!("  - External MCP tools (skip): {}", analysis.external_tools_count);
        info!("  - Disabled tools (skip): {}", analysis.disabled_tools_count);
        info!("  - Generator-enhanced tools (skip): {}", analysis.generator_enhanced_count);
        info!("  - Already enhanced: {}", analysis.already_enhanced);
        info!("  - Need enhancement: {}", analysis.needs_enhancement);
        
        if analysis.needs_enhancement == 0 {
            info!("✅ All tools are already enhanced - no enhancement generation needed");
            return Ok(());
        }
        
        info!("⚡ {} tools need enhancement - proceeding with enhancement generation", analysis.needs_enhancement);
        
        // Use the pre-filtered regular tools from centralized analysis
        // regular_tools from centralized analysis already contains ONLY tools that need enhancement
        let tools_needing_enhancement = regular_tools.clone();
        
        // Log which tools will be enhanced
        for (tool_name, _tool_def) in &regular_tools {
            debug!("Tool '{}' needs enhancement (verified by centralized analysis)", tool_name);
        }
        
        let needing_count = tools_needing_enhancement.len();
        
        if needing_count == 0 {
            info!("✅ All tools already have enhancements, initialization complete");
            return Ok(());
        }
        
        info!("🔄 Generating enhancements for {} tools at startup", needing_count);
        let start_time = std::time::Instant::now();
        
        // Process tools in batches or individually based on configuration
        if self.config.batch_size > 1 && needing_count > 1 {
            self.process_tools_in_batches(tools_needing_enhancement).await?;
        } else {
            let mut success_count = 0;
            let mut failure_count = 0;
            
            for (tool_name, tool_def) in tools_needing_enhancement {
                match self.generate_and_store_enhancement(&tool_name, &tool_def).await {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        failure_count += 1;
                        warn!("Failed to generate initial enhancement for tool '{}': {}", tool_name, e);
                    }
                }
            }
            
            let duration = start_time.elapsed();
            info!("✅ Initial enhancement generation completed in {:.2}s", duration.as_secs_f64());
            info!("  - Success: {}", success_count);
            info!("  - Failed: {}", failure_count);
            
            if failure_count > 0 {
                warn!("⚠️  {} tools failed initial enhancement generation", failure_count);
            }
        }
        
        Ok(())
    }
    
    /// Process tools that were added/updated during registry reload
    pub async fn on_tools_changed(&self, changed_tools: Vec<(String, ToolDefinition)>) -> Result<()> {
        if changed_tools.is_empty() {
            debug!("No tools changed, skipping enhancement generation");
            return Ok(());
        }

        info!("🔄 Processing {} changed tools for enhancement generation", changed_tools.len());
        
        // Filter out tools that already have up-to-date enhancements
        let tools_needing_enhancement = self.filter_tools_needing_enhancement(&changed_tools).await;
        
        if tools_needing_enhancement.is_empty() {
            debug!("All tools already have up-to-date enhancements");
            return Ok(());
        }

        info!("📝 Generating enhancements for {} tools", tools_needing_enhancement.len());
        
        if self.config.batch_size > 1 && tools_needing_enhancement.len() > 1 {
            // Process in batches for better performance
            self.process_tools_in_batches(tools_needing_enhancement).await
        } else {
            // Process individually
            for (tool_name, tool_def) in tools_needing_enhancement {
                if let Err(e) = self.generate_and_store_enhancement(&tool_name, &tool_def).await {
                    warn!("Failed to generate enhancement for tool '{}': {}", tool_name, e);
                }
            }
            Ok(())
        }
    }

    /// Filter tools that need enhancement generation (new or changed)
    /// Only includes regular tools that should be enhanced (excludes external MCP tools and disabled tools)
    async fn filter_tools_needing_enhancement(&self, tools: &[(String, ToolDefinition)]) -> Vec<(String, ToolDefinition)> {
        let mut needs_enhancement = Vec::new();
        
        for (tool_name, tool_def) in tools {
            // Skip disabled tools
            if Self::should_exclude_tool(tool_def) {
                debug!("Tool '{}' is excluded from enhancement (disabled tool)", tool_name);
                continue;
            }
            
            // Skip external/remote MCP tools - they should get enhancements from their source MCP servers
            if Self::is_external_mcp_tool(tool_def) {
                debug!("Tool '{}' is from external MCP server, skipping automatic enhancement generation", tool_name);
                continue;
            }
            
            // Check if we have a cached enhancement
            let cache = self.enhanced_cache.read().await;
            if let Some(cached_tool) = cache.get(tool_name) {
                // Check if the base tool has changed (simple version comparison)
                if self.has_tool_changed(tool_def, &cached_tool.base) {
                    debug!("Tool '{}' has changed, needs re-enhancement", tool_name);
                    needs_enhancement.push((tool_name.clone(), tool_def.clone()));
                } else {
                    debug!("Tool '{}' unchanged, using cached enhancement", tool_name);
                }
            } else {
                debug!("Tool '{}' has no cached enhancement, needs generation", tool_name);
                needs_enhancement.push((tool_name.clone(), tool_def.clone()));
            }
        }
        
        needs_enhancement
    }

    /// Check if a tool definition has changed compared to cached version
    fn has_tool_changed(&self, current: &ToolDefinition, cached: &ToolDefinition) -> bool {
        // Simple change detection - compare description and schema
        current.description != cached.description || 
        serde_json::to_string(&current.input_schema).unwrap_or_default() != 
        serde_json::to_string(&cached.input_schema).unwrap_or_default()
    }

    /// Process tools in batches for better performance
    async fn process_tools_in_batches(&self, tools: Vec<(String, ToolDefinition)>) -> Result<()> {
        let chunks: Vec<_> = tools.chunks(self.config.batch_size).collect();
        
        for (batch_idx, batch) in chunks.iter().enumerate() {
            info!("📦 Processing batch {}/{} ({} tools)", batch_idx + 1, chunks.len(), batch.len());
            
            // Process batch concurrently
            let batch_tasks: Vec<_> = batch.iter().map(|(tool_name, tool_def)| {
                let tool_name = tool_name.clone();
                let tool_def = tool_def.clone();
                async move {
                    self.generate_and_store_enhancement(&tool_name, &tool_def).await
                }
            }).collect();
            
            let results = future::join_all(batch_tasks).await;
            
            // Log results
            let mut success_count = 0;
            let mut failure_count = 0;
            for (idx, result) in results.iter().enumerate() {
                match result {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        failure_count += 1;
                        warn!("Failed to enhance tool '{}': {}", batch[idx].0, e);
                    }
                }
            }
            
            info!("✅ Batch {}/{} completed: {} success, {} failures", 
                  batch_idx + 1, chunks.len(), success_count, failure_count);
        }
        
        Ok(())
    }

    /// Generate and store enhancement for a single tool
    async fn generate_and_store_enhancement(&self, tool_name: &str, tool_def: &ToolDefinition) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        match self.enhance_single_tool(tool_name, tool_def).await {
            Ok(mut enhanced_tool) => {
                // Set the last generation timestamp
                enhanced_tool.last_generated_at = Some(chrono::Utc::now());
                
                // Store in cache
                if self.config.cache_enhancements {
                    let mut cache = self.enhanced_cache.write().await;
                    cache.insert(tool_name.to_string(), enhanced_tool.clone());
                }
                
                // Store in persistent storage if available
                if let Some(storage) = &self.storage_service {
                    let base_tool_hash = Self::calculate_tool_hash(tool_def);
                    if let Err(e) = storage.store_enhanced_tool(tool_name, enhanced_tool, base_tool_hash).await {
                        warn!("Failed to store enhanced tool '{}' to persistent storage: {}", tool_name, e);
                    } else {
                        debug!("Stored enhanced tool '{}' to persistent storage", tool_name);
                    }
                }
                
                let duration = start_time.elapsed();
                info!("✅ Enhanced tool '{}' in {}ms", tool_name, duration.as_millis());
                Ok(())
            }
            Err(e) => {
                // Store in failure cache to avoid retrying immediately
                let mut failure_cache = self.failure_cache.write().await;
                failure_cache.insert(tool_name.to_string(), chrono::Utc::now());
                
                Err(e)
            }
        }
    }

    /// Get all enhanced tool definitions for use in discovery/ranking
    /// This method now uses pre-generated enhancements only (no on-demand generation)
    pub async fn get_enhanced_tools(&self) -> Result<HashMap<String, EnhancedToolDefinition>> {
        let start_time = Instant::now();
        debug!("📖 Loading pre-generated enhanced tools for smart discovery");
        
        // Get all enabled tools from registry
        let base_tools = self.registry.get_enabled_tools();
        let mut enhanced_tools = HashMap::new();
        let mut from_cache = 0;
        let mut fallback_to_base = 0;
        
        let cache = self.enhanced_cache.read().await;
        
        for (tool_name, tool_def) in base_tools {
            // All enabled tools can be enhanced (including smart_discovery_tool)
            // Skip disabled tools only
            
            // Try to get pre-generated enhancement from cache
            if let Some(enhanced_tool) = cache.get(&tool_name) {
                enhanced_tools.insert(tool_name.clone(), enhanced_tool.clone());
                from_cache += 1;
            } else {
                // Fallback to base tool definition (expected for external MCP tools)
                debug!("No pre-generated enhancement found for tool '{}', using base definition", tool_name);
                enhanced_tools.insert(tool_name.clone(), EnhancedToolDefinition::from_base(tool_def.clone()));
                fallback_to_base += 1;
            }
        }
        
        let duration = start_time.elapsed();
        
        // Only log detailed stats once per process, not on every call
        static LOGGED_ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LOGGED_ONCE.swap(true, std::sync::atomic::Ordering::Relaxed) {
            info!("📖 Loaded {} enhanced tools in {}ms", enhanced_tools.len(), duration.as_millis());
            info!("  - From pre-generated cache: {}", from_cache);
            info!("  - Fallback to base: {}", fallback_to_base);
            
            if fallback_to_base > 0 {
                warn!("⚠️  {} tools missing pre-generated enhancements. Consider running regeneration.", fallback_to_base);
            }
        } else {
            // Subsequent calls only get debug logging
            debug!("📖 Loaded {} enhanced tools in {}ms (cached: {}, fallback: {})", 
                   enhanced_tools.len(), duration.as_millis(), from_cache, fallback_to_base);
        }
        
        Ok(enhanced_tools)
    }
    
    /// Calculate a hash of the base tool definition for change detection
    fn calculate_tool_hash(tool_def: &ToolDefinition) -> String {
        let mut hasher = Sha256::new();
        // Hash the critical parts of the tool definition that would affect enhancement
        hasher.update(tool_def.name.as_bytes());
        hasher.update(tool_def.description.as_bytes());
        if let Ok(schema_json) = serde_json::to_string(&tool_def.input_schema) {
            hasher.update(schema_json.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }
    
    /// Check if enhancement should be regenerated based on tool changes
    async fn should_regenerate_enhancement(&self, tool_name: &str, tool_def: &ToolDefinition) -> bool {
        // Check persistent storage first if available
        if let Some(storage) = &self.storage_service {
            let base_tool_hash = Self::calculate_tool_hash(tool_def);
            match storage.is_enhancement_current(tool_name, &base_tool_hash).await {
                Ok(is_current) => return !is_current,
                Err(e) => {
                    debug!("Failed to check enhancement currency for '{}': {}", tool_name, e);
                }
            }
        }
        
        // Fallback to cache check
        let cache = self.enhanced_cache.read().await;
        !cache.contains_key(tool_name)
    }
    
    /// Enhance a single tool through the sampling → elicitation pipeline
    async fn enhance_single_tool(&self, tool_name: &str, tool_def: &ToolDefinition) -> Result<EnhancedToolDefinition> {
        // Check cache first if enabled
        if self.config.cache_enhancements {
            let cache = self.enhanced_cache.read().await;
            if let Some(cached_tool) = cache.get(tool_name) {
                debug!("Using cached enhancement for tool: {}", tool_name);
                return Ok(cached_tool.clone());
            }
        }
        
        // Check failure cache to avoid repeatedly trying failed enhancements
        if let Some(failure_time) = self.failure_cache.read().await.get(tool_name) {
            let age = chrono::Utc::now().signed_duration_since(*failure_time);
            if age.num_minutes() < 60 { // Don't retry for 1 hour
                debug!("Skipping recently failed enhancement for tool: {}", tool_name);
                return Ok(EnhancedToolDefinition::from_base(tool_def.clone()));
            }
        }
        
        let start_time = Instant::now();
        let mut enhanced_tool = EnhancedToolDefinition::from_base(tool_def.clone());
        let mut generation_metadata = EnhancementGenerationMetadata {
            llm_model: None,
            llm_confidence: None,
            elicitation_template: None,
            required_review: self.config.require_approval,
            approved_by: None,
            approved_at: None,
            generation_time_ms: None,
        };
        
        // Step 1: Description enhancement (better descriptions using LLM)
        if self.config.enable_description_enhancement {
            if let Some(request_generator) = &self.request_generator {
                match request_generator.generate_enhanced_description(&tool_name, &tool_def).await {
                    Ok(result) => {
                        if result.success {
                            if let Some(enhanced_description) = result.content {
                                enhanced_tool.llm_enhanced_description = Some(enhanced_description);
                                generation_metadata.llm_model = result.metadata.model_used;
                                generation_metadata.llm_confidence = result.metadata.confidence_score;
                                debug!("✅ Description enhancement completed for tool: {} ({}ms)", tool_name, result.metadata.generation_time_ms);
                            }
                        } else {
                            let error_msg = result.error.unwrap_or("Unknown error".to_string());
                            warn!("❌ Description enhancement failed for tool '{}': {}", tool_name, error_msg);
                            if !self.config.graceful_degradation {
                                return Err(ProxyError::validation(format!("Description enhancement failed: {}", error_msg)));
                            }
                        }
                    }
                    Err(e) => {
                        warn!("❌ Request generator failed for tool '{}': {}", tool_name, e);
                        if !self.config.graceful_degradation {
                            return Err(e);
                        }
                    }
                }
            }
        }
        
        // Step 2: Tool enhancement (keyword generation and metadata)
        if self.should_use_tool_enhancement(tool_def) {
            if let Some(request_generator) = &self.request_generator {
                // Generate keywords for the tool
                match request_generator.generate_tool_keywords(&tool_name, &tool_def, None).await {
                    Ok(result) => {
                        if result.success {
                            if let Some(keywords_text) = result.content {
                                // Parse keywords from comma-separated string
                                let keywords: Vec<String> = keywords_text
                                    .split(',')
                                    .map(|k| k.trim().to_string())
                                    .filter(|k| !k.is_empty())
                                    .collect();
                                
                                let elicitation_metadata = ElicitationMetadata {
                                    enhanced_keywords: Some(keywords),
                                    enhanced_categories: None, // Could be generated separately
                                    usage_patterns: None,      // Could be generated separately
                                    parameter_help: None,      // Could be generated separately
                                    parameter_examples: None,  // Could be generated separately
                                    elicitation_requests: None,
                                };
                                
                                enhanced_tool.elicitation_metadata = Some(elicitation_metadata);
                                generation_metadata.elicitation_template = Some("keyword_extraction".to_string());
                                debug!("✅ Tool enhancement completed for tool: {} ({}ms)", tool_name, result.metadata.generation_time_ms);
                            }
                        } else {
                            let error_msg = result.error.unwrap_or("Unknown error".to_string());
                            warn!("❌ Tool enhancement failed for tool '{}': {}", tool_name, error_msg);
                            if !self.config.graceful_degradation {
                                return Err(ProxyError::validation(format!("Tool enhancement failed: {}", error_msg)));
                            }
                        }
                    }
                    Err(e) => {
                        warn!("❌ Request generator failed for tool enhancement of tool '{}': {}", tool_name, e);
                        if !self.config.graceful_degradation {
                            return Err(e);
                        }
                    }
                }
            }
        }
        
        // Set enhancement metadata
        generation_metadata.generation_time_ms = Some(start_time.elapsed().as_millis() as u64);
        enhanced_tool.enhancement_metadata = Some(generation_metadata);
        enhanced_tool.enhanced_at = Some(chrono::Utc::now());
        
        // Determine final enhancement source
        enhanced_tool.enhancement_source = match (
            enhanced_tool.llm_enhanced_description.is_some(),
            enhanced_tool.elicitation_metadata.is_some()
        ) {
            (true, true) => EnhancementSource::Both,
            (true, false) => EnhancementSource::LlmDescription,
            (false, true) => EnhancementSource::Elicitation,
            (false, false) => EnhancementSource::Base,
        };
        
        // Cache if enabled
        if self.config.cache_enhancements && enhanced_tool.is_enhanced() {
            let mut cache = self.enhanced_cache.write().await;
            cache.insert(tool_name.to_string(), enhanced_tool.clone());
        }
        
        Ok(enhanced_tool)
    }
    
    /// Enhance tool description using tool enhancement service
    async fn enhance_with_tool_enhancement(
        &self,
        enhanced_tool: &mut EnhancedToolDefinition,
        tool_enhancement_service: &crate::mcp::tool_enhancement::ToolEnhancementService,
        generation_metadata: &mut EnhancementGenerationMetadata,
    ) -> Result<()> {
        // Create tool enhancement request for description enhancement
        let enhancement_request = tool_enhancement_service.generate_enhanced_description_request(
            &enhanced_tool.base.name,
            &enhanced_tool.base.description,
            &serde_json::to_value(&enhanced_tool.base.input_schema).unwrap_or(serde_json::Value::Null)
        ).await
        .map_err(|e| ProxyError::mcp(format!("Tool enhancement request generation error: {}", e.message)))?;
        
        let response = tool_enhancement_service.execute_server_generated_request(enhancement_request).await
            .map_err(|e| ProxyError::mcp(format!("Tool enhancement service error: {}", e.message)))?;
        
        // Extract enhanced description from response
        if let crate::mcp::types::tool_enhancement::ToolEnhancementContent::Text(enhanced_description) = &response.message.content {
            enhanced_tool.llm_enhanced_description = Some(enhanced_description.clone());
            generation_metadata.llm_model = Some(response.model);
            // Confidence based on stop reason and usage
            generation_metadata.llm_confidence = Some(match response.stop_reason {
                crate::mcp::types::tool_enhancement::ToolEnhancementStopReason::EndTurn => 0.9,
                crate::mcp::types::tool_enhancement::ToolEnhancementStopReason::MaxTokens => 0.7,
                _ => 0.6,
            });
        }
        
        Ok(())
    }
    
    /// Enhance tool metadata using elicitation service
    async fn enhance_with_elicitation(
        &self,
        enhanced_tool: &mut EnhancedToolDefinition,
        _elicitation_service: &ElicitationService,
        generation_metadata: &mut EnhancementGenerationMetadata,
    ) -> Result<()> {
        // For now, create basic elicitation metadata
        // TODO: Implement full elicitation service integration
        let mut elicitation_metadata = ElicitationMetadata {
            enhanced_keywords: None,
            enhanced_categories: None,
            usage_patterns: None,
            parameter_help: None,
            parameter_examples: None,
            elicitation_requests: None,
        };
        
        // Generate enhanced keywords based on tool name and description
        let mut keywords = vec![enhanced_tool.base.name.clone()];
        
        // Add keywords from description
        let description = enhanced_tool.effective_description();
        let desc_words: Vec<String> = description
            .split_whitespace()
            .filter(|word| word.len() > 3) // Skip short words
            .map(|word| word.trim_matches(&['.', ',', '!', '?', ';', ':'][..]).to_lowercase())
            .filter(|word| !["this", "that", "with", "from", "they", "have", "will", "been", "were"].contains(&word.as_str()))
            .collect();
        
        keywords.extend(desc_words);
        keywords.sort();
        keywords.dedup();
        
        elicitation_metadata.enhanced_keywords = Some(keywords);
        
        // TODO: Add more sophisticated elicitation metadata generation
        enhanced_tool.elicitation_metadata = Some(elicitation_metadata);
        generation_metadata.elicitation_template = Some("basic_keyword_extraction".to_string());
        
        Ok(())
    }
    
    /// Clear enhancement cache
    pub async fn clear_cache(&self) {
        info!("Clearing tool enhancement cache");
        self.enhanced_cache.write().await.clear();
        self.failure_cache.write().await.clear();
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> serde_json::Value {
        let enhanced_cache = self.enhanced_cache.read().await;
        let failure_cache = self.failure_cache.read().await;
        
        serde_json::json!({
            "enhanced_tools_cached": enhanced_cache.len(),
            "failed_enhancements_cached": failure_cache.len(),
            "cache_enabled": self.config.cache_enhancements,
            "description_enhancement_enabled": self.config.enable_description_enhancement,
            "tool_enhancement_enabled": self.config.enable_tool_enhancement
        })
    }
}

/// Implement EnhancementCallback trait for ToolEnhancementPipeline
#[async_trait::async_trait]
impl EnhancementCallback for ToolEnhancementPipeline {
    async fn on_tools_changed(&self, changed_tools: Vec<(String, ToolDefinition)>) -> Result<()> {
        info!("🔔 ToolEnhancementService received notification of {} changed tools - processing in background", changed_tools.len());
        
        // We need to create Arc references to the fields we need since self isn't Clone
        let config = self.config.clone();
        let registry = Arc::clone(&self.registry);
        let tool_enhancement_service = self.tool_enhancement_service.clone();
        let elicitation_service = self.elicitation_service.clone();
        let storage_service = self.storage_service.clone();
        let enhanced_cache = Arc::clone(&self.enhanced_cache);
        let elicitation_config = self.elicitation_config.clone();
        let smart_discovery_enabled = self.smart_discovery_enabled;
        
        // Process tool changes in background thread to avoid blocking
        tokio::spawn(async move {
            info!("🔄 Processing {} tool changes in background thread", changed_tools.len());
            
            // Create a temporary pipeline instance for processing
            let temp_pipeline = ToolEnhancementPipeline {
                config,
                registry,
                tool_enhancement_service,
                elicitation_service,
                request_generator: None, // Not needed for background processing
                enhanced_cache,
                failure_cache: Arc::new(RwLock::new(HashMap::new())), // Create new failure cache
                storage_service,
                elicitation_config,
                smart_discovery_enabled,
            };
            
            // Process tools that need enhancement directly 
            let tools_needing_enhancement = temp_pipeline.filter_tools_needing_enhancement(&changed_tools).await;
            
            if tools_needing_enhancement.is_empty() {
                info!("✅ No tools need enhancement in background processing");
                return;
            }

            info!("🔄 Processing {} tools needing enhancement in background", tools_needing_enhancement.len());
            
            // Process tools in batches or individually
            if temp_pipeline.config.batch_size > 1 && tools_needing_enhancement.len() > 1 {
                if let Err(e) = temp_pipeline.process_tools_in_batches(tools_needing_enhancement).await {
                    error!("Failed to process tool batches in background: {}", e);
                } else {
                    info!("✅ Tool batches processed successfully in background");
                }
            } else {
                let mut success_count = 0;
                let mut failure_count = 0;
                
                for (tool_name, tool_def) in tools_needing_enhancement {
                    match temp_pipeline.generate_and_store_enhancement(&tool_name, &tool_def).await {
                        Ok(_) => success_count += 1,
                        Err(e) => {
                            failure_count += 1;
                            error!("Failed to enhance tool '{}' in background: {}", tool_name, e);
                        }
                    }
                }
                
                info!("✅ Background tool enhancement completed: {} success, {} failures", success_count, failure_count);
            }
        });
        
        // Return immediately without blocking
        Ok(())
    }
}