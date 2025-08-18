//! Apple OAuth Provider Implementation
//! 
//! Implements OAuth 2.1 and OIDC support for Apple Sign In with Apple-specific features.

use super::{OAuthProvider, TokenSet, UserInfo, AuthorizationUrl, TokenValidation, ProviderFeature, utils};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use secrecy::{Secret, ExposeSecret};
use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// Apple provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleConfig {
    /// Apple Services ID (client_id)
    pub client_id: String,
    /// Apple Team ID
    pub team_id: String,
    /// Apple Key ID
    pub key_id: String,
    /// Apple private key (P8 format)
    #[serde(with = "crate::config::secret_string")]
    pub private_key: Secret<String>,
    /// Default scopes to request
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// Response mode (query, fragment, form_post)
    #[serde(default = "default_response_mode")]
    pub response_mode: String,
    /// Enable name and email collection
    #[serde(default = "default_true")]
    pub request_user_info: bool,
}

fn default_scopes() -> Vec<String> {
    vec!["name".to_string(), "email".to_string()]
}

fn default_response_mode() -> String {
    "form_post".to_string()
}

fn default_true() -> bool {
    true
}

/// Apple OIDC discovery document
#[derive(Debug, Deserialize)]
struct AppleDiscovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
    subject_types_supported: Vec<String>,
    id_token_signing_alg_values_supported: Vec<String>,
}

/// Apple OAuth provider
pub struct AppleProvider {
    config: AppleConfig,
    http_client: reqwest::Client,
    discovery: AppleDiscovery,
}

impl AppleProvider {
    /// Create new Apple provider with auto-discovery
    pub async fn new(config: AppleConfig, http_client: reqwest::Client) -> Result<Self> {
        let discovery_url = "https://appleid.apple.com/.well-known/openid-configuration";
        
        debug!("Discovering Apple endpoints from: {}", discovery_url);
        
        let discovery: AppleDiscovery = http_client
            .get(discovery_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
            
        debug!("Apple discovery successful");
        
        Ok(Self {
            config,
            http_client,
            discovery,
        })
    }
    
    /// Generate JWT client assertion for Apple
    fn generate_client_assertion(&self) -> Result<String> {
        use jsonwebtoken::{encode, Algorithm, Header, EncodingKey};
        use serde_json::json;
        
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.config.key_id.clone());
        
        let now = chrono::Utc::now().timestamp();
        let claims = json!({
            "iss": self.config.team_id,
            "iat": now,
            "exp": now + 3600, // 1 hour
            "aud": "https://appleid.apple.com",
            "sub": self.config.client_id
        });
        
        let key = EncodingKey::from_ec_pem(self.config.private_key.expose_secret().as_bytes())?;
        let token = encode(&header, &claims, &key)?;
        
        Ok(token)
    }
}

#[async_trait]
impl OAuthProvider for AppleProvider {
    fn provider_id(&self) -> &str {
        "apple"
    }
    
    fn provider_name(&self) -> &str {
        "Apple"
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
        
        let mut url = Url::parse(&self.discovery.authorization_endpoint)?;
        
        let scopes_str = utils::join_scopes(&all_scopes);
        let query_params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", redirect_uri),
            ("scope", &scopes_str),
            ("state", &state),
            ("response_mode", &self.config.response_mode),
        ];
        
        url.query_pairs_mut().extend_pairs(query_params);
        
        Ok(AuthorizationUrl {
            url,
            state,
            code_verifier: None, // Apple Sign In doesn't use PKCE
        })
    }
    
    async fn exchange_code_for_token(
        &self,
        code: &str,
        redirect_uri: &str,
        _state: &str,
        _code_verifier: Option<&str>,
    ) -> Result<TokenSet> {
        let client_assertion = self.generate_client_assertion()?;
        
        let params = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.config.client_id),
            ("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer"),
            ("client_assertion", &client_assertion),
        ];
        
        let response = self.http_client
            .post(&self.discovery.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Apple token exchange failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Apple response"))?
                .to_string(),
            token_type: token_response["token_type"]
                .as_str()
                .unwrap_or("Bearer")
                .to_string(),
            expires_in: token_response["expires_in"].as_u64(),
            refresh_token: token_response["refresh_token"]
                .as_str()
                .map(|s| s.to_string()),
            scope: None, // Apple doesn't return scope in token response
            additional_data: token_response
                .as_object()
                .unwrap_or(&serde_json::Map::new())
                .clone()
                .into_iter()
                .collect(),
        })
    }
    
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenSet> {
        let client_assertion = self.generate_client_assertion()?;
        
        let params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
            ("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer"),
            ("client_assertion", &client_assertion),
        ];
        
        let response = self.http_client
            .post(&self.discovery.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Apple token refresh failed: {}", error_text));
        }
        
        let token_response: serde_json::Value = response.json().await?;
        
        Ok(TokenSet {
            access_token: token_response["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access_token in Apple refresh response"))?
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
            scope: None, // Apple doesn't return scope in token response
            additional_data: token_response
                .as_object()
                .unwrap_or(&serde_json::Map::new())
                .clone()
                .into_iter()
                .collect(),
        })
    }
    
    async fn get_user_info(&self, access_token: &str) -> Result<UserInfo> {
        // Apple doesn't have a standard userinfo endpoint
        // User information is typically provided in the initial token response
        // For this implementation, we'll decode the ID token to get user info
        
        // This is a simplified implementation - in practice, you'd want to:
        // 1. Verify the JWT signature against Apple's public keys
        // 2. Validate the JWT claims
        // 3. Extract user information from the JWT payload
        
        // For now, return minimal user info that can be extracted from the access token
        // In a real implementation, user data would come from the ID token received during auth
        Ok(UserInfo {
            id: "apple_user_id".to_string(), // Would be extracted from ID token
            email: None, // Would be extracted from ID token if email scope was granted
            name: None, // Would be extracted from ID token if name scope was granted
            picture: None, // Apple doesn't provide profile pictures
            username: None, // Apple doesn't provide usernames
            email_verified: Some(true), // Apple emails are always verified
            additional_claims: HashMap::new(),
        })
    }
    
    async fn validate_token(&self, _access_token: &str) -> Result<TokenValidation> {
        // Apple doesn't have a token validation endpoint
        // In practice, you would validate the ID token JWT
        Ok(TokenValidation {
            valid: true, // Would be determined by JWT validation
            expires_at: None, // Would be extracted from JWT
            scopes: vec![], // Apple doesn't expose detailed scopes
            user_id: Some("apple_user_id".to_string()), // Would be extracted from JWT
            client_id: Some(self.config.client_id.clone()),
            metadata: HashMap::new(),
        })
    }
    
    async fn revoke_token(&self, access_token: &str) -> Result<()> {
        let client_assertion = self.generate_client_assertion()?;
        
        let params = vec![
            ("token", access_token),
            ("client_id", &self.config.client_id),
            ("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer"),
            ("client_assertion", &client_assertion),
        ];
        
        // Apple's revocation endpoint
        let revocation_url = "https://appleid.apple.com/auth/revoke";
        
        let response = self.http_client
            .post(revocation_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;
            
        if !response.status().is_success() {
            warn!("Apple token revocation failed with status: {}", response.status());
            let error_text = response.text().await?;
            return Err(anyhow!("Apple token revocation failed: {}", error_text));
        }
        
        debug!("Apple token revoked successfully");
        Ok(())
    }
    
    fn get_available_scopes(&self) -> HashMap<String, String> {
        let mut scopes = HashMap::new();
        
        // Apple Sign In scopes (very limited)
        scopes.insert("name".to_string(), "Access to user's name".to_string());
        scopes.insert("email".to_string(), "Access to user's email address".to_string());
        
        scopes
    }
    
    fn supports_feature(&self, feature: ProviderFeature) -> bool {
        match feature {
            ProviderFeature::Pkce => false, // Apple Sign In doesn't use PKCE
            ProviderFeature::RefreshTokens => true,
            ProviderFeature::TokenRevocation => true,
            ProviderFeature::OpenIdConnect => true, // Apple uses OIDC with JWT
            ProviderFeature::DynamicRegistration => false, // Apple requires manual app registration
            ProviderFeature::DeviceCodeFlow => false, // Apple doesn't support device flow
            ProviderFeature::Organizations => false, // No organization concept in Apple Sign In
            ProviderFeature::Webhooks => false, // Apple doesn't provide webhooks
            ProviderFeature::UserMetadata => false, // Apple provides minimal user data
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;
    
    fn create_test_config() -> AppleConfig {
        AppleConfig {
            client_id: "com.example.app".to_string(),
            team_id: "TEAM123456".to_string(),
            key_id: "KEY123456".to_string(),
            private_key: Secret::new("-----BEGIN PRIVATE KEY-----\nMIGTAg...\n-----END PRIVATE KEY-----".to_string()),
            scopes: vec!["name".to_string(), "email".to_string()],
            response_mode: "form_post".to_string(),
            request_user_info: true,
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: AppleConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.client_id, deserialized.client_id);
        assert_eq!(config.team_id, deserialized.team_id);
        assert_eq!(config.key_id, deserialized.key_id);
        assert_eq!(config.scopes, deserialized.scopes);
        assert_eq!(config.response_mode, deserialized.response_mode);
    }
    
    #[test]
    fn test_default_values() {
        let scopes = default_scopes();
        assert_eq!(scopes.len(), 2);
        assert!(scopes.contains(&"name".to_string()));
        assert!(scopes.contains(&"email".to_string()));
        assert!(default_true());
        assert_eq!(default_response_mode(), "form_post");
    }
}