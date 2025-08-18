//! OAuth Integration Layer
//! 
//! Integrates the new modular OAuth provider system with the existing OAuth validator infrastructure.

use crate::auth::{
    oauth_providers::OAuthProviderConfig,
    provider_manager::{ProviderManager, ProviderManagerConfig, OAuthSession},
    oauth::{OAuthValidator, OAuthValidationResult, OAuthUserInfo, ResourceIndicatorsConfig},
    UserContext
};
use crate::config::{AuthConfig, OAuthConfig};
use crate::error::Result;
use actix_web::HttpRequest;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Unified OAuth authentication system
pub struct UnifiedOAuthSystem {
    /// Provider manager for modular OAuth providers
    provider_manager: ProviderManager,
    /// Legacy OAuth validator for backward compatibility
    oauth_validator: Option<OAuthValidator>,
    /// Active OAuth sessions
    sessions: Arc<RwLock<HashMap<String, OAuthSession>>>,
    /// Resource indicators configuration
    resource_indicators: ResourceIndicatorsConfig,
}

impl UnifiedOAuthSystem {
    /// Create new unified OAuth system from configuration
    pub async fn new(auth_config: AuthConfig, user_context: Option<UserContext>) -> Result<Self> {
        // Extract provider manager configuration from auth config
        let provider_manager_config = Self::extract_provider_config(&auth_config)?;
        let provider_manager = ProviderManager::new(provider_manager_config).await?;
        
        // Create legacy OAuth validator if old OAuth config exists
        let oauth_validator = if auth_config.oauth.is_some() {
            if let Some(uc) = user_context {
                Some(OAuthValidator::with_token_storage(auth_config.clone(), uc).await?)
            } else {
                Some(OAuthValidator::new(auth_config.clone()))
            }
        } else {
            None
        };
        
        // Extract resource indicators configuration
        let resource_indicators = if let Some(oauth_config) = &auth_config.oauth {
            ResourceIndicatorsConfig::from_oauth_config(oauth_config)
        } else {
            ResourceIndicatorsConfig::default()
        };
        
        Ok(Self {
            provider_manager,
            oauth_validator,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            resource_indicators,
        })
    }
    
    /// Extract provider manager configuration from auth config
    fn extract_provider_config(auth_config: &AuthConfig) -> Result<ProviderManagerConfig> {
        let mut providers = HashMap::new();
        
        // Convert legacy OAuth config to provider config if present
        if let Some(oauth_config) = &auth_config.oauth {
            let provider_config = Self::convert_legacy_oauth_config(oauth_config)?;
            providers.insert(oauth_config.provider.clone(), provider_config);
        }
        
        // TODO: Add support for reading multiple providers from extended config
        // This would be where we'd read the new provider configurations from YAML
        
        Ok(ProviderManagerConfig {
            providers,
            default_provider: auth_config.oauth.as_ref().map(|c| c.provider.clone()),
            redirect_uri_base: "http://localhost:3001".to_string(), // TODO: Make configurable
            enable_auto_discovery: true,
        })
    }
    
    /// Convert legacy OAuth config to new provider config
    fn convert_legacy_oauth_config(oauth_config: &OAuthConfig) -> Result<OAuthProviderConfig> {
        // Try to determine provider type from URLs or provider string
        let provider_type = Self::detect_provider_type(&oauth_config.provider, &oauth_config.auth_url)?;
        
        match provider_type.as_str() {
            "google" => {
                Ok(OAuthProviderConfig::Google(crate::auth::oauth_providers::google::GoogleConfig {
                    client_id: oauth_config.client_id.clone(),
                    client_secret: oauth_config.client_secret.clone(),
                    scopes: vec![
                        "openid".to_string(),
                        "profile".to_string(),
                        "email".to_string(),
                    ],
                    hosted_domain: None,
                    enable_offline_access: true,
                    prompt: "consent".to_string(),
                    access_type: "offline".to_string(),
                }))
            }
            "microsoft" => {
                Ok(OAuthProviderConfig::Microsoft(crate::auth::oauth_providers::microsoft::MicrosoftConfig {
                    tenant_id: "common".to_string(), // TODO: Extract from auth_url
                    client_id: oauth_config.client_id.clone(),
                    client_secret: oauth_config.client_secret.clone(),
                    scopes: vec![
                        "openid".to_string(),
                        "profile".to_string(),
                        "email".to_string(),
                        "https://graph.microsoft.com/User.Read".to_string(),
                    ],
                    graph_version: "v1.0".to_string(),
                    prompt: "select_account".to_string(),
                    response_mode: "query".to_string(),
                    domain_hint: None,
                }))
            }
            "github" => {
                Ok(OAuthProviderConfig::GitHub(crate::auth::oauth_providers::github::GitHubConfig {
                    client_id: oauth_config.client_id.clone(),
                    client_secret: oauth_config.client_secret.clone(),
                    scopes: vec!["user:email".to_string(), "read:user".to_string()],
                    enterprise_url: None,
                }))
            }
            _ => {
                // Fallback to generic OIDC
                Ok(OAuthProviderConfig::GenericOidc(crate::auth::oauth_providers::generic_oidc::GenericOidcConfig {
                    provider_name: oauth_config.provider.clone(),
                    issuer_url: oauth_config.auth_url.clone(),
                    client_id: oauth_config.client_id.clone(),
                    client_secret: oauth_config.client_secret.clone(),
                    scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
                    discovery_url: None,
                    audience: None,
                    additional_auth_params: HashMap::new(),
                }))
            }
        }
    }
    
    /// Detect provider type from configuration
    fn detect_provider_type(provider: &str, auth_url: &str) -> Result<String> {
        // First try provider string
        let provider_lower = provider.to_lowercase();
        if provider_lower.contains("google") || auth_url.contains("accounts.google.com") {
            return Ok("google".to_string());
        }
        if provider_lower.contains("microsoft") || auth_url.contains("login.microsoftonline.com") {
            return Ok("microsoft".to_string());
        }
        if provider_lower.contains("github") || auth_url.contains("github.com") {
            return Ok("github".to_string());
        }
        if provider_lower.contains("auth0") || auth_url.contains("auth0.com") {
            return Ok("auth0".to_string());
        }
        if provider_lower.contains("clerk") || auth_url.contains("clerk.") {
            return Ok("clerk".to_string());
        }
        if provider_lower.contains("apple") || auth_url.contains("appleid.apple.com") {
            return Ok("apple".to_string());
        }
        
        // Default to generic OIDC
        Ok("generic_oidc".to_string())
    }
    
    /// Validate an HTTP request using the unified OAuth system
    pub async fn validate_request(&self, req: &HttpRequest) -> Result<Option<OAuthValidationResult>> {
        // Try new provider system first
        if let Some(session) = self.validate_with_provider_system(req).await? {
            return Ok(Some(session));
        }
        
        // Fallback to legacy OAuth validator
        if let Some(validator) = &self.oauth_validator {
            return validator.validate_request(req).await;
        }
        
        Ok(None)
    }
    
    /// Validate request using the new provider system
    async fn validate_with_provider_system(&self, req: &HttpRequest) -> Result<Option<OAuthValidationResult>> {
        // Extract authorization header
        let auth_header = req.headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok());
            
        if let Some(auth_value) = auth_header {
            if let Some(token) = auth_value.strip_prefix("Bearer ") {
                return self.validate_token(token).await;
            }
        }
        
        Ok(None)
    }
    
    /// Validate a token using the provider system
    async fn validate_token(&self, token: &str) -> Result<Option<OAuthValidationResult>> {
        // Check active sessions first
        let sessions = self.sessions.read().await;
        for session in sessions.values() {
            if session.access_token == token {
                if session.is_expired() {
                    // Clone the session before dropping the lock
                    let session_clone = session.clone();
                    drop(sessions);
                    return self.refresh_session_if_possible(&session_clone).await;
                } else {
                    return Ok(Some(self.session_to_validation_result(session)));
                }
            }
        }
        
        // If not in sessions, try to validate with providers
        // This would require knowing which provider the token belongs to
        // For now, return None - in practice, sessions should be managed properly
        Ok(None)
    }
    
    /// Convert OAuth session to validation result
    fn session_to_validation_result(&self, session: &OAuthSession) -> OAuthValidationResult {
        OAuthValidationResult {
            user_info: OAuthUserInfo {
                id: session.user_info.id.clone(),
                email: session.user_info.email.clone(),
                name: session.user_info.name.clone(),
                login: session.user_info.username.clone(),
            },
            expires_at: session.expires_at.map(|dt| dt.timestamp() as u64),
            scopes: session.scopes.clone(),
            audience: None, // TODO: Extract from resource indicators
            resources: None, // TODO: Extract from resource indicators
            issuer: Some(session.provider_id.clone()),
        }
    }
    
    /// Attempt to refresh a session if possible
    async fn refresh_session_if_possible(&self, session: &OAuthSession) -> Result<Option<OAuthValidationResult>> {
        if let Some(refresh_token) = &session.refresh_token {
            if let Ok(token_set) = self.provider_manager.refresh_token(&session.provider_id, refresh_token).await {
                if let Ok(user_info) = self.provider_manager.get_user_info(&session.provider_id, &token_set.access_token).await {
                    let new_session = OAuthSession::new(session.provider_id.clone(), token_set, user_info);
                    let result = self.session_to_validation_result(&new_session);
                    
                    // Update session
                    let mut sessions = self.sessions.write().await;
                    sessions.insert(new_session.access_token.clone(), new_session);
                    
                    return Ok(Some(result));
                }
            }
        }
        Ok(None)
    }
    
    /// Start OAuth authorization flow
    pub async fn start_authorization(&self, provider_id: &str, scopes: Vec<String>) -> Result<String> {
        let auth_url = self.provider_manager.start_authorization(provider_id, &scopes, None).await?;
        Ok(auth_url.url.to_string())
    }
    
    /// Complete OAuth authorization flow
    pub async fn complete_authorization(
        &self,
        provider_id: &str,
        code: &str,
        state: &str,
    ) -> Result<OAuthSession> {
        let token_set = self.provider_manager.complete_authorization(provider_id, code, state, None).await?;
        let user_info = self.provider_manager.get_user_info(provider_id, &token_set.access_token).await?;
        
        let session = OAuthSession::new(provider_id.to_string(), token_set, user_info);
        
        // Store session
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.access_token.clone(), session.clone());
        
        Ok(session)
    }
    
    /// Get provider manager for direct access
    pub fn provider_manager(&self) -> &ProviderManager {
        &self.provider_manager
    }
    
    /// Get active sessions
    pub async fn get_active_sessions(&self) -> Vec<OAuthSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }
    
    /// Revoke session
    pub async fn revoke_session(&self, access_token: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(access_token) {
            // Try to revoke with provider
            let _ = self.provider_manager.revoke_token(&session.provider_id, access_token).await;
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthType;
    use secrecy::Secret;
    
    fn create_test_auth_config() -> AuthConfig {
        AuthConfig {
            enabled: true,
            r#type: AuthType::OAuth,
            oauth: Some(OAuthConfig {
                provider: "google".to_string(),
                client_id: "test_client_id".to_string(),
                client_secret: Secret::new("test_secret".to_string()),
                auth_url: "https://accounts.google.com/o/oauth2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                oauth_2_1_enabled: true,
                resource_indicators_enabled: true,
                default_resources: vec!["https://api.example.com".to_string()],
                default_audience: vec!["example-api".to_string()],
                require_explicit_resources: false,
            }),
            api_keys: None,
            jwt: None,
        }
    }
    
    #[test]
    fn test_provider_type_detection() {
        assert_eq!(
            UnifiedOAuthSystem::detect_provider_type("google", "https://accounts.google.com/o/oauth2/auth").unwrap(),
            "google"
        );
        assert_eq!(
            UnifiedOAuthSystem::detect_provider_type("microsoft", "https://login.microsoftonline.com/common/oauth2/v2.0/authorize").unwrap(),
            "microsoft"
        );
        assert_eq!(
            UnifiedOAuthSystem::detect_provider_type("github", "https://github.com/login/oauth/authorize").unwrap(),
            "github"
        );
    }
    
    #[tokio::test]
    async fn test_provider_config_extraction() {
        let auth_config = create_test_auth_config();
        let provider_config = UnifiedOAuthSystem::extract_provider_config(&auth_config).unwrap();
        
        assert_eq!(provider_config.default_provider, Some("google".to_string()));
        assert!(provider_config.providers.contains_key("google"));
    }
}