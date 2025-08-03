//! MCP Request Generator Service
//!
//! Coordinates server-side generation of sampling and elicitation requests
//! for MCP 2025-06-18 compliance. This service determines when to generate
//! requests and orchestrates the sampling and elicitation services.

use crate::config::Config;
use crate::error::{ProxyError, Result};
use crate::mcp::sampling::SamplingService;
use crate::mcp::elicitation::ElicitationService;
use crate::mcp::types::sampling::{SamplingRequest, SamplingResponse, SamplingError};
use crate::mcp::types::elicitation::{ElicitationRequest, ElicitationResponse, ElicitationError};
use crate::mcp::types::elicitation::ElicitationAction;
use crate::registry::types::ToolDefinition;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use chrono::{DateTime, Utc};

/// Configuration for request generation
#[derive(Debug, Clone)]
pub struct RequestGeneratorConfig {
    /// Whether request generation is enabled
    pub enabled: bool,
    /// Whether to automatically generate enhanced descriptions
    pub auto_generate_descriptions: bool,
    /// Whether to automatically generate usage examples
    pub auto_generate_examples: bool,
    /// Whether to automatically generate keywords
    pub auto_generate_keywords: bool,
    /// Maximum concurrent generation requests
    pub max_concurrent_requests: usize,
    /// Cache TTL for generated content (seconds)
    pub cache_ttl_seconds: u64,
    /// Whether to require approval for generated content
    pub require_approval: bool,
}

impl Default for RequestGeneratorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_generate_descriptions: true,
            auto_generate_examples: false, // More expensive, so off by default
            auto_generate_keywords: true,
            max_concurrent_requests: 5,
            cache_ttl_seconds: 3600, // 1 hour
            require_approval: false, // For development, enable in production
        }
    }
}

/// Result of request generation
#[derive(Debug, Clone)]
pub struct RequestGenerationResult {
    /// Whether generation was successful
    pub success: bool,
    /// Generated content (if successful)
    pub content: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Generation metadata
    pub metadata: RequestGenerationMetadata,
}

/// Metadata about request generation
#[derive(Debug, Clone)]
pub struct RequestGenerationMetadata {
    /// Type of generation performed
    pub generation_type: String,
    /// Tool name that was enhanced
    pub tool_name: String,
    /// Time taken for generation (milliseconds)
    pub generation_time_ms: u64,
    /// Model used for generation
    pub model_used: Option<String>,
    /// Confidence score (if applicable)
    pub confidence_score: Option<f64>,
    /// Whether this required approval
    pub requires_approval: bool,
    /// Timestamp of generation
    pub generated_at: DateTime<Utc>,
}

/// Cache entry for generated content
#[derive(Debug, Clone)]
struct GeneratedContentCache {
    /// The generated content
    content: String,
    /// Generation metadata
    metadata: RequestGenerationMetadata,
    /// When this was generated
    generated_at: DateTime<Utc>,
    /// When this expires
    expires_at: DateTime<Utc>,
    /// Whether this has been approved
    approved: bool,
}

/// MCP Request Generator Service
pub struct RequestGeneratorService {
    /// Service configuration
    config: RequestGeneratorConfig,
    /// Sampling service for description generation
    sampling_service: Arc<SamplingService>,
    /// Elicitation service for parameter collection
    elicitation_service: Arc<ElicitationService>,
    /// Cache for generated content
    content_cache: Arc<RwLock<HashMap<String, GeneratedContentCache>>>,
    /// Active generation requests
    active_requests: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl RequestGeneratorService {
    /// Create a new request generator service
    pub fn new(
        config: RequestGeneratorConfig,
        sampling_service: Arc<SamplingService>,
        elicitation_service: Arc<ElicitationService>,
    ) -> Self {
        Self {
            config,
            sampling_service,
            elicitation_service,
            content_cache: Arc::new(RwLock::new(HashMap::new())),
            active_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create from main config
    pub fn from_config(
        config: &Config,
        sampling_service: Arc<SamplingService>,
        elicitation_service: Arc<ElicitationService>,
    ) -> Self {
        let generator_config = RequestGeneratorConfig {
            enabled: config.smart_discovery.as_ref()
                .map(|sd| sd.enable_sampling.unwrap_or(false) || sd.enable_elicitation.unwrap_or(false))
                .unwrap_or(false),
            auto_generate_descriptions: config.smart_discovery.as_ref()
                .map(|sd| sd.enable_sampling.unwrap_or(false))
                .unwrap_or(false),
            auto_generate_examples: false, // Expensive, off by default
            auto_generate_keywords: config.smart_discovery.as_ref()
                .map(|sd| sd.enable_elicitation.unwrap_or(false))
                .unwrap_or(false),
            max_concurrent_requests: 5,
            cache_ttl_seconds: 3600,
            require_approval: true, // Enable approval in production
        };

        Self::new(
            generator_config,
            sampling_service,
            elicitation_service,
        )
    }

    /// Generate enhanced description for a tool
    pub async fn generate_enhanced_description(
        &self,
        tool_name: &str,
        tool_def: &ToolDefinition,
    ) -> Result<RequestGenerationResult> {
        if !self.config.enabled || !self.config.auto_generate_descriptions {
            return Ok(RequestGenerationResult {
                success: false,
                content: None,
                error: Some("Description generation is disabled".to_string()),
                metadata: RequestGenerationMetadata {
                    generation_type: "enhanced_description".to_string(),
                    tool_name: tool_name.to_string(),
                    generation_time_ms: 0,
                    model_used: None,
                    confidence_score: None,
                    requires_approval: self.config.require_approval,
                    generated_at: Utc::now(),
                },
            });
        }

        let cache_key = format!("desc_{}", tool_name);
        
        // Check cache first
        if let Some(cached) = self.get_cached_content(&cache_key).await {
            info!("📋 Using cached enhanced description for: {}", tool_name);
            return Ok(RequestGenerationResult {
                success: true,
                content: Some(cached.content),
                error: None,
                metadata: cached.metadata,
            });
        }

        // Check if already generating
        if self.is_generation_active(&cache_key).await {
            return Ok(RequestGenerationResult {
                success: false,
                content: None,
                error: Some("Generation already in progress".to_string()),
                metadata: RequestGenerationMetadata {
                    generation_type: "enhanced_description".to_string(),
                    tool_name: tool_name.to_string(),
                    generation_time_ms: 0,
                    model_used: None,
                    confidence_score: None,
                    requires_approval: self.config.require_approval,
                    generated_at: Utc::now(),
                },
            });
        }

        let start_time = std::time::Instant::now();
        self.mark_generation_active(&cache_key).await;

        let result = match self.sampling_service.generate_enhanced_description_request(
            tool_name,
            &tool_def.description,
            &serde_json::to_value(&tool_def.input_schema).unwrap_or(json!({})),
        ).await {
            Ok(request) => {
                info!("🎯 Generated sampling request for enhanced description: {}", tool_name);
                
                // Execute the request
                match self.sampling_service.execute_server_generated_request(request).await {
                    Ok(response) => {
                        let content = self.extract_content_from_sampling_response(&response);
                        let generation_time = start_time.elapsed().as_millis() as u64;
                        
                        let metadata = RequestGenerationMetadata {
                            generation_type: "enhanced_description".to_string(),
                            tool_name: tool_name.to_string(),
                            generation_time_ms: generation_time,
                            model_used: Some(response.model.clone()),
                            confidence_score: Some(0.8), // Default confidence for successful generation
                            requires_approval: self.config.require_approval,
                            generated_at: Utc::now(),
                        };

                        // Cache the result
                        self.cache_generated_content(&cache_key, &content, &metadata).await;

                        RequestGenerationResult {
                            success: true,
                            content: Some(content),
                            error: None,
                            metadata,
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute enhanced description request for '{}': {}", tool_name, e.message);
                        RequestGenerationResult {
                            success: false,
                            content: None,
                            error: Some(e.message),
                            metadata: RequestGenerationMetadata {
                                generation_type: "enhanced_description".to_string(),
                                tool_name: tool_name.to_string(),
                                generation_time_ms: start_time.elapsed().as_millis() as u64,
                                model_used: None,
                                confidence_score: None,
                                requires_approval: self.config.require_approval,
                                generated_at: Utc::now(),
                            },
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to generate enhanced description request for '{}': {}", tool_name, e.message);
                RequestGenerationResult {
                    success: false,
                    content: None,
                    error: Some(e.message),
                    metadata: RequestGenerationMetadata {
                        generation_type: "enhanced_description".to_string(),
                        tool_name: tool_name.to_string(),
                        generation_time_ms: start_time.elapsed().as_millis() as u64,
                        model_used: None,
                        confidence_score: None,
                        requires_approval: self.config.require_approval,
                        generated_at: Utc::now(),
                    },
                }
            }
        };

        self.mark_generation_complete(&cache_key).await;
        Ok(result)
    }

    /// Generate elicitation request for missing parameters
    pub async fn generate_parameter_elicitation(
        &self,
        tool_name: &str,
        tool_def: &ToolDefinition,
        missing_parameters: &[String],
        user_request: &str,
    ) -> Result<RequestGenerationResult> {
        if !self.config.enabled {
            return Ok(RequestGenerationResult {
                success: false,
                content: None,
                error: Some("Request generation is disabled".to_string()),
                metadata: RequestGenerationMetadata {
                    generation_type: "parameter_elicitation".to_string(),
                    tool_name: tool_name.to_string(),
                    generation_time_ms: 0,
                    model_used: None,
                    confidence_score: None,
                    requires_approval: false, // Parameter elicitation doesn't need approval
                    generated_at: Utc::now(),
                },
            });
        }

        let start_time = std::time::Instant::now();
        
        match self.elicitation_service.generate_parameter_elicitation_request(
            tool_name,
            &tool_def.description,
            &serde_json::to_value(&tool_def.input_schema).unwrap_or(json!({})),
            missing_parameters,
            user_request,
        ).await {
            Ok(request) => {
                info!("📝 Generated elicitation request for missing parameters: {} (missing: {:?})", 
                      tool_name, missing_parameters);
                
                // Process the request (store it for client interaction)
                match self.elicitation_service.process_server_generated_request(request.clone()).await {
                    Ok(request_id) => {
                        let generation_time = start_time.elapsed().as_millis() as u64;
                        
                        Ok(RequestGenerationResult {
                            success: true,
                            content: Some(request_id), // Return the request ID for tracking
                            error: None,
                            metadata: RequestGenerationMetadata {
                                generation_type: "parameter_elicitation".to_string(),
                                tool_name: tool_name.to_string(),
                                generation_time_ms: generation_time,
                                model_used: None, // Elicitation doesn't use a model directly
                                confidence_score: Some(1.0), // High confidence for parameter collection
                                requires_approval: false,
                                generated_at: Utc::now(),
                            },
                        })
                    }
                    Err(e) => {
                        error!("Failed to process elicitation request for '{}': {}", tool_name, e.message);
                        Ok(RequestGenerationResult {
                            success: false,
                            content: None,
                            error: Some(e.message),
                            metadata: RequestGenerationMetadata {
                                generation_type: "parameter_elicitation".to_string(),
                                tool_name: tool_name.to_string(),
                                generation_time_ms: start_time.elapsed().as_millis() as u64,
                                model_used: None,
                                confidence_score: None,
                                requires_approval: false,
                                generated_at: Utc::now(),
                            },
                        })
                    }
                }
            }
            Err(e) => {
                error!("Failed to generate parameter elicitation request for '{}': {}", tool_name, e.message);
                Ok(RequestGenerationResult {
                    success: false,
                    content: None,
                    error: Some(e.message),
                    metadata: RequestGenerationMetadata {
                        generation_type: "parameter_elicitation".to_string(),
                        tool_name: tool_name.to_string(),
                        generation_time_ms: start_time.elapsed().as_millis() as u64,
                        model_used: None,
                        confidence_score: None,
                        requires_approval: false,
                        generated_at: Utc::now(),
                    },
                })
            }
        }
    }

    /// Generate keywords for a tool
    pub async fn generate_tool_keywords(
        &self,
        tool_name: &str,
        tool_def: &ToolDefinition,
        existing_keywords: Option<&[String]>,
    ) -> Result<RequestGenerationResult> {
        if !self.config.enabled || !self.config.auto_generate_keywords {
            return Ok(RequestGenerationResult {
                success: false,
                content: None,
                error: Some("Keyword generation is disabled".to_string()),
                metadata: RequestGenerationMetadata {
                    generation_type: "keyword_extraction".to_string(),
                    tool_name: tool_name.to_string(),
                    generation_time_ms: 0,
                    model_used: None,
                    confidence_score: None,
                    requires_approval: self.config.require_approval,
                    generated_at: Utc::now(),
                },
            });
        }

        let cache_key = format!("keywords_{}", tool_name);
        
        // Check cache first
        if let Some(cached) = self.get_cached_content(&cache_key).await {
            info!("🏷️ Using cached keywords for: {}", tool_name);
            return Ok(RequestGenerationResult {
                success: true,
                content: Some(cached.content),
                error: None,
                metadata: cached.metadata,
            });
        }

        let start_time = std::time::Instant::now();
        
        match self.sampling_service.generate_keyword_extraction_request(
            tool_name,
            &tool_def.description,
            existing_keywords,
        ).await {
            Ok(request) => {
                info!("🔍 Generated sampling request for keyword extraction: {}", tool_name);
                
                // Execute the request
                match self.sampling_service.execute_server_generated_request(request).await {
                    Ok(response) => {
                        let content = self.extract_content_from_sampling_response(&response);
                        let generation_time = start_time.elapsed().as_millis() as u64;
                        
                        let metadata = RequestGenerationMetadata {
                            generation_type: "keyword_extraction".to_string(),
                            tool_name: tool_name.to_string(),
                            generation_time_ms: generation_time,
                            model_used: Some(response.model.clone()),
                            confidence_score: Some(0.7), // Moderate confidence for keyword extraction
                            requires_approval: self.config.require_approval,
                            generated_at: Utc::now(),
                        };

                        // Cache the result
                        self.cache_generated_content(&cache_key, &content, &metadata).await;

                        Ok(RequestGenerationResult {
                            success: true,
                            content: Some(content),
                            error: None,
                            metadata,
                        })
                    }
                    Err(e) => {
                        error!("Failed to execute keyword extraction request for '{}': {}", tool_name, e.message);
                        Ok(RequestGenerationResult {
                            success: false,
                            content: None,
                            error: Some(e.message),
                            metadata: RequestGenerationMetadata {
                                generation_type: "keyword_extraction".to_string(),
                                tool_name: tool_name.to_string(),
                                generation_time_ms: start_time.elapsed().as_millis() as u64,
                                model_used: None,
                                confidence_score: None,
                                requires_approval: self.config.require_approval,
                                generated_at: Utc::now(),
                            },
                        })
                    }
                }
            }
            Err(e) => {
                error!("Failed to generate keyword extraction request for '{}': {}", tool_name, e.message);
                Ok(RequestGenerationResult {
                    success: false,
                    content: None,
                    error: Some(e.message),
                    metadata: RequestGenerationMetadata {
                        generation_type: "keyword_extraction".to_string(),
                        tool_name: tool_name.to_string(),
                        generation_time_ms: start_time.elapsed().as_millis() as u64,
                        model_used: None,
                        confidence_score: None,
                        requires_approval: self.config.require_approval,
                        generated_at: Utc::now(),
                    },
                })
            }
        }
    }

    /// Extract content from sampling response
    fn extract_content_from_sampling_response(&self, response: &SamplingResponse) -> String {
        use crate::mcp::types::sampling::SamplingContent;
        
        match &response.message.content {
            SamplingContent::Text(text) => text.clone(),
            SamplingContent::Parts(parts) => {
                parts.iter()
                    .filter_map(|part| {
                        use crate::mcp::types::sampling::SamplingContentPart;
                        match part {
                            SamplingContentPart::Text { text } => Some(text.clone()),
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    }

    /// Get cached content if available and not expired
    async fn get_cached_content(&self, cache_key: &str) -> Option<GeneratedContentCache> {
        let cache = self.content_cache.read().await;
        if let Some(cached) = cache.get(cache_key) {
            if Utc::now() < cached.expires_at {
                return Some(cached.clone());
            }
        }
        None
    }

    /// Cache generated content
    async fn cache_generated_content(
        &self,
        cache_key: &str,
        content: &str,
        metadata: &RequestGenerationMetadata,
    ) {
        let mut cache = self.content_cache.write().await;
        let expires_at = Utc::now() + chrono::Duration::seconds(self.config.cache_ttl_seconds as i64);
        
        cache.insert(cache_key.to_string(), GeneratedContentCache {
            content: content.to_string(),
            metadata: metadata.clone(),
            generated_at: Utc::now(),
            expires_at,
            approved: !self.config.require_approval, // Auto-approve if approval not required
        });
        
        debug!("Cached generated content for key: {} (expires: {})", cache_key, expires_at);
    }

    /// Check if generation is already active for this key
    async fn is_generation_active(&self, cache_key: &str) -> bool {
        let active = self.active_requests.read().await;
        active.contains_key(cache_key)
    }

    /// Mark generation as active
    async fn mark_generation_active(&self, cache_key: &str) {
        let mut active = self.active_requests.write().await;
        active.insert(cache_key.to_string(), Utc::now());
    }

    /// Mark generation as complete
    async fn mark_generation_complete(&self, cache_key: &str) {
        let mut active = self.active_requests.write().await;
        active.remove(cache_key);
    }

    /// Clean up expired cache entries
    pub async fn cleanup_expired_cache(&self) {
        let mut cache = self.content_cache.write().await;
        let now = Utc::now();
        
        let expired_keys: Vec<String> = cache.iter()
            .filter(|(_, entry)| now > entry.expires_at)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in expired_keys {
            cache.remove(&key);
            debug!("Removed expired cache entry: {}", key);
        }
    }

    /// Get service status
    pub async fn get_status(&self) -> Value {
        let cache_count = self.content_cache.read().await.len();
        let active_count = self.active_requests.read().await.len();
        
        json!({
            "enabled": self.config.enabled,
            "auto_generate_descriptions": self.config.auto_generate_descriptions,
            "auto_generate_keywords": self.config.auto_generate_keywords,
            "cache_entries": cache_count,
            "active_requests": active_count,
            "max_concurrent_requests": self.config.max_concurrent_requests,
            "cache_ttl_seconds": self.config.cache_ttl_seconds,
            "require_approval": self.config.require_approval
        })
    }
}