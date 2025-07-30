//! MCP Metrics Integration Tests
//!
//! Tests for MCP service metrics collection and integration with tool metrics.

use chrono::Utc;
use magictunnel::{
    config::Config,
    discovery::SmartDiscoveryService,
    mcp::{
        metrics::{McpMetricsCollector, McpServiceMetrics, McpHealthThresholds},
        server::McpServer,
    },
    registry::RegistryService,
    metrics::tool_metrics::{
        ToolMetricsCollector, ToolExecutionRecord, ToolExecutionResult, DiscoveryRanking
    },
};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Test MCP metrics collection
#[tokio::test]
async fn test_mcp_metrics_collection() {
    let thresholds = McpHealthThresholds::default();
    let collector = McpMetricsCollector::new(thresholds);
    
    // Initialize a service
    collector.initialize_service("test_service").await;
    
    // Record some successful and failed requests
    collector.record_request_success("test_service", 150.0, "list_tools").await;
    collector.record_request_success("test_service", 2500.0, "call_tool").await;
    collector.record_request_error("test_service", "timeout", "call_tool").await;
    collector.record_request_success("test_service", 300.0, "list_resources").await;
    
    // Get service metrics
    let metrics = collector.get_service_metrics("test_service").await;
    assert!(metrics.is_some());
    
    let service_metrics = metrics.unwrap();
    assert_eq!(service_metrics.service_name, "test_service");
    assert_eq!(service_metrics.total_requests, 4);
    assert_eq!(service_metrics.total_errors, 1);
    assert!((service_metrics.success_rate - 0.75).abs() < 0.01); // 3/4 = 0.75
    
    // Check request types tracking
    assert_eq!(service_metrics.request_types.get("call_tool"), Some(&2));
    assert_eq!(service_metrics.request_types.get("list_tools"), Some(&1));
    assert_eq!(service_metrics.request_types.get("list_resources"), Some(&1));
    
    // Check error types tracking
    assert_eq!(service_metrics.error_types.get("timeout"), Some(&1));
}

/// Test MCP metrics summary
#[tokio::test]
async fn test_mcp_metrics_summary() {
    let thresholds = McpHealthThresholds::default();
    let collector = McpMetricsCollector::new(thresholds);
    
    // Initialize multiple services
    collector.initialize_service("service_a").await;
    collector.initialize_service("service_b").await;
    collector.initialize_service("service_c").await;
    
    // Add metrics for multiple services
    collector.record_request_success("service_a", 1000.0, "call_tool").await;
    collector.record_request_success("service_a", 1500.0, "call_tool").await;
    collector.record_request_success("service_a", 200.0, "list_tools").await;
    
    collector.record_request_error("service_b", "network_error", "call_tool").await;
    collector.record_request_success("service_b", 500.0, "list_resources").await;
    
    collector.record_request_success("service_c", 800.0, "call_tool").await;
    
    // Get summary
    let summary = collector.get_summary().await;
    
    assert_eq!(summary.total_services, 3);
    assert_eq!(summary.total_requests, 6);
    assert_eq!(summary.total_errors, 1);
    assert!((summary.overall_error_rate - (1.0/6.0)).abs() < 0.01); // 1/6
    
    // Check service health counts (depends on thresholds and actual metrics)
    assert_eq!(summary.healthy_services + summary.degraded_services + 
               summary.unhealthy_services + summary.down_services, 3);
}

/// Test integration between MCP metrics and tool metrics
#[tokio::test]
async fn test_mcp_tool_metrics_integration() {
    // Create temporary directory for persistent storage
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().join("integration_test_metrics.json");
    
    // Create tool metrics collector
    let tool_metrics = Arc::new(
        ToolMetricsCollector::new_with_storage(100, &storage_path).await.unwrap()
    );
    
    // Create MCP metrics collector
    let thresholds = McpHealthThresholds::default();
    let mcp_metrics = Arc::new(McpMetricsCollector::new(thresholds));
    
    // Initialize a tool in tool metrics
    tool_metrics.initialize_tool("integrated_tool", "integration").await;
    
    // Initialize MCP services
    mcp_metrics.initialize_service("discovery_service").await;
    mcp_metrics.initialize_service("tool_service").await;
    
    // Simulate a tool execution that also involves MCP calls
    
    // 1. Record MCP service call (e.g., to discover the tool)
    mcp_metrics.record_request_success("discovery_service", 250.0, "list_tools").await;
    
    // 2. Record tool execution
    let execution_record = ToolExecutionRecord {
        execution_id: Uuid::new_v4().to_string(),
        tool_name: "integrated_tool".to_string(),
        start_time: Utc::now(),
        duration_ms: 1200,
        result: ToolExecutionResult::Success {
            output_size: 512,
            output_type: "json".to_string(),
        },
        execution_source: "smart_discovery".to_string(),
        input_hash: "integration_hash".to_string(),
        service_source: Some("discovery_service".to_string()),
        discovery_context: Some(DiscoveryRanking {
            position: 1,
            confidence_score: 0.92,
            discovery_method: "hybrid".to_string(),
            query: "integration test".to_string(),
            timestamp: Utc::now(),
        }),
    };
    
    tool_metrics.record_execution(execution_record).await;
    
    // 3. Record MCP service call for tool execution
    mcp_metrics.record_request_success("tool_service", 1200.0, "call_tool").await;
    
    // Verify both metrics systems have recorded the data
    
    // Check tool metrics
    let tool_metrics_data = tool_metrics.get_tool_metrics("integrated_tool").await.unwrap();
    assert_eq!(tool_metrics_data.total_executions, 1);
    assert_eq!(tool_metrics_data.successful_executions, 1);
    assert_eq!(tool_metrics_data.avg_execution_time_ms, 1200.0);
    
    // Check MCP metrics
    let discovery_metrics = mcp_metrics.get_service_metrics("discovery_service").await.unwrap();
    assert_eq!(discovery_metrics.total_requests, 1);
    assert_eq!(discovery_metrics.total_errors, 0);
    assert!(discovery_metrics.avg_response_time_ms > 0.0);
    
    let tool_service_metrics = mcp_metrics.get_service_metrics("tool_service").await.unwrap();
    assert_eq!(tool_service_metrics.total_requests, 1);
    assert_eq!(tool_service_metrics.total_errors, 0);
    assert!(tool_service_metrics.avg_response_time_ms > 0.0);
    
    // Verify correlation through service_source
    let recent_executions = tool_metrics.get_recent_executions(Some(10)).await;
    assert_eq!(recent_executions.len(), 1);
    assert_eq!(recent_executions[0].service_source, Some("discovery_service".to_string()));
}

/// Test metrics collection under concurrent load
#[tokio::test]
async fn test_concurrent_metrics_collection() {
    use std::sync::Arc;
    use tokio::task::JoinSet;
    
    let tool_metrics = Arc::new(ToolMetricsCollector::new(1000));
    let thresholds = McpHealthThresholds::default();
    let mcp_metrics = Arc::new(McpMetricsCollector::new(thresholds));
    
    // Initialize tools and services
    tool_metrics.initialize_tool("concurrent_tool_1", "testing").await;
    tool_metrics.initialize_tool("concurrent_tool_2", "testing").await;
    
    for i in 0..5 {
        let service_name = format!("concurrent_service_{}", i);
        mcp_metrics.initialize_service(&service_name).await;
    }
    
    let mut join_set = JoinSet::new();
    
    // Spawn concurrent tasks that record metrics
    for i in 0..50 {
        let tool_metrics_clone = Arc::clone(&tool_metrics);
        let mcp_metrics_clone = Arc::clone(&mcp_metrics);
        
        join_set.spawn(async move {
            let tool_name = if i % 2 == 0 { "concurrent_tool_1" } else { "concurrent_tool_2" };
            let service_name = format!("concurrent_service_{}", i % 5);
            
            // Record MCP metrics
            if i % 10 == 0 {
                mcp_metrics_clone.record_request_error(&service_name, "concurrent_error", "call_tool").await;
            } else {
                mcp_metrics_clone.record_request_success(&service_name, (100 + i * 10) as f64, "call_tool").await;
            }
            
            // Record tool execution
            let record = ToolExecutionRecord {
                execution_id: format!("concurrent_{}", i),
                tool_name: tool_name.to_string(),
                start_time: Utc::now(),
                duration_ms: 500 + i * 20,
                result: if i % 8 == 0 {
                    ToolExecutionResult::Error {
                        error_type: "ConcurrentError".to_string(),
                        error_message: "Concurrent test error".to_string(),
                        is_timeout: false,
                    }
                } else {
                    ToolExecutionResult::Success {
                        output_size: 256,
                        output_type: "json".to_string(),
                    }
                },
                execution_source: "concurrent_test".to_string(),
                input_hash: format!("concurrent_hash_{}", i),
                service_source: Some(service_name.clone()),
                discovery_context: Some(DiscoveryRanking {
                    position: ((i % 10) + 1) as u32,
                    confidence_score: 0.5 + (i as f64 * 0.01),
                    discovery_method: "concurrent".to_string(),
                    query: format!("concurrent query {}", i),
                    timestamp: Utc::now(),
                }),
            };
            
            tool_metrics_clone.record_execution(record).await;
        });
    }
    
    // Wait for all tasks to complete
    while let Some(_) = join_set.join_next().await {}
    
    // Verify tool metrics
    let tool1_metrics = tool_metrics.get_tool_metrics("concurrent_tool_1").await.unwrap();
    let tool2_metrics = tool_metrics.get_tool_metrics("concurrent_tool_2").await.unwrap();
    
    assert_eq!(tool1_metrics.total_executions + tool2_metrics.total_executions, 50);
    assert_eq!(tool1_metrics.total_executions, 25); // Even indices (0, 2, 4, ..., 48)
    assert_eq!(tool2_metrics.total_executions, 25); // Odd indices (1, 3, 5, ..., 49)
    
    // Verify MCP metrics
    let mcp_summary = mcp_metrics.get_summary().await;
    assert_eq!(mcp_summary.total_services, 5); // concurrent_service_0 through concurrent_service_4
    assert_eq!(mcp_summary.total_requests, 50);
    
    // Check that most requests succeeded (90% success rate expected)
    assert_eq!(mcp_summary.total_errors, 5); // i % 10 == 0 for i in 0..50
    assert!((mcp_summary.overall_error_rate - 0.1).abs() < 0.01); // 5/50 = 0.1
}

/// Test metrics persistence across restarts
#[tokio::test]
async fn test_metrics_persistence_across_restarts() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().join("persistence_test_metrics.json");
    
    // First session: Create metrics and record data
    {
        let tool_metrics = ToolMetricsCollector::new_with_storage(100, &storage_path).await.unwrap();
        
        tool_metrics.initialize_tool("persistent_tool", "persistence").await;
        
        let record = ToolExecutionRecord {
            execution_id: Uuid::new_v4().to_string(),
            tool_name: "persistent_tool".to_string(),
            start_time: Utc::now(),
            duration_ms: 1500,
            result: ToolExecutionResult::Success {
                output_size: 1024,
                output_type: "json".to_string(),
            },
            execution_source: "persistence_test".to_string(),
            input_hash: "persistence_hash".to_string(),
            service_source: Some("persistence_service".to_string()),
            discovery_context: Some(DiscoveryRanking {
                position: 1,
                confidence_score: 0.95,
                discovery_method: "persistent".to_string(),
                query: "persistence test".to_string(),
                timestamp: Utc::now(),
            }),
        };
        
        tool_metrics.record_execution(record).await;
        
        // Verify data exists
        let metrics = tool_metrics.get_tool_metrics("persistent_tool").await.unwrap();
        assert_eq!(metrics.total_executions, 1);
        assert_eq!(metrics.avg_execution_time_ms, 1500.0);
    } // tool_metrics goes out of scope, simulating restart
    
    // Second session: Load from persistence and verify data
    {
        let tool_metrics = ToolMetricsCollector::new_with_storage(100, &storage_path).await.unwrap();
        
        // Data should be loaded from disk
        let loaded_metrics = tool_metrics.get_tool_metrics("persistent_tool").await.unwrap();
        assert_eq!(loaded_metrics.total_executions, 1);
        assert_eq!(loaded_metrics.successful_executions, 1);
        assert_eq!(loaded_metrics.avg_execution_time_ms, 1500.0);
        assert_eq!(loaded_metrics.tool_name, "persistent_tool");
        assert_eq!(loaded_metrics.category, "persistence");
        assert_eq!(loaded_metrics.top_30_appearances, 1);
        
        let loaded_executions = tool_metrics.get_recent_executions(Some(10)).await;
        assert_eq!(loaded_executions.len(), 1);
        assert_eq!(loaded_executions[0].tool_name, "persistent_tool");
        assert_eq!(loaded_executions[0].duration_ms, 1500);
        
        // Add more data to verify continuation
        let record2 = ToolExecutionRecord {
            execution_id: Uuid::new_v4().to_string(),
            tool_name: "persistent_tool".to_string(),
            start_time: Utc::now(),
            duration_ms: 2000,
            result: ToolExecutionResult::Success {
                output_size: 512,
                output_type: "text".to_string(),
            },
            execution_source: "persistence_test_2".to_string(),
            input_hash: "persistence_hash_2".to_string(),
            service_source: Some("persistence_service".to_string()),
            discovery_context: None,
        };
        
        tool_metrics.record_execution(record2).await;
        
        // Verify updated metrics
        let updated_metrics = tool_metrics.get_tool_metrics("persistent_tool").await.unwrap();
        assert_eq!(updated_metrics.total_executions, 2);
        assert_eq!(updated_metrics.successful_executions, 2);
        assert_eq!(updated_metrics.avg_execution_time_ms, 1750.0); // (1500 + 2000) / 2
        
        let updated_executions = tool_metrics.get_recent_executions(Some(10)).await;
        assert_eq!(updated_executions.len(), 2);
    }
}

/// Test error scenarios in metrics collection
#[tokio::test]
async fn test_metrics_error_scenarios() {
    let tool_metrics = ToolMetricsCollector::new(100);
    let thresholds = McpHealthThresholds::default();
    let mcp_metrics = McpMetricsCollector::new(thresholds);
    
    // Test recording execution for non-initialized tool (should auto-initialize)
    let record = ToolExecutionRecord {
        execution_id: Uuid::new_v4().to_string(),
        tool_name: "auto_init_tool".to_string(),
        start_time: Utc::now(),
        duration_ms: 1000,
        result: ToolExecutionResult::Success {
            output_size: 256,
            output_type: "json".to_string(),
        },
        execution_source: "error_test".to_string(),
        input_hash: "error_hash".to_string(),
        service_source: None,
        discovery_context: None,
    };
    
    tool_metrics.record_execution(record).await;
    
    // Should have auto-initialized the tool
    let metrics = tool_metrics.get_tool_metrics("auto_init_tool").await.unwrap();
    assert_eq!(metrics.total_executions, 1);
    assert_eq!(metrics.category, "unknown"); // Default category
    
    // Test getting metrics for non-existent tool
    let non_existent = tool_metrics.get_tool_metrics("does_not_exist").await;
    assert!(non_existent.is_none());
    
    // Test getting MCP metrics for non-existent service
    let non_existent_service = mcp_metrics.get_service_metrics("does_not_exist").await;
    assert!(non_existent_service.is_none());
    
    // Test with invalid file path for persistence
    let invalid_path = "/invalid/path/that/does/not/exist/metrics.json";
    let result = ToolMetricsCollector::new_with_storage(100, invalid_path).await;
    
    // Should handle the error gracefully and return an error
    assert!(result.is_err());
}

/// Test metrics data validation and sanitization
#[tokio::test]
async fn test_metrics_data_validation() {
    let tool_metrics = ToolMetricsCollector::new(100);
    
    // Test with extreme values
    let record = ToolExecutionRecord {
        execution_id: "test_id".to_string(),
        tool_name: "validation_tool".to_string(),
        start_time: Utc::now(),
        duration_ms: u64::MAX, // Extreme duration
        result: ToolExecutionResult::Success {
            output_size: usize::MAX, // Extreme output size
            output_type: "json".to_string(),
        },
        execution_source: "validation_test".to_string(),
        input_hash: "validation_hash".to_string(),
        service_source: None,
        discovery_context: Some(DiscoveryRanking {
            position: u32::MAX, // Extreme position
            confidence_score: f64::INFINITY, // Invalid confidence score
            discovery_method: "validation".to_string(),
            query: "validation test".to_string(),
            timestamp: Utc::now(),
        }),
    };
    
    // Should handle extreme values without panicking
    tool_metrics.record_execution(record).await;
    
    let metrics = tool_metrics.get_tool_metrics("validation_tool").await.unwrap();
    assert_eq!(metrics.total_executions, 1);
    // Implementation should handle extreme values appropriately
    assert!(metrics.avg_execution_time_ms.is_finite());
    
    // Test with empty/invalid strings
    let record2 = ToolExecutionRecord {
        execution_id: "".to_string(), // Empty ID
        tool_name: "".to_string(), // Empty tool name
        start_time: Utc::now(),
        duration_ms: 100,
        result: ToolExecutionResult::Success {
            output_size: 0,
            output_type: "".to_string(), // Empty type
        },
        execution_source: "".to_string(), // Empty source
        input_hash: "".to_string(), // Empty hash
        service_source: None,
        discovery_context: None,
    };
    
    // Should handle empty values appropriately
    tool_metrics.record_execution(record2).await;
}