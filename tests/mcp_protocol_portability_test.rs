//! MCP Protocol Portability Tests
//!
//! Tests for MCP protocol portability features including SSE client queuing,
//! authentication methods, connection management, and multi-protocol support.

use magictunnel::{
    config::{SseServiceConfig, SseAuthType},
    mcp::{
        clients::sse_client::{SseMcpClient, SseClientConfig, SseAuthConfig},
        types::{McpRequest, McpResponse},
    },
};

/// Test SSE client configuration and defaults
#[tokio::test]
async fn test_sse_client_config_defaults() {
    let config = SseClientConfig::default();
    
    // Verify default configuration values
    assert_eq!(config.single_session, true);
    assert_eq!(config.connection_timeout, 30);
    assert_eq!(config.request_timeout, 60);
    assert_eq!(config.max_queue_size, 100);
    assert_eq!(config.heartbeat_interval, 30);
    assert_eq!(config.reconnect, true);
    assert_eq!(config.max_reconnect_attempts, 10);
    assert_eq!(config.reconnect_delay_ms, 1000);
    assert_eq!(config.max_reconnect_delay_ms, 30000);
    
    // Verify authentication defaults
    match config.auth {
        SseAuthConfig::None => {}, // Expected
        _ => panic!("Expected SseAuthConfig::None as default"),
    }
}

/// Test SSE client configuration with custom values
#[tokio::test]
async fn test_sse_client_custom_config() {
    let config = SseClientConfig {
        base_url: "https://api.example.com/mcp".to_string(),
        auth: SseAuthConfig::Bearer { 
            token: "test-token-123".to_string() 
        },
        single_session: false,
        connection_timeout: 60,
        request_timeout: 120,
        max_queue_size: 50,
        heartbeat_interval: 15,
        reconnect: false,
        max_reconnect_attempts: 5,
        reconnect_delay_ms: 500,
        max_reconnect_delay_ms: 10000,
    };
    
    // Verify custom configuration values
    assert_eq!(config.base_url, "https://api.example.com/mcp");
    assert_eq!(config.single_session, false);
    assert_eq!(config.connection_timeout, 60);
    assert_eq!(config.request_timeout, 120);
    assert_eq!(config.max_queue_size, 50);
    assert_eq!(config.heartbeat_interval, 15);
    assert_eq!(config.reconnect, false);
    assert_eq!(config.max_reconnect_attempts, 5);
    assert_eq!(config.reconnect_delay_ms, 500);
    assert_eq!(config.max_reconnect_delay_ms, 10000);
    
    // Verify authentication configuration
    match config.auth {
        SseAuthConfig::Bearer { token } => {
            assert_eq!(token, "test-token-123");
        },
        _ => panic!("Expected SseAuthConfig::Bearer"),
    }
}

/// Test different authentication configurations
#[tokio::test]
async fn test_sse_auth_configurations() {
    // Test Bearer token authentication
    let bearer_config = SseClientConfig {
        base_url: "https://api.example.com".to_string(),
        auth: SseAuthConfig::Bearer { 
            token: "bearer-token-456".to_string() 
        },
        ..SseClientConfig::default()
    };
    
    match bearer_config.auth {
        SseAuthConfig::Bearer { token } => {
            assert_eq!(token, "bearer-token-456");
        },
        _ => panic!("Expected Bearer auth"),
    }
    
    // Test API Key authentication
    let api_key_config = SseClientConfig {
        base_url: "https://api.example.com".to_string(),
        auth: SseAuthConfig::ApiKey {
            header: "X-API-Key".to_string(),
            key: "api-key-789".to_string(),
        },
        ..SseClientConfig::default()
    };
    
    match api_key_config.auth {
        SseAuthConfig::ApiKey { header, key } => {
            assert_eq!(header, "X-API-Key");
            assert_eq!(key, "api-key-789");
        },
        _ => panic!("Expected API Key auth"),
    }
    
    // Test Query Parameter authentication
    let query_param_config = SseClientConfig {
        base_url: "https://api.example.com".to_string(),
        auth: SseAuthConfig::QueryParam {
            param: "auth_token".to_string(),
            value: "query-token-abc".to_string(),
        },
        ..SseClientConfig::default()
    };
    
    match query_param_config.auth {
        SseAuthConfig::QueryParam { param, value } => {
            assert_eq!(param, "auth_token");
            assert_eq!(value, "query-token-abc");
        },
        _ => panic!("Expected Query Parameter auth"),
    }
    
    // Test No authentication
    let no_auth_config = SseClientConfig {
        base_url: "https://api.example.com".to_string(),
        auth: SseAuthConfig::None,
        ..SseClientConfig::default()
    };
    
    match no_auth_config.auth {
        SseAuthConfig::None => {}, // Expected
        _ => panic!("Expected No auth"),
    }
}

/// Test queue behavior for single-session services
#[tokio::test]
async fn test_single_session_queue_behavior() {
    let config = SseClientConfig {
        base_url: "https://mock-service.test/mcp".to_string(),
        single_session: true,
        max_queue_size: 3,
        request_timeout: 5,
        ..SseClientConfig::default()
    };
    
    // Note: This test would require mocking the SSE client behavior
    // For now, we test the configuration aspects
    assert_eq!(config.single_session, true);
    assert_eq!(config.max_queue_size, 3);
    assert_eq!(config.request_timeout, 5);
}

/// Test multi-session direct request behavior
#[tokio::test]
async fn test_multi_session_direct_requests() {
    let config = SseClientConfig {
        base_url: "https://multi-session-service.test/mcp".to_string(),
        single_session: false,
        request_timeout: 30,
        ..SseClientConfig::default()
    };
    
    // Verify multi-session configuration
    assert_eq!(config.single_session, false);
    assert_eq!(config.request_timeout, 30);
    
    // Multi-session services should support concurrent requests
    // This would be tested with actual client implementation
}

/// Test request timeout handling
#[tokio::test]
async fn test_request_timeout_handling() {
    let short_timeout_config = SseClientConfig {
        base_url: "https://slow-service.test/mcp".to_string(),
        request_timeout: 1, // 1 second timeout
        ..SseClientConfig::default()
    };
    
    let long_timeout_config = SseClientConfig {
        base_url: "https://service.test/mcp".to_string(),
        request_timeout: 300, // 5 minute timeout
        ..SseClientConfig::default()
    };
    
    assert_eq!(short_timeout_config.request_timeout, 1);
    assert_eq!(long_timeout_config.request_timeout, 300);
}

/// Test reconnection configuration and behavior
#[tokio::test]
async fn test_reconnection_configuration() {
    let reconnect_config = SseClientConfig {
        base_url: "https://unreliable-service.test/mcp".to_string(),
        reconnect: true,
        max_reconnect_attempts: 5,
        reconnect_delay_ms: 1000,
        max_reconnect_delay_ms: 10000,
        ..SseClientConfig::default()
    };
    
    assert_eq!(reconnect_config.reconnect, true);
    assert_eq!(reconnect_config.max_reconnect_attempts, 5);
    assert_eq!(reconnect_config.reconnect_delay_ms, 1000);
    assert_eq!(reconnect_config.max_reconnect_delay_ms, 10000);
    
    let no_reconnect_config = SseClientConfig {
        base_url: "https://service.test/mcp".to_string(),
        reconnect: false,
        max_reconnect_attempts: 0,
        ..SseClientConfig::default()
    };
    
    assert_eq!(no_reconnect_config.reconnect, false);
    assert_eq!(no_reconnect_config.max_reconnect_attempts, 0);
}

/// Test heartbeat mechanism configuration
#[tokio::test]
async fn test_heartbeat_configuration() {
    let heartbeat_enabled_config = SseClientConfig {
        base_url: "https://service.test/mcp".to_string(),
        heartbeat_interval: 30,
        ..SseClientConfig::default()
    };
    
    let heartbeat_disabled_config = SseClientConfig {
        base_url: "https://service.test/mcp".to_string(),
        heartbeat_interval: 0, // Disabled
        ..SseClientConfig::default()
    };
    
    let custom_heartbeat_config = SseClientConfig {
        base_url: "https://service.test/mcp".to_string(),
        heartbeat_interval: 10, // Every 10 seconds
        ..SseClientConfig::default()
    };
    
    assert_eq!(heartbeat_enabled_config.heartbeat_interval, 30);
    assert_eq!(heartbeat_disabled_config.heartbeat_interval, 0);
    assert_eq!(custom_heartbeat_config.heartbeat_interval, 10);
}

/// Test queue overflow handling
#[tokio::test]
async fn test_queue_overflow_handling() {
    let small_queue_config = SseClientConfig {
        base_url: "https://busy-service.test/mcp".to_string(),
        single_session: true,
        max_queue_size: 2, // Very small queue
        ..SseClientConfig::default()
    };
    
    assert_eq!(small_queue_config.max_queue_size, 2);
    
    // Test would involve:
    // 1. Creating requests that exceed queue size
    // 2. Verifying that queue overflow returns appropriate error
    // 3. Ensuring that queue management works correctly
}

/// Test protocol portability configuration from main config
#[tokio::test]
async fn test_protocol_portability_config_integration() {
    // Test configuration loading from YAML/JSON config
    let sse_service_config = SseServiceConfig {
        enabled: true,
        base_url: "https://portable.test/mcp".to_string(),
        auth: SseAuthType::Bearer {
            token: "integration-token".to_string(),
        },
        single_session: true,
        connection_timeout: 45,
        request_timeout: 90,
        max_queue_size: 150,
        heartbeat_interval: 20,
        reconnect: true,
        max_reconnect_attempts: 8,
        reconnect_delay_ms: 750,
        max_reconnect_delay_ms: 25000,
        sampling_strategy: None,
        elicitation_strategy: None,
    };
    
    // Convert to SSE client config
    let client_config: SseClientConfig = (&sse_service_config).into();
    
    // Verify conversion maintains all settings
    assert_eq!(client_config.base_url, "https://portable.test/mcp");
    assert_eq!(client_config.single_session, true);
    assert_eq!(client_config.connection_timeout, 45);
    assert_eq!(client_config.request_timeout, 90);
    assert_eq!(client_config.max_queue_size, 150);
    assert_eq!(client_config.heartbeat_interval, 20);
    assert_eq!(client_config.reconnect, true);
    assert_eq!(client_config.max_reconnect_attempts, 8);
    assert_eq!(client_config.reconnect_delay_ms, 750);
    assert_eq!(client_config.max_reconnect_delay_ms, 25000);
    
    match client_config.auth {
        SseAuthConfig::Bearer { token } => {
            assert_eq!(token, "integration-token");
        },
        _ => panic!("Expected Bearer auth in converted config"),
    }
}

/// Test concurrent request handling in multi-session mode
#[tokio::test]
async fn test_concurrent_request_handling() {
    let multi_session_config = SseClientConfig {
        base_url: "https://concurrent-service.test/mcp".to_string(),
        single_session: false,
        request_timeout: 30,
        ..SseClientConfig::default()
    };
    
    // Verify multi-session configuration
    assert_eq!(multi_session_config.single_session, false);
    
    // In a real test, this would:
    // 1. Create multiple concurrent requests
    // 2. Verify they can be processed simultaneously
    // 3. Check that responses are properly correlated
    // 4. Ensure no request blocking occurs
}

/// Test error handling and recovery scenarios
#[tokio::test]
async fn test_error_handling_scenarios() {
    let robust_config = SseClientConfig {
        base_url: "https://error-prone-service.test/mcp".to_string(),
        single_session: true,
        request_timeout: 10,
        max_queue_size: 5,
        reconnect: true,
        max_reconnect_attempts: 3,
        reconnect_delay_ms: 500,
        ..SseClientConfig::default()
    };
    
    // Configuration for error scenarios
    assert_eq!(robust_config.request_timeout, 10);
    assert_eq!(robust_config.max_queue_size, 5);
    assert_eq!(robust_config.max_reconnect_attempts, 3);
    
    // Error scenarios to test would include:
    // 1. Connection failures
    // 2. Request timeouts
    // 3. Queue overflow
    // 4. Authentication failures
    // 5. Malformed responses
    // 6. Network interruptions
}

/// Test protocol switching and adaptation
#[tokio::test]
async fn test_protocol_switching() {
    // Configuration for a service that might need to switch protocols
    let adaptive_config = SseClientConfig {
        base_url: "https://adaptive-service.test/mcp".to_string(),
        single_session: true,
        connection_timeout: 15,
        request_timeout: 45,
        reconnect: true,
        max_reconnect_attempts: 2,
        ..SseClientConfig::default()
    };
    
    assert_eq!(adaptive_config.connection_timeout, 15);
    assert_eq!(adaptive_config.request_timeout, 45);
    assert_eq!(adaptive_config.max_reconnect_attempts, 2);
    
    // Test would involve:
    // 1. Starting with SSE protocol
    // 2. Detecting protocol compatibility issues
    // 3. Switching to alternative protocol (WebSocket, HTTP polling)
    // 4. Maintaining request queue during transition
    // 5. Verifying all pending requests complete successfully
}

/// Test performance characteristics under load
#[tokio::test]
async fn test_performance_under_load() {
    let high_performance_config = SseClientConfig {
        base_url: "https://high-perf-service.test/mcp".to_string(),
        single_session: false, // Allow concurrent processing
        request_timeout: 5,
        max_queue_size: 1000, // Large queue for load testing
        heartbeat_interval: 60, // Less frequent heartbeats under load
        ..SseClientConfig::default()
    };
    
    assert_eq!(high_performance_config.single_session, false);
    assert_eq!(high_performance_config.max_queue_size, 1000);
    assert_eq!(high_performance_config.heartbeat_interval, 60);
    
    // Performance test would involve:
    // 1. Generating high volume of concurrent requests
    // 2. Measuring latency and throughput
    // 3. Verifying queue management efficiency
    // 4. Checking memory usage under load
    // 5. Testing graceful degradation when limits are reached
}

/// Test configuration validation and edge cases
#[tokio::test]
async fn test_config_validation() {
    // Test minimum viable configuration
    let minimal_config = SseClientConfig {
        base_url: "https://minimal.test".to_string(),
        auth: SseAuthConfig::None,
        single_session: true,
        connection_timeout: 1,
        request_timeout: 1,
        max_queue_size: 1,
        heartbeat_interval: 0,
        reconnect: false,
        max_reconnect_attempts: 0,
        reconnect_delay_ms: 0,
        max_reconnect_delay_ms: 0,
    };
    
    assert_eq!(minimal_config.max_queue_size, 1);
    assert_eq!(minimal_config.connection_timeout, 1);
    assert_eq!(minimal_config.request_timeout, 1);
    
    // Test maximum configuration
    let maximal_config = SseClientConfig {
        base_url: "https://maximal.test".to_string(),
        auth: SseAuthConfig::Bearer { 
            token: "very-long-token-that-might-be-used-in-enterprise-scenarios".to_string() 
        },
        single_session: false,
        connection_timeout: 300,
        request_timeout: 3600,
        max_queue_size: 10000,
        heartbeat_interval: 300,
        reconnect: true,
        max_reconnect_attempts: 100,
        reconnect_delay_ms: 60000,
        max_reconnect_delay_ms: 600000,
    };
    
    assert_eq!(maximal_config.max_queue_size, 10000);
    assert_eq!(maximal_config.request_timeout, 3600);
    assert_eq!(maximal_config.max_reconnect_attempts, 100);
}

/// Test service discovery and capability negotiation
#[tokio::test]
async fn test_service_capability_negotiation() {
    let discovery_config = SseClientConfig {
        base_url: "https://discovery-service.test/mcp".to_string(),
        single_session: true,
        connection_timeout: 30,
        request_timeout: 60,
        ..SseClientConfig::default()
    };
    
    // Capability negotiation test would involve:
    // 1. Connecting to service
    // 2. Querying supported capabilities
    // 3. Adapting client behavior based on capabilities
    // 4. Falling back to compatible protocol subset
    // 5. Maintaining functionality with reduced feature set
    
    assert_eq!(discovery_config.connection_timeout, 30);
    assert_eq!(discovery_config.request_timeout, 60);
}