//! Integration tests for API key authentication

use actix_web::{test, web, App};
use magictunnel::config::{AuthConfig, AuthType, ApiKeyConfig, ApiKeyEntry};
use magictunnel::mcp::server::{health_check, list_tools_handler, call_tool_handler};
use magictunnel::mcp::types::ToolCall;
use magictunnel::mcp::server::McpServer;
use magictunnel::registry::service::RegistryService;
use magictunnel::config::RegistryConfig;
use serde_json::json;
use std::sync::Arc;

/// Create a test authentication configuration
fn create_test_auth_config() -> AuthConfig {
    let mut config = AuthConfig::default();
    config.enabled = true;
    config.r#type = AuthType::ApiKey;
    config.api_keys = Some(ApiKeyConfig {
        keys: vec![
            ApiKeyEntry::with_permissions(
                "test_admin_key_123456789".to_string(),
                "Test Admin Key".to_string(),
                vec!["read".to_string(), "write".to_string(), "admin".to_string()],
            ),
            ApiKeyEntry::with_permissions(
                "test_read_key_123456789".to_string(),
                "Test Read Key".to_string(),
                vec!["read".to_string()],
            ),
        ],
        require_header: true,
        header_name: "Authorization".to_string(),
        header_format: "Bearer {key}".to_string(),
    });
    config
}

/// Create a test registry service
async fn create_test_registry() -> Arc<RegistryService> {
    let registry_config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec!["capabilities/testing".to_string()],
        hot_reload: false,
        validation: magictunnel::config::ValidationConfig {
            strict: false,
            allow_unknown_fields: true,
        },
    };
    
    RegistryService::start_with_hot_reload(registry_config).await.unwrap()
}

#[actix_web::test]
async fn test_health_check_no_auth() {
    let app = test::init_service(
        App::new().route("/health", web::get().to(health_check))
    ).await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_list_tools_with_valid_api_key() {
    let auth_config = create_test_auth_config();
    let registry = create_test_registry().await;
    let mcp_server = Arc::new(
        McpServer::with_registry(registry.clone())
            .with_authentication(auth_config).unwrap()
    );

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(registry))
            .app_data(web::Data::new(mcp_server))
            .route("/mcp/tools", web::get().to(list_tools_handler))
    ).await;

    let req = test::TestRequest::get()
        .uri("/mcp/tools")
        .insert_header(("Authorization", "Bearer test_admin_key_123456789"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_list_tools_with_invalid_api_key() {
    let auth_config = create_test_auth_config();
    let registry = create_test_registry().await;
    let mcp_server = Arc::new(
        McpServer::with_registry(registry.clone())
            .with_authentication(auth_config).unwrap()
    );

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(registry))
            .app_data(web::Data::new(mcp_server))
            .route("/mcp/tools", web::get().to(list_tools_handler))
    ).await;

    let req = test::TestRequest::get()
        .uri("/mcp/tools")
        .insert_header(("Authorization", "Bearer invalid_key"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_list_tools_missing_api_key() {
    let auth_config = create_test_auth_config();
    let registry = create_test_registry().await;
    let mcp_server = Arc::new(
        McpServer::with_registry(registry.clone())
            .with_authentication(auth_config).unwrap()
    );

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(registry))
            .app_data(web::Data::new(mcp_server))
            .route("/mcp/tools", web::get().to(list_tools_handler))
    ).await;

    let req = test::TestRequest::get()
        .uri("/mcp/tools")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_call_tool_insufficient_permissions() {
    let auth_config = create_test_auth_config();
    let registry = create_test_registry().await;
    let mcp_server = Arc::new(
        McpServer::with_registry(registry.clone())
            .with_authentication(auth_config).unwrap()
    );

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(registry))
            .app_data(web::Data::new(mcp_server))
            .route("/mcp/call", web::post().to(call_tool_handler))
    ).await;

    let tool_call = ToolCall {
        name: "test_tool".to_string(),
        arguments: json!({}),
    };

    let req = test::TestRequest::post()
        .uri("/mcp/call")
        .insert_header(("Authorization", "Bearer test_read_key_123456789")) // Read-only key
        .set_json(&tool_call)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403); // Forbidden - insufficient permissions
}

#[actix_web::test]
async fn test_call_tool_with_admin_permissions() {
    let auth_config = create_test_auth_config();
    let registry = create_test_registry().await;
    let mcp_server = Arc::new(
        McpServer::with_registry(registry.clone())
            .with_authentication(auth_config).unwrap()
    );

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(registry))
            .app_data(web::Data::new(mcp_server))
            .route("/mcp/call", web::post().to(call_tool_handler))
    ).await;

    let tool_call = ToolCall {
        name: "test_tool".to_string(),
        arguments: json!({}),
    };

    let req = test::TestRequest::post()
        .uri("/mcp/call")
        .insert_header(("Authorization", "Bearer test_admin_key_123456789")) // Admin key
        .set_json(&tool_call)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should not be 401 or 403 (auth/permission errors)
    // Might be 400 if tool doesn't exist, but that's a different error
    assert_ne!(resp.status(), 401);
    assert_ne!(resp.status(), 403);
}

#[actix_web::test]
async fn test_disabled_authentication() {
    let mut auth_config = create_test_auth_config();
    auth_config.enabled = false; // Disable authentication
    
    let registry = create_test_registry().await;
    let mcp_server = Arc::new(
        McpServer::with_registry(registry.clone())
            .with_authentication(auth_config).unwrap()
    );

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(registry))
            .app_data(web::Data::new(mcp_server))
            .route("/mcp/tools", web::get().to(list_tools_handler))
    ).await;

    // Should work without any authentication header
    let req = test::TestRequest::get()
        .uri("/mcp/tools")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}
