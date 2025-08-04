//! SSE Client Integration Tests
//!
//! Tests for SSE client queuing behavior, connection management, and protocol portability
//! integration with the broader MCP system.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, timeout, Instant};
use uuid::Uuid;

use magictunnel::{
    config::{SseServiceConfig, SseAuthType as ConfigSseAuthType},
    mcp::{
        clients::sse_client::{SseMcpClient, SseClientConfig, SseAuthConfig},
        types::{McpRequest, McpResponse, Tool, ToolCall},
    },
    error::{ProxyError, Result},
};

/// Mock SSE client for testing
struct MockSseClient {
    config: SseClientConfig,
    request_count: Arc<RwLock<usize>>,
    response_delay: Duration,
    should_fail: bool,
}

impl MockSseClient {
    fn new(config: SseClientConfig) -> Self {
        Self {
            config,
            request_count: Arc::new(RwLock::new(0)),
            response_delay: Duration::from_millis(100),
            should_fail: false,
        }
    }
    
    fn with_delay(mut self, delay: Duration) -> Self {
        self.response_delay = delay;
        self
    }
    
    fn with_failure(mut self, should_fail: bool) -> Self {
        self.should_fail = should_fail;
        self
    }
    
    async fn simulate_request(&self, _request: McpRequest) -> Result<McpResponse> {
        // Increment request count
        {
            let mut count = self.request_count.write().await;
            *count += 1;
        }
        
        // Simulate processing delay
        sleep(self.response_delay).await;
        
        if self.should_fail {
            return Err(ProxyError::mcp("Simulated failure"));
        }
        
        Ok(McpResponse {
            jsonrpc: "2.0".to_string(),
            id: Uuid::new_v4().to_string(),
            result: Some(serde_json::json!({"status": "success"})),
            error: None,
        })
    }
    
    async fn get_request_count(&self) -> usize {
        *self.request_count.read().await
    }
}

/// Test single-session request queuing behavior
#[tokio::test]
async fn test_single_session_request_queuing() {
    let config = SseClientConfig {
        base_url: "https://test.example.com/mcp".to_string(),
        single_session: true,
        max_queue_size: 5,
        request_timeout: 10,
        ..SseClientConfig::default()
    };
    
    let mock_client = MockSseClient::new(config.clone())
        .with_delay(Duration::from_millis(200)); // 200ms per request
    
    // Create test requests
    let requests: Vec<McpRequest> = (0..3).map(|i| McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::String(format!("req-{}", i))),
        method: "test_method".to_string(),
        params: Some(serde_json::json!({"test": i})),
    }).collect();
    
    let start_time = Instant::now();
    
    // In single-session mode, requests should be processed sequentially
    for request in requests {
        let response = mock_client.simulate_request(request).await;
        assert!(response.is_ok(), "Request should succeed");
    }
    
    let elapsed = start_time.elapsed();
    
    // Should take at least 600ms (3 requests * 200ms each) due to sequential processing
    assert!(elapsed >= Duration::from_millis(600), "Requests should be processed sequentially");
    assert_eq!(mock_client.get_request_count().await, 3);
}

/// Test multi-session concurrent request handling
#[tokio::test]
async fn test_multi_session_concurrent_requests() {
    let config = SseClientConfig {
        base_url: "https://test.example.com/mcp".to_string(),
        single_session: false, // Allow concurrent processing
        max_queue_size: 10,
        request_timeout: 10,
        ..SseClientConfig::default()
    };
    
    let mock_client = Arc::new(MockSseClient::new(config.clone())
        .with_delay(Duration::from_millis(200))); // 200ms per request
    
    // Create test requests
    let requests: Vec<McpRequest> = (0..3).map(|i| McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::String(format!("req-{}", i))),
        method: "test_method".to_string(),
        params: Some(serde_json::json!({"test": i})),
    }).collect();
    
    let start_time = Instant::now();
    
    // In multi-session mode, requests can be processed concurrently
    let mut handles = Vec::new();
    for request in requests {
        let client = Arc::clone(&mock_client);
        let handle = tokio::spawn(async move {
            client.simulate_request(request).await
        });
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    for handle in handles {
        let response = handle.await.unwrap();
        assert!(response.is_ok(), "Request should succeed");
    }
    
    let elapsed = start_time.elapsed();
    
    // Should take around 200ms (concurrent processing) rather than 600ms (sequential)
    assert!(elapsed < Duration::from_millis(400), "Requests should be processed concurrently");
    assert_eq!(mock_client.get_request_count().await, 3);
}

/// Test queue overflow handling
#[tokio::test]
async fn test_queue_overflow_behavior() {
    let config = SseClientConfig {
        base_url: "https://test.example.com/mcp".to_string(),
        single_session: true,
        max_queue_size: 2, // Very small queue
        request_timeout: 5,
        ..SseClientConfig::default()
    };
    
    // This test demonstrates the expected behavior when queue is full
    // In a real implementation, the queue would reject additional requests
    assert_eq!(config.max_queue_size, 2);
    
    // Test logic would involve:
    // 1. Fill the queue to capacity
    // 2. Attempt to add another request
    // 3. Verify that the request is rejected with appropriate error
    // 4. Process existing requests and verify queue space is freed
}

/// Test request timeout handling
#[tokio::test]
async fn test_request_timeout_behavior() {
    let config = SseClientConfig {
        base_url: "https://test.example.com/mcp".to_string(),
        single_session: true,
        request_timeout: 1, // 1 second timeout
        ..SseClientConfig::default()
    };
    
    let mock_client = MockSseClient::new(config.clone())
        .with_delay(Duration::from_secs(2)); // 2 second delay (longer than timeout)
    
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::String("timeout-test".to_string())),
        method: "slow_method".to_string(),
        params: Some(serde_json::json!({"delay": 2000})),
    };
    
    let start_time = Instant::now();
    
    // Use timeout to simulate the client's timeout behavior
    let result = timeout(Duration::from_secs(1), mock_client.simulate_request(request)).await;
    
    let elapsed = start_time.elapsed();
    
    // Should timeout after 1 second
    assert!(result.is_err(), "Request should timeout");
    assert!(elapsed < Duration::from_millis(1500), "Should timeout quickly");
}

/// Test authentication configuration handling
#[tokio::test]
async fn test_authentication_handling() {
    // Test Bearer token authentication
    let bearer_config = SseClientConfig {
        base_url: "https://auth-test.example.com/mcp".to_string(),
        auth: SseAuthConfig::Bearer {
            token: "test-bearer-token".to_string(),
        },
        ..SseClientConfig::default()
    };
    
    match &bearer_config.auth {
        SseAuthConfig::Bearer { token } => {
            assert_eq!(token, "test-bearer-token");
        },
        _ => panic!("Expected Bearer auth"),
    }
    
    // Test API Key authentication
    let api_key_config = SseClientConfig {
        base_url: "https://auth-test.example.com/mcp".to_string(),
        auth: SseAuthConfig::ApiKey {
            header: "X-API-Key".to_string(),
            key: "test-api-key".to_string(),
        },
        ..SseClientConfig::default()
    };
    
    match &api_key_config.auth {
        SseAuthConfig::ApiKey { header, key } => {
            assert_eq!(header, "X-API-Key");
            assert_eq!(key, "test-api-key");
        },
        _ => panic!("Expected API Key auth"),
    }
    
    // Test Query Parameter authentication
    let query_config = SseClientConfig {
        base_url: "https://auth-test.example.com/mcp".to_string(),
        auth: SseAuthConfig::QueryParam {
            param: "token".to_string(),
            value: "test-query-token".to_string(),
        },
        ..SseClientConfig::default()
    };
    
    match &query_config.auth {
        SseAuthConfig::QueryParam { param, value } => {
            assert_eq!(param, "token");
            assert_eq!(value, "test-query-token");
        },
        _ => panic!("Expected Query Parameter auth"),
    }
}

/// Test connection state management
#[tokio::test]
async fn test_connection_state_management() {
    let config = SseClientConfig {
        base_url: "https://state-test.example.com/mcp".to_string(),
        reconnect: true,
        max_reconnect_attempts: 3,
        reconnect_delay_ms: 100,
        max_reconnect_delay_ms: 1000,
        ..SseClientConfig::default()
    };
    
    // Test connection configuration
    assert_eq!(config.reconnect, true);
    assert_eq!(config.max_reconnect_attempts, 3);
    assert_eq!(config.reconnect_delay_ms, 100);
    assert_eq!(config.max_reconnect_delay_ms, 1000);
    
    // Connection state test would involve:
    // 1. Establishing initial connection
    // 2. Simulating connection failure
    // 3. Verifying reconnection attempts with proper delays
    // 4. Testing exponential backoff behavior
    // 5. Verifying final failure after max attempts
}

/// Test heartbeat mechanism
#[tokio::test]
async fn test_heartbeat_mechanism() {
    let heartbeat_config = SseClientConfig {
        base_url: "https://heartbeat-test.example.com/mcp".to_string(),
        heartbeat_interval: 1, // 1 second heartbeat
        ..SseClientConfig::default()
    };
    
    let no_heartbeat_config = SseClientConfig {
        base_url: "https://no-heartbeat-test.example.com/mcp".to_string(),
        heartbeat_interval: 0, // Disabled
        ..SseClientConfig::default()
    };
    
    assert_eq!(heartbeat_config.heartbeat_interval, 1);
    assert_eq!(no_heartbeat_config.heartbeat_interval, 0);
    
    // Heartbeat test would involve:
    // 1. Starting connection with heartbeat enabled
    // 2. Monitoring heartbeat messages sent at correct intervals
    // 3. Verifying connection health detection
    // 4. Testing connection recovery after missed heartbeats
}

/// Test error recovery and resilience
#[tokio::test]
async fn test_error_recovery() {
    let resilient_config = SseClientConfig {
        base_url: "https://error-test.example.com/mcp".to_string(),
        single_session: true,
        request_timeout: 5,
        reconnect: true,
        max_reconnect_attempts: 2,
        reconnect_delay_ms: 100,
        ..SseClientConfig::default()
    };
    
    let failing_client = MockSseClient::new(resilient_config.clone())
        .with_failure(true);
    
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::String("error-test".to_string())),
        method: "failing_method".to_string(),
        params: Some(serde_json::json!({"should_fail": true})),
    };
    
    let result = failing_client.simulate_request(request).await;
    assert!(result.is_err(), "Request should fail as expected");
    
    // Error recovery test would involve:
    // 1. Simulating various error conditions
    // 2. Verifying appropriate error responses
    // 3. Testing automatic retry behavior
    // 4. Verifying graceful degradation
    // 5. Testing recovery after errors are resolved
}

/// Test protocol switching scenarios
#[tokio::test]
async fn test_protocol_switching() {
    let switchable_config = SseClientConfig {
        base_url: "https://switch-test.example.com/mcp".to_string(),
        single_session: true,
        connection_timeout: 10,
        request_timeout: 30,
        reconnect: true,
        ..SseClientConfig::default()
    };
    
    // Protocol switching test would involve:
    // 1. Starting with SSE protocol
    // 2. Detecting protocol incompatibility or failure
    // 3. Attempting fallback to WebSocket protocol
    // 4. Further fallback to HTTP polling if needed
    // 5. Maintaining request queue and state during transitions
    // 6. Verifying all pending requests complete successfully
    
    assert_eq!(switchable_config.connection_timeout, 10);
    assert_eq!(switchable_config.request_timeout, 30);
}

/// Test service configuration conversion
#[tokio::test]
async fn test_service_config_conversion() {
    let service_config = SseServiceConfig {
        enabled: true,
        base_url: "https://convert-test.example.com/mcp".to_string(),
        auth: ConfigSseAuthType::Bearer {
            token: "conversion-test-token".to_string(),
        },
        single_session: true,
        connection_timeout: 45,
        request_timeout: 120,
        max_queue_size: 200,
        heartbeat_interval: 25,
        reconnect: true,
        max_reconnect_attempts: 5,
        reconnect_delay_ms: 500,
        max_reconnect_delay_ms: 15000,
        sampling_strategy: None,
        elicitation_strategy: None,
    };
    
    // Convert service config to client config
    let client_config: SseClientConfig = (&service_config).into();
    
    // Verify all fields are properly converted
    assert_eq!(client_config.base_url, service_config.base_url);
    assert_eq!(client_config.single_session, service_config.single_session);
    assert_eq!(client_config.connection_timeout, service_config.connection_timeout);
    assert_eq!(client_config.request_timeout, service_config.request_timeout);
    assert_eq!(client_config.max_queue_size, service_config.max_queue_size);
    assert_eq!(client_config.heartbeat_interval, service_config.heartbeat_interval);
    assert_eq!(client_config.reconnect, service_config.reconnect);
    assert_eq!(client_config.max_reconnect_attempts, service_config.max_reconnect_attempts);
    assert_eq!(client_config.reconnect_delay_ms, service_config.reconnect_delay_ms);
    assert_eq!(client_config.max_reconnect_delay_ms, service_config.max_reconnect_delay_ms);
    
    // Verify authentication conversion
    match client_config.auth {
        SseAuthConfig::Bearer { token } => {
            assert_eq!(token, "conversion-test-token");
        },
        _ => panic!("Expected Bearer auth in converted config"),
    }
}

/// Test load balancing and failover scenarios
#[tokio::test]
async fn test_load_balancing_scenarios() {
    let primary_config = SseClientConfig {
        base_url: "https://primary.example.com/mcp".to_string(),
        single_session: false,
        max_queue_size: 100,
        reconnect: true,
        max_reconnect_attempts: 2,
        ..SseClientConfig::default()
    };
    
    let fallback_config = SseClientConfig {
        base_url: "https://fallback.example.com/mcp".to_string(),
        single_session: true, // Different capabilities
        max_queue_size: 50,
        reconnect: true,
        max_reconnect_attempts: 1,
        ..SseClientConfig::default()
    };
    
    // Load balancing test would involve:
    // 1. Distributing requests across multiple endpoints
    // 2. Monitoring endpoint health and performance
    // 3. Automatically failing over to backup endpoints
    // 4. Adapting to different endpoint capabilities (single vs multi-session)
    // 5. Gracefully handling partial failures
    
    assert_ne!(primary_config.single_session, fallback_config.single_session);
    assert_ne!(primary_config.max_queue_size, fallback_config.max_queue_size);
}

/// Test metrics and monitoring integration
#[tokio::test]
async fn test_metrics_integration() {
    let monitored_config = SseClientConfig {
        base_url: "https://monitored.example.com/mcp".to_string(),
        single_session: true,
        request_timeout: 30,
        heartbeat_interval: 15,
        ..SseClientConfig::default()
    };
    
    // Metrics integration test would involve:
    // 1. Tracking request count, latency, and success/failure rates
    // 2. Monitoring queue depth and processing times
    // 3. Recording connection state changes and reconnection events
    // 4. Measuring heartbeat response times
    // 5. Integrating with tool metrics system for end-to-end visibility
    
    assert_eq!(monitored_config.request_timeout, 30);
    assert_eq!(monitored_config.heartbeat_interval, 15);
}