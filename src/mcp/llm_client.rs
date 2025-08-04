//! LLM Client for MagicTunnel-handled sampling and elicitation requests

use crate::config::LlmConfig;
use crate::error::{ProxyError, Result};
use crate::mcp::types::{SamplingRequest, SamplingResponse, SamplingMessage, SamplingMessageRole, SamplingContent, SamplingStopReason, SamplingUsage, ElicitationRequest, ElicitationResponse};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, error, info};

/// LLM client for handling sampling and elicitation requests using configured LLM providers
#[derive(Debug, Clone)]
pub struct LlmClient {
    /// LLM configuration
    config: LlmConfig,
    /// HTTP client for API calls
    http_client: reqwest::Client,
}

impl LlmClient {
    /// Create a new LLM client with the given configuration
    pub fn new(config: LlmConfig) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ProxyError::config(&format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Process a sampling request using the configured LLM provider
    pub async fn handle_sampling_request(&self, request: &SamplingRequest) -> Result<SamplingResponse> {
        debug!("Processing sampling request with LLM provider: {}", self.config.provider);

        match self.config.provider.as_str() {
            "openai" => self.handle_openai_sampling(request).await,
            "anthropic" => self.handle_anthropic_sampling(request).await,
            "ollama" => self.handle_ollama_sampling(request).await,
            _ => Err(ProxyError::config(&format!("Unsupported LLM provider: {}", self.config.provider)))
        }
    }

    /// Process an elicitation request using the configured LLM provider
    pub async fn handle_elicitation_request(&self, request: &ElicitationRequest) -> Result<ElicitationResponse> {
        debug!("Processing elicitation request with LLM provider: {}", self.config.provider);

        // Convert elicitation request to a sampling request for LLM processing
        let sampling_request = self.convert_elicitation_to_sampling(request)?;
        let sampling_response = self.handle_sampling_request(&sampling_request).await?;
        
        // Convert back to elicitation response
        self.convert_sampling_to_elicitation(sampling_response, request)
    }

    /// Handle OpenAI sampling requests
    async fn handle_openai_sampling(&self, request: &SamplingRequest) -> Result<SamplingResponse> {
        let api_key = self.get_api_key()?;
        let base_url = self.config.api_base_url.as_deref().unwrap_or("https://api.openai.com/v1");

        // Convert MCP request to OpenAI format
        let openai_messages = self.convert_mcp_to_openai_messages(request)?;
        
        let openai_request = json!({
            "model": self.config.model,
            "messages": openai_messages,
            "max_tokens": self.config.max_tokens.unwrap_or(4000),
            "temperature": self.config.temperature.unwrap_or(0.7),
        });

        let response = self.http_client
            .post(&format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| ProxyError::routing(&format!("OpenAI API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProxyError::routing(&format!("OpenAI API error: {}", error_text)));
        }

        let openai_response: Value = response.json().await
            .map_err(|e| ProxyError::routing(&format!("Failed to parse OpenAI response: {}", e)))?;

        self.convert_openai_to_mcp_response(openai_response)
    }

    /// Handle Anthropic sampling requests
    async fn handle_anthropic_sampling(&self, request: &SamplingRequest) -> Result<SamplingResponse> {
        let api_key = self.get_api_key()?;
        let base_url = self.config.api_base_url.as_deref().unwrap_or("https://api.anthropic.com/v1");

        // Convert MCP request to Anthropic format
        let (system_prompt, anthropic_messages) = self.convert_mcp_to_anthropic_messages(request)?;
        
        let mut anthropic_request = json!({
            "model": self.config.model,
            "messages": anthropic_messages,
            "max_tokens": self.config.max_tokens.unwrap_or(4000),
        });

        if let Some(system) = system_prompt {
            anthropic_request["system"] = json!(system);
        }

        let response = self.http_client
            .post(&format!("{}/messages", base_url))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| ProxyError::routing(&format!("Anthropic API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProxyError::routing(&format!("Anthropic API error: {}", error_text)));
        }

        let anthropic_response: Value = response.json().await
            .map_err(|e| ProxyError::routing(&format!("Failed to parse Anthropic response: {}", e)))?;

        self.convert_anthropic_to_mcp_response(anthropic_response)
    }

    /// Handle Ollama sampling requests
    async fn handle_ollama_sampling(&self, request: &SamplingRequest) -> Result<SamplingResponse> {
        let base_url = self.config.api_base_url.as_deref().unwrap_or("http://localhost:11434");

        // Convert MCP request to Ollama format
        let ollama_messages = self.convert_mcp_to_ollama_messages(request)?;
        
        let ollama_request = json!({
            "model": self.config.model,
            "messages": ollama_messages,
            "stream": false,
            "options": {
                "num_predict": self.config.max_tokens.unwrap_or(4000),
                "temperature": self.config.temperature.unwrap_or(0.7),
            }
        });

        let response = self.http_client
            .post(&format!("{}/api/chat", base_url))
            .header("Content-Type", "application/json")
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| ProxyError::routing(&format!("Ollama API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProxyError::routing(&format!("Ollama API error: {}", error_text)));
        }

        let ollama_response: Value = response.json().await
            .map_err(|e| ProxyError::routing(&format!("Failed to parse Ollama response: {}", e)))?;

        self.convert_ollama_to_mcp_response(ollama_response)
    }

    /// Get API key from environment variable
    fn get_api_key(&self) -> Result<String> {
        if let Some(env_var) = &self.config.api_key_env {
            std::env::var(env_var)
                .map_err(|_| ProxyError::config(&format!("Environment variable {} not found", env_var)))
        } else {
            Err(ProxyError::config("No API key environment variable configured"))
        }
    }

    /// Convert MCP messages to OpenAI format
    fn convert_mcp_to_openai_messages(&self, request: &SamplingRequest) -> Result<Vec<Value>> {
        let mut messages = Vec::new();

        // Add system message if present
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }

        // Convert MCP messages
        for msg in &request.messages {
            let role = match msg.role {
                SamplingMessageRole::User => "user",
                SamplingMessageRole::Assistant => "assistant",
                SamplingMessageRole::System => "system",
                SamplingMessageRole::Tool => "tool",
            };

            let content = match &msg.content {
                SamplingContent::Text(text) => json!(text),
                SamplingContent::Parts(parts) => {
                    // For now, just extract text parts
                    let text_parts: Vec<String> = parts.iter()
                        .filter_map(|part| match part {
                            crate::mcp::types::SamplingContentPart::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect();
                    json!(text_parts.join("\n"))
                }
            };

            messages.push(json!({
                "role": role,
                "content": content
            }));
        }

        Ok(messages)
    }

    /// Convert MCP messages to Anthropic format
    fn convert_mcp_to_anthropic_messages(&self, request: &SamplingRequest) -> Result<(Option<String>, Vec<Value>)> {
        let mut messages = Vec::new();
        let system_prompt = request.system_prompt.clone();

        // Convert MCP messages (skip system messages as they go in system field)
        for msg in &request.messages {
            if matches!(msg.role, SamplingMessageRole::System) {
                continue; // System messages handled separately in Anthropic
            }

            let role = match msg.role {
                SamplingMessageRole::User => "user",
                SamplingMessageRole::Assistant => "assistant",
                SamplingMessageRole::System => continue,
                SamplingMessageRole::Tool => "tool",
            };

            let content = match &msg.content {
                SamplingContent::Text(text) => text.clone(),
                SamplingContent::Parts(parts) => {
                    // Extract text parts
                    parts.iter()
                        .filter_map(|part| match part {
                            crate::mcp::types::SamplingContentPart::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<String>>()
                        .join("\n")
                }
            };

            messages.push(json!({
                "role": role,
                "content": content
            }));
        }

        Ok((system_prompt, messages))
    }

    /// Convert MCP messages to Ollama format
    fn convert_mcp_to_ollama_messages(&self, request: &SamplingRequest) -> Result<Vec<Value>> {
        self.convert_mcp_to_openai_messages(request) // Ollama uses similar format to OpenAI
    }

    /// Convert OpenAI response to MCP format
    fn convert_openai_to_mcp_response(&self, response: Value) -> Result<SamplingResponse> {
        let choices = response.get("choices")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ProxyError::routing("Invalid OpenAI response: missing choices"))?;

        let choice = choices.first()
            .ok_or_else(|| ProxyError::routing("Invalid OpenAI response: empty choices"))?;

        let message = choice.get("message")
            .ok_or_else(|| ProxyError::routing("Invalid OpenAI response: missing message"))?;

        let content = message.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::routing("Invalid OpenAI response: missing content"))?;

        let usage = response.get("usage");
        let mcp_usage = usage.map(|u| SamplingUsage {
            input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            output_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            cost_usd: None, // Would need rate calculation
        });

        Ok(SamplingResponse {
            message: SamplingMessage {
                role: SamplingMessageRole::Assistant,
                content: SamplingContent::Text(content.to_string()),
                name: Some("MagicTunnel-LLM".to_string()),
                metadata: Some([
                    ("provider".to_string(), json!("openai")),
                    ("model".to_string(), json!(self.config.model)),
                ].into_iter().collect()),
            },
            model: self.config.model.clone(),
            stop_reason: SamplingStopReason::EndTurn,
            usage: mcp_usage,
            metadata: Some([
                ("llm_provider".to_string(), json!("openai")),
                ("magictunnel_handled".to_string(), json!(true)),
            ].into_iter().collect()),
        })
    }

    /// Convert Anthropic response to MCP format
    fn convert_anthropic_to_mcp_response(&self, response: Value) -> Result<SamplingResponse> {
        let content_array = response.get("content")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ProxyError::routing("Invalid Anthropic response: missing content array"))?;

        let text_content = content_array.iter()
            .find_map(|item| {
                if item.get("type")?.as_str()? == "text" {
                    item.get("text")?.as_str()
                } else {
                    None
                }
            })
            .ok_or_else(|| ProxyError::routing("Invalid Anthropic response: no text content"))?;

        let usage = response.get("usage");
        let mcp_usage = usage.map(|u| SamplingUsage {
            input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: 0, // Calculate from input + output
            cost_usd: None,
        });

        Ok(SamplingResponse {
            message: SamplingMessage {
                role: SamplingMessageRole::Assistant,
                content: SamplingContent::Text(text_content.to_string()),
                name: Some("MagicTunnel-LLM".to_string()),
                metadata: Some([
                    ("provider".to_string(), json!("anthropic")),
                    ("model".to_string(), json!(self.config.model)),
                ].into_iter().collect()),
            },
            model: self.config.model.clone(),
            stop_reason: SamplingStopReason::EndTurn,
            usage: mcp_usage,
            metadata: Some([
                ("llm_provider".to_string(), json!("anthropic")),
                ("magictunnel_handled".to_string(), json!(true)),
            ].into_iter().collect()),
        })
    }

    /// Convert Ollama response to MCP format
    fn convert_ollama_to_mcp_response(&self, response: Value) -> Result<SamplingResponse> {
        let message = response.get("message")
            .ok_or_else(|| ProxyError::routing("Invalid Ollama response: missing message"))?;

        let content = message.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::routing("Invalid Ollama response: missing content"))?;

        Ok(SamplingResponse {
            message: SamplingMessage {
                role: SamplingMessageRole::Assistant,
                content: SamplingContent::Text(content.to_string()),
                name: Some("MagicTunnel-LLM".to_string()),
                metadata: Some([
                    ("provider".to_string(), json!("ollama")),
                    ("model".to_string(), json!(self.config.model)),
                ].into_iter().collect()),
            },
            model: self.config.model.clone(),
            stop_reason: SamplingStopReason::EndTurn,
            usage: None, // Ollama doesn't provide usage stats in this format
            metadata: Some([
                ("llm_provider".to_string(), json!("ollama")),
                ("magictunnel_handled".to_string(), json!(true)),
            ].into_iter().collect()),
        })
    }

    /// Convert elicitation request to sampling request for LLM processing
    fn convert_elicitation_to_sampling(&self, request: &ElicitationRequest) -> Result<SamplingRequest> {
        let prompt_text = format!(
            "Please analyze the following JSON schema and generate a user-friendly prompt that would elicit data matching this schema:\n\nSchema:\n{}\n\nContext: {}\n\nGenerate a clear, concise prompt that explains what information is needed and how it should be structured.",
            serde_json::to_string_pretty(&request.requested_schema)
                .map_err(|e| ProxyError::validation(&format!("Invalid schema: {}", e)))?,
            request.context.as_ref().map(|c| c.reason.as_deref().unwrap_or("No reason provided")).unwrap_or("No additional context provided")
        );

        Ok(SamplingRequest {
            system_prompt: Some("You are an expert at creating user-friendly prompts from JSON schemas. Generate clear, helpful prompts that guide users to provide properly structured data.".to_string()),
            messages: vec![crate::mcp::types::SamplingMessage {
                role: SamplingMessageRole::User,
                content: SamplingContent::Text(prompt_text),
                name: None,
                metadata: None,
            }],
            model_preferences: Some(crate::mcp::types::sampling::ModelPreferences {
                intelligence: None,
                speed: None,
                cost: None,
                preferred_models: Some(vec![self.config.model.clone()]),
                excluded_models: None,
            }),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            top_p: None,
            stop: None,
            metadata: None,
        })
    }

    /// Convert sampling response back to elicitation response
    fn convert_sampling_to_elicitation(&self, sampling_response: SamplingResponse, original_request: &ElicitationRequest) -> Result<ElicitationResponse> {
        let prompt = match sampling_response.message.content {
            SamplingContent::Text(text) => text,
            SamplingContent::Parts(parts) => {
                parts.iter()
                    .filter_map(|part| match part {
                        crate::mcp::types::SamplingContentPart::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<String>>()
                    .join("\n")
            }
        };

        Ok(ElicitationResponse {
            action: crate::mcp::types::elicitation::ElicitationAction::Accept,
            data: Some(json!({
                "generated_prompt": prompt,
                "schema": original_request.requested_schema.clone()
            })),
            reason: Some("Generated prompt using LLM".to_string()),
            metadata: Some([
                ("llm_provider".to_string(), json!(self.config.provider)),
                ("model".to_string(), json!(self.config.model)),
                ("magictunnel_handled".to_string(), json!(true)),
                ("response_type".to_string(), json!("generated_prompt")),
            ].into_iter().collect()),
            timestamp: Some(chrono::Utc::now()),
        })
    }
}