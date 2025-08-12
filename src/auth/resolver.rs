//! Multi-level authentication resolution system
//! 
//! This module implements the hierarchical authentication resolution logic:
//! Tool level → Capability level → Server level
//! 
//! The resolver determines which authentication method to use for a given tool
//! by checking in order of specificity (most specific wins).

use crate::auth::config::{MultiLevelAuthConfig, AuthMethod};
use crate::auth::user_context::UserContext;
use crate::error::ProxyError;
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::{debug, trace};
use secrecy::ExposeSecret;
use crate::error::Result;

/// Authentication resolver for hierarchical auth method resolution with user context support
#[derive(Debug)]
pub struct AuthResolver {
    /// The multi-level authentication configuration
    config: MultiLevelAuthConfig,
    /// Thread-safe cache for resolved auth methods to improve performance
    cache: RwLock<HashMap<String, Option<AuthMethod>>>,
    /// User context for session-aware authentication
    user_context: Option<UserContext>,
}

/// Authentication requirement result from resolution
#[derive(Debug, Clone, PartialEq)]
pub enum AuthRequirement {
    /// No authentication required
    None,
    /// OAuth 2.1 authentication required
    OAuth {
        provider: String,
        scopes: Vec<String>,
        auth_url: String,
    },
    /// Device Code Flow authentication required
    DeviceCode {
        provider: String,
        scopes: Vec<String>,
        device_code: String,
        user_code: String,
        verification_uri: String,
        verification_uri_complete: Option<String>,
        expires_in: u64,
        interval: u64,
    },
    /// API key authentication required
    ApiKey {
        key_ref: String,
    },
    /// Service account authentication required
    ServiceAccount {
        account_ref: String,
    },
}

impl AuthResolver {
    /// Create a new auth resolver with the given configuration
    pub fn new(config: MultiLevelAuthConfig) -> Result<Self> {
        // Validate configuration before creating resolver
        config.validate()?;
        
        Ok(Self {
            config,
            cache: RwLock::new(HashMap::new()),
            user_context: None,
        })
    }

    /// Create a new auth resolver with user context for session-aware authentication
    pub fn with_user_context(config: MultiLevelAuthConfig, user_context: UserContext) -> Result<Self> {
        // Validate configuration and user context
        config.validate()?;
        user_context.validate()?;
        
        debug!("Creating auth resolver with user context: {}", user_context.get_unique_user_id());
        
        Ok(Self {
            config,
            cache: RwLock::new(HashMap::new()),
            user_context: Some(user_context),
        })
    }

    /// Set user context for session-aware authentication
    pub fn set_user_context(&mut self, user_context: UserContext) -> Result<()> {
        user_context.validate()?;
        debug!("Setting user context: {}", user_context.get_unique_user_id());
        self.user_context = Some(user_context);
        
        // Clear cache since user context affects authentication
        self.clear_cache();
        Ok(())
    }

    /// Get the user context if available
    pub fn get_user_context(&self) -> Option<&UserContext> {
        self.user_context.as_ref()
    }

    /// Resolve authentication method for a tool using hierarchical resolution
    /// 
    /// Resolution order:
    /// 1. Tool-specific auth (highest priority)
    /// 2. Capability-specific auth (medium priority)  
    /// 3. Server-level auth (lowest priority)
    /// 4. No auth required (fallback)
    pub fn resolve_auth_for_tool(&self, tool_name: &str) -> Option<AuthMethod> {
        if !self.config.enabled {
            trace!("Authentication disabled, no auth required for tool: {}", tool_name);
            return None;
        }

        // Check cache first for performance
        {
            let cache = self.cache.read().unwrap();
            if let Some(cached_result) = cache.get(tool_name) {
                trace!("Using cached auth resolution for tool: {}", tool_name);
                return cached_result.clone();
            }
        }

        // 1. Check tool-level auth (highest priority)
        if let Some(auth_method) = self.config.tools.get(tool_name) {
            debug!("Found tool-level auth for '{}': {:?}", tool_name, auth_method);
            let result = Some(auth_method.clone());
            
            // Cache the result
            {
                let mut cache = self.cache.write().unwrap();
                cache.insert(tool_name.to_string(), result.clone());
            }
            
            return result;
        }

        // 2. Check capability-level auth (medium priority)
        // Extract capability name from tool name (tool names often follow capability.tool format)
        let capability_name = self.extract_capability_from_tool(tool_name);
        if let Some(capability) = capability_name {
            if let Some(auth_method) = self.config.capabilities.get(&capability) {
                debug!(
                    "Found capability-level auth for tool '{}' via capability '{}': {:?}", 
                    tool_name, capability, auth_method
                );
                let result = Some(auth_method.clone());
                
                // Cache the result
                {
                    let mut cache = self.cache.write().unwrap();
                    cache.insert(tool_name.to_string(), result.clone());
                }
                
                return result;
            }
        }

        // 3. Check server-level auth (lowest priority)
        if let Some(auth_method) = &self.config.server_level {
            debug!("Using server-level auth for tool '{}': {:?}", tool_name, auth_method);
            let result = Some(auth_method.clone());
            
            // Cache the result
            {
                let mut cache = self.cache.write().unwrap();
                cache.insert(tool_name.to_string(), result.clone());
            }
            
            return result;
        }

        // 4. No authentication required
        debug!("No authentication required for tool: {}", tool_name);
        let result = None;
        
        // Cache the result
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(tool_name.to_string(), result.clone());
        }
        
        result
    }

    /// Resolve authentication for a specific capability
    pub fn resolve_auth_for_capability(&self, capability_name: &str) -> Option<AuthMethod> {
        if !self.config.enabled {
            return None;
        }

        // 1. Check capability-level auth first
        if let Some(auth_method) = self.config.capabilities.get(capability_name) {
            debug!("Found capability-level auth for '{}': {:?}", capability_name, auth_method);
            return Some(auth_method.clone());
        }

        // 2. Fall back to server-level auth
        if let Some(auth_method) = &self.config.server_level {
            debug!("Using server-level auth for capability '{}': {:?}", capability_name, auth_method);
            return Some(auth_method.clone());
        }

        debug!("No authentication required for capability: {}", capability_name);
        None
    }

    /// Get server-level authentication method
    pub fn get_server_level_auth(&self) -> Option<AuthMethod> {
        if !self.config.enabled {
            return None;
        }
        
        self.config.server_level.clone()
    }

    /// Check if a tool requires any authentication
    pub fn requires_auth(&self, tool_name: &str) -> bool {
        self.resolve_auth_for_tool(tool_name).is_some()
    }

    /// Check if a capability requires any authentication
    pub fn capability_requires_auth(&self, capability_name: &str) -> bool {
        self.resolve_auth_for_capability(capability_name).is_some()
    }

    /// Get all tools that require authentication
    pub fn get_auth_required_tools(&self) -> Vec<String> {
        let mut auth_tools = Vec::new();
        
        // Add all explicitly configured tool-level auth
        for tool_name in self.config.tools.keys() {
            auth_tools.push(tool_name.clone());
        }
        
        // Note: We don't enumerate all possible tools here since that would require
        // access to the registry. This method returns only explicitly configured tools.
        // For runtime auth checking, use resolve_auth_for_tool() on specific tool names.
        
        auth_tools
    }

    /// Get all capabilities that require authentication
    pub fn get_auth_required_capabilities(&self) -> Vec<String> {
        self.config.capabilities.keys().cloned().collect()
    }

    /// Check if OAuth 2.1 provider exists and is configured properly
    pub fn validate_oauth_provider(&self, provider_name: &str) -> Result<()> {
        match self.config.oauth_providers.get(provider_name) {
            Some(provider) => {
                if provider.client_id.is_empty() || provider.client_secret.expose_secret().is_empty() {
                    return Err(ProxyError::config(format!(
                        "OAuth provider '{}' missing client_id or client_secret", 
                        provider_name
                    )));
                }
                if !provider.oauth_enabled && !provider.device_code_enabled {
                    return Err(ProxyError::config(format!(
                        "OAuth provider '{}' must have at least one flow enabled", 
                        provider_name
                    )));
                }
                Ok(())
            }
            None => Err(ProxyError::config(format!(
                "OAuth provider '{}' not found", 
                provider_name
            ))),
        }
    }

    /// Check if API key exists and is configured properly
    pub fn validate_api_key(&self, key_ref: &str) -> Result<()> {
        match self.config.api_keys.iter().find(|k| k.key_ref == key_ref) {
            Some(key_entry) => {
                if key_entry.key.is_empty() {
                    return Err(ProxyError::config(format!(
                        "API key '{}' missing key value", 
                        key_ref
                    )));
                }
                if !key_entry.active {
                    return Err(ProxyError::config(format!(
                        "API key '{}' is inactive", 
                        key_ref
                    )));
                }
                Ok(())
            }
            None => Err(ProxyError::config(format!(
                "API key '{}' not found", 
                key_ref
            ))),
        }
    }

    /// Check if service account exists and is configured properly
    pub fn validate_service_account(&self, account_ref: &str) -> Result<()> {
        match self.config.service_accounts.get(account_ref) {
            Some(account) => {
                if account.credentials.is_empty() {
                    return Err(ProxyError::config(format!(
                        "Service account '{}' missing credentials", 
                        account_ref
                    )));
                }
                Ok(())
            }
            None => Err(ProxyError::config(format!(
                "Service account '{}' not found", 
                account_ref
            ))),
        }
    }

    /// Update the configuration and clear cache
    pub fn update_config(&mut self, config: MultiLevelAuthConfig) -> Result<()> {
        config.validate()?;
        self.config = config;
        {
            let mut cache = self.cache.write().unwrap();
            cache.clear();
        }
        debug!("Auth resolver configuration updated and cache cleared");
        Ok(())
    }

    /// Clear the resolution cache (useful for testing or dynamic updates)
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
        debug!("Auth resolver cache cleared");
    }

    /// Get configuration reference for inspection
    pub fn get_config(&self) -> &MultiLevelAuthConfig {
        &self.config
    }

    /// Extract capability name from tool name using common naming conventions
    /// 
    /// Common patterns:
    /// - "capability.tool" -> "capability"
    /// - "capability_tool" -> "capability" 
    /// - "CapabilityTool" -> "Capability"
    /// - "capability-tool" -> "capability"
    fn extract_capability_from_tool(&self, tool_name: &str) -> Option<String> {
        // Pattern 1: dot notation (capability.tool)
        if let Some(dot_pos) = tool_name.find('.') {
            let capability = &tool_name[..dot_pos];
            trace!("Extracted capability '{}' from tool '{}' using dot notation", capability, tool_name);
            return Some(capability.to_string());
        }

        // Pattern 2: underscore notation (capability_tool)
        if let Some(underscore_pos) = tool_name.find('_') {
            let capability = &tool_name[..underscore_pos];
            trace!("Extracted capability '{}' from tool '{}' using underscore notation", capability, tool_name);
            return Some(capability.to_string());
        }

        // Pattern 3: hyphen notation (capability-tool)
        if let Some(hyphen_pos) = tool_name.find('-') {
            let capability = &tool_name[..hyphen_pos];
            trace!("Extracted capability '{}' from tool '{}' using hyphen notation", capability, tool_name);
            return Some(capability.to_string());
        }

        // Pattern 4: CamelCase (CapabilityTool -> Capability)
        // Look for uppercase letters that might indicate word boundaries
        let chars: Vec<char> = tool_name.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            if i > 0 && ch.is_uppercase() {
                let capability = &tool_name[..i];
                trace!("Extracted capability '{}' from tool '{}' using CamelCase", capability, tool_name);
                return Some(capability.to_string());
            }
        }

        // No pattern matched - tool name might be the capability name itself
        // or there's no clear capability separation
        trace!("Could not extract capability from tool name: {}", tool_name);
        None
    }

    /// Get session file path for storing authentication tokens/state
    pub fn get_session_file_path(&self, filename: &str) -> Option<std::path::PathBuf> {
        self.user_context.as_ref().map(|ctx| ctx.get_session_file_path(filename))
    }

    /// Get hostname-specific session file path for multi-machine isolation
    pub fn get_hostname_session_file_path(&self, filename: &str) -> Option<std::path::PathBuf> {
        self.user_context.as_ref().map(|ctx| ctx.get_hostname_session_file_path(filename))
    }

    /// Check if secure storage is available for this user context
    pub fn has_secure_storage(&self) -> bool {
        self.user_context.as_ref().map(|ctx| ctx.has_secure_storage()).unwrap_or(false)
    }

    /// Get the unique user identifier for session management
    pub fn get_unique_user_id(&self) -> Option<String> {
        self.user_context.as_ref().map(|ctx| ctx.get_unique_user_id())
    }

    /// Get authentication statistics for monitoring
    pub fn get_auth_stats(&self) -> AuthStats {
        let mut stats = AuthStats {
            enabled: self.config.enabled,
            total_oauth_providers: self.config.oauth_providers.len(),
            total_api_keys: self.config.api_keys.len(),
            total_service_accounts: self.config.service_accounts.len(),
            server_level_auth: self.config.server_level.is_some(),
            capability_level_auths: self.config.capabilities.len(),
            tool_level_auths: self.config.tools.len(),
            cache_size: self.cache.read().unwrap().len(),
            auth_method_distribution: HashMap::new(),
            has_user_context: self.user_context.is_some(),
            user_id: self.get_unique_user_id(),
            has_secure_storage: self.has_secure_storage(),
            session_directory: self.user_context.as_ref().map(|ctx| ctx.session_dir.to_string_lossy().to_string()),
        };

        // Count auth method distribution
        let mut count_auth_method = |auth_method: &AuthMethod| {
            let method_type = match auth_method {
                AuthMethod::OAuth { .. } => "oauth",
                AuthMethod::DeviceCode { .. } => "device_code",
                AuthMethod::ApiKey { .. } => "api_key",
                AuthMethod::ServiceAccount { .. } => "service_account",
            };
            *stats.auth_method_distribution.entry(method_type.to_string()).or_insert(0) += 1;
        };

        if let Some(ref auth_method) = self.config.server_level {
            count_auth_method(auth_method);
        }

        for auth_method in self.config.capabilities.values() {
            count_auth_method(auth_method);
        }

        for auth_method in self.config.tools.values() {
            count_auth_method(auth_method);
        }

        stats
    }
}

/// Authentication resolver statistics for monitoring
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuthStats {
    pub enabled: bool,
    pub total_oauth_providers: usize,
    pub total_api_keys: usize,
    pub total_service_accounts: usize,
    pub server_level_auth: bool,
    pub capability_level_auths: usize,
    pub tool_level_auths: usize,
    pub cache_size: usize,
    pub auth_method_distribution: HashMap<String, usize>,
    pub has_user_context: bool,
    pub user_id: Option<String>,
    pub has_secure_storage: bool,
    pub session_directory: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::config::{OAuthProviderConfig, ApiKeyEntry};

    fn create_test_config() -> MultiLevelAuthConfig {
        let mut config = MultiLevelAuthConfig::new();
        config.enabled = true;
        
        // Add OAuth provider
        config.oauth_providers.insert(
            "github".to_string(),
            OAuthProviderConfig::github("client_id".to_string(), "client_secret".to_string())
        );
        
        // Add API key
        config.api_keys.push(ApiKeyEntry::new(
            "test_key".to_string(),
            "Test Key".to_string(),
            "secret_key_value".to_string()
        ));
        
        config
    }

    #[test]
    fn test_resolver_creation() {
        let config = create_test_config();
        let resolver = AuthResolver::new(config);
        assert!(resolver.is_ok());
    }

    #[test]
    fn test_tool_level_auth_resolution() {
        let mut config = create_test_config();
        
        // Set tool-level auth
        config.tools.insert(
            "github_tool".to_string(),
            AuthMethod::OAuth {
                provider: "github".to_string(),
                scopes: vec!["repo".to_string()],
            }
        );
        
        let resolver = AuthResolver::new(config).unwrap();
        let auth_method = resolver.resolve_auth_for_tool("github_tool");
        
        assert!(auth_method.is_some());
        match auth_method.unwrap() {
            AuthMethod::OAuth { provider, scopes } => {
                assert_eq!(provider, "github");
                assert_eq!(scopes, vec!["repo"]);
            }
            _ => panic!("Expected OAuth auth method"),
        }
    }

    #[test]
    fn test_capability_level_auth_resolution() {
        let mut config = create_test_config();
        
        // Set capability-level auth
        config.capabilities.insert(
            "github".to_string(),
            AuthMethod::ApiKey {
                key_ref: "test_key".to_string(),
            }
        );
        
        let resolver = AuthResolver::new(config).unwrap();
        
        // Tool with capability prefix should resolve to capability auth
        let auth_method = resolver.resolve_auth_for_tool("github.create_issue");
        
        assert!(auth_method.is_some());
        match auth_method.unwrap() {
            AuthMethod::ApiKey { key_ref } => {
                assert_eq!(key_ref, "test_key");
            }
            _ => panic!("Expected API key auth method"),
        }
    }

    #[test]
    fn test_server_level_auth_resolution() {
        let mut config = create_test_config();
        
        // Set server-level auth
        config.server_level = Some(AuthMethod::OAuth {
            provider: "github".to_string(),
            scopes: vec!["user:email".to_string()],
        });
        
        let resolver = AuthResolver::new(config).unwrap();
        
        // Tool with no specific auth should use server-level
        let auth_method = resolver.resolve_auth_for_tool("some_random_tool");
        
        assert!(auth_method.is_some());
        match auth_method.unwrap() {
            AuthMethod::OAuth { provider, scopes } => {
                assert_eq!(provider, "github");
                assert_eq!(scopes, vec!["user:email"]);
            }
            _ => panic!("Expected OAuth auth method"),
        }
    }

    #[test]
    fn test_no_auth_required() {
        let config = create_test_config();
        let resolver = AuthResolver::new(config).unwrap();
        
        // Tool with no auth configured should return None
        let auth_method = resolver.resolve_auth_for_tool("some_tool");
        assert!(auth_method.is_none());
    }

    #[test]
    fn test_auth_disabled() {
        let mut config = create_test_config();
        config.enabled = false;
        
        let resolver = AuthResolver::new(config).unwrap();
        let auth_method = resolver.resolve_auth_for_tool("any_tool");
        assert!(auth_method.is_none());
    }

    #[test]
    fn test_capability_extraction() {
        let config = create_test_config();
        let resolver = AuthResolver::new(config).unwrap();
        
        // Test different naming patterns
        assert_eq!(
            resolver.extract_capability_from_tool("github.create_issue"),
            Some("github".to_string())
        );
        assert_eq!(
            resolver.extract_capability_from_tool("github_create_issue"),
            Some("github".to_string())
        );
        assert_eq!(
            resolver.extract_capability_from_tool("github-create-issue"),
            Some("github".to_string())
        );
        assert_eq!(
            resolver.extract_capability_from_tool("GithubCreateIssue"),
            Some("Github".to_string())
        );
        assert_eq!(
            resolver.extract_capability_from_tool("simpletools"),
            None
        );
    }

    #[test]
    fn test_provider_validation() {
        let config = create_test_config();
        let resolver = AuthResolver::new(config).unwrap();
        
        assert!(resolver.validate_oauth_provider("github").is_ok());
        assert!(resolver.validate_oauth_provider("nonexistent").is_err());
    }

    #[test]
    fn test_auth_stats() {
        let mut config = create_test_config();
        config.server_level = Some(AuthMethod::OAuth {
            provider: "github".to_string(),
            scopes: vec!["user:email".to_string()],
        });
        
        let resolver = AuthResolver::new(config).unwrap();
        let stats = resolver.get_auth_stats();
        
        assert!(stats.enabled);
        assert_eq!(stats.total_oauth_providers, 1);
        assert_eq!(stats.total_api_keys, 1);
        assert!(stats.server_level_auth);
        assert_eq!(stats.auth_method_distribution.get("oauth"), Some(&1));
        assert!(!stats.has_user_context);
        assert!(stats.user_id.is_none());
        assert!(!stats.has_secure_storage);
        assert!(stats.session_directory.is_none());
    }

    #[test]
    fn test_auth_resolver_with_user_context() {
        use crate::auth::user_context::UserContext;
        
        let config = create_test_config();
        let user_context = UserContext::default();
        
        let resolver = AuthResolver::with_user_context(config, user_context).unwrap();
        
        // Test that user context is properly set
        assert!(resolver.get_user_context().is_some());
        assert!(resolver.get_unique_user_id().is_some());
        
        // Test session file paths
        let session_file = resolver.get_session_file_path("test.json");
        assert!(session_file.is_some());
        
        let hostname_file = resolver.get_hostname_session_file_path("test.json");
        assert!(hostname_file.is_some());
        
        // Test stats with user context
        let stats = resolver.get_auth_stats();
        assert!(stats.has_user_context);
        assert!(stats.user_id.is_some());
        assert!(stats.session_directory.is_some());
    }

    #[test]
    fn test_user_context_integration() {
        use crate::auth::user_context::UserContext;
        
        let config = create_test_config();
        let mut resolver = AuthResolver::new(config).unwrap();
        
        // Initially no user context
        assert!(resolver.get_user_context().is_none());
        
        // Set user context
        let user_context = UserContext::default();
        resolver.set_user_context(user_context).unwrap();
        
        // Now has user context
        assert!(resolver.get_user_context().is_some());
        assert!(resolver.get_unique_user_id().is_some());
    }
}