//! External Content Manager
//!
//! This service fetches and manages prompts and resources from external MCP servers,
//! ensuring they are properly stored and referenced in capability files while
//! respecting the authority of external servers.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{Result};
use crate::mcp::types::{PromptTemplate, Resource, PromptListResponse, ResourceListResponse};
use crate::mcp::external_manager::ExternalMcpManager;
use crate::mcp::content_storage::ContentStorageService;
use crate::registry::types::{PromptReference, ResourceReference, GenerationReferenceMetadata, ToolDefinition};

/// Configuration for external content management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalContentConfig {
    /// Whether to auto-fetch content from external servers
    pub auto_fetch_enabled: bool,
    /// Refresh interval in seconds
    pub refresh_interval_seconds: u64,
    /// Whether to cache external content locally
    pub cache_external_content: bool,
    /// Maximum cache age in hours
    pub max_cache_age_hours: u64,
    /// Whether to save external content to persistent storage (NEW)
    pub save_external_content: bool,
    /// Whether to version saved external content (NEW)
    pub version_external_content: bool,
}

/// External content manager service
pub struct ExternalContentManager {
    config: ExternalContentConfig,
    external_mcp_manager: Option<Arc<ExternalMcpManager>>,
    content_storage: Arc<ContentStorageService>,
    /// Cache of external content to avoid repeated fetches
    external_content_cache: Arc<RwLock<HashMap<String, CachedExternalContent>>>,
}

/// Cached external content with timestamp
#[derive(Debug, Clone)]
struct CachedExternalContent {
    prompts: Vec<PromptTemplate>,
    resources: Vec<Resource>,
    last_fetched: chrono::DateTime<chrono::Utc>,
}

impl ExternalContentManager {
    /// Create a new external content manager
    pub fn new(
        config: ExternalContentConfig,
        external_mcp_manager: Option<Arc<ExternalMcpManager>>,
        content_storage: Arc<ContentStorageService>,
    ) -> Self {
        let manager = Self {
            config,
            external_mcp_manager,
            content_storage,
            external_content_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Start periodic refresh if auto-fetch is enabled
        if manager.config.auto_fetch_enabled {
            manager.start_periodic_refresh();
        }
        
        manager
    }
    
    /// Start periodic refresh of external content (NEW METHOD)
    fn start_periodic_refresh(&self) {
        let config = self.config.clone();
        let external_mcp_manager = self.external_mcp_manager.clone();
        let cache = self.external_content_cache.clone();
        let content_storage = self.content_storage.clone();
        let self_config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(config.refresh_interval_seconds)
            );
            
            loop {
                interval.tick().await;
                
                if let Some(external_manager) = &external_mcp_manager {
                    debug!("🔄 Starting periodic refresh of external MCP content");
                    
                    // Get list of external servers
                    let all_tools = external_manager.get_all_tools().await;
                    
                    for (server_name, _tools) in all_tools {
                        // Fetch fresh content from each server
                        match Self::fetch_content_from_server_static(&external_manager, &server_name).await {
                            Ok((prompts, resources)) => {
                                // Update cache
                                {
                                    let mut cache_guard = cache.write().await;
                                    cache_guard.insert(server_name.clone(), CachedExternalContent {
                                        prompts: prompts.clone(),
                                        resources: resources.clone(),
                                        last_fetched: chrono::Utc::now(),
                                    });
                                }
                                
                                // Save to storage if enabled
                                if self_config.save_external_content {
                                    Self::save_content_to_storage_static(
                                        &content_storage, &server_name, &prompts, &resources
                                    ).await;
                                }
                                
                                debug!("📋 Refreshed {} prompts and {} resources from server '{}'", 
                                       prompts.len(), resources.len(), server_name);
                            }
                            Err(e) => {
                                warn!("Failed to refresh content from external MCP server '{}': {}", server_name, e);
                            }
                        }
                    }
                    
                    info!("✅ Completed periodic refresh of external MCP content");
                } else {
                    debug!("No external MCP manager available for periodic refresh");
                }
            }
        });
        
        info!("🔄 Started periodic refresh of external content every {} seconds", self.config.refresh_interval_seconds);
    }
    
    /// Static method to fetch content from server (for use in async spawn)
    async fn fetch_content_from_server_static(
        external_manager: &Arc<ExternalMcpManager>, 
        server_name: &str
    ) -> crate::error::Result<(Vec<PromptTemplate>, Vec<Resource>)> {
        let prompts = Self::fetch_prompts_from_server_static(external_manager, server_name).await?;
        let resources = Self::fetch_resources_from_server_static(external_manager, server_name).await?;
        Ok((prompts, resources))
    }
    
    /// Static method to fetch prompts from server
    async fn fetch_prompts_from_server_static(
        external_manager: &Arc<ExternalMcpManager>, 
        server_name: &str
    ) -> crate::error::Result<Vec<PromptTemplate>> {
        let response = external_manager.send_request_to_server(
            server_name,
            "prompts/list",
            None
        ).await?;

        if let Some(error) = response.error {
            debug!("External MCP server '{}' returned error for prompts/list: {}", server_name, error.message);
            return Ok(Vec::new());
        }

        let prompts_list = match response.result {
            Some(result) => {
                match serde_json::from_value::<crate::mcp::types::PromptListResponse>(result) {
                    Ok(list) => list.prompts,
                    Err(e) => {
                        warn!("Failed to parse prompts list from external MCP server '{}': {}", server_name, e);
                        return Ok(Vec::new());
                    }
                }
            }
            None => {
                debug!("No prompts result from external MCP server '{}'", server_name);
                return Ok(Vec::new());
            }
        };

        Ok(prompts_list)
    }
    
    /// Static method to fetch resources from server
    async fn fetch_resources_from_server_static(
        external_manager: &Arc<ExternalMcpManager>, 
        server_name: &str
    ) -> crate::error::Result<Vec<Resource>> {
        let response = external_manager.send_request_to_server(
            server_name,
            "resources/list",
            None
        ).await?;

        if let Some(error) = response.error {
            debug!("External MCP server '{}' returned error for resources/list: {}", server_name, error.message);
            return Ok(Vec::new());
        }

        let resources_list = match response.result {
            Some(result) => {
                match serde_json::from_value::<crate::mcp::types::ResourceListResponse>(result) {
                    Ok(list) => list.resources,
                    Err(e) => {
                        warn!("Failed to parse resources list from external MCP server '{}': {}", server_name, e);
                        return Ok(Vec::new());
                    }
                }
            }
            None => {
                debug!("No resources result from external MCP server '{}'", server_name);
                return Ok(Vec::new());
            }
        };

        Ok(resources_list)
    }
    
    /// Static method to save content to storage
    async fn save_content_to_storage_static(
        content_storage: &Arc<ContentStorageService>,
        server_name: &str, 
        prompts: &[PromptTemplate], 
        resources: &[Resource]
    ) {
        // Save prompts
        for prompt in prompts {
            let generation_metadata = Some(GenerationReferenceMetadata {
                model_used: Some(format!("external_mcp_server_{}", server_name)),
                confidence_score: Some(1.0),
                generated_at: Some(chrono::Utc::now().to_rfc3339()),
                generation_time_ms: Some(0),
                version: Some(chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string()),
                external_source: Some(server_name.to_string()),
            });
            
            if let Err(e) = content_storage.store_prompt(
                &format!("external_{}", server_name),
                &prompt.name,
                prompt.clone(),
                format!("External prompt from MCP server: {}", server_name),
                generation_metadata
            ).await {
                warn!("Failed to save external prompt '{}' from server '{}': {}", prompt.name, server_name, e);
            }
        }
        
        // Save resources
        for resource in resources {
            let generation_metadata = Some(GenerationReferenceMetadata {
                model_used: Some(format!("external_mcp_server_{}", server_name)),
                confidence_score: Some(1.0),
                generated_at: Some(chrono::Utc::now().to_rfc3339()),
                generation_time_ms: Some(0),
                version: Some(chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string()),
                external_source: Some(server_name.to_string()),
            });
            
            let resource_content = crate::mcp::types::ResourceContent {
                uri: resource.uri.clone(),
                mime_type: resource.mime_type.clone(),
                text: Some(format!("External resource from MCP server: {}", server_name)),
                blob: None,
            };
            
            if let Err(e) = content_storage.store_resource(
                &format!("external_{}", server_name),
                &resource.name,
                resource.clone(),
                resource_content,
                generation_metadata
            ).await {
                warn!("Failed to save external resource '{}' from server '{}': {}", resource.name, server_name, e);
            }
        }
    }

    /// Fetch and update prompts/resources for a specific tool from external MCP servers
    pub async fn fetch_external_content_for_tool(&self, tool_name: &str) -> Result<(Vec<PromptReference>, Vec<ResourceReference>)> {
        let external_manager = match &self.external_mcp_manager {
            Some(manager) => manager,
            None => {
                debug!("No external MCP manager configured, skipping external content fetch for tool '{}'", tool_name);
                return Ok((Vec::new(), Vec::new()));
            }
        };

        debug!("Fetching external content for tool: {}", tool_name);

        // Check if tool exists in any external MCP server
        let all_tools = external_manager.get_all_tools().await;
        let mut server_with_tool = None;
        
        for (server_name, tools) in &all_tools {
            if tools.iter().any(|tool| tool.name == tool_name) {
                server_with_tool = Some(server_name.clone());
                break;
            }
        }

        let server_name = match server_with_tool {
            Some(name) => name,
            None => {
                debug!("Tool '{}' not found in any external MCP server", tool_name);
                return Ok((Vec::new(), Vec::new()));
            }
        };

        // Check cache first if enabled
        if self.config.cache_external_content {
            if let Some(cached) = self.get_cached_content(&server_name).await {
                if !self.is_cache_stale(&cached) {
                    debug!("Using cached external content for server '{}'", server_name);
                    return self.filter_content_for_tool(&cached, tool_name).await;
                }
            }
        }

        // Fetch fresh content
        let (prompts, resources) = self.fetch_fresh_external_content(&server_name).await?;
        
        // Update cache
        if self.config.cache_external_content {
            self.update_cache(&server_name, prompts.clone(), resources.clone()).await;
        }

        // Filter and convert to references for the specific tool
        let cached_content = CachedExternalContent {
            prompts,
            resources,
            last_fetched: chrono::Utc::now(),
        };
        
        self.filter_content_for_tool(&cached_content, tool_name).await
    }

    /// Fetch and update prompts/resources for all tools from external MCP servers
    pub async fn fetch_all_external_content(&self) -> Result<HashMap<String, (Vec<PromptReference>, Vec<ResourceReference>)>> {
        let external_manager = match &self.external_mcp_manager {
            Some(manager) => manager,
            None => {
                debug!("No external MCP manager configured, skipping external content fetch");
                return Ok(HashMap::new());
            }
        };

        info!("Fetching external content from all MCP servers");
        let mut all_content = HashMap::new();
        let all_tools = external_manager.get_all_tools().await;

        for (server_name, tools) in &all_tools {
            debug!("Fetching content from external MCP server: {}", server_name);
            
            match self.fetch_fresh_external_content(server_name).await {
                Ok((prompts, resources)) => {
                    // Update cache
                    if self.config.cache_external_content {
                        self.update_cache(server_name, prompts.clone(), resources.clone()).await;
                    }

                    // Process each tool from this server
                    for tool in tools {
                        let cached_content = CachedExternalContent {
                            prompts: prompts.clone(),
                            resources: resources.clone(),
                            last_fetched: chrono::Utc::now(),
                        };
                        
                        match self.filter_content_for_tool(&cached_content, &tool.name).await {
                            Ok((tool_prompts, tool_resources)) => {
                                if !tool_prompts.is_empty() || !tool_resources.is_empty() {
                                    all_content.insert(tool.name.clone(), (tool_prompts, tool_resources));
                                    debug!("Fetched content for tool '{}' from server '{}'", tool.name, server_name);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to process content for tool '{}' from server '{}': {}", tool.name, server_name, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch content from external MCP server '{}': {}", server_name, e);
                }
            }
        }

        info!("Fetched external content for {} tools", all_content.len());
        Ok(all_content)
    }

    /// Update tool definition with external content references
    pub async fn update_tool_with_external_content(&self, tool_def: &mut ToolDefinition) -> Result<bool> {
        debug!("Updating tool '{}' with external content", tool_def.name);
        
        let (prompt_refs, resource_refs) = self.fetch_external_content_for_tool(&tool_def.name).await?;
        
        if prompt_refs.is_empty() && resource_refs.is_empty() {
            debug!("No external content found for tool '{}'", tool_def.name);
            return Ok(false);
        }

        // Clear existing external references (keep local ones)
        tool_def.prompt_refs.retain(|p| p.generation_metadata.as_ref()
            .and_then(|m| m.external_source.as_ref()).is_none());
        tool_def.resource_refs.retain(|r| r.generation_metadata.as_ref()
            .and_then(|m| m.external_source.as_ref()).is_none());

        // Add new external references
        tool_def.prompt_refs.extend(prompt_refs);
        tool_def.resource_refs.extend(resource_refs);

        info!("Updated tool '{}' with {} prompts and {} resources from external MCP servers", 
              tool_def.name, tool_def.prompt_refs.len(), tool_def.resource_refs.len());
        
        Ok(true)
    }

    /// Fetch fresh content from a specific external MCP server
    async fn fetch_fresh_external_content(&self, server_name: &str) -> Result<(Vec<PromptTemplate>, Vec<Resource>)> {
        let external_manager = self.external_mcp_manager.as_ref().unwrap(); // Safe since we checked above
        
        debug!("Fetching fresh content from external MCP server: {}", server_name);
        
        // Fetch prompts
        let prompts = match self.fetch_prompts_from_server(external_manager, server_name).await {
            Ok(prompts) => prompts,
            Err(e) => {
                warn!("Failed to fetch prompts from external MCP server '{}': {}", server_name, e);
                Vec::new()
            }
        };

        // Fetch resources
        let resources = match self.fetch_resources_from_server(external_manager, server_name).await {
            Ok(resources) => resources,
            Err(e) => {
                warn!("Failed to fetch resources from external MCP server '{}': {}", server_name, e);
                Vec::new()
            }
        };

        debug!("Fetched {} prompts and {} resources from server '{}'", 
               prompts.len(), resources.len(), server_name);
        
        Ok((prompts, resources))
    }

    /// Fetch prompts from external MCP server
    async fn fetch_prompts_from_server(&self, external_manager: &ExternalMcpManager, server_name: &str) -> Result<Vec<PromptTemplate>> {
        debug!("Fetching prompts from external MCP server: {}", server_name);
        
        let response = external_manager.send_request_to_server(
            server_name,
            "prompts/list",
            None
        ).await?;

        if let Some(error) = response.error {
            debug!("External MCP server '{}' returned error for prompts/list: {}", server_name, error.message);
            return Ok(Vec::new());
        }

        let prompts_list = match response.result {
            Some(result) => {
                match serde_json::from_value::<PromptListResponse>(result) {
                    Ok(list) => list.prompts,
                    Err(e) => {
                        warn!("Failed to parse prompts list from external MCP server '{}': {}", server_name, e);
                        return Ok(Vec::new());
                    }
                }
            }
            None => {
                debug!("No prompts result from external MCP server '{}'", server_name);
                return Ok(Vec::new());
            }
        };

        debug!("Fetched {} prompts from external MCP server '{}'", prompts_list.len(), server_name);
        Ok(prompts_list)
    }

    /// Fetch resources from external MCP server
    async fn fetch_resources_from_server(&self, external_manager: &ExternalMcpManager, server_name: &str) -> Result<Vec<Resource>> {
        debug!("Fetching resources from external MCP server: {}", server_name);
        
        let response = external_manager.send_request_to_server(
            server_name,
            "resources/list",
            None
        ).await?;

        if let Some(error) = response.error {
            debug!("External MCP server '{}' returned error for resources/list: {}", server_name, error.message);
            return Ok(Vec::new());
        }

        let resources_list = match response.result {
            Some(result) => {
                match serde_json::from_value::<ResourceListResponse>(result) {
                    Ok(list) => list.resources,
                    Err(e) => {
                        warn!("Failed to parse resources list from external MCP server '{}': {}", server_name, e);
                        return Ok(Vec::new());
                    }
                }
            }
            None => {
                debug!("No resources result from external MCP server '{}'", server_name);
                return Ok(Vec::new());
            }
        };

        debug!("Fetched {} resources from external MCP server '{}'", resources_list.len(), server_name);
        Ok(resources_list)
    }

    /// Filter content for a specific tool and create references
    async fn filter_content_for_tool(&self, cached_content: &CachedExternalContent, tool_name: &str) -> Result<(Vec<PromptReference>, Vec<ResourceReference>)> {
        let mut prompt_refs = Vec::new();
        let mut resource_refs = Vec::new();

        // Filter and create prompt references
        for prompt in &cached_content.prompts {
            if self.is_content_related_to_tool(&prompt.name, tool_name) ||
               prompt.description.as_ref().map_or(false, |desc| self.is_content_related_to_tool(desc, tool_name)) {
                
                let generation_metadata = GenerationReferenceMetadata {
                    model_used: Some("external_mcp".to_string()),
                    confidence_score: Some(1.0), // External content is authoritative
                    generated_at: Some(cached_content.last_fetched.to_rfc3339()),
                    generation_time_ms: Some(0),
                    version: Some("external".to_string()),
                    external_source: Some(format!("external_mcp_server")), // Mark as external
                };

                let prompt_ref = PromptReference {
                    name: prompt.name.clone(),
                    prompt_type: "external".to_string(),
                    description: prompt.description.clone(),
                    storage_path: None, // External content not stored locally
                    generation_metadata: Some(generation_metadata),
                };

                prompt_refs.push(prompt_ref);
            }
        }

        // Filter and create resource references
        for resource in &cached_content.resources {
            if self.is_content_related_to_tool(&resource.name, tool_name) ||
               resource.description.as_ref().map_or(false, |desc| self.is_content_related_to_tool(desc, tool_name)) ||
               self.is_content_related_to_tool(&resource.uri, tool_name) {
                
                let generation_metadata = GenerationReferenceMetadata {
                    model_used: Some("external_mcp".to_string()),
                    confidence_score: Some(1.0), // External content is authoritative
                    generated_at: Some(cached_content.last_fetched.to_rfc3339()),
                    generation_time_ms: Some(0),
                    version: Some("external".to_string()),
                    external_source: Some(format!("external_mcp_server")), // Mark as external
                };

                let resource_ref = ResourceReference {
                    name: resource.name.clone(),
                    resource_type: "external".to_string(),
                    uri: resource.uri.clone(),
                    mime_type: resource.mime_type.clone(),
                    description: resource.description.clone(),
                    storage_path: None, // External content not stored locally
                    generation_metadata: Some(generation_metadata),
                };

                resource_refs.push(resource_ref);
            }
        }

        debug!("Filtered {} prompts and {} resources for tool '{}'", 
               prompt_refs.len(), resource_refs.len(), tool_name);
        
        Ok((prompt_refs, resource_refs))
    }

    /// Check if content is related to a tool
    fn is_content_related_to_tool(&self, content_name: &str, tool_name: &str) -> bool {
        content_name.contains(tool_name) ||
        content_name.to_lowercase().contains(&tool_name.to_lowercase()) ||
        tool_name.contains(content_name) ||
        tool_name.to_lowercase().contains(&content_name.to_lowercase())
    }

    /// Get cached content for a server
    async fn get_cached_content(&self, server_name: &str) -> Option<CachedExternalContent> {
        let cache = self.external_content_cache.read().await;
        cache.get(server_name).cloned()
    }

    /// Check if cached content is stale
    fn is_cache_stale(&self, cached: &CachedExternalContent) -> bool {
        let max_age = chrono::Duration::hours(self.config.max_cache_age_hours as i64);
        chrono::Utc::now() - cached.last_fetched > max_age
    }

    /// Update cache with fresh content
    async fn update_cache(&self, server_name: &str, prompts: Vec<PromptTemplate>, resources: Vec<Resource>) {
        let mut cache = self.external_content_cache.write().await;
        cache.insert(server_name.to_string(), CachedExternalContent {
            prompts: prompts.clone(),
            resources: resources.clone(),
            last_fetched: chrono::Utc::now(),
        });
        debug!("Updated cache for external MCP server: {}", server_name);
        
        // NEW: Save to persistent storage if enabled
        if self.config.save_external_content {
            self.save_external_content_to_storage(server_name, &prompts, &resources).await;
        }
    }
    
    /// Save external content to persistent storage with versioning (NEW METHOD)
    async fn save_external_content_to_storage(&self, server_name: &str, prompts: &[PromptTemplate], resources: &[Resource]) {
        // Save prompts
        for prompt in prompts {
            let generation_metadata = Some(GenerationReferenceMetadata {
                model_used: Some(format!("external_mcp_server_{}", server_name)),
                confidence_score: Some(1.0), // External content is authoritative
                generated_at: Some(chrono::Utc::now().to_rfc3339()),
                generation_time_ms: Some(0), // No generation time for external content
                version: Some(chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string()),
                external_source: Some(server_name.to_string()),
            });
            
            if let Err(e) = self.content_storage.store_prompt(
                &format!("external_{}", server_name),
                &prompt.name,
                prompt.clone(),
                format!("External prompt from MCP server: {}", server_name),
                generation_metadata
            ).await {
                warn!("Failed to save external prompt '{}' from server '{}': {}", prompt.name, server_name, e);
            } else {
                debug!("Saved external prompt '{}' from server '{}'", prompt.name, server_name);
            }
        }
        
        // Save resources
        for resource in resources {
            let generation_metadata = Some(GenerationReferenceMetadata {
                model_used: Some(format!("external_mcp_server_{}", server_name)),
                confidence_score: Some(1.0), // External content is authoritative
                generated_at: Some(chrono::Utc::now().to_rfc3339()),
                generation_time_ms: Some(0), // No generation time for external content
                version: Some(chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string()),
                external_source: Some(server_name.to_string()),
            });
            
            // Create dummy resource content since we're just storing metadata
            let resource_content = crate::mcp::types::ResourceContent {
                uri: resource.uri.clone(),
                mime_type: resource.mime_type.clone(),
                text: Some(format!("External resource from MCP server: {}", server_name)),
                blob: None,
            };
            
            if let Err(e) = self.content_storage.store_resource(
                &format!("external_{}", server_name),
                &resource.name,
                resource.clone(),
                resource_content,
                generation_metadata
            ).await {
                warn!("Failed to save external resource '{}' from server '{}': {}", resource.name, server_name, e);
            } else {
                debug!("Saved external resource '{}' from server '{}'", resource.name, server_name);
            }
        }
        
        info!("Saved {} prompts and {} resources from external MCP server '{}' to persistent storage", 
              prompts.len(), resources.len(), server_name);
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.external_content_cache.write().await;
        cache.clear();
        info!("Cleared external content cache");
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> HashMap<String, (usize, usize, String)> {
        let cache = self.external_content_cache.read().await;
        let mut stats = HashMap::new();
        
        for (server_name, cached_content) in cache.iter() {
            stats.insert(
                server_name.clone(),
                (
                    cached_content.prompts.len(),
                    cached_content.resources.len(),
                    cached_content.last_fetched.to_rfc3339(),
                )
            );
        }
        
        stats
    }
}

impl Default for ExternalContentConfig {
    fn default() -> Self {
        Self {
            auto_fetch_enabled: true,
            refresh_interval_seconds: 3600, // 1 hour
            cache_external_content: true,
            max_cache_age_hours: 24, // 24 hours
            save_external_content: true, // NEW: Save to persistent storage
            version_external_content: true, // NEW: Version the saved content
        }
    }
}