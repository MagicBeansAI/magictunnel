//! Basic MCP 2025-06-18 Specification Tests
//!
//! Simple tests to validate that the 2025-06-18 capabilities are properly declared and accessible

use magictunnel::mcp::server::McpServer;
use magictunnel::config::{Config, RegistryConfig};
use serde_json::json;
use std::sync::Arc;

/// Create a test server with default config that enables 2025-06-18 capabilities
async fn create_test_server() -> Result<Arc<McpServer>, Box<dyn std::error::Error>> {
    let mut config = Config::default();
    config.registry = RegistryConfig::default();
    
    // Enable smart discovery which activates the 2025-06-18 capabilities
    config.smart_discovery = Some(magictunnel::discovery::SmartDiscoveryConfig::default());
    
    let server = McpServer::with_config(&config).await?;
    Ok(Arc::new(server))
}

/// Test that MCP server declares 2025-06-18 protocol version
#[tokio::test]
async fn test_protocol_version_declaration() {
    let server = create_test_server().await.unwrap();
    let capabilities = server.get_capabilities();
    
    // Should declare 2025-06-18 protocol version
    assert_eq!(capabilities["protocolVersion"], "2025-06-18");
    
    // Should have implementation info
    let implementation = &capabilities["implementation"];
    assert_eq!(implementation["name"], "MagicTunnel");
    assert!(implementation["version"].is_string());
}

/// Test that all required 2025-06-18 capabilities are declared
#[tokio::test]
async fn test_capabilities_declaration() {
    let server = create_test_server().await.unwrap();
    let capabilities = server.get_capabilities();
    let caps = &capabilities["capabilities"];
    
    // Verify new 2025-06-18 capabilities are present
    assert!(caps["sampling"].is_object(), "sampling capability must be declared");
    assert!(caps["elicitation"].is_object(), "elicitation capability must be declared");
    assert!(caps["roots"].is_object(), "roots capability must be declared");
    
    // Verify existing capabilities are still present
    assert!(caps["tools"].is_object(), "tools capability must be present");
    assert!(caps["resources"].is_object(), "resources capability must be present");
    assert!(caps["prompts"].is_object(), "prompts capability must be present");
    assert!(caps["logging"].is_object(), "logging capability must be present");
}

/// Test that MCP sampling service is properly integrated
#[tokio::test]
async fn test_mcp_sampling_service_integration() {
    let server = create_test_server().await.unwrap();
    
    // Create a minimal sampling request to test if the handler is registered
    let sampling_request = json!({
        "jsonrpc": "2.0",
        "id": "test-sampling-basic",
        "method": "sampling/createMessage",
        "params": {
            "messages": [
                {
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": "Hello"
                    }
                }
            ]
        }
    });
    
    // Should not return "method not found" error
    let request: magictunnel::mcp::types::McpRequest = serde_json::from_value(sampling_request).unwrap();
    let response = server.handle_mcp_request(request).await;
    
    // Response should be Ok (even if sampling fails due to no LLM config)
    assert!(response.is_ok());
    
    if let Ok(Some(response_str)) = response {
        let response_value: serde_json::Value = serde_json::from_str(&response_str).unwrap();
        
        // Should not be a "method not found" error
        if let Some(error) = response_value.get("error") {
            assert_ne!(error["code"], -32601); // Method not found
        }
    }
}

/// Test that elicitation service is properly integrated
#[tokio::test]
async fn test_elicitation_service_integration() {
    let server = create_test_server().await.unwrap();
    
    // Create a minimal elicitation request
    let elicitation_request = json!({
        "jsonrpc": "2.0",
        "id": "test-elicitation-basic",
        "method": "elicitation/create",
        "params": {
            "prompt": "Please provide a name",
            "inputType": "text",
            "required": false
        }
    });
    
    // Should not return "method not found" error
    let request: magictunnel::mcp::types::McpRequest = serde_json::from_value(elicitation_request).unwrap();
    let response = server.handle_mcp_request(request).await;
    
    // Response should be Ok
    assert!(response.is_ok());
    
    if let Ok(Some(response_str)) = response {
        let response_value: serde_json::Value = serde_json::from_str(&response_str).unwrap();
        
        // Should not be a "method not found" error
        if let Some(error) = response_value.get("error") {
            assert_ne!(error["code"], -32601); // Method not found
        }
        
        // If successful, should return a request_id
        if let Some(result) = response_value.get("result") {
            assert!(result["request_id"].is_string());
        }
    }
}

/// Test that roots service is properly integrated
#[tokio::test]
async fn test_roots_service_integration() {
    let server = create_test_server().await.unwrap();
    
    // Create a minimal roots request
    let roots_request = json!({
        "jsonrpc": "2.0",
        "id": "test-roots-basic",
        "method": "roots/list",
        "params": {
            "limit": 5
        }
    });
    
    // Should not return "method not found" error
    let request: magictunnel::mcp::types::McpRequest = serde_json::from_value(roots_request).unwrap();
    let response = server.handle_mcp_request(request).await;
    
    // Response should be Ok
    assert!(response.is_ok());
    
    if let Ok(Some(response_str)) = response {
        let response_value: serde_json::Value = serde_json::from_str(&response_str).unwrap();
        
        // Should not be a "method not found" error
        if let Some(error) = response_value.get("error") {
            assert_ne!(error["code"], -32601); // Method not found
        }
        
        // If successful, should return roots array
        if let Some(result) = response_value.get("result") {
            assert!(result["roots"].is_array());
        }
    }
}

/// Test that legacy methods still work (backward compatibility)
#[tokio::test]
async fn test_backward_compatibility() {
    let server = create_test_server().await.unwrap();
    
    // Test that existing methods still work
    let legacy_methods = vec![
        "tools/list",
        "resources/list", 
        "prompts/list",
    ];
    
    for method in legacy_methods {
        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("test-legacy-{}", method.replace("/", "-")),
            "method": method,
            "params": {}
        });
        
        let mcp_request: magictunnel::mcp::types::McpRequest = serde_json::from_value(request).unwrap();
        let response = server.handle_mcp_request(mcp_request).await;
        
        // Should get a response
        assert!(response.is_ok());
        
        if let Ok(Some(response_str)) = response {
            let response_value: serde_json::Value = serde_json::from_str(&response_str).unwrap();
            
            // Should have proper JSON-RPC structure
            assert_eq!(response_value["jsonrpc"], "2.0");
            
            // Should not be "method not found" error
            if let Some(error) = response_value.get("error") {
                assert_ne!(error["code"], -32601);
            }
        }
    }
}

/// Test error handling for invalid 2025-06-18 methods
#[tokio::test]
async fn test_invalid_method_handling() {
    let server = create_test_server().await.unwrap();
    
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
        
        let mcp_request: magictunnel::mcp::types::McpRequest = serde_json::from_value(request).unwrap();
        let response = server.handle_mcp_request(mcp_request).await;
        
        // Should get a response
        assert!(response.is_ok());
        
        if let Ok(Some(response_str)) = response {
            let response_value: serde_json::Value = serde_json::from_str(&response_str).unwrap();
            
            // Should return method not found error
            assert_eq!(response_value["jsonrpc"], "2.0");
            if let Some(error) = response_value.get("error") {
                assert_eq!(error["code"], -32601); // Method not found
            }
        }
    }
}

/// Test JSON-RPC error response format compliance
#[tokio::test]
async fn test_error_response_format() {
    let server = create_test_server().await.unwrap();
    
    // Send request with invalid method to trigger error
    let request = json!({
        "jsonrpc": "2.0",
        "id": "test-error-format",
        "method": "nonexistent/method",
        "params": {}
    });
    
    let mcp_request: magictunnel::mcp::types::McpRequest = serde_json::from_value(request).unwrap();
    let response = server.handle_mcp_request(mcp_request).await;
    
    assert!(response.is_ok());
    
    if let Ok(Some(response_str)) = response {
        let response_value: serde_json::Value = serde_json::from_str(&response_str).unwrap();
        
        // Verify error response format
        assert_eq!(response_value["jsonrpc"], "2.0");
        assert_eq!(response_value["id"], "test-error-format");
        
        let error = &response_value["error"];
        assert!(error.is_object());
        assert!(error["code"].is_number());
        assert!(error["message"].is_string());
        
        // Error code should be valid JSON-RPC error code
        let error_code = error["code"].as_i64().unwrap();
        assert_eq!(error_code, -32601); // Method not found
    }
}

/// Integration test for complete MCP 2025-06-18 workflow
#[tokio::test]
async fn test_complete_workflow() {
    let server = create_test_server().await.unwrap();
    
    // 1. Check capabilities
    let capabilities = server.get_capabilities();
    assert_eq!(capabilities["protocolVersion"], "2025-06-18");
    
    // Verify all three new capabilities are declared
    let caps = &capabilities["capabilities"];
    assert!(caps["sampling"].is_object());
    assert!(caps["elicitation"].is_object());
    assert!(caps["roots"].is_object());
    
    // 2. Test each capability endpoint exists (even if not fully functional without config)
    let endpoints = vec![
        ("sampling/createMessage", json!({"messages": []})),
        ("elicitation/create", json!({"prompt": "test", "inputType": "text", "required": false})),
        ("roots/list", json!({})),
    ];
    
    for (method, params) in endpoints {
        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("workflow-{}", method.replace("/", "-")),
            "method": method,
            "params": params
        });
        
        let mcp_request: magictunnel::mcp::types::McpRequest = serde_json::from_value(request).unwrap();
        let response = server.handle_mcp_request(mcp_request).await;
        
        // All endpoints should be reachable (not method not found)
        assert!(response.is_ok());
        
        if let Ok(Some(response_str)) = response {
            let response_value: serde_json::Value = serde_json::from_str(&response_str).unwrap();
            
            // Should not be "method not found" error
            if let Some(error) = response_value.get("error") {
                assert_ne!(error["code"], -32601, "Method {} should be found", method);
            }
        }
    }
}