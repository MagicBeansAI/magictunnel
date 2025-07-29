//! JWT authentication integration tests

use actix_web::{test, web, App, HttpRequest, HttpResponse};
use magictunnel::auth::{AuthenticationMiddleware, JwtValidator, JwtUserInfo};
use magictunnel::config::{AuthConfig, AuthType, JwtConfig};
use magictunnel::mcp::server::McpServer;
use magictunnel::registry::service::RegistryService;
use magictunnel::config::RegistryConfig;
use serde_json::json;
use std::sync::Arc;

/// Create a test JWT configuration
fn create_test_jwt_config() -> AuthConfig {
    AuthConfig {
        enabled: true,
        r#type: AuthType::Jwt,
        api_keys: None,
        oauth: None,
        jwt: Some(JwtConfig {
            secret: "test_jwt_secret_key_that_is_at_least_32_characters_long".to_string(),
            algorithm: "HS256".to_string(),
            expiration: 3600, // 1 hour
            issuer: Some("test-issuer".to_string()),
            audience: Some("test-audience".to_string()),
        }),
    }
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

/// Simple health check handler for testing
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(json!({"status": "ok"}))
}

/// Simple tools list handler for testing
async fn list_tools_handler(
    req: HttpRequest,
    registry: web::Data<Arc<RegistryService>>,
    mcp_server: web::Data<Arc<McpServer>>,
) -> HttpResponse {
    // Check authentication with read permission
    if let Err(auth_error) = check_authentication(&req, mcp_server.auth_middleware(), "read").await {
        return auth_error;
    }

    let tools = registry.list_tools();
    HttpResponse::Ok().json(tools)
}

/// Helper function to check authentication for HTTP requests (copied from server.rs for testing)
async fn check_authentication(
    req: &HttpRequest,
    auth_middleware: &Option<Arc<AuthenticationMiddleware>>,
    required_permission: &str,
) -> std::result::Result<(), HttpResponse> {
    if let Some(auth) = auth_middleware {
        match auth.validate_http_request(req).await {
            Ok(Some(auth_result)) => {
                // Check if the authenticated user has the required permission
                if !auth.check_permission(&auth_result, required_permission) {
                    let error_response = json!({
                        "error": {
                            "code": "INSUFFICIENT_PERMISSIONS",
                            "message": format!("User does not have '{}' permission", required_permission),
                            "type": "authorization_error"
                        }
                    });
                    return Err(HttpResponse::Forbidden()
                        .content_type("application/json")
                        .json(error_response));
                }
                Ok(())
            }
            Ok(None) => {
                // Authentication disabled
                Ok(())
            }
            Err(e) => {
                let error_response = json!({
                    "error": {
                        "code": "AUTHENTICATION_FAILED",
                        "message": e.to_string(),
                        "type": "authentication_error"
                    }
                });
                Err(HttpResponse::Unauthorized()
                    .content_type("application/json")
                    .json(error_response))
            }
        }
    } else {
        // No authentication configured
        Ok(())
    }
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
async fn test_jwt_token_generation_and_validation() {
    let config = create_test_jwt_config();
    let jwt_config = config.jwt.as_ref().unwrap();
    let validator = JwtValidator::new(Some(jwt_config.clone())).unwrap();

    // Generate a test token
    let user_info = JwtUserInfo {
        id: "test_user_123".to_string(),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        roles: Some(vec!["user".to_string()]),
    };

    let permissions = vec!["read".to_string(), "write".to_string()];
    let token = validator.generate_token("test_user_123", permissions.clone(), Some(user_info.clone())).unwrap();

    // Validate the token
    let validation_result = validator.validate_token(&token, jwt_config).unwrap().unwrap();
    
    assert_eq!(validation_result.user_info.id, "test_user_123");
    assert_eq!(validation_result.user_info.email, Some("test@example.com".to_string()));
    assert_eq!(validation_result.permissions, permissions);
}

#[actix_web::test]
async fn test_list_tools_with_valid_jwt() {
    let auth_config = create_test_jwt_config();
    let jwt_config = auth_config.jwt.as_ref().unwrap();
    let validator = JwtValidator::new(Some(jwt_config.clone())).unwrap();
    
    // Generate a valid JWT token
    let user_info = JwtUserInfo {
        id: "test_admin".to_string(),
        email: Some("admin@example.com".to_string()),
        name: Some("Test Admin".to_string()),
        roles: Some(vec!["admin".to_string()]),
    };
    let permissions = vec!["read".to_string(), "write".to_string(), "admin".to_string()];
    let token = validator.generate_token("test_admin", permissions, Some(user_info)).unwrap();

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
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_list_tools_with_invalid_jwt() {
    let auth_config = create_test_jwt_config();
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
        .insert_header(("Authorization", "Bearer invalid_jwt_token"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_list_tools_missing_jwt() {
    let auth_config = create_test_jwt_config();
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

    // Request without Authorization header
    let req = test::TestRequest::get()
        .uri("/mcp/tools")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_jwt_token_in_query_parameter() {
    let auth_config = create_test_jwt_config();
    let jwt_config = auth_config.jwt.as_ref().unwrap();
    let validator = JwtValidator::new(Some(jwt_config.clone())).unwrap();
    
    // Generate a valid JWT token
    let user_info = JwtUserInfo {
        id: "test_user".to_string(),
        email: Some("user@example.com".to_string()),
        name: Some("Test User".to_string()),
        roles: Some(vec!["user".to_string()]),
    };
    let permissions = vec!["read".to_string()];
    let token = validator.generate_token("test_user", permissions, Some(user_info)).unwrap();

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

    // Test with token in query parameter
    let req = test::TestRequest::get()
        .uri(&format!("/mcp/tools?token={}", urlencoding::encode(&token)))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_disabled_jwt_authentication() {
    let mut auth_config = create_test_jwt_config();
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

    // Should work without any authentication header when disabled
    let req = test::TestRequest::get()
        .uri("/mcp/tools")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[cfg(test)]
mod jwt_middleware_tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[actix_web::test]
    async fn test_jwt_middleware_creation() {
        let config = create_test_jwt_config();
        let middleware = AuthenticationMiddleware::new(config).unwrap();
        assert!(middleware.is_logging_enabled());
    }

    #[tokio::test]
    async fn test_jwt_middleware_disabled_auth() {
        let mut config = create_test_jwt_config();
        config.enabled = false;
        let middleware = AuthenticationMiddleware::new(config).unwrap();

        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer some_jwt_token"))
            .to_http_request();

        let result = middleware.validate_http_request(&req).await.unwrap();
        assert!(result.is_none()); // Should return None when disabled
    }

    #[actix_web::test]
    async fn test_jwt_permission_check() {
        use magictunnel::auth::{JwtValidationResult, JwtUserInfo, AuthenticationResult};

        let config = create_test_jwt_config();
        let middleware = AuthenticationMiddleware::new(config).unwrap();

        let user_info = JwtUserInfo {
            id: "test_user".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            roles: Some(vec!["user".to_string()]),
        };

        let jwt_result = JwtValidationResult {
            claims: magictunnel::auth::JwtClaims {
                sub: "test_user".to_string(),
                iat: 1234567890,
                exp: 1234567890 + 3600,
                iss: Some("test-issuer".to_string()),
                aud: Some("test-audience".to_string()),
                permissions: Some(vec!["read".to_string(), "write".to_string()]),
                user_info: Some(user_info.clone()),
            },
            user_info,
            permissions: vec!["read".to_string(), "write".to_string()],
        };

        let auth_result = AuthenticationResult::Jwt(jwt_result);

        // Test permission checks
        assert!(middleware.check_permission(&auth_result, "read"));
        assert!(middleware.check_permission(&auth_result, "write"));
        assert!(!middleware.check_permission(&auth_result, "admin"));
    }
}
