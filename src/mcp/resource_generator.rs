//! Resource Generation Service
//!
//! This module provides server-side resource generation for tools, APIs, and capabilities.
//! Similar to sampling/elicitation, it uses LLM providers to generate contextual resources
//! like documentation, examples, schemas, and configuration files.

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, error};

use crate::error::{ProxyError, Result};
use crate::registry::types::ToolDefinition;
use crate::mcp::types::{Resource, ResourceContent, ResourceAnnotations};

/// Generation metadata for tracking request generation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Model used for generation
    pub model_used: Option<String>,
    /// Confidence score of the generation
    pub confidence_score: Option<f32>,
    /// Time taken for generation in milliseconds
    pub generation_time_ms: u64,
}
use crate::mcp::external_manager::ExternalMcpManager;

/// Configuration for resource generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGenerationConfig {
    /// LLM provider configurations
    pub providers: Vec<crate::mcp::sampling::LLMProviderConfig>,
    /// Default model to use for generation
    pub default_model: String,
    /// Maximum number of resources to generate per tool
    pub max_resources_per_tool: usize,
    /// Whether to include documentation resources
    pub include_documentation: bool,
    /// Whether to include example resources
    pub include_examples: bool,
    /// Whether to include schema resources
    pub include_schemas: bool,
    /// Whether to include configuration resources
    pub include_configuration: bool,
    /// Custom resource template for generation
    pub generation_template: Option<String>,
}

impl Default for ResourceGenerationConfig {
    fn default() -> Self {
        Self {
            providers: vec![], // Will be populated from main config
            default_model: "default".to_string(), // Should be overridden in config
            max_resources_per_tool: 5,
            include_documentation: true,
            include_examples: true,
            include_schemas: true,
            include_configuration: false,
            generation_template: None,
        }
    }
}

/// Generated resource with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedResource {
    /// The generated resource
    pub resource: Resource,
    /// Content of the resource
    pub content: ResourceContent,
    /// Generation metadata
    pub metadata: GenerationMetadata,
    /// Confidence score for the generated resource
    pub confidence: f32,
}

/// Request for resource generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGenerationRequest {
    /// Tool to generate resources for
    pub tool_name: String,
    /// Tool definition
    pub tool_definition: ToolDefinition,
    /// Types of resources to generate
    pub resource_types: Vec<ResourceType>,
    /// Generation configuration
    pub config: ResourceGenerationConfig,
}

/// Types of resources that can be generated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceType {
    /// API documentation
    Documentation,
    /// Usage examples
    Examples,
    /// JSON Schema definitions
    Schema,
    /// Configuration templates
    Configuration,
    /// OpenAPI specification
    OpenAPI,
    /// Test cases
    TestCases,
    /// Custom resource type
    Custom(String),
}

/// Response from resource generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGenerationResponse {
    /// Whether generation was successful
    pub success: bool,
    /// Generated resources
    pub resources: Vec<GeneratedResource>,
    /// Error message if generation failed
    pub error: Option<String>,
    /// Overall generation metadata
    pub metadata: GenerationMetadata,
}

/// Resource generation service
pub struct ResourceGeneratorService {
    /// Configuration for resource generation
    config: ResourceGenerationConfig,
    /// HTTP client for LLM API calls
    http_client: reqwest::Client,
    /// External MCP manager for fetching resources from external servers
    external_mcp_manager: Option<Arc<ExternalMcpManager>>,
}

impl ResourceGeneratorService {
    /// Create a new resource generator service
    pub fn new(
        config: ResourceGenerationConfig,
        external_mcp_manager: Option<Arc<ExternalMcpManager>>,
    ) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProxyError::connection(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            http_client,
            external_mcp_manager,
        })
    }

    /// Generate resources for a tool
    pub async fn generate_resources(&self, request: ResourceGenerationRequest) -> Result<ResourceGenerationResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Generating resources for tool: {}", request.tool_name);
        debug!("Resource types requested: {:?}", request.resource_types);
        
        // Step 1: Check if tool is from external MCP server and fetch existing resources
        if let Some(external_resources) = self.fetch_external_resources(&request.tool_name).await? {
            info!("Found {} existing resources from external MCP server for tool '{}'", external_resources.len(), request.tool_name);
            
            let metadata = GenerationMetadata {
                model_used: Some("external_mcp_server".to_string()),
                confidence_score: Some(1.0), // External resources are authoritative
                generation_time_ms: start_time.elapsed().as_millis() as u64,
            };
            
            return Ok(ResourceGenerationResponse {
                success: true,
                resources: external_resources,
                error: None,
                metadata,
            });
        }

        let mut generated_resources = Vec::new();
        let mut generation_errors = Vec::new();

        // Generate each requested resource type
        for resource_type in &request.resource_types {
            match self.generate_single_resource(&request, resource_type).await {
                Ok(resource) => {
                    debug!("Successfully generated {} resource for tool: {}", 
                           resource_type_to_string(resource_type), request.tool_name);
                    generated_resources.push(resource);
                }
                Err(e) => {
                    warn!("Failed to generate {} resource for tool '{}': {}", 
                          resource_type_to_string(resource_type), request.tool_name, e);
                    generation_errors.push(format!("{}: {}", resource_type_to_string(resource_type), e));
                }
            }
        }

        let generation_time_ms = start_time.elapsed().as_millis() as u64;
        let success = !generated_resources.is_empty();

        let metadata = GenerationMetadata {
            model_used: Some(self.get_model_name()),
            confidence_score: if success { 
                Some(generated_resources.iter().map(|r| r.confidence).sum::<f32>() / generated_resources.len() as f32)
            } else { 
                None 
            },
            generation_time_ms,
        };

        let response = ResourceGenerationResponse {
            success,
            resources: generated_resources,
            error: if generation_errors.is_empty() {
                None
            } else {
                Some(generation_errors.join("; "))
            },
            metadata,
        };

        if response.success {
            info!("Successfully generated {} resources for tool '{}' in {}ms", 
                  response.resources.len(), request.tool_name, generation_time_ms);
        } else {
            error!("Failed to generate resources for tool '{}': {}", 
                   request.tool_name, response.error.as_deref().unwrap_or("Unknown error"));
        }

        Ok(response)
    }

    /// Generate a single resource for a specific type
    async fn generate_single_resource(&self, request: &ResourceGenerationRequest, resource_type: &ResourceType) -> Result<GeneratedResource> {
        let system_prompt = self.create_system_prompt(resource_type);
        let user_prompt = self.create_user_prompt(request, resource_type);

        // Create sampling request for LLM
        let sampling_request = crate::mcp::types::sampling::SamplingRequest {
            messages: vec![
                crate::mcp::types::sampling::SamplingMessage {
                    role: crate::mcp::types::sampling::SamplingMessageRole::System,
                    content: crate::mcp::types::sampling::SamplingContent::Text(system_prompt),
                    name: None,
                    metadata: None,
                },
                crate::mcp::types::sampling::SamplingMessage {
                    role: crate::mcp::types::sampling::SamplingMessageRole::User,
                    content: crate::mcp::types::sampling::SamplingContent::Text(user_prompt),
                    name: None,
                    metadata: None,
                }
            ],
            model_preferences: Some(crate::mcp::types::sampling::ModelPreferences {
                intelligence: Some(0.8),
                speed: Some(0.6),
                cost: Some(0.7),
                preferred_models: None,
                excluded_models: None,
            }),
            system_prompt: None,
            max_tokens: Some(2000), // More tokens for resource content
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop: None,
            metadata: None,
        };

        // Send request to LLM provider
        let llm_response = self.call_llm_provider(&sampling_request).await?;

        // Extract content from LLM response
        let content = match &llm_response.message.content {
            crate::mcp::types::sampling::SamplingContent::Text(text) => text.clone(),
            _ => return Err(ProxyError::validation("Expected text response from LLM".to_string())),
        };

        // Create resource from generated content
        let (resource, resource_content) = self.create_resource_from_content(&content, &request.tool_name, resource_type)?;

        // Calculate confidence based on response quality
        let confidence = self.calculate_resource_confidence(&llm_response, &content, resource_type);

        let metadata = GenerationMetadata {
            model_used: Some(llm_response.model),
            confidence_score: Some(confidence),
            generation_time_ms: 0, // Will be set by caller
        };

        Ok(GeneratedResource {
            resource,
            content: resource_content,
            metadata,
            confidence,
        })
    }

    /// Create system prompt for resource generation
    fn create_system_prompt(&self, resource_type: &ResourceType) -> String {
        match resource_type {
            ResourceType::Documentation => {
                "You are an expert technical writer. Create comprehensive documentation for the given API tool. \
                Include purpose, parameters, return values, and usage guidelines. \
                Format as markdown with clear sections and examples.".to_string()
            }
            ResourceType::Examples => {
                "You are an expert at creating practical code examples. \
                Generate realistic usage examples for the given API tool. \
                Include different scenarios, parameter combinations, and expected outputs. \
                Format as code blocks with explanations.".to_string()
            }
            ResourceType::Schema => {
                "You are an expert at creating JSON Schema definitions. \
                Generate a comprehensive JSON Schema that validates the tool's input and output. \
                Include all properties, types, constraints, and descriptions. \
                Format as valid JSON Schema.".to_string()
            }
            ResourceType::Configuration => {
                "You are an expert at creating configuration templates. \
                Generate configuration files or templates needed to use the tool effectively. \
                Include all necessary settings, parameters, and explanations. \
                Format as YAML or JSON configuration.".to_string()
            }
            ResourceType::OpenAPI => {
                "You are an expert at creating OpenAPI specifications. \
                Generate a complete OpenAPI 3.0 specification for the given tool. \
                Include paths, parameters, responses, and schemas. \
                Format as valid OpenAPI YAML.".to_string()
            }
            ResourceType::TestCases => {
                "You are an expert at creating test cases. \
                Generate comprehensive test cases for the given tool. \
                Include positive and negative test scenarios, edge cases, and expected results. \
                Format as structured test specifications.".to_string()
            }
            ResourceType::Custom(description) => {
                format!("You are an expert at creating custom resources. \
                Create a resource for: {}. \
                Make it practical, well-formatted, and useful for developers.", description)
            }
        }
    }

    /// Create user prompt with tool context
    fn create_user_prompt(&self, request: &ResourceGenerationRequest, resource_type: &ResourceType) -> String {
        let tool_def = &request.tool_definition;
        let parameters = serde_json::to_string_pretty(&tool_def.input_schema).unwrap_or_default();
        
        format!(
            "Tool Name: {}\n\
            Description: {}\n\
            Parameters Schema: {}\n\
            Routing Type: {}\n\
            Routing Config: {}\n\n\
            Create a {} resource for this tool. \
            The resource should be comprehensive, practical, and well-formatted. \
            Return only the resource content, no explanation or metadata.",
            tool_def.name,
            tool_def.description,
            parameters,
            tool_def.routing.r#type,
            serde_json::to_string_pretty(&tool_def.routing.config).unwrap_or_default(),
            resource_type_to_string(resource_type).to_lowercase()
        )
    }

    /// Create resource and content from generated text
    fn create_resource_from_content(&self, content: &str, tool_name: &str, resource_type: &ResourceType) -> Result<(Resource, ResourceContent)> {
        let file_extension = self.get_file_extension(resource_type);
        let mime_type = self.get_mime_type(resource_type);
        
        let resource_name = format!("{}_{}", tool_name, resource_type_to_string(resource_type).to_lowercase());
        let uri = format!("generated://{}.{}", resource_name, file_extension);
        
        // Create resource annotations
        let annotations = ResourceAnnotations::new()
            .with_size(content.len() as u64)
            .with_last_modified(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .with_generated(true)
            .with_tool_name(tool_name.to_string())
            .with_resource_type(resource_type_to_string(resource_type).to_string());

        // Create resource
        let resource = Resource::complete(
            uri.clone(),
            format!("{} {}", resource_type_to_string(resource_type), tool_name),
            Some(format!("Generated {} for {}", resource_type_to_string(resource_type).to_lowercase(), tool_name)),
            mime_type.clone(),
            Some(annotations),
        );

        // Create resource content
        let resource_content = ResourceContent::text(uri, content.to_string(), mime_type);

        Ok((resource, resource_content))
    }

    /// Get file extension for resource type
    fn get_file_extension(&self, resource_type: &ResourceType) -> &str {
        match resource_type {
            ResourceType::Documentation => "md",
            ResourceType::Examples => "md",
            ResourceType::Schema => "json",
            ResourceType::Configuration => "yaml",
            ResourceType::OpenAPI => "yaml",
            ResourceType::TestCases => "md",
            ResourceType::Custom(_) => "txt",
        }
    }

    /// Get MIME type for resource type
    fn get_mime_type(&self, resource_type: &ResourceType) -> Option<String> {
        Some(match resource_type {
            ResourceType::Documentation => "text/markdown",
            ResourceType::Examples => "text/markdown",
            ResourceType::Schema => "application/json",
            ResourceType::Configuration => "application/yaml",
            ResourceType::OpenAPI => "application/yaml",
            ResourceType::TestCases => "text/markdown",
            ResourceType::Custom(_) => "text/plain",
        }.to_string())
    }

    /// Calculate confidence score for generated resource
    fn calculate_resource_confidence(&self, response: &crate::mcp::types::sampling::SamplingResponse, content: &str, resource_type: &ResourceType) -> f32 {
        let mut confidence: f32 = 0.8; // Base confidence

        // Adjust based on stop reason
        match response.stop_reason {
            crate::mcp::types::sampling::SamplingStopReason::EndTurn => confidence += 0.1,
            crate::mcp::types::sampling::SamplingStopReason::MaxTokens => confidence -= 0.2,
            _ => confidence -= 0.1,
        }

        // Adjust based on content quality indicators
        match resource_type {
            ResourceType::Schema => {
                if content.contains("\"type\":") && content.contains("\"properties\":") {
                    confidence += 0.15; // Valid JSON schema structure
                }
            }
            ResourceType::OpenAPI => {
                if content.contains("openapi:") && content.contains("paths:") {
                    confidence += 0.15; // Valid OpenAPI structure
                }
            }
            ResourceType::Documentation | ResourceType::Examples => {
                if content.contains("#") && content.contains("```") {
                    confidence += 0.1; // Has markdown formatting
                }
            }
            _ => {}
        }
        
        if content.len() > 100 && content.len() < 10000 {
            confidence += 0.05; // Good length
        } else {
            confidence -= 0.1; // Too short or too long
        }

        // Clamp between 0.0 and 1.0
        confidence.max(0.0f32).min(1.0f32)
    }

    /// Get the model name for the current provider
    fn get_model_name(&self) -> String {
        self.config.default_model.clone()
    }
    
    /// Fetch resources from external MCP servers for the given tool
    async fn fetch_external_resources(&self, tool_name: &str) -> Result<Option<Vec<GeneratedResource>>> {
        let external_manager = match &self.external_mcp_manager {
            Some(manager) => manager,
            None => {
                debug!("No external MCP manager configured, skipping external resource fetch");
                return Ok(None);
            }
        };
        
        debug!("Checking external MCP servers for resources for tool: {}", tool_name);
        
        // Check if tool exists in any external MCP server
        let all_tools = external_manager.get_all_tools().await;
        
        // Find which server(s) provide this tool
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
                debug!("Tool '{}' not found in any external MCP server, proceeding with LLM generation", tool_name);
                return Ok(None);
            }
        };
        
        info!("Tool '{}' found in external MCP server '{}', fetching resources", tool_name, server_name);
        
        // Fetch resources from the external server
        match self.fetch_resources_from_server(&server_name, tool_name).await {
            Ok(resources) => {
                if resources.is_empty() {
                    info!("No resources found for tool '{}' in external MCP server '{}'", tool_name, server_name);
                    Ok(None)
                } else {
                    info!("Successfully fetched {} resources for tool '{}' from external MCP server '{}'", resources.len(), tool_name, server_name);
                    Ok(Some(resources))
                }
            }
            Err(e) => {
                warn!("Failed to fetch resources for tool '{}' from external MCP server '{}': {}", tool_name, server_name, e);
                Ok(None)
            }
        }
    }
    
    /// Fetch resources from a specific external MCP server
    async fn fetch_resources_from_server(&self, server_name: &str, tool_name: &str) -> Result<Vec<GeneratedResource>> {
        let external_manager = self.external_mcp_manager.as_ref().unwrap(); // Safe since we checked above
        
        debug!("Fetching resources from external MCP server '{}' for tool '{}'", server_name, tool_name);
        
        // Try to get resources from the external server using resources/list
        let request_result = external_manager.send_request_to_server(
            server_name,
            "resources/list",
            Some(serde_json::json!({
                "cursor": null
            }))
        ).await;
        
        let response = match request_result {
            Ok(response) => response,
            Err(e) => {
                debug!("External MCP server '{}' doesn't support resources/list: {}", server_name, e);
                return Ok(Vec::new());
            }
        };
        
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
        
        // Filter resources related to the tool and fetch their content
        let mut generated_resources = Vec::new();
        
        for resource in resources_list {
            // Check if resource is related to the tool (by name, description, or URI)
            let is_tool_related = resource.name.contains(tool_name) ||
                resource.description.as_ref().map_or(false, |desc| desc.contains(tool_name)) ||
                resource.uri.contains(tool_name);
            
            if !is_tool_related {
                continue;
            }
            
            // Fetch the resource content
            let resource_content = match self.fetch_resource_content_from_server(server_name, &resource.uri).await {
                Ok(content) => content,
                Err(e) => {
                    warn!("Failed to fetch content for resource '{}' from external MCP server '{}': {}", resource.uri, server_name, e);
                    continue;
                }
            };
            
            let generated_resource = GeneratedResource {
                resource,
                content: resource_content,
                metadata: GenerationMetadata {
                    model_used: Some(format!("external_mcp_server:{}", server_name)),
                    confidence_score: Some(1.0), // External resources are authoritative
                    generation_time_ms: 0,
                },
                confidence: 1.0, // External resources are authoritative
            };
            
            generated_resources.push(generated_resource);
        }
        
        Ok(generated_resources)
    }
    
    /// Fetch resource content from external MCP server
    async fn fetch_resource_content_from_server(&self, server_name: &str, resource_uri: &str) -> Result<ResourceContent> {
        let external_manager = self.external_mcp_manager.as_ref().unwrap(); // Safe since we checked above
        
        debug!("Fetching resource content for '{}' from external MCP server '{}'", resource_uri, server_name);
        
        let request_result = external_manager.send_request_to_server(
            server_name,
            "resources/read",
            Some(serde_json::json!({
                "uri": resource_uri
            }))
        ).await;
        
        let response = match request_result {
            Ok(response) => response,
            Err(e) => {
                return Err(ProxyError::mcp(format!("Failed to read resource '{}' from external MCP server '{}': {}", resource_uri, server_name, e)));
            }
        };
        
        if let Some(error) = response.error {
            return Err(ProxyError::mcp(format!("External MCP server '{}' returned error for resource '{}': {}", server_name, resource_uri, error.message)));
        }
        
        let resource_content = match response.result {
            Some(result) => {
                match serde_json::from_value::<ResourceContent>(result) {
                    Ok(content) => content,
                    Err(e) => {
                        return Err(ProxyError::mcp(format!("Failed to parse resource content from external MCP server '{}': {}", server_name, e)));
                    }
                }
            }
            None => {
                return Err(ProxyError::mcp(format!("No resource content from external MCP server '{}'", server_name)));
            }
        };
        
        debug!("Successfully fetched resource content for '{}' from external MCP server '{}'", resource_uri, server_name);
        
        Ok(resource_content)
    }
    
    /// Call LLM provider with sampling request
    async fn call_llm_provider(&self, sampling_request: &crate::mcp::types::sampling::SamplingRequest) -> Result<crate::mcp::types::sampling::SamplingResponse> {
        // Find provider for the default model
        let provider = self.config.providers.iter()
            .find(|p| p.models.contains(&self.config.default_model))
            .ok_or_else(|| ProxyError::validation(format!("No provider found for model: {}", self.config.default_model)))?;
        
        match provider.provider_type {
            crate::mcp::sampling::LLMProviderType::OpenAI => {
                self.call_openai_api(sampling_request, provider).await
            }
            crate::mcp::sampling::LLMProviderType::Anthropic => {
                self.call_anthropic_api(sampling_request, provider).await
            }
            crate::mcp::sampling::LLMProviderType::Ollama => {
                self.call_ollama_api(sampling_request, provider).await
            }
            crate::mcp::sampling::LLMProviderType::Custom => {
                Err(ProxyError::validation("Custom LLM provider not yet implemented".to_string()))
            }
        }
    }
    
    /// Call OpenAI API
    async fn call_openai_api(&self, sampling_request: &crate::mcp::types::sampling::SamplingRequest, provider: &crate::mcp::sampling::LLMProviderConfig) -> Result<crate::mcp::types::sampling::SamplingResponse> {
        let api_key = provider.api_key.as_ref()
            .ok_or_else(|| ProxyError::validation("No API key configured for OpenAI provider".to_string()))?;
        
        // Extract messages from sampling request
        let mut openai_messages = Vec::new();
        for msg in &sampling_request.messages {
            let content = match &msg.content {
                crate::mcp::types::sampling::SamplingContent::Text(text) => text.clone(),
                _ => continue, // Skip non-text messages for now
            };
            
            let role = match msg.role {
                crate::mcp::types::sampling::SamplingMessageRole::System => "system",
                crate::mcp::types::sampling::SamplingMessageRole::User => "user",
                crate::mcp::types::sampling::SamplingMessageRole::Assistant => "assistant",
                crate::mcp::types::sampling::SamplingMessageRole::Tool => "user", // Treat tool messages as user messages
            };
            
            openai_messages.push(serde_json::json!({
                "role": role,
                "content": content
            }));
        }
        
        let request_body = serde_json::json!({
            "model": self.config.default_model,
            "messages": openai_messages,
            "max_tokens": sampling_request.max_tokens.unwrap_or(2000), // More tokens for resource content
            "temperature": sampling_request.temperature.unwrap_or(0.7),
            "top_p": sampling_request.top_p.unwrap_or(0.9)
        });
        
        let response = self.http_client
            .post(&provider.endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("OpenAI API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProxyError::connection(format!("OpenAI API error: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ProxyError::validation(format!("Failed to parse OpenAI response: {}", e)))?;
        
        // Extract content from OpenAI response
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        let stop_reason = match response_json["choices"][0]["finish_reason"].as_str() {
            Some("stop") => crate::mcp::types::sampling::SamplingStopReason::EndTurn,
            Some("length") => crate::mcp::types::sampling::SamplingStopReason::MaxTokens,
            _ => crate::mcp::types::sampling::SamplingStopReason::EndTurn,
        };
        
        Ok(crate::mcp::types::sampling::SamplingResponse {
            message: crate::mcp::types::sampling::SamplingMessage {
                role: crate::mcp::types::sampling::SamplingMessageRole::Assistant,
                content: crate::mcp::types::sampling::SamplingContent::Text(content),
                name: None,
                metadata: None,
            },
            model: self.config.default_model.clone(),
            stop_reason,
            usage: None,
            metadata: None,
        })
    }
    
    /// Call Anthropic API
    async fn call_anthropic_api(&self, sampling_request: &crate::mcp::types::sampling::SamplingRequest, provider: &crate::mcp::sampling::LLMProviderConfig) -> Result<crate::mcp::types::sampling::SamplingResponse> {
        let api_key = provider.api_key.as_ref()
            .ok_or_else(|| ProxyError::validation("No API key configured for Anthropic provider".to_string()))?;
        
        // Extract messages from sampling request
        let mut anthropic_messages = Vec::new();
        let mut system_message = String::new();
        
        for msg in &sampling_request.messages {
            let content = match &msg.content {
                crate::mcp::types::sampling::SamplingContent::Text(text) => text.clone(),
                _ => continue, // Skip non-text messages for now
            };
            
            match msg.role {
                crate::mcp::types::sampling::SamplingMessageRole::System => {
                    system_message = content;
                }
                crate::mcp::types::sampling::SamplingMessageRole::User => {
                    anthropic_messages.push(serde_json::json!({
                        "role": "user",
                        "content": content
                    }));
                }
                crate::mcp::types::sampling::SamplingMessageRole::Assistant => {
                    anthropic_messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": content
                    }));
                }
                crate::mcp::types::sampling::SamplingMessageRole::Tool => {
                    // Treat tool messages as user messages for Anthropic
                    anthropic_messages.push(serde_json::json!({
                        "role": "user",
                        "content": content
                    }));
                }
            }
        }
        
        let mut request_body = serde_json::json!({
            "model": self.config.default_model,
            "messages": anthropic_messages,
            "max_tokens": sampling_request.max_tokens.unwrap_or(2000),
            "temperature": sampling_request.temperature.unwrap_or(0.7)
        });
        
        if !system_message.is_empty() {
            request_body["system"] = serde_json::Value::String(system_message);
        }
        
        let response = self.http_client
            .post(&provider.endpoint)
            .header("x-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("Anthropic API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProxyError::connection(format!("Anthropic API error: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ProxyError::validation(format!("Failed to parse Anthropic response: {}", e)))?;
        
        // Extract content from Anthropic response
        let content = response_json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        let stop_reason = match response_json["stop_reason"].as_str() {
            Some("end_turn") => crate::mcp::types::sampling::SamplingStopReason::EndTurn,
            Some("max_tokens") => crate::mcp::types::sampling::SamplingStopReason::MaxTokens,
            _ => crate::mcp::types::sampling::SamplingStopReason::EndTurn,
        };
        
        Ok(crate::mcp::types::sampling::SamplingResponse {
            message: crate::mcp::types::sampling::SamplingMessage {
                role: crate::mcp::types::sampling::SamplingMessageRole::Assistant,
                content: crate::mcp::types::sampling::SamplingContent::Text(content),
                name: None,
                metadata: None,
            },
            model: self.config.default_model.clone(),
            stop_reason,
            usage: None,
            metadata: None,
        })
    }
    
    /// Call Ollama API
    async fn call_ollama_api(&self, sampling_request: &crate::mcp::types::sampling::SamplingRequest, provider: &crate::mcp::sampling::LLMProviderConfig) -> Result<crate::mcp::types::sampling::SamplingResponse> {
        
        // Extract messages from sampling request
        let mut ollama_messages = Vec::new();
        for msg in &sampling_request.messages {
            let content = match &msg.content {
                crate::mcp::types::sampling::SamplingContent::Text(text) => text.clone(),
                _ => continue, // Skip non-text messages for now
            };
            
            let role = match msg.role {
                crate::mcp::types::sampling::SamplingMessageRole::System => "system",
                crate::mcp::types::sampling::SamplingMessageRole::User => "user",
                crate::mcp::types::sampling::SamplingMessageRole::Assistant => "assistant",
                crate::mcp::types::sampling::SamplingMessageRole::Tool => "user", // Treat tool messages as user messages
            };
            
            ollama_messages.push(serde_json::json!({
                "role": role,
                "content": content
            }));
        }
        
        let request_body = serde_json::json!({
            "model": self.config.default_model,
            "messages": ollama_messages,
            "options": {
                "temperature": sampling_request.temperature.unwrap_or(0.7),
                "top_p": sampling_request.top_p.unwrap_or(0.9),
                "num_predict": sampling_request.max_tokens.unwrap_or(2000) // More tokens for resource content
            },
            "stream": false
        });
        
        let response = self.http_client
            .post(&format!("{}/api/chat", &provider.endpoint))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("Ollama API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProxyError::connection(format!("Ollama API error: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ProxyError::validation(format!("Failed to parse Ollama response: {}", e)))?;
        
        // Extract content from Ollama response
        let content = response_json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        let stop_reason = if response_json["done"].as_bool().unwrap_or(false) {
            crate::mcp::types::sampling::SamplingStopReason::EndTurn
        } else {
            crate::mcp::types::sampling::SamplingStopReason::MaxTokens
        };
        
        Ok(crate::mcp::types::sampling::SamplingResponse {
            message: crate::mcp::types::sampling::SamplingMessage {
                role: crate::mcp::types::sampling::SamplingMessageRole::Assistant,
                content: crate::mcp::types::sampling::SamplingContent::Text(content),
                name: None,
                metadata: None,
            },
            model: self.config.default_model.clone(),
            stop_reason,
            usage: None,
            metadata: None,
        })
    }
}

/// Convert resource type to string
fn resource_type_to_string(resource_type: &ResourceType) -> &str {
    match resource_type {
        ResourceType::Documentation => "Documentation",
        ResourceType::Examples => "Examples",
        ResourceType::Schema => "Schema",
        ResourceType::Configuration => "Configuration",
        ResourceType::OpenAPI => "OpenAPI",
        ResourceType::TestCases => "TestCases",
        ResourceType::Custom(name) => name,
    }
}

/// Extension trait for ResourceAnnotations
trait ResourceAnnotationsExt {
    fn with_generated(self, generated: bool) -> Self;
    fn with_tool_name(self, tool_name: String) -> Self;
    fn with_resource_type(self, resource_type: String) -> Self;
}

impl ResourceAnnotationsExt for ResourceAnnotations {
    fn with_generated(mut self, generated: bool) -> Self {
        let tag = format!("generated:{}", generated);
        if let Some(ref mut tags) = self.tags {
            tags.push(tag);
        } else {
            self.tags = Some(vec![tag]);
        }
        self
    }

    fn with_tool_name(mut self, tool_name: String) -> Self {
        let tag = format!("tool:{}", tool_name);
        if let Some(ref mut tags) = self.tags {
            tags.push(tag);
        } else {
            self.tags = Some(vec![tag]);
        }
        self
    }

    fn with_resource_type(mut self, resource_type: String) -> Self {
        let tag = format!("type:{}", resource_type);
        if let Some(ref mut tags) = self.tags {
            tags.push(tag);
        } else {
            self.tags = Some(vec![tag]);
        }
        self
    }
}