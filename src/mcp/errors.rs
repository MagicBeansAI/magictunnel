//! MCP-compliant error handling
//! 
//! This module provides MCP specification-compliant error types and codes
//! for proper JSON-RPC 2.0 error responses.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::ProxyError;

/// MCP-compliant error codes following JSON-RPC 2.0 specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpErrorCode {
    // Standard JSON-RPC error codes
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    
    // MCP-specific error codes (above -32000 as per spec)
    ToolNotFound = -32000,
    ToolExecutionFailed = -31999,
    ResourceNotFound = -31998,
    ResourceAccessDenied = -31997,
    PromptNotFound = -31996,
    PromptExecutionFailed = -31995,
    AuthenticationFailed = -31994,
    AuthorizationFailed = -31993,
    ConfigurationError = -31992,
    ValidationError = -31991,
    RateLimitExceeded = -31990,
    ServiceUnavailable = -31989,
    TimeoutError = -31988,
    NetworkError = -31987,
    SerializationError = -31986,
}

impl McpErrorCode {
    /// Get the error code as i32
    pub fn code(&self) -> i32 {
        *self as i32
    }
    
    /// Get a default message for this error code
    pub fn default_message(&self) -> &'static str {
        match self {
            McpErrorCode::ParseError => "Parse error",
            McpErrorCode::InvalidRequest => "Invalid request",
            McpErrorCode::MethodNotFound => "Method not found",
            McpErrorCode::InvalidParams => "Invalid params",
            McpErrorCode::InternalError => "Internal error",
            McpErrorCode::ToolNotFound => "Tool not found",
            McpErrorCode::ToolExecutionFailed => "Tool execution failed",
            McpErrorCode::ResourceNotFound => "Resource not found",
            McpErrorCode::ResourceAccessDenied => "Resource access denied",
            McpErrorCode::PromptNotFound => "Prompt not found",
            McpErrorCode::PromptExecutionFailed => "Prompt execution failed",
            McpErrorCode::AuthenticationFailed => "Authentication failed",
            McpErrorCode::AuthorizationFailed => "Authorization failed",
            McpErrorCode::ConfigurationError => "Configuration error",
            McpErrorCode::ValidationError => "Validation error",
            McpErrorCode::RateLimitExceeded => "Rate limit exceeded",
            McpErrorCode::ServiceUnavailable => "Service unavailable",
            McpErrorCode::TimeoutError => "Timeout error",
            McpErrorCode::NetworkError => "Network error",
            McpErrorCode::SerializationError => "Serialization error",
        }
    }
}

/// MCP-compliant error structure following JSON-RPC 2.0 specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data (optional)
    pub data: Option<Value>,
}

impl McpError {
    /// Create a new MCP error
    pub fn new(code: McpErrorCode, message: String) -> Self {
        Self {
            code: code.code(),
            message,
            data: None,
        }
    }
    
    /// Create a new MCP error with additional data
    pub fn with_data(code: McpErrorCode, message: String, data: Value) -> Self {
        Self {
            code: code.code(),
            message,
            data: Some(data),
        }
    }
    
    /// Create a parse error
    pub fn parse_error(message: String) -> Self {
        Self::new(McpErrorCode::ParseError, message)
    }
    
    /// Create an invalid request error
    pub fn invalid_request(message: String) -> Self {
        Self::new(McpErrorCode::InvalidRequest, message)
    }
    
    /// Create a method not found error
    pub fn method_not_found(method: String) -> Self {
        Self::with_data(
            McpErrorCode::MethodNotFound,
            format!("Method '{}' not found", method),
            serde_json::json!({ "method": method })
        )
    }
    
    /// Create an invalid params error
    pub fn invalid_params(message: String) -> Self {
        Self::new(McpErrorCode::InvalidParams, message)
    }
    
    /// Create an internal error
    pub fn internal_error(message: String) -> Self {
        Self::new(McpErrorCode::InternalError, message)
    }
    
    /// Create a tool not found error
    pub fn tool_not_found(tool_name: String) -> Self {
        Self::with_data(
            McpErrorCode::ToolNotFound,
            format!("Tool '{}' not found", tool_name),
            serde_json::json!({ "tool_name": tool_name })
        )
    }
    
    /// Create a tool execution failed error
    pub fn tool_execution_failed(tool_name: String, error: String) -> Self {
        Self::with_data(
            McpErrorCode::ToolExecutionFailed,
            format!("Tool '{}' execution failed: {}", tool_name, error),
            serde_json::json!({ 
                "tool_name": tool_name,
                "execution_error": error
            })
        )
    }
    
    /// Create a resource not found error
    pub fn resource_not_found(uri: String) -> Self {
        Self::with_data(
            McpErrorCode::ResourceNotFound,
            format!("Resource '{}' not found", uri),
            serde_json::json!({ "uri": uri })
        )
    }
    
    /// Create a prompt not found error
    pub fn prompt_not_found(name: String) -> Self {
        Self::with_data(
            McpErrorCode::PromptNotFound,
            format!("Prompt '{}' not found", name),
            serde_json::json!({ "prompt_name": name })
        )
    }
    
    /// Create a validation error
    pub fn validation_error(message: String, details: Option<Value>) -> Self {
        let mut error = Self::new(McpErrorCode::ValidationError, message);
        if let Some(data) = details {
            error.data = Some(data);
        }
        error
    }
    
    /// Create a rate limit exceeded error
    pub fn rate_limit_exceeded(limit: u32, window: String) -> Self {
        Self::with_data(
            McpErrorCode::RateLimitExceeded,
            format!("Rate limit of {} requests per {} exceeded", limit, window),
            serde_json::json!({ 
                "limit": limit,
                "window": window
            })
        )
    }
    
    /// Create a timeout error
    pub fn timeout_error(operation: String, timeout_ms: u64) -> Self {
        Self::with_data(
            McpErrorCode::TimeoutError,
            format!("Operation '{}' timed out after {}ms", operation, timeout_ms),
            serde_json::json!({ 
                "operation": operation,
                "timeout_ms": timeout_ms
            })
        )
    }
}

/// Convert ProxyError to MCP-compliant error
impl From<ProxyError> for McpError {
    fn from(error: ProxyError) -> Self {
        match error {
            ProxyError::Config { message } => {
                McpError::with_data(
                    McpErrorCode::ConfigurationError,
                    message,
                    serde_json::json!({ "category": "config" })
                )
            }
            ProxyError::Registry { message } => {
                McpError::with_data(
                    McpErrorCode::InternalError,
                    format!("Registry error: {}", message),
                    serde_json::json!({ "category": "registry" })
                )
            }
            ProxyError::Mcp { message } => {
                McpError::internal_error(format!("MCP protocol error: {}", message))
            }
            ProxyError::Routing { message } => {
                McpError::with_data(
                    McpErrorCode::InternalError,
                    format!("Routing error: {}", message),
                    serde_json::json!({ "category": "routing" })
                )
            }
            ProxyError::ToolExecution { tool_name, message } => {
                McpError::tool_execution_failed(tool_name, message)
            }
            ProxyError::Auth { message } => {
                McpError::with_data(
                    McpErrorCode::AuthenticationFailed,
                    message,
                    serde_json::json!({ "category": "auth" })
                )
            }
            ProxyError::Validation { message } => {
                McpError::validation_error(message, None)
            }
            ProxyError::Io(e) => {
                McpError::with_data(
                    McpErrorCode::InternalError,
                    format!("IO error: {}", e),
                    serde_json::json!({ "category": "io" })
                )
            }
            ProxyError::Serde(e) => {
                McpError::with_data(
                    McpErrorCode::SerializationError,
                    format!("Serialization error: {}", e),
                    serde_json::json!({ "category": "serialization" })
                )
            }
            ProxyError::Yaml(e) => {
                McpError::with_data(
                    McpErrorCode::SerializationError,
                    format!("YAML parsing error: {}", e),
                    serde_json::json!({ "category": "yaml" })
                )
            }
            ProxyError::Http(e) => {
                McpError::with_data(
                    McpErrorCode::NetworkError,
                    format!("HTTP error: {}", e),
                    serde_json::json!({ "category": "http" })
                )
            }
            ProxyError::JsonSchema(e) => {
                McpError::with_data(
                    McpErrorCode::ValidationError,
                    format!("JSON Schema validation error: {}", e),
                    serde_json::json!({ "category": "json_schema" })
                )
            }
            ProxyError::Internal(e) => {
                McpError::internal_error(format!("Internal error: {}", e))
            }
            ProxyError::Connection { message } => {
                McpError::with_data(
                    McpErrorCode::InternalError,
                    format!("Connection error: {}", message),
                    serde_json::json!({ "category": "connection" })
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_error_code_values() {
        assert_eq!(McpErrorCode::ParseError.code(), -32700);
        assert_eq!(McpErrorCode::InvalidRequest.code(), -32600);
        assert_eq!(McpErrorCode::MethodNotFound.code(), -32601);
        assert_eq!(McpErrorCode::InvalidParams.code(), -32602);
        assert_eq!(McpErrorCode::InternalError.code(), -32603);
        assert_eq!(McpErrorCode::ToolNotFound.code(), -32000);
    }

    #[test]
    fn test_mcp_error_creation() {
        let error = McpError::tool_not_found("test_tool".to_string());
        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "Tool 'test_tool' not found");
        assert_eq!(error.data, Some(json!({"tool_name": "test_tool"})));
    }

    #[test]
    fn test_proxy_error_conversion() {
        let proxy_error = ProxyError::tool_execution("test_tool".to_string(), "execution failed".to_string());
        let mcp_error: McpError = proxy_error.into();
        
        assert_eq!(mcp_error.code, -31999);
        assert!(mcp_error.message.contains("test_tool"));
        assert!(mcp_error.message.contains("execution failed"));
    }

    #[test]
    fn test_error_serialization() {
        let error = McpError::method_not_found("unknown_method".to_string());
        let serialized = serde_json::to_string(&error).unwrap();
        let deserialized: McpError = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(error.code, deserialized.code);
        assert_eq!(error.message, deserialized.message);
        assert_eq!(error.data, deserialized.data);
    }
}
