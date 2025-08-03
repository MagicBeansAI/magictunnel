//! External Content Manager
//!
//! This service fetches and manages prompts and resources from external MCP servers,
//! ensuring they are properly stored and referenced in capability files while
//! respecting the authority of external servers.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, error};

use crate::error::{ProxyError, Result};
use crate::mcp::types::{PromptTemplate, Resource, ResourceContent, PromptListResponse, ResourceListResponse};
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
        Self {
            config,
            external_mcp_manager,
            content_storage,
            external_content_cache: Arc::new(RwLock::new(HashMap::new())),
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
            prompts,
            resources,
            last_fetched: chrono::Utc::now(),
        });
        debug!("Updated cache for external MCP server: {}", server_name);
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
        }
    }
}