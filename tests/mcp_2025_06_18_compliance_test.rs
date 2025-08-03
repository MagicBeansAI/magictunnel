//! Comprehensive MCP 2025-06-18 Specification Compliance Tests
//!
//! Tests for sampling, elicitation, and roots capabilities according to MCP 2025-06-18 spec

use magictunnel::mcp::server::McpServer;
use magictunnel::mcp::types::*;
use magictunnel::config::{Config, RegistryConfig};
use serde_json::{json, Value};
use std::sync::Arc;

/// Create a test server with MCP 2025-06-18 capabilities enabled
async fn create_test_server_with_2025_capabilities() -> Result<Arc<McpServer>, Box<dyn std::error::Error>> {
    let mut config = Config::default();
    
    // Enable smart discovery to activate 2025-06-18 capabilities
    config.smart_discovery = Some(magictunnel::discovery::SmartDiscoveryConfig {
        enabled: true,
        tool_selection_mode: "rule_based".to_string(),
        default_confidence_threshold: 0.7,
        max_tools_to_consider: 10,
        max_high_quality_matches: 5,
        high_quality_threshold: 0.95,
        use_fuzzy_matching: true,
        llm_mapper: magictunnel::discovery::LlmMapperConfig::default(),
        llm_tool_selection: magictunnel::discovery::LlmToolSelectionConfig::default(),
        cache: magictunnel::discovery::DiscoveryCacheConfig::default(),
        fallback: magictunnel::discovery::FallbackConfig::default(),
        semantic_search: magictunnel::discovery::SemanticSearchConfig::default(),
        enable_sequential_mode: true,
        tool_metrics_enabled: Some(true),
        enable_sampling: Some(false),
        enable_elicitation: Some(false),
    });
    
    // Set up registry config
    config.registry = RegistryConfig::default();
    
    let server = McpServer::with_config(&config).await?;
    Ok(Arc::new(server))
}

/// Test MCP 2025-06-18 protocol version compliance
#[tokio::test]
async fn test_protocol_version_2025_06_18() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    let capabilities = server.get_capabilities();
    
    // Check protocol version
    assert_eq!(capabilities["protocolVersion"], "2025-06-18");
    
    // Verify implementation info
    let implementation = &capabilities["implementation"];
    assert_eq!(implementation["name"], "MagicTunnel");
    assert!(implementation["version"].is_string());
}

/// Test that all required 2025-06-18 capabilities are declared
#[tokio::test]
async fn test_mcp_2025_capabilities_declaration() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    let capabilities = server.get_capabilities();
    let caps = &capabilities["capabilities"];
    
    // Verify sampling capability
    assert!(caps["sampling"].is_object(), "sampling capability must be declared");
    
    // Verify elicitation capability
    assert!(caps["elicitation"].is_object(), "elicitation capability must be declared");
    
    // Verify roots capability
    assert!(caps["roots"].is_object(), "roots capability must be declared");
    
    // Verify existing capabilities are still present
    assert!(caps["tools"].is_object(), "tools capability must be present");
    assert!(caps["resources"].is_object(), "resources capability must be present");
    assert!(caps["prompts"].is_object(), "prompts capability must be present");
}

/// Test sampling/createMessage handler with valid request
#[tokio::test]
async fn test_sampling_create_message_handler() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    
    let sampling_request = json!({
        "jsonrpc": "2.0",
        "id": "test-sampling-1",
        "method": "sampling/createMessage",
        "params": {
            "messages": [
                {
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": "Hello, how are you?"
                    }
                }
            ],
            "model_preferences": {
                "hints": [
                    {
                        "name": "model_name",
                        "value": "gpt-3.5-turbo"
                    }
                ],
                "cost_priority": 0.7,
                "speed_priority": 0.5,
                "intelligence_priority": 0.8
            },
            "max_tokens": 100,
            "temperature": 0.7
        }
    });
    
    let request: McpRequest = serde_json::from_value(sampling_request).unwrap();
    let response = server.handle_mcp_request(request).await;
    assert!(response.is_ok());
    
    let response_str = response.unwrap().unwrap_or_else(|| "{}".to_string());
    let response_value: Value = serde_json::from_str(&response_str).unwrap();
    
    // Should have proper JSON-RPC structure
    assert_eq!(response_value["jsonrpc"], "2.0");
    assert_eq!(response_value["id"], "test-sampling-1");
    
    // Should have result or error (depending on LLM availability)
    assert!(response_value.get("result").is_some() || response_value.get("error").is_some());
}

/// Test sampling handler with invalid parameters
#[tokio::test]
async fn test_sampling_invalid_parameters() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    
    let invalid_request = json!({
        "jsonrpc": "2.0",
        "id": "test-sampling-invalid",
        "method": "sampling/createMessage",
        "params": {
            // Missing required messages field
            "max_tokens": 100
        }
    });
    
    let request: McpRequest = serde_json::from_value(invalid_request).unwrap();
    let response = server.handle_mcp_request(request).await;
    assert!(response.is_ok());
    
    let response_str = response.unwrap().unwrap_or_else(|| "{}".to_string());
    let response_value: Value = serde_json::from_str(&response_str).unwrap();
    
    // Should return error for invalid parameters
    assert_eq!(response_value["jsonrpc"], "2.0");
    assert_eq!(response_value["id"], "test-sampling-invalid");
    assert!(response_value["error"].is_object());
    assert_eq!(response_value["error"]["code"], -32602); // Invalid params
}

/// Test elicitation/create handler with valid schema
#[tokio::test]
async fn test_elicitation_create_handler() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    
    let elicitation_request = json!({
        "jsonrpc": "2.0",
        "id": "test-elicitation-1",
        "method": "elicitation/create",
        "params": {
            "message": "Please provide your basic information",
            "requested_schema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Your full name"
                    },
                    "age": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 150
                    },
                    "email": {
                        "type": "string",
                        "format": "email"
                    }
                },
                "required": ["name"]
            },
            "context": {
                "source": "user_registration",
                "reason": "Complete user profile",
                "privacy_level": "internal"
            },
            "timeout_seconds": 300,
            "priority": "normal"
        }
    });
    
    let request: McpRequest = serde_json::from_value(elicitation_request).unwrap();
    let response = server.handle_mcp_request(request).await;
    assert!(response.is_ok());
    
    let response_str = response.unwrap().unwrap_or_else(|| "{}".to_string());
    let response_value: Value = serde_json::from_str(&response_str).unwrap();
    
    // Should have proper JSON-RPC structure
    assert_eq!(response_value["jsonrpc"], "2.0");
    assert_eq!(response_value["id"], "test-elicitation-1");
    
    // For elicitation, should return a request_id
    if let Some(result) = response_value.get("result") {
        assert!(result["request_id"].is_string());
    } else {
        // Or an error if elicitation service isn't configured
        assert!(response_value["error"].is_object());
    }
}

/// Test elicitation with complex nested schema (should be rejected)
#[tokio::test]
async fn test_elicitation_nested_schema_rejection() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    
    let nested_schema_request = json!({
        "jsonrpc": "2.0",
        "id": "test-elicitation-nested",
        "method": "elicitation/create",
        "params": {
            "message": "Please provide complex nested data",
            "requested_schema": {
                "type": "object",
                "properties": {
                    "user": {
                        "type": "object", // Nested object - should be rejected
                        "properties": {
                            "name": {"type": "string"},
                            "details": {
                                "type": "object", // Double nested - definitely rejected
                                "properties": {
                                    "age": {"type": "integer"}
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    
    let request: McpRequest = serde_json::from_value(nested_schema_request).unwrap();
    let response = server.handle_mcp_request(request).await;
    assert!(response.is_ok());
    
    let response_str = response.unwrap().unwrap_or_else(|| "{}".to_string());
    let response_value: Value = serde_json::from_str(&response_str).unwrap();
    
    // Should return error for complex schema
    assert_eq!(response_value["jsonrpc"], "2.0");
    assert_eq!(response_value["id"], "test-elicitation-nested");
    assert!(response_value["error"].is_object());
    
    // Error should indicate schema complexity issue
    let error_message = response_value["error"]["message"].as_str().unwrap();
    assert!(error_message.to_lowercase().contains("schema") || 
            error_message.to_lowercase().contains("complex"));
}

/// Test roots/list handler
#[tokio::test]
async fn test_roots_list_handler() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    
    let roots_request = json!({
        "jsonrpc": "2.0",
        "id": "test-roots-1",
        "method": "roots/list",
        "params": {
            "limit": 10,
            "filter": {
                "types": ["filesystem"],
                "accessible_only": true
            }
        }
    });
    
    let request: McpRequest = serde_json::from_value(roots_request).unwrap();
    let response = server.handle_mcp_request(request).await;
    assert!(response.is_ok());
    
    let response_str = response.unwrap().unwrap_or_else(|| "{}".to_string());
    let response_value: Value = serde_json::from_str(&response_str).unwrap();
    
    // Should have proper JSON-RPC structure
    assert_eq!(response_value["jsonrpc"], "2.0");
    assert_eq!(response_value["id"], "test-roots-1");
    
    if let Some(result) = response_value.get("result") {
        // Should have roots array
        assert!(result["roots"].is_array());
        
        // Validate roots structure
        let roots = result["roots"].as_array().unwrap();
        for root in roots {
            assert!(root["id"].is_string());
            assert!(root["type"].is_string());
            assert!(root["uri"].is_string());
            assert!(root["accessible"].is_boolean());
        }
        
        // Should have pagination info if applicable
        if let Some(total_count) = result.get("total_count") {
            assert!(total_count.is_number());
        }
    } else {
        // Or an error if roots service isn't configured
        assert!(response_value["error"].is_object());
    }
}

/// Test roots/list with pagination
#[tokio::test]
async fn test_roots_list_pagination() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    
    let paginated_request = json!({
        "jsonrpc": "2.0",
        "id": "test-roots-pagination",
        "method": "roots/list",
        "params": {
            "limit": 5,
            "cursor": null
        }
    });
    
    let request: McpRequest = serde_json::from_value(paginated_request).unwrap();
    let response = server.handle_mcp_request(request).await;
    assert!(response.is_ok());
    
    let response_str = response.unwrap().unwrap_or_else(|| "{}".to_string());
    let response_value: Value = serde_json::from_str(&response_str).unwrap();
    
    if let Some(result) = response_value.get("result") {
        let roots = result["roots"].as_array().unwrap();
        
        // Should respect limit
        assert!(roots.len() <= 5);
        
        // Should have next_cursor if there are more results
        if let Some(next_cursor) = result.get("next_cursor") {
            assert!(next_cursor.is_string() || next_cursor.is_null());
        }
    }
}

/// Test invalid method handling for 2025-06-18 methods
#[tokio::test]
async fn test_invalid_2025_methods() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    
    let invalid_methods = vec![
        "sampling/invalid",
        "elicitation/invalid", 
        "roots/invalid"
    ];
    
    for method in invalid_methods {
        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("test-invalid-{}", method.replace("/", "-")),
            "method": method,
            "params": {}
        });
        
        let mcp_request: McpRequest = serde_json::from_value(request).unwrap();
        let response = server.handle_mcp_request(mcp_request).await;
        assert!(response.is_ok());
        
        if let Ok(Some(response_str)) = response {
            let response_value: Value = serde_json::from_str(&response_str).unwrap();
            
            // Should return method not found error
            assert_eq!(response_value["jsonrpc"], "2.0");
            assert!(response_value["error"].is_object());
            assert_eq!(response_value["error"]["code"], -32601); // Method not found
        }
    }
}

/// Test backward compatibility with existing MCP methods
#[tokio::test]
async fn test_backward_compatibility() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    
    // Test existing methods still work
    let legacy_methods = vec![
        ("tools/list", json!({})),
        ("resources/list", json!({})),
        ("prompts/list", json!({})),
    ];
    
    for (method, params) in legacy_methods {
        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("test-legacy-{}", method.replace("/", "-")),
            "method": method,
            "params": params
        });
        
        let mcp_request: McpRequest = serde_json::from_value(request).unwrap();
        let response = server.handle_mcp_request(mcp_request).await;
        assert!(response.is_ok());
        
        if let Ok(Some(response_str)) = response {
            let response_value: Value = serde_json::from_str(&response_str).unwrap();
            
            // Should have proper JSON-RPC structure (either success or known error)
            assert_eq!(response_value["jsonrpc"], "2.0");
            assert!(response_value.get("result").is_some() || response_value.get("error").is_some());
            
            // Should not be "method not found" error
            if let Some(error) = response_value.get("error") {
                assert_ne!(error["code"], -32601);
            }
        }
    }
}

/// Test MCP 2025-06-18 error response format compliance
#[tokio::test]
async fn test_error_response_format() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    
    // Send malformed request to trigger error
    let malformed_request = json!({
        "jsonrpc": "2.0",
        "id": "test-error-format",
        "method": "sampling/createMessage",
        "params": "invalid-params-should-be-object"
    });
    
    let mcp_request: McpRequest = serde_json::from_value(malformed_request).unwrap();
    let response = server.handle_mcp_request(mcp_request).await;
    assert!(response.is_ok());
    
    let response_str = response.unwrap().unwrap_or_else(|| "{}".to_string());
    let response_value: Value = serde_json::from_str(&response_str).unwrap();
    
    // Verify error response format
    assert_eq!(response_value["jsonrpc"], "2.0");
    assert_eq!(response_value["id"], "test-error-format");
    
    let error = &response_value["error"];
    assert!(error.is_object());
    assert!(error["code"].is_number());
    assert!(error["message"].is_string());
    
    // Error code should be valid JSON-RPC error code
    let error_code = error["code"].as_i64().unwrap();
    assert!(error_code >= -32768 && error_code <= -32000 || error_code >= -32099 && error_code <= -32000);
}

/// Integration test for complete MCP 2025-06-18 workflow
#[tokio::test]
async fn test_complete_2025_06_18_workflow() {
    let server = create_test_server_with_2025_capabilities().await.unwrap();
    
    // 1. Check capabilities
    let capabilities = server.get_capabilities();
    assert_eq!(capabilities["protocolVersion"], "2025-06-18");
    
    // 2. List roots
    let roots_request = json!({
        "jsonrpc": "2.0",
        "id": "workflow-roots",
        "method": "roots/list",
        "params": {"limit": 5}
    });
    
    let roots_mcp_request: McpRequest = serde_json::from_value(roots_request).unwrap();
    let roots_response = server.handle_mcp_request(roots_mcp_request).await;
    assert!(roots_response.is_ok());
    
    // 3. Create elicitation request
    let elicitation_request = json!({
        "jsonrpc": "2.0",
        "id": "workflow-elicitation",
        "method": "elicitation/create",
        "params": {
            "message": "Provide test data",
            "requested_schema": {
                "type": "object",
                "properties": {
                    "test_field": {"type": "string"}
                }
            }
        }
    });
    
    let elicitation_mcp_request: McpRequest = serde_json::from_value(elicitation_request).unwrap();
    let elicitation_response = server.handle_mcp_request(elicitation_mcp_request).await;
    assert!(elicitation_response.is_ok());
    
    // 4. Attempt sampling (may fail without LLM config, but should handle gracefully)
    let sampling_request = json!({
        "jsonrpc": "2.0",
        "id": "workflow-sampling",
        "method": "sampling/createMessage",
        "params": {
            "messages": [{"role": "user", "content": {"type": "text", "text": "Test"}}]
        }
    });
    
    let sampling_mcp_request: McpRequest = serde_json::from_value(sampling_request).unwrap();
    let sampling_response = server.handle_mcp_request(sampling_mcp_request).await;
    assert!(sampling_response.is_ok());
    
    // All requests should return valid JSON-RPC responses
    let roots_json: Value = serde_json::from_str(&roots_response.unwrap().unwrap_or_else(|| "{}".to_string())).unwrap();
    let elicitation_json: Value = serde_json::from_str(&elicitation_response.unwrap().unwrap_or_else(|| "{}".to_string())).unwrap();
    let sampling_json: Value = serde_json::from_str(&sampling_response.unwrap().unwrap_or_else(|| "{}".to_string())).unwrap();
    
    assert_eq!(roots_json["jsonrpc"], "2.0");
    assert_eq!(elicitation_json["jsonrpc"], "2.0");
    assert_eq!(sampling_json["jsonrpc"], "2.0");
}