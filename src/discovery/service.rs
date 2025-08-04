//! Smart Tool Discovery Service
//!
//! This service implements the core logic for discovering tools based on natural language
//! requests and mapping parameters using LLM integration.

use crate::discovery::types::*;
use crate::discovery::llm_mapper::{LlmParameterMapper, LlmMapperConfig};
use crate::discovery::cache::{DiscoveryCache, DiscoveryCacheConfig, ToolMatchCacheKey, LlmCacheKey, create_schema_hash};
use crate::discovery::fallback::{FallbackManager, FallbackConfig, ErrorCategory, SmartDiscoveryError};
use crate::discovery::semantic::{SemanticSearchService, SemanticSearchConfig};
use crate::discovery::embedding_manager::{EmbeddingManager, EmbeddingManagerConfig};
use crate::discovery::enhancement::{ToolEnhancementPipeline, ToolEnhancementConfig};
use crate::error::{ProxyError, Result};
use crate::registry::service::RegistryService;
use crate::registry::types::ToolDefinition;
use crate::routing::Router;
use crate::mcp::types::ToolCall;
use crate::metrics::tool_metrics::{ToolMetricsCollector, ToolExecutionRecord, ToolExecutionResult, DiscoveryRanking};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use chrono::Utc;
use uuid::Uuid;

/// Helper function for serde default value of true
fn default_true() -> bool {
    true
}

/// Configuration for LLM-based tool selection
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmToolSelectionConfig {
    /// Whether LLM tool selection is enabled
    pub enabled: bool,
    
    /// LLM provider (openai, anthropic, ollama, etc.)
    pub provider: String,
    
    /// Model name to use for tool selection
    pub model: String,
    
    /// API key (if required)
    pub api_key: Option<String>,
    
    /// Environment variable name for API key (if using env var)
    pub api_key_env: Option<String>,
    
    /// Base URL for API (if different from default)
    pub base_url: Option<String>,
    
    /// Request timeout in seconds
    pub timeout: u64,
    
    /// Maximum retries for failed requests
    pub max_retries: u32,
    
    /// Batch size for processing tools (to manage context limits)
    pub batch_size: usize,
    
    /// Maximum context tokens to use
    pub max_context_tokens: usize,
}

impl Default for LlmToolSelectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: None,
            api_key_env: Some("OPENAI_API_KEY".to_string()),
            base_url: None,
            timeout: 30,
            max_retries: 3,
            batch_size: 15,
            max_context_tokens: 4000,
        }
    }
}

/// Configuration for the Smart Discovery Service
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmartDiscoveryConfig {
    /// Whether smart discovery is enabled
    pub enabled: bool,
    
    /// Tool selection mode: "rule_based" or "llm_based"
    pub tool_selection_mode: String,
    
    /// Default confidence threshold for tool matching
    pub default_confidence_threshold: f64,
    
    /// Maximum number of tools to consider for matching
    pub max_tools_to_consider: usize,
    
    /// Maximum high-quality matches to collect before stopping processing
    pub max_high_quality_matches: usize,
    
    /// Confidence threshold for considering a match as high-quality (0.0-1.0)
    pub high_quality_threshold: f64,
    
    /// Whether to use fuzzy matching for tool names (rule-based mode only)
    pub use_fuzzy_matching: bool,
    
    /// Enable sampling service for enhanced LLM interactions (MCP 2025-06-18)
    pub enable_sampling: Option<bool>,
    
    /// Enable elicitation service for structured data collection (MCP 2025-06-18)
    pub enable_elicitation: Option<bool>,
    
    /// LLM mapper configuration
    pub llm_mapper: LlmMapperConfig,
    
    /// LLM tool selection configuration
    pub llm_tool_selection: LlmToolSelectionConfig,
    
    /// Cache configuration
    pub cache: DiscoveryCacheConfig,
    
    /// Fallback configuration
    pub fallback: FallbackConfig,
    
    /// Semantic search configuration
    pub semantic_search: SemanticSearchConfig,
    
    /// Whether to enable sequential mode for multi-step workflows
    #[serde(default = "default_true")]
    pub enable_sequential_mode: bool,
    
    /// Whether to enable tool metrics collection
    pub tool_metrics_enabled: Option<bool>,
    
    /// Default sampling routing strategy for tools discovered through smart discovery
    pub default_sampling_strategy: Option<crate::config::SamplingElicitationStrategy>,
    
    /// Default elicitation routing strategy for tools discovered through smart discovery
    pub default_elicitation_strategy: Option<crate::config::SamplingElicitationStrategy>,
}

impl Default for SmartDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tool_selection_mode: "rule_based".to_string(),
            default_confidence_threshold: 0.7,
            max_tools_to_consider: 10,
            max_high_quality_matches: 5,
            high_quality_threshold: 0.95,
            use_fuzzy_matching: true,
            enable_sampling: Some(false), // Disabled by default, enable via config
            enable_elicitation: Some(false), // Disabled by default, enable via config
            llm_mapper: LlmMapperConfig::default(),
            llm_tool_selection: LlmToolSelectionConfig::default(),
            cache: DiscoveryCacheConfig::default(),
            fallback: FallbackConfig::default(),
            semantic_search: SemanticSearchConfig::default(),
            enable_sequential_mode: true,
            tool_metrics_enabled: Some(true),
            default_sampling_strategy: None, // Inherit from server-level config
            default_elicitation_strategy: None, // Inherit from server-level config
        }
    }
}

/// Smart Tool Discovery Service
pub struct SmartDiscoveryService {
    /// Registry service for accessing tools
    registry: Arc<RegistryService>,
    
    /// Configuration for the discovery service
    config: SmartDiscoveryConfig,
    
    /// LLM parameter mapper
    llm_mapper: LlmParameterMapper,
    
    /// Cache manager for performance optimization
    cache: DiscoveryCache,
    
    /// Fallback manager for handling failures
    fallback_manager: std::sync::Mutex<FallbackManager>,
    
    /// Agent router for executing discovered tools
    router: Arc<tokio::sync::RwLock<Option<Arc<Router>>>>,
    
    /// Semantic search service
    semantic_search: Option<Arc<SemanticSearchService>>,
    
    /// Embedding manager for dynamic lifecycle management
    embedding_manager: Option<Arc<EmbeddingManager>>,
    
    /// Tool metrics collector for tracking usage and performance
    tool_metrics: Option<Arc<ToolMetricsCollector>>,
    
    /// Tool enhancement service for sampling/elicitation pipeline
    enhancement_service: Option<Arc<ToolEnhancementPipeline>>,
}

impl SmartDiscoveryService {
    /// Create a new Smart Discovery Service
    pub async fn new(registry: Arc<RegistryService>, config: SmartDiscoveryConfig) -> Result<Self> {
        Self::new_with_router(registry, config, None).await
    }
    
    /// Create a new Smart Discovery Service with an optional router for tool execution
    pub async fn new_with_router(registry: Arc<RegistryService>, config: SmartDiscoveryConfig, router: Option<Arc<Router>>) -> Result<Self> {
        Self::new_with_all_services(registry, config, router, None, None).await
    }
    
    /// Create a new Smart Discovery Service with all optional services
    pub async fn new_with_all_services(
        registry: Arc<RegistryService>, 
        config: SmartDiscoveryConfig, 
        router: Option<Arc<Router>>,
        tool_enhancement_service: Option<Arc<crate::mcp::tool_enhancement::ToolEnhancementService>>,
        elicitation_service: Option<Arc<crate::mcp::elicitation::ElicitationService>>,
    ) -> Result<Self> {
        let llm_mapper = LlmParameterMapper::new(config.llm_mapper.clone())?;
        let cache = DiscoveryCache::new(config.cache.clone());
        let fallback_manager = std::sync::Mutex::new(FallbackManager::new(config.fallback.clone()));
        
        // Initialize tool enhancement service first if sampling/elicitation enabled
        let enhancement_service = if config.enable_sampling.unwrap_or(false) || config.enable_elicitation.unwrap_or(false) {
            info!("🚀 Initializing tool enhancement service for sampling/elicitation pipeline");
            let enhancement_config = ToolEnhancementConfig {
                enable_sampling_enhancement: config.enable_sampling.unwrap_or(false),
                enable_elicitation_enhancement: config.enable_elicitation.unwrap_or(false),
                require_approval: false, // TODO: Make configurable
                cache_enhancements: true,
                enhancement_timeout_seconds: 30,
                batch_size: 10,
                graceful_degradation: true,
            };
            
            let service = ToolEnhancementPipeline::new(
                enhancement_config,
                Arc::clone(&registry),
                tool_enhancement_service,
                elicitation_service,
            );
            Some(Arc::new(service))
        } else {
            None
        };
        
        // Initialize semantic search service with enhancement support if enabled
        let semantic_search = if config.semantic_search.enabled {
            let service = if let Some(ref enhancement_service) = enhancement_service {
                info!("🌟 Creating semantic search service with sampling/elicitation enhancement support");
                SemanticSearchService::new_with_enhancement(
                    config.semantic_search.clone(),
                    Arc::clone(enhancement_service),
                )
            } else {
                info!("🔧 Creating semantic search service with base descriptions only");
                SemanticSearchService::new(config.semantic_search.clone())
            };
            Some(Arc::new(service))
        } else {
            None
        };
        
        // Initialize embedding manager with enhancement support if semantic search is enabled
        let embedding_manager = if let Some(ref semantic_service) = semantic_search {
            let manager_config = EmbeddingManagerConfig::default();
            let manager = if let Some(ref enhancement_service) = enhancement_service {
                info!("🌟 Creating embedding manager with sampling/elicitation enhancement support");
                EmbeddingManager::new_with_enhancement(
                    Arc::clone(&registry),
                    Arc::clone(semantic_service),
                    manager_config,
                    Arc::clone(enhancement_service),
                )
            } else {
                info!("🔧 Creating embedding manager with base descriptions only");
                EmbeddingManager::new(
                    Arc::clone(&registry),
                    Arc::clone(semantic_service),
                    manager_config,
                )
            };
            Some(Arc::new(manager))
        } else {
            None
        };
        
        // Initialize tool metrics collector if enabled in config
        let tool_metrics = if config.tool_metrics_enabled.unwrap_or(true) {
            let storage_path = "data/tool_metrics.json";
            match ToolMetricsCollector::new_with_storage(10000, storage_path).await {
                Ok(collector) => Some(Arc::new(collector)),
                Err(e) => {
                    warn!("Failed to create persistent tool metrics collector: {}. Using in-memory only.", e);
                    Some(Arc::new(ToolMetricsCollector::new(10000)))
                }
            }
        } else {
            None
        };
        
        Ok(Self { 
            registry, 
            config, 
            llm_mapper, 
            cache, 
            fallback_manager,
            semantic_search,
            embedding_manager,
            router: Arc::new(tokio::sync::RwLock::new(router)),
            tool_metrics,
            enhancement_service,
        })
    }

    /// Set the router for tool execution (can be called after service creation)
    pub async fn set_router(&self, router: Arc<Router>) {
        info!("Setting agent router for smart discovery service tool execution");
        *self.router.write().await = Some(router);
    }
    
    /// Create a new Smart Discovery Service with default configuration
    pub async fn new_with_defaults(registry: Arc<RegistryService>) -> Result<Self> {
        Self::new(registry, SmartDiscoveryConfig::default()).await
    }
    
    /// Initialize the smart discovery service (call after construction)
    pub async fn initialize(&self) -> Result<()> {
        if let Some(semantic_search) = &self.semantic_search {
            semantic_search.initialize().await?;
            info!("Semantic search service initialized");
            
            // Initialize embedding manager
            if let Some(embedding_manager) = &self.embedding_manager {
                embedding_manager.initialize().await?;
                info!("Embedding manager initialized");
            }
        }
        Ok(())
    }
    
    /// Get the tool metrics collector (if enabled)
    pub fn tool_metrics(&self) -> Option<Arc<ToolMetricsCollector>> {
        self.tool_metrics.clone()
    }
    
    /// Get enhanced tools for discovery (uses enhancement pipeline if available, otherwise base tools)
    async fn get_enhanced_tools_for_discovery(&self) -> Result<Vec<(String, ToolDefinition)>> {
        if let Some(enhancement_service) = &self.enhancement_service {
            info!("🔄 Using enhanced tools from sampling/elicitation pipeline");
            let enhanced_tools = enhancement_service.get_enhanced_tools().await?;
            
            // Count enhanced descriptions before consuming the HashMap
            let enhanced_count = enhanced_tools.values().filter(|t| t.sampling_enhanced_description.is_some()).count();
            
            // Convert EnhancedToolDefinition back to (String, ToolDefinition) for compatibility
            // Use effective_description() to get enhanced description if available
            let tools: Vec<(String, ToolDefinition)> = enhanced_tools
                .into_iter()
                .map(|(name, enhanced_tool)| {
                    let mut tool_def = enhanced_tool.base.clone();
                    
                    // Replace description with enhanced description if available
                    if let Some(enhanced_desc) = &enhanced_tool.sampling_enhanced_description {
                        tool_def.description = enhanced_desc.clone();
                    }
                    
                    // TODO: Integrate elicitation metadata into tool selection somehow
                    // For now we're focusing on the description enhancement which affects semantic search
                    
                    (name, tool_def)
                })
                .filter(|(tool_name, _)| {
                    // Skip smart_discovery_tool itself to avoid recursion
                    tool_name != "smart_discovery_tool" && tool_name != "smart_tool_discovery"
                })
                .collect();
            
            info!("📊 Enhanced tools prepared: {} tools with {} enhanced descriptions", 
                  tools.len(), enhanced_count);
            
            Ok(tools)
        } else {
            debug!("🔧 Using base tools (enhancement service not available)");
            let tools: Vec<(String, ToolDefinition)> = self.registry.get_enabled_tools()
                .into_iter()
                .filter(|(tool_name, _)| {
                    // Skip smart_discovery_tool itself to avoid recursion
                    tool_name != "smart_discovery_tool" && tool_name != "smart_tool_discovery"
                })
                .collect();
            Ok(tools)
        }
    }

    /// Process a smart discovery request
    pub fn discover_and_execute(&self, request: SmartDiscoveryRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SmartDiscoveryResponse>> + Send + '_>> {
        Box::pin(async move {
        info!("Processing smart discovery request: {}", request.request);
        
        // Check if discovery is enabled
        if !self.config.enabled {
            return self.create_error_response_with_fallback(
                "Smart discovery is disabled".to_string(),
                &ErrorCategory::SystemError,
                &request,
            ).await;
        }

        // Check if sequential mode is enabled and request is multi-step
        let sequential_mode = self.config.enable_sequential_mode && request.sequential_mode.unwrap_or(self.config.enable_sequential_mode);
        let mut original_request_for_next_step: Option<SmartDiscoveryRequest> = None;
        
        // If sequential mode is enabled, decompose into first step but continue with normal flow
        let effective_request = if sequential_mode {
            if let Some(first_step_request) = self.decompose_into_first_step(&request).await? {
                info!("🚀 Sequential mode: Processing first step of multi-step request: {}", first_step_request.request);
                original_request_for_next_step = Some(request.clone());
                first_step_request
            } else {
                request
            }
        } else {
            request
        };

        // Step 1: Find matching tools
        let tool_matches = match self.find_matching_tools(&effective_request).await {
            Ok(matches) => matches,
            Err(e) => {
                warn!("Failed to find matching tools: {}", e);
                return self.create_error_response_with_fallback(
                    format!("Failed to find matching tools: {}", e),
                    &ErrorCategory::NoToolsFound,
                    &effective_request,
                ).await;
            }
        };
        
        // Step 2: Select best tool match
        let best_match = match self.select_best_tool_match(&tool_matches, &effective_request) {
            Ok(match_) => match_,
            Err(e) => {
                warn!("Failed to select best tool match: {}", e);
                return self.create_error_response_with_fallback(
                    format!("Failed to select best tool match: {}", e),
                    &if tool_matches.is_empty() { ErrorCategory::NoToolsFound } else { ErrorCategory::LowConfidence },
                    &effective_request,
                ).await;
            }
        };
        
        // Step 3: Extract parameters using LLM
        let tool_def = match self.registry.get_tool(&best_match.tool_name) {
            Some(def) => def,
            None => {
                error!("Tool '{}' not found in registry", best_match.tool_name);
                return self.create_error_response_with_fallback(
                    format!("Tool '{}' not found in registry", best_match.tool_name),
                    &ErrorCategory::SystemError,
                    &effective_request,
                ).await;
            }
        };
        
        info!("🎯 PARAMETER EXTRACTION - Selected tool: '{}' for request: '{}'", 
              best_match.tool_name, effective_request.request);
        info!("📋 Tool description: {}", tool_def.description);
        if let Ok(schema_json) = serde_json::to_string_pretty(&tool_def.input_schema) {
            info!("📝 Expected parameters schema:\n{}", schema_json);
        }
        
        // Check LLM cache first
        let schema_hash = create_schema_hash(&tool_def.input_schema);
        let llm_cache_key = LlmCacheKey::new(&effective_request, &best_match.tool_name, &schema_hash);
        
        let parameter_extraction = if let Some(cached_extraction) = self.cache.get_llm_response(&llm_cache_key).await {
            info!("✅ Using cached LLM parameter extraction for tool: {}", best_match.tool_name);
            info!("📦 Cached extraction status: {:?}", cached_extraction.status);
            if let Ok(params_json) = serde_json::to_string_pretty(&cached_extraction.parameters) {
                info!("📦 Cached extracted parameters:\n{}", params_json);
            }
            cached_extraction
        } else {
            info!("🤖 Starting LLM parameter extraction for tool: '{}'", best_match.tool_name);
            match self.llm_mapper.extract_parameters(&effective_request, &tool_def).await {
                Ok(extraction) => {
                    info!("✅ LLM parameter extraction completed with status: {:?}", extraction.status);
                    if let Ok(params_json) = serde_json::to_string_pretty(&extraction.parameters) {
                        info!("📤 Extracted parameters:\n{}", params_json);
                    }
                    if !extraction.warnings.is_empty() {
                        warn!("⚠️  Parameter extraction warnings: {:?}", extraction.warnings);
                    }
                    if !extraction.used_defaults.is_empty() {
                        info!("🔧 Used default values: {:?}", extraction.used_defaults);
                    }
                    
                    // Cache the LLM response for future use
                    self.cache.store_llm_response(llm_cache_key, extraction.clone()).await;
                    info!("💾 Cached LLM parameter extraction for tool: {}", best_match.tool_name);
                    extraction
                }
                Err(e) => {
                    error!("❌ LLM parameter extraction failed for tool '{}': {}", best_match.tool_name, e);
                    return self.create_error_response_with_fallback(
                        format!("LLM parameter extraction failed: {}", e),
                        &ErrorCategory::ParameterExtractionFailed,
                        &effective_request,
                    ).await;
                }
            }
        };
        
        // Record tool usage for fallback statistics
        if let Ok(mut fallback_manager) = self.fallback_manager.lock() {
            fallback_manager.record_tool_usage(&best_match.tool_name);
            
            // If this was successful after previous failures, record the resolution
            if matches!(parameter_extraction.status, ExtractionStatus::Success) {
                fallback_manager.record_successful_resolution(
                    &effective_request.request,
                    &effective_request.request, // Same request that worked
                    &best_match.tool_name,
                );
            }
        }
        
        // Step 4: Build response with discovery metadata
        let mut metadata = SmartDiscoveryMetadata::default();
        metadata.original_tool = Some(best_match.tool_name.clone());
        metadata.confidence_score = best_match.confidence_score;
        metadata.reasoning = Some(best_match.reasoning.clone());
        metadata.mapped_parameters = Some(parameter_extraction.parameters.clone());
        metadata.extraction_status = Some(format!("{:?}", parameter_extraction.status));
        
        // Include all tool candidates with their confidence scores for debugging and analysis
        let tool_candidates: Vec<crate::discovery::types::ToolCandidateInfo> = tool_matches.iter().map(|m| {
            crate::discovery::types::ToolCandidateInfo {
                tool_name: m.tool_name.clone(),
                confidence_score: m.confidence_score,
                reasoning: m.reasoning.clone(),
                meets_threshold: m.meets_threshold,
            }
        }).collect();
        metadata.tool_candidates = Some(tool_candidates);
        
        // Record discovery rankings for tool metrics
        if let Some(ref metrics_collector) = self.tool_metrics {
            for (position, tool_match) in tool_matches.iter().enumerate() {
                if position < 30 { // Only track top 30 tools
                    let ranking = DiscoveryRanking {
                        position: (position + 1) as u32, // 1-based position
                        confidence_score: tool_match.confidence_score,
                        discovery_method: self.config.tool_selection_mode.clone(),
                        query: effective_request.request.clone(),
                        timestamp: Utc::now(),
                    };
                    
                    metrics_collector.record_discovery_ranking(&tool_match.tool_name, ranking).await;
                }
            }
        }
        
        info!("🎬 FINAL RESULT - Tool: '{}', Status: {:?}, Success: {}", 
              best_match.tool_name, 
              parameter_extraction.status,
              matches!(parameter_extraction.status, ExtractionStatus::Success | ExtractionStatus::Incomplete));
        
        if let Ok(final_params_json) = serde_json::to_string_pretty(&parameter_extraction.parameters) {
            info!("🎯 FINAL PARAMETERS TO CALL TOOL WITH:\n{}", final_params_json);
        }
        
        // Determine success based on extraction status
        let extraction_success = matches!(parameter_extraction.status, ExtractionStatus::Success | ExtractionStatus::Incomplete);
        
        // Execute the discovered tool if we have a router and extraction was successful
        let router_opt = self.router.read().await.clone();
        let (data, execution_error, final_success) = if extraction_success && router_opt.is_some() {
            info!("🚀 EXECUTING DISCOVERED TOOL: '{}' with agent router", best_match.tool_name);
            
            // Create a tool call for the discovered tool with extracted parameters
            let tool_call = ToolCall {
                name: best_match.tool_name.clone(),
                arguments: serde_json::Value::Object(parameter_extraction.parameters.clone().into_iter().collect()),
            };
            
            // Record execution start time for metrics
            let execution_start = Utc::now();
            let execution_start_instant = std::time::Instant::now();
            
            // Execute the tool using the router
            match router_opt.as_ref().unwrap().route(&tool_call, &tool_def).await {
                Ok(agent_result) => {
                    let duration_ms = execution_start_instant.elapsed().as_millis() as u64;
                    info!("✅ TOOL EXECUTION SUCCESS - Tool: '{}' executed successfully in {}ms", best_match.tool_name, duration_ms);
                    
                    // Record successful execution in metrics
                    if let Some(ref metrics_collector) = self.tool_metrics {
                        let discovery_context = Some(DiscoveryRanking {
                            position: 1, // This was the selected tool
                            confidence_score: best_match.confidence_score,
                            discovery_method: self.config.tool_selection_mode.clone(),
                            query: effective_request.request.clone(),
                            timestamp: execution_start,
                        });
                        
                        let output_size = match &agent_result.data {
                            Some(data) => data.to_string().len(),
                            None => 0,
                        };
                        
                        let execution_record = ToolExecutionRecord {
                            execution_id: Uuid::new_v4().to_string(),
                            tool_name: best_match.tool_name.clone(),
                            start_time: execution_start,
                            duration_ms,
                            result: ToolExecutionResult::Success {
                                output_size,
                                output_type: "json".to_string(), // Could be determined from agent_result
                            },
                            input_hash: format!("{:x}", md5::compute(serde_json::to_string(&parameter_extraction.parameters).unwrap_or_default())),
                            discovery_context,
                            execution_source: "smart_discovery".to_string(),
                            service_source: agent_result.metadata
                                .as_ref()
                                .and_then(|m| m.get("service_name"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        };
                        
                        metrics_collector.record_execution(execution_record).await;
                    }
                    
                    // Convert AgentResult to the format expected by smart discovery
                    let execution_data = serde_json::json!({
                        "message": "Tool discovered, parameters extracted, and executed successfully",
                        "tool": best_match.tool_name,
                        "confidence": best_match.confidence_score,
                        "parameters": parameter_extraction.parameters,
                        "extraction_status": format!("{:?}", parameter_extraction.status),
                        "warnings": parameter_extraction.warnings,
                        "used_defaults": parameter_extraction.used_defaults,
                        "execution_result": agent_result.data,
                        "execution_success": true,
                        "execution_metadata": agent_result.metadata
                    });
                    
                    (Some(execution_data), None, true)
                }
                Err(e) => {
                    let duration_ms = execution_start_instant.elapsed().as_millis() as u64;
                    error!("❌ TOOL EXECUTION FAILED - Tool: '{}', Error: {} ({}ms)", best_match.tool_name, e, duration_ms);
                    
                    // Record failed execution in metrics
                    if let Some(ref metrics_collector) = self.tool_metrics {
                        let discovery_context = Some(DiscoveryRanking {
                            position: 1, // This was the selected tool
                            confidence_score: best_match.confidence_score,
                            discovery_method: self.config.tool_selection_mode.clone(),
                            query: effective_request.request.clone(),
                            timestamp: execution_start,
                        });
                        
                        let error_type = if e.to_string().contains("timeout") {
                            "timeout"
                        } else if e.to_string().contains("network") || e.to_string().contains("connection") {
                            "network_error"
                        } else if e.to_string().contains("parameter") || e.to_string().contains("argument") {
                            "parameter_error"
                        } else {
                            "execution_error"
                        };
                        
                        let execution_record = ToolExecutionRecord {
                            execution_id: Uuid::new_v4().to_string(),
                            tool_name: best_match.tool_name.clone(),
                            start_time: execution_start,
                            duration_ms,
                            result: ToolExecutionResult::Error {
                                error_type: error_type.to_string(),
                                error_message: e.to_string(),
                                is_timeout: error_type == "timeout",
                            },
                            input_hash: format!("{:x}", md5::compute(serde_json::to_string(&parameter_extraction.parameters).unwrap_or_default())),
                            discovery_context,
                            execution_source: "smart_discovery".to_string(),
                            service_source: None,
                        };
                        
                        metrics_collector.record_execution(execution_record).await;
                    }
                    
                    // Return discovery data with execution error
                    let discovery_data = serde_json::json!({
                        "message": "Tool discovered and parameters extracted, but execution failed",
                        "tool": best_match.tool_name,
                        "confidence": best_match.confidence_score,
                        "parameters": parameter_extraction.parameters,
                        "extraction_status": format!("{:?}", parameter_extraction.status),
                        "warnings": parameter_extraction.warnings,
                        "used_defaults": parameter_extraction.used_defaults,
                        "execution_result": null,
                        "execution_success": false,
                        "execution_error": e.to_string()
                    });
                    
                    (Some(discovery_data), Some(format!("Tool execution failed: {}", e)), false)
                }
            }
        } else if extraction_success {
            // No router available, return discovery data only
            info!("ℹ️  No router available - returning discovery results only");
            let discovery_data = serde_json::json!({
                "message": "Tool discovered and parameters extracted successfully (no execution - router not available)",
                "tool": best_match.tool_name,
                "confidence": best_match.confidence_score,
                "parameters": parameter_extraction.parameters,
                "extraction_status": format!("{:?}", parameter_extraction.status),
                "warnings": parameter_extraction.warnings,
                "used_defaults": parameter_extraction.used_defaults,
                "execution_result": null,
                "execution_success": false,
                "execution_note": "Router not available for tool execution"
            });
            
            (Some(discovery_data), None, true)
        } else {
            // Parameter extraction failed
            (None, None, false)
        };
        
        let error = if final_success {
            execution_error
        } else {
            let warning_text = parameter_extraction.warnings.join("\n");
            Some(format!(
                "🔧 Parameter extraction needs your help:\n\n{}\n\n💬 Tip: Try rephrasing your request with specific values (file names, URLs, search terms, etc.)",
                warning_text
            ))
        };
        
        let include_details = effective_request.include_error_details.unwrap_or(false);
        
        // Generate next step recommendation if this is sequential mode and successful
        let next_step = if final_success && original_request_for_next_step.is_some() {
            let original_req = original_request_for_next_step.as_ref().unwrap();
            info!("🔮 Sequential mode: Generating next step recommendation for original request");
            
            // Create a temporary response to pass to next step generation
            let temp_response = SmartDiscoveryResponse {
                success: final_success,
                data: data.clone(),
                error: None,
                error_summary: None,
                error_details: None,
                metadata: metadata.clone(),
                next_step: None,
            };
            
            match self.generate_next_step_recommendation(original_req, &effective_request, &temp_response).await {
                Ok(Some(next_step_rec)) => {
                    info!("💡 Generated next step recommendation: {}", next_step_rec.suggested_request);
                    Some(next_step_rec)
                }
                Ok(None) => {
                    warn!("⚠️ No next step recommendation generated");
                    None
                }
                Err(e) => {
                    error!("❌ Failed to generate next step recommendation: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        Ok(SmartDiscoveryResponse {
            success: final_success,
            data,
            error: error.clone(),
            error_summary: if final_success { None } else { 
                let summary = self.generate_error_summary(&parameter_extraction.warnings);
                let detail_hint = if !include_details {
                    " 📝 Add 'include_error_details: true' to your request for technical details."
                } else {
                    ""
                };
                Some(format!("{}{}", summary, detail_hint))
            },
            error_details: if final_success || !include_details { None } else {
                Some(self.generate_error_details(&parameter_extraction, &tool_def.name))
            },
            metadata,
            next_step,
        })
        })
    }

    /// Find all tools that might match the request
    async fn find_matching_tools(&self, request: &SmartDiscoveryRequest) -> Result<Vec<ToolMatch>> {
        // Check cache first
        let cache_key = ToolMatchCacheKey::from_request(request, &self.config.tool_selection_mode);
        if let Some(cached_matches) = self.cache.get_tool_matches(&cache_key).await {
            debug!("Using cached tool matches for request: {} (mode: {})", request.request, self.config.tool_selection_mode);
            return Ok(cached_matches);
        }
        
        // Get enhanced tools for discovery (uses sampling/elicitation pipeline if available)
        // TODO: Add caching for enhanced tools
        let all_tools = self.get_enhanced_tools_for_discovery().await?;
        
        debug!("Found {} discoverable tools to search", all_tools.len());
        
        // Choose tool selection method based on configuration
        let matches = match self.config.tool_selection_mode.as_str() {
            "llm_based" => {
                info!("Using LLM-based tool selection for request: \"{}\"", request.request);
                self.find_matching_tools_llm(request, &all_tools).await?
            }
            "semantic_based" => {
                info!("Using semantic-based tool selection for request: \"{}\"", request.request);
                self.find_matching_tools_semantic(request, &all_tools).await?
            }
            "hybrid" => {
                info!("Using hybrid tool selection for request: \"{}\"", request.request);
                self.find_matching_tools_hybrid(request, &all_tools).await?
            }
            "rule_based" | _ => {
                info!("Using rule-based tool selection for request: \"{}\"", request.request);
                self.find_matching_tools_rule_based(request, &all_tools).await?
            }
        };
        
        debug!("Found {} potential tool matches", matches.len());
        
        // Cache the results for future use
        self.cache.store_tool_matches(cache_key, matches.clone()).await;
        
        Ok(matches)
    }

    /// Select the best tool match from the candidates
    fn select_best_tool_match(&self, matches: &[ToolMatch], request: &SmartDiscoveryRequest) -> Result<ToolMatch> {
        if matches.is_empty() {
            return Err(ProxyError::validation("No tools found matching the request"));
        }
        
        let threshold = self.get_confidence_threshold(request);
        
        info!("Tool Discovery - Request: \"{}\"", request.request);
        info!("Tool Discovery - Confidence threshold: {:.2}", threshold);
        info!("Tool Discovery - Evaluating {} tool candidates:", matches.len());
        
        for (i, tool_match) in matches.iter().enumerate() {
            info!("  {}. {} (confidence: {:.2}, meets_threshold: {}) - {}", 
                  i + 1, 
                  tool_match.tool_name, 
                  tool_match.confidence_score, 
                  tool_match.meets_threshold,
                  tool_match.reasoning);
        }
        
        // Find the best match that meets the threshold
        let best_match = matches.iter()
            .find(|m| m.meets_threshold)
            .or_else(|| matches.first()) // Fall back to highest confidence even if below threshold
            .ok_or_else(|| ProxyError::validation("No suitable tool match found"))?;
        
        info!("Tool Discovery - SELECTED: '{}' with confidence {:.2} (meets_threshold: {})", 
              best_match.tool_name, best_match.confidence_score, best_match.meets_threshold);
        
        Ok(best_match.clone())
    }

    /// Calculate confidence score for a tool match
    /// Calculate enhanced confidence score using elicitation metadata
    fn calculate_enhanced_confidence_for_tool(&self, enhanced_tool: &EnhancedToolDefinition, request: &SmartDiscoveryRequest) -> f64 {
        let mut confidence = 0.0;
        let request_lower = request.request.to_lowercase();
        let tool_name_lower = enhanced_tool.base.name.to_lowercase();
        
        // Use enhanced description if available, otherwise base description
        let tool_desc = enhanced_tool.effective_description();
        let tool_desc_lower = tool_desc.to_lowercase();
        
        let mut score_breakdown = Vec::new();
        
        // Exact name match gets highest confidence
        if tool_name_lower == request_lower {
            confidence += 0.8;
            score_breakdown.push(format!("exact_name_match: +0.8"));
        }
        // Partial name match - check if any significant words from the request appear in the tool name
        else {
            let request_words: Vec<&str> = request_lower.split_whitespace()
                .filter(|word| word.len() > 2) // Skip short words like "to", "a", "the"
                .collect();
            
            let mut word_matches = 0;
            let mut matched_words = Vec::new();
            
            for word in &request_words {
                if tool_name_lower.contains(word) {
                    word_matches += 1;
                    matched_words.push(*word);
                }
            }
            
            if word_matches > 0 {
                let word_match_score = (word_matches as f64 / request_words.len() as f64) * 0.6;
                confidence += word_match_score;
                score_breakdown.push(format!("name_words({}): +{:.2}", matched_words.join(","), word_match_score));
            }
        }
        
        // Enhanced keyword matching using elicitation metadata
        if let Some(elicitation_metadata) = &enhanced_tool.elicitation_metadata {
            if let Some(enhanced_keywords) = &elicitation_metadata.enhanced_keywords {
                let mut keyword_score = 0.0f64;
                let mut matched_keywords = Vec::new();
                
                for keyword in enhanced_keywords {
                    let keyword_lower = keyword.to_lowercase();
                    if request_lower.contains(&keyword_lower) {
                        keyword_score += 0.15; // Higher weight for elicitation keywords
                        matched_keywords.push(keyword.as_str());
                    }
                }
                
                if keyword_score > 0.0 {
                    confidence += keyword_score.min(0.4); // Cap at 0.4
                    score_breakdown.push(format!("elicitation_keywords({}): +{:.2}", 
                                               matched_keywords.join(","), keyword_score.min(0.4)));
                }
            }
            
            // Enhanced categories matching
            if let Some(enhanced_categories) = &elicitation_metadata.enhanced_categories {
                let mut category_score = 0.0f64;
                let mut matched_categories = Vec::new();
                
                for category in enhanced_categories {
                    let category_lower = category.to_lowercase();
                    if request_lower.contains(&category_lower) {
                        category_score += 0.1;
                        matched_categories.push(category.as_str());
                    }
                }
                
                if category_score > 0.0 {
                    confidence += category_score.min(0.3); // Cap at 0.3
                    score_breakdown.push(format!("elicitation_categories({}): +{:.2}", 
                                               matched_categories.join(","), category_score.min(0.3)));
                }
            }
            
            // Usage patterns matching
            if let Some(usage_patterns) = &elicitation_metadata.usage_patterns {
                let mut pattern_score = 0.0f64;
                let mut matched_patterns = Vec::new();
                
                for pattern in usage_patterns {
                    let pattern_lower = pattern.to_lowercase();
                    if request_lower.contains(&pattern_lower) || 
                       self.calculate_string_similarity(&request_lower, &pattern_lower) > 0.7 {
                        pattern_score += 0.12;
                        matched_patterns.push(pattern.as_str());
                    }
                }
                
                if pattern_score > 0.0 {
                    confidence += pattern_score.min(0.35); // Cap at 0.35
                    score_breakdown.push(format!("usage_patterns({}): +{:.2}", 
                                               matched_patterns.join(","), pattern_score.min(0.35)));
                }
            }
        }
        
        // Description matching (using enhanced description if available)
        let desc_similarity = self.calculate_string_similarity(&request_lower, &tool_desc_lower);
        if desc_similarity > 0.3 {
            let desc_score = desc_similarity * 0.4;
            confidence += desc_score;
            let desc_type = if enhanced_tool.sampling_enhanced_description.is_some() { 
                "enhanced_desc" 
            } else { 
                "base_desc" 
            };
            score_breakdown.push(format!("{}({:.1}%): +{:.2}", desc_type, desc_similarity * 100.0, desc_score));
        }
        
        // Standard keyword matching (fallback for tools without elicitation)
        if enhanced_tool.elicitation_metadata.is_none() {
            let keyword_score = self.calculate_keyword_match_score(&request.request, &enhanced_tool.base.name, &tool_desc);
            if keyword_score > 0.0 {
                confidence += keyword_score;
                score_breakdown.push(format!("standard_keywords: +{:.2}", keyword_score));
            }
        }
        
        // Fuzzy matching if enabled (simplified approach)
        if self.config.use_fuzzy_matching {
            let fuzzy_score = self.calculate_string_similarity(&request_lower, &tool_name_lower) * 0.2;
            if fuzzy_score > 0.0 {
                confidence += fuzzy_score;
                score_breakdown.push(format!("fuzzy: +{:.2}", fuzzy_score));
            }
        }
        
        // Ensure confidence is between 0 and 1
        let final_confidence = confidence.min(1.0f64).max(0.0f64);
        
        // Log detailed scoring for tools with reasonable confidence
        if final_confidence > 0.05 {
            debug!("Enhanced confidence breakdown for '{}': {} = {:.3}", 
                   enhanced_tool.base.name, score_breakdown.join(", "), final_confidence);
        }
        
        final_confidence
    }

    /// Generate enhanced reasoning that includes elicitation metadata
    fn generate_enhanced_reasoning(&self, enhanced_tool: &EnhancedToolDefinition, request: &SmartDiscoveryRequest, confidence: f64) -> String {
        let mut reasons = Vec::new();
        
        // Base matching info
        if enhanced_tool.base.name.to_lowercase() == request.request.to_lowercase() {
            reasons.push("exact name match".to_string());
        }
        
        // Enhanced description info
        if enhanced_tool.sampling_enhanced_description.is_some() {
            reasons.push("sampling-enhanced description".to_string());
        }
        
        // Elicitation metadata info
        if let Some(elicitation) = &enhanced_tool.elicitation_metadata {
            if let Some(keywords) = &elicitation.enhanced_keywords {
                let matched_keywords: Vec<&str> = keywords.iter()
                    .filter(|k| request.request.to_lowercase().contains(&k.to_lowercase()))
                    .map(|s| s.as_str())
                    .collect();
                if !matched_keywords.is_empty() {
                    reasons.push(format!("elicitation keywords: {}", matched_keywords.iter().take(3).cloned().collect::<Vec<_>>().join(", ")));
                }
            }
            
            if let Some(categories) = &elicitation.enhanced_categories {
                let matched_categories: Vec<&str> = categories.iter()
                    .filter(|c| request.request.to_lowercase().contains(&c.to_lowercase()))
                    .map(|s| s.as_str())
                    .collect();
                if !matched_categories.is_empty() {
                    reasons.push(format!("categories: {}", matched_categories.iter().take(2).cloned().collect::<Vec<_>>().join(", ")));
                }
            }
            
            if let Some(patterns) = &elicitation.usage_patterns {
                let matched_patterns: Vec<&str> = patterns.iter()
                    .filter(|p| request.request.to_lowercase().contains(&p.to_lowercase()) || 
                                self.calculate_string_similarity(&request.request.to_lowercase(), &p.to_lowercase()) > 0.7)
                    .map(|s| s.as_str())
                    .collect();
                if !matched_patterns.is_empty() {
                    reasons.push(format!("usage patterns: {}", matched_patterns.iter().take(2).cloned().collect::<Vec<_>>().join(", ")));
                }
            }
        }
        
        // Enhancement source info
        let source_info = match enhanced_tool.enhancement_source {
            crate::discovery::types::EnhancementSource::Both => "sampling + elicitation",
            crate::discovery::types::EnhancementSource::Sampling => "sampling only",
            crate::discovery::types::EnhancementSource::Elicitation => "elicitation only", 
            crate::discovery::types::EnhancementSource::Base => "base tool",
            crate::discovery::types::EnhancementSource::Manual => "manual enhancement",
            crate::discovery::types::EnhancementSource::External => "external enhancement",
        };
        
        format!("Enhanced tool match ({}) with confidence {:.2}: {}", 
                source_info, confidence, reasons.join("; "))
    }

    fn calculate_confidence_for_tool(&self, tool_def: &ToolDefinition, request: &SmartDiscoveryRequest) -> f64 {
        let mut confidence = 0.0;
        let request_lower = request.request.to_lowercase();
        let tool_name_lower = tool_def.name.to_lowercase();
        let tool_desc_lower = tool_def.description.to_lowercase();
        
        let mut score_breakdown = Vec::new();
        
        // Exact name match gets highest confidence
        if tool_name_lower == request_lower {
            confidence += 0.8;
            score_breakdown.push(format!("exact_name_match: +0.8"));
        }
        // Partial name match - check if any significant words from the request appear in the tool name
        else {
            let request_words: Vec<&str> = request_lower.split_whitespace()
                .filter(|word| word.len() > 2) // Skip short words like "to", "a", "the"
                .collect();
            
            let mut word_matches = 0;
            let mut matched_words = Vec::new();
            
            for word in &request_words {
                if tool_name_lower.contains(word) {
                    word_matches += 1;
                    matched_words.push(*word);
                }
            }
            
            if word_matches > 0 {
                let word_match_score = (word_matches as f64 / request_words.len() as f64) * 0.6;
                confidence += word_match_score;
                score_breakdown.push(format!("word_match: +{:.3} (matched: {:?})", word_match_score, matched_words));
            }
            // Fuzzy name matching if no word matches
            else if self.config.use_fuzzy_matching {
                let name_similarity = self.calculate_string_similarity(&tool_name_lower, &request_lower);
                let fuzzy_score = name_similarity * 0.5;
                confidence += fuzzy_score;
                score_breakdown.push(format!("fuzzy_name_match: +{:.3}", fuzzy_score));
            }
        }
        
        // Description matching - check if any significant words from the request appear in the description
        let request_words: Vec<&str> = request_lower.split_whitespace()
            .filter(|word| word.len() > 2) // Skip short words like "to", "a", "the"
            .collect();
        
        let mut desc_word_matches = 0;
        let mut desc_matched_words = Vec::new();
        
        for word in &request_words {
            if tool_desc_lower.contains(word) {
                desc_word_matches += 1;
                desc_matched_words.push(*word);
            }
        }
        
        if desc_word_matches > 0 {
            let desc_match_score = (desc_word_matches as f64 / request_words.len() as f64) * 0.4;
            confidence += desc_match_score;
            score_breakdown.push(format!("description_match: +{:.3} (matched: {:?})", desc_match_score, desc_matched_words));
        }
        
        // Keyword matching for common operations
        let keyword_score = self.calculate_keyword_match_score(&request_lower, &tool_name_lower, &tool_desc_lower);
        if keyword_score > 0.0 {
            confidence += keyword_score;
            score_breakdown.push(format!("keyword_match: +{:.3}", keyword_score));
        }
        
        // Context matching if provided
        if let Some(context) = &request.context {
            let context_lower = context.to_lowercase();
            let context_words: Vec<&str> = context_lower.split_whitespace()
                .filter(|word| word.len() > 2)
                .collect();
            
            let mut context_word_matches = 0;
            let mut context_matched_words = Vec::new();
            
            for word in &context_words {
                if tool_desc_lower.contains(word) || tool_name_lower.contains(word) {
                    context_word_matches += 1;
                    context_matched_words.push(*word);
                }
            }
            
            if context_word_matches > 0 {
                let context_match_score = (context_word_matches as f64 / context_words.len() as f64) * 0.2;
                confidence += context_match_score;
                score_breakdown.push(format!("context_match: +{:.3} (matched: {:?})", context_match_score, context_matched_words));
            }
        }
        
        // Apply constraint validation 
        let (can_fulfill, constraint_reasoning, constraint_score) = self.validate_tool_constraints(tool_def, &request.request);
        confidence *= constraint_score;
        
        if !can_fulfill {
            score_breakdown.push(format!("constraint_violation: x{:.2} ({})", constraint_score, constraint_reasoning));
            info!("🚨 Tool '{}' has constraint violations: {}", tool_def.name, constraint_reasoning);
        } else if constraint_score < 1.0 {
            score_breakdown.push(format!("minor_constraints: x{:.2}", constraint_score));
        }
        
        // Ensure confidence is between 0 and 1
        let final_confidence = confidence.min(1.0f64).max(0.0f64);
        
        // Log detailed scoring for tools with reasonable confidence or constraint issues
        if final_confidence > 0.05 || !can_fulfill {
            debug!("Confidence breakdown for '{}': {} = {:.3}", 
                   tool_def.name, score_breakdown.join(", "), final_confidence);
        }
        
        final_confidence
    }

    /// Calculate keyword match score for common operations
    fn calculate_keyword_match_score(&self, request: &str, tool_name: &str, tool_desc: &str) -> f64 {
        let keywords = vec![
            ("read", vec!["read", "get", "fetch", "load", "retrieve"]),
            ("write", vec!["write", "save", "store", "put", "create"]),
            ("search", vec!["search", "find", "lookup", "query", "grep"]),
            ("http", vec!["http", "request", "api", "web", "url"]),
            ("file", vec!["file", "document", "path", "directory"]),
            ("database", vec!["database", "db", "sql", "query", "table"]),
            ("ai", vec!["ai", "llm", "generate", "chat", "completion"]),
            ("network", vec!["ping", "traceroute", "dns", "mtr", "network", "connectivity", "latency"]),
            ("monitor", vec!["monitor", "check", "status", "health", "test"]),
            ("measure", vec!["measure", "measurement", "benchmark", "performance", "speed"]),
        ];
        
        let mut score = 0.0f64;
        
        for (_category, terms) in keywords {
            for term in terms {
                if request.contains(term) {
                    if tool_name.contains(term) || tool_desc.contains(term) {
                        score += 0.1f64;
                    }
                }
            }
        }
        
        score.min(0.3f64) // Cap at 0.3 to prevent keyword matching from dominating
    }

    /// Calculate string similarity using simple character overlap
    fn calculate_string_similarity(&self, s1: &str, s2: &str) -> f64 {
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }
        
        let s1_chars: std::collections::HashSet<char> = s1.chars().collect();
        let s2_chars: std::collections::HashSet<char> = s2.chars().collect();
        
        let intersection = s1_chars.intersection(&s2_chars).count();
        let union = s1_chars.union(&s2_chars).count();
        
        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Generate reasoning for why a tool was selected
    fn generate_reasoning(&self, tool_def: &ToolDefinition, request: &SmartDiscoveryRequest, confidence: f64) -> String {
        let mut reasons = Vec::new();
        
        let request_lower = request.request.to_lowercase();
        let tool_name_lower = tool_def.name.to_lowercase();
        let tool_desc_lower = tool_def.description.to_lowercase();
        
        if tool_name_lower.contains(&request_lower) || request_lower.contains(&tool_name_lower) {
            reasons.push("tool name matches request".to_string());
        }
        
        if tool_desc_lower.contains(&request_lower) {
            reasons.push("tool description matches request".to_string());
        }
        
        if reasons.is_empty() {
            reasons.push("general semantic similarity".to_string());
        }
        
        format!("Selected due to {} (confidence: {:.2})", reasons.join(", "), confidence)
    }

    /// Get the confidence threshold for a request
    fn get_confidence_threshold(&self, request: &SmartDiscoveryRequest) -> f64 {
        request.confidence_threshold
            .unwrap_or(self.config.default_confidence_threshold)
    }

    /// Create an error response with fallback suggestions
    async fn create_error_response_with_fallback(
        &self,
        error: String,
        error_category: &ErrorCategory,
        request: &SmartDiscoveryRequest,
    ) -> Result<SmartDiscoveryResponse> {
        // Get available tools for fallback (all enabled tools)
        let available_tools = self.registry.get_enabled_tools();
        let available_tools_vec: Vec<_> = available_tools.iter()
            .map(|(name, def)| (name.clone(), def.clone()))
            .collect();
        
        // Record failure pattern for learning and execute fallback strategies
        let fallback_result = if let Ok(mut fallback_manager) = self.fallback_manager.lock() {
            // Record the failure pattern for learning
            fallback_manager.record_failure_pattern(&request.request, error_category.clone());
            
            // Add learned suggestions to the response
            let learned_suggestions = fallback_manager.get_learned_suggestions(&request.request);
            
            let mut result = fallback_manager.execute_fallback(request, &available_tools_vec, &error);
            
            // Enhance the fallback result with learned suggestions
            if !learned_suggestions.is_empty() {
                // Add learned suggestions as additional fallback suggestions
                for suggestion in learned_suggestions {
                    result.suggestions.push(crate::discovery::fallback::FallbackSuggestion {
                        tool_name: "learned_pattern".to_string(),
                        confidence_score: 0.6,
                        strategy: crate::discovery::fallback::FallbackStrategy::SimilarTools,
                        reasoning: format!("Based on learning: {}", suggestion),
                        meets_threshold: true,
                    });
                }
            }
            
            Some(result)
        } else {
            warn!("Failed to acquire fallback manager lock");
            None
        };
        
        // Create enhanced error
        let enhanced_error = if let Ok(fallback_manager) = self.fallback_manager.lock() {
            fallback_manager.create_enhanced_error(&error, error_category.clone(), fallback_result.clone())
        } else {
            SmartDiscoveryError {
                primary_error: error.clone(),
                category: error_category.clone(),
                fallback_result: fallback_result.clone(),
                user_message: error.clone(),
                suggested_actions: vec!["Try rephrasing your request".to_string()],
            }
        };
        
        // Build metadata with fallback information
        let mut metadata = SmartDiscoveryMetadata::default();
        metadata.reasoning = Some(enhanced_error.user_message.clone());
        
        // Build response data with fallback suggestions
        let mut response_data = serde_json::json!({
            "message": enhanced_error.user_message,
            "error_category": format!("{:?}", enhanced_error.category),
            "suggested_actions": enhanced_error.suggested_actions,
        });
        
        // Add parameter help for parameter extraction failures
        if matches!(error_category, ErrorCategory::ParameterExtractionFailed) {
            // Try to get the tool that was matched to provide parameter help
            if let Some(tool_name) = metadata.original_tool.as_ref() {
                if let Some(tool_def) = self.registry.get_tool(tool_name) {
                    // Extract missing parameters from the error message
                    let missing_params = self.extract_missing_parameters_from_error(&error);
                    if !missing_params.is_empty() {
                        let param_suggestions = self.llm_mapper.generate_parameter_suggestions(&tool_def, &missing_params);
                        response_data["parameter_help"] = serde_json::json!({
                            "missing_required": missing_params,
                            "parameter_info": param_suggestions
                        });
                    }
                    
                    // Add usage examples for the tool
                    let usage_examples = self.generate_usage_examples(&tool_def);
                    response_data["usage_examples"] = serde_json::json!({
                        "tool_name": tool_name,
                        "examples": usage_examples,
                        "description": format!("Here are some ways you can use the '{}' tool:", tool_name)
                    });
                    
                    // Add interactive clarification request
                    if !missing_params.is_empty() {
                        let clarification_request = self.llm_mapper.generate_clarification_request(&tool_def, &missing_params);
                        response_data["clarification_request"] = serde_json::to_value(&clarification_request)
                            .unwrap_or_else(|_| serde_json::json!({}));
                    }
                }
            }
        }
        
        // For no tools found errors, provide examples of what's available
        if matches!(error_category, ErrorCategory::NoToolsFound) {
            let has_suggestions = fallback_result.as_ref()
                .map(|fr| !fr.suggestions.is_empty())
                .unwrap_or(false);
                
            if !has_suggestions {
                let all_examples = self.get_tool_usage_examples();
                // Show examples from a few popular tools
                let sample_tools: Vec<_> = all_examples.iter().take(5).collect();
                if !sample_tools.is_empty() {
                    response_data["available_tools_examples"] = serde_json::json!({
                        "description": "Here are some examples of what you can do with available tools:",
                        "examples": sample_tools.iter().map(|(tool_name, examples)| serde_json::json!({
                            "tool": tool_name,
                            "examples": examples
                        })).collect::<Vec<_>>()
                    });
                }
            }
        }
        
        if let Some(fallback) = &fallback_result {
            response_data["fallback_suggestions"] = serde_json::json!({
                "strategies_attempted": fallback.strategies_attempted,
                "has_viable_suggestions": fallback.has_viable_suggestions,
                "suggestions": fallback.suggestions.iter().map(|s| serde_json::json!({
                    "tool_name": s.tool_name,
                    "confidence_score": s.confidence_score,
                    "strategy": format!("{:?}", s.strategy),
                    "reasoning": s.reasoning,
                    "meets_threshold": s.meets_threshold,
                })).collect::<Vec<_>>()
            });
        }
        
        let include_details = request.include_error_details.unwrap_or(false);
        
        Ok(SmartDiscoveryResponse {
            success: false,
            data: Some(response_data),
            error: Some(enhanced_error.primary_error.clone()),
            error_summary: Some({
                let detail_hint = if !include_details {
                    " 📝 Add 'include_error_details: true' to your request for technical details."
                } else {
                    ""
                };
                format!("{}{}", enhanced_error.user_message, detail_hint)
            }),
            error_details: if include_details {
                Some(crate::discovery::types::ErrorDetails {
                    technical_details: Some(enhanced_error.primary_error.clone()),
                    diagnostics: Some(serde_json::json!({
                        "error_category": format!("{:?}", enhanced_error.category),
                        "fallback_attempted": enhanced_error.fallback_result.is_some(),
                        "strategies_used": enhanced_error.fallback_result.as_ref()
                            .map(|fr| fr.strategies_attempted)
                            .unwrap_or(0)
                    })),
                    debug_info: Some(format!("Request: '{}'", request.request)),
                    help_instructions: Some(
                        "For more help, try: 1) Rephrasing your request, 2) Being more specific, 3) Providing examples".to_string()
                    ),
                })
            } else {
                None
            },
            metadata,
            next_step: None,
        })
    }

    /// Create an error response
    fn create_error_response(
        &self,
        error: String,
        _suggestions: Option<Vec<String>>,
        metadata: Option<SmartDiscoveryMetadata>,
    ) -> Result<SmartDiscoveryResponse> {
        Ok(SmartDiscoveryResponse {
            success: false,
            data: None,
            error: Some(error),
            error_summary: None,
            error_details: None,
            metadata: metadata.unwrap_or_default(),
            next_step: None,
        })
    }

    /// Check if the service is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get statistics about the registry and cache
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let (total, visible, hidden) = self.registry.visibility_stats();
        let (_, enabled, disabled) = self.registry.enabled_stats();
        let cache_stats = self.cache.get_stats().await;
        
        let mut stats = HashMap::new();
        stats.insert("total_tools".to_string(), serde_json::Value::Number(total.into()));
        stats.insert("visible_tools".to_string(), serde_json::Value::Number(visible.into()));
        stats.insert("hidden_tools".to_string(), serde_json::Value::Number(hidden.into()));
        stats.insert("enabled_tools".to_string(), serde_json::Value::Number(enabled.into()));
        stats.insert("disabled_tools".to_string(), serde_json::Value::Number(disabled.into()));
        stats.insert("discoverable_tools".to_string(), serde_json::Value::Number(enabled.into()));
        stats.insert("discovery_enabled".to_string(), serde_json::Value::Bool(self.config.enabled));
        stats.insert("default_confidence_threshold".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(self.config.default_confidence_threshold).unwrap()));
        
        // Add cache statistics
        stats.insert("cache_enabled".to_string(), serde_json::Value::Bool(self.cache.is_enabled()));
        stats.insert("cache_hits".to_string(), serde_json::Value::Number(cache_stats.hits.into()));
        stats.insert("cache_misses".to_string(), serde_json::Value::Number(cache_stats.misses.into()));
        stats.insert("cache_hit_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(cache_stats.hit_rate).unwrap()));
        stats.insert("cache_evictions".to_string(), serde_json::Value::Number(cache_stats.evictions.into()));
        stats.insert("cache_entries".to_string(), serde_json::Value::Number(cache_stats.entries.into()));
        
        // Add learning statistics
        let learning_stats = self.get_learning_stats();
        stats.insert("learning_stats".to_string(), learning_stats);
        
        stats
    }

    /// Clear all caches
    pub async fn clear_cache(&self) {
        self.cache.clear_all().await;
        info!("Cleared all smart discovery caches");
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> serde_json::Value {
        let stats = self.cache.get_stats().await;
        serde_json::json!({
            "enabled": self.cache.is_enabled(),
            "hits": stats.hits,
            "misses": stats.misses,
            "hit_rate": stats.hit_rate,
            "evictions": stats.evictions,
            "entries": stats.entries
        })
    }

    /// Get learning statistics
    pub fn get_learning_stats(&self) -> serde_json::Value {
        if let Ok(fallback_manager) = self.fallback_manager.lock() {
            fallback_manager.get_learning_stats()
        } else {
            serde_json::json!({
                "learning_enabled": false,
                "error": "Failed to acquire fallback manager lock"
            })
        }
    }
    
    /// Generate usage examples for a tool
    pub fn generate_usage_examples(&self, tool_def: &ToolDefinition) -> Vec<String> {
        let tool_name = &tool_def.name;
        let tool_desc = &tool_def.description;
        
        // Generate examples based on tool name and description patterns
        if tool_name.contains("read") || tool_desc.to_lowercase().contains("read") {
            vec![
                format!("read the config.yaml file"),
                format!("read data from /path/to/file.txt"),
                format!("load content from document.json"),
            ]
        } else if tool_name.contains("write") || tool_desc.to_lowercase().contains("write") {
            vec![
                format!("write data to output.json"),
                format!("save results to /tmp/results.txt"),
                format!("create a new file with content"),
            ]
        } else if tool_name.contains("http") || tool_desc.to_lowercase().contains("http") || tool_desc.to_lowercase().contains("api") {
            vec![
                format!("make a GET request to https://api.example.com/data"),
                format!("post data to https://httpbin.org/post"),
                format!("fetch information from a REST API"),
            ]
        } else if tool_name.contains("search") || tool_desc.to_lowercase().contains("search") {
            vec![
                format!("search for 'error' in log files"),
                format!("find files matching *.py"),
                format!("look for TODO comments in code"),
            ]
        } else if tool_name.contains("ping") || tool_desc.to_lowercase().contains("ping") {
            vec![
                format!("ping google.com"),
                format!("check connectivity to 8.8.8.8"),
                format!("test if server.example.com is reachable"),
            ]
        } else if tool_name.contains("database") || tool_name.contains("db") || tool_desc.to_lowercase().contains("database") {
            vec![
                format!("query the users table"),
                format!("run SELECT * FROM products"),
                format!("check database connectivity"),
            ]
        } else {
            vec![
                format!("use {} to accomplish your task", tool_name),
                format!("try: {}", tool_desc.to_lowercase()),
                format!("invoke {} with your specific parameters", tool_name),
            ]
        }
    }
    
    /// Get usage examples for all tools
    pub fn get_tool_usage_examples(&self) -> HashMap<String, Vec<String>> {
        let mut examples = HashMap::new();
        
        let all_tools = self.registry.get_enabled_tools();
        for (tool_name, tool_def) in all_tools.iter() {
            let tool_examples = self.generate_usage_examples(tool_def);
            examples.insert(tool_name.clone(), tool_examples);
        }
        
        examples
    }
    
    /// Get fallback statistics
    pub fn get_fallback_stats(&self) -> serde_json::Value {
        if let Ok(fallback_manager) = self.fallback_manager.lock() {
            let config = fallback_manager.get_config();
            let usage_stats = fallback_manager.get_usage_stats();
            let recent_tools = fallback_manager.get_recent_tools();
            
            serde_json::json!({
                "enabled": config.enabled,
                "min_confidence_threshold": config.min_confidence_threshold,
                "max_fallback_suggestions": config.max_fallback_suggestions,
                "strategies": {
                    "fuzzy_fallback": config.enable_fuzzy_fallback,
                    "keyword_fallback": config.enable_keyword_fallback,
                    "category_fallback": config.enable_category_fallback,
                    "partial_match_fallback": config.enable_partial_match_fallback,
                },
                "usage_statistics": usage_stats,
                "recent_tools": recent_tools,
                "total_tools_tracked": usage_stats.len(),
                "total_usage": usage_stats.values().sum::<u64>(),
            })
        } else {
            serde_json::json!({
                "enabled": false,
                "error": "Failed to acquire fallback manager lock"
            })
        }
    }

    /// Check if fallback is enabled
    pub fn is_fallback_enabled(&self) -> bool {
        if let Ok(fallback_manager) = self.fallback_manager.lock() {
            fallback_manager.is_enabled()
        } else {
            false
        }
    }

    /// Rule-based tool matching with elicitation enhancement
    async fn find_matching_tools_rule_based(&self, request: &SmartDiscoveryRequest, all_tools: &[(String, ToolDefinition)]) -> Result<Vec<ToolMatch>> {
        // If enhancement service is available, use enhanced rule-based matching
        if let Some(enhancement_service) = &self.enhancement_service {
            debug!("🎯 Using elicitation-enhanced rule-based matching");
            return self.find_matching_tools_rule_based_enhanced(request, enhancement_service).await;
        }
        
        // Fallback to original rule-based matching
        debug!("🔧 Using original rule-based matching (no enhancement service)");
        self.find_matching_tools_rule_based_original(request, all_tools).await
    }

    /// Enhanced rule-based matching that leverages elicitation metadata
    async fn find_matching_tools_rule_based_enhanced(&self, request: &SmartDiscoveryRequest, enhancement_service: &ToolEnhancementPipeline) -> Result<Vec<ToolMatch>> {
        let mut matches = Vec::new();
        
        // Get enhanced tools
        let enhanced_tools = match enhancement_service.get_enhanced_tools().await {
            Ok(tools) => tools,
            Err(e) => {
                warn!("Failed to get enhanced tools for rule-based matching: {}. Using base tools.", e);
                let base_tools: Vec<(String, ToolDefinition)> = self.registry.get_enabled_tools().into_iter().collect();
                return self.find_matching_tools_rule_based_original(request, &base_tools).await;
            }
        };
        
        info!("🔍 Evaluating {} enhanced tools for rule-based matching", enhanced_tools.len());
        
        // If preferred tools are specified, check them first
        if let Some(preferred_tools) = &request.preferred_tools {
            for tool_name in preferred_tools {
                // Skip smart_tool_discovery to avoid recursion
                if tool_name == "smart_discovery_tool" || tool_name == "smart_tool_discovery" {
                    continue;
                }
                
                if let Some(enhanced_tool) = enhanced_tools.get(tool_name) {
                    let confidence = self.calculate_enhanced_confidence_for_tool(enhanced_tool, request);
                    
                    matches.push(ToolMatch {
                        tool_name: tool_name.clone(),
                        confidence_score: confidence,
                        reasoning: format!("Preferred enhanced tool '{}' with confidence {:.2}", tool_name, confidence),
                        meets_threshold: confidence >= self.get_confidence_threshold(request),
                    });
                }
            }
        }
        
        // Check if we already have enough high-quality matches from preferred tools
        let high_quality_threshold = self.config.high_quality_threshold;
        let high_quality_matches = matches.iter()
            .filter(|m| m.confidence_score >= high_quality_threshold)
            .count();
        
        if high_quality_matches >= self.config.max_high_quality_matches {
            info!("Already found {} high-quality matches from preferred enhanced tools, skipping full evaluation", 
                 high_quality_matches);
        } else {
            // Search through all enhanced tools for matches
            for (tool_name, enhanced_tool) in enhanced_tools.iter() {
                // Skip smart_tool_discovery to avoid recursion
                if tool_name == "smart_discovery_tool" || tool_name == "smart_tool_discovery" {
                    continue;
                }
                
                // Skip if already added as preferred tool
                if let Some(preferred) = &request.preferred_tools {
                    if preferred.contains(tool_name) {
                        continue;
                    }
                }
                
                let confidence = self.calculate_enhanced_confidence_for_tool(enhanced_tool, request);
                debug!("Enhanced Tool Evaluation: {} -> confidence: {:.3} (base: \"{}\", sampling: {:?})", 
                       tool_name, confidence, enhanced_tool.base.description, 
                       enhanced_tool.sampling_enhanced_description.as_ref().map(|s| &s[..50.min(s.len())]));
                
                // Only include if confidence is reasonable
                if confidence > 0.1 {
                    info!("Including enhanced tool: {} (confidence: {:.3})", tool_name, confidence);
                    matches.push(ToolMatch {
                        tool_name: tool_name.clone(),
                        confidence_score: confidence,
                        reasoning: self.generate_enhanced_reasoning(enhanced_tool, request, confidence),
                        meets_threshold: confidence >= self.get_confidence_threshold(request),
                    });
                    
                    // Check if we have enough high-quality matches to stop early
                    let current_high_quality = matches.iter()
                        .filter(|m| m.confidence_score >= high_quality_threshold)
                        .count();
                    
                    if current_high_quality >= self.config.max_high_quality_matches {
                        info!("Found {} high-quality enhanced matches (>= {:.1}), stopping early processing", 
                             current_high_quality, high_quality_threshold);
                        break;
                    }
                }
            }
        }
        
        // Sort by confidence score
        matches.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit number of matches
        matches.truncate(self.config.max_tools_to_consider);
        
        info!("Enhanced rule-based matching completed: {} matches found", matches.len());
        Ok(matches)
    }

    /// Original rule-based tool matching (fallback)
    async fn find_matching_tools_rule_based_original(&self, request: &SmartDiscoveryRequest, all_tools: &[(String, ToolDefinition)]) -> Result<Vec<ToolMatch>> {
        let mut matches = Vec::new();
        
        // If preferred tools are specified, check them first
        if let Some(preferred_tools) = &request.preferred_tools {
            for tool_name in preferred_tools {
                // Skip smart_tool_discovery to avoid recursion
                if tool_name == "smart_discovery_tool" || tool_name == "smart_tool_discovery" {
                    continue;
                }
                
                if let Some(tool_def) = all_tools.iter().find(|(name, _)| name == tool_name) {
                    let confidence = self.calculate_confidence_for_tool(&tool_def.1, request);
                    
                    matches.push(ToolMatch {
                        tool_name: tool_name.clone(),
                        confidence_score: confidence,
                        reasoning: format!("Preferred tool '{}' with confidence {:.2}", tool_name, confidence),
                        meets_threshold: confidence >= self.get_confidence_threshold(request),
                    });
                }
            }
        }
        
        // Check if we already have enough high-quality matches from preferred tools
        let high_quality_threshold = self.config.high_quality_threshold;
        let high_quality_matches = matches.iter()
            .filter(|m| m.confidence_score >= high_quality_threshold)
            .count();
        
        if high_quality_matches >= self.config.max_high_quality_matches {
            info!("Already found {} high-quality matches from preferred tools, skipping full evaluation", 
                 high_quality_matches);
        } else {
            // Search through all tools for matches
            info!("Evaluating {} tools for request: \"{}\"", all_tools.len(), request.request);
            for (tool_name, tool_def) in all_tools.iter() {
                // Skip smart_tool_discovery to avoid recursion
                if tool_name == "smart_discovery_tool" || tool_name == "smart_tool_discovery" {
                    continue;
                }
                
                // Skip if already added as preferred tool
                if let Some(preferred) = &request.preferred_tools {
                    if preferred.contains(tool_name) {
                        continue;
                    }
                }
                
                let confidence = self.calculate_confidence_for_tool(tool_def, request);
                debug!("Tool Evaluation: {} -> confidence: {:.3} (description: \"{}\")", 
                       tool_name, confidence, tool_def.description);
                
                // Only include if confidence is reasonable
                if confidence > 0.1 {
                    info!("Including tool: {} (confidence: {:.3})", tool_name, confidence);
                    matches.push(ToolMatch {
                        tool_name: tool_name.clone(),
                        confidence_score: confidence,
                        reasoning: self.generate_reasoning(tool_def, request, confidence),
                        meets_threshold: confidence >= self.get_confidence_threshold(request),
                    });
                    
                    // Check if we have enough high-quality matches to stop early
                    let current_high_quality = matches.iter()
                        .filter(|m| m.confidence_score >= high_quality_threshold)
                        .count();
                    
                    if current_high_quality >= self.config.max_high_quality_matches {
                        info!("Found {} high-quality matches (>= {:.1}), stopping early processing", 
                             current_high_quality, high_quality_threshold);
                        break;
                    }
                } else {
                    debug!("Excluding tool: {} (confidence: {:.3} <= 0.1)", tool_name, confidence);
                }
            }
        }
        
        // Sort by confidence score (highest first)
        matches.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit to max tools to consider
        matches.truncate(self.config.max_tools_to_consider);
        
        Ok(matches)
    }

    /// LLM-based tool matching using batch processing
    async fn find_matching_tools_llm(&self, request: &SmartDiscoveryRequest, all_tools: &[(String, ToolDefinition)]) -> Result<Vec<ToolMatch>> {
        if !self.config.llm_tool_selection.enabled {
            warn!("LLM tool selection is disabled, falling back to rule-based");
            return self.find_matching_tools_rule_based(request, all_tools).await;
        }

        let mut matches = Vec::new();
        
        // If preferred tools are specified, check them first with LLM
        if let Some(preferred_tools) = &request.preferred_tools {
            let preferred_tool_defs: Vec<_> = preferred_tools.iter()
                .filter(|name| *name != "smart_discovery_tool" && *name != "smart_tool_discovery")
                .filter_map(|name| all_tools.iter().find(|(tool_name, _)| tool_name == name))
                .collect();
            
            if !preferred_tool_defs.is_empty() {
                let preferred_matches = self.evaluate_tools_with_llm(request, &preferred_tool_defs).await?;
                matches.extend(preferred_matches);
            }
        }
        
        // Process tools in batches to manage context limits
        let batch_size = self.config.llm_tool_selection.batch_size;
        let remaining_tools: Vec<_> = all_tools.iter()
            .filter(|(tool_name, _)| {
                // Skip smart_tool_discovery to avoid recursion
                if tool_name == "smart_discovery_tool" || tool_name == "smart_tool_discovery" {
                    return false;
                }
                
                if let Some(preferred) = &request.preferred_tools {
                    !preferred.contains(tool_name)
                } else {
                    true
                }
            })
            .collect();
        
        info!("Processing {} tools in batches of {} for LLM evaluation", remaining_tools.len(), batch_size);
        
        // Use configurable high-quality threshold
        let high_quality_threshold = self.config.high_quality_threshold;
        
        for batch in remaining_tools.chunks(batch_size) {
            let batch_matches = self.evaluate_tools_with_llm(request, batch).await?;
            matches.extend(batch_matches);
            
            // Check if we have enough high-quality matches to stop early
            let high_quality_matches = matches.iter()
                .filter(|m| m.confidence_score >= high_quality_threshold)
                .count();
            
            if high_quality_matches >= self.config.max_high_quality_matches {
                info!("Found {} high-quality matches (>= {:.1}), stopping early processing", 
                     high_quality_matches, high_quality_threshold);
                break;
            }
        }
        
        // Sort by confidence score (highest first)
        matches.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit to max tools to consider
        matches.truncate(self.config.max_tools_to_consider);
        
        Ok(matches)
    }

    /// Evaluate a batch of tools using LLM
    async fn evaluate_tools_with_llm(&self, request: &SmartDiscoveryRequest, tools: &[&(String, ToolDefinition)]) -> Result<Vec<ToolMatch>> {
        let mut matches = Vec::new();
        
        // Build the prompt for LLM evaluation
        let prompt = self.build_llm_tool_selection_prompt(request, tools);
        
        debug!("Evaluating {} tools with LLM", tools.len());
        
        // Make LLM API call
        let response = self.call_llm_for_tool_selection(&prompt).await?;
        
        // Parse the response and create tool matches
        let parsed_matches = self.parse_llm_tool_selection_response(&response, tools)?;
        
        for tool_match in parsed_matches {
            if tool_match.confidence_score > 0.1 {
                info!("LLM selected tool: {} (confidence: {:.3})", tool_match.tool_name, tool_match.confidence_score);
                matches.push(tool_match);
            } else {
                debug!("LLM excluded tool: {} (confidence: {:.3} <= 0.1)", tool_match.tool_name, tool_match.confidence_score);
            }
        }
        
        Ok(matches)
    }

    /// Build the prompt for LLM tool selection
    fn build_llm_tool_selection_prompt(&self, request: &SmartDiscoveryRequest, tools: &[&(String, ToolDefinition)]) -> String {
        let mut prompt = String::new();
        
        prompt.push_str("🔍 CONSTRAINT-AWARE TOOL SELECTION EXPERT\n");
        prompt.push_str("You must carefully check tool descriptions for limitations, constraints, and scope restrictions.\n\n");
        
        prompt.push_str(&format!("User Request: \"{}\"\n", request.request));
        
        if let Some(context) = &request.context {
            prompt.push_str(&format!("Context: \"{}\"\n", context));
        }
        
        prompt.push_str("\nAvailable Tools:\n");
        for (i, (tool_name, tool_def)) in tools.iter().enumerate() {
            prompt.push_str(&format!("{}. {} - {}\n", i + 1, tool_name, tool_def.description));
        }
        
        prompt.push_str("\n🚨 CRITICAL CONSTRAINT CHECKING:\n");
        prompt.push_str("1. READ each tool description for limitations like 'limited to', 'only supports', 'restricted to'\n");
        prompt.push_str("2. If tool has constraints that conflict with the user request, give LOW score (0.0-0.3)\n");
        prompt.push_str("3. Check scope restrictions (e.g., 'academic papers only' vs 'cooking recipes')\n");
        prompt.push_str("4. Verify capability boundaries (e.g., 'read-only' vs 'write operations')\n\n");
        
        prompt.push_str("SCORING GUIDELINES:\n");
        prompt.push_str("• 0.9-1.0: Perfect match, no constraints violated, can fully satisfy request\n");
        prompt.push_str("• 0.7-0.8: Good match, minor constraints but still suitable\n");
        prompt.push_str("• 0.5-0.6: Fair match, some limitations but might work\n");
        prompt.push_str("• 0.3-0.4: Poor match, significant constraint violations\n");
        prompt.push_str("• 0.0-0.2: No match, tool constraints prevent fulfilling request\n\n");
        
        prompt.push_str("⚠️ CONSTRAINT VIOLATION EXAMPLES:\n");
        prompt.push_str("• Search tool 'academic papers only' + request 'cooking recipes' = score 0.1\n");
        prompt.push_str("• File tool 'read-only access' + request 'write/edit file' = score 0.0\n");
        prompt.push_str("• API 'US data only' + request 'European data' = score 0.2\n\n");
        
        prompt.push_str("Respond in JSON format:\n");
        prompt.push_str("{\n");
        prompt.push_str("\"evaluations\": [\n");
        prompt.push_str("  {\n");
        prompt.push_str("    \"tool_name\": \"tool1\",\n");
        prompt.push_str("    \"confidence_score\": 0.95,\n");
        prompt.push_str("    \"reasoning\": \"Perfect match because...\",\n");
        prompt.push_str("    \"constraint_violations\": \"none\",\n");
        prompt.push_str("    \"can_fulfill_request\": true\n");
        prompt.push_str("  },\n");
        prompt.push_str("  {\n");
        prompt.push_str("    \"tool_name\": \"tool2\",\n");
        prompt.push_str("    \"confidence_score\": 0.1,\n");
        prompt.push_str("    \"reasoning\": \"Tool limited to X but user wants Y\",\n");
        prompt.push_str("    \"constraint_violations\": \"major\",\n");
        prompt.push_str("    \"can_fulfill_request\": false\n");
        prompt.push_str("  }\n");
        prompt.push_str("]\n");
        prompt.push_str("}\n");
        
        prompt
    }

    /// Validate tool constraints against user request (rule-based approach)
    fn validate_tool_constraints(&self, tool_def: &ToolDefinition, user_request: &str) -> (bool, String, f64) {
        let tool_desc = &tool_def.description.to_lowercase();
        let user_req = user_request.to_lowercase();
        
        // Common constraint patterns that indicate limitations
        let constraint_patterns = [
            ("limited to", "scope limitation"),
            ("only supports", "capability restriction"),  
            ("restricted to", "access restriction"),
            ("read-only", "write operation restriction"),
            ("cannot", "explicit limitation"),
            ("does not support", "unsupported feature"),
            ("excludes", "exclusion constraint"),
            ("academic papers only", "content type restriction"),
            ("us data only", "geographic restriction"),
            ("requires authentication", "auth requirement"),
        ];
        
        let mut constraint_violations = Vec::new();
        let mut constraint_score = 1.0;
        
        // Check for explicit constraint patterns
        for (pattern, violation_type) in &constraint_patterns {
            if tool_desc.contains(pattern) {
                // Extract the constraint context (next 20 words after the pattern)
                if let Some(pos) = tool_desc.find(pattern) {
                    let context_start = pos + pattern.len();
                    let context = tool_desc.chars().skip(context_start).take(100).collect::<String>();
                    
                    // Check if user request conflicts with this constraint
                    let conflicts = self.check_constraint_conflict(&context, &user_req, pattern);
                    if conflicts {
                        constraint_violations.push(format!("{}: {}", violation_type, pattern));
                        constraint_score *= 0.3; // Heavily penalize constraint violations
                        info!("🚨 Constraint violation detected - Tool: {}, Pattern: '{}', User request: '{}'", 
                              tool_def.name, pattern, user_request);
                    }
                }
            }
        }
        
        // Additional specific constraint checks
        if tool_desc.contains("academic") && (user_req.contains("recipe") || user_req.contains("shopping") || user_req.contains("entertainment")) {
            constraint_violations.push("academic constraint vs general content".to_string());
            constraint_score *= 0.2;
        }
        
        if tool_desc.contains("read") && !tool_desc.contains("write") && (user_req.contains("write") || user_req.contains("edit") || user_req.contains("create") || user_req.contains("update")) {
            constraint_violations.push("read-only vs write operation".to_string());
            constraint_score *= 0.1;
        }
        
        let can_fulfill = constraint_violations.is_empty();
        let reasoning = if can_fulfill {
            "No constraint violations detected".to_string()
        } else {
            format!("Constraint violations: {}", constraint_violations.join(", "))
        };
        
        (can_fulfill, reasoning, constraint_score)
    }
    
    /// Check if user request conflicts with a specific constraint
    fn check_constraint_conflict(&self, constraint_context: &str, user_request: &str, constraint_pattern: &str) -> bool {
        match constraint_pattern {
            "limited to" | "only supports" | "restricted to" => {
                // Extract what it's limited to and check if user wants something else
                let _context_words: Vec<&str> = constraint_context.split_whitespace().take(10).collect();
                let _context_str = _context_words.join(" ");
                
                // Simple keyword-based conflict detection
                if constraint_context.contains("academic") && !user_request.contains("academic") && !user_request.contains("research") && !user_request.contains("paper") {
                    return true;
                }
                if constraint_context.contains("us") && (user_request.contains("europe") || user_request.contains("asia") || user_request.contains("global")) {
                    return true;
                }
                if constraint_context.contains("news") && (user_request.contains("recipe") || user_request.contains("shopping") || user_request.contains("game")) {
                    return true;
                }
                false
            }
            "read-only" => {
                user_request.contains("write") || user_request.contains("edit") || user_request.contains("create") || user_request.contains("update")
            }
            "cannot" | "does not support" => {
                // More complex logic could be added here
                false
            }
            _ => false
        }
    }

    /// Find tools that would match the request but have constraint violations
    fn find_constraint_violating_tools(&self, user_request: &str, all_tools: &[(String, ToolDefinition)]) -> Vec<(String, String)> {
        let mut violating_tools = Vec::new();
        
        for (tool_name, tool_def) in all_tools {
            // Calculate base confidence without constraints
            let dummy_request = SmartDiscoveryRequest {
                request: user_request.to_string(),
                context: None,
                preferred_tools: None,
                confidence_threshold: None,
                include_error_details: None,
                sequential_mode: None,
            };
            
            // Check if tool would match without constraints
            let base_confidence = self.calculate_keyword_match_score(user_request, &tool_name.to_lowercase(), &tool_def.description.to_lowercase());
            
            // Check constraints
            let (can_fulfill, constraint_reasoning, constraint_score) = self.validate_tool_constraints(tool_def, user_request);
            
            // If base confidence is reasonable but constraints prevent usage
            if base_confidence > 0.1 && (!can_fulfill || constraint_score < 0.5) {
                violating_tools.push((tool_name.clone(), constraint_reasoning.clone()));
                info!("🔍 Found constraint-violating tool: {} - {}", tool_name, constraint_reasoning);
            }
        }
        
        violating_tools
    }

    /// Call LLM API for tool selection
    async fn call_llm_for_tool_selection(&self, prompt: &str) -> Result<String> {
        let config = &self.config.llm_tool_selection;
        
        match config.provider.as_str() {
            "openai" => self.call_openai_api(prompt).await,
            "anthropic" => self.call_anthropic_api(prompt).await,
            "ollama" => self.call_ollama_api(prompt).await,
            _ => {
                warn!("Unsupported LLM provider: {}, falling back to rule-based scoring", config.provider);
                Ok("{\"evaluations\": []}".to_string())
            }
        }
    }

    /// Call OpenAI API for tool selection
    async fn call_openai_api(&self, prompt: &str) -> Result<String> {
        let config = &self.config.llm_tool_selection;
        
        let api_key = config.api_key.as_ref()
            .ok_or_else(|| ProxyError::validation("OpenAI API key not configured".to_string()))?;
        
        let base_url = config.base_url.as_deref().unwrap_or("https://api.openai.com");
        let url = format!("{}/v1/chat/completions", base_url);
        
        let client = reqwest::Client::new();
        let request_body = serde_json::json!({
            "model": config.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a tool selection expert. Respond only with valid JSON in the exact format requested."
                },
                {
                    "role": "user", 
                    "content": prompt
                }
            ],
            "max_tokens": config.max_context_tokens,
            "temperature": 0.1
        });
        
        debug!("Making OpenAI API call to: {}", url);
        
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(config.timeout))
            .send()
            .await
            .map_err(|e| ProxyError::validation(format!("OpenAI API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProxyError::validation(format!("OpenAI API error: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ProxyError::validation(format!("Failed to parse OpenAI response: {}", e)))?;
        
        let content = response_json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| ProxyError::validation("Invalid OpenAI response format".to_string()))?;
        
        debug!("OpenAI API response received: {} characters", content.len());
        Ok(content.to_string())
    }

    /// Call Anthropic API for tool selection
    async fn call_anthropic_api(&self, prompt: &str) -> Result<String> {
        let config = &self.config.llm_tool_selection;
        
        let api_key = config.api_key.as_ref()
            .ok_or_else(|| ProxyError::validation("Anthropic API key not configured".to_string()))?;
        
        let base_url = config.base_url.as_deref().unwrap_or("https://api.anthropic.com");
        let url = format!("{}/v1/messages", base_url);
        
        let client = reqwest::Client::new();
        let request_body = serde_json::json!({
            "model": config.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": config.max_context_tokens,
            "temperature": 0.1
        });
        
        debug!("Making Anthropic API call to: {}", url);
        
        let response = client
            .post(&url)
            .header("x-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(config.timeout))
            .send()
            .await
            .map_err(|e| ProxyError::validation(format!("Anthropic API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProxyError::validation(format!("Anthropic API error: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ProxyError::validation(format!("Failed to parse Anthropic response: {}", e)))?;
        
        let content = response_json
            .get("content")
            .and_then(|content| content.get(0))
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .ok_or_else(|| ProxyError::validation("Invalid Anthropic response format".to_string()))?;
        
        debug!("Anthropic API response received: {} characters", content.len());
        Ok(content.to_string())
    }

    /// Call Ollama API for tool selection
    async fn call_ollama_api(&self, prompt: &str) -> Result<String> {
        let config = &self.config.llm_tool_selection;
        
        let base_url = config.base_url.as_deref().unwrap_or("http://localhost:11434");
        let url = format!("{}/api/generate", base_url);
        
        let client = reqwest::Client::new();
        let request_body = serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.1
            }
        });
        
        debug!("Making Ollama API call to: {}", url);
        
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(config.timeout))
            .send()
            .await
            .map_err(|e| ProxyError::validation(format!("Ollama API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProxyError::validation(format!("Ollama API error: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ProxyError::validation(format!("Failed to parse Ollama response: {}", e)))?;
        
        let content = response_json
            .get("response")
            .and_then(|response| response.as_str())
            .ok_or_else(|| ProxyError::validation("Invalid Ollama response format".to_string()))?;
        
        debug!("Ollama API response received: {} characters", content.len());
        Ok(content.to_string())
    }

    /// Parse LLM response for tool selection
    fn parse_llm_tool_selection_response(&self, response: &str, tools: &[&(String, ToolDefinition)]) -> Result<Vec<ToolMatch>> {
        let mut matches = Vec::new();
        
        // Debug: Log the raw response
        debug!("Raw LLM response: {}", response);
        
        // Extract JSON from response (handle markdown code blocks)
        let json_str = self.extract_json_from_response(response);
        debug!("Extracted JSON: {}", json_str);
        
        // Try to parse JSON response
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| {
                warn!("Failed to parse LLM response as JSON. Response: {}", response);
                warn!("Extracted JSON: {}", json_str);
                ProxyError::validation(format!("Failed to parse LLM response: {}", e))
            })?;
        
        if let Some(evaluations) = parsed.get("evaluations").and_then(|v| v.as_array()) {
            for evaluation in evaluations {
                if let (Some(tool_name), Some(confidence), Some(reasoning)) = (
                    evaluation.get("tool_name").and_then(|v| v.as_str()),
                    evaluation.get("confidence_score").and_then(|v| v.as_f64()),
                    evaluation.get("reasoning").and_then(|v| v.as_str()),
                ) {
                    // Extract constraint information
                    let constraint_violations = evaluation.get("constraint_violations")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let can_fulfill = evaluation.get("can_fulfill_request")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    
                    // Enhanced reasoning with constraint information
                    let enhanced_reasoning = if constraint_violations != "none" && !can_fulfill {
                        format!("🚨 CONSTRAINT VIOLATION: {} - {} (violations: {})", reasoning, 
                               if can_fulfill { "might still work" } else { "cannot fulfill request" }, 
                               constraint_violations)
                    } else {
                        format!("✅ {}", reasoning)
                    };
                    
                    // Lower confidence further if constraints are violated
                    let adjusted_confidence = if constraint_violations == "major" && !can_fulfill {
                        (confidence * 0.3).max(0.0) // Heavily penalize major violations
                    } else if constraint_violations == "minor" && !can_fulfill {
                        (confidence * 0.7).max(0.0) // Moderately penalize minor violations
                    } else {
                        confidence
                    };
                    
                    debug!("🔍 Tool constraint analysis: {} - confidence: {:.3} -> {:.3}, violations: {}, can_fulfill: {}", 
                           tool_name, confidence, adjusted_confidence, constraint_violations, can_fulfill);
                    
                    matches.push(ToolMatch {
                        tool_name: tool_name.to_string(),
                        confidence_score: adjusted_confidence,
                        reasoning: enhanced_reasoning,
                        meets_threshold: adjusted_confidence >= self.get_confidence_threshold(&SmartDiscoveryRequest {
                            request: "".to_string(),
                            context: None,
                            preferred_tools: None,
                            confidence_threshold: None,
                            include_error_details: None,
                            sequential_mode: None,
                        }),
                    });
                }
            }
        } else {
            // Fallback to rule-based scoring if LLM response is malformed
            warn!("LLM response malformed, falling back to rule-based scoring");
            
            // Create a dummy request for rule-based fallback
            let request = SmartDiscoveryRequest {
                request: "".to_string(),
                context: None,
                preferred_tools: None,
                confidence_threshold: None,
                include_error_details: None,
                sequential_mode: None,
            };
            
            for (tool_name, tool_def) in tools {
                let confidence = self.calculate_confidence_for_tool(tool_def, &request);
                matches.push(ToolMatch {
                    tool_name: tool_name.clone(),
                    confidence_score: confidence,
                    reasoning: format!("Rule-based fallback: {}", self.generate_reasoning(tool_def, &request, confidence)),
                    meets_threshold: confidence >= self.get_confidence_threshold(&request),
                });
            }
        }
        
        Ok(matches)
    }
    
    /// Generate a brief, user-friendly error summary
    fn generate_error_summary(&self, warnings: &[String]) -> String {
        if warnings.is_empty() {
            return "🤔 Something didn't work as expected. Try rephrasing your request.".to_string();
        }
        
        // Extract the core issue from the first warning
        let first_warning = &warnings[0];
        
        if first_warning.contains("Missing required parameters") {
            "📝 I need more information to complete your request".to_string()
        } else if first_warning.contains("couldn't extract") || first_warning.contains("extraction failed") {
            "🤖 I had trouble understanding the details in your request".to_string()
        } else if first_warning.contains("null") {
            "❓ Some information was unclear in your request".to_string()
        } else {
            "🤔 I need help understanding your request better".to_string()
        }
    }
    
    /// Generate detailed error information
    fn generate_error_details(
        &self,
        parameter_extraction: &ParameterExtraction,
        tool_name: &str,
    ) -> crate::discovery::types::ErrorDetails {
        let technical_details = if parameter_extraction.warnings.is_empty() {
            "Parameter extraction completed but no parameters were extracted".to_string()
        } else {
            parameter_extraction.warnings.join("; ")
        };
        
        let diagnostics = serde_json::json!({
            "extraction_status": format!("{:?}", parameter_extraction.status),
            "parameters_extracted": parameter_extraction.parameters.len(),
            "warnings_count": parameter_extraction.warnings.len(),
            "defaults_used": parameter_extraction.used_defaults.len(),
            "tool_name": tool_name,
            "parameters_found": parameter_extraction.parameters.keys().collect::<Vec<_>>()
        });
        
        let debug_info = format!(
            "Tool: {} | Status: {:?} | Extracted: {} params | Warnings: {} | Defaults: {}",
            tool_name,
            parameter_extraction.status,
            parameter_extraction.parameters.len(),
            parameter_extraction.warnings.len(),
            parameter_extraction.used_defaults.len()
        );
        
        let help_instructions = format!(
            "To improve parameter extraction for '{}': \n1. Be more specific about values (file names, URLs, search terms) \n2. Use complete sentences describing what you want to do \n3. Include examples or context about your goal \n4. Check the tool's documentation for required parameters",
            tool_name
        );
        
        crate::discovery::types::ErrorDetails {
            technical_details: Some(technical_details),
            diagnostics: Some(diagnostics),
            debug_info: Some(debug_info),
            help_instructions: Some(help_instructions),
        }
    }
    
    /// Extract missing parameter names from error messages
    fn extract_missing_parameters_from_error(&self, error: &str) -> Vec<String> {
        let mut missing_params = Vec::new();
        
        // Look for "Missing required parameters: param1, param2"
        if let Some(start) = error.find("Missing required parameters: ") {
            let params_start = start + "Missing required parameters: ".len();
            if let Some(end) = error[params_start..].find('\n') {
                let params_text = &error[params_start..params_start + end];
                missing_params = params_text
                    .split(',') 
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
            }
        }
        
        missing_params
    }
    
    /// Extract JSON from LLM response, handling markdown code blocks
    fn extract_json_from_response(&self, response: &str) -> String {
        // Try to find JSON between ```json and ``` markers
        if let Some(start) = response.find("```json") {
            let json_start = start + 7; // Skip "```json"
            if let Some(end) = response[json_start..].find("```") {
                let json_end = json_start + end;
                return response[json_start..json_end].trim().to_string();
            }
        }
        
        // Try to find JSON between ``` and ``` markers
        if let Some(start) = response.find("```") {
            let content_start = start + 3;
            if let Some(end) = response[content_start..].find("```") {
                let json_end = content_start + end;
                let content = response[content_start..json_end].trim();
                // Check if it looks like JSON
                if content.starts_with('{') && content.ends_with('}') {
                    return content.to_string();
                }
            }
        }
        
        // Look for JSON object boundaries
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if end > start {
                    return response[start..=end].trim().to_string();
                }
            }
        }
        
        // Return as-is if no JSON structure found
        response.trim().to_string()
    }
    
    /// Semantic-based tool matching using embeddings
    async fn find_matching_tools_semantic(&self, request: &SmartDiscoveryRequest, all_tools: &[(String, ToolDefinition)]) -> Result<Vec<ToolMatch>> {
        let mut matches = Vec::new();
        
        // Check if semantic search is available
        let semantic_search = match &self.semantic_search {
            Some(service) => service,
            None => {
                warn!("Semantic search not available, falling back to rule-based");
                return self.find_matching_tools_rule_based(request, all_tools).await;
            }
        };
        
        // Get semantic matches
        let semantic_matches = semantic_search.search_similar_tools(&request.request).await?;
        
        info!("Found {} semantic matches for request: '{}'", semantic_matches.len(), request.request);
        
        // Convert semantic matches to tool matches
        for semantic_match in semantic_matches {
            // Skip smart_tool_discovery to avoid recursion
            if semantic_match.tool_name == "smart_discovery_tool" || semantic_match.tool_name == "smart_tool_discovery" {
                continue;
            }
            
            if let Some((_, _tool_def)) = all_tools.iter().find(|(name, _)| name == &semantic_match.tool_name) {
                let reasoning = format!("Semantic similarity: {:.3}", semantic_match.similarity_score);
                
                matches.push(ToolMatch {
                    tool_name: semantic_match.tool_name,
                    confidence_score: semantic_match.similarity_score,
                    reasoning,
                    meets_threshold: semantic_match.similarity_score >= self.get_confidence_threshold(request),
                });
            }
        }
        
        // Handle preferred tools if specified
        if let Some(preferred_tools) = &request.preferred_tools {
            for tool_name in preferred_tools {
                // Skip smart_tool_discovery to avoid recursion
                if tool_name == "smart_discovery_tool" || tool_name == "smart_tool_discovery" {
                    continue;
                }
                
                if !matches.iter().any(|m| &m.tool_name == tool_name) {
                    if let Some((_, tool_def)) = all_tools.iter().find(|(name, _)| name == tool_name) {
                        // Use rule-based scoring for preferred tools not found semantically
                        let confidence = self.calculate_confidence_for_tool(tool_def, request);
                        matches.push(ToolMatch {
                            tool_name: tool_name.clone(),
                            confidence_score: confidence,
                            reasoning: format!("Preferred tool (rule-based: {:.3})", confidence),
                            meets_threshold: confidence >= self.get_confidence_threshold(request),
                        });
                    }
                }
            }
        }
        
        // Sort by confidence score (highest first)
        matches.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit to max tools to consider
        matches.truncate(self.config.max_tools_to_consider);
        
        Ok(matches)
    }
    
    /// Hybrid tool matching combining semantic, rule-based, and optionally LLM approaches
    /// Weight distribution: LLM (55%), Semantic (30%), Rule-based (15%)
    async fn find_matching_tools_hybrid(&self, request: &SmartDiscoveryRequest, all_tools: &[(String, ToolDefinition)]) -> Result<Vec<ToolMatch>> {
        info!("Starting hybrid tool matching with {} strategies", 
              if self.config.llm_tool_selection.enabled { 3 } else { 2 });
        
        let mut all_matches: HashMap<String, ToolMatch> = HashMap::new();
        
        // Weighted scoring approach: Each method contributes to a tool's total confidence score
        // Maximum possible score: 1.0 (if all methods give 100% confidence)
        // Run semantic and rule-based evaluation sequentially to ensure both evaluate all tools
        
        info!("Running semantic and rule-based evaluation for {} tools", all_tools.len());
        
        // Run semantic search for all tools - Weight: 30%
        let semantic_result = if let Some(semantic_search) = &self.semantic_search {
            match semantic_search.search_similar_tools(&request.request).await {
                Ok(matches) => {
                    info!("✅ Semantic search completed: {} matches found", matches.len());
                    matches
                }
                Err(e) => {
                    warn!("❌ Semantic search failed: {}", e);
                    Vec::new()
                }
            }
        } else {
            info!("⚠️  Semantic search disabled");
            Vec::new()
        };
        
        // Run rule-based evaluation for all tools - Weight: 15%
        let rule_based_result = match self.find_matching_tools_rule_based(request, all_tools).await {
            Ok(matches) => {
                info!("✅ Rule-based search completed: {} matches found", matches.len());
                matches
            }
            Err(e) => {
                warn!("❌ Rule-based search failed: {}", e);
                Vec::new()
            }
        };
        
        // Process semantic matches
        for semantic_match in semantic_result {
            if let Some((_, _tool_def)) = all_tools.iter().find(|(name, _)| name == &semantic_match.tool_name) {
                let weighted_score = semantic_match.similarity_score * 0.30; // Weight semantic as 30%
                let tool_match = ToolMatch {
                    tool_name: semantic_match.tool_name.clone(),
                    confidence_score: weighted_score,
                    reasoning: format!("Semantic: {:.3}", semantic_match.similarity_score),
                    meets_threshold: false, // Will be recalculated
                };
                let tool_name = semantic_match.tool_name.clone();
                all_matches.insert(semantic_match.tool_name, tool_match);
                debug!("📊 Tool '{}' added via Semantic with score {:.3}", tool_name, weighted_score);
            }
        }
        
        // Process rule-based matches
        for rule_match in rule_based_result {
            let weighted_score = rule_match.confidence_score * 0.15; // Weight rule-based as 15%
            
            if let Some(existing) = all_matches.get_mut(&rule_match.tool_name) {
                // Combine with existing semantic score
                existing.confidence_score += weighted_score;
                existing.reasoning = format!("{}, Rule: {:.3}", existing.reasoning, rule_match.confidence_score);
                debug!("📊 Tool '{}' enhanced via Rule-based, combined score: {:.3}", existing.tool_name, existing.confidence_score);
            } else {
                // Tool only found by rule-based
                let tool_match = ToolMatch {
                    tool_name: rule_match.tool_name.clone(),
                    confidence_score: weighted_score,
                    reasoning: format!("Rule: {:.3}", rule_match.confidence_score),
                    meets_threshold: false, // Will be recalculated
                };
                all_matches.insert(rule_match.tool_name.clone(), tool_match);
                debug!("📊 Tool '{}' added via Rule-based with score {:.3}", rule_match.tool_name, weighted_score);
            }
        }
        
        info!("🔗 Combined scoring complete: {} tools have semantic+rule scores", all_matches.len());
        
        // 3. LLM-based matches (if enabled and available) - Weight: 55%
        if self.config.llm_tool_selection.enabled {
            // Multi-criteria selection: 30 tools total for balanced cost/coverage
            // - 10 from top scorers (best semantic+rule matches)
            // - 5 random sample (discovery of unexpected matches)  
            // - 5 from low scorers (<=0.2, catch tools other methods missed)
            // - 10 from most likely category (focused domain relevance)
            let llm_candidates = self.select_llm_candidates(&request, &all_matches, all_tools).await?;
                
            if !llm_candidates.is_empty() {
                // Convert to the expected format for evaluate_tools_with_llm (&[&(String, ToolDefinition)])
                let llm_candidate_refs: Vec<&(String, ToolDefinition)> = llm_candidates.iter().collect();
                match self.evaluate_tools_with_llm(request, &llm_candidate_refs).await {
                    Ok(llm_matches) => {
                        info!("LLM evaluation found {} matches", llm_matches.len());
                        for llm_match in llm_matches {
                            let weighted_score = llm_match.confidence_score * 0.55; // Weight LLM as 55%
                            
                            if let Some(existing) = all_matches.get_mut(&llm_match.tool_name) {
                                existing.confidence_score += weighted_score;
                                existing.reasoning = format!("{}, LLM: {:.3}", existing.reasoning, llm_match.confidence_score);
                            } else {
                                all_matches.insert(llm_match.tool_name.clone(), ToolMatch {
                                    tool_name: llm_match.tool_name,
                                    confidence_score: weighted_score,
                                    reasoning: format!("LLM: {:.3}", llm_match.confidence_score),
                                    meets_threshold: false, // Will be recalculated
                                });
                            }
                        }
                    }
                    Err(e) => {
                        warn!("LLM evaluation failed: {}", e);
                    }
                }
            }
        }
        
        // Convert to final matches and recalculate threshold compliance
        // Note: Final confidence scores represent weighted combination of all methods
        // Examples with new weights:
        // - Perfect LLM only: 1.0 * 0.55 = 0.55
        // - Perfect Semantic only: 1.0 * 0.30 = 0.30  
        // - Perfect Rule-based only: 1.0 * 0.15 = 0.15
        // - Perfect all methods: 1.0 * 0.55 + 1.0 * 0.30 + 1.0 * 0.15 = 1.0
        let threshold = self.get_confidence_threshold(request);
        let mut final_matches: Vec<ToolMatch> = all_matches.into_values()
            .map(|mut m| {
                m.meets_threshold = m.confidence_score >= threshold;
                m.reasoning = format!("Hybrid({}) = {:.3}", m.reasoning, m.confidence_score);
                m
            })
            .collect();
        
        // Sort by confidence score (highest first)
        final_matches.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit to max tools to consider
        final_matches.truncate(self.config.max_tools_to_consider);
        
        info!("Hybrid matching completed: {} final matches", final_matches.len());
        Ok(final_matches)
    }
    
    /// Select LLM candidates using multi-criteria approach for optimal cost/coverage balance
    /// Returns up to 30 tools: 10 top scorers + 5 random + 5 low scorers + 10 category-matched
    async fn select_llm_candidates(
        &self,
        request: &SmartDiscoveryRequest,
        all_matches: &HashMap<String, ToolMatch>,
        all_tools: &[(String, ToolDefinition)]
    ) -> Result<Vec<(String, ToolDefinition)>> {
        use std::collections::HashSet;
        
        let mut selected = HashSet::new();
        let mut candidates = Vec::new();
        
        // 1. Top 10 scorers from combined semantic+rule results
        let mut sorted_matches: Vec<_> = all_matches.iter()
            .map(|(name, tool_match)| (name.clone(), tool_match.confidence_score))
            .collect();
        sorted_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        for (tool_name, _) in sorted_matches.iter().take(10) {
            if let Some(tool_def) = all_tools.iter().find(|(name, _)| name == tool_name) {
                selected.insert(tool_name.clone());
                candidates.push(tool_def.clone());
            }
        }
        info!("Selected {} top scorers for LLM evaluation", candidates.len());
        
        // 2. 5 diverse tools for discovery (excluding already selected)
        // Use deterministic selection to avoid random dependency
        let unselected_tools: Vec<_> = all_tools.iter()
            .filter(|(name, _)| !selected.contains(name))
            .collect();
        
        if !unselected_tools.is_empty() {
            // Take every Nth tool for diversity instead of random sampling
            let step = (unselected_tools.len() / 5).max(1);
            let diverse_sample = unselected_tools.iter()
                .step_by(step)
                .take(5);
            for &tool_def in diverse_sample {
                selected.insert(tool_def.0.clone());
                candidates.push(tool_def.clone());
            }
        }
        info!("Added {} diverse tools for discovery", std::cmp::min(5, unselected_tools.len()));
        
        // 3. 5 low scorers (<=0.2) to catch tools other methods missed
        let low_scorers: Vec<_> = all_matches.iter()
            .filter(|(name, tool_match)| tool_match.confidence_score <= 0.2 && !selected.contains(*name))
            .take(5)
            .collect();
        let low_scorer_count = low_scorers.len();
            
        for (tool_name, _) in low_scorers {
            if let Some(tool_def) = all_tools.iter().find(|(name, _)| name == tool_name) {
                selected.insert(tool_name.clone());
                candidates.push(tool_def.clone());
            }
        }
        
        // Also include tools that got 0 score from semantic+rule (truly missed tools)
        let zero_scorers: Vec<_> = all_tools.iter()
            .filter(|(name, _)| !all_matches.contains_key(name) && !selected.contains(name))
            .take(5 - low_scorer_count)
            .collect();
        let zero_scorer_count = zero_scorers.len();
            
        for tool_def in zero_scorers {
            selected.insert(tool_def.0.clone());
            candidates.push(tool_def.clone());
        }
        info!("Added {} low/zero scorers to catch missed tools", low_scorer_count + zero_scorer_count);
        
        // 4. 10 tools from most likely category based on request content
        let category_tools = self.select_category_tools(request, all_tools, &selected, 10).await;
        for tool_def in category_tools {
            if !selected.contains(&tool_def.0) {
                selected.insert(tool_def.0.clone());
                candidates.push(tool_def);
            }
        }
        
        info!("Selected total of {} tools for LLM evaluation using multi-criteria approach", candidates.len());
        Ok(candidates)
    }
    
    /// Select tools from the most likely category based on request content
    async fn select_category_tools(
        &self,
        request: &SmartDiscoveryRequest,
        all_tools: &[(String, ToolDefinition)],
        already_selected: &HashSet<String>,
        max_count: usize
    ) -> Vec<(String, ToolDefinition)> {
        // Detect likely categories from request text
        let request_lower = request.request.to_lowercase();
        let categories = self.detect_request_categories(&request_lower);
        
        let mut category_candidates = Vec::new();
        
        let num_categories = categories.len();
        for category in categories {
            let category_tools: Vec<_> = all_tools.iter()
                .filter(|(name, tool_def)| {
                    !already_selected.contains(name) && 
                    self.tool_matches_category(tool_def, &category)
                })
                .take(max_count / num_categories.max(1)) // Distribute across categories
                .collect();
                
            for tool_def in category_tools {
                category_candidates.push(tool_def.clone());
                if category_candidates.len() >= max_count {
                    break;
                }
            }
            
            if category_candidates.len() >= max_count {
                break;
            }
        }
        
        // If we didn't fill quota from categories, add more general tools
        if category_candidates.len() < max_count {
            let remaining_tools: Vec<_> = all_tools.iter()
                .filter(|(name, _)| !already_selected.contains(name))
                .filter(|tool_def| !category_candidates.iter().any(|selected| selected.0 == tool_def.0))
                .take(max_count - category_candidates.len())
                .collect();
                
            for tool_def in remaining_tools {
                category_candidates.push(tool_def.clone());
            }
        }
        
        info!("Selected {} category-relevant tools", category_candidates.len());
        category_candidates
    }
    
    /// Detect likely categories from request text
    fn detect_request_categories(&self, request_lower: &str) -> Vec<&'static str> {
        let mut categories = Vec::new();
        
        // Network/connectivity terms
        if request_lower.contains("ping") || request_lower.contains("network") || 
           request_lower.contains("connectivity") || request_lower.contains("traceroute") ||
           request_lower.contains("dns") || request_lower.contains("http") {
            categories.push("network");
        }
        
        // File/filesystem terms  
        if request_lower.contains("file") || request_lower.contains("directory") ||
           request_lower.contains("read") || request_lower.contains("write") ||
           request_lower.contains("folder") || request_lower.contains("path") {
            categories.push("filesystem");
        }
        
        // Database terms
        if request_lower.contains("database") || request_lower.contains("sql") ||
           request_lower.contains("query") || request_lower.contains("table") ||
           request_lower.contains("postgres") || request_lower.contains("sqlite") {
            categories.push("database");
        }
        
        // Git/version control terms
        if request_lower.contains("git") || request_lower.contains("commit") ||
           request_lower.contains("branch") || request_lower.contains("repository") {
            categories.push("git");
        }
        
        // System/monitoring terms
        if request_lower.contains("system") || request_lower.contains("process") ||
           request_lower.contains("memory") || request_lower.contains("cpu") ||
           request_lower.contains("disk") || request_lower.contains("monitor") {
            categories.push("system");
        }
        
        // Default to general if no specific category detected
        if categories.is_empty() {
            categories.push("general");
        }
        
        categories
    }
    
    /// Check if a tool matches a given category
    fn tool_matches_category(&self, tool_def: &ToolDefinition, category: &str) -> bool {
        let tool_text = format!("{} {}", tool_def.name.to_lowercase(), tool_def.description.to_lowercase());
        
        match category {
            "network" => {
                tool_text.contains("ping") || tool_text.contains("network") || 
                tool_text.contains("connectivity") || tool_text.contains("traceroute") ||
                tool_text.contains("dns") || tool_text.contains("http") || 
                tool_text.contains("globalping") || tool_text.contains("mtr")
            },
            "filesystem" => {
                tool_text.contains("file") || tool_text.contains("directory") ||
                tool_text.contains("read") || tool_text.contains("write") ||
                tool_text.contains("filesystem") || tool_text.contains("path")
            },
            "database" => {
                tool_text.contains("database") || tool_text.contains("sql") ||
                tool_text.contains("query") || tool_text.contains("table") ||
                tool_text.contains("postgres") || tool_text.contains("sqlite")
            },
            "git" => {
                tool_text.contains("git") || tool_text.contains("commit") ||
                tool_text.contains("branch") || tool_text.contains("repository")
            },
            "system" => {
                tool_text.contains("system") || tool_text.contains("process") ||
                tool_text.contains("memory") || tool_text.contains("cpu") ||
                tool_text.contains("disk") || tool_text.contains("monitor") ||
                tool_text.contains("service") || tool_text.contains("check")
            },
            "general" => true, // General category matches everything
            _ => false
        }
    }
    
    /// Generate smart discovery config for tests
    /// Decompose a request into its first executable step
    async fn decompose_into_first_step(&self, request: &SmartDiscoveryRequest) -> Result<Option<SmartDiscoveryRequest>> {
        // Check if user explicitly wants sequential mode or if request appears multi-step
        let should_decompose = request.sequential_mode.unwrap_or(false) || self.is_likely_multi_step(&request.request);
        
        if should_decompose {
            if let Ok(first_step) = self.extract_first_step_with_llm(request).await {
                return Ok(Some(first_step));
            }
        }
        Ok(None)
    }

    /// Check if a request is likely to be multi-step
    fn is_likely_multi_step(&self, request: &str) -> bool {
        let request_lower = request.to_lowercase();
        
        // Keywords that often indicate multi-step workflows
        let multi_step_indicators = vec![
            "then", "after", "next", "also", "and then", "followed by", "once", "when",
            "first", "second", "step", "analyze", "compare", "process", "workflow",
            "create and", "read and", "copy and", "download and", "extract and"
        ];
        
        multi_step_indicators.iter().any(|indicator| request_lower.contains(indicator))
    }

    /// Extract the first step using LLM
    async fn extract_first_step_with_llm(&self, request: &SmartDiscoveryRequest) -> Result<SmartDiscoveryRequest> {
        let prompt = format!(
            r#"Analyze this request and extract the FIRST executable step only:

USER REQUEST: "{}"

INSTRUCTIONS:
1. Identify if this is a multi-step request
2. Extract ONLY the first step that can be executed independently
3. The first step should produce useful output that can inform subsequent steps
4. Return the first step as a clear, actionable request

EXAMPLES:
Request: "Read the config file and then update the database settings"
First Step: "Read the config file"

Request: "Download the logs, analyze errors, and create a summary report"
First Step: "Download the logs"

Request: "List all Python files in the project and then find ones containing 'TODO'"
First Step: "List all Python files in the project"

Extract the first step as a simple, clear request:"#,
            request.request
        );

        // Make a simple LLM call to get the first step
        if let Ok(first_step_text) = self.call_llm_for_first_step(&prompt).await {
            return Ok(SmartDiscoveryRequest {
                request: first_step_text.trim().to_string(),
                context: request.context.clone(),
                preferred_tools: request.preferred_tools.clone(),
                confidence_threshold: request.confidence_threshold,
                include_error_details: request.include_error_details,
                sequential_mode: Some(false), // Don't recurse
            });
        }

        Err(ProxyError::routing("Failed to extract first step".to_string()))
    }


    /// Generate recommendation for the next step
    async fn generate_next_step_recommendation(
        &self, 
        original_request: &SmartDiscoveryRequest,
        completed_step: &SmartDiscoveryRequest, 
        step_result: &SmartDiscoveryResponse
    ) -> Result<Option<NextStepRecommendation>> {
        debug!("🔮 Starting next step recommendation generation");
        debug!("   Original: {}", original_request.request);
        debug!("   Completed: {}", completed_step.request);
        
        let prompt = format!(
            r#"Based on the completed step and its result, recommend the next logical step:

ORIGINAL REQUEST: "{}"
COMPLETED STEP: "{}"
STEP RESULT: {}

INSTRUCTIONS:
1. Analyze what was accomplished in the completed step
2. Determine the next logical step toward fulfilling the original request
3. Suggest specific next action with potential inputs based on the results
4. Provide 2-3 alternative next steps

Respond in JSON format:
{{
  "suggested_request": "Next step description",
  "reasoning": "Why this is the logical next step",
  "potential_inputs": {{"key": "value from results"}},
  "alternatives": ["Alternative 1", "Alternative 2"]
}}"#,
            original_request.request,
            completed_step.request,
            step_result.data.as_ref().map(|d| d.to_string()).unwrap_or_else(|| "No result data".to_string())
        );

        debug!("🤖 Calling LLM for next step recommendation");
        match self.call_llm_for_next_step(&prompt).await {
            Ok(recommendation_json) => {
                debug!("✅ LLM response for next step: {}", recommendation_json);
                
                // Clean up the response (remove markdown code blocks if present)
                let mut cleaned_json = recommendation_json.trim();
                
                // Remove opening markdown block
                if cleaned_json.starts_with("```json") {
                    cleaned_json = &cleaned_json[7..]; // Remove "```json"
                } else if cleaned_json.starts_with("```") {
                    cleaned_json = &cleaned_json[3..]; // Remove "```"
                }
                
                // Remove closing markdown block
                if cleaned_json.ends_with("```") {
                    let len = cleaned_json.len();
                    cleaned_json = &cleaned_json[..len-3]; // Remove trailing "```"
                }
                
                let cleaned_json = cleaned_json.trim();
                
                debug!("🧹 Cleaned JSON: {}", cleaned_json);
                
                match serde_json::from_str::<NextStepRecommendation>(cleaned_json) {
                    Ok(recommendation) => {
                        info!("🎯 Successfully parsed next step recommendation: {}", recommendation.suggested_request);
                        return Ok(Some(recommendation));
                    }
                    Err(e) => {
                        error!("❌ Failed to parse next step recommendation JSON: {}", e);
                        error!("   Raw JSON: {}", recommendation_json);
                        error!("   Cleaned JSON: {}", cleaned_json);
                    }
                }
            }
            Err(e) => {
                error!("❌ LLM call failed for next step recommendation: {}", e);
            }
        }

        Ok(None)
    }

    /// Call LLM for first step extraction
    async fn call_llm_for_first_step(&self, prompt: &str) -> Result<String> {
        // Use the same LLM configuration as parameter extraction
        let config = &self.config.llm_mapper;
        
        if !config.enabled {
            return Err(ProxyError::routing("LLM mapper is disabled".to_string()));
        }
        
        match config.provider.as_str() {
            "openai" | "openai-compatible" => {
                self.call_openai_llm_sequential(prompt, "first_step").await
            }
            "anthropic" => {
                self.call_anthropic_llm_sequential(prompt, "first_step").await
            }
            "ollama" => {
                self.call_ollama_llm_sequential(prompt, "first_step").await
            }
            _ => Err(ProxyError::routing(format!("Unsupported LLM provider: {}", config.provider)))
        }
    }

    /// Call LLM for next step recommendation
    async fn call_llm_for_next_step(&self, prompt: &str) -> Result<String> {
        // Use the same LLM configuration as parameter extraction
        let config = &self.config.llm_mapper;
        
        if !config.enabled {
            return Err(ProxyError::routing("LLM mapper is disabled".to_string()));
        }
        
        match config.provider.as_str() {
            "openai" | "openai-compatible" => {
                self.call_openai_llm_sequential(prompt, "next_step").await
            }
            "anthropic" => {
                self.call_anthropic_llm_sequential(prompt, "next_step").await
            }
            "ollama" => {
                self.call_ollama_llm_sequential(prompt, "next_step").await
            }
            _ => Err(ProxyError::routing(format!("Unsupported LLM provider: {}", config.provider)))
        }
    }

    /// Call OpenAI LLM for sequential operations
    async fn call_openai_llm_sequential(&self, prompt: &str, operation_type: &str) -> Result<String> {
        let config = &self.config.llm_mapper;
        
        let api_key = if let Some(key) = &config.api_key {
            key.clone()
        } else if let Some(env_var) = &config.api_key_env {
            std::env::var(env_var).map_err(|_| ProxyError::routing("API key environment variable not set".to_string()))?
        } else {
            return Err(ProxyError::routing("API key required for OpenAI LLM".to_string()));
        };

        let base_url = config.base_url.as_ref()
            .map(|u| u.clone())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let max_tokens = match operation_type {
            "first_step" => Some(500), // Shorter response for first step
            "next_step" => Some(800),  // Longer for JSON response
            _ => Some(600),
        };

        let request_body = json!({
            "model": config.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.1,
            "max_tokens": max_tokens
        });

        let url = format!("{}/chat/completions", base_url);
        
        debug!("🤖 Calling OpenAI LLM for {}: {}", operation_type, url);
        
        let client = reqwest::Client::new();
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(config.timeout),
            client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
        )
        .await
        .map_err(|_| ProxyError::routing("LLM request timeout".to_string()))?
        .map_err(|e| ProxyError::routing(format!("LLM request failed: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| ProxyError::routing(format!("Failed to read LLM response: {}", e)))?;

        if !status.is_success() {
            return Err(ProxyError::routing(format!(
                "LLM request failed with status {}: {}", status, response_text
            )));
        }

        let openai_response: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| ProxyError::routing(format!("Failed to parse LLM response: {}", e)))?;

        let content = openai_response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ProxyError::routing("No content in LLM response".to_string()))?
            .to_string();

        debug!("🤖 LLM {} response: {}", operation_type, content);
        Ok(content)
    }

    /// Call Anthropic LLM for sequential operations
    async fn call_anthropic_llm_sequential(&self, prompt: &str, operation_type: &str) -> Result<String> {
        let config = &self.config.llm_mapper;
        
        let api_key = if let Some(key) = &config.api_key {
            key.clone()
        } else if let Some(env_var) = &config.api_key_env {
            std::env::var(env_var).map_err(|_| ProxyError::routing("API key environment variable not set".to_string()))?
        } else {
            return Err(ProxyError::routing("API key required for Anthropic LLM".to_string()));
        };

        let base_url = config.base_url.as_ref()
            .map(|u| u.clone())
            .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());

        let max_tokens = match operation_type {
            "first_step" => 500,
            "next_step" => 800,
            _ => 600,
        };

        let request_body = json!({
            "model": config.model,
            "max_tokens": max_tokens,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let url = format!("{}/messages", base_url);
        
        debug!("🤖 Calling Anthropic LLM for {}: {}", operation_type, url);
        
        let client = reqwest::Client::new();
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(config.timeout),
            client
                .post(&url)
                .header("x-api-key", api_key)
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(&request_body)
                .send()
        )
        .await
        .map_err(|_| ProxyError::routing("LLM request timeout".to_string()))?
        .map_err(|e| ProxyError::routing(format!("LLM request failed: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| ProxyError::routing(format!("Failed to read LLM response: {}", e)))?;

        if !status.is_success() {
            return Err(ProxyError::routing(format!(
                "LLM request failed with status {}: {}", status, response_text
            )));
        }

        let anthropic_response: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| ProxyError::routing(format!("Failed to parse LLM response: {}", e)))?;

        let content = anthropic_response["content"][0]["text"]
            .as_str()
            .ok_or_else(|| ProxyError::routing("No content in Anthropic response".to_string()))?
            .to_string();

        debug!("🤖 Anthropic {} response: {}", operation_type, content);
        Ok(content)
    }

    /// Call Ollama LLM for sequential operations
    async fn call_ollama_llm_sequential(&self, prompt: &str, operation_type: &str) -> Result<String> {
        let config = &self.config.llm_mapper;
        
        let base_url = config.base_url.as_ref()
            .map(|u| u.clone())
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        let max_predict = match operation_type {
            "first_step" => 500,
            "next_step" => 800,
            _ => 600,
        };

        let request_body = json!({
            "model": config.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.1,
                "num_predict": max_predict
            }
        });

        let url = format!("{}/api/generate", base_url);
        
        debug!("🤖 Calling Ollama LLM for {}: {}", operation_type, url);
        
        let client = reqwest::Client::new();
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(config.timeout),
            client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
        )
        .await
        .map_err(|_| ProxyError::routing("LLM request timeout".to_string()))?
        .map_err(|e| ProxyError::routing(format!("LLM request failed: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| ProxyError::routing(format!("Failed to read LLM response: {}", e)))?;

        if !status.is_success() {
            return Err(ProxyError::routing(format!(
                "LLM request failed with status {}: {}", status, response_text
            )));
        }

        let ollama_response: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| ProxyError::routing(format!("Failed to parse LLM response: {}", e)))?;

        let content = ollama_response["response"]
            .as_str()
            .ok_or_else(|| ProxyError::routing("No response in Ollama response".to_string()))?
            .to_string();

        debug!("🤖 Ollama {} response: {}", operation_type, content);
        Ok(content)
    }

    #[cfg(test)]
    pub fn create_test_config() -> SmartDiscoveryConfig {
        SmartDiscoveryConfig {
            enabled: true,
            tool_selection_mode: "rule_based".to_string(),
            default_confidence_threshold: 0.7,
            max_tools_to_consider: 10,
            max_high_quality_matches: 5,
            high_quality_threshold: 0.95,
            use_fuzzy_matching: true,
            enable_sampling: Some(false),
            enable_elicitation: Some(false),
            llm_mapper: LlmMapperConfig {
                provider: "mock".to_string(),
                model: "test-model".to_string(),
                api_key: None,
                api_key_env: None,
                base_url: None,
                timeout: 30,
                max_retries: 3,
                enabled: false, // Disable LLM for testing
            },
            llm_tool_selection: LlmToolSelectionConfig {
                enabled: false,
                provider: "mock".to_string(),
                model: "test-model".to_string(),
                api_key: None,
                api_key_env: None,
                base_url: None,
                timeout: 30,
                max_retries: 3,
                batch_size: 5,
                max_context_tokens: 1000,
            },
            cache: DiscoveryCacheConfig::default(),
            fallback: FallbackConfig::default(),
            semantic_search: SemanticSearchConfig::default(),
            enable_sequential_mode: true,
            tool_metrics_enabled: Some(true),
            default_sampling_strategy: Some(crate::config::SamplingElicitationStrategy::MagictunnelHandled),
            default_elicitation_strategy: Some(crate::config::SamplingElicitationStrategy::MagictunnelHandled),
        }
    }
}