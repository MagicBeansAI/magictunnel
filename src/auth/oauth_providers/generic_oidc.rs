//! Generic OIDC Provider Implementation
//! 
//! Implements OAuth 2.1 and OIDC support for any standards-compliant OIDC provider.

use super::{OAuthProvider, TokenSet, UserInfo, AuthorizationUrl, TokenValidation, ProviderFeature, utils};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use secrecy::{Secret, ExposeSecret};
use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// Generic OIDC provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericOidcConfig {
    /// Provider name for identification
    pub provider_name: String,
    /// OIDC issuer URL (e.g., "https://accounts.google.com")
    pub issuer_url: String,
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    #[serde(with = "crate::config::secret_string")]
    pub client_secret: Secret<String>,
    /// Default scopes to request
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// Custom discovery URL override (optional)
    pub discovery_url: Option<String>,
    /// Custom audience parameter (optional)
    pub audience: Option<String>,
    /// Additional authorization parameters
    #[serde(default)]
    pub additional_auth_params: HashMap<String, String>,
}

fn default_scopes() -> Vec<String> {
    vec!["openid".to_string(), "profile".to_string(), "email".to_string()]
}

/// OIDC discovery document
#[derive(Debug, Deserialize)]
struct OidcDiscovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    jwks_uri: String,
    revocation_endpoint: Option<String>,
    introspection_endpoint: Option<String>,
    registration_endpoint: Option<String>,
    end_session_endpoint: Option<String>,
    scopes_supported: Option<Vec<String>>,
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
    subject_types_supported: Vec<String>,
    id_token_signing_alg_values_supported: Vec<String>,
}

/// Generic OIDC provider
pub struct GenericOidcProvider {
    config: GenericOidcConfig,
    http_client: reqwest::Client,
    discovery: OidcDiscovery,
}

impl GenericOidcProvider {
    /// Create new generic OIDC provider with auto-discovery
    pub async fn new(config: GenericOidcConfig, http_client: reqwest::Client) -> Result<Self> {
        let discovery_url = if let Some(discovery_url) = &config.discovery_url {
            discovery_url.clone()
        } else {
            format!("{}/.well-known/openid-configuration", config.issuer_url.trim_end_matches('/'))
        };
        
        debug!("Discovering OIDC endpoints from: {}", discovery_url);
        
        let discovery: OidcDiscovery = http_client
            .get(&discovery_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
            
        debug!("OIDC discovery successful for issuer: {}", discovery.issuer);
        
        Ok(Self {
            config,
            http_client,
            discovery,
        })
    }
    
    /// Get provider ID based on issuer
    fn get_provider_id(&self) -> String {
        if self.config.issuer_url.contains("accounts.google.com") {
            "google".to_string()
        } else if self.config.issuer_url.contains("login.microsoftonline.com") {
            "microsoft".to_string()
        } else if self.config.issuer_url.contains("id.twitch.tv") {
            "twitch".to_string()
        } else if self.config.issuer_url.contains("appleid.apple.com") {
            "apple".to_string()
        } else {
            // Generate ID from domain
            if let Ok(url) = Url::parse(&self.config.issuer_url) {
                if let Some(domain) = url.host_str() {
                    domain.replace('.', "_").replace('-', "_")
                } else {
                    "generic_oidc".to_string()
                }
            } else {
                "generic_oidc".to_string()
            }
        }
    }
}

#[async_trait]
impl OAuthProvider for GenericOidcProvider {
    fn provider_id(&self) -> &str {
        // Return static string for trait, but use get_provider_id() internally
        "generic_oidc"
    }
    
    fn provider_name(&self) -> &str {
        &self.config.provider_name
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
        
        // Add custom audience if specified
        if let Some(audience) = &self.config.audience {
            query_params.push(("audience", audience.as_str()));
        }
        
        // Add additional auth parameters
        for (key, value) in &self.config.additional_auth_params {
            query_params.push((key.as_str(), value.as_str()));
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
            return Err(anyhow!("OIDC token exchange failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in OIDC response"))?
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
            return Err(anyhow!("OIDC token refresh failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in OIDC refresh response"))?
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
            return Err(anyhow!("OIDC userinfo request failed: {}", error_text));
        }
        
        let user_data: serde_json::Value = response.json().await?;
        
        // Extract standard OIDC claims
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
                .ok_or_else(|| anyhow!("Missing 'sub' in OIDC userinfo"))?
                .to_string(),
            email: user_data["email"].as_str().map(|s| s.to_string()),
            name: user_data["name"].as_str()
                .or_else(|| user_data["given_name"].as_str())
                .map(|s| s.to_string()),
            picture: user_data["picture"].as_str().map(|s| s.to_string()),
            username: user_data["preferred_username"].as_str()
                .or_else(|| user_data["nickname"].as_str())
                .map(|s| s.to_string()),
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
                warn!("OIDC token revocation failed with status: {}", response.status());
                let error_text = response.text().await?;
                return Err(anyhow!("OIDC token revocation failed: {}", error_text));
            }
            
            debug!("OIDC token revoked successfully");
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
                warn!("OIDC session end failed with status: {}", response.status());
            } else {
                debug!("OIDC session ended successfully");
            }
        } else {
            warn!("OIDC token revocation not supported (no revocation endpoint)");
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
        scopes.insert("address".to_string(), "Access to user's address".to_string());
        scopes.insert("phone".to_string(), "Access to user's phone number".to_string());
        
        // Add discovered scopes if available
        if let Some(supported_scopes) = &self.discovery.scopes_supported {
            for scope in supported_scopes {
                if !scopes.contains_key(scope) {
                    scopes.insert(scope.clone(), format!("Provider scope: {}", scope));
                }
            }
        }
        
        // Provider-specific scopes based on issuer
        if self.config.issuer_url.contains("accounts.google.com") {
            scopes.insert("https://www.googleapis.com/auth/userinfo.profile".to_string(), "Google profile access".to_string());
            scopes.insert("https://www.googleapis.com/auth/userinfo.email".to_string(), "Google email access".to_string());
            scopes.insert("https://www.googleapis.com/auth/drive".to_string(), "Google Drive access".to_string());
            scopes.insert("https://www.googleapis.com/auth/calendar".to_string(), "Google Calendar access".to_string());
        } else if self.config.issuer_url.contains("login.microsoftonline.com") {
            scopes.insert("https://graph.microsoft.com/User.Read".to_string(), "Microsoft Graph user read".to_string());
            scopes.insert("https://graph.microsoft.com/Mail.Read".to_string(), "Microsoft Graph mail read".to_string());
            scopes.insert("https://graph.microsoft.com/Calendars.Read".to_string(), "Microsoft Graph calendar read".to_string());
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
            ProviderFeature::DeviceCodeFlow => false, // Not standardized in OIDC discovery
            ProviderFeature::Organizations => false, // Provider-specific
            ProviderFeature::Webhooks => false, // Provider-specific
            ProviderFeature::UserMetadata => false, // Provider-specific
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;
    
    fn create_test_config() -> GenericOidcConfig {
        GenericOidcConfig {
            provider_name: "Test OIDC Provider".to_string(),
            issuer_url: "https://accounts.google.com".to_string(),
            client_id: "test_client_id".to_string(),
            client_secret: Secret::new("test_client_secret".to_string()),
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            discovery_url: None,
            audience: None,
            additional_auth_params: HashMap::new(),
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: GenericOidcConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.provider_name, deserialized.provider_name);
        assert_eq!(config.issuer_url, deserialized.issuer_url);
        assert_eq!(config.client_id, deserialized.client_id);
        assert_eq!(config.scopes, deserialized.scopes);
    }
    
    #[test]
    fn test_default_scopes() {
        let scopes = default_scopes();
        assert_eq!(scopes.len(), 3);
        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"profile".to_string()));
        assert!(scopes.contains(&"email".to_string()));
    }
}