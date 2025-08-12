//! Service Account authentication implementation
//! 
//! This module provides validation for service account authentication including:
//! - Personal Access Tokens (GitHub PAT, etc.)
//! - Service Account Keys (Google service account keys, etc.)
//! - Application Credentials
//! - Custom service account types

use crate::auth::config::{ServiceAccountConfig, ServiceAccountType};
use crate::error::{ProxyError, Result};
use actix_web::HttpRequest;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use tokio::time::{timeout, Duration};

/// Service account validation result containing user information
#[derive(Debug, Clone)]
pub struct ServiceAccountValidationResult {
    /// Service account user information
    pub user_info: ServiceAccountUserInfo,
    /// RBAC permissions for this service account
    pub permissions: Vec<String>,
    /// Service account type
    pub account_type: ServiceAccountType,
    /// Provider-specific metadata
    pub metadata: HashMap<String, String>,
    /// Token expiration (if applicable)
    pub expires_at: Option<u64>,
}

/// Service account user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccountUserInfo {
    /// Unique service account identifier
    pub id: String,
    /// Service account name/title
    pub name: Option<String>,
    /// Service account email (if available)
    pub email: Option<String>,
    /// Provider-specific login/username
    pub login: Option<String>,
}

/// Service account authentication validator
pub struct ServiceAccountValidator {
    /// HTTP client for making validation requests
    client: Arc<Client>,
    /// Service account configurations
    configs: HashMap<String, ServiceAccountConfig>,
}

impl ServiceAccountValidator {
    /// Create a new service account validator
    pub fn new(configs: HashMap<String, ServiceAccountConfig>) -> Self {
        let client = Arc::new(
            Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent(format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap_or_else(|_| Client::new())
        );

        Self { client, configs }
    }

    /// Check if service account authentication is configured
    pub fn is_enabled(&self) -> bool {
        !self.configs.is_empty()
    }

    /// Validate an HTTP request for service account authentication
    pub async fn validate_request(
        &self,
        req: &HttpRequest,
    ) -> Result<Option<ServiceAccountValidationResult>> {
        if !self.is_enabled() {
            debug!("Service account authentication not configured");
            return Ok(None);
        }

        // Extract service account credentials from request
        let (account_ref, credentials) = match self.extract_credentials(req) {
            Ok(Some(creds)) => creds,
            Ok(None) => {
                debug!("No service account credentials found in request");
                return Ok(None);
            }
            Err(e) => {
                debug!("Failed to extract service account credentials: {}", e);
                return Err(e);
            }
        };

        // Get service account configuration
        let config = self.configs.get(&account_ref).ok_or_else(|| {
            ProxyError::auth(format!("Unknown service account reference: {}", account_ref))
        })?;

        // Validate credentials against the service account type
        self.validate_credentials(config, &credentials).await
    }

    /// Extract service account credentials from HTTP request
    fn extract_credentials(
        &self, 
        req: &HttpRequest
    ) -> Result<Option<(String, String)>> {
        // Try X-Service-Account-Ref header first
        if let Some(account_ref) = req.headers().get("X-Service-Account-Ref") {
            let account_ref = account_ref.to_str()
                .map_err(|_| ProxyError::auth("Invalid X-Service-Account-Ref header"))?
                .to_string();

            // Try X-Service-Account-Token header for credentials
            if let Some(token) = req.headers().get("X-Service-Account-Token") {
                let token = token.to_str()
                    .map_err(|_| ProxyError::auth("Invalid X-Service-Account-Token header"))?
                    .to_string();
                
                return Ok(Some((account_ref, token)));
            }

            // Try Authorization header with "ServiceAccount" prefix
            if let Some(auth_header) = req.headers().get("Authorization") {
                let auth_value = auth_header.to_str()
                    .map_err(|_| ProxyError::auth("Invalid Authorization header"))?;

                if let Some(token) = auth_value.strip_prefix("ServiceAccount ") {
                    return Ok(Some((account_ref, token.to_string())));
                }
            }

            return Err(ProxyError::auth("Service account reference provided but no credentials found"));
        }

        // Try Authorization header with "ServiceAccount" scheme
        if let Some(auth_header) = req.headers().get("Authorization") {
            let auth_value = auth_header.to_str()
                .map_err(|_| ProxyError::auth("Invalid Authorization header"))?;

            if let Some(token_part) = auth_value.strip_prefix("ServiceAccount ") {
                // Look for account_ref:token format
                if let Some((account_ref, token)) = token_part.split_once(':') {
                    return Ok(Some((account_ref.to_string(), token.to_string())));
                }
                
                // If no account_ref specified, try to find default account
                if self.configs.len() == 1 {
                    let default_account = self.configs.keys().next().unwrap().clone();
                    return Ok(Some((default_account, token_part.to_string())));
                }
            }
        }

        Ok(None)
    }

    /// Validate service account credentials based on account type
    async fn validate_credentials(
        &self,
        config: &ServiceAccountConfig,
        provided_credentials: &str,
    ) -> Result<Option<ServiceAccountValidationResult>> {
        debug!("Validating service account credentials for type: {:?}", config.account_type);

        match &config.account_type {
            ServiceAccountType::PersonalAccessToken => {
                self.validate_personal_access_token(config, provided_credentials).await
            }
            ServiceAccountType::ServiceKey => {
                self.validate_service_key(config, provided_credentials).await
            }
            ServiceAccountType::ApplicationCredentials => {
                self.validate_application_credentials(config, provided_credentials).await
            }
            ServiceAccountType::Custom(custom_type) => {
                self.validate_custom_service_account(config, provided_credentials, custom_type).await
            }
        }
    }

    /// Validate Personal Access Token (GitHub PAT, etc.)
    async fn validate_personal_access_token(
        &self,
        config: &ServiceAccountConfig,
        provided_token: &str,
    ) -> Result<Option<ServiceAccountValidationResult>> {
        // Check if the provided token matches the configured credentials
        if provided_token != config.credentials {
            debug!("Personal access token validation failed: token mismatch");
            return Err(ProxyError::auth("Invalid personal access token"));
        }

        // Determine provider from configuration
        let provider = config.provider_config.as_ref()
            .and_then(|pc| pc.get("provider"))
            .map(|s| s.as_str())
            .unwrap_or("unknown");

        match provider {
            "github" => self.validate_github_pat(config, provided_token).await,
            "gitlab" => self.validate_gitlab_pat(config, provided_token).await,
            _ => {
                // For unknown providers, do basic validation
                info!("Validating personal access token for unknown provider: {}", provider);
                Ok(Some(self.create_basic_validation_result(config, provider)?))
            }
        }
    }

    /// Validate GitHub Personal Access Token
    async fn validate_github_pat(
        &self,
        config: &ServiceAccountConfig,
        token: &str,
    ) -> Result<Option<ServiceAccountValidationResult>> {
        let url = "https://api.github.com/user";
        
        debug!("Validating GitHub PAT by fetching user information");
        
        let response = timeout(
            Duration::from_secs(10),
            self.client
                .get(url)
                .header("Authorization", format!("token {}", token))
                .header("User-Agent", format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
                .send()
        ).await
        .map_err(|_| ProxyError::auth("GitHub API request timeout"))?
        .map_err(|e| ProxyError::auth(format!("GitHub API request failed: {}", e)))?;

        if !response.status().is_success() {
            match response.status().as_u16() {
                401 => return Err(ProxyError::auth("Invalid GitHub personal access token")),
                403 => return Err(ProxyError::auth("GitHub personal access token has insufficient permissions")),
                404 => return Err(ProxyError::auth("GitHub user not found")),
                _ => return Err(ProxyError::auth(format!("GitHub API error: {}", response.status()))),
            }
        }

        #[derive(Deserialize)]
        struct GitHubUser {
            id: u64,
            login: String,
            name: Option<String>,
            email: Option<String>,
        }

        let user: GitHubUser = response.json().await
            .map_err(|e| ProxyError::auth(format!("Failed to parse GitHub user response: {}", e)))?;

        let mut metadata = HashMap::new();
        metadata.insert("provider".to_string(), "github".to_string());
        metadata.insert("github_id".to_string(), user.id.to_string());

        let user_info = ServiceAccountUserInfo {
            id: format!("github:{}", user.login),
            name: user.name.clone(),
            email: user.email.clone(),
            login: Some(user.login.clone()),
        };

        let permissions = config.rbac_roles.clone().unwrap_or_else(|| {
            // Default permissions for GitHub PAT
            vec!["read".to_string(), "write".to_string()]
        });

        info!(
            user_id = %user_info.id,
            user_login = %user.login,
            "GitHub personal access token validated successfully"
        );

        Ok(Some(ServiceAccountValidationResult {
            user_info,
            permissions,
            account_type: ServiceAccountType::PersonalAccessToken,
            metadata,
            expires_at: None, // GitHub PATs don't typically have expiration
        }))
    }

    /// Validate GitLab Personal Access Token
    async fn validate_gitlab_pat(
        &self,
        config: &ServiceAccountConfig,
        token: &str,
    ) -> Result<Option<ServiceAccountValidationResult>> {
        // GitLab API endpoint (use gitlab.com by default, but could be configurable)
        let base_url = config.provider_config.as_ref()
            .and_then(|pc| pc.get("api_base_url"))
            .map(|s| s.as_str())
            .unwrap_or("https://gitlab.com/api/v4");
        let url = format!("{}/user", base_url);
        
        debug!("Validating GitLab PAT by fetching user information");
        
        let response = timeout(
            Duration::from_secs(10),
            self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
        ).await
        .map_err(|_| ProxyError::auth("GitLab API request timeout"))?
        .map_err(|e| ProxyError::auth(format!("GitLab API request failed: {}", e)))?;

        if !response.status().is_success() {
            match response.status().as_u16() {
                401 => return Err(ProxyError::auth("Invalid GitLab personal access token")),
                403 => return Err(ProxyError::auth("GitLab personal access token has insufficient permissions")),
                _ => return Err(ProxyError::auth(format!("GitLab API error: {}", response.status()))),
            }
        }

        #[derive(Deserialize)]
        struct GitLabUser {
            id: u64,
            username: String,
            name: Option<String>,
            email: Option<String>,
        }

        let user: GitLabUser = response.json().await
            .map_err(|e| ProxyError::auth(format!("Failed to parse GitLab user response: {}", e)))?;

        let mut metadata = HashMap::new();
        metadata.insert("provider".to_string(), "gitlab".to_string());
        metadata.insert("gitlab_id".to_string(), user.id.to_string());

        let user_info = ServiceAccountUserInfo {
            id: format!("gitlab:{}", user.username),
            name: user.name.clone(),
            email: user.email.clone(),
            login: Some(user.username.clone()),
        };

        let permissions = config.rbac_roles.clone().unwrap_or_else(|| {
            vec!["read".to_string(), "write".to_string()]
        });

        info!(
            user_id = %user_info.id,
            user_login = %user.username,
            "GitLab personal access token validated successfully"
        );

        Ok(Some(ServiceAccountValidationResult {
            user_info,
            permissions,
            account_type: ServiceAccountType::PersonalAccessToken,
            metadata,
            expires_at: None,
        }))
    }

    /// Validate Service Account Key (Google service account, etc.)
    async fn validate_service_key(
        &self,
        config: &ServiceAccountConfig,
        provided_credentials: &str,
    ) -> Result<Option<ServiceAccountValidationResult>> {
        // Check if the provided credentials match the configured credentials
        if provided_credentials != config.credentials {
            debug!("Service account key validation failed: credentials mismatch");
            return Err(ProxyError::auth("Invalid service account key"));
        }

        // Determine provider from configuration
        let provider = config.provider_config.as_ref()
            .and_then(|pc| pc.get("provider"))
            .map(|s| s.as_str())
            .unwrap_or("unknown");

        match provider {
            "google" => self.validate_google_service_key(config, provided_credentials).await,
            _ => {
                info!("Validating service key for unknown provider: {}", provider);
                Ok(Some(self.create_basic_validation_result(config, provider)?))
            }
        }
    }

    /// Validate Google Service Account Key
    async fn validate_google_service_key(
        &self,
        config: &ServiceAccountConfig,
        _credentials: &str,
    ) -> Result<Option<ServiceAccountValidationResult>> {
        // For Google service accounts, the credentials are typically JSON key files
        // In a real implementation, you would parse the JSON and validate the service account
        
        #[derive(Deserialize)]
        struct GoogleServiceAccount {
            #[serde(rename = "type")]
            account_type: String,
            project_id: String,
            private_key_id: String,
            client_email: String,
            client_id: String,
        }

        let service_account: GoogleServiceAccount = serde_json::from_str(&config.credentials)
            .map_err(|e| ProxyError::auth(format!("Invalid Google service account key: {}", e)))?;

        if service_account.account_type != "service_account" {
            return Err(ProxyError::auth("Invalid Google service account type"));
        }

        let mut metadata = HashMap::new();
        metadata.insert("provider".to_string(), "google".to_string());
        metadata.insert("project_id".to_string(), service_account.project_id);
        metadata.insert("client_id".to_string(), service_account.client_id);

        let user_info = ServiceAccountUserInfo {
            id: format!("google:{}", service_account.client_email),
            name: Some(service_account.client_email.clone()),
            email: Some(service_account.client_email.clone()),
            login: Some(service_account.client_email),
        };

        let permissions = config.rbac_roles.clone().unwrap_or_else(|| {
            vec!["read".to_string(), "write".to_string()]
        });

        info!(
            user_id = %user_info.id,
            "Google service account key validated successfully"
        );

        Ok(Some(ServiceAccountValidationResult {
            user_info,
            permissions,
            account_type: ServiceAccountType::ServiceKey,
            metadata,
            expires_at: None,
        }))
    }

    /// Validate Application Credentials
    async fn validate_application_credentials(
        &self,
        config: &ServiceAccountConfig,
        provided_credentials: &str,
    ) -> Result<Option<ServiceAccountValidationResult>> {
        // Basic credential validation
        if provided_credentials != config.credentials {
            debug!("Application credentials validation failed: credentials mismatch");
            return Err(ProxyError::auth("Invalid application credentials"));
        }

        let provider = config.provider_config.as_ref()
            .and_then(|pc| pc.get("provider"))
            .map(|s| s.as_str())
            .unwrap_or("application");

        info!("Application credentials validated successfully for provider: {}", provider);

        Ok(Some(self.create_basic_validation_result(config, provider)?))
    }

    /// Validate Custom Service Account type
    async fn validate_custom_service_account(
        &self,
        config: &ServiceAccountConfig,
        provided_credentials: &str,
        custom_type: &str,
    ) -> Result<Option<ServiceAccountValidationResult>> {
        // Basic credential validation for custom types
        if provided_credentials != config.credentials {
            debug!("Custom service account validation failed: credentials mismatch for type: {}", custom_type);
            return Err(ProxyError::auth(format!("Invalid {} service account credentials", custom_type)));
        }

        let provider = config.provider_config.as_ref()
            .and_then(|pc| pc.get("provider"))
            .map(|s| s.as_str())
            .unwrap_or(custom_type);

        info!("Custom service account validated successfully for type: {} provider: {}", custom_type, provider);

        Ok(Some(ServiceAccountValidationResult {
            user_info: ServiceAccountUserInfo {
                id: format!("{}:{}", provider, "service_account"),
                name: Some(format!("{} Service Account", custom_type)),
                email: None,
                login: Some("service_account".to_string()),
            },
            permissions: config.rbac_roles.clone().unwrap_or_else(|| {
                vec!["read".to_string(), "write".to_string()]
            }),
            account_type: ServiceAccountType::Custom(custom_type.to_string()),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("provider".to_string(), provider.to_string());
                metadata.insert("custom_type".to_string(), custom_type.to_string());
                metadata
            },
            expires_at: None,
        }))
    }

    /// Create a basic validation result for unknown providers
    fn create_basic_validation_result(
        &self,
        config: &ServiceAccountConfig,
        provider: &str,
    ) -> Result<ServiceAccountValidationResult> {
        let user_info = ServiceAccountUserInfo {
            id: format!("{}:service_account", provider),
            name: Some(format!("{} Service Account", provider)),
            email: None,
            login: Some("service_account".to_string()),
        };

        let permissions = config.rbac_roles.clone().unwrap_or_else(|| {
            vec!["read".to_string(), "write".to_string()]
        });

        let mut metadata = HashMap::new();
        metadata.insert("provider".to_string(), provider.to_string());

        Ok(ServiceAccountValidationResult {
            user_info,
            permissions,
            account_type: config.account_type.clone(),
            metadata,
            expires_at: None,
        })
    }

    /// Check if a service account has a specific permission
    pub fn check_permission(&self, result: &ServiceAccountValidationResult, permission: &str) -> bool {
        let has_permission = result.permissions.contains(&permission.to_string());
        
        debug!(
            user_id = %result.user_info.id,
            permission = permission,
            has_permission = has_permission,
            "Service account permission check"
        );

        has_permission
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_github_config() -> ServiceAccountConfig {
        ServiceAccountConfig {
            account_type: ServiceAccountType::PersonalAccessToken,
            credentials: "ghp_test_token_123".to_string(),
            rbac_user_id: Some("test_user".to_string()),
            rbac_roles: Some(vec!["read".to_string(), "write".to_string()]),
            provider_config: Some({
                let mut config = HashMap::new();
                config.insert("provider".to_string(), "github".to_string());
                config
            }),
        }
    }

    #[test]
    fn test_service_account_validator_creation() {
        let mut configs = HashMap::new();
        configs.insert("github_pat".to_string(), create_test_github_config());

        let validator = ServiceAccountValidator::new(configs);
        assert!(validator.is_enabled());
    }

    #[test]
    fn test_basic_validation_result() {
        let mut configs = HashMap::new();
        configs.insert("test_account".to_string(), create_test_github_config());

        let validator = ServiceAccountValidator::new(configs);
        let config = create_test_github_config();
        
        let result = validator.create_basic_validation_result(&config, "test").unwrap();
        
        assert_eq!(result.user_info.id, "test:service_account");
        assert_eq!(result.permissions, vec!["read", "write"]);
        assert!(result.metadata.contains_key("provider"));
        assert_eq!(result.metadata.get("provider"), Some(&"test".to_string()));
    }

    #[test]
    fn test_permission_check() {
        let mut configs = HashMap::new();
        configs.insert("test_account".to_string(), create_test_github_config());

        let validator = ServiceAccountValidator::new(configs);
        let config = create_test_github_config();
        let result = validator.create_basic_validation_result(&config, "test").unwrap();

        assert!(validator.check_permission(&result, "read"));
        assert!(validator.check_permission(&result, "write"));
        assert!(!validator.check_permission(&result, "admin"));
    }
}