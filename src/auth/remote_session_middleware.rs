//! Remote Session Authentication Middleware
//!
//! This middleware provides comprehensive authentication and session isolation for
//! multi-deployment scenarios. It prevents session collisions and token conflicts
//! by properly identifying and isolating remote clients connecting to MagicTunnel instances.

use crate::{auth::{
    AuthenticationResult, ClientIdentityExtractor, ExtendedMcpInitRequest, IsolatedSession, IsolatedSessionManager, RemoteUserContext, SessionIsolationConfig, TokenStorage, UserContext
}};
use crate::error::ProxyError;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error as ActixError, HttpMessage, HttpRequest, HttpResponse,
};
use futures_util::future::{ok, Ready, LocalBoxFuture};
use std::{collections::HashMap, rc::Rc, sync::Arc};
use tracing::{debug, error, info, trace};
use crate::error::Result;

/// Remote session authentication middleware
pub struct RemoteSessionMiddleware {
    /// Session manager for isolation
    session_manager: Arc<IsolatedSessionManager>,
    
    /// Client identity extractor
    identity_extractor: Arc<ClientIdentityExtractor>,
    
    /// Middleware configuration
    config: RemoteSessionConfig,
}

/// Configuration for remote session middleware
#[derive(Debug, Clone)]
pub struct RemoteSessionConfig {
    /// Enable remote session isolation
    pub enable_isolation: bool,
    
    /// Require client identity validation
    pub require_client_identity: bool,
    
    /// Enable automatic session cleanup
    pub enable_auto_cleanup: bool,
    
    /// Session cleanup interval in seconds
    pub cleanup_interval_seconds: u64,
    
    /// Paths that bypass authentication
    pub bypass_paths: Vec<String>,
    
    /// Enable session recovery
    pub enable_session_recovery: bool,
    
    /// Maximum session recovery attempts
    pub max_recovery_attempts: u32,
}

/// Session request context
#[derive(Debug, Clone)]
pub struct SessionRequestContext {
    /// Session ID if available
    pub session_id: Option<String>,
    
    /// Remote user context
    pub remote_context: Option<RemoteUserContext>,
    
    /// Authentication result
    pub auth_result: Option<AuthenticationResult>,
    
    /// Request validation result
    pub validation_result: RequestValidationResult,
}

/// Request validation result
#[derive(Debug, Clone)]
pub struct RequestValidationResult {
    /// Whether request is valid
    pub valid: bool,
    
    /// Validation score (0.0 - 1.0)
    pub score: f64,
    
    /// Validation issues
    pub issues: Vec<String>,
    
    /// Security recommendations
    pub recommendations: Vec<String>,
    
    /// Should request be blocked
    pub should_block: bool,
}

impl Default for RemoteSessionConfig {
    fn default() -> Self {
        Self {
            enable_isolation: true,
            require_client_identity: false,
            enable_auto_cleanup: true,
            cleanup_interval_seconds: 900, // 15 minutes
            bypass_paths: vec![
                "/health".to_string(),
                "/metrics".to_string(),
                "/dashboard".to_string(),
            ],
            enable_session_recovery: true,
            max_recovery_attempts: 3,
        }
    }
}

impl RemoteSessionMiddleware {
    /// Create new remote session middleware
    pub fn new(
        session_config: SessionIsolationConfig,
        middleware_config: RemoteSessionConfig,
    ) -> Self {
        let session_manager = Arc::new(IsolatedSessionManager::new(session_config));
        let identity_extractor = Arc::new(ClientIdentityExtractor::new());

        Self {
            session_manager,
            identity_extractor,
            config: middleware_config,
        }
    }

    /// Create middleware with custom components
    pub fn with_components(
        session_manager: Arc<IsolatedSessionManager>,
        identity_extractor: Arc<ClientIdentityExtractor>,
        config: RemoteSessionConfig,
    ) -> Self {
        Self {
            session_manager,
            identity_extractor,
            config,
        }
    }

    /// Process request with remote session isolation
    async fn process_request(&self, req: &HttpRequest) -> Result<SessionRequestContext> {
        debug!("Processing request with remote session middleware");

        // Check if path should bypass authentication
        if self.should_bypass_path(req.path()) {
            trace!("Bypassing authentication for path: {}", req.path());
            return Ok(SessionRequestContext {
                session_id: None,
                remote_context: None,
                auth_result: None,
                validation_result: RequestValidationResult {
                    valid: true,
                    score: 1.0,
                    issues: vec![],
                    recommendations: vec![],
                    should_block: false,
                },
            });
        }

        // Try to extract or recover existing session
        let session_context = if let Some(session_id) = self.extract_session_id(req) {
            if let Some(session) = self.session_manager.get_session(&session_id) {
                // Update session activity
                let _ = self.session_manager.update_activity(&session_id);
                
                Some((session_id, session))
            } else if self.config.enable_session_recovery {
                // Attempt session recovery
                self.attempt_session_recovery(req).await?
            } else {
                None
            }
        } else {
            None
        };

        // If no existing session, create new one
        let (session_id, session) = if let Some((id, session)) = session_context {
            (id, session)
        } else {
            self.create_new_session(req).await?
        };

        // Validate request against session context
        let validation_result = self.validate_request(req, &session).await?;

        Ok(SessionRequestContext {
            session_id: Some(session_id),
            remote_context: Some(session.remote_context.clone()),
            auth_result: session.auth_result.clone(),
            validation_result,
        })
    }

    /// Create new session for request
    async fn create_new_session(&self, req: &HttpRequest) -> Result<(String, IsolatedSession)> {
        debug!("Creating new session for request");

        // Extract MCP initialization data if available
        let mcp_init = self.extract_mcp_init_data(req).await.ok();
        
        // Extract client identity
        let identity_result = self.identity_extractor
            .extract_client_identity(req, mcp_init.as_ref())?;

        // Validate client identity if required
        if self.config.require_client_identity {
            self.identity_extractor.validate_extraction_result(&identity_result)?;
        }

        // Create remote user context
        let local_context = UserContext::new()?;
        let mcp_client_info = mcp_init
            .as_ref()
            .map(|init| &init.standard_init);
            
        let remote_context = RemoteUserContext::new(
            local_context,
            req,
            mcp_client_info,
        )?;

        // Create isolated session
        let session_id = self.session_manager.create_session(req, remote_context).await?;
        let session = self.session_manager.get_session(&session_id)
            .ok_or_else(|| ProxyError::auth("Failed to retrieve created session".to_string()))?;

        info!("Created new session: {} for client: {}", session_id, session.remote_context.display_name());
        Ok((session_id, session))
    }

    /// Attempt to recover an existing session
    async fn attempt_session_recovery(&self, req: &HttpRequest) -> Result<Option<(String, IsolatedSession)>> {
        debug!("Attempting session recovery");

        // Try to extract recovery token or persistent session ID
        let recovery_token = req.headers()
            .get("x-session-recovery-token")
            .and_then(|h| h.to_str().ok());

        if let Some(token) = recovery_token {
            // TODO: Implement session recovery logic
            // This would involve checking a persistent store for session data
            // based on the recovery token
            trace!("Session recovery token found: {}", token);
        }

        // For now, return None to indicate no recovery possible
        Ok(None)
    }

    /// Validate request against session context
    async fn validate_request(
        &self,
        req: &HttpRequest,
        session: &IsolatedSession,
    ) -> Result<RequestValidationResult> {
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        let mut score = 1.0;

        // Validate client identity consistency
        let connection_info = req.connection_info();
        let current_client_ip = connection_info
            .realip_remote_addr()
            .unwrap_or("127.0.0.1:0")
            .split(':')
            .next()
            .unwrap_or("127.0.0.1");

        if current_client_ip != session.remote_context.client_identity.client_ip.to_string() {
            issues.push("Client IP address mismatch".to_string());
            score -= 0.3;
            recommendations.push("Verify client identity".to_string());
        }

        // Validate session state
        match session.state {
            crate::auth::IsolationSessionState::Expired => {
                issues.push("Session has expired".to_string());
                score = 0.0;
                recommendations.push("Create new session".to_string());
            }
            crate::auth::IsolationSessionState::Suspended => {
                issues.push("Session is suspended".to_string());
                score -= 0.5;
                recommendations.push("Resume or create new session".to_string());
            }
            crate::auth::IsolationSessionState::Terminated => {
                issues.push("Session is terminated".to_string());
                score = 0.0;
                recommendations.push("Create new session".to_string());
            }
            _ => {}
        }

        // Validate user agent consistency if available
        if let Some(session_ua) = &session.remote_context.client_identity.user_agent {
            if let Some(current_ua) = req.headers().get("user-agent").and_then(|h| h.to_str().ok()) {
                if session_ua != current_ua {
                    issues.push("User agent mismatch".to_string());
                    score -= 0.1;
                }
            }
        }

        // Check session activity
        if session.remote_context.is_session_expired(2) { // 2 hours inactivity
            issues.push("Session inactive for too long".to_string());
            score -= 0.2;
            recommendations.push("Refresh session activity".to_string());
        }

        let should_block = score < 0.3 || issues.iter().any(|issue| 
            issue.contains("expired") || issue.contains("terminated")
        );

        Ok(RequestValidationResult {
            valid: score >= 0.5,
            score,
            issues,
            recommendations,
            should_block,
        })
    }

    /// Extract session ID from request
    fn extract_session_id(&self, req: &HttpRequest) -> Option<String> {
        // Try multiple sources for session ID
        
        // 1. Authorization header
        if let Some(auth_header) = req.headers().get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if let Some(session_id) = auth_str.strip_prefix("Session ") {
                    return Some(session_id.to_string());
                }
            }
        }

        // 2. X-Session-ID header
        if let Some(session_header) = req.headers().get("x-session-id") {
            if let Ok(session_str) = session_header.to_str() {
                return Some(session_str.to_string());
            }
        }

        // 3. Query parameter
        if let Some(query_str) = req.uri().query() {
            for pair in query_str.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    if key == "session_id" {
                        return Some(value.to_string());
                    }
                }
            }
        }

        None
    }

    /// Extract MCP initialization data from request
    async fn extract_mcp_init_data(&self, _req: &HttpRequest) -> Result<ExtendedMcpInitRequest> {
        // TODO: Extract MCP init data from request body or headers
        // For now, return a minimal init structure
        Ok(ExtendedMcpInitRequest {
            standard_init: HashMap::new(),
            client_identity: None,
            security_context: None,
        })
    }

    /// Check if path should bypass authentication
    fn should_bypass_path(&self, path: &str) -> bool {
        self.config.bypass_paths.iter().any(|bypass_path| {
            path.starts_with(bypass_path)
        })
    }

    /// Create authentication error response
    fn create_auth_error_response(message: &str, details: Option<&RequestValidationResult>) -> HttpResponse {
        let mut response_body = serde_json::json!({
            "error": "authentication_failed",
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        if let Some(validation) = details {
            response_body["validation"] = serde_json::json!({
                "score": validation.score,
                "issues": validation.issues,
                "recommendations": validation.recommendations,
            });
        }

        HttpResponse::Unauthorized()
            .content_type("application/json")
            .json(response_body)
    }
}

/// Middleware transform for remote session authentication
impl<S> Transform<S, ServiceRequest> for RemoteSessionMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = ActixError> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = ActixError;
    type InitError = ();
    type Transform = RemoteSessionMiddlewareService<S>;
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RemoteSessionMiddlewareService {
            service: Rc::new(service),
            session_manager: self.session_manager.clone(),
            identity_extractor: self.identity_extractor.clone(),
            config: self.config.clone(),
        })
    }
}

/// Middleware service implementation
pub struct RemoteSessionMiddlewareService<S> {
    service: Rc<S>,
    session_manager: Arc<IsolatedSessionManager>,
    identity_extractor: Arc<ClientIdentityExtractor>,
    config: RemoteSessionConfig,
}

impl<S> Service<ServiceRequest> for RemoteSessionMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = ActixError> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = ActixError;
    type Future = LocalBoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::result::Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let session_manager = self.session_manager.clone();
        let identity_extractor = self.identity_extractor.clone();
        let config = self.config.clone();

        Box::pin(async move {
            // Create temporary middleware instance for processing
            let middleware = RemoteSessionMiddleware {
                session_manager,
                identity_extractor,
                config,
            };

            // Process request with session middleware
            match middleware.process_request(req.request()).await {
                Ok(session_context) => {
                    // Check if request should be blocked
                    if session_context.validation_result.should_block {
                        let response = RemoteSessionMiddleware::create_auth_error_response(
                            "Request blocked due to validation failures",
                            Some(&session_context.validation_result)
                        );
                        
                        return Ok(req.into_response(response));
                    }

                    // Add session context to request extensions
                    req.extensions_mut().insert(session_context);

                    // Continue with the request
                    service.call(req).await
                }
                Err(e) => {
                    error!("Remote session middleware error: {}", e);
                    
                    let response = RemoteSessionMiddleware::create_auth_error_response(
                        &format!("Authentication error: {}", e),
                        None
                    );
                    
                    Ok(req.into_response(response))
                }
            }
        })
    }
}

/// Helper function to extract session context from request extensions
pub fn get_session_context(req: &HttpRequest) -> Option<SessionRequestContext> {
    req.extensions().get::<SessionRequestContext>().cloned()
}

/// Helper function to get remote user context from request
pub fn get_remote_user_context(req: &HttpRequest) -> Option<RemoteUserContext> {
    get_session_context(req)?.remote_context
}

/// Helper function to check if request is authenticated
pub fn is_request_authenticated(req: &HttpRequest) -> bool {
    get_session_context(req)
        .map(|ctx| ctx.auth_result.is_some())
        .unwrap_or(false)
}

/// Helper function to get session-specific token storage
pub async fn get_session_token_storage(req: &HttpRequest) -> Option<Arc<TokenStorage>> {
    let session_context = get_session_context(req)?;
    let _session_id = session_context.session_id.as_ref()?;
    
    // This would need access to the session manager to get the full session
    // For now, we'll return None
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    async fn test_handler(req: HttpRequest) -> HttpResponse {
        if let Some(session_context) = get_session_context(&req) {
            HttpResponse::Ok().json(serde_json::json!({
                "session_id": session_context.session_id,
                "authenticated": session_context.auth_result.is_some(),
                "validation_score": session_context.validation_result.score,
            }))
        } else {
            HttpResponse::Ok().json(serde_json::json!({
                "session_id": null,
                "authenticated": false,
            }))
        }
    }

    #[actix_web::test]
    async fn test_middleware_bypass_paths() {
        let session_config = SessionIsolationConfig::default();
        let middleware_config = RemoteSessionConfig::default();
        let middleware = RemoteSessionMiddleware::new(session_config, middleware_config);

        let app = test::init_service(
            App::new()
                .wrap(middleware)
                .route("/health", web::get().to(test_handler))
                .route("/api/test", web::get().to(test_handler))
        ).await;

        // Health endpoint should bypass authentication
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // API endpoint should require authentication (but will create session)
        let req = test::TestRequest::get().uri("/api/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_session_context_extraction() {
        let session_config = SessionIsolationConfig::default();
        let middleware_config = RemoteSessionConfig::default();
        let middleware = RemoteSessionMiddleware::new(session_config, middleware_config);

        let app = test::init_service(
            App::new()
                .wrap(middleware)
                .route("/test", web::get().to(test_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("user-agent", "test-client/1.0"))
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // TODO: Add more specific assertions about session context
    }
}