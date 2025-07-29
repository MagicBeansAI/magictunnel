//! OpenAPI Specification Generator
//!
//! Converts MCP tool definitions to OpenAPI 3.0 specifications for Custom GPT integration

use crate::openai::types::OpenApiSpec;
use crate::registry::service::RegistryService;
use crate::registry::types::ToolDefinition;
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// OpenAPI specification generator for MCP tools
pub struct OpenApiGenerator {
    registry: Arc<RegistryService>,
}

impl OpenApiGenerator {
    /// Create a new OpenAPI generator
    pub fn new(registry: Arc<RegistryService>) -> Self {
        Self { registry }
    }

    /// Generate OpenAPI 3.0 specification from all enabled MCP tools
    pub async fn generate_spec(&self) -> anyhow::Result<OpenApiSpec> {
        // Get all tools (both visible and hidden) for Custom GPT integration
        let tools = self.registry.get_all_tools_including_hidden();
        
        let mut spec = OpenApiSpec::new("MagicTunnel API", "1.0.0");
        
        info!("Generating OpenAPI spec for {} tools", tools.len());
        
        for (tool_name, tool_def) in tools {
            debug!("Processing tool: {}", tool_name);
            
            // Only include enabled tools in the OpenAPI spec
            if !tool_def.is_enabled() {
                debug!("Skipping disabled tool: {}", tool_name);
                continue;
            }
            
            // Convert MCP tool to OpenAPI endpoint
            self.add_tool_to_spec(&mut spec, &tool_name, &tool_def)?;
        }
        
        info!("Generated OpenAPI spec with {} endpoints", spec.paths.len());
        Ok(spec)
    }

    /// Add a single MCP tool to the OpenAPI specification
    fn add_tool_to_spec(&self, spec: &mut OpenApiSpec, tool_name: &str, tool_def: &ToolDefinition) -> anyhow::Result<()> {
        debug!("Adding tool to OpenAPI spec: {}", tool_name);
        
        // Convert MCP input schema to OpenAPI request body schema
        let input_schema = self.convert_mcp_schema_to_openapi(&tool_def.input_schema)?;
        
        // Use tool description
        let description = if tool_def.description.trim().is_empty() {
            format!("Execute {} tool", tool_name)
        } else {
            tool_def.description.clone()
        };
        
        spec.add_tool_endpoint(tool_name, &description, &input_schema);
        
        debug!("Successfully added tool {} to OpenAPI spec", tool_name);
        Ok(())
    }

    /// Convert MCP JSON Schema to OpenAPI-compatible schema
    fn convert_mcp_schema_to_openapi(&self, mcp_schema: &Value) -> anyhow::Result<Value> {
        // MCP schemas are already JSON Schema compatible, but we may need some transformations
        let mut openapi_schema = mcp_schema.clone();
        
        // Ensure we have a proper object schema for request body
        if !openapi_schema.is_object() {
            warn!("MCP schema is not an object, wrapping in properties");
            openapi_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "arguments": mcp_schema
                }
            });
        }
        
        // Add required field if not present
        if let Some(obj) = openapi_schema.as_object_mut() {
            if !obj.contains_key("type") {
                obj.insert("type".to_string(), Value::String("object".to_string()));
            }
            
            // Ensure we have properties
            if !obj.contains_key("properties") {
                obj.insert("properties".to_string(), serde_json::json!({}));
            }
        }
        
        debug!("Converted MCP schema to OpenAPI format");
        Ok(openapi_schema)
    }

    /// Generate OpenAPI spec as JSON string
    pub async fn generate_spec_json(&self) -> anyhow::Result<String> {
        let spec = self.generate_spec().await?;
        let json = serde_json::to_string_pretty(&spec)
            .map_err(|e| anyhow::anyhow!("Failed to serialize OpenAPI spec: {}", e))?;
        Ok(json)
    }

    /// Generate OpenAPI 3.1.0 specification for smart tool discovery only
    pub async fn generate_smart_discovery_spec(&self) -> anyhow::Result<OpenApiSpec> {
        info!("Generating OpenAPI 3.1.0 spec for smart tool discovery only");
        
        let spec = OpenApiSpec::new_smart_discovery("MagicTunnel Smart Discovery API", "1.0.0");
        
        info!("Generated smart discovery OpenAPI spec with 1 endpoint");
        Ok(spec)
    }

    /// Generate smart discovery OpenAPI spec as JSON string
    pub async fn generate_smart_discovery_spec_json(&self) -> anyhow::Result<String> {
        let spec = self.generate_smart_discovery_spec().await?;
        let json = serde_json::to_string_pretty(&spec)
            .map_err(|e| anyhow::anyhow!("Failed to serialize smart discovery OpenAPI spec: {}", e))?;
        Ok(json)
    }

    /// Get tools count for validation
    pub async fn get_enabled_tools_count(&self) -> anyhow::Result<usize> {
        let tools = self.registry.get_all_tools_including_hidden();
        let count = tools.iter()
            .filter(|(_, tool_def)| tool_def.is_enabled())
            .count();
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_mcp_schema_conversion() {
        // Create a mock registry for testing
        let config = crate::config::RegistryConfig::default();
        let registry = Arc::new(RegistryService::new(config).await.unwrap());
        let generator = OpenApiGenerator::new(registry);

        // Test simple schema conversion
        let mcp_schema = json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        });

        let result = generator.convert_mcp_schema_to_openapi(&mcp_schema).unwrap();
        assert_eq!(result["type"], "object");
        assert!(result["properties"].is_object());
    }
}