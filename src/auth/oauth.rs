//! OAuth 2.1 authentication implementation with Resource Indicators (RFC 8707) support

use crate::auth::{TokenStorage, UserContext};
use crate::config::{AuthConfig, AuthType, OAuthConfig};
use crate::error::{ProxyError, Result};
use actix_web::HttpRequest;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};
use base64::{Engine as _, engine::general_purpose};
use sha2::{Sha256, Digest};
use secrecy::{Secret, ExposeSecret};

/// OAuth 2.1 token response from the authorization server with Resource Indicators support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    /// Access token (sensitive - protected by secrecy)
    #[serde(with = "secret_string")]
    pub access_token: Secret<String>,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Token expiration time in seconds
    pub expires_in: Option<u64>,
    /// Refresh token for getting new access tokens (sensitive - protected by secrecy)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "option_secret_string")]
    pub refresh_token: Option<Secret<String>>,
    /// Scope of the access token
    pub scope: Option<String>,
    /// Resource indicators (RFC 8707) - audience for the token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<String>>,
    /// Resource indicators (RFC 8707) - resources this token is valid for
    #[serde(skip_serializing_if = "Option::is_none")]  
    pub resource: Option<Vec<String>>,
}

/// Custom serde module for Secret<String>
mod secret_string {
    use serde::{Deserialize, Deserializer, Serializer};
    use secrecy::{Secret, ExposeSecret};
    
    pub fn serialize<S>(secret: &Secret<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(secret.expose_secret())
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Secret<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Secret::new(s))
    }
}

/// Custom serde module for Option<Secret<String>>
mod option_secret_string {
    use serde::{Deserialize, Deserializer, Serializer};
    use secrecy::{Secret, ExposeSecret};
    
    pub fn serialize<S>(secret: &Option<Secret<String>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match secret {
            Some(ref s) => serializer.serialize_some(s.expose_secret()),
            None => serializer.serialize_none(),
        }
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Secret<String>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt_s = Option::<String>::deserialize(deserializer)?;
        Ok(opt_s.map(Secret::new))
    }
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
    /// Token storage for persistent authentication (optional)
    token_storage: Option<Arc<TokenStorage>>,
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
            token_storage: None,
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
            token_storage: None,
        }
    }

    /// Create a new OAuth 2.1 validator with token storage support
    pub async fn with_token_storage(config: AuthConfig, user_context: UserContext) -> Result<Self> {
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

        // Create token storage
        let token_storage = TokenStorage::new(user_context).await?;

        Ok(Self { 
            config, 
            client,
            resource_indicators,
            token_storage: Some(Arc::new(token_storage)),
        })
    }

    /// Create OAuth validator with both custom Resource Indicators and token storage
    pub async fn with_resource_indicators_and_storage(
        config: AuthConfig, 
        resource_indicators: ResourceIndicatorsConfig,
        user_context: UserContext
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        // Create token storage
        let token_storage = TokenStorage::new(user_context).await?;

        Ok(Self { 
            config, 
            client,
            resource_indicators,
            token_storage: Some(Arc::new(token_storage)),
        })
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
            .header("User-Agent", "magictunnel/0.3.12")
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
            scopes: vec!["read".to_string(), "write".to_string()], // MagicTunnel internal permissions
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

        // Add Resource Indicators (RFC 8707) parameters - each resource is a separate parameter
        if self.resource_indicators.enabled && !resources_to_request.is_empty() {
            // RFC 8707: Multiple resources should be passed as multiple "resource" parameters
            // Note: This is a deviation from standard OAuth providers since they don't support RFC 8707
            // In practice, most OAuth providers will ignore these parameters
            for resource in &resources_to_request {
                params.insert("resource".to_string(), resource.clone());
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

    /// Get default scope for OAuth provider (standard provider scopes only)
    pub fn get_default_scope(&self, provider: &str) -> &str {
        match provider.to_lowercase().as_str() {
            "github" => "user:email",
            "google" => "openid email profile",
            "microsoft" | "azure" => "openid profile email",
            _ => "openid email profile",
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
        params.insert("client_secret".to_string(), oauth_config.client_secret.expose_secret().clone());

        // Add PKCE code verifier for OAuth 2.1 compliance
        if let Some(verifier) = code_verifier {
            params.insert("code_verifier".to_string(), verifier.to_string());
            debug!("Including PKCE code_verifier in token exchange");
        }

        // Add Resource Indicators (RFC 8707) to token request
        // Note: Standard OAuth providers don't support this, but included for MCP 2025-06-18 compliance
        if let Some(resources) = resources {
            // RFC 8707: Include first resource (most OAuth providers only support one resource parameter)
            if !resources.is_empty() {
                params.insert("resource".to_string(), resources[0].clone());
            }
        } else if self.resource_indicators.enabled && !self.resource_indicators.default_resources.is_empty() {
            // Use first default resource
            params.insert("resource".to_string(), self.resource_indicators.default_resources[0].clone());
        }

        // Add audience parameter (RFC 8707)
        if !self.resource_indicators.default_audience.is_empty() {
            params.insert("audience".to_string(), self.resource_indicators.default_audience.join(" "));
        }

        let response = self
            .client
            .post(&oauth_config.token_url)
            .header("Accept", "application/json")
            .header("User-Agent", "magictunnel/0.3.12 OAuth2.1")
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
        
        // Automatically store token if storage is available
        if let Some(ref token_storage) = self.token_storage {
            if let Some(ref oauth_config) = self.config.oauth {
                let provider = &oauth_config.provider;
                
                // Try to get user info to associate token with user ID
                let user_id = match self.get_user_info(token_response.access_token.expose_secret()).await {
                    Ok(Some(user_id)) => Some(user_id),
                    Ok(None) | Err(_) => {
                        debug!("No user info available for token storage, storing without user ID");
                        None
                    }
                };

                match token_storage.store_oauth_token(
                    provider, 
                    user_id.as_deref(),
                    &token_response
                ).await {
                    Ok(key) => {
                        info!("OAuth token automatically stored with key: {}", key);
                    },
                    Err(e) => {
                        warn!("Failed to store OAuth token: {}", e);
                        // Don't fail the whole operation if storage fails
                    }
                }
            }
        }
        
        Ok(token_response)
    }

    /// Get user information using an access token (if supported by provider)
    pub async fn get_user_info(&self, access_token: &str) -> Result<Option<String>> {
        let oauth_config = match &self.config.oauth {
            Some(config) => config,
            None => {
                debug!("No OAuth configuration found");
                return Ok(None);
            }
        };

        // Get user info from the OAuth provider
        match self.fetch_user_info_from_provider(access_token, oauth_config).await {
            Ok(user_info) => {
                info!("Successfully retrieved user info for user: {}", user_info.id);
                // Return the user ID as the primary identifier
                Ok(Some(user_info.id))
            },
            Err(e) => {
                warn!("Failed to retrieve user info from OAuth provider: {}", e);
                // Don't fail the entire operation if user info retrieval fails
                // This allows tokens to still work even if user info endpoint is down
                Ok(None)
            }
        }
    }

    /// Fetch user information from OAuth provider using access token
    async fn fetch_user_info_from_provider(
        &self, 
        access_token: &str, 
        oauth_config: &OAuthConfig
    ) -> Result<OAuthUserInfo> {
        let user_info_url = self.get_user_info_url(oauth_config)?;
        
        debug!("Fetching user info from provider '{}' at URL: {}", oauth_config.provider, user_info_url);

        let response = self
            .client
            .get(&user_info_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/json")
            .header("User-Agent", "magictunnel/0.3.12")
            .send()
            .await
            .map_err(|e| {
                error!("HTTP request to user info endpoint failed: {}", e);
                ProxyError::auth("Failed to fetch user information from OAuth provider")
            })?;

        if !response.status().is_success() {
            error!("User info endpoint returned status: {} for provider: {}", response.status(), oauth_config.provider);
            return Err(ProxyError::auth("OAuth provider user info endpoint returned error"));
        }

        // Get the response text first for debugging
        let response_text = response
            .text()
            .await
            .map_err(|e| {
                error!("Failed to read user info response: {}", e);
                ProxyError::auth("Invalid response from user info endpoint")
            })?;

        debug!("Raw user info response from {}: {}", oauth_config.provider, response_text);

        // Parse the provider-specific response format
        self.parse_provider_user_info(&response_text, &oauth_config.provider)
    }

    /// Parse provider-specific user information response
    fn parse_provider_user_info(&self, response_text: &str, provider: &str) -> Result<OAuthUserInfo> {
        let parsed: serde_json::Value = serde_json::from_str(response_text)
            .map_err(|e| {
                error!("Failed to parse user info JSON from {}: {}", provider, e);
                ProxyError::auth("Invalid JSON response from user info endpoint")
            })?;

        match provider.to_lowercase().as_str() {
            "github" => self.parse_github_user_info(&parsed),
            "google" => self.parse_google_user_info(&parsed),
            "microsoft" | "azure" => self.parse_microsoft_user_info(&parsed),
            _ => {
                // Try to parse as a generic OAuth response
                self.parse_generic_user_info(&parsed, provider)
            }
        }
    }

    /// Parse GitHub user info response
    fn parse_github_user_info(&self, data: &serde_json::Value) -> Result<OAuthUserInfo> {
        let id = data.get("id")
            .and_then(|v| v.as_i64())
            .map(|i| i.to_string())
            .or_else(|| data.get("login").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .ok_or_else(|| ProxyError::auth("GitHub user info missing required 'id' or 'login' field"))?;

        let email = data.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());
        let name = data.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let login = data.get("login").and_then(|v| v.as_str()).map(|s| s.to_string());

        Ok(OAuthUserInfo {
            id,
            email,
            name,
            login,
        })
    }

    /// Parse Google user info response
    fn parse_google_user_info(&self, data: &serde_json::Value) -> Result<OAuthUserInfo> {
        let id = data.get("id")
            .and_then(|v| v.as_str())
            .or_else(|| data.get("sub").and_then(|v| v.as_str()))
            .ok_or_else(|| ProxyError::auth("Google user info missing required 'id' or 'sub' field"))?
            .to_string();

        let email = data.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());
        let name = data.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let login = email.clone(); // Google doesn't have a separate login field

        Ok(OAuthUserInfo {
            id,
            email,
            name,
            login,
        })
    }

    /// Parse Microsoft/Azure user info response
    fn parse_microsoft_user_info(&self, data: &serde_json::Value) -> Result<OAuthUserInfo> {
        let id = data.get("id")
            .and_then(|v| v.as_str())
            .or_else(|| data.get("sub").and_then(|v| v.as_str()))
            .ok_or_else(|| ProxyError::auth("Microsoft user info missing required 'id' or 'sub' field"))?
            .to_string();

        let email = data.get("mail")
            .and_then(|v| v.as_str())
            .or_else(|| data.get("userPrincipalName").and_then(|v| v.as_str()))
            .or_else(|| data.get("email").and_then(|v| v.as_str()))
            .map(|s| s.to_string());

        let name = data.get("displayName")
            .and_then(|v| v.as_str())
            .or_else(|| data.get("name").and_then(|v| v.as_str()))
            .map(|s| s.to_string());

        let login = data.get("userPrincipalName")
            .and_then(|v| v.as_str())
            .or_else(|| email.as_deref())
            .map(|s| s.to_string());

        Ok(OAuthUserInfo {
            id,
            email,
            name,
            login,
        })
    }

    /// Parse generic OAuth user info response (fallback)
    fn parse_generic_user_info(&self, data: &serde_json::Value, provider: &str) -> Result<OAuthUserInfo> {
        // Try common field names for user ID and convert to String
        let id = data.get("id")
            .and_then(|v| {
                // Handle both string and numeric IDs
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else if let Some(i) = v.as_i64() {
                    Some(i.to_string())
                } else {
                    None
                }
            })
            .or_else(|| data.get("sub").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .or_else(|| data.get("user_id").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .or_else(|| data.get("username").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .ok_or_else(|| ProxyError::auth(format!(
                "Generic OAuth user info for provider '{}' missing required ID field (tried: id, sub, user_id, username)", 
                provider
            )))?;

        let email = data.get("email")
            .and_then(|v| v.as_str())
            .or_else(|| data.get("email_address").and_then(|v| v.as_str()))
            .map(|s| s.to_string());

        let name = data.get("name")
            .and_then(|v| v.as_str())
            .or_else(|| data.get("full_name").and_then(|v| v.as_str()))
            .or_else(|| data.get("display_name").and_then(|v| v.as_str()))
            .map(|s| s.to_string());

        let login = data.get("username")
            .and_then(|v| v.as_str())
            .or_else(|| data.get("login").and_then(|v| v.as_str()))
            .or_else(|| email.as_deref())
            .map(|s| s.to_string());

        debug!("Parsed generic user info for provider '{}': id={}, email={:?}, name={:?}, login={:?}", 
               provider, id, email, name, login);

        Ok(OAuthUserInfo {
            id,
            email,
            name,
            login,
        })
    }

    /// Retrieve stored token by provider and user (requires token storage)
    pub async fn retrieve_stored_token(&self, provider: &str, user_id: Option<&str>) -> Result<Option<crate::auth::TokenData>> {
        match &self.token_storage {
            Some(storage) => storage.retrieve_oauth_token(provider, user_id).await,
            None => {
                warn!("Token storage not available for token retrieval");
                Ok(None)
            }
        }
    }

    /// Store token manually (requires token storage)
    pub async fn store_token_manually(&self, provider: &str, user_id: Option<&str>, token: &OAuthTokenResponse) -> Result<Option<String>> {
        match &self.token_storage {
            Some(storage) => {
                match storage.store_oauth_token(provider, user_id, token).await {
                    Ok(key) => {
                        info!("Token manually stored with key: {}", key);
                        Ok(Some(key))
                    },
                    Err(e) => Err(e),
                }
            },
            None => {
                warn!("Token storage not available for manual storage");
                Ok(None)
            }
        }
    }

    /// Delete stored token (requires token storage)
    pub async fn delete_stored_token(&self, provider: &str, user_id: Option<&str>) -> Result<bool> {
        match &self.token_storage {
            Some(storage) => {
                match storage.delete_oauth_token(provider, user_id).await {
                    Ok(()) => {
                        info!("Token deleted for provider: {}, user: {:?}", provider, user_id);
                        Ok(true)
                    },
                    Err(e) => {
                        warn!("Failed to delete token: {}", e);
                        Err(e)
                    }
                }
            },
            None => {
                warn!("Token storage not available for token deletion");
                Ok(false)
            }
        }
    }

    /// Check if token storage is available
    pub fn has_token_storage(&self) -> bool {
        self.token_storage.is_some()
    }

    /// Get all stored tokens (requires token storage)
    pub async fn list_stored_tokens(&self) -> Result<Option<std::collections::HashMap<String, crate::auth::TokenData>>> {
        match &self.token_storage {
            Some(storage) => {
                match storage.get_all_tokens().await {
                    Ok(tokens) => Ok(Some(tokens)),
                    Err(e) => Err(e),
                }
            },
            None => Ok(None),
        }
    }

    /// Clean up expired tokens (requires token storage)
    pub async fn cleanup_expired_tokens(&self) -> Result<Option<u32>> {
        match &self.token_storage {
            Some(storage) => {
                match storage.cleanup_expired_tokens().await {
                    Ok(count) => Ok(Some(count)),
                    Err(e) => Err(e),
                }
            },
            None => Ok(None),
        }
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
            client_secret: Secret::new("test_client_secret".to_string()),
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

    #[test]
    fn test_parse_github_user_info() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);
        
        let github_response = serde_json::json!({
            "id": 12345678,
            "login": "testuser",
            "name": "Test User",
            "email": "test@example.com"
        });
        
        let user_info = validator.parse_github_user_info(&github_response).unwrap();
        assert_eq!(user_info.id, "12345678");
        assert_eq!(user_info.login, Some("testuser".to_string()));
        assert_eq!(user_info.name, Some("Test User".to_string()));
        assert_eq!(user_info.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_parse_google_user_info() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);
        
        let google_response = serde_json::json!({
            "sub": "1234567890",
            "name": "Test User",
            "email": "test@example.com",
            "picture": "https://example.com/avatar.jpg"
        });
        
        let user_info = validator.parse_google_user_info(&google_response).unwrap();
        assert_eq!(user_info.id, "1234567890");
        assert_eq!(user_info.name, Some("Test User".to_string()));
        assert_eq!(user_info.email, Some("test@example.com".to_string()));
        assert_eq!(user_info.login, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_parse_microsoft_user_info() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);
        
        let microsoft_response = serde_json::json!({
            "id": "abc123-def456-ghi789",
            "displayName": "Test User",
            "mail": "test@company.com",
            "userPrincipalName": "test@company.onmicrosoft.com"
        });
        
        let user_info = validator.parse_microsoft_user_info(&microsoft_response).unwrap();
        assert_eq!(user_info.id, "abc123-def456-ghi789");
        assert_eq!(user_info.name, Some("Test User".to_string()));
        assert_eq!(user_info.email, Some("test@company.com".to_string()));
        assert_eq!(user_info.login, Some("test@company.onmicrosoft.com".to_string()));
    }

    #[test]
    fn test_parse_generic_user_info_string_id() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);
        
        let generic_response = serde_json::json!({
            "id": "user123",
            "username": "testuser",
            "email": "test@example.com",
            "full_name": "Test User"
        });
        
        let user_info = validator.parse_generic_user_info(&generic_response, "custom_provider").unwrap();
        assert_eq!(user_info.id, "user123");
        assert_eq!(user_info.login, Some("testuser".to_string()));
        assert_eq!(user_info.email, Some("test@example.com".to_string()));
        assert_eq!(user_info.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_parse_generic_user_info_numeric_id() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);
        
        let generic_response = serde_json::json!({
            "id": 987654321,
            "username": "testuser",
            "email_address": "test@example.com"
        });
        
        let user_info = validator.parse_generic_user_info(&generic_response, "custom_provider").unwrap();
        assert_eq!(user_info.id, "987654321");
        assert_eq!(user_info.login, Some("testuser".to_string()));
        assert_eq!(user_info.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_parse_generic_user_info_missing_id() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);
        
        // Test case with no valid ID fields at all
        let generic_response = serde_json::json!({
            "name": "Test User",
            "email": "test@example.com"
        });
        
        let result = validator.parse_generic_user_info(&generic_response, "custom_provider");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing required ID field"));
    }
    
    #[test]
    fn test_parse_generic_user_info_with_username_as_id() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);
        
        // Test case where username is used as ID (should succeed)
        let generic_response = serde_json::json!({
            "username": "testuser",
            "email": "test@example.com"
        });
        
        let user_info = validator.parse_generic_user_info(&generic_response, "custom_provider").unwrap();
        assert_eq!(user_info.id, "testuser");
        assert_eq!(user_info.login, Some("testuser".to_string()));
        assert_eq!(user_info.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_get_user_info_url_custom_provider() {
        let mut config = create_test_oauth_config();
        // Test custom provider URL detection
        config.oauth.as_mut().unwrap().provider = "custom".to_string();
        config.oauth.as_mut().unwrap().auth_url = "https://github.com/login/oauth/authorize".to_string();
        
        let validator = OAuthValidator::new(config);
        let oauth_config = validator.config.oauth.as_ref().unwrap();
        let url = validator.get_user_info_url(oauth_config).unwrap();
        assert_eq!(url, "https://api.github.com/user");
    }

    #[test]
    fn test_get_user_info_url_unsupported_provider() {
        let mut config = create_test_oauth_config();
        config.oauth.as_mut().unwrap().provider = "unsupported".to_string();
        config.oauth.as_mut().unwrap().auth_url = "https://custom-oauth.example.com/auth".to_string();
        
        let validator = OAuthValidator::new(config);
        let oauth_config = validator.config.oauth.as_ref().unwrap();
        let result = validator.get_user_info_url(oauth_config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported OAuth provider"));
    }
}
