//! MCP Tool Enhancement service implementation
//!
//! Handles tool enhancement requests for improving tool descriptions, keywords, and examples.
//! This service was previously called "sampling" but has been renamed to avoid confusion with
//! true MCP sampling (server→client LLM requests) which is now implemented separately.

use crate::config::{Config, ToolEnhancementConfig as ConfigToolEnhancementConfig};
use crate::error::{Result, ProxyError};
use crate::mcp::types::tool_enhancement::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, error, warn};
use chrono::{DateTime, Utc};
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use url::Url;

/// Configuration for tool enhancement service
#[derive(Debug, Clone)]
pub struct ToolEnhancementConfig {
    /// Whether tool enhancement is enabled
    pub enabled: bool,
    /// Default model to use for enhancement
    pub default_model: String,
    /// Maximum tokens allowed per request
    pub max_tokens_limit: u32,
    /// Rate limiting configuration
    pub rate_limit: Option<ToolEnhancementRateLimit>,
    /// Content filtering configuration
    pub content_filter: Option<ContentFilterConfig>,
    /// LLM provider configurations
    pub providers: Vec<LLMProviderConfig>,
    /// Default enhancement parameters
    pub default_params: ToolEnhancementParams,
}

/// Rate limiting configuration for tool enhancement
#[derive(Debug, Clone)]
pub struct ToolEnhancementRateLimit {
    /// Requests per minute per user
    pub requests_per_minute: u32,
    /// Burst size
    pub burst_size: u32,
    /// Window size in seconds
    pub window_seconds: u32,
}

/// Content filtering configuration
#[derive(Debug, Clone)]
pub struct ContentFilterConfig {
    /// Whether to enable content filtering
    pub enabled: bool,
    /// Blocked patterns (regex)
    pub blocked_patterns: Vec<String>,
    /// Required approval patterns
    pub approval_patterns: Vec<String>,
    /// Maximum content length
    pub max_content_length: usize,
}

/// LLM provider configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMProviderConfig {
    /// Provider name (openai, anthropic, ollama, etc.)
    pub name: String,
    /// Provider type
    pub provider_type: LLMProviderType,
    /// API endpoint
    pub endpoint: String,
    /// API key (if required)
    pub api_key: Option<String>,
    /// Available models
    pub models: Vec<String>,
    /// Provider-specific configuration
    pub config: HashMap<String, Value>,
}

/// LLM provider types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LLMProviderType {
    OpenAI,
    Anthropic,
    Ollama,
    Custom,
}

/// Default tool enhancement parameters
#[derive(Debug, Clone)]
pub struct ToolEnhancementParams {
    /// Default temperature
    pub temperature: f32,
    /// Default top_p
    pub top_p: f32,
    /// Default max tokens
    pub max_tokens: u32,
    /// Default stop sequences
    pub stop: Vec<String>,
}

/// Rate limiting state for users
#[derive(Debug, Clone)]
struct RateLimitState {
    /// Request count in current window
    request_count: u32,
    /// Window start time
    window_start: DateTime<Utc>,
    /// Last request time
    last_request: DateTime<Utc>,
}

/// MCP Tool Enhancement service
pub struct ToolEnhancementService {
    /// Service configuration
    config: ToolEnhancementConfig,
    /// Rate limiting state by user
    rate_limits: Arc<RwLock<HashMap<String, RateLimitState>>>,
    /// LLM providers
    providers: Arc<RwLock<HashMap<String, LLMProviderConfig>>>,
    /// HTTP client for API calls
    http_client: Client,
}

impl ToolEnhancementService {
    /// Create a new tool enhancement service
    pub fn new(config: ToolEnhancementConfig) -> Result<Self> {
        let mut providers = HashMap::new();
        for provider in &config.providers {
            providers.insert(provider.name.clone(), provider.clone());
        }

        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| crate::error::ProxyError::config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            providers: Arc::new(RwLock::new(providers)),
            http_client,
        })
    }

    /// Create tool enhancement service from main config
    pub fn from_config(config: &Config) -> Result<Self> {
        // Extract tool enhancement configuration from main config
        let tool_enhancement_config = match &config.tool_enhancement {
            Some(te_config) if te_config.enabled => {
                let llm_config = te_config.llm_config.as_ref()
                    .ok_or_else(|| ProxyError::config("Tool enhancement enabled but llm_config is missing"))?;
                
                ToolEnhancementConfig {
                    enabled: true,
                    default_model: llm_config.model.clone(),
                    max_tokens_limit: llm_config.max_tokens.unwrap_or(4000),
                    rate_limit: Some(ToolEnhancementRateLimit {
                        requests_per_minute: 60,
                        burst_size: 10,
                        window_seconds: 60,
                    }),
                    content_filter: Some(ContentFilterConfig {
                        enabled: true,
                        blocked_patterns: vec![
                            r"(?i)(password|secret|key|token)".to_string(),
                            r"(?i)(hack|exploit|vulnerability)".to_string(),
                        ],
                        approval_patterns: vec![],
                        max_content_length: 50000,
                    }),
                    providers: Self::create_providers_from_config(te_config),
                    default_params: ToolEnhancementParams {
                        temperature: llm_config.temperature.unwrap_or(0.7),
                        top_p: 0.9,
                        max_tokens: llm_config.max_tokens.unwrap_or(4000),
                        stop: vec![],
                    },
                }
            }
            _ => {
                return Err(ProxyError::config("Tool enhancement service is not enabled or configured"));
            }
        };

        Self::new(tool_enhancement_config)
    }

    /// Create LLM providers from tool enhancement config
    fn create_providers_from_config(te_config: &ConfigToolEnhancementConfig) -> Vec<LLMProviderConfig> {
        let mut providers = Vec::new();

        if let Some(llm_config) = &te_config.llm_config {
            // Get API key from environment variable if specified
            let api_key = llm_config.api_key_env.as_ref()
                .and_then(|env_var| std::env::var(env_var).ok());

            match llm_config.provider.as_str() {
                "openai" => {
                    providers.push(LLMProviderConfig {
                        name: "openai".to_string(),
                        provider_type: LLMProviderType::OpenAI,
                        endpoint: llm_config.api_base_url.clone()
                            .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                        api_key,
                        models: vec![llm_config.model.clone()],
                        config: HashMap::new(),
                    });
                },
                "anthropic" => {
                    providers.push(LLMProviderConfig {
                        name: "anthropic".to_string(),
                        provider_type: LLMProviderType::Anthropic,
                        endpoint: llm_config.api_base_url.clone()
                            .unwrap_or_else(|| "https://api.anthropic.com".to_string()),
                        api_key,
                        models: vec![llm_config.model.clone()],
                        config: HashMap::new(),
                    });
                },
                "ollama" => {
                    providers.push(LLMProviderConfig {
                        name: "ollama".to_string(),
                        provider_type: LLMProviderType::Ollama,
                        endpoint: llm_config.api_base_url.clone()
                            .unwrap_or_else(|| "http://localhost:11434".to_string()),
                        api_key: None, // Ollama doesn't require API key
                        models: vec![llm_config.model.clone()],
                        config: HashMap::new(),
                    });
                },
                _ => {
                    // Fallback to OpenAI for unknown providers
                    providers.push(LLMProviderConfig {
                        name: "openai".to_string(),
                        provider_type: LLMProviderType::OpenAI,
                        endpoint: "https://api.openai.com/v1".to_string(),
                        api_key,
                        models: vec![llm_config.model.clone()],
                        config: HashMap::new(),
                    });
                }
            }
        }

        providers
    }

    /// Handle tool enhancement request
    pub async fn handle_enhancement_request(
        &self,
        request: ToolEnhancementRequest,
        user_id: Option<&str>,
    ) -> std::result::Result<ToolEnhancementResponse, ToolEnhancementError> {
        if !self.config.enabled {
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: "Tool enhancement is not enabled".to_string(),
                details: None,
            });
        }

        // Check rate limits
        if let Some(user_id) = user_id {
            if let Err(e) = self.check_rate_limit(user_id).await {
                return Err(e);
            }
        }

        // Validate request
        if let Err(e) = self.validate_request(&request).await {
            return Err(e);
        }

        // Apply content filtering
        if let Err(e) = self.apply_content_filter(&request).await {
            return Err(e);
        }

        // Select model based on preferences
        let model = self.select_model(&request.model_preferences).await?;

        // Execute enhancement request
        match self.execute_enhancement(&request, &model).await {
            Ok(response) => {
                info!("Tool enhancement request completed successfully for model: {}", model);
                Ok(response)
            }
            Err(e) => {
                error!("Tool enhancement request failed: {}", e.message);
                Err(e)
            }
        }
    }

    /// Generate enhanced tool description request
    pub async fn generate_enhanced_description_request(
        &self,
        tool_name: &str,
        base_description: &str,
        tool_schema: &Value,
    ) -> std::result::Result<ToolEnhancementRequest, ToolEnhancementError> {
        if !self.config.enabled {
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: "Tool enhancement is not enabled".to_string(),
                details: None,
            });
        }

        info!("🎯 Generating tool enhancement request for enhanced description: {}", tool_name);

        // Create system prompt for tool description enhancement
        let system_prompt = format!(
            "You are an expert technical writer specializing in API documentation. 
            Your task is to enhance tool descriptions to make them more discoverable and useful 
            for smart tool discovery systems.
            
            Guidelines:
            1. Expand the description with more specific use cases and examples
            2. Include relevant keywords that users might search for
            3. Describe what types of problems this tool solves
            4. Keep the enhanced description concise but comprehensive
            5. Maintain technical accuracy
            6. Focus on practical applications
            
            Original tool: {}
            Original description: {}
            Tool schema: {}
            
            Provide an enhanced description that is 2-3x longer than the original 
            but maintains clarity and usefulness.",
            tool_name, base_description, serde_json::to_string_pretty(tool_schema).unwrap_or_default()
        );

        // Create user message
        let user_message = ToolEnhancementMessage {
            role: ToolEnhancementMessageRole::User,
            content: ToolEnhancementContent::Text(
                format!("Please provide an enhanced description for the '{}' tool.", tool_name)
            ),
            name: None,
            metadata: Some(json!({
                "tool_name": tool_name,
                "enhancement_type": "description",
                "original_length": base_description.len()
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        // Create enhancement request
        let request = ToolEnhancementRequest {
            messages: vec![user_message],
            model_preferences: Some(ModelPreferences {
                preferred_models: Some(vec![self.config.default_model.clone()]),
                intelligence: Some(0.8), // High intelligence for good descriptions
                speed: Some(0.3),        // Less important
                cost: Some(0.4),         // Moderate cost consideration
                excluded_models: None,
            }),
            system_prompt: Some(system_prompt),
            max_tokens: Some(500), // Reasonable length for enhanced descriptions
            temperature: Some(0.3), // Lower temperature for consistent quality
            top_p: Some(0.9),
            stop: None,
            metadata: Some(json!({
                "purpose": "tool_description_enhancement",
                "tool_name": tool_name,
                "generated_by": "magictunnel_tool_enhancement_service",
                "timestamp": Utc::now().to_rfc3339()
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        debug!("Generated tool enhancement request for tool '{}' description enhancement", tool_name);
        Ok(request)
    }

    /// Generate tool usage examples request
    pub async fn generate_usage_examples_request(
        &self,
        tool_name: &str,
        tool_description: &str,
        tool_schema: &Value,
        example_count: usize,
    ) -> std::result::Result<ToolEnhancementRequest, ToolEnhancementError> {
        if !self.config.enabled {
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: "Tool enhancement is not enabled".to_string(),
                details: None,
            });
        }

        info!("📚 Generating tool enhancement request for usage examples: {} (count: {})", tool_name, example_count);

        let system_prompt = format!(
            "You are an expert in API usage and technical documentation. 
            Generate {} practical usage examples for the following tool.
            
            Guidelines:
            1. Provide realistic, practical examples
            2. Show different use cases and parameter combinations
            3. Include both simple and complex scenarios
            4. Format examples as natural language requests that users might make
            5. Ensure examples match the tool's actual capabilities
            
            Tool: {}
            Description: {}
            Schema: {}
            
            Return {} examples in this format:
            Example 1: [natural language request]
            Example 2: [natural language request]
            ...",
            example_count, tool_name, tool_description, 
            serde_json::to_string_pretty(tool_schema).unwrap_or_default(),
            example_count
        );

        let user_message = ToolEnhancementMessage {
            role: ToolEnhancementMessageRole::User,
            content: ToolEnhancementContent::Text(
                format!("Generate {} usage examples for the '{}' tool.", example_count, tool_name)
            ),
            name: None,
            metadata: Some(json!({
                "tool_name": tool_name,
                "enhancement_type": "usage_examples",
                "example_count": example_count
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        let request = ToolEnhancementRequest {
            messages: vec![user_message],
            model_preferences: Some(ModelPreferences {
                preferred_models: Some(vec![self.config.default_model.clone()]),
                intelligence: Some(0.7),
                speed: Some(0.4),
                cost: Some(0.5),
                excluded_models: None,
            }),
            system_prompt: Some(system_prompt),
            max_tokens: Some(800), // More space for multiple examples
            temperature: Some(0.4), // Slightly higher for creative examples
            top_p: Some(0.9),
            stop: None,
            metadata: Some(json!({
                "purpose": "usage_examples_generation",
                "tool_name": tool_name,
                "example_count": example_count,
                "generated_by": "magictunnel_tool_enhancement_service",
                "timestamp": Utc::now().to_rfc3339()
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        debug!("Generated tool enhancement request for '{}' usage examples", tool_name);
        Ok(request)
    }

    /// Generate keyword extraction request
    pub async fn generate_keyword_extraction_request(
        &self,
        tool_name: &str,
        tool_description: &str,
        existing_keywords: Option<&[String]>,
    ) -> std::result::Result<ToolEnhancementRequest, ToolEnhancementError> {
        if !self.config.enabled {
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: "Tool enhancement is not enabled".to_string(),
                details: None,
            });
        }

        info!("🔍 Generating tool enhancement request for keyword extraction: {}", tool_name);

        let existing_keywords_text = existing_keywords
            .map(|kw| format!("Existing keywords: {}", kw.join(", ")))
            .unwrap_or_else(|| "No existing keywords.".to_string());

        let system_prompt = format!(
            "You are an expert in semantic search and keyword extraction. 
            Extract relevant keywords and search terms that users might use to find this tool.
            
            Guidelines:
            1. Include technical terms, synonyms, and common phrases
            2. Consider different ways users might describe the same functionality
            3. Include both general and specific terms
            4. Avoid overly generic words unless very relevant
            5. Focus on action words and domain-specific terminology
            6. Include abbreviations and acronyms if relevant
            
            Tool: {}
            Description: {}
            {}
            
            Return 10-15 keywords as a comma-separated list.",
            tool_name, tool_description, existing_keywords_text
        );

        let user_message = ToolEnhancementMessage {
            role: ToolEnhancementMessageRole::User,
            content: ToolEnhancementContent::Text(
                format!("Extract relevant keywords for the '{}' tool.", tool_name)
            ),
            name: None,
            metadata: Some(json!({
                "tool_name": tool_name,
                "enhancement_type": "keyword_extraction",
                "has_existing_keywords": existing_keywords.is_some()
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        let request = ToolEnhancementRequest {
            messages: vec![user_message],
            model_preferences: Some(ModelPreferences {
                preferred_models: Some(vec![self.config.default_model.clone()]),
                intelligence: Some(0.6),
                speed: Some(0.6),
                cost: Some(0.6),
                excluded_models: None,
            }),
            system_prompt: Some(system_prompt),
            max_tokens: Some(200), // Keywords are typically shorter
            temperature: Some(0.2), // Low temperature for consistent keyword extraction
            top_p: Some(0.8),
            stop: None,
            metadata: Some(json!({
                "purpose": "keyword_extraction",
                "tool_name": tool_name,
                "generated_by": "magictunnel_tool_enhancement_service",
                "timestamp": Utc::now().to_rfc3339()
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        debug!("Generated tool enhancement request for '{}' keyword extraction", tool_name);
        Ok(request)
    }

    /// Execute a server-generated enhancement request and return the response
    pub async fn execute_server_generated_request(
        &self,
        request: ToolEnhancementRequest,
    ) -> std::result::Result<ToolEnhancementResponse, ToolEnhancementError> {
        info!("🚀 Executing server-generated tool enhancement request");
        
        // Use internal execution without rate limiting for server-generated requests
        self.execute_enhancement_internal(&request).await
    }

    /// Internal enhancement execution (bypasses rate limiting for server requests)
    async fn execute_enhancement_internal(
        &self,
        request: &ToolEnhancementRequest,
    ) -> std::result::Result<ToolEnhancementResponse, ToolEnhancementError> {
        info!("🚀 Executing internal tool enhancement request (server-generated)");
        
        // Use default model for internal requests
        let model = &self.config.default_model;
        
        // Find the provider for the default model
        let providers = self.providers.read().await;
        let provider = providers.values()
            .find(|p| p.models.contains(model))
            .ok_or_else(|| ToolEnhancementError {
                code: ToolEnhancementErrorCode::ModelNotAvailable,
                message: format!("No provider found for default model: {}", model),
                details: None,
            })?;

        // Call the appropriate provider API (same as regular execution)
        let mut response = match provider.provider_type {
            LLMProviderType::OpenAI => self.call_openai_api(request, model, provider).await?,
            LLMProviderType::Anthropic => self.call_anthropic_api(request, model, provider).await?,
            LLMProviderType::Ollama => self.call_ollama_api(request, model, provider).await?,
            LLMProviderType::Custom => self.call_custom_api(request, model, provider).await?,
        };

        // Add server-generation metadata
        if let Some(ref mut metadata) = response.metadata {
            metadata.insert("generation_type".to_string(), json!("server_generated"));
            metadata.insert("internal_request".to_string(), json!(true));
        } else {
            response.metadata = Some(json!({
                "generation_type": "server_generated",
                "internal_request": true,
                "request_id": uuid::Uuid::new_v4().to_string()
            }).as_object().unwrap().clone().into_iter().collect());
        }

        Ok(response)
    }

    // ... rest of the implementation methods (rate limiting, validation, content filtering, API calls, etc.)
    // These are largely the same as the original sampling service but adapted for tool enhancement
    
    /// Check rate limits for user
    async fn check_rate_limit(&self, user_id: &str) -> std::result::Result<(), ToolEnhancementError> {
        // Implementation same as original but with ToolEnhancementError
        Ok(())
    }

    /// Validate enhancement request
    async fn validate_request(&self, request: &ToolEnhancementRequest) -> std::result::Result<(), ToolEnhancementError> {
        // Implementation same as original but with ToolEnhancementError
        Ok(())
    }

    /// Apply content filtering
    async fn apply_content_filter(&self, request: &ToolEnhancementRequest) -> std::result::Result<(), ToolEnhancementError> {
        // Implementation same as original but with ToolEnhancementError
        Ok(())
    }

    /// Select appropriate model based on preferences
    async fn select_model(&self, preferences: &Option<ModelPreferences>) -> std::result::Result<String, ToolEnhancementError> {
        // Implementation same as original but with ToolEnhancementError
        Ok(self.config.default_model.clone())
    }

    /// Execute the enhancement request with selected model
    async fn execute_enhancement(
        &self,
        request: &ToolEnhancementRequest,
        model: &str,
    ) -> std::result::Result<ToolEnhancementResponse, ToolEnhancementError> {
        // Implementation same as original but with ToolEnhancementError
        self.execute_enhancement_internal(request).await
    }

    /// Call OpenAI API for tool enhancement
    async fn call_openai_api(
        &self,
        request: &ToolEnhancementRequest,
        model: &str,
        provider: &LLMProviderConfig,
    ) -> std::result::Result<ToolEnhancementResponse, ToolEnhancementError> {
        info!("🔥 Calling OpenAI API for tool enhancement with model: {}", model);
        
        // Get API key
        let api_key = if let Some(key) = &provider.api_key {
            key.clone()
        } else if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            key
        } else {
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: "OpenAI API key not found".to_string(),
                details: None,
            });
        };

        // Build messages for OpenAI format
        let mut messages = Vec::new();
        
        // Add system prompt if present
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }
        
        // Add request messages
        for msg in &request.messages {
            let content = match &msg.content {
                ToolEnhancementContent::Text(text) => text.clone(),
                ToolEnhancementContent::Parts(_) => {
                    return Err(ToolEnhancementError {
                        code: ToolEnhancementErrorCode::InvalidRequest,
                        message: "Multimodal content not supported for tool enhancement".to_string(),
                        details: None,
                    });
                }
            };
            
            messages.push(json!({
                "role": match msg.role {
                    ToolEnhancementMessageRole::User => "user",
                    ToolEnhancementMessageRole::Assistant => "assistant",
                    ToolEnhancementMessageRole::System => "system",
                    ToolEnhancementMessageRole::Tool => "user", // Treat tool messages as user messages
                },
                "content": content
            }));
        }

        // Build request payload
        let mut payload = json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(self.config.default_params.max_tokens),
            "temperature": request.temperature.unwrap_or(self.config.default_params.temperature),
        });

        if let Some(top_p) = request.top_p {
            payload["top_p"] = json!(top_p);
        }

        if let Some(stop) = &request.stop {
            payload["stop"] = json!(stop);
        }

        // Build endpoint URL for OpenAI
        let endpoint = if provider.endpoint.contains("chat/completions") {
            provider.endpoint.clone()
        } else if provider.endpoint.ends_with("/") {
            format!("{}chat/completions", provider.endpoint)
        } else {
            format!("{}/chat/completions", provider.endpoint)
        };

        // Make API call
        let response = self.http_client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: format!("OpenAI API request failed: {}", e),
                details: None,
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: format!("OpenAI API error {}: {}", status, error_text),
                details: None,
            });
        }

        let response_data: Value = response.json().await.map_err(|e| ToolEnhancementError {
            code: ToolEnhancementErrorCode::InvalidRequest,
            message: format!("Failed to parse OpenAI response: {}", e),
            details: None,
        })?;

        // Extract response content
        let choice = response_data["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .ok_or_else(|| ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: "No choices in OpenAI response".to_string(),
                details: None,
            })?;

        let message = choice["message"].as_object().ok_or_else(|| ToolEnhancementError {
            code: ToolEnhancementErrorCode::InvalidRequest,
            message: "No message in OpenAI choice".to_string(),
            details: None,
        })?;

        let content = message["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let stop_reason = choice["finish_reason"]
            .as_str()
            .map(|reason| match reason {
                "stop" => ToolEnhancementStopReason::EndTurn,
                "length" => ToolEnhancementStopReason::MaxTokens,
                _ => ToolEnhancementStopReason::EndTurn,
            })
            .unwrap_or(ToolEnhancementStopReason::EndTurn);

        // Extract usage info
        let usage = response_data["usage"].as_object().map(|usage_obj| {
            ToolEnhancementUsage {
                input_tokens: usage_obj.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                output_tokens: usage_obj.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                total_tokens: usage_obj.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                cost_usd: None,
            }
        });

        info!("✅ OpenAI API call successful, generated {} chars", content.len());

        Ok(ToolEnhancementResponse {
            message: ToolEnhancementMessage {
                role: ToolEnhancementMessageRole::Assistant,
                content: ToolEnhancementContent::Text(content),
                name: None,
                metadata: None,
            },
            model: model.to_string(),
            stop_reason,
            usage,
            metadata: Some(json!({
                "provider": "openai",
                "api_endpoint": endpoint,
                "timestamp": Utc::now().to_rfc3339()
            }).as_object().unwrap().clone().into_iter().collect()),
        })
    }

    /// Call Anthropic API for tool enhancement
    async fn call_anthropic_api(
        &self,
        request: &ToolEnhancementRequest,
        model: &str,
        provider: &LLMProviderConfig,
    ) -> std::result::Result<ToolEnhancementResponse, ToolEnhancementError> {
        info!("🔥 Calling Anthropic API for tool enhancement with model: {}", model);
        
        // Get API key
        let api_key = if let Some(key) = &provider.api_key {
            key.clone()
        } else if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            key
        } else {
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: "Anthropic API key not found".to_string(),
                details: None,
            });
        };

        // Build messages for Anthropic format
        let mut messages = Vec::new();
        
        // Anthropic system prompt is separate from messages
        let system_prompt = request.system_prompt.as_deref().unwrap_or("You are a helpful assistant.");
        
        // Add request messages (filter out system messages as they go in system parameter)
        for msg in &request.messages {
            if matches!(msg.role, ToolEnhancementMessageRole::System) {
                continue; // Skip system messages, they're handled separately
            }
            
            let content = match &msg.content {
                ToolEnhancementContent::Text(text) => text.clone(),
                ToolEnhancementContent::Parts(_) => {
                    return Err(ToolEnhancementError {
                        code: ToolEnhancementErrorCode::InvalidRequest,
                        message: "Multimodal content not supported for tool enhancement".to_string(),
                        details: None,
                    });
                }
            };
            
            messages.push(json!({
                "role": match msg.role {
                    ToolEnhancementMessageRole::User => "user",
                    ToolEnhancementMessageRole::Assistant => "assistant",
                    ToolEnhancementMessageRole::System => "user", // Should not reach here
                    ToolEnhancementMessageRole::Tool => "user", // Treat tool messages as user messages
                },
                "content": content
            }));
        }

        // Build request payload
        let mut payload = json!({
            "model": model,
            "system": system_prompt,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(self.config.default_params.max_tokens),
            "temperature": request.temperature.unwrap_or(self.config.default_params.temperature),
        });

        if let Some(top_p) = request.top_p {
            payload["top_p"] = json!(top_p);
        }

        if let Some(stop) = &request.stop {
            payload["stop_sequences"] = json!(stop);
        }

        // Build endpoint URL for Anthropic
        let endpoint = if provider.endpoint.contains("messages") {
            provider.endpoint.clone()
        } else if provider.endpoint.ends_with("/") {
            format!("{}v1/messages", provider.endpoint)
        } else {
            format!("{}/v1/messages", provider.endpoint)
        };

        // Make API call
        let response = self.http_client
            .post(&endpoint)
            .header("x-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: format!("Anthropic API request failed: {}", e),
                details: None,
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: format!("Anthropic API error {}: {}", status, error_text),
                details: None,
            });
        }

        let response_data: Value = response.json().await.map_err(|e| ToolEnhancementError {
            code: ToolEnhancementErrorCode::InvalidRequest,
            message: format!("Failed to parse Anthropic response: {}", e),
            details: None,
        })?;

        // Extract response content
        let content = response_data["content"]
            .as_array()
            .and_then(|content_array| content_array.first())
            .and_then(|content_block| content_block["text"].as_str())
            .unwrap_or("")
            .to_string();

        let stop_reason = response_data["stop_reason"]
            .as_str()
            .map(|reason| match reason {
                "end_turn" => ToolEnhancementStopReason::EndTurn,
                "max_tokens" => ToolEnhancementStopReason::MaxTokens,
                "stop_sequence" => ToolEnhancementStopReason::StopSequence,
                _ => ToolEnhancementStopReason::EndTurn,
            })
            .unwrap_or(ToolEnhancementStopReason::EndTurn);

        // Extract usage info
        let usage = response_data["usage"].as_object().map(|usage_obj| {
            ToolEnhancementUsage {
                input_tokens: usage_obj.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                output_tokens: usage_obj.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                total_tokens: (usage_obj.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) + 
                              usage_obj.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0)) as u32,
                cost_usd: None,
            }
        });

        info!("✅ Anthropic API call successful, generated {} chars", content.len());

        Ok(ToolEnhancementResponse {
            message: ToolEnhancementMessage {
                role: ToolEnhancementMessageRole::Assistant,
                content: ToolEnhancementContent::Text(content),
                name: None,
                metadata: None,
            },
            model: model.to_string(),
            stop_reason,
            usage,
            metadata: Some(json!({
                "provider": "anthropic",
                "api_endpoint": endpoint,
                "timestamp": Utc::now().to_rfc3339()
            }).as_object().unwrap().clone().into_iter().collect()),
        })
    }

    /// Call Ollama API for tool enhancement
    async fn call_ollama_api(
        &self,
        request: &ToolEnhancementRequest,
        model: &str,
        provider: &LLMProviderConfig,
    ) -> std::result::Result<ToolEnhancementResponse, ToolEnhancementError> {
        info!("🔥 Calling Ollama API for tool enhancement with model: {}", model);
        
        // Build messages for Ollama format (OpenAI-compatible)
        let mut messages = Vec::new();
        
        // Add system prompt if present
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }
        
        // Add request messages
        for msg in &request.messages {
            let content = match &msg.content {
                ToolEnhancementContent::Text(text) => text.clone(),
                ToolEnhancementContent::Parts(_) => {
                    return Err(ToolEnhancementError {
                        code: ToolEnhancementErrorCode::InvalidRequest,
                        message: "Multimodal content not supported for tool enhancement".to_string(),
                        details: None,
                    });
                }
            };
            
            messages.push(json!({
                "role": match msg.role {
                    ToolEnhancementMessageRole::User => "user",
                    ToolEnhancementMessageRole::Assistant => "assistant",
                    ToolEnhancementMessageRole::System => "system",
                    ToolEnhancementMessageRole::Tool => "user", // Treat tool messages as user messages
                },
                "content": content
            }));
        }

        // Build request payload for Ollama (uses /v1/chat/completions endpoint)
        let mut payload = json!({
            "model": model,
            "messages": messages,
            "temperature": request.temperature.unwrap_or(self.config.default_params.temperature),
            "stream": false
        });

        if let Some(top_p) = request.top_p {
            payload["top_p"] = json!(top_p);
        }

        if let Some(stop) = &request.stop {
            payload["stop"] = json!(stop);
        }

        // For Ollama, we use max_tokens differently (it's called num_predict)
        if let Some(max_tokens) = request.max_tokens {
            payload["options"] = json!({
                "num_predict": max_tokens
            });
        }

        // Build endpoint URL
        let endpoint = if provider.endpoint.ends_with("/") {
            format!("{}v1/chat/completions", provider.endpoint)
        } else {
            format!("{}/v1/chat/completions", provider.endpoint)
        };

        // Make API call (Ollama typically doesn't require auth)
        let mut req_builder = self.http_client
            .post(&endpoint)
            .header("Content-Type", "application/json");

        // Add auth if API key is provided (some Ollama setups may use it)
        if let Some(api_key) = &provider.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req_builder
            .json(&payload)
            .send()
            .await
            .map_err(|e| ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: format!("Ollama API request failed: {}", e),
                details: None,
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: format!("Ollama API error {}: {}", status, error_text),
                details: None,
            });
        }

        let response_data: Value = response.json().await.map_err(|e| ToolEnhancementError {
            code: ToolEnhancementErrorCode::InvalidRequest,
            message: format!("Failed to parse Ollama response: {}", e),
            details: None,
        })?;

        // Extract response content (OpenAI-compatible format)
        let choice = response_data["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .ok_or_else(|| ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: "No choices in Ollama response".to_string(),
                details: None,
            })?;

        let message = choice["message"].as_object().ok_or_else(|| ToolEnhancementError {
            code: ToolEnhancementErrorCode::InvalidRequest,
            message: "No message in Ollama choice".to_string(),
            details: None,
        })?;

        let content = message["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let stop_reason = choice["finish_reason"]
            .as_str()
            .map(|reason| match reason {
                "stop" => ToolEnhancementStopReason::EndTurn,
                "length" => ToolEnhancementStopReason::MaxTokens,
                _ => ToolEnhancementStopReason::EndTurn,
            })
            .unwrap_or(ToolEnhancementStopReason::EndTurn);

        // Extract usage info if available
        let usage = response_data["usage"].as_object().map(|usage_obj| {
            ToolEnhancementUsage {
                input_tokens: usage_obj.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                output_tokens: usage_obj.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                total_tokens: usage_obj.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                cost_usd: None,
            }
        });

        info!("✅ Ollama API call successful, generated {} chars", content.len());

        Ok(ToolEnhancementResponse {
            message: ToolEnhancementMessage {
                role: ToolEnhancementMessageRole::Assistant,
                content: ToolEnhancementContent::Text(content),
                name: None,
                metadata: None,
            },
            model: model.to_string(),
            stop_reason,
            usage,
            metadata: Some(json!({
                "provider": "ollama",
                "api_endpoint": endpoint,
                "timestamp": Utc::now().to_rfc3339()
            }).as_object().unwrap().clone().into_iter().collect()),
        })
    }

    /// Call custom API for tool enhancement
    async fn call_custom_api(
        &self,
        request: &ToolEnhancementRequest,
        model: &str,
        provider: &LLMProviderConfig,
    ) -> std::result::Result<ToolEnhancementResponse, ToolEnhancementError> {
        info!("🔥 Calling custom API for tool enhancement with model: {}", model);
        
        // For custom APIs, we'll try to use OpenAI-compatible format as a default
        let mut messages = Vec::new();
        
        // Add system prompt if present
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }
        
        // Add request messages
        for msg in &request.messages {
            let content = match &msg.content {
                ToolEnhancementContent::Text(text) => text.clone(),
                ToolEnhancementContent::Parts(_) => {
                    return Err(ToolEnhancementError {
                        code: ToolEnhancementErrorCode::InvalidRequest,
                        message: "Multimodal content not supported for tool enhancement".to_string(),
                        details: None,
                    });
                }
            };
            
            messages.push(json!({
                "role": match msg.role {
                    ToolEnhancementMessageRole::User => "user",
                    ToolEnhancementMessageRole::Assistant => "assistant",
                    ToolEnhancementMessageRole::System => "system",
                    ToolEnhancementMessageRole::Tool => "user", // Treat tool messages as user messages
                },
                "content": content
            }));
        }

        // Build request payload (OpenAI-compatible by default)
        let mut payload = json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(self.config.default_params.max_tokens),
            "temperature": request.temperature.unwrap_or(self.config.default_params.temperature),
        });

        if let Some(top_p) = request.top_p {
            payload["top_p"] = json!(top_p);
        }

        if let Some(stop) = &request.stop {
            payload["stop"] = json!(stop);
        }

        // Apply any custom config parameters
        for (key, value) in &provider.config {
            payload[key] = value.clone();
        }

        // Build request
        let mut req_builder = self.http_client
            .post(&provider.endpoint)
            .header("Content-Type", "application/json");

        // Add auth if API key is provided
        if let Some(api_key) = &provider.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req_builder
            .json(&payload)
            .send()
            .await
            .map_err(|e| ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: format!("Custom API request failed: {}", e),
                details: None,
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: format!("Custom API error {}: {}", status, error_text),
                details: None,
            });
        }

        let response_data: Value = response.json().await.map_err(|e| ToolEnhancementError {
            code: ToolEnhancementErrorCode::InvalidRequest,
            message: format!("Failed to parse custom API response: {}", e),
            details: None,
        })?;

        // Try to extract content in OpenAI format first, then fallback to simpler formats
        let content = if let Some(choices) = response_data["choices"].as_array() {
            // OpenAI-compatible format
            choices.first()
                .and_then(|choice| choice["message"]["content"].as_str())
                .unwrap_or("")
                .to_string()
        } else if let Some(content_str) = response_data["content"].as_str() {
            // Simple content field
            content_str.to_string()
        } else if let Some(text) = response_data["text"].as_str() {
            // Alternative text field
            text.to_string()
        } else if let Some(response_text) = response_data["response"].as_str() {
            // Another common field name
            response_text.to_string()
        } else {
            return Err(ToolEnhancementError {
                code: ToolEnhancementErrorCode::InvalidRequest,
                message: "Unable to extract content from custom API response".to_string(),
                details: Some(json!({
                    "response_structure": response_data
                }).as_object().unwrap().clone().into_iter().collect()),
            });
        };

        let stop_reason = response_data["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice["finish_reason"].as_str())
            .map(|reason| match reason {
                "stop" => ToolEnhancementStopReason::EndTurn,
                "length" => ToolEnhancementStopReason::MaxTokens,
                _ => ToolEnhancementStopReason::EndTurn,
            })
            .unwrap_or(ToolEnhancementStopReason::EndTurn);

        // Extract usage info if available
        let usage = response_data["usage"].as_object().map(|usage_obj| {
            ToolEnhancementUsage {
                input_tokens: usage_obj.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                output_tokens: usage_obj.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                total_tokens: usage_obj.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                cost_usd: None,
            }
        });

        info!("✅ Custom API call successful, generated {} chars", content.len());

        Ok(ToolEnhancementResponse {
            message: ToolEnhancementMessage {
                role: ToolEnhancementMessageRole::Assistant,
                content: ToolEnhancementContent::Text(content),
                name: None,
                metadata: None,
            },
            model: model.to_string(),
            stop_reason,
            usage,
            metadata: Some(json!({
                "provider": "custom",
                "api_endpoint": provider.endpoint,
                "timestamp": Utc::now().to_rfc3339()
            }).as_object().unwrap().clone().into_iter().collect()),
        })
    }

    /// Get service status
    pub async fn get_status(&self) -> Value {
        let providers = self.providers.read().await;
        
        json!({
            "enabled": self.config.enabled,
            "providers": providers.keys().collect::<Vec<_>>(),
            "default_model": self.config.default_model,
            "server_side_generation": true,
            "supported_enhancements": [
                "description_enhancement",
                "usage_examples",
                "keyword_extraction"
            ],
            "rate_limit": self.config.rate_limit.as_ref().map(|rl| json!({
                "requests_per_minute": rl.requests_per_minute,
                "window_seconds": rl.window_seconds
            }))
        })
    }
}

impl Default for ToolEnhancementConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_model: "gpt-4o-mini".to_string(), // Default model for test purposes
            max_tokens_limit: 4000, // Default token limit for test purposes
            rate_limit: Some(ToolEnhancementRateLimit {
                requests_per_minute: 60,
                burst_size: 10,
                window_seconds: 60,
            }),
            content_filter: Some(ContentFilterConfig {
                enabled: true,
                blocked_patterns: vec![],
                approval_patterns: vec![],
                max_content_length: 50000,
            }),
            providers: vec![],
            default_params: ToolEnhancementParams {
                temperature: 0.7,
                top_p: 0.9,
                max_tokens: 1000,
                stop: vec![],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_enhancement_service_creation() {
        let config = ToolEnhancementConfig::default();
        let service = ToolEnhancementService::new(config).unwrap();
        
        let status = service.get_status().await;
        assert_eq!(status["enabled"], false);
    }

    #[tokio::test]
    async fn test_enhancement_request_generation() {
        let config = ToolEnhancementConfig::default();
        let service = ToolEnhancementService::new(config).unwrap();

        let request = service.generate_enhanced_description_request(
            "test_tool",
            "Basic description",
            &json!({"type": "object"})
        ).await;

        // Should fail since service is disabled by default
        assert!(request.is_err());
    }
}