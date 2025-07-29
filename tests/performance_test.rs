//! Performance and load tests for MCP Proxy
//! Tests concurrent connections, throughput, and response times

use actix_test::{start, TestServer};
use actix_web::{web, App};
use futures_util::{StreamExt, TryStreamExt};
use magictunnel::config::RegistryConfig;
use magictunnel::mcp::server::{
    health_check, list_tools_handler, call_tool_handler,
    websocket_handler, sse_handler, streaming_tool_handler, McpServer
};
use magictunnel::registry::RegistryService;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;

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
            .route("/mcp/stream", web::get().to(sse_handler))
            .route("/mcp/call/stream", web::post().to(streaming_tool_handler))
    })
}

#[actix_rt::test]
async fn test_response_time_health_check() {
    let srv = create_test_server().await;
    
    let start_time = Instant::now();
    let mut response = srv.get("/health").send().await.unwrap();
    let duration = start_time.elapsed();

    assert!(response.status().is_success());

    // Health check should respond within 100ms
    assert!(
        duration < Duration::from_millis(100),
        "Health check took {:?}, expected < 100ms",
        duration
    );

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["service"], "magictunnel");
}

#[actix_rt::test]
async fn test_response_time_tools_list() {
    let srv = create_test_server().await;
    
    let start_time = Instant::now();
    let response = srv.get("/mcp/tools").send().await.unwrap();
    let duration = start_time.elapsed();
    
    assert!(response.status().is_success());
    
    // Tools list should respond within 500ms
    assert!(
        duration < Duration::from_millis(500),
        "Tools list took {:?}, expected < 500ms",
        duration
    );
}

#[actix_rt::test]
async fn test_multiple_health_checks() {
    let srv = create_test_server().await;
    let num_requests = 5;

    let start_time = Instant::now();
    let mut response_times = vec![];

    for _i in 0..num_requests {
        let request_start = Instant::now();
        let mut response = srv.get("/health").send().await.unwrap();
        let request_duration = request_start.elapsed();

        assert!(response.status().is_success());

        let body: Value = response.json().await.unwrap();
        assert_eq!(body["service"], "magictunnel");

        response_times.push(request_duration);
    }

    let total_duration = start_time.elapsed();

    // Calculate average response time
    let total_response_time: Duration = response_times.iter().sum();
    let avg_response_time = total_response_time / num_requests as u32;

    println!(
        "Sequential requests: {}, Total time: {:?}, Avg response: {:?}",
        num_requests, total_duration, avg_response_time
    );

    // Average response time should be reasonable
    assert!(
        avg_response_time < Duration::from_millis(500),
        "Average response time {:?} too high",
        avg_response_time
    );
}

#[actix_rt::test]
async fn test_multiple_tool_calls() {
    let srv = create_test_server().await;
    let num_requests = 3;

    let start_time = Instant::now();
    let mut response_times = vec![];

    // Test multiple calls to list tools endpoint (which should work with empty registry)
    for _i in 0..num_requests {
        let request_start = Instant::now();
        let mut response = srv
            .get("/mcp/tools")
            .send()
            .await
            .unwrap();
        let request_duration = request_start.elapsed();

        assert!(response.status().is_success());
        let _body: Value = response.json().await.unwrap();
        response_times.push(request_duration);
    }

    let total_duration = start_time.elapsed();

    // Calculate average response time
    let total_response_time: Duration = response_times.iter().sum();
    let avg_response_time = total_response_time / num_requests as u32;

    println!(
        "Sequential tool calls: {}, Total time: {:?}, Avg response: {:?}",
        num_requests, total_duration, avg_response_time
    );

    // Should handle tool calls efficiently
    assert!(
        avg_response_time < Duration::from_millis(200),
        "Tool call response time {:?} too high",
        avg_response_time
    );
}

#[actix_rt::test]
async fn test_sse_connection_stability() {
    let srv = create_test_server().await;
    
    let response = srv
        .get("/mcp/stream")
        .insert_header(("Accept", "text/event-stream"))
        .send()
        .await
        .unwrap();
    
    assert!(response.status().is_success());
    
    let mut body_stream = response.into_stream().map_ok(|bytes| bytes);
    let mut events_received = 0;
    let start_time = Instant::now();
    let test_duration = Duration::from_secs(3);

    while start_time.elapsed() < test_duration {
        match timeout(Duration::from_millis(1500), body_stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                let chunk_str = String::from_utf8_lossy(&chunk);
                if chunk_str.starts_with("data: ") {
                    events_received += 1;
                }
            }
            Ok(Some(Err(_))) => break, // Stream error
            Ok(None) => break,         // Stream ended
            Err(_) => break,           // Timeout
        }
    }
    
    println!("SSE events received in {:?}: {}", test_duration, events_received);
    
    // Should receive multiple heartbeat events
    assert!(
        events_received >= 2,
        "Should receive at least 2 events in 3 seconds, got {}",
        events_received
    );
}

#[actix_rt::test]
async fn test_streaming_endpoint_performance() {
    let srv = create_test_server().await;
    
    let request_body = json!({
        "name": "test_streaming_tool",
        "arguments": {
            "duration": 1
        }
    });
    
    let start_time = Instant::now();
    let mut response = srv
        .post("/mcp/call/stream")
        .send_json(&request_body)
        .await
        .unwrap();

    // Debug: Print response status and body
    println!("Response status: {}", response.status());
    if !response.status().is_success() {
        let bytes = response.body().await.unwrap();
        let body_str = String::from_utf8_lossy(&bytes);
        println!("Response body: {}", body_str);
        panic!("Request failed with status: {}", response.status());
    }
    
    let mut body_stream = response.into_stream().map_ok(|bytes| bytes);
    let mut chunks_received = 0;
    let mut first_chunk_time = None;

    while let Some(chunk_result) = body_stream.next().await {
        match chunk_result {
            Ok(chunk) if !chunk.is_empty() => {
                if first_chunk_time.is_none() {
                    first_chunk_time = Some(start_time.elapsed());
                }
                chunks_received += 1;

                if chunks_received >= 5 {
                    break; // Stop after receiving several chunks
                }
            }
            Ok(_) => continue, // Empty chunk
            Err(_) => break,   // Stream error
        }
    }
    
    let total_duration = start_time.elapsed();
    
    println!(
        "Streaming: {} chunks in {:?}, first chunk at {:?}",
        chunks_received,
        total_duration,
        first_chunk_time.unwrap_or(Duration::ZERO)
    );
    
    // Should start streaming quickly
    if let Some(first_chunk) = first_chunk_time {
        assert!(
            first_chunk < Duration::from_millis(500),
            "First chunk took too long: {:?}",
            first_chunk
        );
    }
    
    assert!(chunks_received > 0, "Should receive streaming chunks");
}

#[actix_rt::test]
async fn test_memory_usage_stability() {
    let srv = create_test_server().await;
    
    // Perform many requests to test for memory leaks
    let num_requests = 100;
    
    for i in 0..num_requests {
        let mut response = srv.get("/health").send().await.unwrap();
        assert!(response.status().is_success());

        let _body: Value = response.json().await.unwrap();
        
        // Periodically yield to allow cleanup
        if i % 10 == 0 {
            tokio::task::yield_now().await;
        }
    }
    
    // If we get here without panicking or running out of memory, the test passes
    println!("Completed {} requests successfully", num_requests);
}

#[actix_rt::test]
async fn test_error_handling_performance() {
    let srv = create_test_server().await;
    
    // Test that error responses are also fast
    let invalid_request = json!({
        "name": "nonexistent_tool",
        "arguments": {}
    });
    
    let start_time = Instant::now();
    let mut response = srv
        .post("/mcp/call")
        .send_json(&invalid_request)
        .await
        .unwrap();
    let duration = start_time.elapsed();

    // For error handling test, we expect a 400 Bad Request status for nonexistent tool
    assert_eq!(response.status(), 400);

    // Error responses should also be fast
    assert!(
        duration < Duration::from_millis(200),
        "Error response took {:?}, expected < 200ms",
        duration
    );

    let body: Value = response.json().await.unwrap();
    // Should contain error information
    assert!(body.get("error").is_some() || body.get("result").is_some());
}

/// Test Smart Discovery performance with large tool registry simulation
#[actix_rt::test]
async fn test_smart_discovery_large_registry_performance() {
    use magictunnel::config::Config;
    use magictunnel::discovery::{SmartDiscoveryService, SmartDiscoveryConfig, SmartDiscoveryRequest};
    use std::sync::Arc;
    
    // Create config with larger registry simulation settings
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let smart_discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.6,
        max_tools_to_consider: 100, // Test with large consideration set
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: magictunnel::discovery::LlmMapperConfig {
            provider: "mock".to_string(),
            enabled: false, // Use rule-based for performance testing
            ..magictunnel::discovery::LlmMapperConfig::default()
        },
        cache: magictunnel::discovery::DiscoveryCacheConfig {
            enabled: true,
            max_tool_matches: 1000,
            tool_match_ttl: Duration::from_secs(3600),
            max_llm_responses: 500,
            llm_response_ttl: Duration::from_secs(3600),
            max_registry_entries: 100,
            registry_ttl: Duration::from_secs(300),
        },
        ..SmartDiscoveryConfig::default()
    };
    
    let smart_discovery = SmartDiscoveryService::new(registry, smart_discovery_config).unwrap();
    
    // Generate many diverse requests to simulate large registry load
    let test_requests = vec![
        "read configuration file",
        "make HTTP request",
        "query database",
        "process data",
        "generate report",
        "validate input",
        "transform data",
        "send notification",
        "log message",
        "cache result",
        "encrypt data",
        "parse JSON",
        "render template",
        "check health",
        "backup data",
        "compress file",
        "search text",
        "analyze metrics",
        "schedule task",
        "monitor system",
    ];
    
    let num_iterations = 5; // Multiple iterations to test caching
    let start_time = Instant::now();
    let mut response_times = Vec::new();
    let mut cache_hits = 0;
    
    for iteration in 0..num_iterations {
        for (idx, request_text) in test_requests.iter().enumerate() {
            let request_start = Instant::now();
            
            let request = SmartDiscoveryRequest {
                request: format!("{} iteration {}", request_text, iteration),
                context: Some(format!("Performance test iteration {} request {}", iteration, idx)),
                preferred_tools: None,
                confidence_threshold: Some(0.3),
                include_error_details: None,
                sequential_mode: None,
            };
            
            let response = smart_discovery.discover_and_execute(request).await.unwrap();
            let request_duration = request_start.elapsed();
            
            response_times.push(request_duration);
            
            // Verify response quality
            assert!(response.data.is_some() || response.error.is_some(),
                   "No response for request: {}", request_text);
            assert!(response.metadata.confidence_score >= 0.0);
            assert!(response.metadata.confidence_score <= 1.0);
        }
        
        // Check cache effectiveness after first iteration
        if iteration > 0 {
            let cache_stats = smart_discovery.get_cache_stats().await;
            if let Some(hits) = cache_stats.get("hits") {
                if let Some(hit_count) = hits.as_u64() {
                    cache_hits = hit_count;
                }
            }
        }
    }
    
    let total_duration = start_time.elapsed();
    let total_requests = num_iterations * test_requests.len();
    
    // Calculate performance metrics
    let avg_response_time = response_times.iter().sum::<Duration>() / response_times.len() as u32;
    let min_response_time = response_times.iter().min().unwrap();
    let max_response_time = response_times.iter().max().unwrap();
    
    println!("Smart Discovery Large Registry Performance Test:");
    println!("Total requests: {}", total_requests);
    println!("Total time: {:?}", total_duration);
    println!("Average response time: {:?}", avg_response_time);
    println!("Min response time: {:?}", min_response_time);
    println!("Max response time: {:?}", max_response_time);
    println!("Cache hits: {}", cache_hits);
    println!("Requests per second: {:.2}", total_requests as f64 / total_duration.as_secs_f64());
    
    // Performance assertions
    assert!(avg_response_time < Duration::from_millis(100), 
           "Average response time too high: {:?}", avg_response_time);
    assert!(*max_response_time < Duration::from_millis(500),
           "Max response time too high: {:?}", max_response_time);
    assert!(total_duration.as_secs() < 30,
           "Total test time too long: {:?}", total_duration);
    
    // Cache should be effective after first iteration
    assert!(cache_hits > 0, "Cache should have some hits");
    
    // Should achieve reasonable throughput
    let throughput = total_requests as f64 / total_duration.as_secs_f64();
    assert!(throughput >= 10.0, "Throughput too low: {:.2} req/sec", throughput);
}

/// Test Smart Discovery concurrent performance with large registry
#[actix_rt::test]
async fn test_smart_discovery_concurrent_large_registry_performance() {
    use magictunnel::config::Config;
    use magictunnel::discovery::{SmartDiscoveryService, SmartDiscoveryConfig, SmartDiscoveryRequest};
    use std::sync::Arc;
    use tokio::time::timeout;
    
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let smart_discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.5,
        max_tools_to_consider: 50,
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: magictunnel::discovery::LlmMapperConfig {
            provider: "mock".to_string(),
            enabled: false,
            ..magictunnel::discovery::LlmMapperConfig::default()
        },
        cache: magictunnel::discovery::DiscoveryCacheConfig {
            enabled: true,
            max_tool_matches: 500,
            tool_match_ttl: Duration::from_secs(1800),
            max_llm_responses: 200,
            llm_response_ttl: Duration::from_secs(1800),
            max_registry_entries: 75,
            registry_ttl: Duration::from_secs(300),
        },
        ..SmartDiscoveryConfig::default()
    };
    
    let smart_discovery = Arc::new(SmartDiscoveryService::new(registry, smart_discovery_config).unwrap());
    
    // Generate concurrent requests
    let concurrent_requests = 50;
    let request_templates = vec![
        "read file from disk",
        "make API call",
        "query data",
        "process information",
        "generate output",
        "validate input",
        "transform content",
        "send message",
        "log event",
        "cache data",
    ];
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    for i in 0..concurrent_requests {
        let smart_discovery_clone = smart_discovery.clone();
        let request_text = request_templates[i % request_templates.len()].to_string();
        
        let handle = tokio::spawn(async move {
            let request_start = Instant::now();
            
            let request = SmartDiscoveryRequest {
                request: format!("{} - concurrent request {}", request_text, i),
                context: Some(format!("Concurrent performance test {}", i)),
                preferred_tools: None,
                confidence_threshold: Some(0.3),
                include_error_details: None,
                sequential_mode: None,
            };
            
            let response = smart_discovery_clone.discover_and_execute(request).await.unwrap();
            let request_duration = request_start.elapsed();
            
            (i, response, request_duration)
        });
        
        handles.push(handle);
    }
    
    // Wait for all requests with timeout
    let timeout_duration = Duration::from_secs(60);
    let mut results = Vec::new();
    
    for handle in handles {
        match timeout(timeout_duration, handle).await {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(e)) => panic!("Request task failed: {:?}", e),
            Err(_) => panic!("Request timed out after {:?}", timeout_duration),
        }
    }
    
    let total_duration = start_time.elapsed();
    
    // Analyze results
    let mut response_times = Vec::new();
    let mut successful_requests = 0;
    
    for (request_id, response, duration) in results {
        response_times.push(duration);
        
        if response.success || response.error.is_some() {
            successful_requests += 1;
        }
        
        // Verify response quality
        assert!(response.data.is_some() || response.error.is_some(),
               "Concurrent request {} got no response", request_id);
        assert!(response.metadata.confidence_score >= 0.0);
        assert!(response.metadata.confidence_score <= 1.0);
    }
    
    // Calculate performance metrics
    let avg_response_time = response_times.iter().sum::<Duration>() / response_times.len() as u32;
    let max_response_time = response_times.iter().max().unwrap();
    
    println!("Concurrent Large Registry Performance Test:");
    println!("Concurrent requests: {}", concurrent_requests);
    println!("Successful requests: {}", successful_requests);
    println!("Total time: {:?}", total_duration);
    println!("Average response time: {:?}", avg_response_time);
    println!("Max response time: {:?}", max_response_time);
    println!("Concurrent throughput: {:.2} req/sec", 
             concurrent_requests as f64 / total_duration.as_secs_f64());
    
    // Performance assertions for concurrent load
    assert_eq!(successful_requests, concurrent_requests,
              "Not all concurrent requests succeeded");
    assert!(avg_response_time < Duration::from_millis(200),
           "Average concurrent response time too high: {:?}", avg_response_time);
    assert!(*max_response_time < Duration::from_secs(2),
           "Max concurrent response time too high: {:?}", max_response_time);
    assert!(total_duration.as_secs() < 30,
           "Concurrent test took too long: {:?}", total_duration);
    
    // Should achieve good concurrent throughput
    let concurrent_throughput = concurrent_requests as f64 / total_duration.as_secs_f64();
    assert!(concurrent_throughput >= 5.0,
           "Concurrent throughput too low: {:.2} req/sec", concurrent_throughput);
    
    // Check cache effectiveness
    let cache_stats = smart_discovery.get_cache_stats().await;
    println!("Final cache stats: {:?}", cache_stats);
}

/// Test Smart Discovery memory usage and stability under load
#[actix_rt::test]
async fn test_smart_discovery_memory_stability() {
    use magictunnel::config::Config;
    use magictunnel::discovery::{SmartDiscoveryService, SmartDiscoveryConfig, SmartDiscoveryRequest};
    use std::sync::Arc;
    
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    let smart_discovery_config = SmartDiscoveryConfig::default();
    let smart_discovery = SmartDiscoveryService::new(registry, smart_discovery_config).unwrap();
    
    // Perform many requests to test memory stability
    let num_requests = 200;
    let request_patterns = vec![
        "read data from source",
        "process input data",
        "generate output report",
        "validate information",
        "transform content",
    ];
    
    for i in 0..num_requests {
        let pattern = &request_patterns[i % request_patterns.len()];
        
        let request = SmartDiscoveryRequest {
            request: format!("{} iteration {}", pattern, i),
            context: Some(format!("Memory stability test {}", i)),
            preferred_tools: None,
            confidence_threshold: Some(0.4),
            include_error_details: None,
            sequential_mode: None,
        };
        
        let response = smart_discovery.discover_and_execute(request).await.unwrap();
        
        // Verify we still get responses
        assert!(response.data.is_some() || response.error.is_some(),
               "Memory test request {} failed", i);
        
        // Periodically yield to allow cleanup
        if i % 20 == 0 {
            tokio::task::yield_now().await;
            
            // Check that cache isn't growing unbounded
            let cache_stats = smart_discovery.get_cache_stats().await;
            if let Some(entries) = cache_stats.get("entries") {
                if let Some(entry_count) = entries.as_u64() {
                    assert!(entry_count < 1000, 
                           "Cache growing too large: {} entries at iteration {}", 
                           entry_count, i);
                }
            }
        }
    }
    
    // Final verification
    let final_stats = smart_discovery.get_stats().await;
    println!("Memory stability test completed {} requests", num_requests);
    println!("Final stats: discovery_enabled = {}", 
             final_stats.get("discovery_enabled").unwrap_or(&json!(false)));
    
    // Service should still be responsive
    let final_request = SmartDiscoveryRequest {
        request: "final test request".to_string(),
        context: Some("Memory stability final test".to_string()),
        preferred_tools: None,
        confidence_threshold: None,
        include_error_details: None,
        sequential_mode: None,
    };
    
    let final_response = smart_discovery.discover_and_execute(final_request).await.unwrap();
    assert!(final_response.data.is_some() || final_response.error.is_some(),
           "Service not responsive after memory test");
}
