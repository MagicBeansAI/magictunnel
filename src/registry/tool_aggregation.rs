//! Tool Aggregation Service
//! 
//! This module provides a unified service for aggregating tools from multiple sources
//! (local capability files and external MCP servers) and applying conflict resolution.

use crate::config::Config;
use crate::error::Result;
use crate::mcp::external_integration::ExternalMcpIntegration;
use crate::registry::{RegistryService, ToolDefinition};
use crate::routing::{CapabilitySource, ConflictResolver};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Aggregated tool information with source tracking
#[derive(Debug, Clone)]
pub struct AggregatedTool {
    /// Tool name (potentially modified by conflict resolution)
    pub name: String,
    /// Tool definition
    pub definition: ToolDefinition,
    /// Source of the tool
    pub source: CapabilitySource,
    /// Whether this tool was affected by conflict resolution
    pub conflict_resolved: bool,
    /// Original name before conflict resolution (if different)
    pub original_name: Option<String>,
}

/// Service for aggregating tools from multiple sources with conflict resolution
pub struct ToolAggregationService {
    /// Configuration
    config: Arc<Config>,
    /// Registry service for local tools
    registry_service: Option<Arc<RegistryService>>,
    /// External MCP integration for external tools
    external_mcp: Option<Arc<RwLock<ExternalMcpIntegration>>>,
    /// Cached aggregated tools
    cached_tools: RwLock<Option<HashMap<String, AggregatedTool>>>,
}

impl ToolAggregationService {
    /// Create a new tool aggregation service
    pub fn new(config: Arc<Config>) -> Self {
        if let Some(ref cr) = config.conflict_resolution {
            info!("Tool aggregation service will use conflict resolution with strategy: {:?}", cr.strategy);
        } else {
            info!("Tool aggregation service will use tools as-is (no conflict resolution)");
        }

        Self {
            config,
            registry_service: None,
            external_mcp: None,
            cached_tools: RwLock::new(None),
        }
    }

    /// Set the registry service for local tools
    pub fn set_registry_service(&mut self, registry_service: Arc<RegistryService>) {
        self.registry_service = Some(registry_service);
    }

    /// Set the external MCP integration for external tools
    pub fn set_external_mcp(&mut self, external_mcp: Arc<RwLock<ExternalMcpIntegration>>) {
        self.external_mcp = Some(external_mcp);
    }

    /// Get all aggregated tools with conflict resolution applied
    pub async fn get_all_tools(&self) -> Result<HashMap<String, AggregatedTool>> {
        // Check cache first
        {
            let cached = self.cached_tools.read().await;
            if let Some(ref tools) = *cached {
                debug!("Returning cached aggregated tools ({} tools)", tools.len());
                return Ok(tools.clone());
            }
        }

        // Aggregate tools from all sources
        let aggregated_tools = self.aggregate_tools().await?;

        // Cache the result
        {
            let mut cached = self.cached_tools.write().await;
            *cached = Some(aggregated_tools.clone());
        }

        Ok(aggregated_tools)
    }

    /// Get a specific tool by name
    pub async fn get_tool(&self, name: &str) -> Result<Option<AggregatedTool>> {
        let tools = self.get_all_tools().await?;
        Ok(tools.get(name).cloned())
    }

    /// Invalidate the cache (call when tools change)
    pub async fn invalidate_cache(&self) {
        let mut cached = self.cached_tools.write().await;
        *cached = None;
        debug!("Tool aggregation cache invalidated");
    }

    /// Get aggregation statistics
    pub async fn get_stats(&self) -> Result<AggregationStats> {
        let tools = self.get_all_tools().await?;
        
        let mut local_count = 0;
        let mut external_count = 0;
        let mut conflict_resolved_count = 0;

        for tool in tools.values() {
            match &tool.source {
                CapabilitySource::Local => local_count += 1,
                CapabilitySource::Remote { .. } => external_count += 1,
            }
            if tool.conflict_resolved {
                conflict_resolved_count += 1;
            }
        }

        Ok(AggregationStats {
            total_tools: tools.len(),
            local_tools: local_count,
            external_tools: external_count,
            conflict_resolved_tools: conflict_resolved_count,
        })
    }

    /// Aggregate tools from all sources and apply conflict resolution
    async fn aggregate_tools(&self) -> Result<HashMap<String, AggregatedTool>> {
        info!("Aggregating tools from all sources");

        let mut all_tools = Vec::new();

        // Collect local tools
        if let Some(ref registry_service) = self.registry_service {
            let local_tools = self.collect_local_tools(registry_service).await?;
            info!("Collected {} local tools", local_tools.len());
            all_tools.extend(local_tools);
        }

        // Collect external tools
        if let Some(ref external_mcp) = self.external_mcp {
            let external_tools = self.collect_external_tools(external_mcp).await?;
            info!("Collected {} external tools", external_tools.len());
            all_tools.extend(external_tools);
        }

        // Apply conflict resolution if configured
        let resolved_tools = if let Some(ref resolver_config) = self.config.conflict_resolution {
            info!("Applying conflict resolution to {} tools", all_tools.len());
            let mut resolver = ConflictResolver::new(resolver_config.clone());
            resolver.resolve_conflicts(all_tools)?
        } else {
            debug!("No conflict resolution configured, using tools as-is");
            all_tools
        };

        // Convert to aggregated tools
        let mut aggregated_tools = HashMap::new();
        for (name, tool_def, source) in resolved_tools {
            let original_name = tool_def.annotations.as_ref()
                .and_then(|annotations| annotations.get("original_name"))
                .cloned();
            
            let conflict_resolved = tool_def.annotations.as_ref()
                .and_then(|annotations| annotations.get("conflict_resolved"))
                .map(|v| v == "true")
                .unwrap_or(false);

            let aggregated_tool = AggregatedTool {
                name: name.clone(),
                definition: tool_def,
                source,
                conflict_resolved,
                original_name,
            };

            aggregated_tools.insert(name, aggregated_tool);
        }

        info!("Tool aggregation complete: {} tools total", aggregated_tools.len());
        Ok(aggregated_tools)
    }

    /// Collect tools from the registry service (local tools)
    async fn collect_local_tools(
        &self,
        registry_service: &RegistryService,
    ) -> Result<Vec<(String, ToolDefinition, CapabilitySource)>> {
        let local_tools = registry_service.get_all_tools();
        let mut tools = Vec::new();

        for (name, tool_def) in local_tools {
            tools.push((name, tool_def, CapabilitySource::Local));
        }

        Ok(tools)
    }

    /// Collect tools from external MCP integration
    async fn collect_external_tools(
        &self,
        external_mcp: &Arc<RwLock<ExternalMcpIntegration>>,
    ) -> Result<Vec<(String, ToolDefinition, CapabilitySource)>> {
        let integration = external_mcp.read().await;

        // Get tools from external MCP integration
        let external_tools = integration.get_all_tools().await?;
        let mut tools = Vec::new();

        for (server_name, server_tools) in external_tools {
            for tool in server_tools {
                // Convert Tool to ToolDefinition
                let tool_def = self.convert_tool_to_definition(tool)?;
                let source = CapabilitySource::Remote {
                    server_name: server_name.clone()
                };
                tools.push((tool_def.name.clone(), tool_def, source));
            }
        }

        Ok(tools)
    }

    /// Convert MCP Tool to ToolDefinition
    fn convert_tool_to_definition(&self, tool: crate::mcp::types::Tool) -> Result<ToolDefinition> {
        use crate::registry::{RoutingConfig, ToolDefinition};
        use serde_json::json;

        // Create a basic routing config for external MCP tools
        let routing_config = json!({
            "server_name": "external_mcp",
            "tool_name": tool.name.clone()
        });

        let routing = RoutingConfig {
            r#type: "mcp_proxy".to_string(),
            config: routing_config,
        };

        Ok(ToolDefinition {
            name: tool.name.clone(),
            description: tool.description.clone().unwrap_or_else(|| format!("Tool: {}", tool.name)),
            routing,
            input_schema: tool.input_schema,
            annotations: None,
            hidden: false, // Aggregated tools are visible by default
            enabled: true, // Aggregated tools are enabled by default
        })
    }
}

/// Statistics about tool aggregation
#[derive(Debug, Clone)]
pub struct AggregationStats {
    /// Total number of tools after aggregation
    pub total_tools: usize,
    /// Number of local tools
    pub local_tools: usize,
    /// Number of external tools
    pub external_tools: usize,
    /// Number of tools affected by conflict resolution
    pub conflict_resolved_tools: usize,
}
