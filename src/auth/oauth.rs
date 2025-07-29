//! OAuth 2.0 authentication implementation

use crate::config::{AuthConfig, AuthType, OAuthConfig};
use crate::error::{ProxyError, Result};
use actix_web::HttpRequest;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

/// OAuth 2.0 token response from the authorization server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    /// Access token
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Token expiration time in seconds
    pub expires_in: Option<u64>,
    /// Refresh token for getting new access tokens
    pub refresh_token: Option<String>,
    /// Scope of the access token
    pub scope: Option<String>,
}

/// OAuth 2.0 user information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserInfo {
    /// User ID
    pub id: String,
    /// User email
    pub email: Option<String>,
    /// User name
    pub name: Option<String>,
    /// User login/username
    pub login: Option<String>,
}

/// OAuth 2.0 token validation result
#[derive(Debug, Clone)]
pub struct OAuthValidationResult {
    /// User information
    pub user_info: OAuthUserInfo,
    /// Token expiration timestamp
    pub expires_at: Option<u64>,
    /// Token scopes
    pub scopes: Vec<String>,
}

/// OAuth 2.0 authentication validator
pub struct OAuthValidator {
    /// Authentication configuration
    config: AuthConfig,
    /// HTTP client for OAuth requests
    client: Client,
}

impl OAuthValidator {
    /// Create a new OAuth validator
    pub fn new(config: AuthConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { config, client }
    }

    /// Get the configuration (for testing)
    pub fn config(&self) -> &AuthConfig {
        &self.config
    }

    /// Validate an HTTP request for OAuth authentication
    pub async fn validate_request(&self, req: &HttpRequest) -> Result<Option<OAuthValidationResult>> {
        // If authentication is disabled, allow all requests
        if !self.config.enabled {
            debug!("Authentication disabled, allowing request");
            return Ok(None);
        }

        // Only validate OAuth auth type
        if self.config.r#type != AuthType::OAuth {
            debug!("Non-OAuth auth type, skipping OAuth validation");
            return Ok(None);
        }

        let oauth_config = match &self.config.oauth {
            Some(config) => config,
            None => {
                warn!("OAuth authentication enabled but no OAuth configuration found");
                return Err(ProxyError::auth("OAuth configuration missing"));
            }
        };

        // Extract access token from request headers
        let access_token = self.extract_access_token(req)?;

        // Validate the access token
        self.validate_access_token(&access_token, oauth_config).await
    }

    /// Extract access token from request headers
    pub fn extract_access_token(&self, req: &HttpRequest) -> Result<String> {
        // Look for Authorization header with Bearer token
        let auth_header = req
            .headers()
            .get("Authorization")
            .ok_or_else(|| ProxyError::auth("Missing Authorization header"))?
            .to_str()
            .map_err(|_| ProxyError::auth("Invalid Authorization header encoding"))?;

        // Extract token from "Bearer <token>" format
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            Ok(token.trim().to_string())
        } else {
            Err(ProxyError::auth("Invalid Authorization header format. Expected: Bearer <token>"))
        }
    }

    /// Validate an access token with the OAuth provider
    async fn validate_access_token(
        &self,
        access_token: &str,
        oauth_config: &OAuthConfig,
    ) -> Result<Option<OAuthValidationResult>> {
        debug!("Validating OAuth access token with provider: {}", oauth_config.provider);

        // Get user info endpoint based on provider
        let user_info_url = self.get_user_info_url(oauth_config)?;

        // Make request to user info endpoint
        let response = self
            .client
            .get(&user_info_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "magictunnel/0.2.48")
            .send()
            .await
            .map_err(|e| {
                error!("Failed to validate OAuth token: {}", e);
                ProxyError::auth("Failed to validate OAuth token")
            })?;

        if !response.status().is_success() {
            warn!("OAuth token validation failed with status: {}", response.status());
            return Err(ProxyError::auth("Invalid or expired OAuth token"));
        }

        // Parse user information
        let user_info: OAuthUserInfo = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse OAuth user info: {}", e);
                ProxyError::auth("Invalid OAuth user info response")
            })?;

        info!("OAuth token validation successful for user: {}", user_info.id);

        // For now, we'll set a default expiration time (1 hour from now)
        // In a real implementation, you might want to introspect the token or cache validation results
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 3600; // 1 hour

        Ok(Some(OAuthValidationResult {
            user_info,
            expires_at: Some(expires_at),
            scopes: vec!["read".to_string(), "write".to_string()], // Default scopes
        }))
    }

    /// Get user info endpoint URL based on OAuth provider
    pub fn get_user_info_url(&self, oauth_config: &OAuthConfig) -> Result<String> {
        match oauth_config.provider.to_lowercase().as_str() {
            "github" => Ok("https://api.github.com/user".to_string()),
            "google" => Ok("https://www.googleapis.com/oauth2/v2/userinfo".to_string()),
            "microsoft" | "azure" => Ok("https://graph.microsoft.com/v1.0/me".to_string()),
            _ => {
                // For custom providers, try to construct URL from auth_url
                if oauth_config.auth_url.contains("github.com") {
                    Ok("https://api.github.com/user".to_string())
                } else if oauth_config.auth_url.contains("googleapis.com") {
                    Ok("https://www.googleapis.com/oauth2/v2/userinfo".to_string())
                } else if oauth_config.auth_url.contains("microsoft.com") || oauth_config.auth_url.contains("microsoftonline.com") {
                    Ok("https://graph.microsoft.com/v1.0/me".to_string())
                } else {
                    Err(ProxyError::config(format!(
                        "Unsupported OAuth provider: {}. Supported providers: github, google, microsoft",
                        oauth_config.provider
                    )))
                }
            }
        }
    }

    /// Generate OAuth authorization URL for the authorization code flow
    pub fn get_authorization_url(&self, redirect_uri: &str, state: &str) -> Result<String> {
        let oauth_config = match &self.config.oauth {
            Some(config) => config,
            None => return Err(ProxyError::config("OAuth configuration missing")),
        };

        let mut params = HashMap::new();
        params.insert("response_type", "code");
        params.insert("client_id", &oauth_config.client_id);
        params.insert("redirect_uri", redirect_uri);
        params.insert("state", state);
        params.insert("scope", self.get_default_scope(&oauth_config.provider));

        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        Ok(format!("{}?{}", oauth_config.auth_url, query_string))
    }

    /// Get default scope for OAuth provider
    pub fn get_default_scope(&self, provider: &str) -> &str {
        match provider.to_lowercase().as_str() {
            "github" => "user:email",
            "google" => "openid email profile",
            "microsoft" | "azure" => "openid profile email",
            _ => "openid email profile",
        }
    }

    /// Exchange authorization code for access token
    pub async fn exchange_code_for_token(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokenResponse> {
        let oauth_config = match &self.config.oauth {
            Some(config) => config,
            None => return Err(ProxyError::config("OAuth configuration missing")),
        };

        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("redirect_uri", redirect_uri);
        params.insert("client_id", &oauth_config.client_id);
        params.insert("client_secret", &oauth_config.client_secret);

        let response = self
            .client
            .post(&oauth_config.token_url)
            .header("Accept", "application/json")
            .header("User-Agent", "magictunnel/0.2.48")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to exchange OAuth code for token: {}", e);
                ProxyError::auth("Failed to exchange authorization code")
            })?;

        if !response.status().is_success() {
            error!("OAuth token exchange failed with status: {}", response.status());
            return Err(ProxyError::auth("Failed to exchange authorization code"));
        }

        let token_response: OAuthTokenResponse = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse OAuth token response: {}", e);
                ProxyError::auth("Invalid OAuth token response")
            })?;

        info!("OAuth token exchange successful");
        Ok(token_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthType;
    use actix_web::test::TestRequest;

    fn create_test_oauth_config() -> AuthConfig {
        let mut config = AuthConfig::default();
        config.enabled = true;
        config.r#type = AuthType::OAuth;
        config.oauth = Some(OAuthConfig {
            provider: "github".to_string(),
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
        });
        config
    }

    #[test]
    fn test_oauth_validator_creation() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);
        assert!(validator.config.enabled);
    }

    #[test]
    fn test_extract_access_token_success() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);

        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer test_token_123"))
            .to_http_request();

        let result = validator.extract_access_token(&req).unwrap();
        assert_eq!(result, "test_token_123");
    }

    #[test]
    fn test_extract_access_token_missing_header() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);

        let req = TestRequest::default().to_http_request();

        let result = validator.extract_access_token(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_user_info_url() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);

        let oauth_config = validator.config.oauth.as_ref().unwrap();
        let url = validator.get_user_info_url(oauth_config).unwrap();
        assert_eq!(url, "https://api.github.com/user");
    }

    #[test]
    fn test_get_authorization_url() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);

        let url = validator.get_authorization_url("http://localhost:8080/callback", "test_state").unwrap();
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("state=test_state"));
    }
}
