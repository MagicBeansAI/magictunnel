//! Authentication middleware for MCP Proxy

use crate::auth::{ApiKeyValidator, JwtValidator, JwtValidationResult, OAuthValidator, OAuthValidationResult};
use crate::config::{AuthConfig, ApiKeyEntry};
use crate::error::{ProxyError, Result};
use crate::mcp::errors::McpErrorCode;
use crate::routing::middleware::{MiddlewareContext, RouterMiddleware};
use crate::routing::types::AgentResult;
use actix_web::{HttpRequest, HttpResponse};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Authentication result containing user information
#[derive(Debug, Clone)]
pub enum AuthenticationResult {
    /// API key authentication result
    ApiKey(ApiKeyEntry),
    /// OAuth authentication result
    OAuth(OAuthValidationResult),
    /// JWT authentication result
    Jwt(JwtValidationResult),
}

impl AuthenticationResult {
    /// Get user permissions based on authentication type
    pub fn get_permissions(&self) -> Vec<String> {
        match self {
            AuthenticationResult::ApiKey(key_entry) => key_entry.permissions.clone(),
            AuthenticationResult::OAuth(_oauth_result) => {
                // For OAuth, we'll use default permissions for now
                // In a real implementation, you might want to map OAuth scopes to permissions
                vec!["read".to_string(), "write".to_string()]
            }
            AuthenticationResult::Jwt(jwt_result) => jwt_result.permissions.clone(),
        }
    }

    /// Get user identifier
    pub fn get_user_id(&self) -> String {
        match self {
            AuthenticationResult::ApiKey(key_entry) => key_entry.name.clone(),
            AuthenticationResult::OAuth(oauth_result) => oauth_result.user_info.id.clone(),
            AuthenticationResult::Jwt(jwt_result) => jwt_result.user_info.id.clone(),
        }
    }
}

/// Authentication middleware for validating API keys and other auth methods
pub struct AuthenticationMiddleware {
    /// API key validator
    api_key_validator: ApiKeyValidator,
    /// OAuth validator
    oauth_validator: OAuthValidator,
    /// JWT validator
    jwt_validator: JwtValidator,
    /// Whether to log authentication events
    log_auth_events: bool,
}

impl AuthenticationMiddleware {
    /// Create new authentication middleware
    pub fn new(config: AuthConfig) -> Result<Self> {
        let jwt_validator = JwtValidator::new(config.jwt.clone())?;
        Ok(Self {
            api_key_validator: ApiKeyValidator::new(config.clone()),
            oauth_validator: OAuthValidator::new(config.clone()),
            jwt_validator,
            log_auth_events: true,
        })
    }

    /// Create new authentication middleware with logging configuration
    pub fn with_logging(config: AuthConfig, log_auth_events: bool) -> Result<Self> {
        let jwt_validator = JwtValidator::new(config.jwt.clone())?;
        Ok(Self {
            api_key_validator: ApiKeyValidator::new(config.clone()),
            oauth_validator: OAuthValidator::new(config.clone()),
            jwt_validator,
            log_auth_events,
        })
    }

    /// Validate authentication for an HTTP request
    pub async fn validate_http_request(&self, req: &HttpRequest) -> Result<Option<AuthenticationResult>> {
        // If authentication is disabled, allow all requests
        if !self.api_key_validator.is_enabled() {
            debug!("Authentication disabled, allowing request");
            return Ok(None);
        }

        let mut api_key_error: Option<crate::error::ProxyError> = None;
        let mut oauth_error: Option<crate::error::ProxyError> = None;
        let mut jwt_error: Option<crate::error::ProxyError> = None;

        // Try API key authentication first
        match self.api_key_validator.validate_request(req) {
            Ok(Some(key_entry)) => {
                if self.log_auth_events {
                    info!(
                        api_key_name = %key_entry.name,
                        permissions = ?key_entry.permissions,
                        auth_type = "api_key",
                        "Authentication successful"
                    );
                }
                return Ok(Some(AuthenticationResult::ApiKey(key_entry)));
            }
            Ok(None) => {
                // API key auth is disabled or not configured, try OAuth
                debug!("API key authentication not configured, trying OAuth");
            }
            Err(e) => {
                // API key validation failed, store error and try OAuth as fallback
                debug!("API key validation failed, trying OAuth as fallback");
                api_key_error = Some(e);
            }
        }

        // Try OAuth authentication
        match self.oauth_validator.validate_request(req).await {
            Ok(Some(oauth_result)) => {
                if self.log_auth_events {
                    info!(
                        user_id = %oauth_result.user_info.id,
                        user_email = ?oauth_result.user_info.email,
                        auth_type = "oauth",
                        "OAuth authentication successful"
                    );
                }
                return Ok(Some(AuthenticationResult::OAuth(oauth_result)));
            }
            Ok(None) => {
                debug!("OAuth authentication disabled or not configured, trying JWT");
            }
            Err(e) => {
                debug!("OAuth authentication failed, trying JWT as fallback");
                oauth_error = Some(e);
            }
        }

        // Try JWT authentication
        match self.jwt_validator.validate_request(req) {
            Ok(Some(jwt_result)) => {
                if self.log_auth_events {
                    info!(
                        user_id = %jwt_result.user_info.id,
                        user_email = ?jwt_result.user_info.email,
                        auth_type = "jwt",
                        "JWT authentication successful"
                    );
                }
                return Ok(Some(AuthenticationResult::Jwt(jwt_result)));
            }
            Ok(None) => {
                debug!("JWT authentication disabled or not configured");
            }
            Err(e) => {
                debug!("JWT authentication failed");
                jwt_error = Some(e);
            }
        }

        // If we reach here, all authentication methods failed or are not configured
        // Determine which error to return based on what was attempted
        let errors = [
            ("API key", api_key_error.as_ref()),
            ("OAuth", oauth_error.as_ref()),
            ("JWT", jwt_error.as_ref()),
        ];

        // Find the first error to return (prioritize API key, then OAuth, then JWT)
        for (auth_type, error_opt) in &errors {
            if let Some(error) = error_opt {
                if self.log_auth_events {
                    warn!(
                        error = %error,
                        auth_type = auth_type,
                        remote_addr = ?req.connection_info().peer_addr(),
                        user_agent = ?req.headers().get("user-agent"),
                        "Authentication failed"
                    );
                }
                return Err((*error).clone());
            }
        }

        // All methods are not configured, but authentication is enabled
        // This shouldn't happen in normal configuration, but handle gracefully
        debug!("Authentication enabled but no authentication methods configured");
        Ok(None)
    }

    /// Check if an authenticated user has a specific permission
    pub fn check_permission(&self, auth_result: &AuthenticationResult, permission: &str) -> bool {
        let has_permission = match auth_result {
            AuthenticationResult::ApiKey(key_entry) => {
                self.api_key_validator.check_permission(key_entry, permission)
            }
            AuthenticationResult::OAuth(_oauth_result) => {
                // For OAuth, check against default permissions
                // In a real implementation, you might want to map OAuth scopes to permissions
                let permissions = auth_result.get_permissions();
                permissions.contains(&permission.to_string())
            }
            AuthenticationResult::Jwt(jwt_result) => {
                self.jwt_validator.check_permission(jwt_result, permission)
            }
        };

        if self.log_auth_events {
            debug!(
                user_id = %auth_result.get_user_id(),
                permission = %permission,
                granted = has_permission,
                "Permission check"
            );
        }

        has_permission
    }

    /// Create an authentication error response for HTTP endpoints
    pub fn create_auth_error_response(&self, error: &ProxyError) -> HttpResponse {
        let error_body = json!({
            "error": {
                "code": "AUTHENTICATION_FAILED",
                "message": error.to_string(),
                "type": "authentication_error"
            }
        });

        HttpResponse::Unauthorized()
            .content_type("application/json")
            .json(error_body)
    }

    /// Create an authentication error response for MCP protocol
    pub fn create_mcp_auth_error(&self, request_id: Option<&serde_json::Value>, error: &ProxyError) -> String {
        let response = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {
                "code": McpErrorCode::InvalidRequest as i32,
                "message": format!("Authentication failed: {}", error),
                "data": {
                    "type": "authentication_error",
                    "details": error.to_string()
                }
            }
        });

        response.to_string()
    }

    /// Get OAuth authorization URL (for OAuth endpoints)
    pub fn get_oauth_authorization_url(&self, redirect_uri: &str, state: &str) -> Result<String> {
        self.oauth_validator.get_authorization_url(redirect_uri, state)
    }

    /// Exchange OAuth authorization code for token (for OAuth endpoints)
    pub async fn exchange_oauth_code_for_token(&self, code: &str, redirect_uri: &str) -> Result<crate::auth::OAuthTokenResponse> {
        self.oauth_validator.exchange_code_for_token(code, redirect_uri).await
    }

    /// Check if authentication event logging is enabled (for testing)
    pub fn is_logging_enabled(&self) -> bool {
        self.log_auth_events
    }
}

#[async_trait]
impl RouterMiddleware for AuthenticationMiddleware {
    async fn before_execution(&self, context: &MiddlewareContext) -> Result<()> {
        // For now, we'll skip authentication in the router middleware
        // since we don't have access to the HTTP request context here.
        // Authentication will be handled at the HTTP handler level.
        
        if self.log_auth_events {
            debug!(
                execution_id = %context.execution_id,
                tool_name = %context.tool_call.name,
                "Router middleware: Authentication check (HTTP-level auth required)"
            );
        }

        Ok(())
    }

    async fn after_execution(&self, context: &MiddlewareContext, result: &AgentResult) -> Result<()> {
        if self.log_auth_events {
            debug!(
                execution_id = %context.execution_id,
                tool_name = %context.tool_call.name,
                success = result.success,
                "Router middleware: Tool execution completed"
            );
        }

        Ok(())
    }

    async fn on_error(&self, context: &MiddlewareContext, error: &ProxyError) -> Result<()> {
        if self.log_auth_events {
            error!(
                execution_id = %context.execution_id,
                tool_name = %context.tool_call.name,
                error = %error,
                "Router middleware: Tool execution failed"
            );
        }

        Ok(())
    }
}

/// HTTP middleware function for Actix-web integration
pub async fn auth_middleware_fn(
    req: actix_web::dev::ServiceRequest,
    auth_middleware: Arc<AuthenticationMiddleware>,
) -> std::result::Result<actix_web::dev::ServiceRequest, actix_web::Error> {
    match auth_middleware.validate_http_request(req.request()).await {
        Ok(_) => Ok(req),
        Err(e) => {
            let error_message = format!("Authentication failed: {}", e);
            Err(actix_web::error::ErrorUnauthorized(error_message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ApiKeyConfig, ApiKeyEntry, AuthType};
    use actix_web::test::TestRequest;

    fn create_test_auth_config() -> AuthConfig {
        let mut config = AuthConfig::default();
        config.enabled = true;
        config.r#type = AuthType::ApiKey;
        config.api_keys = Some(ApiKeyConfig {
            keys: vec![
                ApiKeyEntry::with_permissions(
                    "admin_key_123456789".to_string(),
                    "Admin Key".to_string(),
                    vec!["read".to_string(), "write".to_string(), "admin".to_string()],
                ),
                ApiKeyEntry::with_permissions(
                    "read_only_key_123456789".to_string(),
                    "Read Only Key".to_string(),
                    vec!["read".to_string()],
                ),
            ],
            require_header: true,
            header_name: "Authorization".to_string(),
            header_format: "Bearer {key}".to_string(),
        });
        config
    }

    #[test]
    fn test_middleware_creation() {
        let config = create_test_auth_config();
        let middleware = AuthenticationMiddleware::new(config).unwrap();
        assert!(middleware.log_auth_events);
    }

    #[tokio::test]
    async fn test_http_request_validation_success() {
        let config = create_test_auth_config();
        let middleware = AuthenticationMiddleware::new(config).unwrap();

        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer admin_key_123456789"))
            .to_http_request();

        let result = middleware.validate_http_request(&req).await.unwrap();
        assert!(result.is_some());

        match result.unwrap() {
            AuthenticationResult::ApiKey(key_entry) => {
                assert_eq!(key_entry.name, "Admin Key");
            }
            _ => panic!("Expected API key authentication result"),
        }
    }

    #[test]
    fn test_permission_check() {
        let config = create_test_auth_config();
        let middleware = AuthenticationMiddleware::new(config).unwrap();

        let admin_key = ApiKeyEntry::with_permissions(
            "admin_key".to_string(),
            "Admin".to_string(),
            vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        );

        let read_only_key = ApiKeyEntry::with_permissions(
            "read_key".to_string(),
            "Read Only".to_string(),
            vec!["read".to_string()],
        );

        let admin_auth = AuthenticationResult::ApiKey(admin_key);
        let read_only_auth = AuthenticationResult::ApiKey(read_only_key);

        assert!(middleware.check_permission(&admin_auth, "admin"));
        assert!(middleware.check_permission(&admin_auth, "read"));
        assert!(middleware.check_permission(&admin_auth, "write"));

        assert!(middleware.check_permission(&read_only_auth, "read"));
        assert!(!middleware.check_permission(&read_only_auth, "write"));
        assert!(!middleware.check_permission(&read_only_auth, "admin"));
    }
}
