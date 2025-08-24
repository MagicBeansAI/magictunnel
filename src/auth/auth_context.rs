//! Authentication Context for MCP Protocol Integration
//!
//! This module provides the critical AuthenticationContext that flows through
//! the entire MCP request pipeline, enabling OAuth 2.1 tokens and other auth
//! credentials to reach external API calls in tool execution.

use crate::auth::{AuthenticationResult, OAuthTokenResponse};
use crate::error::{Result, ProxyError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};
use secrecy::{Secret, ExposeSecret};

/// Provider-specific token information
#[derive(Debug, Clone)]
pub struct ProviderToken {
    /// OAuth access token for API calls
    pub access_token: Secret<String>,
    /// Refresh token for token renewal
    pub refresh_token: Option<Secret<String>>,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Token expiration timestamp (Unix seconds)
    pub expires_at: Option<u64>,
    /// OAuth scopes granted to this token
    pub scopes: Vec<String>,
    /// Provider-specific metadata (user ID, etc.)
    pub metadata: HashMap<String, String>,
}

impl ProviderToken {
    /// Create a new provider token from OAuth response
    pub fn from_oauth_response(
        oauth_response: &OAuthTokenResponse,
        provider: &str,
    ) -> Self {
        let expires_at = oauth_response.expires_in.map(|expires_in| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + expires_in as u64
        });

        let mut metadata = HashMap::new();
        metadata.insert("provider".to_string(), provider.to_string());
        // Note: user_info is not part of OAuthTokenResponse, it's in validation results
        // This will be populated by authentication middleware

        Self {
            access_token: oauth_response.access_token.clone(),
            refresh_token: oauth_response.refresh_token.clone(),
            token_type: oauth_response.token_type.clone(),
            expires_at,
            scopes: oauth_response.scope.clone()
                .map(|s| s.split_whitespace().map(|scope| scope.to_string()).collect())
                .unwrap_or_default(),
            metadata,
        }
    }

    /// Check if token is expired (with 60 second buffer)
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                // Add 60 second buffer to prevent using tokens that expire mid-request
                now >= (expires_at - 60)
            }
            None => false, // No expiration time means token doesn't expire
        }
    }

    /// Get authorization header value for HTTP requests
    pub fn get_authorization_header(&self) -> String {
        format!("{} {}", self.token_type, self.access_token.expose_secret())
    }

    /// Check if token has required scope
    pub fn has_scope(&self, required_scope: &str) -> bool {
        self.scopes.contains(&required_scope.to_string())
    }

    /// Get user ID from metadata
    pub fn get_user_id(&self) -> Option<&String> {
        self.metadata.get("user_id")
    }

    /// Get provider name from metadata
    pub fn get_provider(&self) -> Option<&String> {
        self.metadata.get("provider")
    }
}

/// Authentication method used for the current request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthMethod {
    /// No authentication
    None,
    /// API key authentication
    ApiKey { key_ref: String },
    /// OAuth 2.1 authentication
    OAuth { provider: String, scopes: Vec<String> },
    /// Device Code Flow authentication
    DeviceCode { provider: String, device_code: String },
    /// Service Account authentication
    ServiceAccount { account_ref: String },
    /// JWT token authentication
    Jwt { issuer: String },
}

/// Comprehensive authentication context that flows through MCP request pipeline
#[derive(Debug, Clone)]
pub struct AuthenticationContext {
    /// Unique user identifier
    pub user_id: String,
    /// Provider-specific tokens (OAuth, API keys, etc.)
    pub provider_tokens: HashMap<String, ProviderToken>,
    /// Session ID for request tracking
    pub session_id: String,
    /// Authentication method used
    pub auth_method: AuthMethod,
    /// Scopes granted to this authentication
    pub scopes: Vec<String>,
    /// Request timestamp
    pub timestamp: u64,
    /// Additional authentication metadata
    pub metadata: HashMap<String, String>,
}

impl AuthenticationContext {
    /// Create authentication context from middleware result
    pub fn from_auth_result(
        auth_result: &AuthenticationResult,
        session_id: String,
    ) -> Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        match auth_result {
            AuthenticationResult::ApiKey(key_entry) => {
                let mut metadata = HashMap::new();
                metadata.insert("key_name".to_string(), key_entry.name.clone());
                // Note: using name as key reference for config ApiKeyEntry
                metadata.insert("key_name".to_string(), key_entry.name.clone());

                // Create provider token for API key
                let mut provider_tokens = HashMap::new();
                let provider_token = ProviderToken {
                    access_token: Secret::new(key_entry.key.clone()),
                    refresh_token: None,
                    token_type: "ApiKey".to_string(),
                    expires_at: None,
                    scopes: key_entry.permissions.clone(),
                    metadata: metadata.clone(),
                };
                provider_tokens.insert("api_key".to_string(), provider_token);

                Ok(Self {
                    user_id: key_entry.name.clone(),
                    provider_tokens,
                    session_id,
                    auth_method: AuthMethod::ApiKey {
                        key_ref: key_entry.name.clone(), // Use name as key_ref for config ApiKeyEntry
                    },
                    scopes: key_entry.permissions.clone(),
                    timestamp,
                    metadata,
                })
            }
            AuthenticationResult::OAuth(oauth_result) => {
                let mut metadata = HashMap::new();
                metadata.insert("user_id".to_string(), oauth_result.user_info.id.clone());
                if let Some(ref email) = oauth_result.user_info.email {
                    metadata.insert("user_email".to_string(), email.clone());
                }
                // Note: OAuthValidationResult doesn't have provider field, inferring from context
                metadata.insert("provider".to_string(), "oauth".to_string());

                // Create provider token for OAuth
                let mut provider_tokens = HashMap::new();
                // Note: OAuthValidationResult doesn't have token_response field
                // Create a basic provider token from available information
                let provider_token = ProviderToken {
                    access_token: Secret::new(
                        oauth_result.access_token
                            .clone()
                            .unwrap_or_else(|| {
                                warn!("OAuth validation result missing access token, using fallback");
                                "oauth_token_not_available".to_string()
                            })
                    ),
                    refresh_token: None,
                    token_type: "Bearer".to_string(),
                    expires_at: oauth_result.expires_at,
                    scopes: oauth_result.scopes.clone(),
                    metadata: metadata.clone(),
                };
                provider_tokens.insert("oauth".to_string(), provider_token);

                Ok(Self {
                    user_id: oauth_result.user_info.id.clone(),
                    provider_tokens,
                    session_id,
                    auth_method: AuthMethod::OAuth {
                        provider: "oauth".to_string(), // Generic provider name
                        scopes: oauth_result.scopes.clone(),
                    },
                    scopes: oauth_result.scopes.clone(),
                    timestamp,
                    metadata,
                })
            }
            AuthenticationResult::Jwt(jwt_result) => {
                let mut metadata = HashMap::new();
                metadata.insert("user_id".to_string(), jwt_result.user_info.id.clone());
                if let Some(ref email) = jwt_result.user_info.email {
                    metadata.insert("user_email".to_string(), email.clone());
                }
                // Note: JwtValidationResult doesn't have issuer field
                metadata.insert("issuer".to_string(), "jwt".to_string());

                // Create provider token for JWT
                let mut provider_tokens = HashMap::new();
                let provider_token = ProviderToken {
                    access_token: Secret::new(
                        jwt_result.jwt_token
                            .clone()
                            .unwrap_or_else(|| {
                                warn!("JWT validation result missing token, using fallback");
                                "jwt_token_not_available".to_string()
                            })
                    ),
                    refresh_token: None,
                    token_type: "Bearer".to_string(),
                    expires_at: Some(jwt_result.claims.exp), // Use JWT expiration claim
                    scopes: jwt_result.permissions.clone(),
                    metadata: metadata.clone(),
                };
                provider_tokens.insert("jwt".to_string(), provider_token);

                Ok(Self {
                    user_id: jwt_result.user_info.id.clone(),
                    provider_tokens,
                    session_id,
                    auth_method: AuthMethod::Jwt {
                        issuer: "jwt".to_string(), // Generic issuer
                    },
                    scopes: jwt_result.permissions.clone(),
                    timestamp,
                    metadata,
                })
            }
            AuthenticationResult::ServiceAccount(sa_result) => {
                let mut metadata = HashMap::new();
                metadata.insert("user_id".to_string(), sa_result.user_info.id.clone());
                if let Some(ref name) = sa_result.user_info.name {
                    metadata.insert("account_name".to_string(), name.clone());
                }
                if let Some(ref email) = sa_result.user_info.email {
                    metadata.insert("account_email".to_string(), email.clone());
                }
                metadata.insert("account_type".to_string(), format!("{:?}", sa_result.account_type));

                // Add provider metadata
                for (key, value) in &sa_result.metadata {
                    metadata.insert(key.clone(), value.clone());
                }

                // Create provider token for Service Account
                let mut provider_tokens = HashMap::new();
                let provider_token = ProviderToken {
                    access_token: Secret::new("service_account_token".to_string()),
                    refresh_token: None,
                    token_type: "ServiceAccount".to_string(),
                    expires_at: sa_result.expires_at,
                    scopes: sa_result.permissions.clone(),
                    metadata: metadata.clone(),
                };
                let provider_name = sa_result.metadata.get("provider")
                    .unwrap_or(&"service_account".to_string()).clone();
                provider_tokens.insert(provider_name.clone(), provider_token);

                Ok(Self {
                    user_id: sa_result.user_info.id.clone(),
                    provider_tokens,
                    session_id,
                    auth_method: AuthMethod::ServiceAccount {
                        account_ref: provider_name,
                    },
                    scopes: sa_result.permissions.clone(),
                    timestamp,
                    metadata,
                })
            }
            AuthenticationResult::DeviceCode(device_result) => {
                let mut metadata = HashMap::new();
                // Device code typically doesn't have user info initially
                if let Some(ref user_info) = device_result.user_info {
                    metadata.insert("user_id".to_string(), user_info.id.clone());
                    if let Some(ref name) = user_info.name {
                        metadata.insert("user_name".to_string(), name.clone());
                    }
                    if let Some(ref email) = user_info.email {
                        metadata.insert("user_email".to_string(), email.clone());
                    }
                }

                // Add device-specific metadata
                metadata.insert("device_code".to_string(), device_result.device_code.clone());
                metadata.insert("verification_uri".to_string(), device_result.device_authorization.verification_uri.clone());
                metadata.insert("user_code".to_string(), device_result.device_authorization.user_code.clone());
                metadata.insert("expires_in".to_string(), device_result.device_authorization.expires_in.to_string());

                // Add additional metadata from device result
                for (key, value) in &device_result.metadata {
                    metadata.insert(key.clone(), value.clone());
                }

                // Create provider token for Device Code Flow
                let mut provider_tokens = HashMap::new();
                let provider_token = ProviderToken {
                    access_token: Secret::new(device_result.device_code.clone()), // Use device code as temporary token
                    refresh_token: None,
                    token_type: "DeviceCode".to_string(),
                    expires_at: Some(device_result.expires_at),
                    scopes: device_result.scopes.clone(),
                    metadata: metadata.clone(),
                };
                let provider_name = device_result.metadata.get("provider")
                    .unwrap_or(&"device_code".to_string()).clone();
                provider_tokens.insert(provider_name.clone(), provider_token);

                let user_id = device_result.user_info
                    .as_ref()
                    .map(|info| info.id.clone())
                    .unwrap_or_else(|| format!("device_code:{}", device_result.device_code));

                Ok(Self {
                    user_id,
                    provider_tokens,
                    session_id,
                    auth_method: AuthMethod::DeviceCode {
                        provider: provider_name.clone(),
                        device_code: device_result.device_code.clone(),
                    },
                    scopes: device_result.scopes.clone(),
                    timestamp,
                    metadata,
                })
            }
        }
    }

    /// Create empty authentication context (no auth)
    pub fn none(session_id: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            user_id: "anonymous".to_string(),
            provider_tokens: HashMap::new(),
            session_id,
            auth_method: AuthMethod::None,
            scopes: Vec::new(),
            timestamp,
            metadata: HashMap::new(),
        }
    }

    /// Get provider token by name
    pub fn get_provider_token(&self, provider: &str) -> Option<&ProviderToken> {
        self.provider_tokens.get(provider)
    }

    /// Get authorization header for a specific provider
    pub fn get_authorization_header(&self, provider: &str) -> Option<String> {
        self.provider_tokens
            .get(provider)
            .map(|token| token.get_authorization_header())
    }

    /// Check if authentication has required scope
    pub fn has_scope(&self, required_scope: &str) -> bool {
        self.scopes.contains(&required_scope.to_string())
    }

    /// Check if any provider token has the required scope
    pub fn has_provider_scope(&self, provider: &str, required_scope: &str) -> bool {
        self.provider_tokens
            .get(provider)
            .map(|token| token.has_scope(required_scope))
            .unwrap_or(false)
    }

    /// Get all HTTP headers needed for authenticated requests
    pub fn get_auth_headers(&self, provider: Option<&str>) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        // Add session headers
        headers.insert("X-Session-ID".to_string(), self.session_id.clone());
        headers.insert("X-User-ID".to_string(), self.user_id.clone());

        // Add provider-specific authorization header
        if let Some(provider_name) = provider {
            if let Some(auth_header) = self.get_authorization_header(provider_name) {
                headers.insert("Authorization".to_string(), auth_header);
                headers.insert("X-Auth-Provider".to_string(), provider_name.to_string());
            }
        } else {
            // Use first available token if no provider specified
            for (provider_name, token) in &self.provider_tokens {
                if !token.is_expired() {
                    headers.insert("Authorization".to_string(), token.get_authorization_header());
                    headers.insert("X-Auth-Provider".to_string(), provider_name.to_string());
                    break;
                }
            }
        }

        headers
    }

    /// Validate authentication context and check for expired tokens
    pub fn validate(&self) -> Result<()> {
        // Check for expired tokens
        let mut expired_providers = Vec::new();
        for (provider, token) in &self.provider_tokens {
            if token.is_expired() {
                expired_providers.push(provider.clone());
            }
        }

        if !expired_providers.is_empty() {
            warn!(
                "Authentication context has expired tokens for providers: {:?}",
                expired_providers
            );
            return Err(ProxyError::auth(format!(
                "Expired tokens for providers: {}",
                expired_providers.join(", ")
            )));
        }

        // Ensure user_id is not empty
        if self.user_id.is_empty() || self.user_id == "anonymous" {
            match self.auth_method {
                AuthMethod::None => {
                    // Anonymous access is OK for no auth
                }
                _ => {
                    return Err(ProxyError::auth(
                        "Authentication context missing user ID".to_string()
                    ));
                }
            }
        }

        debug!(
            "Authentication context validated: user_id={}, method={:?}, providers={}",
            self.user_id,
            self.auth_method,
            self.provider_tokens.len()
        );

        Ok(())
    }

    /// Check if context is anonymous (no authentication)
    pub fn is_anonymous(&self) -> bool {
        matches!(self.auth_method, AuthMethod::None)
    }

    /// Get authentication method display name
    pub fn auth_method_display(&self) -> String {
        match &self.auth_method {
            AuthMethod::None => "None".to_string(),
            AuthMethod::ApiKey { key_ref } => format!("API Key ({})", key_ref),
            AuthMethod::OAuth { provider, .. } => format!("OAuth ({})", provider),
            AuthMethod::DeviceCode { provider, .. } => format!("Device Code ({})", provider),
            AuthMethod::ServiceAccount { account_ref } => {
                format!("Service Account ({})", account_ref)
            }
            AuthMethod::Jwt { issuer } => format!("JWT ({})", issuer),
        }
    }
}

/// Tool execution context that includes authentication
#[derive(Debug, Clone)]
pub struct ToolExecutionContext {
    /// Tool name being executed
    pub tool_name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
    /// Authentication context (optional)
    pub auth_context: Option<AuthenticationContext>,
    /// Execution timestamp
    pub timestamp: u64,
    /// Execution metadata
    pub metadata: HashMap<String, String>,
}

impl ToolExecutionContext {
    /// Create new tool execution context
    pub fn new(
        tool_name: String,
        arguments: serde_json::Value,
        auth_context: Option<AuthenticationContext>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            tool_name,
            arguments,
            auth_context,
            timestamp,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to execution context
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get authentication headers for external API calls
    pub fn get_auth_headers(&self, provider: Option<&str>) -> HashMap<String, String> {
        match &self.auth_context {
            Some(auth_ctx) => auth_ctx.get_auth_headers(provider),
            None => HashMap::new(),
        }
    }

    /// Check if execution context has authentication
    pub fn has_auth(&self) -> bool {
        self.auth_context.is_some()
    }

    /// Get user ID from authentication context
    pub fn get_user_id(&self) -> String {
        match &self.auth_context {
            Some(auth_ctx) => auth_ctx.user_id.clone(),
            None => "anonymous".to_string(),
        }
    }

    /// Validate execution context
    pub fn validate(&self) -> Result<()> {
        if self.tool_name.is_empty() {
            return Err(ProxyError::validation(
                "Tool execution context missing tool name".to_string()
            ));
        }

        // Validate authentication context if present
        if let Some(ref auth_ctx) = self.auth_context {
            auth_ctx.validate()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{OAuthValidationResult, OAuthUserInfo};
    use crate::config::ApiKeyEntry as ConfigApiKeyEntry;

    fn create_test_oauth_result() -> OAuthValidationResult {
        OAuthValidationResult {
            user_info: OAuthUserInfo {
                id: "test_user_123".to_string(),
                email: Some("test@example.com".to_string()),
                name: Some("Test User".to_string()),
                login: Some("test_user_123".to_string()),
            },
            expires_at: Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 3600),
            scopes: vec!["repo".to_string(), "user:email".to_string()],
            audience: None,
            resources: None,
            issuer: Some("https://github.com".to_string()),
            access_token: Some("gho_test_access_token_12345".to_string()),
        }
    }

    fn create_test_api_key() -> ConfigApiKeyEntry {
        ConfigApiKeyEntry::with_permissions(
            "test_key_123".to_string(),
            "Test API Key".to_string(),
            vec!["read".to_string(), "write".to_string()],
        )
    }

    #[test]
    fn test_provider_token_from_oauth() {
        let oauth_response = OAuthTokenResponse {
            access_token: Secret::new("test_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            refresh_token: Some(Secret::new("refresh_token".to_string())),
            scope: Some("repo".to_string()),
            audience: None,
            resource: None,
        };

        let provider_token = ProviderToken::from_oauth_response(&oauth_response, "github");

        assert_eq!(provider_token.access_token.expose_secret(), "test_token");
        assert_eq!(provider_token.token_type, "Bearer");
        assert!(provider_token.refresh_token.is_some());
        assert_eq!(provider_token.scopes, vec!["repo"]);
        assert!(!provider_token.is_expired());
    }

    #[test]
    fn test_auth_context_from_oauth() {
        let oauth_result = create_test_oauth_result();
        let auth_result = AuthenticationResult::OAuth(oauth_result);
        
        let auth_context = AuthenticationContext::from_auth_result(
            &auth_result,
            "session_123".to_string(),
        ).unwrap();

        assert_eq!(auth_context.user_id, "test_user_123");
        assert_eq!(auth_context.session_id, "session_123");
        assert!(matches!(auth_context.auth_method, AuthMethod::OAuth { .. }));
        assert_eq!(auth_context.scopes, vec!["repo", "user:email"]);
        assert!(auth_context.provider_tokens.contains_key("oauth"));
    }

    #[test]
    fn test_auth_context_from_api_key() {
        let api_key = create_test_api_key();
        let auth_result = AuthenticationResult::ApiKey(api_key);
        
        let auth_context = AuthenticationContext::from_auth_result(
            &auth_result,
            "session_456".to_string(),
        ).unwrap();

        assert_eq!(auth_context.user_id, "Test API Key");
        assert_eq!(auth_context.session_id, "session_456");
        assert!(matches!(auth_context.auth_method, AuthMethod::ApiKey { .. }));
        assert_eq!(auth_context.scopes, vec!["read", "write"]);
        assert!(auth_context.provider_tokens.contains_key("api_key"));
    }

    #[test]
    fn test_tool_execution_context() {
        let oauth_result = create_test_oauth_result();
        let auth_result = AuthenticationResult::OAuth(oauth_result);
        let auth_context = AuthenticationContext::from_auth_result(
            &auth_result,
            "session_789".to_string(),
        ).unwrap();

        let tool_context = ToolExecutionContext::new(
            "github_create_issue".to_string(),
            serde_json::json!({"title": "Test Issue"}),
            Some(auth_context),
        ).with_metadata("routing_type".to_string(), "external_mcp".to_string());

        assert_eq!(tool_context.tool_name, "github_create_issue");
        assert!(tool_context.has_auth());
        assert_eq!(tool_context.get_user_id(), "test_user_123");

        let headers = tool_context.get_auth_headers(Some("oauth"));
        assert!(headers.contains_key("Authorization"));
        assert!(headers.contains_key("X-Session-ID"));
        assert!(headers.contains_key("X-User-ID"));
        
        assert!(tool_context.validate().is_ok());
    }

    #[test]
    fn test_anonymous_context() {
        let auth_context = AuthenticationContext::none("session_anon".to_string());

        assert_eq!(auth_context.user_id, "anonymous");
        assert!(auth_context.is_anonymous());
        assert_eq!(auth_context.auth_method_display(), "None");
        assert!(auth_context.provider_tokens.is_empty());
        assert!(auth_context.validate().is_ok());
    }

    #[test]
    fn test_auth_headers_generation() {
        let oauth_result = create_test_oauth_result();
        let auth_result = AuthenticationResult::OAuth(oauth_result);
        let auth_context = AuthenticationContext::from_auth_result(
            &auth_result,
            "session_headers".to_string(),
        ).unwrap();

        let headers = auth_context.get_auth_headers(Some("oauth"));
        
        assert!(headers.contains_key("Authorization"));
        assert!(headers.contains_key("X-Session-ID"));
        assert!(headers.contains_key("X-User-ID"));
        assert!(headers.contains_key("X-Auth-Provider"));
        
        assert_eq!(headers.get("X-Session-ID").unwrap(), "session_headers");
        assert_eq!(headers.get("X-User-ID").unwrap(), "test_user_123");
        assert_eq!(headers.get("X-Auth-Provider").unwrap(), "oauth");
    }
}