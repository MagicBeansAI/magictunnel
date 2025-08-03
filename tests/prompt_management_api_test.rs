//! Basic tests for Prompt Management API endpoints
//!
//! This test suite validates the prompt management API endpoints.

use actix_web::{test, web, App, HttpResponse};
use serde_json::{json, Value};
use std::sync::Arc;

use magictunnel::config::Config;
use magictunnel::mcp::{McpServer, prompts::{PromptManager, InMemoryPromptProvider}};
use magictunnel::mcp::resources::ResourceManager;
use magictunnel::mcp::types::{PromptTemplate, PromptArgument};
use magictunnel::registry::RegistryService;
use magictunnel::web::dashboard::DashboardApi;

/// Helper function to create a basic test dashboard API
async fn create_test_dashboard_api() -> DashboardApi {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    let mcp_server = Arc::new(McpServer::new(config.registry.clone()).await.unwrap());
    let resource_manager = Arc::new(ResourceManager::new());
    let prompt_manager = Arc::new(PromptManager::new());

    // Add some test templates to the prompt manager
    let mut provider = InMemoryPromptProvider::new("test_provider".to_string());
    
    let test_template = PromptTemplate {
        name: "example_template".to_string(),
        description: Some("A test template for demonstration".to_string()),
        arguments: vec![
            PromptArgument {
                name: "user_name".to_string(),
                description: Some("The name of the user".to_string()),
                required: true,
            },
            PromptArgument {
                name: "context".to_string(),
                description: Some("Additional context".to_string()),
                required: false,
            },
        ],
    };
    
    let template_content = "Hello {user_name}! {context}".to_string();
    provider.add_template(test_template, template_content).unwrap();
    
    prompt_manager.add_provider(Arc::new(provider)).await;

    DashboardApi::new(
        registry,
        mcp_server,
        None, // External MCP integration
        resource_manager,
        prompt_manager,
        None, // Smart discovery service
    )
}

/// Test basic prompt management status endpoint
#[tokio::test]
async fn test_prompt_management_status_basic() {
    let dashboard_api = create_test_dashboard_api().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/prompts/management/status",
                web::get().to(|api: web::Data<DashboardApi>| async move {
                    match api.get_prompt_management_status().await {
                        Ok(response) => response,
                        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Failed to get status"}))
                    }
                })
            )
    ).await;

    let req = test::TestRequest::get()
        .uri("/dashboard/api/prompts/management/status")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    
    // Basic validation that we get a response structure
    assert!(body.is_object());
    println!("Prompt management status response: {}", serde_json::to_string_pretty(&body).unwrap());
}

/// Test basic prompt templates listing endpoint
#[tokio::test]
async fn test_prompt_templates_list_basic() {
    let dashboard_api = create_test_dashboard_api().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/prompts/management/templates",
                web::get().to(|api: web::Data<DashboardApi>, query: web::Query<magictunnel::web::dashboard::PromptTemplateManagementQuery>| async move {
                    match api.get_prompt_templates_management(query).await {
                        Ok(response) => response,
                        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Failed to get templates"}))
                    }
                })
            )
    ).await;

    let req = test::TestRequest::get()
        .uri("/dashboard/api/prompts/management/templates")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    
    // Basic validation that we get a templates response
    assert!(body.is_object());
    println!("Prompt templates list response: {}", serde_json::to_string_pretty(&body).unwrap());
}

/// Test basic prompt providers endpoint
#[tokio::test]
async fn test_prompt_providers_basic() {
    let dashboard_api = create_test_dashboard_api().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(dashboard_api))
            .route(
                "/dashboard/api/prompts/management/providers",
                web::get().to(|api: web::Data<DashboardApi>| async move {
                    match api.get_prompt_providers().await {
                        Ok(response) => response,
                        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Failed to get providers"}))
                    }
                })
            )
    ).await;

    let req = test::TestRequest::get()
        .uri("/dashboard/api/prompts/management/providers")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    
    // Basic validation that we get a providers response
    assert!(body.is_object());
    println!("Prompt providers response: {}", serde_json::to_string_pretty(&body).unwrap());
}