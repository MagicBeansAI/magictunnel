//! Basic tests for Resource Management API endpoints
//!
//! This test suite validates the resource management API endpoints.

use actix_web::{test, web, App, HttpResponse};
use serde_json::{json, Value};
use std::sync::Arc;

use magictunnel::config::Config;
use magictunnel::mcp::{McpServer, resources::{ResourceManager, FileResourceProvider}};
use magictunnel::mcp::prompts::PromptManager;
use magictunnel::registry::RegistryService;
use magictunnel::web::dashboard::DashboardApi;

/// Helper function to create a basic test dashboard API
async fn create_test_dashboard_api() -> DashboardApi {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    let mcp_server = Arc::new(McpServer::new(config.registry.clone()).await.unwrap());
    let resource_manager = Arc::new(ResourceManager::new());
    let prompt_manager = Arc::new(PromptManager::new());

    // Add a test file provider to the resource manager
    if let Ok(file_provider) = FileResourceProvider::new("/tmp", "file://test".to_string()) {
        resource_manager.add_provider(Arc::new(file_provider)).await;
    }

    DashboardApi::new(
        registry,
        mcp_server,
        None, // External MCP integration
        resource_manager,
        prompt_manager,
        None, // Smart discovery service
    )
}

/// Test basic resource management status endpoint
#[tokio::test]
async fn test_resource_management_status_basic() {
    let dashboard_api = create_test_dashboard_api().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/resources/management/status",
                web::get().to(|api: web::Data<DashboardApi>| async move {
                    match api.get_resource_management_status().await {
                        Ok(response) => response,
                        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Failed to get status"}))
                    }
                })
            )
    ).await;

    let req = test::TestRequest::get()
        .uri("/dashboard/api/resources/management/status")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    
    // Basic validation that we get a response structure
    assert!(body.is_object());
    assert!(body.get("enabled").is_some());
    assert!(body.get("health_status").is_some());
    println!("Resource management status response: {}", serde_json::to_string_pretty(&body).unwrap());
}

/// Test basic resource listing endpoint
#[tokio::test]
async fn test_resource_list_basic() {
    let dashboard_api = create_test_dashboard_api().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/resources/management/resources",
                web::get().to(|api: web::Data<DashboardApi>, query: web::Query<magictunnel::web::dashboard::ResourceManagementQuery>| async move {
                    match api.get_resources_management(query).await {
                        Ok(response) => response,
                        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Failed to get resources"}))
                    }
                })
            )
    ).await;

    let req = test::TestRequest::get()
        .uri("/dashboard/api/resources/management/resources")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    
    // Basic validation that we get a resources response
    assert!(body.is_object());
    assert!(body.get("resources").is_some());
    assert!(body.get("total_count").is_some());
    println!("Resource list response: {}", serde_json::to_string_pretty(&body).unwrap());
}

/// Test basic resource providers endpoint
#[tokio::test]
async fn test_resource_providers_basic() {
    let dashboard_api = create_test_dashboard_api().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/resources/management/providers",
                web::get().to(|api: web::Data<DashboardApi>| async move {
                    match api.get_resource_providers().await {
                        Ok(response) => response,
                        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Failed to get providers"}))
                    }
                })
            )
    ).await;

    let req = test::TestRequest::get()
        .uri("/dashboard/api/resources/management/providers")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    
    // Basic validation that we get a providers response
    assert!(body.is_object());
    println!("Resource providers response: {}", serde_json::to_string_pretty(&body).unwrap());
}