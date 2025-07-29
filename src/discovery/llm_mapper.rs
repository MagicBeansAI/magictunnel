//! LLM Parameter Mapping
//!
//! This module implements LLM-based parameter mapping that converts natural language
//! requests into structured tool parameters using various LLM providers.

use crate::discovery::types::*;
use crate::error::{ProxyError, Result};
use crate::registry::types::ToolDefinition;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout as tokio_timeout;
use tracing::{debug, error, info, warn};

/// Configuration for LLM parameter mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMapperConfig {
    /// LLM provider (openai, anthropic, ollama, etc.)
    pub provider: String,
    
    /// Model name to use
    pub model: String,
    
    /// API key (if required)
    pub api_key: Option<String>,
    
    /// Environment variable name for API key (if using env var)
    pub api_key_env: Option<String>,
    
    /// Base URL for API (if different from default)
    pub base_url: Option<String>,
    
    /// Request timeout in seconds
    pub timeout: u64,
    
    /// Maximum retries for failed requests
    pub max_retries: u32,
    
    /// Whether to enable parameter mapping
    pub enabled: bool,
}

impl Default for LlmMapperConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: None,
            api_key_env: Some("OPENAI_API_KEY".to_string()),
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: true,
        }
    }
}

/// LLM parameter mapper service
pub struct LlmParameterMapper {
    config: LlmMapperConfig,
    client: Client,
}

/// OpenAI API request structure
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    max_tokens: Option<u32>,
}

/// OpenAI message structure
#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

/// OpenAI API response structure
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

/// OpenAI choice structure
#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
}

/// OpenAI response message structure
#[derive(Debug, Deserialize)]
struct OpenAIResponseMessage {
    content: String,
}

impl LlmParameterMapper {
    /// Create a new LLM parameter mapper
    pub fn new(config: LlmMapperConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .danger_accept_invalid_certs(false)
            .use_rustls_tls()
            .tls_built_in_root_certs(true) // Use built-in webpki root certificates
            .build()
            .map_err(|e| ProxyError::routing(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Create a new LLM parameter mapper with default configuration
    pub fn new_with_defaults() -> Result<Self> {
        Self::new(LlmMapperConfig::default())
    }

    /// Extract parameters from a natural language request using LLM
    pub async fn extract_parameters(
        &self,
        request: &SmartDiscoveryRequest,
        tool_def: &ToolDefinition,
    ) -> Result<ParameterExtraction> {
        if !self.config.enabled {
            return Ok(ParameterExtraction {
                parameters: HashMap::new(),
                status: ExtractionStatus::Failed,
                warnings: vec!["LLM parameter mapping is disabled".to_string()],
                used_defaults: HashMap::new(),
            });
        }

        info!("Extracting parameters for tool '{}' using LLM", tool_def.name);

        // Try extraction with retries
        let mut last_error = None;
        for attempt in 1..=self.config.max_retries {
            match self.try_extract_parameters(request, tool_def).await {
                Ok(extraction) => {
                    info!("Successfully extracted parameters for '{}' on attempt {}", tool_def.name, attempt);
                    return Ok(extraction);
                }
                Err(e) => {
                    warn!("Parameter extraction attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(Duration::from_millis(1000 * attempt as u64)).await;
                    }
                }
            }
        }

        // All attempts failed
        error!("Failed to extract parameters after {} attempts", self.config.max_retries);
        Ok(ParameterExtraction {
            parameters: HashMap::new(),
            status: ExtractionStatus::Failed,
            warnings: vec![format!("LLM parameter extraction failed: {}", last_error.unwrap_or_else(|| ProxyError::routing("Unknown error".to_string())))],
            used_defaults: HashMap::new(),
        })
    }

    /// Try to extract parameters (single attempt)
    async fn try_extract_parameters(
        &self,
        request: &SmartDiscoveryRequest,
        tool_def: &ToolDefinition,
    ) -> Result<ParameterExtraction> {
        // Build the prompt
        let prompt = self.build_extraction_prompt(request, tool_def)?;
        
        info!("LLM Parameter Extraction - Tool: {}, Provider: {}, Model: {}", 
              tool_def.name, self.config.provider, self.config.model);
        debug!("LLM Request - User Request: \"{}\"", request.request);
        debug!("LLM Request - Tool Schema: {}", serde_json::to_string_pretty(&tool_def.input_schema).unwrap_or_else(|_| "Unable to serialize schema".to_string()));
        debug!("LLM Request - Full Prompt: {}", prompt);
        
        // Call LLM based on provider
        let llm_response = match self.config.provider.as_str() {
            "openai" | "openai-compatible" => {
                self.call_openai_llm(&prompt).await?
            }
            "ollama" => {
                self.call_ollama_llm(&prompt).await?
            }
            _ => {
                return Err(ProxyError::routing(format!(
                    "Unsupported LLM provider: {}", self.config.provider
                )));
            }
        };

        debug!("LLM Response - Raw: {}", llm_response);

        // Parse the response
        let extraction_result = self.parse_llm_response(&llm_response, tool_def);
        
        match &extraction_result {
            Ok(extraction) => {
                info!("LLM Parameter Extraction - Success: extracted {} parameters", extraction.parameters.len());
                debug!("LLM Parameter Extraction - Parameters: {}", serde_json::to_string_pretty(&extraction.parameters).unwrap_or_else(|_| "Unable to serialize parameters".to_string()));
                if !extraction.warnings.is_empty() {
                    debug!("LLM Parameter Extraction - Warnings: {:?}", extraction.warnings);
                }
            }
            Err(e) => {
                error!("LLM Parameter Extraction - Failed to parse response: {}", e);
            }
        }
        
        extraction_result
    }

    /// Build the extraction prompt for the LLM
    fn build_extraction_prompt(
        &self,
        request: &SmartDiscoveryRequest,
        tool_def: &ToolDefinition,
    ) -> Result<String> {
        let context_section = request.context
            .as_ref()
            .map(|c| format!("CONTEXT: {}\n", c))
            .unwrap_or_default();

        let schema_json = serde_json::to_string_pretty(&tool_def.input_schema)
            .map_err(|e| ProxyError::routing(format!("Failed to serialize tool schema: {}", e)))?;

        let prompt = format!(
            r#"Extract parameters for the '{tool_name}' tool from this user request.

USER REQUEST: "{request}"
{context_section}
TOOL SCHEMA:
{tool_schema}

INSTRUCTIONS:
1. Extract parameter values DIRECTLY from the user request and context
2. Match parameter names exactly as defined in the schema
3. Use appropriate data types (string, number, boolean, array, object)
4. For critical parameters (hosts, files, URLs, etc.): Extract from request OR set to null if truly missing
5. For optional parameters (count, timeout, etc.): Use reasonable defaults OR omit if not relevant
6. NEVER use generic defaults when user provided specific values (e.g., if user says "ping example.com", extract hosts=["example.com"], not a default)
7. For Google Sheets URLs: Extract the spreadsheet ID from URLs like "https://docs.google.com/spreadsheets/d/SPREADSHEET_ID/"
8. For sheet names with special characters: Use exact names including apostrophes, spaces, etc.
9. Return ONLY valid JSON, no explanations or additional text
10. The JSON should be a flat object with parameter names as keys

EXAMPLES:
Request: "ping google.com to check connectivity"
Schema: {{"properties": {{"hosts": {{"type": "array", "default": ["8.8.8.8"]}}, "count": {{"type": "integer", "default": 4}}}}}}
Response: {{"hosts": ["google.com"], "count": 4}}

Request: "read a file called config.yaml"
Schema: {{"properties": {{"path": {{"type": "string"}}, "encoding": {{"type": "string", "default": "utf-8"}}}}}}
Response: {{"path": "config.yaml"}}

Request: "read from Google Sheets https://docs.google.com/spreadsheets/d/1ABC123/edit"
Schema: {{"properties": {{"spreadsheet_id": {{"type": "string"}}, "range": {{"type": "string"}}}}}}
Response: {{"spreadsheet_id": "1ABC123", "range": "A:Z"}}

Request: "search for 'error' in logs directory"
Schema: {{"properties": {{"pattern": {{"type": "string"}}, "directory": {{"type": "string"}}, "limit": {{"type": "integer", "default": 100}}}}}}
Response: {{"pattern": "error", "directory": "logs"}}

JSON Response:"#,
            tool_name = tool_def.name,
            request = request.request,
            context_section = context_section,
            tool_schema = schema_json
        );

        Ok(prompt)
    }

    /// Call OpenAI-compatible LLM
    async fn call_openai_llm(&self, prompt: &str) -> Result<String> {
        let api_key = self.config.api_key.as_ref().ok_or_else(|| {
            ProxyError::routing("API key required for OpenAI LLM".to_string())
        })?;

        let base_url = self.config.base_url.as_ref()
            .map(|u| u.clone())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let request_body = OpenAIRequest {
            model: self.config.model.clone(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.1, // Low temperature for consistent parameter extraction
            max_tokens: Some(1000), // Reasonable limit for parameter extraction
        };

        let url = format!("{}/chat/completions", base_url);
        
        debug!("Calling OpenAI LLM at: {}", url);
        
        let response = tokio_timeout(
            Duration::from_secs(self.config.timeout),
            self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
        )
        .await
        .map_err(|_| ProxyError::routing("LLM request timeout".to_string()))?
        .map_err(|e| ProxyError::routing(format!("LLM request failed: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| ProxyError::routing(format!("Failed to read LLM response: {}", e)))?;

        if !status.is_success() {
            return Err(ProxyError::routing(format!(
                "LLM request failed with status {}: {}", status, response_text
            )));
        }

        let openai_response: OpenAIResponse = serde_json::from_str(&response_text)
            .map_err(|e| ProxyError::routing(format!("Failed to parse LLM response: {}", e)))?;

        let content = openai_response.choices
            .first()
            .ok_or_else(|| ProxyError::routing("No choices in LLM response".to_string()))?
            .message
            .content
            .clone();

        debug!("LLM response: {}", content);
        Ok(content)
    }

    /// Call Ollama LLM
    async fn call_ollama_llm(&self, prompt: &str) -> Result<String> {
        let base_url = self.config.base_url.as_ref()
            .map(|u| u.clone())
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        let request_body = json!({
            "model": self.config.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.1,
                "num_predict": 1000
            }
        });

        let url = format!("{}/api/generate", base_url);
        
        debug!("Calling Ollama LLM at: {}", url);
        
        let response = tokio_timeout(
            Duration::from_secs(self.config.timeout),
            self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
        )
        .await
        .map_err(|_| ProxyError::routing("LLM request timeout".to_string()))?
        .map_err(|e| ProxyError::routing(format!("LLM request failed: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| ProxyError::routing(format!("Failed to read LLM response: {}", e)))?;

        if !status.is_success() {
            return Err(ProxyError::routing(format!(
                "LLM request failed with status {}: {}", status, response_text
            )));
        }

        let ollama_response: Value = serde_json::from_str(&response_text)
            .map_err(|e| ProxyError::routing(format!("Failed to parse LLM response: {}", e)))?;

        let content = ollama_response["response"]
            .as_str()
            .ok_or_else(|| ProxyError::routing("No response in Ollama response".to_string()))?
            .to_string();

        debug!("LLM response: {}", content);
        Ok(content)
    }

    /// Parse LLM response and extract parameters
    fn parse_llm_response(
        &self,
        llm_response: &str,
        tool_def: &ToolDefinition,
    ) -> Result<ParameterExtraction> {
        // Clean up the response (remove markdown code blocks if present)
        let cleaned_response = llm_response.trim()
            .strip_prefix("```json")
            .unwrap_or(llm_response)
            .strip_suffix("```")
            .unwrap_or(llm_response)
            .trim();

        debug!("Parsing LLM response: {}", cleaned_response);

        // Parse JSON response
        let json_response: Value = serde_json::from_str(cleaned_response)
            .map_err(|e| ProxyError::routing(format!("Failed to parse LLM JSON response: {}", e)))?;

        // Convert to parameter map
        let mut parameters = HashMap::new();
        let mut warnings = Vec::new();
        let mut used_defaults = HashMap::new();

        if let Some(obj) = json_response.as_object() {
            for (key, value) in obj {
                if value.is_null() {
                    warnings.push(format!(
                        "🤔 Parameter '{}' couldn't be determined from your request. \n💡 Try being more specific about this value in your request.",
                        key
                    ));
                } else {
                    parameters.insert(key.clone(), value.clone());
                }
            }
        } else {
            return Err(ProxyError::routing("LLM response is not a JSON object".to_string()));
        }

        // Apply intelligent default values for missing parameters
        if let Some(schema_obj) = tool_def.input_schema.as_object() {
            if let Some(properties) = schema_obj.get("properties").and_then(|p| p.as_object()) {
                for (param_name, param_schema) in properties {
                    if !parameters.contains_key(param_name) {
                        if let Some(default_value) = param_schema.get("default") {
                            // Only apply defaults for truly optional parameters
                            // Critical parameters (hosts, files, paths, etc.) should not use generic defaults
                            let is_critical_param = matches!(param_name.as_str(), 
                                "host" | "hosts" | "file" | "path" | "url" | "endpoint" | "target" | "destination"
                            );
                            
                            if !is_critical_param {
                                parameters.insert(param_name.clone(), default_value.clone());
                                used_defaults.insert(param_name.clone(), default_value.clone());
                                info!("Applied default value for '{}': {}", param_name, default_value);
                            } else {
                                info!("Skipped applying default for critical parameter '{}' - LLM should have extracted this", param_name);
                            }
                        }
                    } else if parameters.get(param_name) == Some(&Value::Null) {
                        // LLM explicitly set this to null - it's missing critical info
                        let is_critical_param = matches!(param_name.as_str(), 
                            "host" | "hosts" | "file" | "path" | "url" | "endpoint" | "target" | "destination"
                        );
                        
                        if is_critical_param {
                            warnings.push(format!(
                                "🎯 Missing critical parameter '{}' - I need you to specify this value in your request",
                                param_name
                            ));
                        }
                    }
                }
            }
        }
        
        // Check for required parameters in the schema
        let status = if let Some(schema_obj) = tool_def.input_schema.as_object() {
            if let Some(required_array) = schema_obj.get("required").and_then(|r| r.as_array()) {
                let mut missing_required = Vec::new();
                
                for required_param in required_array {
                    if let Some(param_name) = required_param.as_str() {
                        if !parameters.contains_key(param_name) {
                            missing_required.push(param_name.to_string());
                        }
                    }
                }
                
                if missing_required.is_empty() {
                    ExtractionStatus::Success
                } else {
                    let missing_params = missing_required.join(", ");
                    
                    // Generate helpful parameter suggestions
                    let param_suggestions = self.generate_parameter_suggestions(tool_def, &missing_required);
                    let mut helpful_suggestions = Vec::new();
                    
                    for param_name in &missing_required {
                        if let Some(param_info) = param_suggestions.get(param_name) {
                            let examples = param_info.examples.as_ref()
                                .map(|ex| ex.join(", "))
                                .unwrap_or_else(|| "<value>".to_string());
                            let default_suggestion = format!("Include '{}' in your request", param_name);
                            let suggestion = param_info.suggestions.as_ref()
                                .and_then(|suggestions| suggestions.first())
                                .unwrap_or(&default_suggestion);
                            
                            helpful_suggestions.push(format!(
                                "  • {}: {} (examples: {})",
                                param_name, suggestion, examples
                            ));
                        }
                    }
                    
                    let detailed_help = if !helpful_suggestions.is_empty() {
                        format!("\n\n🎯 Here's how to provide the missing information:\n{}", helpful_suggestions.join("\n"))
                    } else {
                        String::new()
                    };
                    
                    warnings.push(format!(
                        "🎯 I found the right tool but need more information! \n📋 Missing required parameters: {} \n💡 Try including these values in your request explicitly.{}",
                        missing_params, detailed_help
                    ));
                    ExtractionStatus::Incomplete
                }
            } else {
                ExtractionStatus::Success
            }
        } else {
            ExtractionStatus::Success
        };

        info!("Parameter extraction completed with status: {:?}", status);
        debug!("Extracted parameters: {:?}", parameters);

        Ok(ParameterExtraction {
            parameters,
            status,
            warnings,
            used_defaults,
        })
    }

    /// Check if the mapper is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
    
    /// Generate a clarification request for missing parameters
    pub fn generate_clarification_request(
        &self,
        tool_def: &ToolDefinition,
        missing_params: &[String],
    ) -> crate::discovery::types::ClarificationRequest {
        let questions = self.generate_clarification_questions(tool_def, missing_params);
        
        let missing_info = missing_params.iter().map(|p| {
            format!("Value for parameter '{}'", p)
        }).collect();
        
        let message = if missing_params.len() == 1 {
            format!(
                "🤔 I found the '{}' tool, but I need one more piece of information to help you.",
                tool_def.name
            )
        } else {
            format!(
                "🤔 I found the '{}' tool, but I need {} more pieces of information to help you.",
                tool_def.name,
                missing_params.len()
            )
        };
        
        crate::discovery::types::ClarificationRequest {
            message,
            missing_info,
            questions,
        }
    }

    /// Get mapper configuration
    pub fn get_config(&self) -> &LlmMapperConfig {
        &self.config
    }
    
    /// Extract parameter suggestions from tool schema for missing parameters
    pub fn generate_parameter_suggestions(
        &self,
        tool_def: &ToolDefinition,
        missing_params: &[String],
    ) -> HashMap<String, crate::discovery::types::ParameterInfo> {
        let mut param_info = HashMap::new();
        
        if let Some(schema_obj) = tool_def.input_schema.as_object() {
            if let Some(properties) = schema_obj.get("properties").and_then(|p| p.as_object()) {
                let required_params: Vec<String> = schema_obj
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                
                for param_name in missing_params {
                    if let Some(param_schema) = properties.get(param_name) {
                        let param_type = param_schema.get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        let description = param_schema.get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("No description available")
                            .to_string();
                        
                        let is_required = required_params.contains(param_name);
                        
                        // Generate contextual examples and suggestions
                        let (examples, suggestions) = self.generate_contextual_help(param_name, &param_type, &description);
                        
                        param_info.insert(param_name.clone(), crate::discovery::types::ParameterInfo {
                            param_type,
                            description,
                            required: is_required,
                            examples: Some(examples),
                            suggestions: Some(suggestions),
                        });
                    }
                }
            }
        }
        
        param_info
    }
    
    /// Generate contextual help for specific parameter types
    fn generate_contextual_help(&self, param_name: &str, param_type: &str, description: &str) -> (Vec<String>, Vec<String>) {
        let name_lower = param_name.to_lowercase();
        let desc_lower = description.to_lowercase();
        
        // Generate examples based on parameter name and type
        let examples = if name_lower.contains("path") || name_lower.contains("file") {
            vec![
                "/path/to/file.txt".to_string(),
                "./config.yaml".to_string(),
                "../data/input.json".to_string(),
            ]
        } else if name_lower.contains("url") || name_lower.contains("endpoint") {
            vec![
                "https://api.example.com/v1/data".to_string(),
                "http://localhost:3000/api".to_string(),
                "https://jsonplaceholder.typicode.com/posts".to_string(),
            ]
        } else if name_lower.contains("pattern") || name_lower.contains("search") || name_lower.contains("query") {
            vec![
                "error".to_string(),
                "*.log".to_string(),
                "function main".to_string(),
            ]
        } else if name_lower.contains("port") {
            vec!["8080".to_string(), "3000".to_string(), "443".to_string()]
        } else if name_lower.contains("host") || name_lower.contains("server") {
            vec![
                "localhost".to_string(),
                "api.example.com".to_string(),
                "192.168.1.1".to_string(),
            ]
        } else if param_type == "string" {
            vec!["example_value".to_string(), "sample_text".to_string()]
        } else if param_type == "number" || param_type == "integer" {
            vec!["42".to_string(), "100".to_string(), "0".to_string()]
        } else if param_type == "boolean" {
            vec!["true".to_string(), "false".to_string()]
        } else {
            vec!["<value>".to_string()]
        };
        
        // Generate suggestions based on parameter characteristics
        let suggestions = if name_lower.contains("path") || name_lower.contains("file") {
            vec![
                "Try: 'read the config.yaml file' or 'process /path/to/data.json'".to_string(),
                "Include the full or relative file path in your request".to_string(),
                "Use quotes around file paths with spaces".to_string(),
            ]
        } else if name_lower.contains("url") || name_lower.contains("endpoint") {
            vec![
                "Try: 'make a request to https://api.example.com/data'".to_string(),
                "Include the full URL including protocol (http/https)".to_string(),
                "Specify the complete API endpoint you want to call".to_string(),
            ]
        } else if name_lower.contains("pattern") || name_lower.contains("search") {
            vec![
                "Try: 'search for \"error\" in the logs' or 'find files matching *.py'".to_string(),
                "Use specific search terms or patterns you want to find".to_string(),
                "Include quotes around multi-word search terms".to_string(),
            ]
        } else if name_lower.contains("query") {
            vec![
                "Try: 'run the query \"SELECT * FROM users\"' or 'search for \"error messages\"'".to_string(),
                "Include the actual query or search terms you want to use".to_string(),
            ]
        } else if desc_lower.contains("directory") || desc_lower.contains("folder") {
            vec![
                "Try: 'in the logs directory' or 'from the /var/log folder'".to_string(),
                "Specify which directory or folder to work with".to_string(),
            ]
        } else {
            vec![
                format!("Try including a specific value for '{}' in your request", param_name),
                "Be more explicit about this parameter in your natural language request".to_string(),
            ]
        };
        
        (examples, suggestions)
    }
    
    /// Generate interactive clarification questions for missing parameters
    pub fn generate_clarification_questions(
        &self,
        tool_def: &ToolDefinition,
        missing_params: &[String],
    ) -> Vec<crate::discovery::types::ClarificationQuestion> {
        let mut questions = Vec::new();
        
        if let Some(schema_obj) = tool_def.input_schema.as_object() {
            if let Some(properties) = schema_obj.get("properties").and_then(|p| p.as_object()) {
                let required_params: Vec<String> = schema_obj
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                
                for param_name in missing_params {
                    if let Some(param_schema) = properties.get(param_name) {
                        let param_type = param_schema.get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("string")
                            .to_string();
                        
                        let description = param_schema.get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string();
                        
                        let is_required = required_params.contains(param_name);
                        
                        let (question, input_type, choices, examples) = self.generate_question_for_parameter(
                            param_name, &param_type, &description
                        );
                        
                        questions.push(crate::discovery::types::ClarificationQuestion {
                            parameter: param_name.clone(),
                            question,
                            input_type,
                            choices,
                            examples,
                            required: is_required,
                        });
                    }
                }
            }
        }
        
        questions
    }
    
    /// Generate a user-friendly question for a specific parameter
    fn generate_question_for_parameter(
        &self,
        param_name: &str,
        param_type: &str,
        description: &str,
    ) -> (String, String, Option<Vec<String>>, Vec<String>) {
        let name_lower = param_name.to_lowercase();
        let desc_lower = description.to_lowercase();
        
        let (question, input_type, choices, examples) = if name_lower.contains("path") || name_lower.contains("file") {
            (
                format!("📁 What file path would you like to {}?", 
                    if name_lower.contains("read") { "read from" } 
                    else if name_lower.contains("write") { "write to" } 
                    else { "use" }
                ),
                "text".to_string(),
                None,
                vec![
                    "./config.yaml".to_string(),
                    "/path/to/file.txt".to_string(),
                    "../data/input.json".to_string(),
                ]
            )
        } else if name_lower.contains("url") || name_lower.contains("endpoint") {
            (
                "🌐 What URL or API endpoint would you like to use?".to_string(),
                "text".to_string(),
                None,
                vec![
                    "https://api.example.com/v1/data".to_string(),
                    "http://localhost:3000/api".to_string(),
                    "https://jsonplaceholder.typicode.com/posts".to_string(),
                ]
            )
        } else if name_lower.contains("pattern") || name_lower.contains("search") || name_lower.contains("query") {
            (
                "🔍 What would you like to search for?".to_string(),
                "text".to_string(),
                None,
                vec![
                    "error".to_string(),
                    "*.log".to_string(),
                    "function main".to_string(),
                    "TODO".to_string(),
                ]
            )
        } else if name_lower.contains("method") && desc_lower.contains("http") {
            (
                "📦 What HTTP method would you like to use?".to_string(),
                "choice".to_string(),
                Some(vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string()]),
                vec!["GET".to_string()]
            )
        } else if name_lower.contains("port") {
            (
                "🚪 What port number would you like to use?".to_string(),
                "number".to_string(),
                None,
                vec!["8080".to_string(), "3000".to_string(), "443".to_string()]
            )
        } else if name_lower.contains("host") || name_lower.contains("server") {
            (
                "🏠 What hostname or server address would you like to use?".to_string(),
                "text".to_string(),
                None,
                vec![
                    "localhost".to_string(),
                    "api.example.com".to_string(),
                    "192.168.1.1".to_string(),
                ]
            )
        } else if param_type == "boolean" {
            (
                format!("❓ Would you like to {}?", description.trim_end_matches('.')),
                "choice".to_string(),
                Some(vec!["true".to_string(), "false".to_string()]),
                vec!["true".to_string()]
            )
        } else if param_type == "number" || param_type == "integer" {
            (
                format!("🔢 What {} would you like to use?", 
                    if description.is_empty() { "number" } else { description }
                ),
                "number".to_string(),
                None,
                vec!["1".to_string(), "10".to_string(), "100".to_string()]
            )
        } else {
            // Generic string parameter
            let friendly_name = param_name.replace('_', " ");
            (
                format!("✏️ What {} would you like to specify?", 
                    if description.is_empty() { &friendly_name } else { description }
                ),
                "text".to_string(),
                None,
                vec!["example_value".to_string()]
            )
        };
        
        (question, input_type, choices, examples)
    }
}