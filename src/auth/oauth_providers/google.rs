//! Google OAuth Provider Implementation
//! 
//! Implements OAuth 2.1 and OIDC support for Google with Google-specific APIs and scopes.

use super::{OAuthProvider, TokenSet, UserInfo, AuthorizationUrl, TokenValidation, ProviderFeature, utils};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use secrecy::{Secret, ExposeSecret};
use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// Google provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleConfig {
    /// Google OAuth client ID
    pub client_id: String,
    /// Google OAuth client secret
    #[serde(with = "crate::config::secret_string")]
    pub client_secret: Secret<String>,
    /// Default scopes to request
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// Google Workspace domain restriction (optional)
    pub hosted_domain: Option<String>,
    /// Enable offline access (refresh tokens)
    #[serde(default = "default_true")]
    pub enable_offline_access: bool,
    /// Prompt behavior (consent, select_account, none)
    #[serde(default = "default_prompt")]
    pub prompt: String,
    /// Access type (offline or online)
    #[serde(default = "default_access_type")]
    pub access_type: String,
}

fn default_scopes() -> Vec<String> {
    vec![
        "openid".to_string(),
        "profile".to_string(), 
        "email".to_string(),
        "https://www.googleapis.com/auth/userinfo.profile".to_string(),
        "https://www.googleapis.com/auth/userinfo.email".to_string(),
    ]
}

fn default_true() -> bool {
    true
}

fn default_prompt() -> String {
    "consent".to_string()
}

fn default_access_type() -> String {
    "offline".to_string()
}

/// Google OIDC discovery document
#[derive(Debug, Deserialize)]
struct GoogleDiscovery {
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

/// Google OAuth provider
pub struct GoogleProvider {
    config: GoogleConfig,
    http_client: reqwest::Client,
    discovery: GoogleDiscovery,
}

impl GoogleProvider {
    /// Create new Google provider with auto-discovery
    pub async fn new(config: GoogleConfig, http_client: reqwest::Client) -> Result<Self> {
        let discovery_url = "https://accounts.google.com/.well-known/openid-configuration";
        
        debug!("Discovering Google endpoints from: {}", discovery_url);
        
        let discovery: GoogleDiscovery = http_client
            .get(discovery_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
            
        debug!("Google discovery successful");
        
        Ok(Self {
            config,
            http_client,
            discovery,
        })
    }
}

#[async_trait]
impl OAuthProvider for GoogleProvider {
    fn provider_id(&self) -> &str {
        "google"
    }
    
    fn provider_name(&self) -> &str {
        "Google"
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
            ("access_type", &self.config.access_type),
            ("prompt", &self.config.prompt),
        ];
        
        // Add Google Workspace domain restriction
        if let Some(hosted_domain) = &self.config.hosted_domain {
            query_params.push(("hd", hosted_domain.as_str()));
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
        
        let response = self.http_client
            .post(&self.discovery.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Google token exchange failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Google response"))?
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
            return Err(anyhow!("Google token refresh failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Google refresh response"))?
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
            return Err(anyhow!("Google userinfo request failed: {}", error_text));
        }
        
        let user_data: serde_json::Value = response.json().await?;
        
        // Extract Google-specific claims
        let mut additional_claims = HashMap::new();
        if let Some(obj) = user_data.as_object() {
            for (key, value) in obj {
                if !matches!(key.as_str(), "sub" | "email" | "name" | "picture" | "given_name" | "family_name" | "email_verified" | "hd") {
                    additional_claims.insert(key.clone(), value.clone());
                }
            }
        }
        
        Ok(UserInfo {
            id: user_data["sub"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing 'sub' in Google userinfo"))?
                .to_string(),
            email: user_data["email"].as_str().map(|s| s.to_string()),
            name: user_data["name"].as_str()
                .map(|s| s.to_string())
                .or_else(|| {
                    let given = user_data["given_name"].as_str();
                    let family = user_data["family_name"].as_str();
                    match (given, family) {
                        (Some(g), Some(f)) => Some(format!("{} {}", g, f)),
                        (Some(g), None) => Some(g.to_string()),
                        (None, Some(f)) => Some(f.to_string()),
                        _ => None,
                    }
                }),
            picture: user_data["picture"].as_str().map(|s| s.to_string()),
            username: None, // Google doesn't provide username
            email_verified: user_data["email_verified"].as_bool(),
            additional_claims,
        })
    }
    
    async fn validate_token(&self, access_token: &str) -> Result<TokenValidation> {
        // Google token info endpoint
        let response = self.http_client
            .get("https://www.googleapis.com/oauth2/v1/tokeninfo")
            .query(&[("access_token", access_token)])
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Ok(TokenValidation {
                valid: false,
                expires_at: None,
                scopes: vec![],
                user_id: None,
                client_id: None,
                metadata: HashMap::new(),
            });
        }
        
        let token_info: serde_json::Value = response.json().await?;
        
        Ok(TokenValidation {
            valid: true,
            expires_at: token_info["expires_in"]
                .as_u64()
                .map(|seconds| chrono::Utc::now() + chrono::Duration::seconds(seconds as i64)),
            scopes: token_info["scope"]
                .as_str()
                .map(|s| utils::parse_scopes(s))
                .unwrap_or_default(),
            user_id: token_info["user_id"].as_str().map(|s| s.to_string()),
            client_id: token_info["audience"].as_str().map(|s| s.to_string()),
            metadata: token_info
                .as_object()
                .unwrap_or(&serde_json::Map::new())
                .clone()
                .into_iter()
                .collect(),
        })
    }
    
    async fn revoke_token(&self, access_token: &str) -> Result<()> {
        if let Some(revocation_endpoint) = &self.discovery.revocation_endpoint {
            let params = vec![("token", access_token)];
            
            let response = self.http_client
                .post(revocation_endpoint)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .form(&params)
                .send()
                .await?;
                
            if !response.status().is_success() {
                warn!("Google token revocation failed with status: {}", response.status());
                let error_text = response.text().await?;
                return Err(anyhow!("Google token revocation failed: {}", error_text));
            }
            
            debug!("Google token revoked successfully");
        } else {
            warn!("Google token revocation not supported (no revocation endpoint)");
        }
        
        Ok(())
    }
    
    fn get_available_scopes(&self) -> HashMap<String, String> {
        let mut scopes = HashMap::new();
        
        // Standard OIDC scopes
        scopes.insert("openid".to_string(), "OpenID Connect authentication".to_string());
        scopes.insert("profile".to_string(), "Access to basic profile information".to_string());
        scopes.insert("email".to_string(), "Access to email address".to_string());
        
        // Google API scopes
        scopes.insert("https://www.googleapis.com/auth/userinfo.profile".to_string(), "Google profile information".to_string());
        scopes.insert("https://www.googleapis.com/auth/userinfo.email".to_string(), "Google email address".to_string());
        scopes.insert("https://www.googleapis.com/auth/drive".to_string(), "Google Drive access".to_string());
        scopes.insert("https://www.googleapis.com/auth/drive.readonly".to_string(), "Google Drive read-only access".to_string());
        scopes.insert("https://www.googleapis.com/auth/drive.file".to_string(), "Google Drive file access".to_string());
        scopes.insert("https://www.googleapis.com/auth/calendar".to_string(), "Google Calendar access".to_string());
        scopes.insert("https://www.googleapis.com/auth/calendar.readonly".to_string(), "Google Calendar read-only access".to_string());
        scopes.insert("https://www.googleapis.com/auth/gmail.readonly".to_string(), "Gmail read-only access".to_string());
        scopes.insert("https://www.googleapis.com/auth/gmail.modify".to_string(), "Gmail modify access".to_string());
        scopes.insert("https://www.googleapis.com/auth/gmail.compose".to_string(), "Gmail compose access".to_string());
        scopes.insert("https://www.googleapis.com/auth/spreadsheets".to_string(), "Google Sheets access".to_string());
        scopes.insert("https://www.googleapis.com/auth/spreadsheets.readonly".to_string(), "Google Sheets read-only access".to_string());
        scopes.insert("https://www.googleapis.com/auth/documents".to_string(), "Google Docs access".to_string());
        scopes.insert("https://www.googleapis.com/auth/documents.readonly".to_string(), "Google Docs read-only access".to_string());
        scopes.insert("https://www.googleapis.com/auth/presentations".to_string(), "Google Slides access".to_string());
        scopes.insert("https://www.googleapis.com/auth/presentations.readonly".to_string(), "Google Slides read-only access".to_string());
        scopes.insert("https://www.googleapis.com/auth/youtube".to_string(), "YouTube access".to_string());
        scopes.insert("https://www.googleapis.com/auth/youtube.readonly".to_string(), "YouTube read-only access".to_string());
        scopes.insert("https://www.googleapis.com/auth/cloud-platform".to_string(), "Google Cloud Platform access".to_string());
        scopes.insert("https://www.googleapis.com/auth/cloud-platform.read-only".to_string(), "Google Cloud Platform read-only access".to_string());
        
        // Add discovered scopes if available
        if let Some(supported_scopes) = &self.discovery.scopes_supported {
            for scope in supported_scopes {
                if !scopes.contains_key(scope) {
                    scopes.insert(scope.clone(), format!("Google scope: {}", scope));
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
            ProviderFeature::DynamicRegistration => false, // Google requires manual app registration
            ProviderFeature::DeviceCodeFlow => true, // Google supports device flow
            ProviderFeature::Organizations => true, // Google Workspace domains
            ProviderFeature::Webhooks => true, // Google has webhook support via Cloud Functions
            ProviderFeature::UserMetadata => false, // No custom user metadata in Google
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;
    
    fn create_test_config() -> GoogleConfig {
        GoogleConfig {
            client_id: "test_client_id.apps.googleusercontent.com".to_string(),
            client_secret: Secret::new("test_client_secret".to_string()),
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            hosted_domain: Some("example.com".to_string()),
            enable_offline_access: true,
            prompt: "consent".to_string(),
            access_type: "offline".to_string(),
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: GoogleConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.client_id, deserialized.client_id);
        assert_eq!(config.scopes, deserialized.scopes);
        assert_eq!(config.hosted_domain, deserialized.hosted_domain);
        assert_eq!(config.enable_offline_access, deserialized.enable_offline_access);
    }
    
    #[test]
    fn test_default_values() {
        let scopes = default_scopes();
        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"https://www.googleapis.com/auth/userinfo.profile".to_string()));
        assert!(default_true());
        assert_eq!(default_prompt(), "consent");
        assert_eq!(default_access_type(), "offline");
    }
}