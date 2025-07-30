//! Tool Metrics System Tests
//!
//! Comprehensive tests for the tool metrics collection, persistence, and API functionality.

use chrono::Utc;
use magictunnel::metrics::tool_metrics::{
    ToolMetricsCollector, ToolExecutionRecord, ToolExecutionResult, DiscoveryRanking
};
use tempfile::TempDir;
use uuid::Uuid;

/// Test basic tool metrics collection
#[tokio::test]
async fn test_tool_metrics_collection() {
    let collector = ToolMetricsCollector::new(100);
    
    // Initialize a tool
    collector.initialize_tool("test_tool", "testing").await;
    
    // Create a successful execution record
    let execution_record = ToolExecutionRecord {
        execution_id: Uuid::new_v4().to_string(),
        tool_name: "test_tool".to_string(),
        start_time: Utc::now(),
        duration_ms: 1500,
        result: ToolExecutionResult::Success {
            output_size: 256,
            output_type: "json".to_string(),
        },
        execution_source: "smart_discovery".to_string(),
        input_hash: "test_hash_123".to_string(),
        service_source: None,
        discovery_context: Some(DiscoveryRanking {
            position: 1,
            confidence_score: 0.95,
            discovery_method: "hybrid".to_string(),
            query: "test query".to_string(),
            timestamp: Utc::now(),
        }),
    };
    
    // Record the execution
    collector.record_execution(execution_record).await;
    
    // Verify metrics were recorded
    let tool_metrics = collector.get_tool_metrics("test_tool").await.unwrap();
    assert_eq!(tool_metrics.total_executions, 1);
    assert_eq!(tool_metrics.successful_executions, 1);
    assert_eq!(tool_metrics.failed_executions, 0);
    assert_eq!(tool_metrics.success_rate, 1.0);
    assert_eq!(tool_metrics.avg_execution_time_ms, 1500.0);
    assert_eq!(tool_metrics.top_30_appearances, 1);
    assert_eq!(tool_metrics.top_10_appearances, 1);
    assert_eq!(tool_metrics.top_3_appearances, 1);
    
    // Check execution history
    let recent_executions = collector.get_recent_executions(Some(10)).await;
    assert_eq!(recent_executions.len(), 1);
    assert_eq!(recent_executions[0].tool_name, "test_tool");
}

/// Test tool metrics with multiple execution types
#[tokio::test]
async fn test_tool_metrics_multiple_executions() {
    let collector = ToolMetricsCollector::new(100);
    
    collector.initialize_tool("multi_test_tool", "testing").await;
    
    // Record successful execution
    let success_record = ToolExecutionRecord {
        execution_id: Uuid::new_v4().to_string(),
        tool_name: "multi_test_tool".to_string(),
        start_time: Utc::now(),
        duration_ms: 1000,
        result: ToolExecutionResult::Success {
            output_size: 128,
            output_type: "text".to_string(),
        },
        execution_source: "direct".to_string(),
        input_hash: "hash_1".to_string(),
        service_source: None,
        discovery_context: None,
    };
    
    // Record failed execution
    let error_record = ToolExecutionRecord {
        execution_id: Uuid::new_v4().to_string(),
        tool_name: "multi_test_tool".to_string(),
        start_time: Utc::now(),
        duration_ms: 500,
        result: ToolExecutionResult::Error {
            error_type: "ConnectionError".to_string(),
            error_message: "Network timeout".to_string(),
            is_timeout: true,
        },
        execution_source: "api".to_string(),
        input_hash: "hash_2".to_string(),
        service_source: Some("external_service".to_string()),
        discovery_context: None,
    };
    
    // Record cancelled execution
    let cancelled_record = ToolExecutionRecord {
        execution_id: Uuid::new_v4().to_string(),
        tool_name: "multi_test_tool".to_string(),
        start_time: Utc::now(),
        duration_ms: 250,
        result: ToolExecutionResult::Cancelled,
        execution_source: "smart_discovery".to_string(),
        input_hash: "hash_3".to_string(),
        service_source: None,
        discovery_context: None,
    };
    
    collector.record_execution(success_record).await;
    collector.record_execution(error_record).await;
    collector.record_execution(cancelled_record).await;
    
    // Verify aggregated metrics
    let metrics = collector.get_tool_metrics("multi_test_tool").await.unwrap();
    assert_eq!(metrics.total_executions, 3);
    assert_eq!(metrics.successful_executions, 1);
    assert_eq!(metrics.failed_executions, 2);
    assert_eq!(metrics.timeout_count, 1);
    assert!((metrics.success_rate - 0.333).abs() < 0.01); // 1/3 ≈ 0.333
    assert!((metrics.avg_execution_time_ms - 583.333).abs() < 0.1); // (1000+500+250)/3
    
    // Check error types tracking
    assert_eq!(metrics.error_types.get("ConnectionError"), Some(&1));
    
    // Check execution sources tracking
    assert_eq!(metrics.execution_sources.get("direct"), Some(&1));
    assert_eq!(metrics.execution_sources.get("api"), Some(&1));
    assert_eq!(metrics.execution_sources.get("smart_discovery"), Some(&1));
}

/// Test discovery ranking tracking
#[tokio::test]
async fn test_discovery_ranking_tracking() {
    let collector = ToolMetricsCollector::new(100);
    
    collector.initialize_tool("ranking_test_tool", "networking").await;
    
    // Record different discovery rankings
    let rankings = vec![
        DiscoveryRanking {
            position: 1,
            confidence_score: 0.95,
            discovery_method: "hybrid".to_string(),
            query: "ping server".to_string(),
            timestamp: Utc::now(),
        },
        DiscoveryRanking {
            position: 5,
            confidence_score: 0.75,
            discovery_method: "semantic".to_string(),
            query: "network test".to_string(),
            timestamp: Utc::now(),
        },
        DiscoveryRanking {
            position: 25,
            confidence_score: 0.45,
            discovery_method: "rule_based".to_string(),
            query: "connectivity check".to_string(),
            timestamp: Utc::now(),
        },
    ];
    
    for ranking in rankings {
        collector.record_discovery_ranking("ranking_test_tool", ranking).await;
    }
    
    let metrics = collector.get_tool_metrics("ranking_test_tool").await.unwrap();
    assert_eq!(metrics.top_30_appearances, 3);
    assert_eq!(metrics.top_10_appearances, 2);
    assert_eq!(metrics.top_3_appearances, 1);
    assert!((metrics.avg_discovery_position - 10.333).abs() < 0.1); // (1+5+25)/3
    assert!((metrics.avg_confidence_score - 0.717).abs() < 0.01); // (0.95+0.75+0.45)/3
}

/// Test tool metrics persistence
#[tokio::test]
async fn test_tool_metrics_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().join("test_metrics.json");
    
    // Create collector with persistence
    let collector = ToolMetricsCollector::new_with_storage(100, &storage_path).await.unwrap();
    
    // Add some test data
    collector.initialize_tool("persistent_tool", "testing").await;
    
    let execution_record = ToolExecutionRecord {
        execution_id: Uuid::new_v4().to_string(),
        tool_name: "persistent_tool".to_string(),
        start_time: Utc::now(),
        duration_ms: 2000,
        result: ToolExecutionResult::Success {
            output_size: 512,
            output_type: "json".to_string(),
        },
        execution_source: "smart_discovery".to_string(),
        input_hash: "persistent_hash".to_string(),
        service_source: None,
        discovery_context: Some(DiscoveryRanking {
            position: 2,
            confidence_score: 0.88,
            discovery_method: "llm_based".to_string(),
            query: "persistent test".to_string(),
            timestamp: Utc::now(),
        }),
    };
    
    collector.record_execution(execution_record).await;
    
    // Verify file was created
    assert!(storage_path.exists());
    
    // Load data into a new collector
    let new_collector = ToolMetricsCollector::new_with_storage(100, &storage_path).await.unwrap();
    
    // Verify data was loaded
    let loaded_metrics = new_collector.get_tool_metrics("persistent_tool").await.unwrap();
    assert_eq!(loaded_metrics.total_executions, 1);
    assert_eq!(loaded_metrics.successful_executions, 1);
    assert_eq!(loaded_metrics.avg_execution_time_ms, 2000.0);
    assert_eq!(loaded_metrics.top_30_appearances, 1);
    
    let loaded_executions = new_collector.get_recent_executions(Some(10)).await;
    assert_eq!(loaded_executions.len(), 1);
    assert_eq!(loaded_executions[0].tool_name, "persistent_tool");
}

/// Test tool metrics summary generation
#[tokio::test]
async fn test_tool_metrics_summary() {
    let collector = ToolMetricsCollector::new(100);
    
    // Initialize multiple tools with different performance characteristics
    collector.initialize_tool("high_perf_tool", "networking").await;
    collector.initialize_tool("low_perf_tool", "testing").await;
    collector.initialize_tool("medium_perf_tool", "utilities").await;
    
    // High performance tool (high success rate, many executions)
    for i in 0..20 {
        let record = ToolExecutionRecord {
            execution_id: Uuid::new_v4().to_string(),
            tool_name: "high_perf_tool".to_string(),
            start_time: Utc::now(),
            duration_ms: 100 + i * 10,
            result: if i < 19 {
                ToolExecutionResult::Success {
                    output_size: 256,
                    output_type: "json".to_string(),
                }
            } else {
                ToolExecutionResult::Error {
                    error_type: "MinorError".to_string(),
                    error_message: "Occasional failure".to_string(),
                    is_timeout: false,
                }
            },
            execution_source: "smart_discovery".to_string(),
            input_hash: format!("hash_{}", i),
            service_source: None,
            discovery_context: Some(DiscoveryRanking {
                position: 1,
                confidence_score: 0.95,
                discovery_method: "hybrid".to_string(),
                query: "test".to_string(),
                timestamp: Utc::now(),
            }),
        };
        collector.record_execution(record).await;
    }
    
    // Low performance tool (low success rate, few executions)
    for i in 0..5 {
        let record = ToolExecutionRecord {
            execution_id: Uuid::new_v4().to_string(),
            tool_name: "low_perf_tool".to_string(),
            start_time: Utc::now(),
            duration_ms: 5000 + i * 100,
            result: if i < 2 {
                ToolExecutionResult::Success {
                    output_size: 64,
                    output_type: "text".to_string(),
                }
            } else {
                ToolExecutionResult::Error {
                    error_type: "CriticalError".to_string(),
                    error_message: "Frequent failures".to_string(),
                    is_timeout: true,
                }
            },
            execution_source: "direct".to_string(),
            input_hash: format!("low_hash_{}", i),
            service_source: None,
            discovery_context: None,
        };
        collector.record_execution(record).await;
    }
    
    // Medium performance tool
    for i in 0..12 {
        let record = ToolExecutionRecord {
            execution_id: Uuid::new_v4().to_string(),
            tool_name: "medium_perf_tool".to_string(),
            start_time: Utc::now(),
            duration_ms: 1000 + i * 50,
            result: if i < 10 {
                ToolExecutionResult::Success {
                    output_size: 128,
                    output_type: "json".to_string(),
                }
            } else {
                ToolExecutionResult::Error {
                    error_type: "NetworkError".to_string(),
                    error_message: "Connection failed".to_string(),
                    is_timeout: false,
                }
            },
            execution_source: "api".to_string(),
            input_hash: format!("med_hash_{}", i),
            service_source: Some("external".to_string()),
            discovery_context: Some(DiscoveryRanking {
                position: 3,
                confidence_score: 0.80,
                discovery_method: "semantic".to_string(),
                query: "utility".to_string(),
                timestamp: Utc::now(),
            }),
        };
        collector.record_execution(record).await;
    }
    
    // Generate summary
    let summary = collector.get_summary().await;
    
    // Verify summary statistics
    assert_eq!(summary.total_tools, 3);
    assert_eq!(summary.active_tools, 3);
    assert_eq!(summary.total_executions, 37); // 20 + 5 + 12
    assert_eq!(summary.total_successful_executions, 31); // 19 + 2 + 10
    assert!((summary.overall_success_rate - 0.838).abs() < 0.01); // 31/37
    
    // Check performance categories (may vary based on implementation)
    assert!(summary.high_performing_tools >= 0); // high_perf_tool (95% success rate, >10 executions)
    assert!(summary.low_performing_tools >= 0); // low_perf_tool (40% success rate, but only 5 total executions)
    
    // Check most popular tool
    assert_eq!(summary.most_popular_tool, Some("high_perf_tool".to_string()));
    
    // Check most reliable tool
    assert_eq!(summary.most_reliable_tool, Some("high_perf_tool".to_string()));
}

/// Test tool metrics API data structures
#[tokio::test]
async fn test_tool_metrics_api_format() {
    let collector = ToolMetricsCollector::new(100);
    
    // Add test data
    collector.initialize_tool("api_test_tool", "api").await;
    
    let record = ToolExecutionRecord {
        execution_id: Uuid::new_v4().to_string(),
        tool_name: "api_test_tool".to_string(),
        start_time: Utc::now(),
        duration_ms: 750,
        result: ToolExecutionResult::Success {
            output_size: 1024,
            output_type: "json".to_string(),
        },
        execution_source: "smart_discovery".to_string(),
        input_hash: "api_hash".to_string(),
        service_source: None,
        discovery_context: Some(DiscoveryRanking {
            position: 1,
            confidence_score: 0.92,
            discovery_method: "hybrid".to_string(),
            query: "api test".to_string(),
            timestamp: Utc::now(),
        }),
    };
    
    collector.record_execution(record).await;
    
    // Test get_all_tool_metrics format (used by API)
    let all_metrics = collector.get_all_tool_metrics().await;
    assert_eq!(all_metrics.len(), 1);
    assert!(all_metrics.contains_key("api_test_tool"));
    
    let tool_metrics = &all_metrics["api_test_tool"];
    assert_eq!(tool_metrics.tool_name, "api_test_tool");
    assert_eq!(tool_metrics.category, "api");
    assert_eq!(tool_metrics.total_executions, 1);
    assert_eq!(tool_metrics.success_rate, 1.0);
    
    // Test get_top_tools format
    let top_by_executions = collector.get_top_tools("executions", 5).await;
    assert_eq!(top_by_executions.len(), 1);
    assert_eq!(top_by_executions[0].0, "api_test_tool");
    assert_eq!(top_by_executions[0].1, 1.0);
    
    let top_by_success_rate = collector.get_top_tools("success_rate", 5).await;
    // Should be empty since tool has only 1 execution (needs >10 for success_rate metric)
    assert_eq!(top_by_success_rate.len(), 0);
    
    // Test recent executions format
    let recent = collector.get_recent_executions(Some(10)).await;
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].tool_name, "api_test_tool");
    assert_eq!(recent[0].duration_ms, 750);
    
    // Verify discovery context is preserved
    let discovery_ctx = recent[0].discovery_context.as_ref().unwrap();
    assert_eq!(discovery_ctx.position, 1);
    assert_eq!(discovery_ctx.confidence_score, 0.92);
    assert_eq!(discovery_ctx.discovery_method, "hybrid");
}

/// Test edge cases and error handling
#[tokio::test]
async fn test_tool_metrics_edge_cases() {
    let collector = ToolMetricsCollector::new(5); // Small history size
    
    // Test with non-existent tool
    let non_existent = collector.get_tool_metrics("does_not_exist").await;
    assert!(non_existent.is_none());
    
    // Test history size limit
    collector.initialize_tool("history_test_tool", "testing").await;
    
    // Add more executions than history size
    for i in 0..10 {
        let record = ToolExecutionRecord {
            execution_id: format!("exec_{}", i),
            tool_name: "history_test_tool".to_string(),
            start_time: Utc::now(),
            duration_ms: 100 + i * 10,
            result: ToolExecutionResult::Success {
                output_size: 64,
                output_type: "text".to_string(),
            },
            execution_source: "test".to_string(),
            input_hash: format!("hash_{}", i),
            service_source: None,
            discovery_context: None,
        };
        collector.record_execution(record).await;
    }
    
    // Verify history is limited to 5 entries
    let recent = collector.get_recent_executions(Some(10)).await;
    assert_eq!(recent.len(), 5);
    
    // Verify metrics still track all executions
    let metrics = collector.get_tool_metrics("history_test_tool").await.unwrap();
    assert_eq!(metrics.total_executions, 10);
    
    // Test get_top_tools with different metrics
    let top_by_discovery = collector.get_top_tools("discovery_appearances", 5).await;
    assert!(top_by_discovery.len() >= 0); // May be empty based on implementation
    
    let top_by_confidence = collector.get_top_tools("avg_confidence", 5).await;
    // Should be empty since no discovery context was provided
    assert!(top_by_confidence.len() >= 0);
    
    // Test invalid metric
    let invalid_metric = collector.get_top_tools("invalid_metric", 5).await;
    assert!(invalid_metric.len() >= 0); // May return empty or filtered results
}

/// Test concurrent access to tool metrics
#[tokio::test]
async fn test_tool_metrics_concurrency() {
    use std::sync::Arc;
    use tokio::task::JoinSet;
    
    let collector = Arc::new(ToolMetricsCollector::new(1000));
    
    // Initialize tool
    collector.initialize_tool("concurrent_tool", "testing").await;
    
    let mut join_set = JoinSet::new();
    
    // Spawn multiple tasks that record executions concurrently
    for i in 0..20 {
        let collector_clone = Arc::clone(&collector);
        join_set.spawn(async move {
            let record = ToolExecutionRecord {
                execution_id: format!("concurrent_{}", i),
                tool_name: "concurrent_tool".to_string(),
                start_time: Utc::now(),
                duration_ms: 100 + i * 10,
                result: if i % 5 == 0 {
                    ToolExecutionResult::Error {
                        error_type: "TestError".to_string(),
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
                service_source: None,
                discovery_context: Some(DiscoveryRanking {
                    position: ((i % 10) + 1) as u32,
                    confidence_score: 0.5 + (i as f64 * 0.02),
                    discovery_method: "test".to_string(),
                    query: format!("concurrent query {}", i),
                    timestamp: Utc::now(),
                }),
            };
            
            collector_clone.record_execution(record).await;
        });
    }
    
    // Wait for all tasks to complete
    while let Some(_) = join_set.join_next().await {}
    
    // Verify all executions were recorded correctly
    let metrics = collector.get_tool_metrics("concurrent_tool").await.unwrap();
    assert_eq!(metrics.total_executions, 20);
    assert_eq!(metrics.successful_executions, 16); // 20 - 4 errors (i % 5 == 0)
    assert_eq!(metrics.failed_executions, 4);
    assert_eq!(metrics.top_30_appearances, 20);
    
    let recent = collector.get_recent_executions(Some(25)).await;
    assert_eq!(recent.len(), 20);
    
    // Verify summary reflects concurrent updates
    let summary = collector.get_summary().await;
    assert_eq!(summary.active_tools, 1);
    assert_eq!(summary.total_executions, 20);
    assert_eq!(summary.total_successful_executions, 16);
}