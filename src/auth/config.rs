//! Multi-level authentication configuration for OAuth 2.1, Device Code Flow, API Keys, and Service Accounts
//! 
//! This module provides hierarchical authentication configuration at Server/Instance → Capability → Tool levels
//! supporting all four authentication methods: OAuth 2.1, Device Code Flow, API Keys, and Service Accounts.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use url::Url;
use secrecy::{Secret, ExposeSecret};

use crate::error::ProxyError;
use crate::error::Result;

/// Custom serde module for Secret<String>
mod secret_string {
    use serde::{Deserialize, Deserializer, Serializer};
    use secrecy::{Secret, ExposeSecret};
    
    pub fn serialize<S>(secret: &Secret<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(secret.expose_secret())
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Secret<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Secret::new(s))
    }
}

/// Multi-level authentication configuration supporting hierarchical auth resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLevelAuthConfig {
    /// Enable multi-level authentication system
    pub enabled: bool,
    /// Server/Instance level authentication (applies to all tools unless overridden)
    pub server_level: Option<AuthMethod>,
    /// Capability-level authentication (overrides server level)
    pub capabilities: HashMap<String, AuthMethod>,
    /// Tool-level authentication (overrides capability level)
    pub tools: HashMap<String, AuthMethod>,
    /// OAuth provider configurations
    pub oauth_providers: HashMap<String, OAuthProviderConfig>,
    /// API key configurations
    pub api_keys: Vec<ApiKeyEntry>,
    /// Service account configurations
    pub service_accounts: HashMap<String, ServiceAccountConfig>,
}

/// Authentication method enumeration supporting all four authentication types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    /// OAuth 2.1 with PKCE and Resource Indicators
    OAuth { 
        provider: String, 
        scopes: Vec<String> 
    },
    /// Device Code Flow (RFC 8628) for headless environments
    DeviceCode { 
        provider: String, 
        scopes: Vec<String> 
    },
    /// API key authentication for service-to-service
    ApiKey { 
        key_ref: String 
    },
    /// Service account authentication for machine authentication
    ServiceAccount { 
        account_ref: String 
    },
}

/// OAuth provider configuration supporting both OAuth and Device Code flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProviderConfig {
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret (sensitive - protected by secrecy)
    #[serde(with = "secret_string")]
    pub client_secret: Secret<String>,
    /// Default scopes for this provider
    pub scopes: Vec<String>,
    /// Enable standard OAuth 2.1 flow
    pub oauth_enabled: bool,
    /// Enable Device Code Flow (RFC 8628)
    pub device_code_enabled: bool,
    /// OAuth authorization endpoint
    pub authorization_endpoint: Option<String>,
    /// Device authorization endpoint (for Device Code Flow)
    pub device_authorization_endpoint: Option<String>,
    /// Token endpoint (shared by both flows)
    pub token_endpoint: Option<String>,
    /// User info endpoint for token validation
    pub user_info_endpoint: Option<String>,
    /// Resource indicators (RFC 8707)
    pub resource_indicators: Option<Vec<String>>,
    /// Additional provider-specific configuration
    pub extra_params: Option<HashMap<String, String>>,
}

/// API key configuration entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    /// API key identifier/reference
    pub key_ref: String,
    /// Human-readable name for this key
    pub name: String,
    /// The actual API key value (should be from env var)
    pub key: String,
    /// Optional RBAC user ID for authorization
    pub rbac_user_id: Option<String>,
    /// Optional RBAC roles
    pub rbac_roles: Option<Vec<String>>,
    /// Key expiration time (ISO 8601)
    pub expires_at: Option<String>,
    /// Whether this key is active
    pub active: bool,
}

/// Service account configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccountConfig {
    /// Service account type (personal_access_token, service_key, etc.)
    pub account_type: ServiceAccountType,
    /// Service account credentials (should be from env var)
    pub credentials: String,
    /// Optional RBAC user ID for authorization
    pub rbac_user_id: Option<String>,
    /// Optional RBAC roles
    pub rbac_roles: Option<Vec<String>>,
    /// Provider-specific configuration
    pub provider_config: Option<HashMap<String, String>>,
}

/// Service account type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceAccountType {
    /// Personal access token (GitHub PAT, etc.)
    PersonalAccessToken,
    /// Service account key file (Google, etc.)
    ServiceKey,
    /// Application credentials
    ApplicationCredentials,
    /// Custom service account type
    Custom(String),
}

impl Default for MultiLevelAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_level: None,
            capabilities: HashMap::new(),
            tools: HashMap::new(),
            oauth_providers: HashMap::new(),
            api_keys: Vec::new(),
            service_accounts: HashMap::new(),
        }
    }
}

impl MultiLevelAuthConfig {
    /// Create a new multi-level auth config with sensible defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable authentication with default OAuth provider
    pub fn with_oauth_provider(mut self, provider_name: String, config: OAuthProviderConfig) -> Self {
        self.enabled = true;
        self.oauth_providers.insert(provider_name, config);
        self
    }

    /// Add an API key configuration
    pub fn with_api_key(mut self, key: ApiKeyEntry) -> Self {
        self.enabled = true;
        self.api_keys.push(key);
        self
    }

    /// Add a service account configuration
    pub fn with_service_account(mut self, account_name: String, config: ServiceAccountConfig) -> Self {
        self.enabled = true;
        self.service_accounts.insert(account_name, config);
        self
    }

    /// Set server-level authentication
    pub fn with_server_auth(mut self, auth_method: AuthMethod) -> Self {
        self.enabled = true;
        self.server_level = Some(auth_method);
        self
    }

    /// Set capability-level authentication
    pub fn with_capability_auth(mut self, capability: String, auth_method: AuthMethod) -> Self {
        self.enabled = true;
        self.capabilities.insert(capability, auth_method);
        self
    }

    /// Set tool-level authentication
    pub fn with_tool_auth(mut self, tool: String, auth_method: AuthMethod) -> Self {
        self.enabled = true;
        self.tools.insert(tool, auth_method);
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Validate OAuth providers
        for (provider_name, config) in &self.oauth_providers {
            if config.client_id.is_empty() {
                return Err(ProxyError::config(format!(
                    "OAuth provider '{}' missing client_id", 
                    provider_name
                )));
            }
            if config.client_secret.expose_secret().is_empty() {
                return Err(ProxyError::config(format!(
                    "OAuth provider '{}' missing client_secret", 
                    provider_name
                )));
            }
            
            // Validate OAuth endpoints are valid URLs
            config.validate_urls()?;
            if !config.oauth_enabled && !config.device_code_enabled {
                return Err(ProxyError::config(format!(
                    "OAuth provider '{}' must have at least one flow enabled (oauth_enabled or device_code_enabled)", 
                    provider_name
                )));
            }
        }

        // Validate API keys
        for key in &self.api_keys {
            if key.key_ref.is_empty() {
                return Err(ProxyError::config("API key missing key_ref".to_string()));
            }
            if key.key.is_empty() {
                return Err(ProxyError::config(format!(
                    "API key '{}' missing key value", 
                    key.key_ref
                )));
            }
        }

        // Validate service accounts
        for (account_name, config) in &self.service_accounts {
            if config.credentials.is_empty() {
                return Err(ProxyError::config(format!(
                    "Service account '{}' missing credentials", 
                    account_name
                )));
            }
        }

        // Validate auth method references
        self.validate_auth_method_refs()?;

        Ok(())
    }

    /// Validate that all auth method references point to valid configurations
    fn validate_auth_method_refs(&self) -> Result<()> {
        let check_auth_method = |auth_method: &AuthMethod, context: &str| -> Result<()> {
            match auth_method {
                AuthMethod::OAuth { provider, .. } | 
                AuthMethod::DeviceCode { provider, .. } => {
                    if !self.oauth_providers.contains_key(provider) {
                        return Err(ProxyError::config(format!(
                            "{} references unknown OAuth provider: {}", 
                            context, provider
                        )));
                    }
                }
                AuthMethod::ApiKey { key_ref } => {
                    if !self.api_keys.iter().any(|k| k.key_ref == *key_ref) {
                        return Err(ProxyError::config(format!(
                            "{} references unknown API key: {}", 
                            context, key_ref
                        )));
                    }
                }
                AuthMethod::ServiceAccount { account_ref } => {
                    if !self.service_accounts.contains_key(account_ref) {
                        return Err(ProxyError::config(format!(
                            "{} references unknown service account: {}", 
                            context, account_ref
                        )));
                    }
                }
            }
            Ok(())
        };

        // Check server level auth
        if let Some(auth_method) = &self.server_level {
            check_auth_method(auth_method, "Server level auth")?;
        }

        // Check capability level auth
        for (capability, auth_method) in &self.capabilities {
            check_auth_method(auth_method, &format!("Capability '{}' auth", capability))?;
        }

        // Check tool level auth
        for (tool, auth_method) in &self.tools {
            check_auth_method(auth_method, &format!("Tool '{}' auth", tool))?;
        }

        Ok(())
    }
}

impl OAuthProviderConfig {
    /// Create a new OAuth provider config with both flows enabled
    pub fn new_dual_flow(
        client_id: String, 
        client_secret: String, 
        scopes: Vec<String>
    ) -> Self {
        Self {
            client_id,
            client_secret: Secret::new(client_secret),
            scopes,
            oauth_enabled: true,
            device_code_enabled: true,
            authorization_endpoint: None,
            device_authorization_endpoint: None,
            token_endpoint: None,
            user_info_endpoint: None,
            resource_indicators: None,
            extra_params: None,
        }
    }

    /// Validate OAuth endpoint URLs
    pub fn validate_urls(&self) -> Result<()> {
        if let Some(ref url) = self.authorization_endpoint {
            Url::parse(url).map_err(|_| {
                ProxyError::config(format!("Invalid authorization_endpoint URL: {}", url))
            })?;
        }

        if let Some(ref url) = self.device_authorization_endpoint {
            Url::parse(url).map_err(|_| {
                ProxyError::config(format!("Invalid device_authorization_endpoint URL: {}", url))
            })?;
        }

        if let Some(ref url) = self.token_endpoint {
            Url::parse(url).map_err(|_| {
                ProxyError::config(format!("Invalid token_endpoint URL: {}", url))
            })?;
        }

        if let Some(ref url) = self.user_info_endpoint {
            Url::parse(url).map_err(|_| {
                ProxyError::config(format!("Invalid user_info_endpoint URL: {}", url))
            })?;
        }

        Ok(())
    }

    /// Create a GitHub OAuth provider config
    pub fn github(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret: Secret::new(client_secret),
            scopes: vec!["user:email".to_string()],
            oauth_enabled: true,
            device_code_enabled: true,
            authorization_endpoint: Some("https://github.com/login/oauth/authorize".to_string()),
            device_authorization_endpoint: Some("https://github.com/login/device/code".to_string()),
            token_endpoint: Some("https://github.com/login/oauth/access_token".to_string()),
            user_info_endpoint: Some("https://api.github.com/user".to_string()),
            resource_indicators: None,
            extra_params: None,
        }
    }

    /// Create a Google OAuth provider config
    pub fn google(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret: Secret::new(client_secret),
            scopes: vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
            oauth_enabled: true,
            device_code_enabled: true,
            authorization_endpoint: Some("https://accounts.google.com/o/oauth2/v2/auth".to_string()),
            device_authorization_endpoint: Some("https://oauth2.googleapis.com/device/code".to_string()),
            token_endpoint: Some("https://oauth2.googleapis.com/token".to_string()),
            user_info_endpoint: Some("https://openidconnect.googleapis.com/v1/userinfo".to_string()),
            resource_indicators: None,
            extra_params: None,
        }
    }

    /// Create a headless-only OAuth provider (Device Code Flow only)
    pub fn headless_only(
        client_id: String, 
        client_secret: String, 
        device_authorization_endpoint: String,
        token_endpoint: String,
        scopes: Vec<String>
    ) -> Self {
        Self {
            client_id,
            client_secret: Secret::new(client_secret),
            scopes,
            oauth_enabled: false,  // Disable browser-based OAuth
            device_code_enabled: true,
            authorization_endpoint: None,
            device_authorization_endpoint: Some(device_authorization_endpoint),
            token_endpoint: Some(token_endpoint),
            user_info_endpoint: None,
            resource_indicators: None,
            extra_params: None,
        }
    }
}

impl ApiKeyEntry {
    /// Create a new API key entry
    pub fn new(key_ref: String, name: String, key: String) -> Self {
        Self {
            key_ref,
            name,
            key,
            rbac_user_id: None,
            rbac_roles: None,
            expires_at: None,
            active: true,
        }
    }

    /// Create an API key with RBAC configuration
    pub fn with_rbac(
        key_ref: String, 
        name: String, 
        key: String, 
        rbac_user_id: String, 
        rbac_roles: Vec<String>
    ) -> Self {
        Self {
            key_ref,
            name,
            key,
            rbac_user_id: Some(rbac_user_id),
            rbac_roles: Some(rbac_roles),
            expires_at: None,
            active: true,
        }
    }
}

impl ServiceAccountConfig {
    /// Create a new service account configuration
    pub fn new(account_type: ServiceAccountType, credentials: String) -> Self {
        Self {
            account_type,
            credentials,
            rbac_user_id: None,
            rbac_roles: None,
            provider_config: None,
        }
    }

    /// Create a GitHub Personal Access Token service account
    pub fn github_pat(token: String) -> Self {
        Self {
            account_type: ServiceAccountType::PersonalAccessToken,
            credentials: token,
            rbac_user_id: None,
            rbac_roles: None,
            provider_config: Some({
                let mut config = HashMap::new();
                config.insert("provider".to_string(), "github".to_string());
                config
            }),
        }
    }

    /// Create a Google service account with key file
    pub fn google_service_key(key_file_content: String) -> Self {
        Self {
            account_type: ServiceAccountType::ServiceKey,
            credentials: key_file_content,
            rbac_user_id: None,
            rbac_roles: None,
            provider_config: Some({
                let mut config = HashMap::new();
                config.insert("provider".to_string(), "google".to_string());
                config
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_level_auth_config_default() {
        let config = MultiLevelAuthConfig::default();
        assert!(!config.enabled);
        assert!(config.server_level.is_none());
        assert!(config.capabilities.is_empty());
        assert!(config.tools.is_empty());
    }

    #[test]
    fn test_oauth_provider_config_github() {
        let config = OAuthProviderConfig::github(
            "client_id".to_string(), 
            "client_secret".to_string()
        );
        assert!(config.oauth_enabled);
        assert!(config.device_code_enabled);
        assert_eq!(config.scopes, vec!["user:email"]);
        assert!(config.authorization_endpoint.is_some());
        assert!(config.device_authorization_endpoint.is_some());
    }

    #[test]
    fn test_headless_only_provider() {
        let config = OAuthProviderConfig::headless_only(
            "client_id".to_string(),
            "client_secret".to_string(),
            "https://example.com/device/code".to_string(),
            "https://example.com/token".to_string(),
            vec!["scope1".to_string(), "scope2".to_string()]
        );
        assert!(!config.oauth_enabled);
        assert!(config.device_code_enabled);
        assert_eq!(config.scopes, vec!["scope1", "scope2"]);
    }

    #[test]
    fn test_config_validation() {
        let mut config = MultiLevelAuthConfig::new();
        config.enabled = true;
        
        // Add valid OAuth provider
        config.oauth_providers.insert(
            "github".to_string(),
            OAuthProviderConfig::github("id".to_string(), "secret".to_string())
        );
        
        // Add server-level auth referencing the provider
        config.server_level = Some(AuthMethod::OAuth {
            provider: "github".to_string(),
            scopes: vec!["user:email".to_string()],
        });
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_provider_reference() {
        let mut config = MultiLevelAuthConfig::new();
        config.enabled = true;
        
        // Add server-level auth referencing non-existent provider
        config.server_level = Some(AuthMethod::OAuth {
            provider: "nonexistent".to_string(),
            scopes: vec!["user:email".to_string()],
        });
        
        assert!(config.validate().is_err());
    }
}