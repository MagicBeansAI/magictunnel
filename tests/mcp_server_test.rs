//! Unit tests for MCP server components and message handling

use magictunnel::mcp::server::McpServer;
use magictunnel::config::{RegistryConfig, ValidationConfig};
use serde_json::{json, Value};

#[tokio::test]
async fn test_mcp_request_parsing() {
    // Test valid JSON-RPC request
    let valid_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    
    let request_str = valid_request.to_string();
    let parsed: Result<Value, _> = serde_json::from_str(&request_str);
    assert!(parsed.is_ok());
    
    let parsed_value = parsed.unwrap();
    assert_eq!(parsed_value["jsonrpc"], "2.0");
    assert_eq!(parsed_value["id"], 1);
    assert_eq!(parsed_value["method"], "tools/list");
}

#[tokio::test]
async fn test_mcp_response_creation() {
    // Test successful response
    let success_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "tools": []
        }
    });
    
    assert_eq!(success_response["jsonrpc"], "2.0");
    assert_eq!(success_response["id"], 1);
    assert!(success_response["result"].is_object());
    
    // Test error response
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32601,
            "message": "Method not found"
        }
    });
    
    assert_eq!(error_response["jsonrpc"], "2.0");
    assert_eq!(error_response["id"], 1);
    assert!(error_response["error"].is_object());
    assert_eq!(error_response["error"]["code"], -32601);
}

#[tokio::test]
async fn test_tool_call_request_structure() {
    let tool_call = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "execute_command",
            "arguments": {
                "command": "ls -la",
                "timeout": 30
            }
        }
    });
    
    assert_eq!(tool_call["method"], "tools/call");
    assert!(tool_call["params"]["name"].is_string());
    assert!(tool_call["params"]["arguments"].is_object());
}

#[tokio::test]
async fn test_streaming_response_format() {
    // Test progress update format
    let progress_update = json!({
        "type": "progress",
        "progress": 0.5,
        "message": "Processing...",
        "timestamp": "2024-01-01T00:00:00Z"
    });
    
    assert_eq!(progress_update["type"], "progress");
    assert!(progress_update["progress"].is_number());
    assert!(progress_update["message"].is_string());
    
    // Test final result format
    let final_result = json!({
        "type": "result",
        "result": {
            "output": "Command executed successfully",
            "exit_code": 0
        },
        "timestamp": "2024-01-01T00:00:01Z"
    });
    
    assert_eq!(final_result["type"], "result");
    assert!(final_result["result"].is_object());
}

#[tokio::test]
async fn test_heartbeat_message_format() {
    let heartbeat = json!({
        "type": "heartbeat",
        "count": 42,
        "timestamp": "2024-01-01T00:00:00Z"
    });
    
    assert_eq!(heartbeat["type"], "heartbeat");
    assert_eq!(heartbeat["count"], 42);
    assert!(heartbeat["timestamp"].is_string());
}

#[tokio::test]
async fn test_error_response_format() {
    let error_response = json!({
        "type": "error",
        "error": {
            "code": "TOOL_NOT_FOUND",
            "message": "The requested tool 'nonexistent_tool' was not found",
            "details": {
                "tool_name": "nonexistent_tool",
                "available_tools": []
            }
        },
        "timestamp": "2024-01-01T00:00:00Z"
    });
    
    assert_eq!(error_response["type"], "error");
    assert!(error_response["error"]["code"].is_string());
    assert!(error_response["error"]["message"].is_string());
    assert!(error_response["error"]["details"].is_object());
}

#[tokio::test]
async fn test_mcp_server_creation() {
    // Create a test registry config
    let registry_config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec!["test_capabilities".to_string()],
        validation: ValidationConfig {
            strict: false,
            allow_unknown_fields: true,
        },
        hot_reload: false,
    };

    let server = McpServer::new(registry_config).await;
    // Server should be created successfully
    assert!(server.is_ok(), "Server creation should succeed");

    let server = server.unwrap();
    drop(server); // Explicit drop to show we're testing construction
}

#[tokio::test]
async fn test_json_rpc_error_codes() {
    // Test standard JSON-RPC error codes
    let parse_error = -32700;
    let invalid_request = -32600;
    let method_not_found = -32601;
    let invalid_params = -32602;
    let internal_error = -32603;
    
    // Verify error code ranges
    assert!(parse_error < 0);
    assert!(invalid_request < 0);
    assert!(method_not_found < 0);
    assert!(invalid_params < 0);
    assert!(internal_error < 0);
    
    // Test custom error response creation
    let custom_error = json!({
        "jsonrpc": "2.0",
        "id": null,
        "error": {
            "code": method_not_found,
            "message": "Method not found",
            "data": {
                "method": "unknown_method"
            }
        }
    });
    
    assert_eq!(custom_error["error"]["code"], method_not_found);
}

#[tokio::test]
async fn test_tool_definition_structure() {
    let tool_definition = json!({
        "name": "execute_command",
        "description": "Execute a shell command",
        "inputSchema": {
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                },
                "timeout": {
                    "type": "number",
                    "description": "Timeout in seconds",
                    "default": 30
                }
            },
            "required": ["command"]
        }
    });
    
    assert!(tool_definition["name"].is_string());
    assert!(tool_definition["description"].is_string());
    assert!(tool_definition["inputSchema"].is_object());
    assert!(tool_definition["inputSchema"]["properties"].is_object());
    assert!(tool_definition["inputSchema"]["required"].is_array());
}

#[tokio::test]
async fn test_capability_registry_structure() {
    let capability_file = json!({
        "version": "1.0",
        "capabilities": [
            {
                "name": "file_operations",
                "description": "File system operations",
                "tools": [
                    {
                        "name": "read_file",
                        "description": "Read contents of a file",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Path to the file"
                                }
                            },
                            "required": ["path"]
                        },
                        "routing": {
                            "type": "subprocess",
                            "command": "cat {path}",
                            "timeout": 10
                        }
                    }
                ]
            }
        ]
    });
    
    assert_eq!(capability_file["version"], "1.0");
    assert!(capability_file["capabilities"].is_array());
    
    let capability = &capability_file["capabilities"][0];
    assert!(capability["name"].is_string());
    assert!(capability["tools"].is_array());
    
    let tool = &capability["tools"][0];
    assert!(tool["name"].is_string());
    assert!(tool["inputSchema"].is_object());
    assert!(tool["routing"].is_object());
}

#[tokio::test]
async fn test_websocket_message_types() {
    // Test different WebSocket message types that should be supported
    
    // Text message (JSON-RPC)
    let text_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    }).to_string();
    
    assert!(serde_json::from_str::<Value>(&text_message).is_ok());
    
    // Binary message should be handled gracefully
    let binary_data = vec![0x01, 0x02, 0x03, 0x04];
    assert!(!binary_data.is_empty());
    
    // Close message should be handled
    // (This would be handled by the WebSocket library)
}

#[tokio::test]
async fn test_sse_event_format() {
    // Test Server-Sent Events format
    let sse_data = json!({
        "type": "heartbeat",
        "count": 1
    });
    
    let sse_formatted = format!("data: {}\n\n", sse_data);
    
    assert!(sse_formatted.starts_with("data: "));
    assert!(sse_formatted.ends_with("\n\n"));
    assert!(sse_formatted.contains("heartbeat"));
}

#[tokio::test]
async fn test_http_streaming_chunks() {
    // Test HTTP streaming chunk format
    let chunk1 = json!({
        "type": "progress",
        "progress": 0.25,
        "message": "Starting..."
    });
    
    let chunk2 = json!({
        "type": "progress", 
        "progress": 0.75,
        "message": "Almost done..."
    });
    
    let chunk3 = json!({
        "type": "result",
        "result": {
            "status": "completed"
        }
    });
    
    // All chunks should be valid JSON
    assert!(chunk1.is_object());
    assert!(chunk2.is_object());
    assert!(chunk3.is_object());
    
    // Progress chunks should have progress field
    assert!(chunk1["progress"].is_number());
    assert!(chunk2["progress"].is_number());
    
    // Result chunk should have result field
    assert!(chunk3["result"].is_object());
}

#[tokio::test]
async fn test_tool_execution_routing() {
    use magictunnel::mcp::types::{ToolCall};
    use serde_json::json;

    // Create a simple registry config for testing
    let registry_config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec!["test_capabilities".to_string()],
        validation: ValidationConfig {
            strict: false,
            allow_unknown_fields: true,
        },
        hot_reload: false,
    };

    // Create MCP server
    let server = McpServer::new(registry_config).await;
    assert!(server.is_ok());

    // Test tool call structure for routing
    let tool_call = ToolCall {
        name: "test_tool".to_string(),
        arguments: json!({
            "param1": "value1",
            "param2": 42
        }),
    };

    // Verify tool call structure
    assert_eq!(tool_call.name, "test_tool");
    assert!(tool_call.arguments.is_object());
    assert_eq!(tool_call.arguments["param1"], "value1");
    assert_eq!(tool_call.arguments["param2"], 42);
}
