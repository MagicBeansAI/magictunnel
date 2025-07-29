//! Enhanced MCP Message Validation
//! 
//! Provides comprehensive JSON-RPC 2.0 and MCP message format validation
//! beyond basic parsing, including protocol compliance checks.

use crate::error::{Result, ProxyError};
use crate::mcp::types::McpRequest;
use serde_json::Value;
use std::collections::HashSet;
use tracing::{debug, warn};

/// JSON-RPC 2.0 version string
pub const JSONRPC_VERSION: &str = "2.0";

/// Maximum allowed message size (1MB)
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Maximum allowed method name length
pub const MAX_METHOD_NAME_LENGTH: usize = 256;

/// Maximum allowed request ID length (for string IDs)
pub const MAX_REQUEST_ID_LENGTH: usize = 256;

/// Valid MCP method names
pub const VALID_MCP_METHODS: &[&str] = &[
    // Core MCP methods
    "initialize",
    "initialized",
    
    // Tool methods
    "tools/list",
    "tools/call",
    
    // Resource methods
    "resources/list",
    "resources/read",
    "resources/subscribe",
    "resources/unsubscribe",
    
    // Prompt methods
    "prompts/list",
    "prompts/get",
    
    // Logging methods
    "logging/setLevel",
    
    // Completion methods
    "completion/complete",
    
    // Notification methods
    "notifications/initialized",
    "notifications/cancelled",
    "notifications/progress",
    "notifications/resources/list_changed",
    "notifications/resources/updated",
    "notifications/prompts/list_changed",
    "notifications/tools/list_changed",
    "notifications/message",
];

/// Enhanced message validator for MCP protocol
#[derive(Debug)]
pub struct McpMessageValidator {
    /// Set of valid method names for quick lookup
    valid_methods: HashSet<String>,
    /// Configuration
    config: ValidationConfig,
}

/// Configuration for message validation
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Enable strict method name validation
    pub strict_method_validation: bool,
    /// Enable strict JSON-RPC 2.0 compliance
    pub strict_jsonrpc_compliance: bool,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Enable parameter validation
    pub validate_parameters: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict_method_validation: true,
            strict_jsonrpc_compliance: true,
            max_message_size: MAX_MESSAGE_SIZE,
            validate_parameters: true,
        }
    }
}

impl McpMessageValidator {
    /// Create a new message validator
    pub fn new() -> Self {
        Self::with_config(ValidationConfig::default())
    }

    /// Create a new message validator with custom configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        let valid_methods = VALID_MCP_METHODS.iter()
            .map(|&s| s.to_string())
            .collect();

        Self {
            valid_methods,
            config,
        }
    }

    /// Validate raw message size and format
    pub fn validate_raw_message(&self, message: &str) -> Result<()> {
        // Check message size
        if message.len() > self.config.max_message_size {
            return Err(ProxyError::mcp(format!(
                "Message size {} exceeds maximum allowed size {}",
                message.len(), self.config.max_message_size
            )));
        }

        // Check if message is valid JSON
        serde_json::from_str::<Value>(message)
            .map_err(|e| ProxyError::mcp(format!("Invalid JSON format: {}", e)))?;

        Ok(())
    }

    /// Validate MCP request message
    pub fn validate_request(&self, request: &McpRequest) -> Result<()> {
        debug!("Validating MCP request: method={}", request.method);

        // Validate method name
        self.validate_method_name(&request.method)?;

        // Validate request ID format
        if let Some(ref id) = request.id {
            self.validate_request_id(id)?;
        }

        // Validate parameters if present
        if self.config.validate_parameters {
            if let Some(ref params) = request.params {
                self.validate_parameters(&request.method, params)?;
            }
        }

        // Method-specific validation
        self.validate_method_specific(request)?;

        debug!("MCP request validation passed for method: {}", request.method);
        Ok(())
    }

    /// Validate method name
    fn validate_method_name(&self, method: &str) -> Result<()> {
        // Check method name length
        if method.len() > MAX_METHOD_NAME_LENGTH {
            return Err(ProxyError::mcp(format!(
                "Method name '{}' exceeds maximum length {}",
                method, MAX_METHOD_NAME_LENGTH
            )));
        }

        // Check for empty method name
        if method.is_empty() {
            return Err(ProxyError::mcp("Method name cannot be empty".to_string()));
        }

        // Strict method validation
        if self.config.strict_method_validation {
            if !self.valid_methods.contains(method) {
                return Err(ProxyError::mcp(format!(
                    "Unknown method '{}'. Valid methods: {:?}",
                    method, VALID_MCP_METHODS
                )));
            }
        }

        Ok(())
    }

    /// Validate request ID format
    fn validate_request_id(&self, id: &Value) -> Result<()> {
        match id {
            Value::String(s) => {
                if s.len() > MAX_REQUEST_ID_LENGTH {
                    return Err(ProxyError::mcp(format!(
                        "Request ID string length {} exceeds maximum {}",
                        s.len(), MAX_REQUEST_ID_LENGTH
                    )));
                }
                if s.is_empty() {
                    return Err(ProxyError::mcp("Request ID string cannot be empty".to_string()));
                }
            }
            Value::Number(n) => {
                // Validate number is within reasonable range
                if let Some(i) = n.as_i64() {
                    if i < 0 {
                        return Err(ProxyError::mcp("Request ID number cannot be negative".to_string()));
                    }
                } else if let Some(f) = n.as_f64() {
                    if !f.is_finite() {
                        return Err(ProxyError::mcp("Request ID number must be finite".to_string()));
                    }
                }
            }
            Value::Null => {
                // Null ID is valid for notifications
            }
            _ => {
                return Err(ProxyError::mcp(
                    "Request ID must be string, number, or null".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Validate method parameters
    fn validate_parameters(&self, method: &str, params: &Value) -> Result<()> {
        match method {
            "initialize" => self.validate_initialize_params(params),
            "tools/call" => self.validate_tool_call_params(params),
            "resources/read" => self.validate_resource_read_params(params),
            "prompts/get" => self.validate_prompt_get_params(params),
            "logging/setLevel" => self.validate_logging_params(params),
            "completion/complete" => self.validate_completion_params(params),
            _ => {
                // For other methods, just validate that params is an object if present
                if !params.is_object() && !params.is_null() {
                    return Err(ProxyError::mcp(format!(
                        "Parameters for method '{}' must be an object or null", method
                    )));
                }
                Ok(())
            }
        }
    }

    /// Validate initialize method parameters
    fn validate_initialize_params(&self, params: &Value) -> Result<()> {
        let obj = params.as_object()
            .ok_or_else(|| ProxyError::mcp("Initialize parameters must be an object".to_string()))?;

        // Validate required clientInfo
        let client_info = obj.get("clientInfo")
            .ok_or_else(|| ProxyError::mcp("Initialize parameters missing 'clientInfo'".to_string()))?;

        let client_obj = client_info.as_object()
            .ok_or_else(|| ProxyError::mcp("clientInfo must be an object".to_string()))?;

        // Validate required name field
        client_obj.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::mcp("clientInfo missing required 'name' field".to_string()))?;

        // Validate required version field
        client_obj.get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::mcp("clientInfo missing required 'version' field".to_string()))?;

        // Validate optional protocolVersion
        if let Some(protocol_version) = obj.get("protocolVersion") {
            protocol_version.as_str()
                .ok_or_else(|| ProxyError::mcp("protocolVersion must be a string".to_string()))?;
        }

        Ok(())
    }

    /// Validate tool call parameters
    fn validate_tool_call_params(&self, params: &Value) -> Result<()> {
        let obj = params.as_object()
            .ok_or_else(|| ProxyError::mcp("Tool call parameters must be an object".to_string()))?;

        // Validate required name field
        obj.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::mcp("Tool call missing required 'name' field".to_string()))?;

        // Validate optional arguments field
        if let Some(arguments) = obj.get("arguments") {
            if !arguments.is_object() {
                return Err(ProxyError::mcp("Tool call 'arguments' must be an object".to_string()));
            }
        }

        Ok(())
    }

    /// Validate resource read parameters
    fn validate_resource_read_params(&self, params: &Value) -> Result<()> {
        let obj = params.as_object()
            .ok_or_else(|| ProxyError::mcp("Resource read parameters must be an object".to_string()))?;

        // Validate required uri field
        obj.get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::mcp("Resource read missing required 'uri' field".to_string()))?;

        Ok(())
    }

    /// Validate prompt get parameters
    fn validate_prompt_get_params(&self, params: &Value) -> Result<()> {
        let obj = params.as_object()
            .ok_or_else(|| ProxyError::mcp("Prompt get parameters must be an object".to_string()))?;

        // Validate required name field
        obj.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::mcp("Prompt get missing required 'name' field".to_string()))?;

        Ok(())
    }

    /// Validate logging parameters
    fn validate_logging_params(&self, params: &Value) -> Result<()> {
        let obj = params.as_object()
            .ok_or_else(|| ProxyError::mcp("Logging parameters must be an object".to_string()))?;

        // Validate required level field
        let level = obj.get("level")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::mcp("Logging missing required 'level' field".to_string()))?;

        // Validate level value
        match level {
            "debug" | "info" | "notice" | "warning" | "error" | "critical" | "alert" | "emergency" => Ok(()),
            _ => Err(ProxyError::mcp(format!("Invalid log level '{}'", level)))
        }
    }

    /// Validate completion parameters
    fn validate_completion_params(&self, params: &Value) -> Result<()> {
        let obj = params.as_object()
            .ok_or_else(|| ProxyError::mcp("Completion parameters must be an object".to_string()))?;

        // Validate required ref field
        let ref_obj = obj.get("ref")
            .ok_or_else(|| ProxyError::mcp("Completion missing required 'ref' field".to_string()))?;

        let ref_obj = ref_obj.as_object()
            .ok_or_else(|| ProxyError::mcp("Completion 'ref' must be an object".to_string()))?;

        // Validate ref type
        let ref_type = ref_obj.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::mcp("Completion ref missing required 'type' field".to_string()))?;

        match ref_type {
            "ref/resource" | "ref/prompt" => Ok(()),
            _ => Err(ProxyError::mcp(format!("Invalid completion ref type '{}'", ref_type)))
        }
    }

    /// Method-specific validation
    fn validate_method_specific(&self, request: &McpRequest) -> Result<()> {
        match request.method.as_str() {
            "initialize" => {
                // Initialize must have an ID
                if request.id.is_none() {
                    return Err(ProxyError::mcp("Initialize request must have an ID".to_string()));
                }
            }
            "initialized" | "notifications/initialized" => {
                // Notifications should not have an ID
                if request.id.is_some() {
                    warn!("Notification method '{}' should not have an ID", request.method);
                }
            }
            method if method.starts_with("notifications/") => {
                // Other notifications should not have an ID
                if request.id.is_some() {
                    warn!("Notification method '{}' should not have an ID", request.method);
                }
            }
            _ => {
                // Regular methods should have an ID
                if request.id.is_none() {
                    return Err(ProxyError::mcp(format!(
                        "Method '{}' must have an ID", request.method
                    )));
                }
            }
        }

        Ok(())
    }
}

impl Default for McpMessageValidator {
    fn default() -> Self {
        Self::new()
    }
}
