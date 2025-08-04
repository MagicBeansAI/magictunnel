//! MCP Sampling service implementation
//!
//! Handles sampling requests for LLM message generation according to MCP 2025-06-18 specification

use crate::config::Config;
use crate::error::Result;
use crate::mcp::types::sampling::*;
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

/// Configuration for sampling service
#[derive(Debug, Clone)]
pub struct SamplingConfig {
    /// Whether sampling is enabled
    pub enabled: bool,
    /// Default model to use for sampling
    pub default_model: String,
    /// Maximum tokens allowed per request
    pub max_tokens_limit: u32,
    /// Rate limiting configuration
    pub rate_limit: Option<SamplingRateLimit>,
    /// Content filtering configuration
    pub content_filter: Option<ContentFilterConfig>,
    /// LLM provider configurations
    pub providers: Vec<LLMProviderConfig>,
    /// Default sampling parameters
    pub default_params: SamplingParams,
}

/// Rate limiting configuration for sampling
#[derive(Debug, Clone)]
pub struct SamplingRateLimit {
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

/// Default sampling parameters
#[derive(Debug, Clone)]
pub struct SamplingParams {
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

/// MCP Sampling service
pub struct SamplingService {
    /// Service configuration
    config: SamplingConfig,
    /// Rate limiting state by user
    rate_limits: Arc<RwLock<HashMap<String, RateLimitState>>>,
    /// LLM providers
    providers: Arc<RwLock<HashMap<String, LLMProviderConfig>>>,
    /// HTTP client for API calls
    http_client: Client,
}

impl SamplingService {
    /// Create a new sampling service
    pub fn new(config: SamplingConfig) -> Result<Self> {
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

    /// Create sampling service from main config
    pub fn from_config(config: &Config) -> Result<Self> {
        // Extract sampling configuration from main config
        let sampling_config = SamplingConfig {
            enabled: config.smart_discovery.as_ref()
                .map(|sd| sd.enabled)
                .unwrap_or(false),
            default_model: config.sampling.as_ref()
                .and_then(|s| s.llm_config.as_ref())
                .map(|llm| llm.model.clone())
                .or_else(|| config.sampling.as_ref().map(|s| s.default_model.clone()))
                .unwrap_or_else(|| "gpt-4o-mini".to_string()), // Use llm_config.model, fallback to default_model, then fallback
            max_tokens_limit: 4000,
            rate_limit: Some(SamplingRateLimit {
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
            providers: Self::create_default_providers(config),
            default_params: SamplingParams {
                temperature: 0.7,
                top_p: 0.9,
                max_tokens: 1000,
                stop: vec![],
            },
        };

        Self::new(sampling_config)
    }

    /// Create default LLM providers from config
    fn create_default_providers(config: &Config) -> Vec<LLMProviderConfig> {
        let mut providers = Vec::new();

        // Add OpenAI from LLM mapper config if available
        if let Some(smart_discovery) = &config.smart_discovery {
            let llm_mapper = &smart_discovery.llm_mapper;
            if llm_mapper.provider == "openai" {
                providers.push(LLMProviderConfig {
                    name: "openai".to_string(),
                    provider_type: LLMProviderType::OpenAI,
                    endpoint: llm_mapper.base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                    api_key: llm_mapper.api_key.clone(),
                    models: vec![llm_mapper.model.clone()],
                    config: HashMap::new(),
                });
            } else if llm_mapper.provider == "anthropic" {
                providers.push(LLMProviderConfig {
                    name: "anthropic".to_string(),
                    provider_type: LLMProviderType::Anthropic,
                    endpoint: llm_mapper.base_url.clone().unwrap_or_else(|| "https://api.anthropic.com".to_string()),
                    api_key: llm_mapper.api_key.clone(),
                    models: vec![llm_mapper.model.clone()],
                    config: HashMap::new(),
                });
            } else if llm_mapper.provider == "ollama" {
                providers.push(LLMProviderConfig {
                    name: "ollama".to_string(),
                    provider_type: LLMProviderType::Ollama,
                    endpoint: llm_mapper.base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string()),
                    api_key: None,
                    models: vec![llm_mapper.model.clone()],
                    config: HashMap::new(),
                });
            }
        }

        providers
    }

    /// Handle sampling request
    pub async fn handle_sampling_request(
        &self,
        request: SamplingRequest,
        user_id: Option<&str>,
    ) -> std::result::Result<SamplingResponse, SamplingError> {
        if !self.config.enabled {
            return Err(SamplingError {
                code: SamplingErrorCode::InvalidRequest,
                message: "Sampling is not enabled".to_string(),
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

        // Execute sampling request
        match self.execute_sampling(&request, &model).await {
            Ok(response) => {
                info!("Sampling request completed successfully for model: {}", model);
                Ok(response)
            }
            Err(e) => {
                error!("Sampling request failed: {}", e.message);
                Err(e)
            }
        }
    }

    /// Check rate limits for user
    async fn check_rate_limit(&self, user_id: &str) -> std::result::Result<(), SamplingError> {
        if let Some(rate_limit) = &self.config.rate_limit {
            let mut limits = self.rate_limits.write().await;
            let now = Utc::now();
            
            let state = limits.entry(user_id.to_string()).or_insert(RateLimitState {
                request_count: 0,
                window_start: now,
                last_request: now,
            });

            // Check if we need to reset the window
            let window_duration = chrono::Duration::seconds(rate_limit.window_seconds as i64);
            if now.signed_duration_since(state.window_start) > window_duration {
                state.request_count = 0;
                state.window_start = now;
            }

            // Check rate limit
            if state.request_count >= rate_limit.requests_per_minute {
                return Err(SamplingError {
                    code: SamplingErrorCode::RateLimitExceeded,
                    message: format!("Rate limit exceeded: {} requests per {} seconds", 
                        rate_limit.requests_per_minute, rate_limit.window_seconds),
                    details: Some(json!({
                        "limit": rate_limit.requests_per_minute,
                        "window_seconds": rate_limit.window_seconds,
                        "reset_time": state.window_start + window_duration
                    }).as_object().unwrap().clone().into_iter().collect()),
                });
            }

            // Update state
            state.request_count += 1;
            state.last_request = now;
        }

        Ok(())
    }

    /// Validate sampling request
    async fn validate_request(&self, request: &SamplingRequest) -> std::result::Result<(), SamplingError> {
        // Check message count
        if request.messages.is_empty() {
            return Err(SamplingError {
                code: SamplingErrorCode::InvalidRequest,
                message: "At least one message is required".to_string(),
                details: None,
            });
        }

        // Check max tokens
        if let Some(max_tokens) = request.max_tokens {
            if max_tokens > self.config.max_tokens_limit {
                return Err(SamplingError {
                    code: SamplingErrorCode::InvalidRequest,
                    message: format!("Max tokens {} exceeds limit {}", 
                        max_tokens, self.config.max_tokens_limit),
                    details: None,
                });
            }
        }

        // Validate temperature
        if let Some(temperature) = request.temperature {
            if temperature < 0.0 || temperature > 2.0 {
                return Err(SamplingError {
                    code: SamplingErrorCode::InvalidRequest,
                    message: "Temperature must be between 0.0 and 2.0".to_string(),
                    details: None,
                });
            }
        }

        // Validate top_p
        if let Some(top_p) = request.top_p {
            if top_p < 0.0 || top_p > 1.0 {
                return Err(SamplingError {
                    code: SamplingErrorCode::InvalidRequest,
                    message: "Top-p must be between 0.0 and 1.0".to_string(),
                    details: None,
                });
            }
        }

        Ok(())
    }

    /// Apply content filtering
    async fn apply_content_filter(&self, request: &SamplingRequest) -> std::result::Result<(), SamplingError> {
        if let Some(filter_config) = &self.config.content_filter {
            if filter_config.enabled {
                // Check each message for blocked content
                for message in &request.messages {
                    if let Err(e) = self.check_message_content(message, filter_config).await {
                        return Err(e);
                    }
                }

                // Check system prompt if present
                if let Some(system_prompt) = &request.system_prompt {
                    if let Err(e) = self.check_text_content(system_prompt, filter_config).await {
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Check message content for filtering
    async fn check_message_content(
        &self,
        message: &SamplingMessage,
        filter_config: &ContentFilterConfig,
    ) -> std::result::Result<(), SamplingError> {
        match &message.content {
            SamplingContent::Text(text) => {
                self.check_text_content(text, filter_config).await
            }
            SamplingContent::Parts(parts) => {
                for part in parts {
                    if let SamplingContentPart::Text { text } = part {
                        if let Err(e) = self.check_text_content(text, filter_config).await {
                            return Err(e);
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// Check text content against filter patterns
    async fn check_text_content(
        &self,
        text: &str,
        filter_config: &ContentFilterConfig,
    ) -> std::result::Result<(), SamplingError> {
        // Check length
        if text.len() > filter_config.max_content_length {
            return Err(SamplingError {
                code: SamplingErrorCode::ContentFiltered,
                message: format!("Content length {} exceeds maximum {}", 
                    text.len(), filter_config.max_content_length),
                details: None,
            });
        }

        // Check blocked patterns
        for pattern in &filter_config.blocked_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(text) {
                    return Err(SamplingError {
                        code: SamplingErrorCode::ContentFiltered,
                        message: "Content contains blocked patterns".to_string(),
                        details: Some(json!({
                            "pattern": pattern
                        }).as_object().unwrap().clone().into_iter().collect()),
                    });
                }
            }
        }

        Ok(())
    }

    /// Select appropriate model based on preferences
    async fn select_model(&self, preferences: &Option<ModelPreferences>) -> std::result::Result<String, SamplingError> {
        let providers = self.providers.read().await;
        
        if providers.is_empty() {
            return Err(SamplingError {
                code: SamplingErrorCode::ModelNotAvailable,
                message: "No LLM providers configured".to_string(),
                details: None,
            });
        }

        // If no preferences, use default model
        if preferences.is_none() {
            return Ok(self.config.default_model.clone());
        }

        let prefs = preferences.as_ref().unwrap();

        // Simple model selection based on preferences
        // In a real implementation, this would be more sophisticated
        if let Some(preferred_models) = &prefs.preferred_models {
            for model in preferred_models {
                // Check if any provider supports this model
                for provider in providers.values() {
                    if provider.models.contains(model) {
                        return Ok(model.clone());
                    }
                }
            }
        }

        // Fall back to intelligence-based selection
        let intelligence_priority = prefs.intelligence.unwrap_or(0.5);
        let speed_priority = prefs.speed.unwrap_or(0.5);
        let cost_priority = prefs.cost.unwrap_or(0.5);

        // Simple heuristic: high intelligence = best model, high speed = fastest model, high cost = cheapest model
        if intelligence_priority > 0.7 {
            // For high intelligence, prefer the configured default model first
            let configured_model = &self.config.default_model;
            
            // Check if any provider has the configured model
            for provider in providers.values() {
                if provider.models.iter().any(|m| m == configured_model) {
                    return Ok(configured_model.clone());
                }
            }
            
            // Fall back to provider-specific high-intelligence models
            if let Some(provider) = providers.get("openai") {
                // Look for GPT-4 variants if available
                if let Some(model) = provider.models.iter().find(|m| m.contains("gpt-4")) {
                    return Ok(model.clone());
                }
                // Fall back to first available model
                if let Some(model) = provider.models.first() {
                    return Ok(model.clone());
                }
            }
            if let Some(provider) = providers.get("anthropic") {
                // Look for Claude-3 variants if available
                if let Some(model) = provider.models.iter().find(|m| m.contains("claude-3") && (m.contains("sonnet") || m.contains("opus"))) {
                    return Ok(model.clone());
                }
                // Fall back to first available model
                if let Some(model) = provider.models.first() {
                    return Ok(model.clone());
                }
            }
        } else if speed_priority > 0.7 {
            // Look for fast models
            if let Some(provider) = providers.get("openai") {
                // Prefer GPT-3.5 variants for speed
                if let Some(model) = provider.models.iter().find(|m| m.contains("gpt-3.5")) {
                    return Ok(model.clone());
                }
                // Fall back to first available model
                if let Some(model) = provider.models.first() {
                    return Ok(model.clone());
                }
            }
        } else if cost_priority > 0.7 {
            // Look for cost-effective models (local/Ollama)
            if let Some(provider) = providers.get("ollama") {
                // Use first available Ollama model
                if let Some(model) = provider.models.first() {
                    return Ok(model.clone());
                }
            }
        }

        // Default fallback
        Ok(self.config.default_model.clone())
    }

    /// Execute the sampling request with selected model
    async fn execute_sampling(
        &self,
        request: &SamplingRequest,
        model: &str,
    ) -> std::result::Result<SamplingResponse, SamplingError> {
        debug!("Executing sampling request with model: {}", model);

        // Find the provider for this model
        let providers = self.providers.read().await;
        let provider = providers.values()
            .find(|p| p.models.contains(&model.to_string()))
            .ok_or_else(|| SamplingError {
                code: SamplingErrorCode::ModelNotAvailable,
                message: format!("No provider found for model: {}", model),
                details: None,
            })?;

        // Call the appropriate provider API with retry logic
        let provider_clone = provider.clone();
        let request_clone = request.clone();
        let model_clone = model.to_string();
        
        self.retry_api_call(|| {
            let provider = provider_clone.clone();
            let request = request_clone.clone();
            let model = model_clone.clone();
            
            async move {
                match provider.provider_type {
                    LLMProviderType::OpenAI => self.call_openai_api(&request, &model, &provider).await,
                    LLMProviderType::Anthropic => self.call_anthropic_api(&request, &model, &provider).await,
                    LLMProviderType::Ollama => self.call_ollama_api(&request, &model, &provider).await,
                    LLMProviderType::Custom => self.call_custom_api(&request, &model, &provider).await,
                }
            }
        }, 3).await // Retry up to 3 times
    }

    /// Generate sampling request for enhanced tool description (server-side generation)
    pub async fn generate_enhanced_description_request(
        &self,
        tool_name: &str,
        base_description: &str,
        tool_schema: &Value,
    ) -> std::result::Result<SamplingRequest, SamplingError> {
        if !self.config.enabled {
            return Err(SamplingError {
                code: SamplingErrorCode::InvalidRequest,
                message: "Sampling is not enabled".to_string(),
                details: None,
            });
        }

        info!("🎯 Generating sampling request for enhanced description: {}", tool_name);

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
        let user_message = SamplingMessage {
            role: SamplingMessageRole::User,
            content: SamplingContent::Text(
                format!("Please provide an enhanced description for the '{}' tool.", tool_name)
            ),
            name: None,
            metadata: Some(json!({
                "tool_name": tool_name,
                "enhancement_type": "description",
                "original_length": base_description.len()
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        // Create sampling request
        let request = SamplingRequest {
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
                "generated_by": "magictunnel_sampling_service",
                "timestamp": Utc::now().to_rfc3339()
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        debug!("Generated sampling request for tool '{}' description enhancement", tool_name);
        Ok(request)
    }

    /// Generate sampling request for tool usage examples
    pub async fn generate_usage_examples_request(
        &self,
        tool_name: &str,
        tool_description: &str,
        tool_schema: &Value,
        example_count: usize,
    ) -> std::result::Result<SamplingRequest, SamplingError> {
        if !self.config.enabled {
            return Err(SamplingError {
                code: SamplingErrorCode::InvalidRequest,
                message: "Sampling is not enabled".to_string(),
                details: None,
            });
        }

        info!("📚 Generating sampling request for usage examples: {} (count: {})", tool_name, example_count);

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

        let user_message = SamplingMessage {
            role: SamplingMessageRole::User,
            content: SamplingContent::Text(
                format!("Generate {} usage examples for the '{}' tool.", example_count, tool_name)
            ),
            name: None,
            metadata: Some(json!({
                "tool_name": tool_name,
                "enhancement_type": "usage_examples",
                "example_count": example_count
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        let request = SamplingRequest {
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
                "generated_by": "magictunnel_sampling_service",
                "timestamp": Utc::now().to_rfc3339()
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        debug!("Generated sampling request for '{}' usage examples", tool_name);
        Ok(request)
    }

    /// Generate sampling request for keyword extraction
    pub async fn generate_keyword_extraction_request(
        &self,
        tool_name: &str,
        tool_description: &str,
        existing_keywords: Option<&[String]>,
    ) -> std::result::Result<SamplingRequest, SamplingError> {
        if !self.config.enabled {
            return Err(SamplingError {
                code: SamplingErrorCode::InvalidRequest,
                message: "Sampling is not enabled".to_string(),
                details: None,
            });
        }

        info!("🔍 Generating sampling request for keyword extraction: {}", tool_name);

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

        let user_message = SamplingMessage {
            role: SamplingMessageRole::User,
            content: SamplingContent::Text(
                format!("Extract relevant keywords for the '{}' tool.", tool_name)
            ),
            name: None,
            metadata: Some(json!({
                "tool_name": tool_name,
                "enhancement_type": "keyword_extraction",
                "has_existing_keywords": existing_keywords.is_some()
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        let request = SamplingRequest {
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
                "generated_by": "magictunnel_sampling_service",
                "timestamp": Utc::now().to_rfc3339()
            }).as_object().unwrap().clone().into_iter().collect()),
        };

        debug!("Generated sampling request for '{}' keyword extraction", tool_name);
        Ok(request)
    }

    /// Execute a server-generated sampling request and return the response
    pub async fn execute_server_generated_request(
        &self,
        request: SamplingRequest,
    ) -> std::result::Result<SamplingResponse, SamplingError> {
        info!("🚀 Executing server-generated sampling request");
        
        // Use internal execution without rate limiting for server-generated requests
        self.execute_sampling_internal(&request).await
    }

    /// Internal sampling execution (bypasses rate limiting for server requests)
    async fn execute_sampling_internal(
        &self,
        request: &SamplingRequest,
    ) -> std::result::Result<SamplingResponse, SamplingError> {
        info!("🚀 Executing internal sampling request (server-generated)");
        
        // Use default model for internal requests
        let model = &self.config.default_model;
        
        // Find the provider for the default model
        let providers = self.providers.read().await;
        let provider = providers.values()
            .find(|p| p.models.contains(model))
            .ok_or_else(|| SamplingError {
                code: SamplingErrorCode::ModelNotAvailable,
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

    /// Call OpenAI API for sampling
    async fn call_openai_api(
        &self,
        request: &SamplingRequest,
        model: &str,
        provider: &LLMProviderConfig,
    ) -> std::result::Result<SamplingResponse, SamplingError> {
        let api_key = provider.api_key.as_ref().ok_or_else(|| SamplingError {
            code: SamplingErrorCode::InvalidRequest,
            message: "OpenAI API key is required".to_string(),
            details: None,
        })?;

        // Convert MCP sampling request to OpenAI format
        let mut messages = Vec::new();
        
        // Add system prompt if present
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }

        // Convert MCP messages to OpenAI format
        for msg in &request.messages {
            let role = match msg.role {
                SamplingMessageRole::User => "user",
                SamplingMessageRole::Assistant => "assistant",
                SamplingMessageRole::System => "system",
                SamplingMessageRole::Tool => "user", // Treat tool messages as user messages for OpenAI
            };

            let content = match &msg.content {
                SamplingContent::Text(text) => text.clone(),
                SamplingContent::Parts(parts) => {
                    // For OpenAI, concatenate text parts (simplified)
                    parts.iter()
                        .filter_map(|part| match part {
                            SamplingContentPart::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            };

            messages.push(json!({
                "role": role,
                "content": content
            }));
        }

        let mut openai_request = json!({
            "model": model,
            "messages": messages
        });

        // Add optional parameters
        if let Some(max_tokens) = request.max_tokens {
            openai_request["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = request.temperature {
            openai_request["temperature"] = json!(temperature);
        }
        if let Some(top_p) = request.top_p {
            openai_request["top_p"] = json!(top_p);
        }
        if let Some(stop) = &request.stop {
            openai_request["stop"] = json!(stop);
        }

        debug!("Sending OpenAI API request: {}", serde_json::to_string_pretty(&openai_request).unwrap_or_default());

        let response = self.http_client
            .post(&format!("{}/chat/completions", provider.endpoint))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| SamplingError {
                code: SamplingErrorCode::InternalError,
                message: format!("OpenAI API request failed: {}", e),
                details: None,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SamplingError {
                code: SamplingErrorCode::InternalError,
                message: format!("OpenAI API error {}: {}", status, error_text),
                details: None,
            });
        }

        let openai_response: Value = response.json().await.map_err(|e| SamplingError {
            code: SamplingErrorCode::InternalError,
            message: format!("Failed to parse OpenAI response: {}", e),
            details: None,
        })?;

        debug!("Received OpenAI API response: {}", serde_json::to_string_pretty(&openai_response).unwrap_or_default());

        // Convert OpenAI response to MCP format
        let choice = openai_response["choices"][0].as_object().ok_or_else(|| SamplingError {
            code: SamplingErrorCode::InternalError,
            message: "Invalid OpenAI response format".to_string(),
            details: None,
        })?;

        let message = choice["message"].as_object().ok_or_else(|| SamplingError {
            code: SamplingErrorCode::InternalError,
            message: "Missing message in OpenAI response".to_string(),
            details: None,
        })?;

        let content = message["content"].as_str().unwrap_or("").to_string();
        let finish_reason = choice["finish_reason"].as_str().unwrap_or("stop");

        let stop_reason = match finish_reason {
            "stop" => SamplingStopReason::EndTurn,
            "length" => SamplingStopReason::MaxTokens,
            _ => SamplingStopReason::EndTurn,
        };

        // Extract usage information
        let usage = if let Some(usage_obj) = openai_response["usage"].as_object() {
            Some(SamplingUsage {
                input_tokens: usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                output_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage_obj["total_tokens"].as_u64().unwrap_or(0) as u32,
                cost_usd: None, // Could calculate based on model pricing
            })
        } else {
            None
        };

        Ok(SamplingResponse {
            message: SamplingMessage {
                role: SamplingMessageRole::Assistant,
                content: SamplingContent::Text(content),
                name: None,
                metadata: None,
            },
            model: model.to_string(),
            stop_reason,
            usage,
            metadata: Some(json!({
                "provider": "openai",
                "request_id": openai_response["id"].as_str().unwrap_or("unknown"),
                "created": openai_response["created"].as_u64().unwrap_or(0)
            }).as_object().unwrap().clone().into_iter().collect()),
        })
    }

    /// Call Anthropic API for sampling
    async fn call_anthropic_api(
        &self,
        request: &SamplingRequest,
        model: &str,
        provider: &LLMProviderConfig,
    ) -> std::result::Result<SamplingResponse, SamplingError> {
        let api_key = provider.api_key.as_ref().ok_or_else(|| SamplingError {
            code: SamplingErrorCode::InvalidRequest,
            message: "Anthropic API key is required".to_string(),
            details: None,
        })?;

        // Convert MCP sampling request to Anthropic format
        let mut messages = Vec::new();
        let mut system_prompt = None;

        // Handle system prompt
        if let Some(system) = &request.system_prompt {
            system_prompt = Some(system.clone());
        }

        // Convert MCP messages to Anthropic format
        for msg in &request.messages {
            let role = match msg.role {
                SamplingMessageRole::User => "user",
                SamplingMessageRole::Assistant => "assistant",
                SamplingMessageRole::System => {
                    // Anthropic handles system messages differently
                    if system_prompt.is_none() {
                        if let SamplingContent::Text(text) = &msg.content {
                            system_prompt = Some(text.clone());
                        }
                    }
                    continue;
                }
                SamplingMessageRole::Tool => "user", // Treat tool messages as user messages for Anthropic
            };

            let content = match &msg.content {
                SamplingContent::Text(text) => text.clone(),
                SamplingContent::Parts(parts) => {
                    // For Anthropic, concatenate text parts (simplified)
                    parts.iter()
                        .filter_map(|part| match part {
                            SamplingContentPart::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            };

            messages.push(json!({
                "role": role,
                "content": content
            }));
        }

        let mut anthropic_request = json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(1000)
        });

        if let Some(system) = system_prompt {
            anthropic_request["system"] = json!(system);
        }

        // Add optional parameters
        if let Some(temperature) = request.temperature {
            anthropic_request["temperature"] = json!(temperature);
        }
        if let Some(top_p) = request.top_p {
            anthropic_request["top_p"] = json!(top_p);
        }
        if let Some(stop) = &request.stop {
            anthropic_request["stop_sequences"] = json!(stop);
        }

        debug!("Sending Anthropic API request: {}", serde_json::to_string_pretty(&anthropic_request).unwrap_or_default());

        let response = self.http_client
            .post(&format!("{}/v1/messages", provider.endpoint))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| SamplingError {
                code: SamplingErrorCode::InternalError,
                message: format!("Anthropic API request failed: {}", e),
                details: None,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SamplingError {
                code: SamplingErrorCode::InternalError,
                message: format!("Anthropic API error {}: {}", status, error_text),
                details: None,
            });
        }

        let anthropic_response: Value = response.json().await.map_err(|e| SamplingError {
            code: SamplingErrorCode::InternalError,
            message: format!("Failed to parse Anthropic response: {}", e),
            details: None,
        })?;

        debug!("Received Anthropic API response: {}", serde_json::to_string_pretty(&anthropic_response).unwrap_or_default());

        // Convert Anthropic response to MCP format
        let content_array = anthropic_response["content"].as_array().ok_or_else(|| SamplingError {
            code: SamplingErrorCode::InternalError,
            message: "Invalid Anthropic response format".to_string(),
            details: None,
        })?;

        let first_content = content_array.first().and_then(|c| c.as_object()).ok_or_else(|| SamplingError {
            code: SamplingErrorCode::InternalError,
            message: "Missing content in Anthropic response".to_string(),
            details: None,
        })?;

        let content = first_content["text"].as_str().unwrap_or("").to_string();
        let stop_reason_str = anthropic_response["stop_reason"].as_str().unwrap_or("end_turn");

        let stop_reason = match stop_reason_str {
            "end_turn" => SamplingStopReason::EndTurn,
            "max_tokens" => SamplingStopReason::MaxTokens,
            "stop_sequence" => SamplingStopReason::StopSequence,
            _ => SamplingStopReason::EndTurn,
        };

        // Extract usage information
        let usage = if let Some(usage_obj) = anthropic_response["usage"].as_object() {
            Some(SamplingUsage {
                input_tokens: usage_obj["input_tokens"].as_u64().unwrap_or(0) as u32,
                output_tokens: usage_obj["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: (usage_obj["input_tokens"].as_u64().unwrap_or(0) + usage_obj["output_tokens"].as_u64().unwrap_or(0)) as u32,
                cost_usd: None, // Could calculate based on model pricing
            })
        } else {
            None
        };

        Ok(SamplingResponse {
            message: SamplingMessage {
                role: SamplingMessageRole::Assistant,
                content: SamplingContent::Text(content),
                name: None,
                metadata: None,
            },
            model: model.to_string(),
            stop_reason,
            usage,
            metadata: Some(json!({
                "provider": "anthropic",
                "request_id": anthropic_response["id"].as_str().unwrap_or("unknown"),
                "model": anthropic_response["model"].as_str().unwrap_or(model)
            }).as_object().unwrap().clone().into_iter().collect()),
        })
    }

    /// Call Ollama API for sampling
    async fn call_ollama_api(
        &self,
        request: &SamplingRequest,
        model: &str,
        provider: &LLMProviderConfig,
    ) -> std::result::Result<SamplingResponse, SamplingError> {
        // Convert MCP sampling request to Ollama format
        let mut messages = Vec::new();

        // Add system prompt if present
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }

        // Convert MCP messages to Ollama format
        for msg in &request.messages {
            let role = match msg.role {
                SamplingMessageRole::User => "user",
                SamplingMessageRole::Assistant => "assistant",
                SamplingMessageRole::System => "system",
                SamplingMessageRole::Tool => "user", // Treat tool messages as user messages for Ollama
            };

            let content = match &msg.content {
                SamplingContent::Text(text) => text.clone(),
                SamplingContent::Parts(parts) => {
                    // For Ollama, concatenate text parts (simplified)
                    parts.iter()
                        .filter_map(|part| match part {
                            SamplingContentPart::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            };

            messages.push(json!({
                "role": role,
                "content": content
            }));
        }

        let mut ollama_request = json!({
            "model": model,
            "messages": messages,
            "stream": false
        });

        // Add optional parameters
        let mut options = json!({});
        if let Some(temperature) = request.temperature {
            options["temperature"] = json!(temperature);
        }
        if let Some(top_p) = request.top_p {
            options["top_p"] = json!(top_p);
        }
        if let Some(stop) = &request.stop {
            options["stop"] = json!(stop);
        }
        
        if !options.as_object().unwrap().is_empty() {
            ollama_request["options"] = options;
        }

        debug!("Sending Ollama API request: {}", serde_json::to_string_pretty(&ollama_request).unwrap_or_default());

        let response = self.http_client
            .post(&format!("{}/api/chat", provider.endpoint))
            .header("Content-Type", "application/json")
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| SamplingError {
                code: SamplingErrorCode::InternalError,
                message: format!("Ollama API request failed: {}", e),
                details: None,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SamplingError {
                code: SamplingErrorCode::InternalError,
                message: format!("Ollama API error {}: {}", status, error_text),
                details: None,
            });
        }

        let ollama_response: Value = response.json().await.map_err(|e| SamplingError {
            code: SamplingErrorCode::InternalError,
            message: format!("Failed to parse Ollama response: {}", e),
            details: None,
        })?;

        debug!("Received Ollama API response: {}", serde_json::to_string_pretty(&ollama_response).unwrap_or_default());

        // Convert Ollama response to MCP format
        let message = ollama_response["message"].as_object().ok_or_else(|| SamplingError {
            code: SamplingErrorCode::InternalError,
            message: "Invalid Ollama response format".to_string(),
            details: None,
        })?;

        let content = message["content"].as_str().unwrap_or("").to_string();
        let done = ollama_response["done"].as_bool().unwrap_or(true);

        let stop_reason = if done {
            SamplingStopReason::EndTurn
        } else {
            SamplingStopReason::MaxTokens
        };

        // Ollama doesn't provide detailed usage stats by default
        let usage = Some(SamplingUsage {
            input_tokens: 0, // Ollama doesn't report this
            output_tokens: 0, // Ollama doesn't report this
            total_tokens: 0,
            cost_usd: Some(0.0), // Ollama is typically free
        });

        Ok(SamplingResponse {
            message: SamplingMessage {
                role: SamplingMessageRole::Assistant,
                content: SamplingContent::Text(content),
                name: None,
                metadata: None,
            },
            model: model.to_string(),
            stop_reason,
            usage,
            metadata: Some(json!({
                "provider": "ollama",
                "model": ollama_response["model"].as_str().unwrap_or(model),
                "created_at": ollama_response["created_at"].as_str().unwrap_or("unknown"),
                "done": done
            }).as_object().unwrap().clone().into_iter().collect()),
        })
    }

    /// Call custom API for sampling (generic implementation)
    async fn call_custom_api(
        &self,
        request: &SamplingRequest,
        model: &str,
        provider: &LLMProviderConfig,
    ) -> std::result::Result<SamplingResponse, SamplingError> {
        warn!("Custom API provider not fully implemented, falling back to basic HTTP call");
        
        // Basic implementation - assumes OpenAI-compatible API
        self.call_openai_api(request, model, provider).await
    }

    /// Retry wrapper for API calls with exponential backoff
    async fn retry_api_call<F, Fut, T>(
        &self,
        operation: F,
        max_retries: u32,
    ) -> std::result::Result<T, SamplingError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, SamplingError>>,
    {
        let mut last_error = None;
        
        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e.clone());
                    
                    // Don't retry on certain error types
                    match e.code {
                        SamplingErrorCode::InvalidRequest 
                        | SamplingErrorCode::ContentFiltered 
                        | SamplingErrorCode::ModelNotAvailable => {
                            return Err(e);
                        }
                        _ => {}
                    }
                    
                    if attempt < max_retries {
                        let delay = Duration::from_millis(100 * (2_u64.pow(attempt)));
                        warn!("API call failed (attempt {}/{}), retrying in {:?}: {}", 
                              attempt + 1, max_retries + 1, delay, e.message);
                        sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| SamplingError {
            code: SamplingErrorCode::InternalError,
            message: "Max retries exceeded".to_string(),
            details: None,
        }))
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

    // Provider Management Methods

    /// List all configured LLM providers
    pub async fn list_providers(&self) -> Result<Vec<LLMProviderConfig>> {
        let providers = self.providers.read().await;
        Ok(providers.values().cloned().collect())
    }

    /// Add a new LLM provider
    pub async fn add_provider(&self, provider: LLMProviderConfig) -> Result<()> {
        let mut providers = self.providers.write().await;
        
        // Check if provider already exists
        if providers.contains_key(&provider.name) {
            return Err(crate::error::ProxyError::config(format!(
                "Provider '{}' already exists", provider.name
            )));
        }
        
        // Validate provider configuration
        self.validate_provider_config(&provider)?;
        
        providers.insert(provider.name.clone(), provider);
        Ok(())
    }

    /// Update an existing LLM provider
    pub async fn update_provider(&self, provider_name: &str, update: crate::web::dashboard::LlmProviderUpdateRequest) -> Result<()> {
        let mut providers = self.providers.write().await;
        
        let provider = providers.get_mut(provider_name)
            .ok_or_else(|| crate::error::ProxyError::config(format!(
                "Provider '{}' not found", provider_name
            )))?;

        // Update fields if provided
        if let Some(provider_type_str) = update.provider_type {
            provider.provider_type = match provider_type_str.as_str() {
                "openai" => LLMProviderType::OpenAI,
                "anthropic" => LLMProviderType::Anthropic,
                "ollama" => LLMProviderType::Ollama,
                "custom" => LLMProviderType::Custom,
                _ => return Err(crate::error::ProxyError::config(format!(
                    "Invalid provider type: {}", provider_type_str
                ))),
            };
        }

        if let Some(endpoint) = update.endpoint {
            provider.endpoint = endpoint;
        }

        if let Some(api_key) = update.api_key {
            provider.api_key = Some(api_key);
        }

        if let Some(models) = update.models {
            provider.models = models;
        }

        if let Some(config) = update.config {
            if let Some(config_obj) = config.as_object() {
                provider.config.extend(config_obj.clone());
            }
        }

        // Validate updated configuration
        self.validate_provider_config(provider)?;
        
        Ok(())
    }

    /// Remove an LLM provider
    pub async fn remove_provider(&self, provider_name: &str) -> Result<()> {
        let mut providers = self.providers.write().await;
        
        if providers.remove(provider_name).is_none() {
            return Err(crate::error::ProxyError::config(format!(
                "Provider '{}' not found", provider_name
            )));
        }
        
        Ok(())
    }

    /// Test an LLM provider connection
    pub async fn test_provider(&self, provider_name: &str, model: &Option<String>, test_prompt: &str, timeout_seconds: u64) -> Result<ProviderTestResult> {
        let providers = self.providers.read().await;
        let provider = providers.get(provider_name)
            .ok_or_else(|| crate::error::ProxyError::config(format!(
                "Provider '{}' not found", provider_name
            )))?;

        let test_model = model.as_ref().or_else(|| provider.models.first()).unwrap_or(&"default".to_string()).clone();
        
        // Create a simple test request
        let test_request = SamplingRequest {
            messages: vec![SamplingMessage {
                role: SamplingMessageRole::User,
                content: SamplingContent::Text(test_prompt.to_string()),
                name: None,
                metadata: None,
            }],
            model_preferences: Some(ModelPreferences {
                intelligence: None,
                speed: None,
                cost: None,
                preferred_models: Some(vec![test_model.clone()]),
                excluded_models: None,
            }),
            system_prompt: None,
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: None,
            stop: None,
            metadata: None,
        };

        // Make the test request with timeout
        let timeout_duration = tokio::time::Duration::from_secs(timeout_seconds);
        let test_future = self.make_llm_request(provider, &test_request, &test_model);
        
        match tokio::time::timeout(timeout_duration, test_future).await {
            Ok(Ok(response)) => {
                Ok(ProviderTestResult {
                    model_used: test_model,
                    response: match response.message.content {
                        SamplingContent::Text(text) => text,
                        _ => "Test successful".to_string(),
                    },
                })
            }
            Ok(Err(e)) => Err(crate::error::ProxyError::mcp(format!(
                "Provider test failed: {}", e
            ))),
            Err(_) => Err(crate::error::ProxyError::mcp(format!(
                "Provider test timed out after {} seconds", timeout_seconds
            ))),
        }
    }

    /// Get provider status and health information
    pub async fn get_provider_status(&self, provider_name: &str) -> Result<ProviderStatus> {
        let providers = self.providers.read().await;
        let _provider = providers.get(provider_name)
            .ok_or_else(|| crate::error::ProxyError::config(format!(
                "Provider '{}' not found", provider_name
            )))?;

        // For now, return basic status - can be enhanced with actual health checks
        Ok(ProviderStatus {
            status: "healthy".to_string(),
            last_check: Some(chrono::Utc::now().to_rfc3339()),
            details: Some("Provider is configured and available".to_string()),
            available_models: Some(_provider.models.clone()),
        })
    }

    /// Validate provider configuration
    fn validate_provider_config(&self, provider: &LLMProviderConfig) -> Result<()> {
        // Validate provider name
        if provider.name.trim().is_empty() {
            return Err(crate::error::ProxyError::config("Provider name cannot be empty".to_string()));
        }

        // Validate endpoint URL
        if provider.endpoint.trim().is_empty() {
            return Err(crate::error::ProxyError::config("Provider endpoint cannot be empty".to_string()));
        }

        // Validate that endpoint is a valid URL
        if let Err(_) = Url::parse(&provider.endpoint) {
            return Err(crate::error::ProxyError::config(format!(
                "Invalid endpoint URL: {}", provider.endpoint
            )));
        }

        // Validate models list
        if provider.models.is_empty() {
            return Err(crate::error::ProxyError::config("Provider must have at least one model".to_string()));
        }

        Ok(())
    }

    /// Make LLM request to a specific provider (helper method for testing)
    async fn make_llm_request(&self, provider: &LLMProviderConfig, request: &SamplingRequest, model: &str) -> Result<SamplingResponse> {
        // This is a simplified implementation for testing
        // In a real implementation, you would make actual HTTP requests to the provider
        
        match provider.provider_type {
            LLMProviderType::OpenAI => {
                // Simulate OpenAI API call
                if provider.api_key.is_none() {
                    return Err(crate::error::ProxyError::mcp("OpenAI API key required".to_string()));
                }
                
                // Return a mock response for now
                Ok(SamplingResponse {
                    message: SamplingMessage {
                        role: SamplingMessageRole::Assistant,
                        content: SamplingContent::Text("Test response from OpenAI provider".to_string()),
                        name: None,
                        metadata: None,
                    },
                    model: model.to_string(),
                    stop_reason: SamplingStopReason::EndTurn,
                    usage: None,
                    metadata: None,
                })
            }
            LLMProviderType::Anthropic => {
                // Simulate Anthropic API call
                if provider.api_key.is_none() {
                    return Err(crate::error::ProxyError::mcp("Anthropic API key required".to_string()));
                }
                
                Ok(SamplingResponse {
                    message: SamplingMessage {
                        role: SamplingMessageRole::Assistant,
                        content: SamplingContent::Text("Test response from Anthropic provider".to_string()),
                        name: None,
                        metadata: None,
                    },
                    model: model.to_string(),
                    stop_reason: SamplingStopReason::EndTurn,
                    usage: None,
                    metadata: None,
                })
            }
            LLMProviderType::Ollama => {
                // Simulate Ollama API call (no API key needed)
                Ok(SamplingResponse {
                    message: SamplingMessage {
                        role: SamplingMessageRole::Assistant,
                        content: SamplingContent::Text("Test response from Ollama provider".to_string()),
                        name: None,
                        metadata: None,
                    },
                    model: model.to_string(),
                    stop_reason: SamplingStopReason::EndTurn,
                    usage: None,
                    metadata: None,
                })
            }
            LLMProviderType::Custom => {
                // Simulate custom provider call
                Ok(SamplingResponse {
                    message: SamplingMessage {
                        role: SamplingMessageRole::Assistant,
                        content: SamplingContent::Text("Test response from custom provider".to_string()),
                        name: None,
                        metadata: None,
                    },
                    model: model.to_string(),
                    stop_reason: SamplingStopReason::EndTurn,
                    usage: None,
                    metadata: None,
                })
            }
        }
    }
}

/// Result of provider connection test
#[derive(Debug, Clone)]
pub struct ProviderTestResult {
    pub model_used: String,
    pub response: String,
}

/// Provider status information
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub status: String,
    pub last_check: Option<String>,
    pub details: Option<String>,
    pub available_models: Option<Vec<String>>,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_model: "gpt-4o-mini".to_string(), // Reasonable default, should be overridden in config
            max_tokens_limit: 4000,
            rate_limit: Some(SamplingRateLimit {
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
            default_params: SamplingParams {
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
    async fn test_sampling_service_creation() {
        let config = SamplingConfig::default();
        let service = SamplingService::new(config).unwrap();
        
        let status = service.get_status().await;
        assert_eq!(status["enabled"], false);
    }

    #[tokio::test]
    async fn test_request_validation() {
        let config = SamplingConfig::default();
        let service = SamplingService::new(config).unwrap();

        // Test empty messages
        let empty_request = SamplingRequest {
            messages: vec![],
            model_preferences: None,
            system_prompt: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
            metadata: None,
        };

        let result = service.validate_request(&empty_request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, SamplingErrorCode::InvalidRequest);
    }

    #[tokio::test]
    async fn test_content_filtering() {
        let mut config = SamplingConfig::default();
        config.content_filter = Some(ContentFilterConfig {
            enabled: true,
            blocked_patterns: vec!["password".to_string()],
            approval_patterns: vec![],
            max_content_length: 100,
        });

        let service = SamplingService::new(config).unwrap();

        let request = SamplingRequest {
            messages: vec![SamplingMessage {
                role: SamplingMessageRole::User,
                content: SamplingContent::Text("What is my password?".to_string()),
                name: None,
                metadata: None,
            }],
            model_preferences: None,
            system_prompt: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
            metadata: None,
        };

        let result = service.apply_content_filter(&request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, SamplingErrorCode::ContentFiltered);
    }
}