//! Modular OAuth Provider System
//! 
//! This module provides a pluggable architecture for OAuth providers, making it easy to add
//! support for new OAuth/OIDC providers without modifying core authentication logic.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use anyhow::Result;

pub mod auth0;
pub mod clerk;
pub mod supertokens;
pub mod keycloak;
pub mod github;
pub mod generic_oidc;
pub mod google;
pub mod microsoft;
pub mod apple;

/// OAuth provider configuration from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OAuthProviderConfig {
    /// Auth0 provider configuration
    Auth0(auth0::Auth0Config),
    /// Clerk provider configuration  
    Clerk(clerk::ClerkConfig),
    /// SuperTokens provider configuration
    SuperTokens(supertokens::SuperTokensConfig),
    /// Keycloak provider configuration
    Keycloak(keycloak::KeycloakConfig),
    /// GitHub provider configuration (existing)
    GitHub(github::GitHubConfig),
    /// Generic OIDC provider configuration
    GenericOidc(generic_oidc::GenericOidcConfig),
    /// Google provider configuration
    Google(google::GoogleConfig),
    /// Microsoft provider configuration
    Microsoft(microsoft::MicrosoftConfig),
    /// Apple provider configuration
    Apple(apple::AppleConfig),
}

/// OAuth token set returned by providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    /// Access token for API calls
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Token expiration in seconds
    pub expires_in: Option<u64>,
    /// Refresh token for token renewal
    pub refresh_token: Option<String>,
    /// Granted scopes
    pub scope: Option<String>,
    /// Additional provider-specific data
    pub additional_data: HashMap<String, serde_json::Value>,
}

/// User information from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// Unique user identifier
    pub id: String,
    /// User email address
    pub email: Option<String>,
    /// User display name
    pub name: Option<String>,
    /// Profile picture URL
    pub picture: Option<String>,
    /// Username/handle
    pub username: Option<String>,
    /// Email verification status
    pub email_verified: Option<bool>,
    /// Provider-specific additional claims
    pub additional_claims: HashMap<String, serde_json::Value>,
}

/// OAuth authorization URL and state
#[derive(Debug, Clone)]
pub struct AuthorizationUrl {
    /// Authorization URL to redirect user to
    pub url: Url,
    /// State parameter for CSRF protection
    pub state: String,
    /// PKCE code verifier (for OAuth 2.1)
    pub code_verifier: Option<String>,
}

/// OAuth provider trait that all providers must implement
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Provider identifier (e.g., "auth0", "clerk", "github")
    fn provider_id(&self) -> &str;
    
    /// Human-readable provider name
    fn provider_name(&self) -> &str;
    
    /// Get the authorization URL for OAuth flow
    async fn get_authorization_url(
        &self,
        scopes: &[String],
        redirect_uri: &str,
    ) -> Result<AuthorizationUrl>;
    
    /// Exchange authorization code for access token
    async fn exchange_code_for_token(
        &self,
        code: &str,
        redirect_uri: &str,
        state: &str,
        code_verifier: Option<&str>,
    ) -> Result<TokenSet>;
    
    /// Refresh an access token using refresh token
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenSet>;
    
    /// Get user information using access token
    async fn get_user_info(&self, access_token: &str) -> Result<UserInfo>;
    
    /// Validate an access token and return token info
    async fn validate_token(&self, access_token: &str) -> Result<TokenValidation>;
    
    /// Revoke an access token
    async fn revoke_token(&self, access_token: &str) -> Result<()>;
    
    /// Get provider-specific scopes with their descriptions
    fn get_available_scopes(&self) -> HashMap<String, String>;
    
    /// Check if provider supports specific features
    fn supports_feature(&self, feature: ProviderFeature) -> bool;
}

/// Token validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidation {
    /// Whether token is valid
    pub valid: bool,
    /// Token expiration timestamp
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Active scopes for this token
    pub scopes: Vec<String>,
    /// User ID associated with token
    pub user_id: Option<String>,
    /// Client ID that issued the token
    pub client_id: Option<String>,
    /// Additional validation info
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Provider-specific features
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProviderFeature {
    /// Supports PKCE (OAuth 2.1)
    Pkce,
    /// Supports refresh tokens
    RefreshTokens,
    /// Supports token revocation
    TokenRevocation,
    /// Supports OIDC (OpenID Connect)
    OpenIdConnect,
    /// Supports dynamic client registration
    DynamicRegistration,
    /// Supports device code flow
    DeviceCodeFlow,
    /// Provider-specific organizations/teams
    Organizations,
    /// Webhook support for real-time updates
    Webhooks,
    /// Custom user metadata
    UserMetadata,
}

/// Provider factory for creating OAuth provider instances
pub struct OAuthProviderFactory {
    http_client: reqwest::Client,
}

impl OAuthProviderFactory {
    /// Create new provider factory
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }
    
    /// Create OAuth provider from configuration
    pub async fn create_provider(
        &self,
        config: OAuthProviderConfig,
    ) -> Result<Box<dyn OAuthProvider>> {
        match config {
            OAuthProviderConfig::Auth0(config) => {
                Ok(Box::new(auth0::Auth0Provider::new(config, self.http_client.clone()).await?))
            }
            OAuthProviderConfig::Clerk(config) => {
                Ok(Box::new(clerk::ClerkProvider::new(config, self.http_client.clone()).await?))
            }
            OAuthProviderConfig::SuperTokens(config) => {
                Ok(Box::new(supertokens::SuperTokensProvider::new(config, self.http_client.clone()).await?))
            }
            OAuthProviderConfig::Keycloak(config) => {
                Ok(Box::new(keycloak::KeycloakProvider::new(config, self.http_client.clone()).await?))
            }
            OAuthProviderConfig::GitHub(config) => {
                Ok(Box::new(github::GitHubProvider::new(config, self.http_client.clone()).await?))
            }
            OAuthProviderConfig::GenericOidc(config) => {
                Ok(Box::new(generic_oidc::GenericOidcProvider::new(config, self.http_client.clone()).await?))
            }
            OAuthProviderConfig::Google(config) => {
                Ok(Box::new(google::GoogleProvider::new(config, self.http_client.clone()).await?))
            }
            OAuthProviderConfig::Microsoft(config) => {
                Ok(Box::new(microsoft::MicrosoftProvider::new(config, self.http_client.clone()).await?))
            }
            OAuthProviderConfig::Apple(config) => {
                Ok(Box::new(apple::AppleProvider::new(config, self.http_client.clone()).await?))
            }
        }
    }
    
    /// Auto-detect provider type from configuration
    pub fn detect_provider_type(&self, issuer: &str) -> Result<String> {
        match issuer {
            url if url.contains("auth0.com") => Ok("auth0".to_string()),
            url if url.contains("clerk.accounts.dev") || url.contains("clerk.com") => Ok("clerk".to_string()),
            url if url.contains("supertokens.io") || url.contains("supertokens.com") => Ok("supertokens".to_string()),
            url if url.contains("keycloak") => Ok("keycloak".to_string()),
            url if url.contains("github.com") => Ok("github".to_string()),
            url if url.contains("accounts.google.com") => Ok("google".to_string()),
            url if url.contains("login.microsoftonline.com") => Ok("microsoft".to_string()),
            url if url.contains("appleid.apple.com") => Ok("apple".to_string()),
            _ => Ok("generic_oidc".to_string()),
        }
    }
}

/// Utility functions for OAuth providers
pub mod utils {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use rand::Rng;
    use sha2::{Sha256, Digest};
    
    /// Generate secure random state parameter
    pub fn generate_state() -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        URL_SAFE_NO_PAD.encode(&random_bytes)
    }
    
    /// Generate PKCE code verifier and challenge
    pub fn generate_pkce() -> (String, String) {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        let code_verifier = URL_SAFE_NO_PAD.encode(&random_bytes);
        
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let challenge_bytes = hasher.finalize();
        let code_challenge = URL_SAFE_NO_PAD.encode(&challenge_bytes);
        
        (code_verifier, code_challenge)
    }
    
    /// Parse scope string into vector
    pub fn parse_scopes(scope_str: &str) -> Vec<String> {
        scope_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }
    
    /// Join scopes into string
    pub fn join_scopes(scopes: &[String]) -> String {
        scopes.join(" ")
    }
    
    /// Add provider prefix to scopes
    pub fn add_provider_prefix(scopes: &[String], provider_id: &str) -> Vec<String> {
        scopes
            .iter()
            .map(|scope| format!("{}:{}", provider_id, scope))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::utils;
    
    #[test]
    fn test_generate_state() {
        let state1 = utils::generate_state();
        let state2 = utils::generate_state();
        
        assert_ne!(state1, state2);
        assert!(state1.len() >= 32);
    }
    
    #[test]
    fn test_generate_pkce() {
        let (verifier1, challenge1) = utils::generate_pkce();
        let (verifier2, challenge2) = utils::generate_pkce();
        
        assert_ne!(verifier1, verifier2);
        assert_ne!(challenge1, challenge2);
        assert!(verifier1.len() >= 32);
        assert!(challenge1.len() >= 32);
    }
    
    #[test]
    fn test_scope_utils() {
        let scopes = vec!["read".to_string(), "write".to_string()];
        let scope_str = utils::join_scopes(&scopes);
        assert_eq!(scope_str, "read write");
        
        let parsed = utils::parse_scopes(&scope_str);
        assert_eq!(parsed, scopes);
        
        let prefixed = utils::add_provider_prefix(&scopes, "github");
        assert_eq!(prefixed, vec!["github:read", "github:write"]);
    }
}