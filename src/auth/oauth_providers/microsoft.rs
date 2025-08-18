//! Microsoft OAuth Provider Implementation
//! 
//! Implements OAuth 2.1 and OIDC support for Microsoft Azure AD with Microsoft Graph API scopes.

use super::{OAuthProvider, TokenSet, UserInfo, AuthorizationUrl, TokenValidation, ProviderFeature, utils};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use secrecy::{Secret, ExposeSecret};
use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// Microsoft provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftConfig {
    /// Azure AD tenant ID (or "common" for multi-tenant)
    pub tenant_id: String,
    /// Microsoft OAuth client ID (Application ID)
    pub client_id: String,
    /// Microsoft OAuth client secret (Application Secret)
    #[serde(with = "crate::config::secret_string")]
    pub client_secret: Secret<String>,
    /// Default scopes to request
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// Microsoft Graph API version
    #[serde(default = "default_graph_version")]
    pub graph_version: String,
    /// Prompt behavior (login, consent, select_account, none)
    #[serde(default = "default_prompt")]
    pub prompt: String,
    /// Response mode (query, fragment, form_post)
    #[serde(default = "default_response_mode")]
    pub response_mode: String,
    /// Domain hint for faster authentication
    pub domain_hint: Option<String>,
}

fn default_scopes() -> Vec<String> {
    vec![
        "openid".to_string(),
        "profile".to_string(),
        "email".to_string(),
        "https://graph.microsoft.com/User.Read".to_string(),
    ]
}

fn default_graph_version() -> String {
    "v1.0".to_string()
}

fn default_prompt() -> String {
    "select_account".to_string()
}

fn default_response_mode() -> String {
    "query".to_string()
}

/// Microsoft OIDC discovery document
#[derive(Debug, Deserialize)]
struct MicrosoftDiscovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: Option<String>,
    jwks_uri: String,
    end_session_endpoint: Option<String>,
    scopes_supported: Option<Vec<String>>,
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
}

/// Microsoft OAuth provider
pub struct MicrosoftProvider {
    config: MicrosoftConfig,
    http_client: reqwest::Client,
    discovery: MicrosoftDiscovery,
}

impl MicrosoftProvider {
    /// Create new Microsoft provider with auto-discovery
    pub async fn new(config: MicrosoftConfig, http_client: reqwest::Client) -> Result<Self> {
        let discovery_url = format!(
            "https://login.microsoftonline.com/{}/v2.0/.well-known/openid-configuration",
            config.tenant_id
        );
        
        debug!("Discovering Microsoft endpoints from: {}", discovery_url);
        
        let discovery: MicrosoftDiscovery = http_client
            .get(&discovery_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
            
        debug!("Microsoft discovery successful for tenant: {}", config.tenant_id);
        
        Ok(Self {
            config,
            http_client,
            discovery,
        })
    }
    
    /// Get Microsoft Graph API base URL
    fn get_graph_api_base_url(&self) -> String {
        format!("https://graph.microsoft.com/{}", self.config.graph_version)
    }
}

#[async_trait]
impl OAuthProvider for MicrosoftProvider {
    fn provider_id(&self) -> &str {
        "microsoft"
    }
    
    fn provider_name(&self) -> &str {
        "Microsoft"
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
            ("prompt", &self.config.prompt),
            ("response_mode", &self.config.response_mode),
        ];
        
        // Add domain hint if specified
        if let Some(domain_hint) = &self.config.domain_hint {
            query_params.push(("domain_hint", domain_hint.as_str()));
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
            return Err(anyhow!("Microsoft token exchange failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Microsoft response"))?
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
            return Err(anyhow!("Microsoft token refresh failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Microsoft refresh response"))?
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
        // Use Microsoft Graph API to get user information
        let response = self.http_client
            .get(&format!("{}/me", self.get_graph_api_base_url()))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Microsoft Graph userinfo request failed: {}", error_text));
        }
        
        let user_data: serde_json::Value = response.json().await?;
        
        // Extract Microsoft-specific claims
        let mut additional_claims = HashMap::new();
        if let Some(obj) = user_data.as_object() {
            for (key, value) in obj {
                if !matches!(key.as_str(), "id" | "mail" | "displayName" | "userPrincipalName" | "givenName" | "surname") {
                    additional_claims.insert(key.clone(), value.clone());
                }
            }
        }
        
        Ok(UserInfo {
            id: user_data["id"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing user ID in Microsoft response"))?
                .to_string(),
            email: user_data["mail"].as_str()
                .or_else(|| user_data["userPrincipalName"].as_str())
                .map(|s| s.to_string()),
            name: user_data["displayName"].as_str()
                .map(|s| s.to_string())
                .or_else(|| {
                    let given = user_data["givenName"].as_str();
                    let family = user_data["surname"].as_str();
                    match (given, family) {
                        (Some(g), Some(f)) => Some(format!("{} {}", g, f)),
                        (Some(g), None) => Some(g.to_string()),
                        (None, Some(f)) => Some(f.to_string()),
                        _ => None,
                    }
                }),
            picture: None, // Microsoft Graph doesn't return picture in basic profile
            username: user_data["userPrincipalName"].as_str().map(|s| s.to_string()),
            email_verified: Some(true), // Microsoft emails are always verified
            additional_claims,
        })
    }
    
    async fn validate_token(&self, access_token: &str) -> Result<TokenValidation> {
        // Microsoft doesn't have a standard token validation endpoint
        // We validate by making a Graph API request
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
        // Microsoft uses the logout endpoint for token revocation
        if let Some(end_session_endpoint) = &self.discovery.end_session_endpoint {
            let params = vec![
                ("token", access_token),
                ("token_type_hint", "access_token"),
            ];
            
            let response = self.http_client
                .post(end_session_endpoint)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .form(&params)
                .send()
                .await?;
                
            if !response.status().is_success() {
                warn!("Microsoft token revocation failed with status: {}", response.status());
                let error_text = response.text().await?;
                return Err(anyhow!("Microsoft token revocation failed: {}", error_text));
            }
            
            debug!("Microsoft token revoked successfully");
        } else {
            warn!("Microsoft token revocation not supported (no end_session endpoint)");
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
        
        // Microsoft Graph API scopes
        scopes.insert("https://graph.microsoft.com/User.Read".to_string(), "Read user profile".to_string());
        scopes.insert("https://graph.microsoft.com/User.ReadWrite".to_string(), "Read and write user profile".to_string());
        scopes.insert("https://graph.microsoft.com/User.ReadBasic.All".to_string(), "Read basic profiles of all users".to_string());
        scopes.insert("https://graph.microsoft.com/User.Read.All".to_string(), "Read all user profiles".to_string());
        scopes.insert("https://graph.microsoft.com/User.ReadWrite.All".to_string(), "Read and write all user profiles".to_string());
        scopes.insert("https://graph.microsoft.com/Mail.Read".to_string(), "Read user mail".to_string());
        scopes.insert("https://graph.microsoft.com/Mail.ReadWrite".to_string(), "Read and write user mail".to_string());
        scopes.insert("https://graph.microsoft.com/Mail.Send".to_string(), "Send mail as user".to_string());
        scopes.insert("https://graph.microsoft.com/Calendars.Read".to_string(), "Read user calendars".to_string());
        scopes.insert("https://graph.microsoft.com/Calendars.ReadWrite".to_string(), "Read and write user calendars".to_string());
        scopes.insert("https://graph.microsoft.com/Contacts.Read".to_string(), "Read user contacts".to_string());
        scopes.insert("https://graph.microsoft.com/Contacts.ReadWrite".to_string(), "Read and write user contacts".to_string());
        scopes.insert("https://graph.microsoft.com/Files.Read".to_string(), "Read user files".to_string());
        scopes.insert("https://graph.microsoft.com/Files.ReadWrite".to_string(), "Read and write user files".to_string());
        scopes.insert("https://graph.microsoft.com/Files.Read.All".to_string(), "Read all files user can access".to_string());
        scopes.insert("https://graph.microsoft.com/Files.ReadWrite.All".to_string(), "Read and write all files user can access".to_string());
        scopes.insert("https://graph.microsoft.com/Sites.Read.All".to_string(), "Read all SharePoint sites".to_string());
        scopes.insert("https://graph.microsoft.com/Sites.ReadWrite.All".to_string(), "Read and write all SharePoint sites".to_string());
        scopes.insert("https://graph.microsoft.com/Directory.Read.All".to_string(), "Read directory data".to_string());
        scopes.insert("https://graph.microsoft.com/Directory.ReadWrite.All".to_string(), "Read and write directory data".to_string());
        scopes.insert("https://graph.microsoft.com/Group.Read.All".to_string(), "Read all groups".to_string());
        scopes.insert("https://graph.microsoft.com/Group.ReadWrite.All".to_string(), "Read and write all groups".to_string());
        scopes.insert("https://graph.microsoft.com/TeamSettings.Read.All".to_string(), "Read Microsoft Teams settings".to_string());
        scopes.insert("https://graph.microsoft.com/TeamSettings.ReadWrite.All".to_string(), "Read and write Microsoft Teams settings".to_string());
        
        // Add discovered scopes if available
        if let Some(supported_scopes) = &self.discovery.scopes_supported {
            for scope in supported_scopes {
                if !scopes.contains_key(scope) {
                    scopes.insert(scope.clone(), format!("Microsoft scope: {}", scope));
                }
            }
        }
        
        scopes
    }
    
    fn supports_feature(&self, feature: ProviderFeature) -> bool {
        match feature {
            ProviderFeature::Pkce => true,
            ProviderFeature::RefreshTokens => true,
            ProviderFeature::TokenRevocation => self.discovery.end_session_endpoint.is_some(),
            ProviderFeature::OpenIdConnect => true,
            ProviderFeature::DynamicRegistration => false, // Microsoft requires manual app registration
            ProviderFeature::DeviceCodeFlow => true, // Microsoft supports device flow
            ProviderFeature::Organizations => true, // Azure AD tenants
            ProviderFeature::Webhooks => true, // Microsoft Graph has webhook support
            ProviderFeature::UserMetadata => true, // Microsoft Graph supports extension attributes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;
    
    fn create_test_config() -> MicrosoftConfig {
        MicrosoftConfig {
            tenant_id: "common".to_string(),
            client_id: "test_client_id".to_string(),
            client_secret: Secret::new("test_client_secret".to_string()),
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            graph_version: "v1.0".to_string(),
            prompt: "select_account".to_string(),
            response_mode: "query".to_string(),
            domain_hint: Some("contoso.com".to_string()),
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: MicrosoftConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.tenant_id, deserialized.tenant_id);
        assert_eq!(config.client_id, deserialized.client_id);
        assert_eq!(config.scopes, deserialized.scopes);
        assert_eq!(config.domain_hint, deserialized.domain_hint);
    }
    
    #[test]
    fn test_default_values() {
        let scopes = default_scopes();
        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"https://graph.microsoft.com/User.Read".to_string()));
        assert_eq!(default_graph_version(), "v1.0");
        assert_eq!(default_prompt(), "select_account");
        assert_eq!(default_response_mode(), "query");
    }
}