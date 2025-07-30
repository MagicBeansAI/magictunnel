//! Dashboard API Tool Metrics Tests
//!
//! Tests for the dashboard API endpoints that expose tool metrics data.

use actix_web::{test, web, App, HttpResponse};
use chrono::Utc;
use magictunnel::{
    config::Config,
    discovery::SmartDiscoveryService,
    registry::RegistryService,
    web::dashboard::DashboardApi,
    metrics::tool_metrics::{
        ToolMetricsCollector, ToolExecutionRecord, ToolExecutionResult, DiscoveryRanking
    },
};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

/// Helper function to create a test DashboardApi with tool metrics
async fn create_test_dashboard_api() -> (DashboardApi, Arc<ToolMetricsCollector>) {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    // Create smart discovery service with tool metrics enabled
    let mut smart_discovery_config = magictunnel::discovery::SmartDiscoveryConfig::default();
    smart_discovery_config.tool_metrics_enabled = Some(true);
    let smart_discovery = SmartDiscoveryService::new(
        registry.clone(), 
        smart_discovery_config
    ).await.unwrap();
    
    // Get the tool metrics collector from the discovery service
    let final_tool_metrics = smart_discovery.tool_metrics().expect("Tool metrics should be enabled");
    
    // Create mock MCP server, resource manager, and prompt manager
    let mcp_server = Arc::new(magictunnel::mcp::McpServer::new(config.registry.clone()).await.unwrap());
    let resource_manager = Arc::new(magictunnel::mcp::resources::ResourceManager::new());
    let prompt_manager = Arc::new(magictunnel::mcp::prompts::PromptManager::new());
    
    let dashboard_api = DashboardApi::new(
        registry,
        mcp_server,
        None, // External MCP integration
        resource_manager,
        prompt_manager,
        Some(Arc::new(smart_discovery)),
    );
    
    (dashboard_api, final_tool_metrics)
}

/// Helper function to populate test data
async fn populate_test_metrics(collector: &ToolMetricsCollector) {
    // Initialize test tools with unique names to avoid conflicts
    collector.initialize_tool("test_ping_tool", "networking").await;
    collector.initialize_tool("test_file_tool", "filesystem").await;
    collector.initialize_tool("test_calc_tool", "utilities").await;
    
    // Add execution records for test_ping_tool (high performance)
    for i in 0..15 {
        let record = ToolExecutionRecord {
            execution_id: Uuid::new_v4().to_string(),
            tool_name: "test_ping_tool".to_string(),
            start_time: Utc::now(),
            duration_ms: 200 + i * 10,
            result: if i < 14 {
                ToolExecutionResult::Success {
                    output_size: 512,
                    output_type: "json".to_string(),
                }
            } else {
                ToolExecutionResult::Error {
                    error_type: "NetworkError".to_string(),
                    error_message: "Timeout".to_string(),
                    is_timeout: true,
                }
            },
            execution_source: "smart_discovery".to_string(),
            input_hash: format!("ping_hash_{}", i),
            service_source: None,
            discovery_context: Some(DiscoveryRanking {
                position: 1,
                confidence_score: 0.9 + (i as f64 * 0.005),
                discovery_method: "hybrid".to_string(),
                query: "ping server".to_string(),
                timestamp: Utc::now(),
            }),
        };
        collector.record_execution(record).await;
    }
    
    // Add execution records for test_file_tool (medium performance)
    for i in 0..8 {
        let record = ToolExecutionRecord {
            execution_id: Uuid::new_v4().to_string(),
            tool_name: "test_file_tool".to_string(),
            start_time: Utc::now(),
            duration_ms: 1000 + i * 50,
            result: if i < 6 {
                ToolExecutionResult::Success {
                    output_size: 1024,
                    output_type: "text".to_string(),
                }
            } else {
                ToolExecutionResult::Error {
                    error_type: "FileNotFound".to_string(),
                    error_message: "File does not exist".to_string(),
                    is_timeout: false,
                }
            },
            execution_source: "direct".to_string(),
            input_hash: format!("file_hash_{}", i),
            service_source: Some("filesystem_service".to_string()),
            discovery_context: Some(DiscoveryRanking {
                position: (2 + (i % 3)) as u32,
                confidence_score: 0.8 - (i as f64 * 0.02),
                discovery_method: "semantic".to_string(),
                query: "file operation".to_string(),
                timestamp: Utc::now(),
            }),
        };
        collector.record_execution(record).await;
    }
    
    // Add execution records for test_calc_tool (low usage)
    for i in 0..3 {
        let record = ToolExecutionRecord {
            execution_id: Uuid::new_v4().to_string(),
            tool_name: "test_calc_tool".to_string(),
            start_time: Utc::now(),
            duration_ms: 50 + i * 5,
            result: ToolExecutionResult::Success {
                output_size: 64,
                output_type: "json".to_string(),
            },
            execution_source: "api".to_string(),
            input_hash: format!("calc_hash_{}", i),
            service_source: None,
            discovery_context: Some(DiscoveryRanking {
                position: (5 + i) as u32,
                confidence_score: 0.7 + (i as f64 * 0.05),
                discovery_method: "rule_based".to_string(),
                query: "calculate".to_string(),
                timestamp: Utc::now(),
            }),
        };
        collector.record_execution(record).await;
    }
}

/// Test GET /dashboard/api/tool-metrics/summary endpoint
#[tokio::test]
async fn test_tool_metrics_summary_endpoint() {
    let (dashboard_api, tool_metrics) = create_test_dashboard_api().await;
    populate_test_metrics(&tool_metrics).await;
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/tool-metrics/summary",
                web::get().to(|api: web::Data<DashboardApi>| async move {
                    api.get_tool_metrics_summary().await.unwrap_or_else(|_| {
                        HttpResponse::InternalServerError().json(json!({"error": "Failed to get summary"}))
                    })
                })
            )
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/dashboard/api/tool-metrics/summary")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    
    // Verify summary structure
    assert!(body.get("total_tools").is_some());
    assert!(body.get("active_tools").is_some());
    assert!(body.get("total_executions").is_some());
    assert!(body.get("total_successful_executions").is_some());
    assert!(body.get("overall_success_rate").is_some());
    assert!(body.get("most_popular_tool").is_some());
    
    // Verify expected values (accounts for existing ping_globalping tool)
    assert!(body["total_tools"].as_u64().unwrap() >= 4); // At least our 3 test tools + ping_globalping
    assert!(body["active_tools"].as_u64().unwrap() >= 4);
    // Don't check exact execution counts since ping_globalping has variable data
    assert!(body["total_executions"].as_u64().unwrap() >= 26); // At least our test data
    assert!(body["total_successful_executions"].as_u64().unwrap() >= 23); // At least our test data
    // Most popular could be test_ping_tool or ping_globalping depending on existing data
}

/// Test GET /dashboard/api/tool-metrics/all endpoint
#[tokio::test]
async fn test_all_tool_metrics_endpoint() {
    let (dashboard_api, tool_metrics) = create_test_dashboard_api().await;
    populate_test_metrics(&tool_metrics).await;
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/tool-metrics/all",
                web::get().to(|api: web::Data<DashboardApi>| async move {
                    api.get_all_tool_metrics().await.unwrap_or_else(|_| {
                        HttpResponse::InternalServerError().json(json!({"error": "Failed to get all metrics"}))
                    })
                })
            )
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/dashboard/api/tool-metrics/all")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    
    // Verify response structure
    assert!(body.get("tool_metrics").is_some());
    assert!(body.get("total_tools").is_some());
    assert!(body.get("timestamp").is_some());
    
    let tool_metrics_obj = &body["tool_metrics"];
    assert!(tool_metrics_obj.is_object());
    
    // Verify all test tools are present
    assert!(tool_metrics_obj.get("test_ping_tool").is_some());
    assert!(tool_metrics_obj.get("test_file_tool").is_some());
    assert!(tool_metrics_obj.get("test_calc_tool").is_some());
    
    // Verify test_ping_tool metrics
    let ping_metrics = &tool_metrics_obj["test_ping_tool"];
    assert_eq!(ping_metrics["tool_name"], "test_ping_tool");
    assert_eq!(ping_metrics["category"], "networking");
    assert_eq!(ping_metrics["total_executions"], 15);
    assert_eq!(ping_metrics["successful_executions"], 14);
    assert_eq!(ping_metrics["failed_executions"], 1);
    assert_eq!(ping_metrics["top_30_appearances"], 15);
    assert_eq!(ping_metrics["top_10_appearances"], 15);
    assert_eq!(ping_metrics["top_3_appearances"], 15);
    
    // Verify test_file_tool metrics
    let file_metrics = &tool_metrics_obj["test_file_tool"];
    assert_eq!(file_metrics["tool_name"], "test_file_tool");
    assert_eq!(file_metrics["category"], "filesystem");
    assert_eq!(file_metrics["total_executions"], 8);
    assert_eq!(file_metrics["successful_executions"], 6);
    assert_eq!(file_metrics["failed_executions"], 2);
}

/// Test GET /dashboard/api/tool-metrics/top/{metric} endpoint
#[tokio::test]
async fn test_top_tools_endpoint() {
    let (dashboard_api, tool_metrics) = create_test_dashboard_api().await;
    populate_test_metrics(&tool_metrics).await;
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/tool-metrics/top/{metric}",
                web::get().to(|path: web::Path<String>, api: web::Data<DashboardApi>| async move {
                    let metric = path.into_inner();
                    api.get_top_tools(&metric, Some(10)).await.unwrap_or_else(|_| {
                        HttpResponse::InternalServerError().json(json!({"error": "Failed to get top tools"}))
                    })
                })
            )
    ).await;
    
    // Test top by executions
    let req = test::TestRequest::get()
        .uri("/dashboard/api/tool-metrics/top/executions")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    
    // Verify response structure
    assert!(body.get("top_tools").is_some());
    assert!(body.get("metric").is_some());
    assert!(body.get("limit").is_some());
    assert!(body.get("timestamp").is_some());
    
    let top_tools = body["top_tools"].as_array().unwrap();
    assert_eq!(body["metric"], "executions");
    
    // Verify that test_ping_tool appears with exact test data
    let ping_tool_entry = top_tools.iter().find(|tool| tool["tool_name"] == "test_ping_tool");
    assert!(ping_tool_entry.is_some());
    assert_eq!(ping_tool_entry.unwrap()["value"], 15.0);
    
    // Verify that test_file_tool and test_calc_tool appear with expected values
    let file_tool_entry = top_tools.iter().find(|tool| tool["tool_name"] == "test_file_tool");
    assert!(file_tool_entry.is_some());
    assert_eq!(file_tool_entry.unwrap()["value"], 8.0);
    
    let calc_tool_entry = top_tools.iter().find(|tool| tool["tool_name"] == "test_calc_tool");
    assert!(calc_tool_entry.is_some());
    assert_eq!(calc_tool_entry.unwrap()["value"], 3.0);
    
    // Test top by success_rate
    let req = test::TestRequest::get()
        .uri("/dashboard/api/tool-metrics/top/success_rate")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    let top_tools = body["top_tools"].as_array().unwrap();
    
    // calc_tool should be first (100% success rate with >10 executions filter might not apply)
    // ping_tool should have high success rate (14/15 = 0.933)
    // file_tool should have lower success rate (6/8 = 0.75)
    assert_eq!(body["metric"], "success_rate");
    assert!(top_tools.len() >= 2); // At least ping_tool and file_tool should qualify
}

/// Test GET /dashboard/api/tool-metrics/executions/recent endpoint
#[tokio::test]
async fn test_recent_executions_endpoint() {
    let (dashboard_api, tool_metrics) = create_test_dashboard_api().await;
    populate_test_metrics(&tool_metrics).await;
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/tool-metrics/executions/recent",
                web::get().to(|api: web::Data<DashboardApi>| async move {
                    api.get_recent_tool_executions(Some(10)).await.unwrap_or_else(|_| {
                        HttpResponse::InternalServerError().json(json!({"error": "Failed to get recent executions"}))
                    })
                })
            )
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/dashboard/api/tool-metrics/executions/recent")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    
    // Verify response structure
    assert!(body.get("executions").is_some());
    assert!(body.get("total").is_some());
    assert!(body.get("limit").is_some());
    assert!(body.get("timestamp").is_some());
    
    let executions = body["executions"].as_array().unwrap();
    assert!(body["total"].as_u64().unwrap() >= 26); // At least our test data
    assert_eq!(body["limit"], 10); // Requested limit
    assert_eq!(executions.len(), 10); // Should return 10 most recent
    
    // Verify execution structure
    let first_execution = &executions[0];
    assert!(first_execution.get("execution_id").is_some());
    assert!(first_execution.get("tool_name").is_some());
    assert!(first_execution.get("start_time").is_some());
    assert!(first_execution.get("duration_ms").is_some());
    assert!(first_execution.get("result").is_some());
    assert!(first_execution.get("execution_source").is_some());
    assert!(first_execution.get("input_hash").is_some());
    
    // Verify result structure
    let result = &first_execution["result"];
    assert!(result.get("Success").is_some() || result.get("Error").is_some() || result.get("Cancelled").is_some());
    
    // Verify discovery context if present
    if let Some(discovery_context) = first_execution.get("discovery_context") {
        assert!(discovery_context.get("position").is_some());
        assert!(discovery_context.get("confidence_score").is_some());
        assert!(discovery_context.get("discovery_method").is_some());
        assert!(discovery_context.get("query").is_some());
        assert!(discovery_context.get("timestamp").is_some());
    }
}

/// Test GET /dashboard/api/tool-metrics/{tool_name} endpoint
#[tokio::test]
async fn test_individual_tool_metrics_endpoint() {
    let (dashboard_api, tool_metrics) = create_test_dashboard_api().await;
    populate_test_metrics(&tool_metrics).await;
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/tool-metrics/{tool_name}",
                web::get().to(|path: web::Path<String>, api: web::Data<DashboardApi>| async move {
                    let tool_name = path.into_inner();
                    api.get_tool_metrics(&tool_name).await.unwrap_or_else(|_| {
                        HttpResponse::InternalServerError().json(json!({"error": "Failed to get tool metrics"}))
                    })
                })
            )
    ).await;
    
    // Test existing tool
    let req = test::TestRequest::get()
        .uri("/dashboard/api/tool-metrics/test_ping_tool")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    
    // Verify response structure
    assert!(body.get("metrics").is_some());
    assert!(body.get("tool_name").is_some());
    assert!(body.get("timestamp").is_some());
    
    let metrics = &body["metrics"];
    assert_eq!(metrics["tool_name"], "test_ping_tool");
    assert_eq!(metrics["category"], "networking");
    assert_eq!(metrics["total_executions"], 15);
    assert_eq!(metrics["successful_executions"], 14);
    assert_eq!(metrics["failed_executions"], 1);
    
    // Test non-existent tool
    let req = test::TestRequest::get()
        .uri("/dashboard/api/tool-metrics/non_existent_tool")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should return success with null/empty metrics or appropriate error
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

/// Test error handling in tool metrics endpoints
#[tokio::test]
async fn test_tool_metrics_error_handling() {
    // Create dashboard API without tool metrics (discovery service = None)
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    // Create mock MCP server, resource manager, and prompt manager
    let mcp_server = Arc::new(magictunnel::mcp::McpServer::new(config.registry.clone()).await.unwrap());
    let resource_manager = Arc::new(magictunnel::mcp::resources::ResourceManager::new());
    let prompt_manager = Arc::new(magictunnel::mcp::prompts::PromptManager::new());
    
    let dashboard_api = DashboardApi::new(
        registry,
        mcp_server,
        None, // External MCP integration
        resource_manager,
        prompt_manager,
        None, // No discovery service - should cause metrics to be unavailable
    );
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/tool-metrics/summary",
                web::get().to(|api: web::Data<DashboardApi>| async move {
                    api.get_tool_metrics_summary().await.unwrap_or_else(|_| {
                        HttpResponse::InternalServerError().json(json!({"error": "Failed to get summary"}))
                    })
                })
            )
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/dashboard/api/tool-metrics/summary")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    
    // Should return appropriate error or empty metrics when discovery service is not available
    assert!(
        body.get("error").is_some() || 
        (body.get("total_tools").is_some() && body["total_tools"] == 0)
    );
}

/// Test tool metrics API query parameters
#[tokio::test]
async fn test_tool_metrics_query_parameters() {
    let (dashboard_api, tool_metrics) = create_test_dashboard_api().await;
    populate_test_metrics(&tool_metrics).await;
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/tool-metrics/top/{metric}",
                web::get().to(|path: web::Path<String>, query: web::Query<std::collections::HashMap<String, String>>, api: web::Data<DashboardApi>| async move {
                    let metric = path.into_inner();
                    let limit = query.get("limit").and_then(|s| s.parse::<usize>().ok());
                    api.get_top_tools(&metric, limit).await.unwrap_or_else(|_| {
                        HttpResponse::InternalServerError().json(json!({"error": "Failed to get top tools"}))
                    })
                })
            )
            .route(
                "/dashboard/api/tool-metrics/executions/recent",
                web::get().to(|query: web::Query<std::collections::HashMap<String, String>>, api: web::Data<DashboardApi>| async move {
                    let limit = query.get("limit").and_then(|s| s.parse::<usize>().ok());
                    api.get_recent_tool_executions(limit).await.unwrap_or_else(|_| {
                        HttpResponse::InternalServerError().json(json!({"error": "Failed to get recent executions"}))
                    })
                })
            )
    ).await;
    
    // Test top tools with limit parameter
    let req = test::TestRequest::get()
        .uri("/dashboard/api/tool-metrics/top/executions?limit=2")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    let top_tools = body["top_tools"].as_array().unwrap();
    assert_eq!(body["limit"], 2);
    assert_eq!(top_tools.len(), 2);
    
    // Test recent executions with limit parameter
    let req = test::TestRequest::get()
        .uri("/dashboard/api/tool-metrics/executions/recent?limit=5")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: Value = test::read_body_json(resp).await;
    let executions = body["executions"].as_array().unwrap();
    assert_eq!(body["limit"], 5);
    assert_eq!(executions.len(), 5);
}