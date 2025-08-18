//! Keycloak OAuth Provider Implementation
//! 
//! Implements OAuth 2.1 and OIDC support for Keycloak with realm and role management.

use super::{OAuthProvider, TokenSet, UserInfo, AuthorizationUrl, TokenValidation, ProviderFeature, utils};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use secrecy::{Secret, ExposeSecret};
use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// Keycloak provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeycloakConfig {
    /// Keycloak server URL (e.g., "https://keycloak.example.com")
    pub server_url: String,
    /// Keycloak realm name
    pub realm: String,
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    #[serde(with = "crate::config::secret_string")]
    pub client_secret: Secret<String>,
    /// Default scopes to request
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// Enable role mapping
    #[serde(default = "default_true")]
    pub enable_role_mapping: bool,
    /// Admin client for enhanced features (optional)
    pub admin_client_id: Option<String>,
    /// Admin client secret for enhanced features (optional)
    #[serde(with = "crate::auth::oauth::option_secret_string")]
    pub admin_client_secret: Option<Secret<String>>,
}

fn default_scopes() -> Vec<String> {
    vec!["openid".to_string(), "profile".to_string(), "email".to_string(), "roles".to_string()]
}

fn default_true() -> bool {
    true
}

/// Keycloak OIDC discovery document
#[derive(Debug, Deserialize)]
struct KeycloakDiscovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    jwks_uri: String,
    end_session_endpoint: Option<String>,
    revocation_endpoint: Option<String>,
    introspection_endpoint: Option<String>,
    registration_endpoint: Option<String>,
    scopes_supported: Option<Vec<String>>,
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
}

/// Keycloak OAuth provider
pub struct KeycloakProvider {
    config: KeycloakConfig,
    http_client: reqwest::Client,
    discovery: KeycloakDiscovery,
}

impl KeycloakProvider {
    /// Create new Keycloak provider with auto-discovery
    pub async fn new(config: KeycloakConfig, http_client: reqwest::Client) -> Result<Self> {
        let discovery_url = format!("{}/realms/{}/.well-known/openid-configuration", 
            config.server_url, config.realm);
        
        debug!("Discovering Keycloak endpoints from: {}", discovery_url);
        
        let discovery: KeycloakDiscovery = http_client
            .get(&discovery_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
            
        debug!("Keycloak discovery successful for realm: {}", config.realm);
        
        Ok(Self {
            config,
            http_client,
            discovery,
        })
    }
    
    /// Get Keycloak admin API base URL
    fn get_admin_api_base_url(&self) -> String {
        format!("{}/admin/realms/{}", self.config.server_url, self.config.realm)
    }
    
    /// Get admin access token for enhanced API access
    async fn get_admin_token(&self) -> Result<String> {
        if let (Some(admin_client_id), Some(admin_client_secret)) = 
            (&self.config.admin_client_id, &self.config.admin_client_secret) {
            
            let params = vec![
                ("grant_type", "client_credentials"),
                ("client_id", admin_client_id.as_str()),
                ("client_secret", admin_client_secret.expose_secret()),
            ];
            
            let response = self.http_client
                .post(&self.discovery.token_endpoint)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .form(&params)
                .send()
                .await?;
                
            if response.status().is_success() {
                let token_response: serde_json::Value = response.json().await?;
                return Ok(token_response["access_token"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing access_token in admin token response"))?
                    .to_string());
            }
        }
        
        Err(anyhow!("Admin credentials not configured or invalid"))
    }
}

#[async_trait]
impl OAuthProvider for KeycloakProvider {
    fn provider_id(&self) -> &str {
        "keycloak"
    }
    
    fn provider_name(&self) -> &str {
        "Keycloak"
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
            return Err(anyhow!("Keycloak token exchange failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Keycloak response"))?
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
            return Err(anyhow!("Keycloak token refresh failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Keycloak refresh response"))?
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
        let response = self.http_client
            .get(&self.discovery.userinfo_endpoint)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Keycloak userinfo request failed: {}", error_text));
        }
        
        let user_data: serde_json::Value = response.json().await?;
        
        // Extract Keycloak-specific claims including roles
        let mut additional_claims = HashMap::new();
        if let Some(obj) = user_data.as_object() {
            for (key, value) in obj {
                if !matches!(key.as_str(), "sub" | "email" | "name" | "picture" | "preferred_username" | "email_verified") {
                    additional_claims.insert(key.clone(), value.clone());
                }
            }
        }
        
        Ok(UserInfo {
            id: user_data["sub"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing 'sub' in Keycloak userinfo"))?
                .to_string(),
            email: user_data["email"].as_str().map(|s| s.to_string()),
            name: user_data["name"].as_str()
                .or_else(|| user_data["given_name"].as_str())
                .map(|s| s.to_string()),
            picture: user_data["picture"].as_str().map(|s| s.to_string()),
            username: user_data["preferred_username"].as_str().map(|s| s.to_string()),
            email_verified: user_data["email_verified"].as_bool(),
            additional_claims,
        })
    }
    
    async fn validate_token(&self, access_token: &str) -> Result<TokenValidation> {
        if let Some(introspection_endpoint) = &self.discovery.introspection_endpoint {
            let params = vec![
                ("token", access_token),
                ("client_id", &self.config.client_id),
                ("client_secret", self.config.client_secret.expose_secret()),
            ];
            
            let response = self.http_client
                .post(introspection_endpoint)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .form(&params)
                .send()
                .await?;
                
            if response.status().is_success() {
                let introspection_data: serde_json::Value = response.json().await?;
                
                let active = introspection_data["active"].as_bool().unwrap_or(false);
                
                if active {
                    return Ok(TokenValidation {
                        valid: true,
                        expires_at: introspection_data["exp"]
                            .as_u64()
                            .map(|ts| chrono::DateTime::from_timestamp(ts as i64, 0))
                            .flatten()
                            .map(|dt| dt.with_timezone(&chrono::Utc)),
                        scopes: introspection_data["scope"]
                            .as_str()
                            .map(|s| utils::parse_scopes(s))
                            .unwrap_or_default(),
                        user_id: introspection_data["sub"].as_str().map(|s| s.to_string()),
                        client_id: introspection_data["client_id"].as_str().map(|s| s.to_string()),
                        metadata: introspection_data
                            .as_object()
                            .unwrap_or(&serde_json::Map::new())
                            .clone()
                            .into_iter()
                            .collect(),
                    });
                }
            }
        }
        
        // Fallback to userinfo endpoint
        match self.get_user_info(access_token).await {
            Ok(user_info) => Ok(TokenValidation {
                valid: true,
                expires_at: None,
                scopes: vec![],
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
                warn!("Keycloak token revocation failed with status: {}", response.status());
                let error_text = response.text().await?;
                return Err(anyhow!("Keycloak token revocation failed: {}", error_text));
            }
            
            debug!("Keycloak token revoked successfully");
        } else if let Some(end_session_endpoint) = &self.discovery.end_session_endpoint {
            // Use logout endpoint as fallback
            let params = vec![
                ("id_token_hint", access_token),
            ];
            
            let response = self.http_client
                .post(end_session_endpoint)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .form(&params)
                .send()
                .await?;
                
            if !response.status().is_success() {
                warn!("Keycloak session end failed with status: {}", response.status());
            } else {
                debug!("Keycloak session ended successfully");
            }
        } else {
            warn!("Keycloak token revocation not supported (no revocation endpoint)");
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
        
        // Keycloak-specific scopes
        scopes.insert("roles".to_string(), "Access to user roles".to_string());
        scopes.insert("web-origins".to_string(), "Access to web origins".to_string());
        scopes.insert("microprofile-jwt".to_string(), "MicroProfile JWT support".to_string());
        scopes.insert("acr".to_string(), "Authentication Context Class Reference".to_string());
        
        // Add discovered scopes if available
        if let Some(supported_scopes) = &self.discovery.scopes_supported {
            for scope in supported_scopes {
                if !scopes.contains_key(scope) {
                    scopes.insert(scope.clone(), format!("Keycloak scope: {}", scope));
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
            ProviderFeature::DeviceCodeFlow => true, // Keycloak supports device flow
            ProviderFeature::Organizations => false, // Not a core Keycloak feature
            ProviderFeature::Webhooks => true, // Keycloak has event listeners
            ProviderFeature::UserMetadata => true, // Keycloak has user attributes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;
    
    fn create_test_config() -> KeycloakConfig {
        KeycloakConfig {
            server_url: "https://keycloak.example.com".to_string(),
            realm: "test-realm".to_string(),
            client_id: "test_client_id".to_string(),
            client_secret: Secret::new("test_client_secret".to_string()),
            scopes: vec!["openid".to_string(), "profile".to_string(), "roles".to_string()],
            enable_role_mapping: true,
            admin_client_id: Some("admin-client".to_string()),
            admin_client_secret: Some(Secret::new("admin-secret".to_string())),
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: KeycloakConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.server_url, deserialized.server_url);
        assert_eq!(config.realm, deserialized.realm);
        assert_eq!(config.client_id, deserialized.client_id);
        assert_eq!(config.enable_role_mapping, deserialized.enable_role_mapping);
    }
    
    #[test]
    fn test_default_values() {
        let scopes = default_scopes();
        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"roles".to_string()));
        assert!(default_true());
    }
}