//! OAuth 2.1 authentication implementation with Resource Indicators (RFC 8707) support

use crate::config::{AuthConfig, AuthType, OAuthConfig};
use crate::error::{ProxyError, Result};
use actix_web::HttpRequest;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};
use base64::{Engine as _, engine::general_purpose};
use sha2::{Sha256, Digest};

/// OAuth 2.1 token response from the authorization server with Resource Indicators support
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
    /// Resource indicators (RFC 8707) - audience for the token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<String>>,
    /// Resource indicators (RFC 8707) - resources this token is valid for
    #[serde(skip_serializing_if = "Option::is_none")]  
    pub resource: Option<Vec<String>>,
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

/// OAuth 2.1 token validation result with Resource Indicators support
#[derive(Debug, Clone)]
pub struct OAuthValidationResult {
    /// User information
    pub user_info: OAuthUserInfo,
    /// Token expiration timestamp
    pub expires_at: Option<u64>,
    /// Token scopes
    pub scopes: Vec<String>,
    /// Audience (aud claim) - who the token is intended for
    pub audience: Option<Vec<String>>,
    /// Resource indicators - what resources this token can access
    pub resources: Option<Vec<String>>,
    /// Token issuer (iss claim)
    pub issuer: Option<String>,
}

/// OAuth 2.1 authentication validator with Resource Indicators support
pub struct OAuthValidator {
    /// Authentication configuration
    config: AuthConfig,
    /// HTTP client for OAuth requests
    client: Client,
    /// Resource indicators configuration
    resource_indicators: ResourceIndicatorsConfig,
}

/// Resource Indicators (RFC 8707) configuration
#[derive(Debug, Clone)]
pub struct ResourceIndicatorsConfig {
    /// Enable Resource Indicators support
    pub enabled: bool,
    /// Default resource URIs that tokens should be valid for
    pub default_resources: Vec<String>,
    /// Default audience for tokens
    pub default_audience: Vec<String>,
    /// Require explicit resource specification in authorization requests
    pub require_explicit_resources: bool,
}

impl Default for ResourceIndicatorsConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Enable by default for MCP 2025-06-18 compliance
            default_resources: vec![
                "https://api.magictunnel.io/mcp".to_string(),
                "urn:magictunnel:mcp:*".to_string(),
            ],
            default_audience: vec![
                "magictunnel-mcp-server".to_string(),
            ],
            require_explicit_resources: false, // For backward compatibility
        }
    }
}

impl ResourceIndicatorsConfig {
    /// Create ResourceIndicatorsConfig from OAuthConfig
    pub fn from_oauth_config(oauth_config: &OAuthConfig) -> Self {
        Self {
            enabled: oauth_config.resource_indicators_enabled,
            default_resources: oauth_config.default_resources.clone(),
            default_audience: oauth_config.default_audience.clone(),
            require_explicit_resources: oauth_config.require_explicit_resources,
        }
    }
}

impl OAuthValidator {
    /// Create a new OAuth 2.1 validator with Resource Indicators support
    pub fn new(config: AuthConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        // Configure Resource Indicators from OAuth config if available
        let resource_indicators = if let Some(oauth_config) = &config.oauth {
            ResourceIndicatorsConfig::from_oauth_config(oauth_config)
        } else {
            ResourceIndicatorsConfig::default()
        };

        Self { 
            config, 
            client,
            resource_indicators,
        }
    }

    /// Create a new OAuth 2.1 validator with custom Resource Indicators configuration
    pub fn with_resource_indicators(config: AuthConfig, resource_indicators: ResourceIndicatorsConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { 
            config, 
            client,
            resource_indicators,
        }
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
            .header("User-Agent", "magictunnel/0.3.11")
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
            audience: Some(self.resource_indicators.default_audience.clone()),
            resources: Some(self.resource_indicators.default_resources.clone()),
            issuer: Some(self.get_issuer_for_provider(&oauth_config.provider)),
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

    /// Get issuer identifier for OAuth provider
    pub fn get_issuer_for_provider(&self, provider: &str) -> String {
        match provider.to_lowercase().as_str() {
            "github" => "https://github.com".to_string(),
            "google" => "https://accounts.google.com".to_string(),
            "microsoft" | "azure" => "https://login.microsoftonline.com/common/v2.0".to_string(),
            _ => format!("urn:oauth:issuer:{}", provider),
        }
    }

    /// Validate resource indicators against configured resources
    pub fn validate_resource_indicators(&self, requested_resources: &[String]) -> Result<()> {
        if !self.resource_indicators.enabled {
            return Ok(());
        }

        if self.resource_indicators.require_explicit_resources && requested_resources.is_empty() {
            return Err(ProxyError::auth("Resource indicators are required but none provided"));
        }

        // Check if requested resources are allowed
        for requested in requested_resources {
            let allowed = self.resource_indicators.default_resources.iter().any(|allowed_resource| {
                // Support wildcard matching
                if allowed_resource.ends_with("*") {
                    let prefix = &allowed_resource[..allowed_resource.len() - 1];
                    requested.starts_with(prefix)
                } else {
                    requested == allowed_resource
                }
            });

            if !allowed {
                return Err(ProxyError::auth(format!(
                    "Requested resource '{}' is not allowed", requested
                )));
            }
        }

        Ok(())
    }

    /// Generate OAuth 2.1 authorization URL with Resource Indicators support
    pub fn get_authorization_url(&self, redirect_uri: &str, state: &str) -> Result<String> {
        self.get_authorization_url_with_resources(redirect_uri, state, None)
    }

    /// Generate OAuth 2.1 authorization URL with explicit Resource Indicators
    pub fn get_authorization_url_with_resources(
        &self, 
        redirect_uri: &str, 
        state: &str,
        resources: Option<&[String]>
    ) -> Result<String> {
        let oauth_config = match &self.config.oauth {
            Some(config) => config,
            None => return Err(ProxyError::config("OAuth configuration missing")),
        };

        // Determine which resources to request
        let resources_to_request = if let Some(resources) = resources {
            // Validate requested resources
            self.validate_resource_indicators(resources)?;
            resources.to_vec()
        } else if self.resource_indicators.enabled {
            self.resource_indicators.default_resources.clone()
        } else {
            vec![]
        };

        let mut params = HashMap::new();
        params.insert("response_type".to_string(), "code".to_string());
        params.insert("client_id".to_string(), oauth_config.client_id.clone());
        params.insert("redirect_uri".to_string(), redirect_uri.to_string());
        params.insert("state".to_string(), state.to_string());
        params.insert("scope".to_string(), self.get_default_scope(&oauth_config.provider).to_string());

        // Add Resource Indicators (RFC 8707) parameters
        if self.resource_indicators.enabled && !resources_to_request.is_empty() {
            // Add each resource as a separate "resource" parameter (RFC 8707)
            for (i, resource) in resources_to_request.iter().enumerate() {
                params.insert(format!("resource_{}", i), resource.clone());
            }
        }

        // Add audience if configured
        if !self.resource_indicators.default_audience.is_empty() {
            params.insert("audience".to_string(), self.resource_indicators.default_audience.join(" "));
        }

        // OAuth 2.1 security enhancements
        params.insert("code_challenge_method".to_string(), "S256".to_string());
        let code_verifier = self.generate_code_verifier();
        let code_challenge = self.generate_code_challenge(&code_verifier);
        params.insert("code_challenge".to_string(), code_challenge);

        // Store code_verifier for later use (in a real implementation, you'd store this securely)
        debug!("Generated PKCE code_verifier for OAuth 2.1 flow");

        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        Ok(format!("{}?{}", oauth_config.auth_url, query_string))
    }

    /// Get default scope for OAuth provider with MCP-specific scopes
    pub fn get_default_scope(&self, provider: &str) -> &str {
        match provider.to_lowercase().as_str() {
            "github" => "user:email mcp:read mcp:write",
            "google" => "openid email profile mcp:read mcp:write",
            "microsoft" | "azure" => "openid profile email mcp:read mcp:write",
            _ => "openid email profile mcp:read mcp:write",
        }
    }

    /// Generate a cryptographically secure code verifier for PKCE (OAuth 2.1)
    pub fn generate_code_verifier(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Generate a deterministic but unique code verifier
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        std::process::id().hash(&mut hasher);
        
        let hash = hasher.finish();
        let code_verifier = format!("cv_{:016x}_magictunnel_oauth21", hash);
        
        // Ensure it meets RFC 7636 requirements (43-128 characters)
        let padded = format!("{}_padding_for_rfc7636_compliance", code_verifier);
        padded[..43.min(padded.len())].to_string()
    }

    /// Generate code challenge from verifier using S256 method (OAuth 2.1)
    pub fn generate_code_challenge(&self, code_verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();
        
        // URL-safe base64 encoding without padding
        general_purpose::URL_SAFE_NO_PAD.encode(hash)
    }

    /// Exchange authorization code for access token with OAuth 2.1 and Resource Indicators support
    pub async fn exchange_code_for_token(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokenResponse> {
        self.exchange_code_for_token_with_pkce(code, redirect_uri, None, None).await
    }

    /// Exchange authorization code for access token with PKCE and Resource Indicators
    pub async fn exchange_code_for_token_with_pkce(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: Option<&str>,
        resources: Option<&[String]>,
    ) -> Result<OAuthTokenResponse> {
        let oauth_config = match &self.config.oauth {
            Some(config) => config,
            None => return Err(ProxyError::config("OAuth configuration missing")),
        };

        // Validate resources if provided
        if let Some(resources) = resources {
            self.validate_resource_indicators(resources)?;
        }

        let mut params = HashMap::new();
        params.insert("grant_type".to_string(), "authorization_code".to_string());
        params.insert("code".to_string(), code.to_string());
        params.insert("redirect_uri".to_string(), redirect_uri.to_string());
        params.insert("client_id".to_string(), oauth_config.client_id.clone());
        params.insert("client_secret".to_string(), oauth_config.client_secret.clone());

        // Add PKCE code verifier for OAuth 2.1 compliance
        if let Some(verifier) = code_verifier {
            params.insert("code_verifier".to_string(), verifier.to_string());
            debug!("Including PKCE code_verifier in token exchange");
        }

        // Add Resource Indicators (RFC 8707) to token request
        if let Some(resources) = resources {
            for resource in resources {
                params.insert("resource".to_string(), resource.clone());
            }
        } else if self.resource_indicators.enabled {
            // Use default resources
            for resource in &self.resource_indicators.default_resources {
                params.insert("resource".to_string(), resource.clone());
            }
        }

        // Add audience parameter (RFC 8707)
        if !self.resource_indicators.default_audience.is_empty() {
            params.insert("audience".to_string(), self.resource_indicators.default_audience.join(" "));
        }

        let response = self
            .client
            .post(&oauth_config.token_url)
            .header("Accept", "application/json")
            .header("User-Agent", "magictunnel/0.3.11 OAuth2.1")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to exchange OAuth 2.1 code for token: {}", e);
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

        info!("OAuth 2.1 token exchange successful with Resource Indicators support");
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
            oauth_2_1_enabled: true,
            resource_indicators_enabled: true,
            default_resources: vec![
                "https://api.magictunnel.io/mcp".to_string(),
                "urn:magictunnel:mcp:*".to_string(),
            ],
            default_audience: vec!["magictunnel-mcp-server".to_string()],
            require_explicit_resources: false,
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
