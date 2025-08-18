//! Clerk OAuth Provider Implementation
//! 
//! Implements OAuth 2.1 and OIDC support for Clerk with organization and session management.

use super::{OAuthProvider, TokenSet, UserInfo, AuthorizationUrl, TokenValidation, ProviderFeature, utils};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use secrecy::{Secret, ExposeSecret};
use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// Clerk provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClerkConfig {
    /// Clerk domain (e.g., "prepared-mule-23.clerk.accounts.dev")
    pub domain: String,
    /// Clerk publishable key (client ID)
    pub client_id: String,
    /// Clerk secret key (client secret)
    #[serde(with = "crate::config::secret_string")]
    pub client_secret: Secret<String>,
    /// Default scopes to request
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// Enable organization features
    #[serde(default)]
    pub enable_organizations: bool,
    /// Enable session management
    #[serde(default = "default_true")]
    pub enable_sessions: bool,
    /// Custom Clerk API version
    #[serde(default = "default_api_version")]
    pub api_version: String,
}

fn default_scopes() -> Vec<String> {
    vec!["openid".to_string(), "profile".to_string(), "email".to_string()]
}

fn default_true() -> bool {
    true
}

fn default_api_version() -> String {
    "v1".to_string()
}

/// Clerk OIDC discovery document
#[derive(Debug, Deserialize)]
struct ClerkDiscovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    jwks_uri: String,
    scopes_supported: Option<Vec<String>>,
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
}

/// Clerk OAuth provider
pub struct ClerkProvider {
    config: ClerkConfig,
    http_client: reqwest::Client,
    discovery: ClerkDiscovery,
}

impl ClerkProvider {
    /// Create new Clerk provider with auto-discovery
    pub async fn new(config: ClerkConfig, http_client: reqwest::Client) -> Result<Self> {
        let discovery_url = format!("https://{}/.well-known/openid-configuration", config.domain);
        
        debug!("Discovering Clerk endpoints from: {}", discovery_url);
        
        let discovery: ClerkDiscovery = http_client
            .get(&discovery_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
            
        debug!("Clerk discovery successful for domain: {}", config.domain);
        
        Ok(Self {
            config,
            http_client,
            discovery,
        })
    }
    
    /// Get Clerk API base URL
    fn get_api_base_url(&self) -> String {
        format!("https://api.clerk.com/{}", self.config.api_version)
    }
}

#[async_trait]
impl OAuthProvider for ClerkProvider {
    fn provider_id(&self) -> &str {
        "clerk"
    }
    
    fn provider_name(&self) -> &str {
        "Clerk"
    }
    
    async fn get_authorization_url(
        &self,
        scopes: &[String],
        redirect_uri: &str,
    ) -> Result<AuthorizationUrl> {
        let state = utils::generate_state();
        let (code_verifier, code_challenge) = utils::generate_pkce();
        
        let mut all_scopes = self.config.scopes.clone();
        all_scopes.extend_from_slice(scopes);
        all_scopes.sort();
        all_scopes.dedup();
        
        let mut url = Url::parse(&self.discovery.authorization_endpoint)?;
        
        let scopes_str = utils::join_scopes(&all_scopes);
        let query_params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", redirect_uri),
            ("scope", &scopes_str),
            ("state", &state),
            ("code_challenge", &code_challenge),
            ("code_challenge_method", "S256"),
        ];
        
        url.query_pairs_mut().extend_pairs(query_params);
        
        Ok(AuthorizationUrl {
            url,
            state,
            code_verifier: Some(code_verifier),
        })
    }
    
    async fn exchange_code_for_token(
        &self,
        code: &str,
        redirect_uri: &str,
        _state: &str,
        code_verifier: Option<&str>,
    ) -> Result<TokenSet> {
        let mut params = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.config.client_id),
            ("client_secret", self.config.client_secret.expose_secret()),
        ];
        
        if let Some(verifier) = code_verifier {
            params.push(("code_verifier", verifier));
        }
        
        let response = self.http_client
            .post(&self.discovery.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Clerk token exchange failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Clerk response"))?
                .to_string(),
            token_type: token_response["token_type"]
                .as_str()
                .unwrap_or("Bearer")
                .to_string(),
            expires_in: token_response["expires_in"].as_u64(),
            refresh_token: token_response["refresh_token"]
                .as_str()
                .map(|s| s.to_string()),
            scope: token_response["scope"]
                .as_str()
                .map(|s| s.to_string()),
            additional_data: token_response
                .as_object()
                .unwrap_or(&serde_json::Map::new())
                .clone()
                .into_iter()
                .collect(),
        })
    }
    
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenSet> {
        let params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
            ("client_secret", self.config.client_secret.expose_secret()),
        ];
        
        let response = self.http_client
            .post(&self.discovery.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Clerk token refresh failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Clerk refresh response"))?
                .to_string(),
            token_type: token_response["token_type"]
                .as_str()
                .unwrap_or("Bearer")
                .to_string(),
            expires_in: token_response["expires_in"].as_u64(),
            refresh_token: token_response["refresh_token"]
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| Some(refresh_token.to_string())),
            scope: token_response["scope"]
                .as_str()
                .map(|s| s.to_string()),
            additional_data: token_response
                .as_object()
                .unwrap_or(&serde_json::Map::new())
                .clone()
                .into_iter()
                .collect(),
        })
    }
    
    async fn get_user_info(&self, access_token: &str) -> Result<UserInfo> {
        // Clerk provides enhanced user info through their API
        let response = self.http_client
            .get(&format!("{}/me", self.get_api_base_url()))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;
            
        if !response.status().is_success() {
            // Fallback to standard OIDC userinfo endpoint
            let response = self.http_client
                .get(&self.discovery.userinfo_endpoint)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;
                
            if !response.status().is_success() {
                let error_text = response.text().await?;
                return Err(anyhow!("Clerk userinfo request failed: {}", error_text));
            }
        }
        
        let user_data: serde_json::Value = response.json().await?;
        
        // Extract Clerk-specific claims
        let mut additional_claims = HashMap::new();
        if let Some(obj) = user_data.as_object() {
            for (key, value) in obj {
                if !matches!(key.as_str(), "sub" | "email" | "name" | "picture" | "username" | "email_verified") {
                    additional_claims.insert(key.clone(), value.clone());
                }
            }
        }
        
        Ok(UserInfo {
            id: user_data["sub"]
                .as_str()
                .or_else(|| user_data["id"].as_str())
                .ok_or_else(|| anyhow!("Missing user ID in Clerk response"))?
                .to_string(),
            email: user_data["email"].as_str()
                .or_else(|| user_data["email_addresses"].as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|email| email["email_address"].as_str()))
                .map(|s| s.to_string()),
            name: user_data["name"].as_str()
                .map(|s| s.to_string())
                .or_else(|| {
                    let first = user_data["first_name"].as_str()?;
                    let last = user_data["last_name"].as_str()?;
                    Some(format!("{} {}", first, last))
                }),
            picture: user_data["picture"].as_str()
                .or_else(|| user_data["image_url"].as_str())
                .map(|s| s.to_string()),
            username: user_data["username"].as_str().map(|s| s.to_string()),
            email_verified: user_data["email_verified"].as_bool()
                .or_else(|| {
                    user_data["email_addresses"].as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|email| email["verification"]["status"].as_str())
                        .map(|status| status == "verified")
                }),
            additional_claims,
        })
    }
    
    async fn validate_token(&self, access_token: &str) -> Result<TokenValidation> {
        // Clerk has a token verification endpoint
        let response = self.http_client
            .get(&format!("{}/sessions/verify", self.get_api_base_url()))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await;
            
        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(session_data) = resp.json::<serde_json::Value>().await {
                    Ok(TokenValidation {
                        valid: true,
                        expires_at: session_data["expire_at"]
                            .as_str()
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&chrono::Utc)),
                        scopes: vec![], // Clerk doesn't expose scopes in verification
                        user_id: session_data["user_id"].as_str().map(|s| s.to_string()),
                        client_id: Some(self.config.client_id.clone()),
                        metadata: session_data
                            .as_object()
                            .unwrap_or(&serde_json::Map::new())
                            .clone()
                            .into_iter()
                            .collect(),
                    })
                } else {
                    Ok(TokenValidation {
                        valid: true,
                        expires_at: None,
                        scopes: vec![],
                        user_id: None,
                        client_id: Some(self.config.client_id.clone()),
                        metadata: HashMap::new(),
                    })
                }
            }
            _ => Ok(TokenValidation {
                valid: false,
                expires_at: None,
                scopes: vec![],
                user_id: None,
                client_id: None,
                metadata: HashMap::new(),
            }),
        }
    }
    
    async fn revoke_token(&self, access_token: &str) -> Result<()> {
        // Clerk doesn't have a standard revocation endpoint, but we can invalidate sessions
        let response = self.http_client
            .post(&format!("{}/sessions/revoke", self.get_api_base_url()))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;
            
        if !response.status().is_success() {
            warn!("Clerk session revocation failed with status: {}", response.status());
            // Don't fail hard, as the token might still expire naturally
        } else {
            debug!("Clerk session revoked successfully");
        }
        
        Ok(())
    }
    
    fn get_available_scopes(&self) -> HashMap<String, String> {
        let mut scopes = HashMap::new();
        
        // Standard OIDC scopes
        scopes.insert("openid".to_string(), "OpenID Connect authentication".to_string());
        scopes.insert("profile".to_string(), "Access to basic profile information".to_string());
        scopes.insert("email".to_string(), "Access to email address".to_string());
        
        // Clerk-specific scopes
        scopes.insert("organizations".to_string(), "Access to user's organizations".to_string());
        scopes.insert("sessions".to_string(), "Access to session management".to_string());
        scopes.insert("user_metadata".to_string(), "Access to user metadata".to_string());
        
        // Add discovered scopes if available
        if let Some(supported_scopes) = &self.discovery.scopes_supported {
            for scope in supported_scopes {
                if !scopes.contains_key(scope) {
                    scopes.insert(scope.clone(), format!("Clerk scope: {}", scope));
                }
            }
        }
        
        scopes
    }
    
    fn supports_feature(&self, feature: ProviderFeature) -> bool {
        match feature {
            ProviderFeature::Pkce => true,
            ProviderFeature::RefreshTokens => true,
            ProviderFeature::TokenRevocation => true, // Via session management
            ProviderFeature::OpenIdConnect => true,
            ProviderFeature::DynamicRegistration => false, // Not supported by Clerk
            ProviderFeature::DeviceCodeFlow => false, // Not supported by Clerk
            ProviderFeature::Organizations => self.config.enable_organizations,
            ProviderFeature::Webhooks => true, // Clerk has webhook support
            ProviderFeature::UserMetadata => true, // Clerk has rich metadata support
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;
    
    fn create_test_config() -> ClerkConfig {
        ClerkConfig {
            domain: "prepared-mule-23.clerk.accounts.dev".to_string(),
            client_id: "pk_test_123".to_string(),
            client_secret: Secret::new("sk_test_456".to_string()),
            scopes: vec!["openid".to_string(), "profile".to_string()],
            enable_organizations: true,
            enable_sessions: true,
            api_version: "v1".to_string(),
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: ClerkConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.domain, deserialized.domain);
        assert_eq!(config.client_id, deserialized.client_id);
        assert_eq!(config.enable_organizations, deserialized.enable_organizations);
    }
    
    #[test]
    fn test_default_values() {
        let scopes = default_scopes();
        assert_eq!(scopes.len(), 3);
        assert!(default_true());
        assert_eq!(default_api_version(), "v1");
    }
}