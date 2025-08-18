//! GitHub OAuth Provider Implementation
//! 
//! Implements OAuth 2.1 support for GitHub with repository and organization scopes.

use super::{OAuthProvider, TokenSet, UserInfo, AuthorizationUrl, TokenValidation, ProviderFeature, utils};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use secrecy::{Secret, ExposeSecret};
use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// GitHub provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    /// GitHub OAuth app client ID
    pub client_id: String,
    /// GitHub OAuth app client secret
    #[serde(with = "crate::config::secret_string")]
    pub client_secret: Secret<String>,
    /// Default scopes to request
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// GitHub Enterprise Server URL (optional, defaults to github.com)
    pub enterprise_url: Option<String>,
}

fn default_scopes() -> Vec<String> {
    vec!["user:email".to_string(), "read:user".to_string()]
}

/// GitHub OAuth provider
pub struct GitHubProvider {
    config: GitHubConfig,
    http_client: reqwest::Client,
    base_url: String,
    api_base_url: String,
}

impl GitHubProvider {
    /// Create new GitHub provider
    pub async fn new(config: GitHubConfig, http_client: reqwest::Client) -> Result<Self> {
        let (base_url, api_base_url) = if let Some(enterprise_url) = &config.enterprise_url {
            (
                enterprise_url.clone(),
                format!("{}/api/v3", enterprise_url.trim_end_matches('/'))
            )
        } else {
            (
                "https://github.com".to_string(),
                "https://api.github.com".to_string()
            )
        };
        
        debug!("GitHub provider initialized with base URL: {}", base_url);
        
        Ok(Self {
            config,
            http_client,
            base_url,
            api_base_url,
        })
    }
    
    /// Get GitHub authorization endpoint
    fn get_authorization_endpoint(&self) -> String {
        format!("{}/login/oauth/authorize", self.base_url)
    }
    
    /// Get GitHub token endpoint
    fn get_token_endpoint(&self) -> String {
        format!("{}/login/oauth/access_token", self.base_url)
    }
}

#[async_trait]
impl OAuthProvider for GitHubProvider {
    fn provider_id(&self) -> &str {
        "github"
    }
    
    fn provider_name(&self) -> &str {
        "GitHub"
    }
    
    async fn get_authorization_url(
        &self,
        scopes: &[String],
        redirect_uri: &str,
    ) -> Result<AuthorizationUrl> {
        let state = utils::generate_state();
        
        let mut all_scopes = self.config.scopes.clone();
        all_scopes.extend_from_slice(scopes);
        all_scopes.sort();
        all_scopes.dedup();
        
        let mut url = Url::parse(&self.get_authorization_endpoint())?;
        
        let scopes_str = utils::join_scopes(&all_scopes);
        let query_params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("scope", &scopes_str),
            ("state", &state),
            ("allow_signup", "true"),
        ];
        
        url.query_pairs_mut().extend_pairs(query_params);
        
        Ok(AuthorizationUrl {
            url,
            state,
            code_verifier: None, // GitHub doesn't support PKCE in OAuth apps
        })
    }
    
    async fn exchange_code_for_token(
        &self,
        code: &str,
        redirect_uri: &str,
        _state: &str,
        _code_verifier: Option<&str>,
    ) -> Result<TokenSet> {
        let params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.expose_secret()),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];
        
        let response = self.http_client
            .post(&self.get_token_endpoint())
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("GitHub token exchange failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        if let Some(error) = token_response.get("error") {
            return Err(anyhow!("GitHub OAuth error: {}", error));
        }
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in GitHub response"))?
                .to_string(),
            token_type: token_response["token_type"]
                .as_str()
                .unwrap_or("token")
                .to_string(),
            expires_in: None, // GitHub tokens don't expire
            refresh_token: None, // GitHub doesn't use refresh tokens
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
    
    async fn refresh_token(&self, _refresh_token: &str) -> Result<TokenSet> {
        Err(anyhow!("GitHub does not support refresh tokens"))
    }
    
    async fn get_user_info(&self, access_token: &str) -> Result<UserInfo> {
        let response = self.http_client
            .get(&format!("{}/user", self.api_base_url))
            .header("Authorization", format!("token {}", access_token))
            .header("User-Agent", "MagicTunnel-OAuth")
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("GitHub user info request failed: {}", error_text));
        }
        
        let user_data: serde_json::Value = response.json().await?;
        
        // Get user emails for email verification status
        let emails_response = self.http_client
            .get(&format!("{}/user/emails", self.api_base_url))
            .header("Authorization", format!("token {}", access_token))
            .header("User-Agent", "MagicTunnel-OAuth")
            .send()
            .await;
            
        let primary_email_verified = if let Ok(resp) = emails_response {
            if resp.status().is_success() {
                if let Ok(emails) = resp.json::<serde_json::Value>().await {
                    emails.as_array()
                        .and_then(|arr| arr.iter().find(|email| email["primary"].as_bool() == Some(true)))
                        .and_then(|email| email["verified"].as_bool())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // Extract additional claims
        let mut additional_claims = HashMap::new();
        if let Some(obj) = user_data.as_object() {
            for (key, value) in obj {
                if !matches!(key.as_str(), "id" | "login" | "email" | "name" | "avatar_url") {
                    additional_claims.insert(key.clone(), value.clone());
                }
            }
        }
        
        Ok(UserInfo {
            id: user_data["id"]
                .as_u64()
                .map(|id| id.to_string())
                .ok_or_else(|| anyhow!("Missing user ID in GitHub response"))?,
            email: user_data["email"].as_str().map(|s| s.to_string()),
            name: user_data["name"].as_str().map(|s| s.to_string()),
            picture: user_data["avatar_url"].as_str().map(|s| s.to_string()),
            username: user_data["login"].as_str().map(|s| s.to_string()),
            email_verified: primary_email_verified,
            additional_claims,
        })
    }
    
    async fn validate_token(&self, access_token: &str) -> Result<TokenValidation> {
        // GitHub doesn't have a token introspection endpoint, validate by getting user info
        match self.get_user_info(access_token).await {
            Ok(user_info) => Ok(TokenValidation {
                valid: true,
                expires_at: None, // GitHub tokens don't expire
                scopes: vec![], // Would need to parse from token or store separately
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
        // GitHub uses DELETE /applications/{client_id}/token for token revocation
        let response = self.http_client
            .delete(&format!("{}/applications/{}/token", self.api_base_url, self.config.client_id))
            .header("Authorization", format!("token {}", access_token))
            .header("User-Agent", "MagicTunnel-OAuth")
            .json(&serde_json::json!({
                "access_token": access_token
            }))
            .send()
            .await?;
            
        if !response.status().is_success() {
            warn!("GitHub token revocation failed with status: {}", response.status());
            let error_text = response.text().await?;
            return Err(anyhow!("GitHub token revocation failed: {}", error_text));
        }
        
        debug!("GitHub token revoked successfully");
        Ok(())
    }
    
    fn get_available_scopes(&self) -> HashMap<String, String> {
        let mut scopes = HashMap::new();
        
        // User scopes
        scopes.insert("user".to_string(), "Read/write access to profile info only".to_string());
        scopes.insert("user:email".to_string(), "Access to user email addresses".to_string());
        scopes.insert("user:follow".to_string(), "Access to follow or unfollow other users".to_string());
        scopes.insert("read:user".to_string(), "Read access to a user's profile data".to_string());
        
        // Repository scopes
        scopes.insert("repo".to_string(), "Full access to repositories".to_string());
        scopes.insert("repo:status".to_string(), "Access to commit statuses".to_string());
        scopes.insert("repo_deployment".to_string(), "Access to deployment statuses".to_string());
        scopes.insert("public_repo".to_string(), "Access to public repositories only".to_string());
        scopes.insert("repo:invite".to_string(), "Access to repository invitations".to_string());
        scopes.insert("security_events".to_string(), "Read access to security events".to_string());
        
        // Organization scopes
        scopes.insert("admin:org".to_string(), "Full access to organization".to_string());
        scopes.insert("write:org".to_string(), "Read and write access to organization".to_string());
        scopes.insert("read:org".to_string(), "Read access to organization".to_string());
        scopes.insert("admin:public_key".to_string(), "Full access to public keys".to_string());
        scopes.insert("write:public_key".to_string(), "Write access to public keys".to_string());
        scopes.insert("read:public_key".to_string(), "Read access to public keys".to_string());
        
        // Additional scopes
        scopes.insert("gist".to_string(), "Access to gists".to_string());
        scopes.insert("notifications".to_string(), "Access to notifications".to_string());
        scopes.insert("delete_repo".to_string(), "Access to delete repositories".to_string());
        scopes.insert("write:discussion".to_string(), "Read and write access to discussions".to_string());
        scopes.insert("read:discussion".to_string(), "Read access to discussions".to_string());
        scopes.insert("write:packages".to_string(), "Upload packages to registry".to_string());
        scopes.insert("read:packages".to_string(), "Download packages from registry".to_string());
        scopes.insert("delete:packages".to_string(), "Delete packages from registry".to_string());
        scopes.insert("admin:gpg_key".to_string(), "Full access to GPG keys".to_string());
        scopes.insert("write:gpg_key".to_string(), "Write access to GPG keys".to_string());
        scopes.insert("read:gpg_key".to_string(), "Read access to GPG keys".to_string());
        scopes.insert("codespace".to_string(), "Full access to codespaces".to_string());
        scopes.insert("workflow".to_string(), "Update GitHub Action workflows".to_string());
        
        scopes
    }
    
    fn supports_feature(&self, feature: ProviderFeature) -> bool {
        match feature {
            ProviderFeature::Pkce => false, // OAuth Apps don't support PKCE
            ProviderFeature::RefreshTokens => false, // GitHub doesn't use refresh tokens
            ProviderFeature::TokenRevocation => true,
            ProviderFeature::OpenIdConnect => false, // GitHub uses OAuth 2.0, not OIDC
            ProviderFeature::DynamicRegistration => false,
            ProviderFeature::DeviceCodeFlow => true, // GitHub supports device flow
            ProviderFeature::Organizations => true,
            ProviderFeature::Webhooks => true, // GitHub has extensive webhook support
            ProviderFeature::UserMetadata => false, // GitHub doesn't have custom user metadata
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;
    
    fn create_test_config() -> GitHubConfig {
        GitHubConfig {
            client_id: "test_client_id".to_string(),
            client_secret: Secret::new("test_client_secret".to_string()),
            scopes: vec!["user:email".to_string(), "read:user".to_string()],
            enterprise_url: None,
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: GitHubConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.client_id, deserialized.client_id);
        assert_eq!(config.scopes, deserialized.scopes);
        assert_eq!(config.enterprise_url, deserialized.enterprise_url);
    }
    
    #[test]
    fn test_default_scopes() {
        let scopes = default_scopes();
        assert_eq!(scopes.len(), 2);
        assert!(scopes.contains(&"user:email".to_string()));
        assert!(scopes.contains(&"read:user".to_string()));
    }
    
    #[tokio::test]
    async fn test_provider_creation() {
        let config = create_test_config();
        let http_client = reqwest::Client::new();
        let provider = GitHubProvider::new(config, http_client).await.unwrap();
        
        assert_eq!(provider.provider_id(), "github");
        assert_eq!(provider.provider_name(), "GitHub");
        assert_eq!(provider.base_url, "https://github.com");
        assert_eq!(provider.api_base_url, "https://api.github.com");
    }
}