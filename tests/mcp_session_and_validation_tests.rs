//! MCP Session Management and Message Validation Tests
//!
//! Comprehensive tests for MCP session lifecycle management, message validation,
//! and protocol version negotiation functionality.

use magictunnel::mcp::session::{McpSessionManager, SessionConfig};
use magictunnel::mcp::validation::{McpMessageValidator, ValidationConfig};
use magictunnel::mcp::types::McpRequest;
use serde_json::{json, Value};

#[cfg(test)]
mod session_management_tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let manager = McpSessionManager::new();
        
        // Test successful session creation
        let session_id = manager.create_session().expect("Should create session");
        assert!(!session_id.is_empty());
        
        // Test session exists
        let session = manager.get_session(&session_id);
        assert!(session.is_some());
        
        let session = session.unwrap();
        assert_eq!(session.id, session_id);
        assert!(!session.initialized);
        assert!(session.client_info.is_none());
    }

    #[test]
    fn test_session_limit() {
        let config = SessionConfig {
            max_sessions: 2,
            ..Default::default()
        };
        let manager = McpSessionManager::with_config(config);
        
        // Create maximum sessions
        let _session1 = manager.create_session().expect("Should create first session");
        let _session2 = manager.create_session().expect("Should create second session");
        
        // Third session should fail
        let result = manager.create_session();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum number of sessions"));
    }

    #[test]
    fn test_request_id_uniqueness() {
        let manager = McpSessionManager::new();
        let session_id = manager.create_session().expect("Should create session");
        
        // First use of request ID should succeed
        let result = manager.validate_request_id(&session_id, "test-id-1");
        assert!(result.is_ok());
        
        // Second use of same request ID should fail
        let result = manager.validate_request_id(&session_id, "test-id-1");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate request ID"));
        
        // Different request ID should succeed
        let result = manager.validate_request_id(&session_id, "test-id-2");
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_cleanup() {
        let manager = McpSessionManager::new();
        let session_id = manager.create_session().expect("Should create session");
        
        // Session should exist
        assert!(manager.get_session(&session_id).is_some());
        
        // Remove session
        let result = manager.remove_session(&session_id);
        assert!(result.is_ok());
        
        // Session should no longer exist
        assert!(manager.get_session(&session_id).is_none());
        
        // Removing non-existent session should fail
        let result = manager.remove_session(&session_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_protocol_version_negotiation() {
        let manager = McpSessionManager::new();
        let session_id = manager.create_session().expect("Should create session");
        
        // Test initialize with supported version
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("init-1".to_string())),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            })),
        };
        
        let result = manager.handle_initialize(&session_id, &request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2024-11-05");
        
        // Check session was updated
        let session = manager.get_session(&session_id).unwrap();
        assert!(session.initialized);
        assert_eq!(session.protocol_version, "2024-11-05");
        assert!(session.client_info.is_some());
        
        let client_info = session.client_info.unwrap();
        assert_eq!(client_info.name, "test-client");
        assert_eq!(client_info.version, "1.0.0");
    }

    #[test]
    fn test_unsupported_protocol_version() {
        let config = SessionConfig {
            strict_version_validation: true,
            ..Default::default()
        };
        let manager = McpSessionManager::with_config(config);
        let session_id = manager.create_session().expect("Should create session");
        
        // Test initialize with unsupported version
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("init-1".to_string())),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "1.0.0",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            })),
        };
        
        let result = manager.handle_initialize(&session_id, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported protocol version"));
    }

    #[test]
    fn test_session_stats() {
        let manager = McpSessionManager::new();
        
        // Initial stats
        let stats = manager.get_stats();
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.initialized_sessions, 0);
        
        // Create sessions
        let session1 = manager.create_session().expect("Should create session");
        let session2 = manager.create_session().expect("Should create session");
        
        // Stats after creation
        let stats = manager.get_stats();
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.initialized_sessions, 0);
        
        // Initialize one session
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("init-1".to_string())),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            })),
        };
        
        let _ = manager.handle_initialize(&session1, &request);
        
        // Stats after initialization
        let stats = manager.get_stats();
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.initialized_sessions, 1);
    }
}

#[cfg(test)]
mod message_validation_tests {
    use super::*;

    #[test]
    fn test_raw_message_validation() {
        let validator = McpMessageValidator::new();
        
        // Valid JSON should pass
        let result = validator.validate_raw_message(r#"{"method": "test"}"#);
        assert!(result.is_ok());
        
        // Invalid JSON should fail
        let result = validator.validate_raw_message(r#"{"method": "test""#);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid JSON"));
        
        // Empty message should fail
        let result = validator.validate_raw_message("");
        assert!(result.is_err());
    }

    #[test]
    fn test_message_size_validation() {
        let config = ValidationConfig {
            max_message_size: 100,
            ..Default::default()
        };
        let validator = McpMessageValidator::with_config(config);
        
        // Small message should pass
        let small_message = r#"{"method": "test"}"#;
        let result = validator.validate_raw_message(small_message);
        assert!(result.is_ok());
        
        // Large message should fail
        let large_message = "x".repeat(200);
        let result = validator.validate_raw_message(&large_message);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum allowed size"));
    }

    #[test]
    fn test_method_name_validation() {
        let validator = McpMessageValidator::new();
        
        // Valid method should pass
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("test-1".to_string())),
            method: "tools/list".to_string(),
            params: None,
        };
        let result = validator.validate_request(&request);
        assert!(result.is_ok());
        
        // Invalid method should fail
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("test-2".to_string())),
            method: "invalid/method".to_string(),
            params: None,
        };
        let result = validator.validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown method"));
        
        // Empty method should fail
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("test-3".to_string())),
            method: "".to_string(),
            params: None,
        };
        let result = validator.validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_request_id_validation() {
        let validator = McpMessageValidator::new();
        
        // String ID should pass
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("test-id".to_string())),
            method: "tools/list".to_string(),
            params: None,
        };
        let result = validator.validate_request(&request);
        assert!(result.is_ok());
        
        // Number ID should pass
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(serde_json::Number::from(123))),
            method: "tools/list".to_string(),
            params: None,
        };
        let result = validator.validate_request(&request);
        assert!(result.is_ok());
        
        // Null ID should pass (for notifications)
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Null),
            method: "notifications/initialized".to_string(),
            params: None,
        };
        let result = validator.validate_request(&request);
        assert!(result.is_ok());
        
        // Invalid ID type should fail
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Bool(true)),
            method: "tools/list".to_string(),
            params: None,
        };
        let result = validator.validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be string, number, or null"));
    }

    #[test]
    fn test_initialize_parameter_validation() {
        let validator = McpMessageValidator::new();
        
        // Valid initialize parameters
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("init-1".to_string())),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            })),
        };
        let result = validator.validate_request(&request);
        assert!(result.is_ok());
        
        // Missing clientInfo should fail
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("init-2".to_string())),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05"
            })),
        };
        let result = validator.validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing 'clientInfo'"));
        
        // Missing name in clientInfo should fail
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("init-3".to_string())),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "clientInfo": {
                    "version": "1.0.0"
                }
            })),
        };
        let result = validator.validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing required 'name'"));
    }

    #[test]
    fn test_tool_call_parameter_validation() {
        let validator = McpMessageValidator::new();
        
        // Valid tool call parameters
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("call-1".to_string())),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "test-tool",
                "arguments": {
                    "param1": "value1"
                }
            })),
        };
        let result = validator.validate_request(&request);
        assert!(result.is_ok());
        
        // Missing name should fail
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("call-2".to_string())),
            method: "tools/call".to_string(),
            params: Some(json!({
                "arguments": {
                    "param1": "value1"
                }
            })),
        };
        let result = validator.validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing required 'name'"));
    }

    #[test]
    fn test_method_specific_validation() {
        let validator = McpMessageValidator::new();
        
        // Initialize must have ID
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            })),
        };
        let result = validator.validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must have an ID"));
        
        // Regular methods must have ID
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "tools/list".to_string(),
            params: None,
        };
        let result = validator.validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must have an ID"));
        
        // Notifications should not have ID (but we only warn, don't fail)
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String("notif-1".to_string())),
            method: "notifications/initialized".to_string(),
            params: None,
        };
        let result = validator.validate_request(&request);
        assert!(result.is_ok()); // Should pass but log warning
    }
}
