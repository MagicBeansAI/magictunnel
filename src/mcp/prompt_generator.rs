//! Prompt Generation Service
//!
//! This module provides server-side prompt template generation for tools, APIs, and capabilities.
//! Similar to sampling/elicitation, it uses LLM providers to generate contextual prompt templates
//! that help users interact with tools more effectively.

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, error};

use crate::error::{ProxyError, Result};
use crate::registry::types::{ToolDefinition, PromptReference, GenerationReferenceMetadata};
use crate::mcp::types::{PromptTemplate, PromptArgument};
use crate::mcp::content_storage::ContentStorageService;

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

/// Configuration for prompt generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGenerationConfig {
    /// LLM provider configurations
    pub providers: Vec<crate::mcp::sampling::LLMProviderConfig>,
    /// Default model to use for generation
    pub default_model: String,
    /// Maximum number of prompts to generate per tool
    pub max_prompts_per_tool: usize,
    /// Whether to include parameter validation prompts
    pub include_parameter_validation: bool,
    /// Whether to include usage example prompts
    pub include_usage_examples: bool,
    /// Whether to include troubleshooting prompts
    pub include_troubleshooting: bool,
    /// Custom prompt template for generation
    pub generation_template: Option<String>,
}

impl Default for PromptGenerationConfig {
    fn default() -> Self {
        Self {
            providers: vec![], // Will be populated from main config
            default_model: "default".to_string(), // Should be overridden in config
            max_prompts_per_tool: 3,
            include_parameter_validation: true,
            include_usage_examples: true,
            include_troubleshooting: false,
            generation_template: None,
        }
    }
}

/// Generated prompt template with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedPrompt {
    /// The generated prompt template
    pub template: PromptTemplate,
    /// Content of the prompt template
    pub content: String,
    /// Generation metadata
    pub metadata: GenerationMetadata,
    /// Confidence score for the generated prompt
    pub confidence: f32,
}

/// Request for prompt generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGenerationRequest {
    /// Tool to generate prompts for
    pub tool_name: String,
    /// Tool definition
    pub tool_definition: ToolDefinition,
    /// Types of prompts to generate
    pub prompt_types: Vec<PromptType>,
    /// Generation configuration
    pub config: PromptGenerationConfig,
}

/// Types of prompts that can be generated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromptType {
    /// General usage prompt for the tool
    Usage,
    /// Parameter validation and input guidance
    ParameterValidation,
    /// Usage examples and scenarios
    UsageExamples,
    /// Troubleshooting common issues
    Troubleshooting,
    /// Custom prompt type
    Custom(String),
}

/// Response from prompt generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGenerationResponse {
    /// Whether generation was successful
    pub success: bool,
    /// Generated prompts
    pub prompts: Vec<GeneratedPrompt>,
    /// Error message if generation failed
    pub error: Option<String>,
    /// Overall generation metadata
    pub metadata: GenerationMetadata,
}

/// Prompt generation service
pub struct PromptGeneratorService {
    /// Configuration for prompt generation
    config: PromptGenerationConfig,
    /// HTTP client for LLM API calls
    http_client: reqwest::Client,
    /// External MCP manager for fetching prompts from external servers
    external_mcp_manager: Option<Arc<ExternalMcpManager>>,
    /// Content storage service for persisting generated prompts
    content_storage: Option<Arc<ContentStorageService>>,
}

impl PromptGeneratorService {
    /// Create a new prompt generator service
    pub fn new(
        config: PromptGenerationConfig,
        external_mcp_manager: Option<Arc<ExternalMcpManager>>,
        content_storage: Option<Arc<ContentStorageService>>,
    ) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProxyError::connection(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            http_client,
            external_mcp_manager,
            content_storage,
        })
    }

    /// Generate prompts for a tool
    pub async fn generate_prompts(&self, request: PromptGenerationRequest) -> Result<PromptGenerationResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Generating prompts for tool: {}", request.tool_name);
        debug!("Prompt types requested: {:?}", request.prompt_types);
        
        // Step 1: Check if tool is from external MCP server and fetch existing prompts
        if let Some(external_prompts) = self.fetch_external_prompts(&request.tool_name).await? {
            info!("Found {} existing prompts from external MCP server for tool '{}'", external_prompts.len(), request.tool_name);
            
            let metadata = GenerationMetadata {
                model_used: Some("external_mcp_server".to_string()),
                confidence_score: Some(1.0), // External prompts are authoritative
                generation_time_ms: start_time.elapsed().as_millis() as u64,
            };
            
            return Ok(PromptGenerationResponse {
                success: true,
                prompts: external_prompts,
                error: None,
                metadata,
            });
        }

        let mut generated_prompts = Vec::new();
        let mut generation_errors = Vec::new();

        // Generate each requested prompt type
        for prompt_type in &request.prompt_types {
            match self.generate_single_prompt(&request, prompt_type).await {
                Ok(prompt) => {
                    debug!("Successfully generated {} prompt for tool: {}", 
                           prompt_type_to_string(prompt_type), request.tool_name);
                    generated_prompts.push(prompt);
                }
                Err(e) => {
                    warn!("Failed to generate {} prompt for tool '{}': {}", 
                          prompt_type_to_string(prompt_type), request.tool_name, e);
                    generation_errors.push(format!("{}: {}", prompt_type_to_string(prompt_type), e));
                }
            }
        }

        let generation_time_ms = start_time.elapsed().as_millis() as u64;
        let success = !generated_prompts.is_empty();

        let metadata = GenerationMetadata {
            model_used: Some(self.get_model_name()),
            confidence_score: if success { 
                Some(generated_prompts.iter().map(|p| p.confidence).sum::<f32>() / generated_prompts.len() as f32)
            } else { 
                None 
            },
            generation_time_ms,
        };

        let response = PromptGenerationResponse {
            success,
            prompts: generated_prompts,
            error: if generation_errors.is_empty() {
                None
            } else {
                Some(generation_errors.join("; "))
            },
            metadata,
        };

        if response.success {
            info!("Successfully generated {} prompts for tool '{}' in {}ms", 
                  response.prompts.len(), request.tool_name, generation_time_ms);
        } else {
            error!("Failed to generate prompts for tool '{}': {}", 
                   request.tool_name, response.error.as_deref().unwrap_or("Unknown error"));
        }

        Ok(response)
    }

    /// Generate a single prompt for a specific type
    async fn generate_single_prompt(&self, request: &PromptGenerationRequest, prompt_type: &PromptType) -> Result<GeneratedPrompt> {
        let system_prompt = self.create_system_prompt(prompt_type);
        let user_prompt = self.create_user_prompt(request, prompt_type);

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
            max_tokens: Some(1000),
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

        // Parse the generated content into a prompt template
        let (template, prompt_content) = self.parse_generated_prompt(&content, &request.tool_name, prompt_type)?;

        // Calculate confidence based on response quality
        let confidence = self.calculate_prompt_confidence(&llm_response, &content);

        let metadata = GenerationMetadata {
            model_used: Some(llm_response.model),
            confidence_score: Some(confidence),
            generation_time_ms: 0, // Will be set by caller
        };

        Ok(GeneratedPrompt {
            template,
            content: prompt_content,
            metadata,
            confidence,
        })
    }

    /// Create system prompt for prompt generation
    fn create_system_prompt(&self, prompt_type: &PromptType) -> String {
        match prompt_type {
            PromptType::Usage => {
                "You are an expert at creating user-friendly prompt templates for API tools and services. \
                Create a helpful prompt template that guides users on how to use the tool effectively. \
                The prompt should be clear, concise, and include placeholders for parameters using {{parameter_name}} syntax. \
                Focus on the main use case and provide context about what the tool does.".to_string()
            }
            PromptType::ParameterValidation => {
                "You are an expert at creating parameter validation prompt templates. \
                Create a prompt template that helps users understand what parameters are required, \
                their expected formats, and how to provide valid input. \
                Use {{parameter_name}} syntax for placeholders and include validation guidance.".to_string()
            }
            PromptType::UsageExamples => {
                "You are an expert at creating example-driven prompt templates. \
                Create a prompt template that provides concrete usage examples and scenarios. \
                Show different ways the tool can be used with realistic examples. \
                Use {{parameter_name}} syntax for placeholders in examples.".to_string()
            }
            PromptType::Troubleshooting => {
                "You are an expert at creating troubleshooting prompt templates. \
                Create a prompt template that helps users diagnose and resolve common issues. \
                Include guidance on error scenarios and how to fix them. \
                Use {{parameter_name}} syntax for placeholders.".to_string()
            }
            PromptType::Custom(description) => {
                format!("You are an expert at creating custom prompt templates. \
                Create a prompt template for: {}. \
                Use {{{{parameter_name}}}} syntax for placeholders and make it user-friendly.", description)
            }
        }
    }

    /// Create user prompt with tool context
    fn create_user_prompt(&self, request: &PromptGenerationRequest, prompt_type: &PromptType) -> String {
        let tool_def = &request.tool_definition;
        let parameters = serde_json::to_string_pretty(&tool_def.input_schema).unwrap_or_default();
        
        format!(
            "Tool Name: {}\n\
            Description: {}\n\
            Parameters Schema: {}\n\
            Routing Type: {}\n\n\
            Create a {} prompt template for this tool. \
            The prompt should be helpful for users and include proper parameter placeholders. \
            Return only the prompt template content, no explanation or metadata.",
            tool_def.name,
            tool_def.description,
            parameters,
            tool_def.routing.r#type,
            prompt_type_to_string(prompt_type).to_lowercase()
        )
    }

    /// Parse generated content into prompt template and content
    fn parse_generated_prompt(&self, content: &str, tool_name: &str, prompt_type: &PromptType) -> Result<(PromptTemplate, String)> {
        // Extract parameter placeholders from content
        let parameter_regex = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();
        let mut arguments = Vec::new();
        
        for captures in parameter_regex.captures_iter(content) {
            if let Some(param_name) = captures.get(1) {
                let name = param_name.as_str().trim().to_string();
                if !arguments.iter().any(|arg: &PromptArgument| arg.name == name) {
                    arguments.push(PromptArgument {
                        name: name.clone(),
                        description: Some(format!("Parameter: {}", name)),
                        required: true, // Default to required, can be refined later
                    });
                }
            }
        }

        // Create prompt template
        let template_name = format!("{}_{}", tool_name, prompt_type_to_string(prompt_type).to_lowercase());
        let template = PromptTemplate {
            name: template_name,
            description: Some(format!("{} prompt for {}", prompt_type_to_string(prompt_type), tool_name)),
            arguments,
        };

        Ok((template, content.to_string()))
    }

    /// Calculate confidence score for generated prompt
    fn calculate_prompt_confidence(&self, response: &crate::mcp::types::sampling::SamplingResponse, content: &str) -> f32 {
        let mut confidence: f32 = 0.8; // Base confidence

        // Adjust based on stop reason
        match response.stop_reason {
            crate::mcp::types::sampling::SamplingStopReason::EndTurn => confidence += 0.1,
            crate::mcp::types::sampling::SamplingStopReason::MaxTokens => confidence -= 0.2,
            _ => confidence -= 0.1,
        }

        // Adjust based on content quality indicators
        if content.contains("{{") && content.contains("}}") {
            confidence += 0.1; // Has parameter placeholders
        }
        
        if content.len() > 50 && content.len() < 2000 {
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
    
    /// Create mock prompt content for testing/demo purposes
    fn create_mock_prompt_content(&self, request: &PromptGenerationRequest, prompt_type: &PromptType) -> String {
        let tool_def = &request.tool_definition;
        
        match prompt_type {
            PromptType::Usage => {
                format!(
                    "Use the {{{{tool_name}}}} tool to {}. \n\nRequired parameters:\n{{{{parameters}}}}\n\nExample usage:\n```\n{{{{tool_name}}}} {{{{example_params}}}}\n```",
                    tool_def.description.to_lowercase()
                )
            }
            PromptType::ParameterValidation => {
                format!(
                    "Before using {}, please ensure:\n\n• All required parameters are provided\n• Parameter types match the expected format\n• Values are within acceptable ranges\n\nRequired: {{{{required_params}}}}\nOptional: {{{{optional_params}}}}",
                    tool_def.name
                )
            }
            PromptType::UsageExamples => {
                format!(
                    "Here are some examples of how to use {}:\n\n**Example 1: Basic usage**\n{{{{basic_example}}}}\n\n**Example 2: Advanced usage**\n{{{{advanced_example}}}}\n\n**Example 3: Error handling**\n{{{{error_example}}}}",
                    tool_def.name
                )
            }
            PromptType::Troubleshooting => {
                format!(
                    "If you encounter issues with {}:\n\n**Common Problems:**\n• {{{{common_issue_1}}}}\n• {{{{common_issue_2}}}}\n\n**Solutions:**\n• {{{{solution_1}}}}\n• {{{{solution_2}}}}\n\n**Need more help?** {{{{help_contact}}}}",
                    tool_def.name
                )
            }
            PromptType::Custom(description) => {
                format!(
                    "Custom prompt for {} ({}):\n\n{{{{custom_content}}}}\n\nTool: {}\nDescription: {}",
                    tool_def.name,
                    description,
                    tool_def.name,
                    tool_def.description
                )
            }
        }
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
        let api_key = match &provider.api_key {
            Some(key) => key,
            None => {
                return Err(ProxyError::validation("No API key configured for OpenAI provider".to_string()));
            }
        };
        
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
            "max_tokens": sampling_request.max_tokens.unwrap_or(1000),
            "temperature": sampling_request.temperature.unwrap_or(0.7),
            "top_p": sampling_request.top_p.unwrap_or(0.9)
        });
        
        let endpoint = format!("{}/chat/completions", provider.endpoint.trim_end_matches('/'));
        let response = self.http_client
            .post(&endpoint)
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
        let api_key = match &provider.api_key {
            Some(key) => key,
            None => {
                return Err(ProxyError::validation("No API key configured for Anthropic provider".to_string()));
            }
        };
        
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
            "max_tokens": sampling_request.max_tokens.unwrap_or(1000),
            "temperature": sampling_request.temperature.unwrap_or(0.7)
        });
        
        if !system_message.is_empty() {
            request_body["system"] = serde_json::Value::String(system_message);
        }
        
        let endpoint = format!("{}/v1/messages", provider.endpoint.trim_end_matches('/'));
        let response = self.http_client
            .post(&endpoint)
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
        let ollama_url = &provider.endpoint;
        
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
                "num_predict": sampling_request.max_tokens.unwrap_or(1000)
            },
            "stream": false
        });
        
        let response = self.http_client
            .post(&format!("{}/api/chat", ollama_url))
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
    
    /// Fetch prompts from external MCP servers for the given tool
    async fn fetch_external_prompts(&self, tool_name: &str) -> Result<Option<Vec<GeneratedPrompt>>> {
        let external_manager = match &self.external_mcp_manager {
            Some(manager) => manager,
            None => {
                debug!("No external MCP manager configured, skipping external prompt fetch");
                return Ok(None);
            }
        };
        
        debug!("Checking external MCP servers for prompts for tool: {}", tool_name);
        
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
        
        info!("Tool '{}' found in external MCP server '{}', fetching prompts", tool_name, server_name);
        
        // Fetch prompts from the external server
        match self.fetch_prompts_from_server(&server_name, tool_name).await {
            Ok(prompts) => {
                if prompts.is_empty() {
                    info!("No prompts found for tool '{}' in external MCP server '{}'", tool_name, server_name);
                    Ok(None)
                } else {
                    info!("Successfully fetched {} prompts for tool '{}' from external MCP server '{}'", prompts.len(), tool_name, server_name);
                    Ok(Some(prompts))
                }
            }
            Err(e) => {
                warn!("Failed to fetch prompts for tool '{}' from external MCP server '{}': {}", tool_name, server_name, e);
                Ok(None)
            }
        }
    }
    
    /// Fetch prompts from a specific external MCP server
    async fn fetch_prompts_from_server(&self, server_name: &str, tool_name: &str) -> Result<Vec<GeneratedPrompt>> {
        let external_manager = self.external_mcp_manager.as_ref().unwrap(); // Safe since we checked above
        
        debug!("Fetching prompts from external MCP server '{}' for tool '{}'", server_name, tool_name);
        
        // Try to get prompts from the external server using prompts/list
        let request_result = external_manager.send_request_to_server(
            server_name,
            "prompts/list",
            Some(serde_json::json!({
                "cursor": null
            }))
        ).await;
        
        let response = match request_result {
            Ok(response) => response,
            Err(e) => {
                debug!("External MCP server '{}' doesn't support prompts/list: {}", server_name, e);
                return Ok(Vec::new());
            }
        };
        
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
        
        // Filter prompts related to the tool and fetch their content
        let mut generated_prompts = Vec::new();
        
        for prompt_template in prompts_list {
            // Check if prompt is related to the tool (by name or description)
            let is_tool_related = prompt_template.name.contains(tool_name) ||
                prompt_template.description.as_ref().map_or(false, |desc| desc.contains(tool_name));
            
            if !is_tool_related {
                continue;
            }
            
            // Fetch the prompt content
            let prompt_content = match self.fetch_prompt_content_from_server(server_name, &prompt_template.name).await {
                Ok(content) => content,
                Err(e) => {
                    warn!("Failed to fetch content for prompt '{}' from external MCP server '{}': {}", prompt_template.name, server_name, e);
                    continue;
                }
            };
            
            let generated_prompt = GeneratedPrompt {
                template: prompt_template,
                content: prompt_content,
                metadata: GenerationMetadata {
                    model_used: Some(format!("external_mcp_server:{}", server_name)),
                    confidence_score: Some(1.0), // External prompts are authoritative
                    generation_time_ms: 0,
                },
                confidence: 1.0, // External prompts are authoritative
            };
            
            generated_prompts.push(generated_prompt);
        }
        
        Ok(generated_prompts)
    }
    
    /// Fetch prompt content from external MCP server
    async fn fetch_prompt_content_from_server(&self, server_name: &str, prompt_name: &str) -> Result<String> {
        let external_manager = self.external_mcp_manager.as_ref().unwrap(); // Safe since we checked above
        
        debug!("Fetching prompt content for '{}' from external MCP server '{}'", prompt_name, server_name);
        
        let request_result = external_manager.send_request_to_server(
            server_name,
            "prompts/get",
            Some(serde_json::json!({
                "name": prompt_name,
                "arguments": {}
            }))
        ).await;
        
        let response = match request_result {
            Ok(response) => response,
            Err(e) => {
                return Err(ProxyError::mcp(format!("Failed to get prompt '{}' from external MCP server '{}': {}", prompt_name, server_name, e)));
            }
        };
        
        if let Some(error) = response.error {
            return Err(ProxyError::mcp(format!("External MCP server '{}' returned error for prompt '{}': {}", server_name, prompt_name, error.message)));
        }
        
        let content = match response.result {
            Some(result) => {
                // For now, assume the result contains text content directly
                // This may need adjustment based on the actual MCP prompt response format
                result.as_str().unwrap_or("").to_string()
            }
            None => {
                return Err(ProxyError::mcp(format!("No prompt result from external MCP server '{}'", server_name)));
            }
        };
        debug!("Successfully fetched prompt content for '{}' from external MCP server '{}' ({} chars)", prompt_name, server_name, content.len());
        
        Ok(content)
    }
}

/// Convert prompt type to string
fn prompt_type_to_string(prompt_type: &PromptType) -> &str {
    match prompt_type {
        PromptType::Usage => "Usage",
        PromptType::ParameterValidation => "Parameter Validation",
        PromptType::UsageExamples => "Usage Examples",
        PromptType::Troubleshooting => "Troubleshooting",
        PromptType::Custom(name) => name,
    }
}

/// Check if a tool is from an external MCP server (helper function)
pub async fn is_external_mcp_tool(tool_name: &str, external_manager: Option<&Arc<ExternalMcpManager>>) -> bool {
    let manager = match external_manager {
        Some(m) => m,
        None => return false,
    };
    
    let all_tools = manager.get_all_tools().await;
    for (_, tools) in all_tools {
        if tools.iter().any(|tool| tool.name == tool_name) {
            return true;
        }
    }
    false
}

/// Get external MCP server name for a tool (helper function)
pub async fn get_external_mcp_server_for_tool(tool_name: &str, external_manager: Option<&Arc<ExternalMcpManager>>) -> Option<String> {
    let manager = match external_manager {
        Some(m) => m,
        None => return None,
    };
    
    let all_tools = manager.get_all_tools().await;
    for (server_name, tools) in all_tools {
        if tools.iter().any(|tool| tool.name == tool_name) {
            return Some(server_name);
        }
    }
    None
}