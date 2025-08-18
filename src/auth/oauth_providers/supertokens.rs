//! SuperTokens OAuth Provider Implementation
//! 
//! Implements OAuth 2.1 and OIDC support for SuperTokens with recipe-based authentication.

use super::{OAuthProvider, TokenSet, UserInfo, AuthorizationUrl, TokenValidation, ProviderFeature, utils};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use secrecy::{Secret, ExposeSecret};
use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// SuperTokens provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperTokensConfig {
    /// SuperTokens app domain (e.g., "your-app.supertokens.io")
    pub app_domain: String,
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    #[serde(with = "crate::config::secret_string")]
    pub client_secret: Secret<String>,
    /// Default scopes to request
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// SuperTokens recipe type
    #[serde(default = "default_recipe")]
    pub recipe: String,
    /// API base path
    #[serde(default = "default_api_base_path")]
    pub api_base_path: String,
    /// Website base path  
    #[serde(default = "default_website_base_path")]
    pub website_base_path: String,
}

fn default_scopes() -> Vec<String> {
    vec!["openid".to_string(), "profile".to_string(), "email".to_string()]
}

fn default_recipe() -> String {
    "thirdparty".to_string()
}

fn default_api_base_path() -> String {
    "/auth".to_string()
}

fn default_website_base_path() -> String {
    "/auth".to_string()
}

/// SuperTokens OIDC discovery document
#[derive(Debug, Deserialize)]
struct SuperTokensDiscovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    jwks_uri: String,
    revocation_endpoint: Option<String>,
    scopes_supported: Option<Vec<String>>,
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
}

/// SuperTokens OAuth provider
pub struct SuperTokensProvider {
    config: SuperTokensConfig,
    http_client: reqwest::Client,
    discovery: SuperTokensDiscovery,
}

impl SuperTokensProvider {
    /// Create new SuperTokens provider with auto-discovery
    pub async fn new(config: SuperTokensConfig, http_client: reqwest::Client) -> Result<Self> {
        let discovery_url = format!("{}{}/.well-known/openid-configuration", 
            config.app_domain, config.api_base_path);
        
        debug!("Discovering SuperTokens endpoints from: {}", discovery_url);
        
        let discovery: SuperTokensDiscovery = http_client
            .get(&discovery_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
            
        debug!("SuperTokens discovery successful for domain: {}", config.app_domain);
        
        Ok(Self {
            config,
            http_client,
            discovery,
        })
    }
    
    /// Get SuperTokens API base URL
    fn get_api_base_url(&self) -> String {
        format!("{}{}", self.config.app_domain, self.config.api_base_path)
    }
}

#[async_trait]
impl OAuthProvider for SuperTokensProvider {
    fn provider_id(&self) -> &str {
        "supertokens"
    }
    
    fn provider_name(&self) -> &str {
        "SuperTokens"
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
            return Err(anyhow!("SuperTokens token exchange failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in SuperTokens response"))?
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
            return Err(anyhow!("SuperTokens token refresh failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in SuperTokens refresh response"))?
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
        // Try SuperTokens API first for enhanced user info
        let api_response = self.http_client
            .get(&format!("{}/user", self.get_api_base_url()))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await;
            
        let user_data: serde_json::Value = if let Ok(resp) = api_response {
            if resp.status().is_success() {
                resp.json().await?
            } else {
                // Fallback to standard OIDC userinfo endpoint
                let response = self.http_client
                    .get(&self.discovery.userinfo_endpoint)
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await?;
                    
                if !response.status().is_success() {
                    let error_text = response.text().await?;
                    return Err(anyhow!("SuperTokens userinfo request failed: {}", error_text));
                }
                
                response.json().await?
            }
        } else {
            // Fallback to standard OIDC userinfo endpoint
            let response = self.http_client
                .get(&self.discovery.userinfo_endpoint)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;
                
            if !response.status().is_success() {
                let error_text = response.text().await?;
                return Err(anyhow!("SuperTokens userinfo request failed: {}", error_text));
            }
            
            response.json().await?
        };
        
        // Extract SuperTokens-specific claims
        let mut additional_claims = HashMap::new();
        if let Some(obj) = user_data.as_object() {
            for (key, value) in obj {
                if !matches!(key.as_str(), "sub" | "email" | "name" | "picture" | "username" | "email_verified" | "user_id") {
                    additional_claims.insert(key.clone(), value.clone());
                }
            }
        }
        
        Ok(UserInfo {
            id: user_data["sub"]
                .as_str()
                .or_else(|| user_data["user_id"].as_str())
                .or_else(|| user_data["userId"].as_str())
                .ok_or_else(|| anyhow!("Missing user ID in SuperTokens response"))?
                .to_string(),
            email: user_data["email"].as_str().map(|s| s.to_string()),
            name: user_data["name"].as_str().map(|s| s.to_string()),
            picture: user_data["picture"].as_str().map(|s| s.to_string()),
            username: user_data["username"].as_str()
                .or_else(|| user_data["nickname"].as_str())
                .map(|s| s.to_string()),
            email_verified: user_data["email_verified"].as_bool()
                .or_else(|| user_data["emailVerified"].as_bool()),
            additional_claims,
        })
    }
    
    async fn validate_token(&self, access_token: &str) -> Result<TokenValidation> {
        // SuperTokens token verification
        let response = self.http_client
            .post(&format!("{}/session/verify", self.get_api_base_url()))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await;
            
        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(session_data) = resp.json::<serde_json::Value>().await {
                    Ok(TokenValidation {
                        valid: true,
                        expires_at: session_data["expiry"]
                            .as_u64()
                            .map(|ts| chrono::DateTime::from_timestamp(ts as i64 / 1000, 0))
                            .flatten()
                            .map(|dt| dt.with_timezone(&chrono::Utc)),
                        scopes: vec![], // SuperTokens doesn't expose scopes in verification
                        user_id: session_data["userId"]
                            .as_str()
                            .or_else(|| session_data["user_id"].as_str())
                            .map(|s| s.to_string()),
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
                warn!("SuperTokens token revocation failed with status: {}", response.status());
            } else {
                debug!("SuperTokens token revoked successfully");
            }
        } else {
            // Try SuperTokens session signout
            let response = self.http_client
                .post(&format!("{}/signout", self.get_api_base_url()))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;
                
            if !response.status().is_success() {
                warn!("SuperTokens session signout failed with status: {}", response.status());
            } else {
                debug!("SuperTokens session signed out successfully");
            }
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
        
        // SuperTokens-specific scopes based on recipe
        match self.config.recipe.as_str() {
            "emailpassword" => {
                scopes.insert("email:write".to_string(), "Update email address".to_string());
                scopes.insert("password:write".to_string(), "Change password".to_string());
            }
            "thirdparty" => {
                scopes.insert("provider:read".to_string(), "Access third-party provider info".to_string());
            }
            "passwordless" => {
                scopes.insert("passwordless:send".to_string(), "Send passwordless login codes".to_string());
            }
            _ => {}
        }
        
        // Add discovered scopes if available
        if let Some(supported_scopes) = &self.discovery.scopes_supported {
            for scope in supported_scopes {
                if !scopes.contains_key(scope) {
                    scopes.insert(scope.clone(), format!("SuperTokens scope: {}", scope));
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
            ProviderFeature::DynamicRegistration => false, // Not supported by SuperTokens
            ProviderFeature::DeviceCodeFlow => false, // Not supported by SuperTokens  
            ProviderFeature::Organizations => false, // Not a core SuperTokens feature
            ProviderFeature::Webhooks => true, // SuperTokens has webhook support
            ProviderFeature::UserMetadata => true, // SuperTokens supports user metadata
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;
    
    fn create_test_config() -> SuperTokensConfig {
        SuperTokensConfig {
            app_domain: "https://your-app.supertokens.io".to_string(),
            client_id: "test_client_id".to_string(),
            client_secret: Secret::new("test_client_secret".to_string()),
            scopes: vec!["openid".to_string(), "profile".to_string()],
            recipe: "thirdparty".to_string(),
            api_base_path: "/auth".to_string(),
            website_base_path: "/auth".to_string(),
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: SuperTokensConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.app_domain, deserialized.app_domain);
        assert_eq!(config.client_id, deserialized.client_id);
        assert_eq!(config.recipe, deserialized.recipe);
    }
    
    #[test]
    fn test_default_values() {
        assert_eq!(default_recipe(), "thirdparty");
        assert_eq!(default_api_base_path(), "/auth");
        assert_eq!(default_website_base_path(), "/auth");
    }
}