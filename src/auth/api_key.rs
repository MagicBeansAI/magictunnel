//! API Key authentication implementation

use crate::config::{AuthConfig, ApiKeyEntry, AuthType};
use crate::error::{ProxyError, Result};
use actix_web::HttpRequest;
use tracing::{debug, warn};

/// API Key authentication validator
pub struct ApiKeyValidator {
    /// Authentication configuration
    config: AuthConfig,
}

impl ApiKeyValidator {
    /// Create a new API key validator
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    /// Check if authentication is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Validate an HTTP request for API key authentication
    pub fn validate_request(&self, req: &HttpRequest) -> Result<Option<ApiKeyEntry>> {
        // If authentication is disabled, allow all requests
        if !self.config.enabled {
            debug!("Authentication disabled, allowing request");
            return Ok(None);
        }

        // Only validate API key auth type
        if self.config.r#type != AuthType::ApiKey {
            debug!("Non-API key auth type, skipping API key validation");
            return Ok(None);
        }

        let api_key_config = match &self.config.api_keys {
            Some(config) => config,
            None => {
                warn!("API key authentication enabled but no API key configuration found");
                return Err(ProxyError::auth("API key configuration missing"));
            }
        };

        // Extract API key from request headers
        let api_key = self.extract_api_key(req, api_key_config)?;

        // Validate the API key
        self.validate_api_key(&api_key, api_key_config)
    }

    /// Extract API key from request headers
    fn extract_api_key(
        &self,
        req: &HttpRequest,
        api_key_config: &crate::config::ApiKeyConfig,
    ) -> Result<String> {
        let header_name = &api_key_config.header_name;
        let header_format = &api_key_config.header_format;

        // Get the header value
        let header_value = req
            .headers()
            .get(header_name)
            .ok_or_else(|| {
                ProxyError::auth(format!("Missing {} header", header_name))
            })?
            .to_str()
            .map_err(|_| {
                ProxyError::auth("Invalid header value encoding")
            })?;

        // Extract key from header format
        // For "Bearer {key}" format, extract the key part
        if header_format.contains("{key}") {
            let prefix = header_format.replace("{key}", "");
            if header_value.starts_with(&prefix) {
                let key = header_value.strip_prefix(&prefix).unwrap_or(header_value);
                Ok(key.trim().to_string())
            } else {
                Err(ProxyError::auth(format!(
                    "Invalid header format. Expected: {}",
                    header_format.replace("{key}", "<api-key>")
                )))
            }
        } else {
            // If no {key} placeholder, use the entire header value
            Ok(header_value.to_string())
        }
    }

    /// Validate an API key against the configuration
    fn validate_api_key(
        &self,
        api_key: &str,
        api_key_config: &crate::config::ApiKeyConfig,
    ) -> Result<Option<ApiKeyEntry>> {
        // Find the API key entry
        let key_entry = api_key_config
            .keys
            .iter()
            .find(|entry| entry.key == api_key)
            .ok_or_else(|| {
                warn!("Invalid API key attempted: {}", api_key);
                ProxyError::auth("Invalid API key")
            })?;

        // Check if the key is valid (active and not expired)
        if !key_entry.is_valid() {
            warn!("Expired or inactive API key attempted: {}", key_entry.name);
            return Err(ProxyError::auth("API key is expired or inactive"));
        }

        debug!("API key validation successful for: {}", key_entry.name);
        Ok(Some(key_entry.clone()))
    }

    /// Check if an API key has a specific permission
    pub fn check_permission(&self, key_entry: &ApiKeyEntry, permission: &str) -> bool {
        key_entry.has_permission(permission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ApiKeyConfig, ApiKeyEntry, AuthType};
    use actix_web::test::TestRequest;

    fn create_test_config() -> AuthConfig {
        let mut config = AuthConfig::default();
        config.enabled = true;
        config.r#type = AuthType::ApiKey;
        config.api_keys = Some(ApiKeyConfig {
            keys: vec![
                ApiKeyEntry::new("test_key_123456789".to_string(), "Test Key".to_string()),
                ApiKeyEntry {
                    key: "expired_key_123456789".to_string(),
                    name: "Expired Key".to_string(),
                    description: None,
                    permissions: vec!["read".to_string()],
                    expires_at: Some("2020-01-01T00:00:00Z".to_string()),
                    active: true,
                },
            ],
            require_header: true,
            header_name: "Authorization".to_string(),
            header_format: "Bearer {key}".to_string(),
        });
        config
    }

    #[test]
    fn test_valid_api_key() {
        let config = create_test_config();
        let validator = ApiKeyValidator::new(config);

        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer test_key_123456789"))
            .to_http_request();

        let result = validator.validate_request(&req).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Test Key");
    }

    #[test]
    fn test_invalid_api_key() {
        let config = create_test_config();
        let validator = ApiKeyValidator::new(config);

        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer invalid_key"))
            .to_http_request();

        let result = validator.validate_request(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_header() {
        let config = create_test_config();
        let validator = ApiKeyValidator::new(config);

        let req = TestRequest::default().to_http_request();

        let result = validator.validate_request(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_key() {
        let config = create_test_config();
        let validator = ApiKeyValidator::new(config);

        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer expired_key_123456789"))
            .to_http_request();

        let result = validator.validate_request(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_disabled_auth() {
        let mut config = create_test_config();
        config.enabled = false;
        let validator = ApiKeyValidator::new(config);

        let req = TestRequest::default().to_http_request();

        let result = validator.validate_request(&req).unwrap();
        assert!(result.is_none());
    }
}
