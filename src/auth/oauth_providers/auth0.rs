//! Auth0 OAuth Provider Implementation
//! 
//! Implements OAuth 2.1 and OIDC support for Auth0 with auto-discovery of endpoints.

use super::{OAuthProvider, TokenSet, UserInfo, AuthorizationUrl, TokenValidation, ProviderFeature, utils};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use secrecy::{Secret, ExposeSecret};
use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// Auth0 provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth0Config {
    /// Auth0 domain (e.g., "your-tenant.auth0.com")
    pub domain: String,
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    #[serde(with = "crate::config::secret_string")]
    pub client_secret: Secret<String>,
    /// Default scopes to request
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// Custom audience (optional)
    pub audience: Option<String>,
    /// Connection to use (optional)
    pub connection: Option<String>,
    /// Custom Auth0 namespace for additional claims
    pub namespace: Option<String>,
}

fn default_scopes() -> Vec<String> {
    vec!["openid".to_string(), "profile".to_string(), "email".to_string()]
}

/// Auth0 OIDC discovery document
#[derive(Debug, Deserialize)]
struct Auth0Discovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    revocation_endpoint: Option<String>,
    registration_endpoint: Option<String>,
    scopes_supported: Option<Vec<String>>,
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
}

/// Auth0 OAuth provider
pub struct Auth0Provider {
    config: Auth0Config,
    http_client: reqwest::Client,
    discovery: Auth0Discovery,
}

impl Auth0Provider {
    /// Create new Auth0 provider with auto-discovery
    pub async fn new(config: Auth0Config, http_client: reqwest::Client) -> Result<Self> {
        let discovery_url = format!("https://{}/.well-known/openid-configuration", config.domain);
        
        debug!("Discovering Auth0 endpoints from: {}", discovery_url);
        
        let discovery: Auth0Discovery = http_client
            .get(&discovery_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
            
        debug!("Auth0 discovery successful for domain: {}", config.domain);
        
        Ok(Self {
            config,
            http_client,
            discovery,
        })
    }
    
    /// Get Auth0 issuer URL
    fn get_issuer(&self) -> String {
        format!("https://{}/", self.config.domain)
    }
}

#[async_trait]
impl OAuthProvider for Auth0Provider {
    fn provider_id(&self) -> &str {
        "auth0"
    }
    
    fn provider_name(&self) -> &str {
        "Auth0"
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
        let mut query_params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", redirect_uri),
            ("scope", &scopes_str),
            ("state", &state),
            ("code_challenge", &code_challenge),
            ("code_challenge_method", "S256"),
        ];
        
        // Add Auth0-specific parameters
        if let Some(audience) = &self.config.audience {
            query_params.push(("audience", audience.as_str()));
        }
        
        if let Some(connection) = &self.config.connection {
            query_params.push(("connection", connection.as_str()));
        }
        
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
        
        if let Some(audience) = &self.config.audience {
            params.push(("audience", audience.as_str()));
        }
        
        let response = self.http_client
            .post(&self.discovery.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Auth0 token exchange failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Auth0 response"))?
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
            return Err(anyhow!("Auth0 token refresh failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Auth0 refresh response"))?
                .to_string(),
            token_type: token_response["token_type"]
                .as_str()
                .unwrap_or("Bearer")
                .to_string(),
            expires_in: token_response["expires_in"].as_u64(),
            refresh_token: token_response["refresh_token"]
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| Some(refresh_token.to_string())), // Keep original if not returned
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
        let response = self.http_client
            .get(&self.discovery.userinfo_endpoint)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Auth0 userinfo request failed: {}", error_text));
        }
        
        let user_data: serde_json::Value = response.json().await?;
        
        // Extract standard claims
        let mut additional_claims = HashMap::new();
        if let Some(obj) = user_data.as_object() {
            for (key, value) in obj {
                if !matches!(key.as_str(), "sub" | "email" | "name" | "picture" | "nickname" | "email_verified") {
                    additional_claims.insert(key.clone(), value.clone());
                }
            }
        }
        
        Ok(UserInfo {
            id: user_data["sub"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing 'sub' in Auth0 userinfo"))?
                .to_string(),
            email: user_data["email"].as_str().map(|s| s.to_string()),
            name: user_data["name"].as_str().map(|s| s.to_string()),
            picture: user_data["picture"].as_str().map(|s| s.to_string()),
            username: user_data["nickname"].as_str().map(|s| s.to_string()),
            email_verified: user_data["email_verified"].as_bool(),
            additional_claims,
        })
    }
    
    async fn validate_token(&self, access_token: &str) -> Result<TokenValidation> {
        // Auth0 doesn't have a standard token introspection endpoint in the free tier
        // We validate by making a userinfo request
        match self.get_user_info(access_token).await {
            Ok(user_info) => Ok(TokenValidation {
                valid: true,
                expires_at: None, // Would need JWT parsing for exact expiry
                scopes: vec![], // Would need JWT parsing for scopes
                user_id: Some(user_info.id),
                client_id: Some(self.config.client_id.clone()),
                metadata: HashMap::new(),
            }),
            Err(_) => Ok(TokenValidation {
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
        if let Some(revocation_endpoint) = &self.discovery.revocation_endpoint {
            let params = vec![
                ("token", access_token),
                ("client_id", &self.config.client_id),
                ("client_secret", self.config.client_secret.expose_secret()),
            ];
            
            let response = self.http_client
                .post(revocation_endpoint)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .form(&params)
                .send()
                .await?;
                
            if !response.status().is_success() {
                warn!("Auth0 token revocation failed with status: {}", response.status());
                let error_text = response.text().await?;
                return Err(anyhow!("Auth0 token revocation failed: {}", error_text));
            }
            
            debug!("Auth0 token revoked successfully");
        } else {
            warn!("Auth0 token revocation not supported (no revocation endpoint)");
        }
        
        Ok(())
    }
    
    fn get_available_scopes(&self) -> HashMap<String, String> {
        let mut scopes = HashMap::new();
        
        // Standard OIDC scopes
        scopes.insert("openid".to_string(), "OpenID Connect authentication".to_string());
        scopes.insert("profile".to_string(), "Access to basic profile information".to_string());
        scopes.insert("email".to_string(), "Access to email address".to_string());
        scopes.insert("offline_access".to_string(), "Access to refresh tokens".to_string());
        
        // Auth0-specific scopes (common ones)
        scopes.insert("read:current_user".to_string(), "Read current user data".to_string());
        scopes.insert("update:current_user_metadata".to_string(), "Update user metadata".to_string());
        scopes.insert("read:user_idp_tokens".to_string(), "Access to identity provider tokens".to_string());
        
        // Add discovered scopes if available
        if let Some(supported_scopes) = &self.discovery.scopes_supported {
            for scope in supported_scopes {
                if !scopes.contains_key(scope) {
                    scopes.insert(scope.clone(), format!("Auth0 scope: {}", scope));
                }
            }
        }
        
        scopes
    }
    
    fn supports_feature(&self, feature: ProviderFeature) -> bool {
        match feature {
            ProviderFeature::Pkce => true,
            ProviderFeature::RefreshTokens => true,
            ProviderFeature::TokenRevocation => self.discovery.revocation_endpoint.is_some(),
            ProviderFeature::OpenIdConnect => true,
            ProviderFeature::DynamicRegistration => self.discovery.registration_endpoint.is_some(),
            ProviderFeature::DeviceCodeFlow => false, // Auth0 supports it but not implemented here
            ProviderFeature::Organizations => true, // Auth0 has organization support
            ProviderFeature::Webhooks => true, // Auth0 Actions and Rules
            ProviderFeature::UserMetadata => true, // Auth0 user_metadata and app_metadata
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;
    
    fn create_test_config() -> Auth0Config {
        Auth0Config {
            domain: "test-tenant.auth0.com".to_string(),
            client_id: "test_client_id".to_string(),
            client_secret: Secret::new("test_client_secret".to_string()),
            scopes: vec!["openid".to_string(), "profile".to_string()],
            audience: Some("https://api.example.com".to_string()),
            connection: None,
            namespace: Some("https://example.com/claims/".to_string()),
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: Auth0Config = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.domain, deserialized.domain);
        assert_eq!(config.client_id, deserialized.client_id);
        assert_eq!(config.scopes, deserialized.scopes);
    }
    
    #[test]
    fn test_default_scopes() {
        let scopes = default_scopes();
        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"profile".to_string()));
        assert!(scopes.contains(&"email".to_string()));
    }
}