//! Comprehensive tests for MCP-compliant error handling

use magictunnel::error::ProxyError;
use magictunnel::mcp::errors::{McpError, McpErrorCode};
use serde_json::{json, Value};

#[cfg(test)]
mod mcp_error_tests {
    use super::*;

    #[test]
    fn test_mcp_error_code_values() {
        // Test standard JSON-RPC error codes
        assert_eq!(McpErrorCode::ParseError.code(), -32700);
        assert_eq!(McpErrorCode::InvalidRequest.code(), -32600);
        assert_eq!(McpErrorCode::MethodNotFound.code(), -32601);
        assert_eq!(McpErrorCode::InvalidParams.code(), -32602);
        assert_eq!(McpErrorCode::InternalError.code(), -32603);
        
        // Test MCP-specific error codes (above -32000)
        assert_eq!(McpErrorCode::ToolNotFound.code(), -32000);
        assert_eq!(McpErrorCode::ToolExecutionFailed.code(), -31999);
        assert_eq!(McpErrorCode::ResourceNotFound.code(), -31998);
        assert_eq!(McpErrorCode::ResourceAccessDenied.code(), -31997);
        assert_eq!(McpErrorCode::PromptNotFound.code(), -31996);
        assert_eq!(McpErrorCode::PromptExecutionFailed.code(), -31995);
        assert_eq!(McpErrorCode::AuthenticationFailed.code(), -31994);
        assert_eq!(McpErrorCode::AuthorizationFailed.code(), -31993);
        assert_eq!(McpErrorCode::ConfigurationError.code(), -31992);
        assert_eq!(McpErrorCode::ValidationError.code(), -31991);
        assert_eq!(McpErrorCode::RateLimitExceeded.code(), -31990);
        assert_eq!(McpErrorCode::ServiceUnavailable.code(), -31989);
        assert_eq!(McpErrorCode::TimeoutError.code(), -31988);
        assert_eq!(McpErrorCode::NetworkError.code(), -31987);
        assert_eq!(McpErrorCode::SerializationError.code(), -31986);
    }

    #[test]
    fn test_mcp_error_default_messages() {
        assert_eq!(McpErrorCode::ParseError.default_message(), "Parse error");
        assert_eq!(McpErrorCode::MethodNotFound.default_message(), "Method not found");
        assert_eq!(McpErrorCode::ToolNotFound.default_message(), "Tool not found");
        assert_eq!(McpErrorCode::ValidationError.default_message(), "Validation error");
    }

    #[test]
    fn test_mcp_error_creation() {
        let error = McpError::new(McpErrorCode::ToolNotFound, "Test message".to_string());
        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "Test message");
        assert!(error.data.is_none());
    }

    #[test]
    fn test_mcp_error_with_data() {
        let data = json!({"tool_name": "test_tool"});
        let error = McpError::with_data(
            McpErrorCode::ToolNotFound,
            "Tool not found".to_string(),
            data.clone()
        );
        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "Tool not found");
        assert_eq!(error.data, Some(data));
    }

    #[test]
    fn test_tool_not_found_error() {
        let error = McpError::tool_not_found("test_tool".to_string());
        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "Tool 'test_tool' not found");
        assert_eq!(error.data, Some(json!({"tool_name": "test_tool"})));
    }

    #[test]
    fn test_tool_execution_failed_error() {
        let error = McpError::tool_execution_failed(
            "test_tool".to_string(),
            "execution failed".to_string()
        );
        assert_eq!(error.code, -31999);
        assert!(error.message.contains("test_tool"));
        assert!(error.message.contains("execution failed"));
        
        let data = error.data.unwrap();
        assert_eq!(data["tool_name"], "test_tool");
        assert_eq!(data["execution_error"], "execution failed");
    }

    #[test]
    fn test_resource_not_found_error() {
        let error = McpError::resource_not_found("file://test.txt".to_string());
        assert_eq!(error.code, -31998);
        assert_eq!(error.message, "Resource 'file://test.txt' not found");
        assert_eq!(error.data, Some(json!({"uri": "file://test.txt"})));
    }

    #[test]
    fn test_prompt_not_found_error() {
        let error = McpError::prompt_not_found("test_prompt".to_string());
        assert_eq!(error.code, -31996);
        assert_eq!(error.message, "Prompt 'test_prompt' not found");
        assert_eq!(error.data, Some(json!({"prompt_name": "test_prompt"})));
    }

    #[test]
    fn test_validation_error() {
        let details = json!({"field": "name", "issue": "required"});
        let error = McpError::validation_error(
            "Validation failed".to_string(),
            Some(details.clone())
        );
        assert_eq!(error.code, -31991);
        assert_eq!(error.message, "Validation failed");
        assert_eq!(error.data, Some(details));
    }

    #[test]
    fn test_rate_limit_exceeded_error() {
        let error = McpError::rate_limit_exceeded(100, "minute".to_string());
        assert_eq!(error.code, -31990);
        assert!(error.message.contains("100"));
        assert!(error.message.contains("minute"));
        
        let data = error.data.unwrap();
        assert_eq!(data["limit"], 100);
        assert_eq!(data["window"], "minute");
    }

    #[test]
    fn test_timeout_error() {
        let error = McpError::timeout_error("tool_execution".to_string(), 5000);
        assert_eq!(error.code, -31988);
        assert!(error.message.contains("tool_execution"));
        assert!(error.message.contains("5000ms"));
        
        let data = error.data.unwrap();
        assert_eq!(data["operation"], "tool_execution");
        assert_eq!(data["timeout_ms"], 5000);
    }

    #[test]
    fn test_method_not_found_error() {
        let error = McpError::method_not_found("unknown_method".to_string());
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method 'unknown_method' not found");
        assert_eq!(error.data, Some(json!({"method": "unknown_method"})));
    }

    #[test]
    fn test_error_serialization() {
        let error = McpError::tool_not_found("test_tool".to_string());
        let serialized = serde_json::to_string(&error).unwrap();
        let deserialized: McpError = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(error.code, deserialized.code);
        assert_eq!(error.message, deserialized.message);
        assert_eq!(error.data, deserialized.data);
    }

    #[test]
    fn test_json_rpc_error_format() {
        let error = McpError::tool_not_found("test_tool".to_string());
        let serialized = serde_json::to_string(&error).unwrap();
        let parsed: Value = serde_json::from_str(&serialized).unwrap();
        
        assert!(parsed["code"].is_number());
        assert!(parsed["message"].is_string());
        assert!(parsed["data"].is_object());
        
        assert_eq!(parsed["code"], -32000);
        assert_eq!(parsed["message"], "Tool 'test_tool' not found");
        assert_eq!(parsed["data"]["tool_name"], "test_tool");
    }
}

#[cfg(test)]
mod proxy_error_conversion_tests {
    use super::*;

    #[test]
    fn test_config_error_conversion() {
        let proxy_error = ProxyError::config("Invalid configuration");
        let mcp_error: McpError = proxy_error.into();
        
        assert_eq!(mcp_error.code, -31992);
        assert!(mcp_error.message.contains("Invalid configuration"));
        assert_eq!(mcp_error.data.unwrap()["category"], "config");
    }

    #[test]
    fn test_tool_execution_error_conversion() {
        let proxy_error = ProxyError::tool_execution("test_tool", "execution failed");
        let mcp_error: McpError = proxy_error.into();
        
        assert_eq!(mcp_error.code, -31999);
        assert!(mcp_error.message.contains("test_tool"));
        assert!(mcp_error.message.contains("execution failed"));
    }

    #[test]
    fn test_auth_error_conversion() {
        let proxy_error = ProxyError::auth("Authentication failed");
        let mcp_error: McpError = proxy_error.into();
        
        assert_eq!(mcp_error.code, -31994);
        assert!(mcp_error.message.contains("Authentication failed"));
        assert_eq!(mcp_error.data.unwrap()["category"], "auth");
    }

    #[test]
    fn test_validation_error_conversion() {
        let proxy_error = ProxyError::validation("Validation failed");
        let mcp_error: McpError = proxy_error.into();
        
        assert_eq!(mcp_error.code, -31991);
        assert!(mcp_error.message.contains("Validation failed"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let proxy_error = ProxyError::Io(io_error);
        let mcp_error: McpError = proxy_error.into();
        
        assert_eq!(mcp_error.code, -32603); // Internal error
        assert!(mcp_error.message.contains("IO error"));
        assert_eq!(mcp_error.data.unwrap()["category"], "io");
    }

    // Note: HTTP error test removed due to complexity of creating mock reqwest::Error
    // The conversion logic is tested through integration tests

    #[test]
    fn test_serialization_error_conversion() {
        // Create a real serde error by trying to parse invalid JSON
        let invalid_json = "{ invalid json";
        let serde_error = serde_json::from_str::<serde_json::Value>(invalid_json).unwrap_err();
        let proxy_error = ProxyError::Serde(serde_error);
        let mcp_error: McpError = proxy_error.into();

        assert_eq!(mcp_error.code, -31986); // Serialization error
        assert!(mcp_error.message.contains("Serialization error"));
        assert_eq!(mcp_error.data.unwrap()["category"], "serialization");
    }

    #[test]
    fn test_registry_error_conversion() {
        let proxy_error = ProxyError::registry("Registry lookup failed");
        let mcp_error: McpError = proxy_error.into();
        
        assert_eq!(mcp_error.code, -32603); // Internal error
        assert!(mcp_error.message.contains("Registry error"));
        assert_eq!(mcp_error.data.unwrap()["category"], "registry");
    }

    #[test]
    fn test_routing_error_conversion() {
        let proxy_error = ProxyError::routing("No route found");
        let mcp_error: McpError = proxy_error.into();
        
        assert_eq!(mcp_error.code, -32603); // Internal error
        assert!(mcp_error.message.contains("Routing error"));
        assert_eq!(mcp_error.data.unwrap()["category"], "routing");
    }

    #[test]
    fn test_mcp_protocol_error_conversion() {
        let proxy_error = ProxyError::mcp("Invalid MCP message");
        let mcp_error: McpError = proxy_error.into();
        
        assert_eq!(mcp_error.code, -32603); // Internal error
        assert!(mcp_error.message.contains("MCP protocol error"));
    }
}
