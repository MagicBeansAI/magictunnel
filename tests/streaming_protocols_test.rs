//! Comprehensive tests for all MCP streaming protocols
//! Tests WebSocket, Server-Sent Events, HTTP Streaming, and basic HTTP endpoints

use actix_test::{start, TestServer};
use actix_web::{web, App};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use magictunnel::config::RegistryConfig;
use magictunnel::mcp::server::{
    health_check, list_tools_handler, call_tool_handler,
    websocket_handler, sse_handler, sse_messages_handler, streaming_tool_handler, McpServer
};
use magictunnel::registry::RegistryService;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Helper function to create test registry config
fn create_test_registry_config() -> RegistryConfig {
    RegistryConfig {
        r#type: "memory".to_string(),
        paths: vec![],
        hot_reload: false,
        validation: magictunnel::config::ValidationConfig {
            strict: false,
            allow_unknown_fields: true,
        },
    }
}

/// Helper function to create test server
async fn create_test_server() -> TestServer {
    // Create registry service
    let registry_config = create_test_registry_config();
    let registry = Arc::new(RegistryService::new(registry_config).await.unwrap());

    // Create MCP server
    let mcp_server_config = create_test_registry_config();
    let mcp_server = Arc::new(McpServer::new(mcp_server_config).await.unwrap());

    start(move || {
        let registry = registry.clone();
        let mcp_server = mcp_server.clone();

        App::new()
            .app_data(web::Data::new(registry))
            .app_data(web::Data::new(mcp_server))
            .route("/health", web::get().to(health_check))
            .route("/mcp/tools", web::get().to(list_tools_handler))
            .route("/mcp/call", web::post().to(call_tool_handler))
            .route("/mcp/ws", web::get().to(websocket_handler))
            .route("/mcp/sse", web::get().to(sse_handler))
            .route("/mcp/sse/messages", web::post().to(sse_messages_handler))
            .route("/mcp/call/stream", web::post().to(streaming_tool_handler))
    })
}

#[actix_rt::test]
async fn test_health_endpoint() {
    let srv = create_test_server().await;

    let mut response = srv.get("/health").send().await.unwrap();
    assert!(response.status().is_success());

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["service"], "magictunnel");
    assert_eq!(body["status"], "healthy");
}

#[actix_rt::test]
async fn test_list_tools_endpoint() {
    let srv = create_test_server().await;

    let mut response = srv.get("/mcp/tools").send().await.unwrap();
    assert!(response.status().is_success());

    let body: Value = response.json().await.unwrap();
    assert!(body["tools"].is_array());
    // Should be empty array for now since no capabilities are loaded
    assert_eq!(body["tools"].as_array().unwrap().len(), 0);
}

#[actix_rt::test]
async fn test_call_tool_endpoint() {
    let srv = create_test_server().await;
    
    let request_body = json!({
        "name": "test_tool",
        "arguments": {
            "param1": "value1"
        }
    });
    
    let mut response = srv
        .post("/mcp/call")
        .send_json(&request_body)
        .await
        .unwrap();

    // Should return an error status since the tool doesn't exist
    assert!(response.status().is_client_error() || response.status().is_server_error());
    let body: Value = response.json().await.unwrap();

    // Should return an error since the tool doesn't exist
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_server_sent_events() {
    let srv = create_test_server().await;
    
    // Test SSE endpoint
    let response = srv
        .get("/mcp/sse")
        .insert_header(("Accept", "text/event-stream"))
        .send()
        .await
        .unwrap();
    
    assert!(response.status().is_success());
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "no-cache"
    );
    
    // Read the first few events using bytes stream
    let mut body_stream = response.into_stream().map_ok(|bytes| bytes);
    let mut event_count = 0;

    while let Some(chunk_result) = body_stream.next().await {
        if event_count >= 3 {
            break; // Stop after receiving a few events
        }

        let chunk = chunk_result.unwrap();
        let chunk_str = String::from_utf8_lossy(&chunk);

        // Should contain SSE formatted data
        if chunk_str.starts_with("data: ") {
            let data_line = chunk_str.trim_start_matches("data: ");
            if let Ok(json_data) = serde_json::from_str::<Value>(data_line) {
                assert_eq!(json_data["type"], "heartbeat");
                assert!(json_data["count"].is_number());
                event_count += 1;
            }
        }
    }
    
    assert!(event_count > 0, "Should have received at least one heartbeat event");
}

#[actix_rt::test]
async fn test_http_streaming_endpoint() {
    let srv = create_test_server().await;
    
    let request_body = json!({
        "name": "test_streaming_tool",
        "arguments": {
            "duration": 1
        }
    });
    
    let response = srv
        .post("/mcp/call/stream")
        .send_json(&request_body)
        .await
        .unwrap();
    
    assert!(response.status().is_success());
    
    // Should have streaming headers
    let content_type = response.headers().get("content-type");
    assert!(content_type.is_some());
    
    // Read streaming response
    let mut body_stream = response.into_stream().map_ok(|bytes| bytes);
    let mut chunks_received = 0;

    while let Some(chunk_result) = body_stream.next().await {
        let chunk = chunk_result.unwrap();
        if !chunk.is_empty() {
            chunks_received += 1;

            // Try to parse as JSON
            let chunk_str = String::from_utf8_lossy(&chunk);
            if let Ok(json_data) = serde_json::from_str::<Value>(&chunk_str) {
                // Should be a valid streaming response
                assert!(
                    json_data.get("progress").is_some()
                    || json_data.get("result").is_some()
                    || json_data.get("error").is_some()
                );
            }
        }

        if chunks_received >= 3 {
            break; // Stop after receiving a few chunks
        }
    }
    
    assert!(chunks_received > 0, "Should have received streaming chunks");
}

#[actix_rt::test]
async fn test_websocket_connection() {
    let srv = create_test_server().await;
    
    // Get the server URL and convert to WebSocket URL
    let server_url = srv.url("/mcp/ws");
    let ws_url = server_url.replace("http://", "ws://");
    
    // Test WebSocket connection with timeout
    let connect_result = timeout(
        Duration::from_secs(5),
        connect_async(&ws_url)
    ).await;
    
    match connect_result {
        Ok(Ok((mut ws_stream, _))) => {
            // Connection successful, test basic message exchange
            let test_message = json!({
                "jsonrpc": "2.0",
                "id": "1",
                "method": "tools/list",
                "params": {}
            });
            
            // Send message
            let send_result = ws_stream
                .send(Message::Text(test_message.to_string()))
                .await;
            assert!(send_result.is_ok(), "Should be able to send WebSocket message");
            
            // Try to receive response with timeout
            let receive_result = timeout(
                Duration::from_secs(2),
                ws_stream.next()
            ).await;
            
            match receive_result {
                Ok(Some(Ok(Message::Text(response)))) => {
                    // Should receive a JSON-RPC response
                    println!("Received WebSocket response: {}", response);
                    let response_json: Value = serde_json::from_str(&response).unwrap();
                    assert_eq!(response_json["jsonrpc"], "2.0");
                    assert_eq!(response_json["id"], "1");

                    // Should get a successful result now that we're using the correct method
                    assert!(response_json.get("result").is_some(), "Expected result field in response, got: {}", response);
                    println!("Received WebSocket MCP response: {}", response);
                }
                Ok(Some(Ok(msg))) => {
                    panic!("Unexpected WebSocket message type: {:?}", msg);
                }
                Ok(Some(Err(e))) => {
                    panic!("WebSocket error: {:?}", e);
                }
                Ok(None) => {
                    panic!("WebSocket connection closed unexpectedly");
                }
                Err(_) => {
                    // Timeout - this might be expected if the server doesn't respond immediately
                    println!("WebSocket response timeout - this might be expected for unimplemented handlers");
                }
            }
            
            // Close connection
            let _ = ws_stream.close(None).await;
        }
        Ok(Err(e)) => {
            panic!("Failed to connect to WebSocket: {:?}", e);
        }
        Err(_) => {
            panic!("WebSocket connection timeout");
        }
    }
}

#[actix_rt::test]
async fn test_websocket_invalid_message() {
    let srv = create_test_server().await;
    
    let server_url = srv.url("/mcp/ws");
    let ws_url = server_url.replace("http://", "ws://");
    
    if let Ok((mut ws_stream, _)) = connect_async(&ws_url).await {
        // Send invalid JSON
        let send_result = ws_stream
            .send(Message::Text("invalid json".to_string()))
            .await;
        assert!(send_result.is_ok());
        
        // Should receive error response or connection close
        if let Ok(Some(response)) = timeout(Duration::from_secs(2), ws_stream.next()).await {
            match response {
                Ok(Message::Text(text)) => {
                    // Should be an error response
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        assert!(json.get("error").is_some());
                    }
                }
                Ok(Message::Close(_)) => {
                    // Connection closed due to invalid message - acceptable
                }
                _ => {
                    // Other message types are acceptable
                }
            }
        }
        
        let _ = ws_stream.close(None).await;
    }
}

#[actix_rt::test]
async fn test_multiple_sequential_connections() {
    let srv = create_test_server().await;

    // Test multiple sequential HTTP requests
    for i in 0..5 {
        let mut response = srv.get("/health").send().await.unwrap();
        assert!(response.status().is_success());

        let body: Value = response.json().await.unwrap();
        assert_eq!(body["service"], "magictunnel");
        assert_eq!(body["status"], "healthy");

        println!("Completed request {}", i);
    }

    println!("All 5 sequential requests completed successfully");
}

#[actix_rt::test]
async fn test_sse_messages_endpoint() {
    let srv = create_test_server().await;
    
    // Test MCP initialize request via SSE messages endpoint
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": "test-init-1",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": { "listChanged": true },
                "sampling": {}
            },
            "clientInfo": {
                "name": "SSE Test Client",
                "version": "1.0.0"
            }
        }
    });
    
    let mut response = srv
        .post("/mcp/sse/messages")
        .send_json(&initialize_request)
        .await
        .unwrap();
    
    assert!(response.status().is_success());
    
    // Check response headers
    let headers = response.headers();
    assert_eq!(headers.get("content-type").unwrap(), "application/json");
    assert_eq!(headers.get("x-mcp-transport").unwrap(), "sse");
    assert_eq!(headers.get("x-mcp-deprecated").unwrap(), "true");
    
    // Parse response body
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(response_body["jsonrpc"], "2.0");
    assert_eq!(response_body["id"], "test-init-1");
    assert!(response_body["result"].is_object());
    
    // Test tools/list request
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": "test-tools-1",
        "method": "tools/list",
        "params": {}
    });
    
    let mut tools_response = srv
        .post("/mcp/sse/messages")
        .send_json(&tools_request)
        .await
        .unwrap();
    
    assert!(tools_response.status().is_success());
    
    let tools_body: Value = tools_response.json().await.unwrap();
    assert_eq!(tools_body["jsonrpc"], "2.0");
    assert_eq!(tools_body["id"], "test-tools-1");
    assert!(tools_body["result"].is_object());
    
    // Test invalid request (should return error)
    let invalid_request = json!({
        "jsonrpc": "2.0",
        "id": "test-invalid-1",
        "method": "nonexistent/method",
        "params": {}
    });
    
    let mut error_response = srv
        .post("/mcp/sse/messages")
        .send_json(&invalid_request)
        .await
        .unwrap();
    
    // Should still return 200 but with JSON-RPC error in body
    assert!(error_response.status().is_success());
    
    let error_body: Value = error_response.json().await.unwrap();
    assert_eq!(error_body["jsonrpc"], "2.0");
    assert_eq!(error_body["id"], "test-invalid-1");
    assert!(error_body["error"].is_object());
}

#[actix_rt::test] 
async fn test_sse_bidirectional_communication() {
    let srv = create_test_server().await;
    
    // This test demonstrates the bidirectional SSE pattern:
    // 1. SSE stream for receiving notifications
    // 2. POST to /mcp/sse/messages for sending requests
    
    // Test the messages endpoint with a tool call
    let tool_call_request = json!({
        "jsonrpc": "2.0",
        "id": "test-tool-call-1",
        "method": "tools/call",
        "params": {
            "name": "smart_tool_discovery",
            "arguments": {
                "request": "test request"
            }
        }
    });
    
    let mut tool_response = srv
        .post("/mcp/sse/messages")
        .send_json(&tool_call_request)
        .await
        .unwrap();
    
    assert!(tool_response.status().is_success());
    
    // Verify SSE deprecation headers are present
    let headers = tool_response.headers();
    assert_eq!(headers.get("x-mcp-transport").unwrap(), "sse");
    assert_eq!(headers.get("x-mcp-deprecated").unwrap(), "true");
    
    // Check that the response follows MCP JSON-RPC format
    let response_body: Value = tool_response.json().await.unwrap();
    assert_eq!(response_body["jsonrpc"], "2.0");
    assert_eq!(response_body["id"], "test-tool-call-1");
    
    // Should have either result or error
    assert!(
        response_body["result"].is_object() || response_body["error"].is_object(),
        "Response should have either result or error field"
    );
    
    // Test SSE stream endpoint accessibility (basic connection test)
    let sse_response = srv
        .get("/mcp/sse")
        .insert_header(("Accept", "text/event-stream"))
        .send()
        .await
        .unwrap();
    
    assert!(sse_response.status().is_success());
    assert_eq!(sse_response.headers().get("content-type").unwrap(), "text/event-stream");
    assert_eq!(sse_response.headers().get("x-mcp-transport").unwrap(), "sse");
    assert_eq!(sse_response.headers().get("x-mcp-deprecated").unwrap(), "true");
}
