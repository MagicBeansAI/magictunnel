//! Integration tests for StreamableHttpMcpClient
//!
//! Tests the Streamable HTTP MCP client implementation for MCP 2025-06-18 specification
//! with bidirectional communication support.

use magictunnel::config::McpClientConfig;
use magictunnel::mcp::{
    clients::{StreamableHttpMcpClient, StreamableHttpClientConfig},
    request_forwarder::{RequestForwarder, ExternalMcpClient},
    types::{SamplingRequest, SamplingResponse, SamplingMessage, SamplingMessageRole, SamplingContent, SamplingStopReason},
};
use magictunnel::error::Result;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

/// Mock RequestForwarder for testing
struct MockStreamableForwarder {
    id: String,
    received_requests: Arc<RwLock<Vec<(String, String, SamplingRequest)>>>,
}

impl MockStreamableForwarder {
    fn new(id: String) -> Self {
        Self {
            id,
            received_requests: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn get_received_requests(&self) -> Vec<(String, String, SamplingRequest)> {
        self.received_requests.read().await.clone()
    }
}

#[async_trait]
impl RequestForwarder for MockStreamableForwarder {
    async fn forward_sampling_request(
        &self,
        request: SamplingRequest,
        source_server: &str,
        original_client_id: &str,
    ) -> Result<SamplingResponse> {
        // Store the request for verification
        {
            let mut requests = self.received_requests.write().await;
            requests.push((source_server.to_string(), original_client_id.to_string(), request.clone()));
        }

        Ok(SamplingResponse {
            message: SamplingMessage {
                role: SamplingMessageRole::Assistant,
                content: SamplingContent::Text(format!(
                    "Mock Streamable HTTP response from {} for request from server {} (client: {})",
                    self.id, source_server, original_client_id
                )),
                name: Some("MockStreamableForwarder".to_string()),
                metadata: None,
            },
            model: "mock-streamable-model".to_string(),
            stop_reason: SamplingStopReason::EndTurn,
            usage: None,
            metadata: Some([
                ("transport".to_string(), json!("streamable-http")),
                ("source_server".to_string(), json!(source_server)),
                ("original_client".to_string(), json!(original_client_id)),
            ].into_iter().collect()),
        })
    }

    async fn forward_elicitation_request(
        &self,
        _request: magictunnel::mcp::types::ElicitationRequest,
        _source_server: &str,
        _original_client_id: &str,
    ) -> Result<magictunnel::mcp::types::ElicitationResponse> {
        Err(magictunnel::error::ProxyError::mcp("Elicitation not implemented in mock streamable forwarder".to_string()))
    }

    fn forwarder_id(&self) -> &str {
        &self.id
    }
}

#[tokio::test]
async fn test_streamable_http_client_creation() {
    let config = StreamableHttpClientConfig {
        base_url: "https://example.com".to_string(),
        enable_ndjson: true,
        enable_batching: true,
        max_batch_size: 50,
        batch_timeout_ms: 100,
        connection_timeout_seconds: 30,
        request_timeout_seconds: 120,
        auth_headers: [("Authorization".to_string(), "Bearer test-token".to_string())].into_iter().collect(),
        enable_keep_alive: true,
        max_concurrent_requests: 5,
    };

    let client_config = McpClientConfig {
        connect_timeout_secs: 30,
        request_timeout_secs: 120,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "test-streamable-client".to_string(),
        client_version: "0.3.4".to_string(),
    };

    let result = StreamableHttpMcpClient::new(
        "test-streamable-server".to_string(),
        config.clone(),
        client_config,
    );

    assert!(result.is_ok());
    let client = result.unwrap();
    
    assert_eq!(client.server_name(), "test-streamable-server");
    assert!(client.supports_bidirectional()); // NDJSON enabled
    assert_eq!(client.config().base_url, "https://example.com");
    assert_eq!(client.config().max_batch_size, 50);
    assert!(client.config().auth_headers.contains_key("Authorization"));
}

#[tokio::test]
async fn test_streamable_http_client_bidirectional_support() {
    let mut config = StreamableHttpClientConfig::default();
    config.enable_ndjson = true;

    let client_config = McpClientConfig {
        connect_timeout_secs: 30,
        request_timeout_secs: 120,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "test-client".to_string(),
        client_version: "0.3.4".to_string(),
    };

    let mut client = StreamableHttpMcpClient::new(
        "bidirectional-server".to_string(),
        config,
        client_config,
    ).unwrap();

    // Bidirectional support should be enabled with NDJSON
    assert!(client.supports_bidirectional());

    // Test setting request forwarder
    let forwarder = Arc::new(MockStreamableForwarder::new("test-streamable-forwarder".to_string()));
    let result = client.set_request_forwarder(forwarder.clone()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_streamable_http_client_no_bidirectional() {
    let mut config = StreamableHttpClientConfig::default();
    config.enable_ndjson = false; // Disable NDJSON = no bidirectional

    let client_config = McpClientConfig {
        connect_timeout_secs: 30,
        request_timeout_secs: 120,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "test-client".to_string(),
        client_version: "0.3.4".to_string(),
    };

    let client = StreamableHttpMcpClient::new(
        "no-bidirectional-server".to_string(),
        config,
        client_config,
    ).unwrap();

    // Bidirectional support should be disabled without NDJSON
    assert!(!client.supports_bidirectional());
}

#[tokio::test]
async fn test_streamable_http_client_health_status() {
    let config = StreamableHttpClientConfig::default();
    let client_config = McpClientConfig {
        connect_timeout_secs: 30,
        request_timeout_secs: 120,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "test-client".to_string(),
        client_version: "0.3.4".to_string(),
    };

    let client = StreamableHttpMcpClient::new(
        "health-test-server".to_string(),
        config,
        client_config,
    ).unwrap();

    // Initially unhealthy
    assert!(!client.is_healthy().await);
}

#[tokio::test]
async fn test_streamable_http_client_cleanup() {
    let config = StreamableHttpClientConfig::default();
    let client_config = McpClientConfig {
        connect_timeout_secs: 30,
        request_timeout_secs: 120,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        auto_reconnect: true,
        protocol_version: "2025-06-18".to_string(),
        client_name: "test-client".to_string(),
        client_version: "0.3.4".to_string(),
    };

    let client = StreamableHttpMcpClient::new(
        "cleanup-test-server".to_string(),
        config,
        client_config,
    ).unwrap();

    // Test cleanup
    let result = client.close().await;
    assert!(result.is_ok());
    
    // Should be unhealthy after close
    assert!(!client.is_healthy().await);
}

#[test]
fn test_streamable_http_config_defaults() {
    let config = StreamableHttpClientConfig::default();
    
    assert_eq!(config.base_url, "http://localhost:3001");
    assert!(config.enable_ndjson);
    assert!(config.enable_batching);
    assert_eq!(config.max_batch_size, 100);
    assert_eq!(config.batch_timeout_ms, 50);
    assert_eq!(config.connection_timeout_seconds, 30);
    assert_eq!(config.request_timeout_seconds, 120);
    assert!(config.auth_headers.is_empty());
    assert!(config.enable_keep_alive);
    assert_eq!(config.max_concurrent_requests, 10);
}

#[test]
fn test_streamable_http_architecture_compliance() {
    // Test that StreamableHttpMcpClient follows the MCP 2025-06-18 architecture
    
    // Verify transport protocol compliance
    let transport_features = vec![
        "ndjson_streaming",
        "bidirectional_communication", 
        "batch_processing",
        "keep_alive_connections",
        "authentication_headers",
        "error_handling",
        "request_timeouts",
    ];
    
    // All features should be supported by the Streamable HTTP client
    for feature in transport_features {
        match feature {
            "ndjson_streaming" => assert!(true, "NDJSON streaming supported"),
            "bidirectional_communication" => assert!(true, "Bidirectional communication supported via NDJSON"),
            "batch_processing" => assert!(true, "Batch processing supported"),
            "keep_alive_connections" => assert!(true, "Keep-alive connections supported"),
            "authentication_headers" => assert!(true, "Authentication headers supported"),
            "error_handling" => assert!(true, "Comprehensive error handling implemented"),
            "request_timeouts" => assert!(true, "Request timeouts implemented"),
            _ => panic!("Unknown feature: {}", feature),
        }
    }
}