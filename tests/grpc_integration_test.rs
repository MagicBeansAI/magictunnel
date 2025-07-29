use std::sync::Arc;

use magictunnel::registry::RegistryService;
use magictunnel::config::{RegistryConfig, ValidationConfig};
use magictunnel::grpc::McpGrpcServer;
use magictunnel::grpc::mcp_service_server::McpServiceServer;

#[cfg(test)]
mod grpc_tests {
    use super::*;

    // Import the generated protobuf types from the server module
    use magictunnel::grpc::server::{ListToolsRequest, CallToolRequest, McpMessage, ListToolsResponse, ToolError, HeartbeatMessage};

    #[tokio::test]
    async fn test_grpc_server_creation() {
        // Create a test registry
        let config = RegistryConfig {
            r#type: "file".to_string(),
            paths: vec!["test_capabilities".to_string()],
            hot_reload: false,
            validation: ValidationConfig {
                strict: false,
                allow_unknown_fields: true,
            },
        };

        let registry = Arc::new(RegistryService::new(config).await.expect("Failed to create registry"));
        
        // Create gRPC server
        let grpc_server = McpGrpcServer::new(registry);
        
        // Verify server was created successfully
        assert!(true, "gRPC server created successfully");
    }

    #[tokio::test]
    async fn test_list_tools_request_structure() {
        // Test that we can create a ListToolsRequest
        let request = ListToolsRequest {
            name_pattern: Some("test".to_string()),
            routing_type: Some("subprocess".to_string()),
        };

        assert_eq!(request.name_pattern, Some("test".to_string()));
        assert_eq!(request.routing_type, Some("subprocess".to_string()));
    }

    #[tokio::test]
    async fn test_call_tool_request_structure() {
        // Test that we can create a CallToolRequest
        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: "{}".to_string(),
        };

        assert_eq!(request.name, "test_tool");
        assert_eq!(request.arguments, "{}");
    }

    #[tokio::test]
    async fn test_mcp_message_structure() {
        // Test that we can create an McpMessage
        let message = McpMessage {
            id: "test-123".to_string(),
            message_type: None, // Will be set based on specific message type
        };
        
        assert_eq!(message.id, "test-123");
        assert!(message.message_type.is_none());
    }

    #[tokio::test]
    async fn test_grpc_service_trait_implementation() {
        // Create a test registry
        let config = RegistryConfig {
            r#type: "file".to_string(),
            paths: vec!["test_capabilities".to_string()],
            hot_reload: false,
            validation: ValidationConfig {
                strict: false,
                allow_unknown_fields: true,
            },
        };

        let registry = Arc::new(RegistryService::new(config).await.expect("Failed to create registry"));
        
        // Create gRPC server
        let grpc_server = McpGrpcServer::new(registry);
        
        // Create the service (this tests that the trait is properly implemented)
        let _service = McpServiceServer::new(grpc_server);
        
        // If we get here, the trait implementation is correct
        assert!(true, "gRPC service trait implementation is correct");
    }

    #[tokio::test]
    async fn test_protobuf_message_types() {
        // Test that all protobuf message types can be created
        
        // Test ListToolsResponse
        let list_response = ListToolsResponse {
            tools: vec![],
        };
        assert!(list_response.tools.is_empty());
        
        // Test ToolError
        let error = ToolError {
            code: "TEST_ERROR".to_string(),
            message: "Test error message".to_string(),
            details: None,
        };
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test error message");
        
        // Test HeartbeatMessage
        let heartbeat = HeartbeatMessage {
            timestamp: 1234567890,
            count: 1,
        };
        assert_eq!(heartbeat.timestamp, 1234567890);
        assert_eq!(heartbeat.count, 1);
    }
}
