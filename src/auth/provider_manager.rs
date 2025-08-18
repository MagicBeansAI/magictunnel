//! OAuth Provider Manager
//! 
//! Manages multiple OAuth providers and provides a unified interface for authentication.

use crate::auth::oauth_providers::{
    OAuthProvider, OAuthProviderConfig, OAuthProviderFactory, TokenSet, UserInfo, 
    AuthorizationUrl, TokenValidation, ProviderFeature
};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};
use serde::{Deserialize, Serialize};

/// OAuth provider manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderManagerConfig {
    /// Map of provider ID to provider configuration
    pub providers: HashMap<String, OAuthProviderConfig>,
    /// Default provider to use when none specified
    pub default_provider: Option<String>,
    /// Global redirect URI base (will be combined with provider-specific paths)
    pub redirect_uri_base: String,
    /// Enable provider auto-discovery
    #[serde(default = "default_true")]
    pub enable_auto_discovery: bool,
}

fn default_true() -> bool {
    true
}

/// OAuth provider manager
pub struct ProviderManager {
    config: ProviderManagerConfig,
    providers: Arc<RwLock<HashMap<String, Arc<dyn OAuthProvider>>>>,
    factory: OAuthProviderFactory,
}

impl ProviderManager {
    /// Create new provider manager
    pub async fn new(config: ProviderManagerConfig) -> Result<Self> {
        let manager = Self {
            config,
            providers: Arc::new(RwLock::new(HashMap::new())),
            factory: OAuthProviderFactory::new(),
        };
        
        // Initialize all configured providers
        manager.initialize_providers().await?;
        
        Ok(manager)
    }
    
    /// Initialize all providers from configuration
    async fn initialize_providers(&self) -> Result<()> {
        let mut providers = self.providers.write().await;
        
        for (provider_id, provider_config) in &self.config.providers {
            match self.factory.create_provider(provider_config.clone()).await {
                Ok(provider) => {
                    debug!("Initialized OAuth provider: {}", provider_id);
                    providers.insert(provider_id.clone(), Arc::from(provider));
                }
                Err(e) => {
                    error!("Failed to initialize OAuth provider {}: {}", provider_id, e);
                    // Continue with other providers instead of failing completely
                }
            }
        }
        
        debug!("Initialized {} OAuth providers", providers.len());
        Ok(())
    }
    
    /// Get provider by ID
    pub async fn get_provider(&self, provider_id: &str) -> Result<Arc<dyn OAuthProvider>> {
        let providers = self.providers.read().await;
        providers
            .get(provider_id)
            .cloned()
            .ok_or_else(|| anyhow!("OAuth provider '{}' not found", provider_id))
    }
    
    /// Get default provider
    pub async fn get_default_provider(&self) -> Result<Arc<dyn OAuthProvider>> {
        if let Some(default_id) = &self.config.default_provider {
            self.get_provider(default_id).await
        } else {
            let providers = self.providers.read().await;
            if let Some((_, provider)) = providers.iter().next() {
                Ok(provider.clone())
            } else {
                Err(anyhow!("No OAuth providers configured"))
            }
        }
    }
    
    /// List all available providers
    pub async fn list_providers(&self) -> Vec<(String, String)> {
        let providers = self.providers.read().await;
        providers
            .iter()
            .map(|(id, provider)| (id.clone(), provider.provider_name().to_string()))
            .collect()
    }
    
    /// Add a new provider at runtime
    pub async fn add_provider(&self, provider_id: String, config: OAuthProviderConfig) -> Result<()> {
        let provider = self.factory.create_provider(config).await?;
        let mut providers = self.providers.write().await;
        providers.insert(provider_id.clone(), Arc::from(provider));
        debug!("Added OAuth provider: {}", provider_id);
        Ok(())
    }
    
    /// Remove a provider
    pub async fn remove_provider(&self, provider_id: &str) -> Result<()> {
        let mut providers = self.providers.write().await;
        if providers.remove(provider_id).is_some() {
            debug!("Removed OAuth provider: {}", provider_id);
            Ok(())
        } else {
            Err(anyhow!("OAuth provider '{}' not found", provider_id))
        }
    }
    
    /// Get redirect URI for a provider
    pub fn get_redirect_uri(&self, provider_id: &str) -> String {
        format!("{}/oauth/callback/{}", 
            self.config.redirect_uri_base.trim_end_matches('/'),
            provider_id
        )
    }
    
    /// Start OAuth authorization flow
    pub async fn start_authorization(
        &self,
        provider_id: &str,
        scopes: &[String],
        additional_params: Option<HashMap<String, String>>,
    ) -> Result<AuthorizationUrl> {
        let provider = self.get_provider(provider_id).await?;
        let redirect_uri = self.get_redirect_uri(provider_id);
        
        debug!("Starting OAuth authorization for provider: {}", provider_id);
        provider.get_authorization_url(scopes, &redirect_uri).await
    }
    
    /// Complete OAuth authorization flow
    pub async fn complete_authorization(
        &self,
        provider_id: &str,
        code: &str,
        state: &str,
        code_verifier: Option<&str>,
    ) -> Result<TokenSet> {
        let provider = self.get_provider(provider_id).await?;
        let redirect_uri = self.get_redirect_uri(provider_id);
        
        debug!("Completing OAuth authorization for provider: {}", provider_id);
        provider.exchange_code_for_token(code, &redirect_uri, state, code_verifier).await
    }
    
    /// Refresh access token
    pub async fn refresh_token(
        &self,
        provider_id: &str,
        refresh_token: &str,
    ) -> Result<TokenSet> {
        let provider = self.get_provider(provider_id).await?;
        debug!("Refreshing token for provider: {}", provider_id);
        provider.refresh_token(refresh_token).await
    }
    
    /// Get user information
    pub async fn get_user_info(
        &self,
        provider_id: &str,
        access_token: &str,
    ) -> Result<UserInfo> {
        let provider = self.get_provider(provider_id).await?;
        debug!("Getting user info for provider: {}", provider_id);
        provider.get_user_info(access_token).await
    }
    
    /// Validate access token
    pub async fn validate_token(
        &self,
        provider_id: &str,
        access_token: &str,
    ) -> Result<TokenValidation> {
        let provider = self.get_provider(provider_id).await?;
        debug!("Validating token for provider: {}", provider_id);
        provider.validate_token(access_token).await
    }
    
    /// Revoke access token
    pub async fn revoke_token(
        &self,
        provider_id: &str,
        access_token: &str,
    ) -> Result<()> {
        let provider = self.get_provider(provider_id).await?;
        debug!("Revoking token for provider: {}", provider_id);
        provider.revoke_token(access_token).await
    }
    
    /// Get available scopes for a provider
    pub async fn get_available_scopes(&self, provider_id: &str) -> Result<HashMap<String, String>> {
        let provider = self.get_provider(provider_id).await?;
        Ok(provider.get_available_scopes())
    }
    
    /// Check if provider supports a specific feature
    pub async fn supports_feature(&self, provider_id: &str, feature: ProviderFeature) -> Result<bool> {
        let provider = self.get_provider(provider_id).await?;
        Ok(provider.supports_feature(feature))
    }
    
    /// Auto-discover provider from issuer URL
    pub async fn discover_provider(&self, issuer_url: &str) -> Result<String> {
        if !self.config.enable_auto_discovery {
            return Err(anyhow!("Provider auto-discovery is disabled"));
        }
        
        self.factory.detect_provider_type(issuer_url)
    }
    
    /// Get provider statistics
    pub async fn get_provider_stats(&self) -> ProviderStats {
        let providers = self.providers.read().await;
        let mut by_type = HashMap::new();
        let mut supported_features = HashMap::new();
        
        for (id, provider) in providers.iter() {
            // Count by provider type
            let provider_type = provider.provider_id().to_string();
            *by_type.entry(provider_type).or_insert(0) += 1;
            
            // Collect supported features
            let mut features = Vec::new();
            for feature in [
                ProviderFeature::Pkce,
                ProviderFeature::RefreshTokens,
                ProviderFeature::TokenRevocation,
                ProviderFeature::OpenIdConnect,
                ProviderFeature::DynamicRegistration,
                ProviderFeature::DeviceCodeFlow,
                ProviderFeature::Organizations,
                ProviderFeature::Webhooks,
                ProviderFeature::UserMetadata,
            ] {
                if provider.supports_feature(feature.clone()) {
                    features.push(feature);
                }
            }
            supported_features.insert(id.clone(), features);
        }
        
        ProviderStats {
            total_providers: providers.len(),
            providers_by_type: by_type,
            supported_features,
        }
    }
}

/// Provider statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderStats {
    /// Total number of configured providers
    pub total_providers: usize,
    /// Count of providers by type
    pub providers_by_type: HashMap<String, usize>,
    /// Supported features by provider
    pub supported_features: HashMap<String, Vec<ProviderFeature>>,
}

/// OAuth session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthSession {
    /// Provider ID
    pub provider_id: String,
    /// Access token
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Token expiration timestamp
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Refresh token
    pub refresh_token: Option<String>,
    /// Granted scopes
    pub scopes: Vec<String>,
    /// User information
    pub user_info: UserInfo,
    /// Session creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl OAuthSession {
    /// Create new OAuth session
    pub fn new(
        provider_id: String,
        token_set: TokenSet,
        user_info: UserInfo,
    ) -> Self {
        let now = chrono::Utc::now();
        let expires_at = token_set.expires_in
            .map(|seconds| now + chrono::Duration::seconds(seconds as i64));
            
        Self {
            provider_id,
            access_token: token_set.access_token,
            token_type: token_set.token_type,
            expires_at,
            refresh_token: token_set.refresh_token,
            scopes: token_set.scope
                .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
                .unwrap_or_default(),
            user_info,
            created_at: now,
            last_activity: now,
        }
    }
    
    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }
    
    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = chrono::Utc::now();
    }
    
    /// Check if refresh is needed (expires within 5 minutes)
    pub fn needs_refresh(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let refresh_threshold = chrono::Utc::now() + chrono::Duration::minutes(5);
            expires_at < refresh_threshold && self.refresh_token.is_some()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::oauth_providers::auth0::Auth0Config;
    use secrecy::Secret;
    
    fn create_test_config() -> ProviderManagerConfig {
        let mut providers = HashMap::new();
        providers.insert("auth0".to_string(), OAuthProviderConfig::Auth0(Auth0Config {
            domain: "test.auth0.com".to_string(),
            client_id: "test_client".to_string(),
            client_secret: Secret::new("test_secret".to_string()),
            scopes: vec!["openid".to_string()],
            audience: None,
            connection: None,
            namespace: None,
        }));
        
        ProviderManagerConfig {
            providers,
            default_provider: Some("auth0".to_string()),
            redirect_uri_base: "https://example.com".to_string(),
            enable_auto_discovery: true,
        }
    }
    
    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: ProviderManagerConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.default_provider, deserialized.default_provider);
        assert_eq!(config.redirect_uri_base, deserialized.redirect_uri_base);
        assert_eq!(config.enable_auto_discovery, deserialized.enable_auto_discovery);
    }
    
    #[test]
    fn test_redirect_uri_generation() {
        let config = create_test_config();
        let manager = ProviderManager {
            config,
            providers: Arc::new(RwLock::new(HashMap::new())),
            factory: OAuthProviderFactory::new(),
        };
        
        let redirect_uri = manager.get_redirect_uri("auth0");
        assert_eq!(redirect_uri, "https://example.com/oauth/callback/auth0");
    }
}