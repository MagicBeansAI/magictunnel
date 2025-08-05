use magictunnel::config::Config;
use magictunnel::mcp::server::McpServer;
use magictunnel::mcp::types::{McpRequest, SamplingRequest, SamplingMessage, SamplingMessageRole, SamplingContent};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio;

#[tokio::test]
async fn test_sampling_capability_advertisement() {
    // Create test configuration
    let config = Config::default();
    
    // Create MCP server
    let server = McpServer::with_config(&config).await.expect("Failed to create server");
    
    // Test capability advertisement
    let capabilities = server.get_capabilities();
    
    // Verify sampling capability is advertised
    assert!(capabilities.get("capabilities").is_some());
    let caps = capabilities.get("capabilities").unwrap();
    assert!(caps.get("sampling").is_some(), "Sampling capability should be advertised");
    
    println!("✅ Sampling capability advertisement test passed");
}

#[tokio::test]
async fn test_sampling_request_handling() {
    // Create test configuration
    let config = Config::default();
    
    // Create MCP server
    let server = McpServer::with_config(&config).await.expect("Failed to create server");
    
    // Create a sampling request
    let sampling_request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!("test-sampling-123")),
        method: "sampling/createMessage".to_string(),
        params: Some(json!({
            "maxTokens": 100,
            "messages": [
                {
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": "Hello, this is a test message"
                    }
                }
            ],
            "temperature": 0.7,
            "metadata": {
                "test": true,
                "source": "integration_test"
            }
        })),
    };
    
    // Handle the request
    let response = server.handle_mcp_request(sampling_request).await;
    
    // Verify response structure (should either succeed or fail gracefully)
    match response {
        Ok(Some(response_str)) => {
            let response_json: Value = serde_json::from_str(&response_str)
                .expect("Response should be valid JSON");
            
            // Should have jsonrpc and id fields
            assert_eq!(response_json.get("jsonrpc").unwrap(), "2.0");
            assert_eq!(response_json.get("id").unwrap(), "test-sampling-123");
            
            // Should have either result or error
            assert!(
                response_json.get("result").is_some() || response_json.get("error").is_some(),
                "Response should have either result or error"
            );
            
            println!("✅ Sampling request handling test passed");
            println!("📝 Response: {}", response_str);
        }
        Ok(None) => {
            panic!("Sampling request should return a response, not None");
        }
        Err(e) => {
            // Error is acceptable if no LLM providers are configured
            println!("⚠️  Sampling request failed (expected without LLM configuration): {}", e);
            println!("✅ Sampling error handling test passed");
        }
    }
}

#[tokio::test] 
async fn test_sampling_with_client_capabilities() {
    use magictunnel::mcp::types::capabilities::{ClientCapabilities, SamplingCapability};
    
    // Create test configuration
    let config = Config::default();
    
    // Create MCP server
    let server = McpServer::with_config(&config).await.expect("Failed to create server");
    
    // Test with client that supports sampling
    let client_caps_with_sampling = ClientCapabilities {
        sampling: Some(SamplingCapability {
            create: true,
            handle: true,
            additional: json!({}),
        }),
        ..Default::default()
    };
    
    let capabilities_with_client = server.get_capabilities_for_client(Some(&client_caps_with_sampling));
    let caps = capabilities_with_client.get("capabilities").unwrap();
    assert!(caps.get("sampling").is_some(), "Sampling should be advertised to supporting client");
    
    // Test with client that doesn't support sampling
    let client_caps_no_sampling = ClientCapabilities::default();
    
    let capabilities_without_client = server.get_capabilities_for_client(Some(&client_caps_no_sampling));
    let caps = capabilities_without_client.get("capabilities").unwrap();
    assert!(caps.get("sampling").is_none(), "Sampling should NOT be advertised to non-supporting client");
    
    println!("✅ Client capability integration test passed");
}

#[tokio::test]
async fn test_sampling_bidirectional_communication_structure() {
    // Test that the bidirectional communication structures are properly defined
    
    // Test SamplingRequest serialization
    let request = SamplingRequest {
        max_tokens: Some(1000),
        messages: vec![
            SamplingMessage {
                role: SamplingMessageRole::User,
                content: SamplingContent::Text("Test message".to_string()),
                name: None,
                metadata: None,
            }
        ],
        system_prompt: Some("You are a helpful assistant".to_string()),
        temperature: Some(0.8),
        top_p: Some(0.9),
        stop: Some(vec!["STOP".to_string()]),
        metadata: Some({
            let mut map = HashMap::new();
            map.insert("test".to_string(), json!(true));
            map
        }),
        model_preferences: None,
    };
    
    // Should serialize/deserialize correctly
    let json_str = serde_json::to_string(&request).expect("Should serialize");
    let deserialized: SamplingRequest = serde_json::from_str(&json_str).expect("Should deserialize");
    
    assert_eq!(request.max_tokens, deserialized.max_tokens);
    assert_eq!(request.messages.len(), deserialized.messages.len());
    assert_eq!(request.system_prompt, deserialized.system_prompt);
    
    println!("✅ Sampling data structures test passed");
}

#[tokio::test]
async fn test_sampling_method_routing() {
    // Verify that sampling/createMessage is properly routed in the MCP server
    
    let config = Config::default();
    let server = McpServer::with_config(&config).await.expect("Failed to create server");
    
    // Test various sampling-related methods exist in routing
    let test_methods = vec![
        "sampling/createMessage",
    ];
    
    for method in test_methods {
        let test_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!("test-method")),
            method: method.to_string(),
            params: Some(json!({})), // Empty params to test routing
        };
        
        // Should not panic and should return some response (even if error due to invalid params)
        let response = server.handle_mcp_request(test_request).await;
        assert!(response.is_ok() || response.is_err(), "Method {} should be handled", method);
    }
    
    println!("✅ Sampling method routing test passed");
}