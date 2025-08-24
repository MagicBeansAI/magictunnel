//! OAuth 2.0 authentication integration tests

use actix_web::test::TestRequest;
use magictunnel::auth::{AuthenticationMiddleware, AuthenticationResult, OAuthValidator};
use magictunnel::config::{AuthConfig, AuthType, OAuthConfig};

fn create_test_oauth_config() -> AuthConfig {
    let mut config = AuthConfig::default();
    config.enabled = true;
    config.r#type = AuthType::OAuth;
    config.oauth = Some(OAuthConfig {
        provider: "github".to_string(),
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string().into(),
        auth_url: "https://github.com/login/oauth/authorize".to_string(),
        token_url: "https://github.com/login/oauth/access_token".to_string(),
        oauth_2_1_enabled: true,
        resource_indicators_enabled: false,
        default_resources: Vec::new(),
        default_audience: Vec::new(),
        require_explicit_resources: false,
    });
    config
}

fn create_test_oauth_config_google() -> AuthConfig {
    let mut config = AuthConfig::default();
    config.enabled = true;
    config.r#type = AuthType::OAuth;
    config.oauth = Some(OAuthConfig {
        provider: "google".to_string(),
        client_id: "test_google_client_id".to_string(),
        client_secret: "test_google_client_secret".to_string().into(),
        auth_url: "https://accounts.google.com/o/oauth2/auth".to_string(),
        token_url: "https://oauth2.googleapis.com/token".to_string(),
        oauth_2_1_enabled: true,
        resource_indicators_enabled: false,
        default_resources: Vec::new(),
        default_audience: Vec::new(),
        require_explicit_resources: false,
    });
    config
}

#[cfg(test)]
mod oauth_validator_tests {
    use super::*;

    #[test]
    fn test_oauth_validator_creation() {
        let config = create_test_oauth_config();
        let _validator = OAuthValidator::new(config);
        // Validator should be created successfully
        assert!(true);
    }

    #[test]
    fn test_extract_access_token_success() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);

        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer github_access_token_123"))
            .to_http_request();

        let result = validator.extract_access_token(&req).unwrap();
        assert_eq!(result, "github_access_token_123");
    }

    #[test]
    fn test_extract_access_token_missing_header() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);

        let req = TestRequest::default().to_http_request();

        let result = validator.extract_access_token(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing Authorization header"));
    }

    #[test]
    fn test_extract_access_token_invalid_format() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);

        let req = TestRequest::default()
            .insert_header(("Authorization", "InvalidFormat token123"))
            .to_http_request();

        let result = validator.extract_access_token(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid Authorization header format"));
    }

    #[test]
    fn test_get_user_info_url_github() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);

        let oauth_config = validator.config().oauth.as_ref().unwrap();
        let url = validator.get_user_info_url(oauth_config).unwrap();
        assert_eq!(url, "https://api.github.com/user");
    }

    #[test]
    fn test_get_user_info_url_google() {
        let config = create_test_oauth_config_google();
        let validator = OAuthValidator::new(config);

        let oauth_config = validator.config().oauth.as_ref().unwrap();
        let url = validator.get_user_info_url(oauth_config).unwrap();
        assert_eq!(url, "https://www.googleapis.com/oauth2/v2/userinfo");
    }

    #[test]
    fn test_get_authorization_url_github() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);

        let url = validator.get_authorization_url("http://localhost:8080/callback", "test_state").unwrap();
        
        assert!(url.contains("https://github.com/login/oauth/authorize"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("state=test_state"));
        assert!(url.contains("scope=user%3Aemail"));
    }

    #[test]
    fn test_get_authorization_url_google() {
        let config = create_test_oauth_config_google();
        let validator = OAuthValidator::new(config);

        let url = validator.get_authorization_url("http://localhost:8080/callback", "test_state").unwrap();
        
        assert!(url.contains("https://accounts.google.com/o/oauth2/auth"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=test_google_client_id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("state=test_state"));
        assert!(url.contains("scope=openid%20email%20profile"));
    }

    #[test]
    fn test_get_default_scope() {
        let config = create_test_oauth_config();
        let validator = OAuthValidator::new(config);

        assert_eq!(validator.get_default_scope("github"), "user:email");
        assert_eq!(validator.get_default_scope("google"), "openid email profile");
        assert_eq!(validator.get_default_scope("microsoft"), "openid profile email");
        assert_eq!(validator.get_default_scope("custom"), "openid email profile");
    }

    #[tokio::test]
    async fn test_oauth_disabled_validation() {
        let mut config = create_test_oauth_config();
        config.enabled = false;
        let validator = OAuthValidator::new(config);

        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer some_token"))
            .to_http_request();

        let result = validator.validate_request(&req).await.unwrap();
        assert!(result.is_none()); // Should return None when disabled
    }

    #[tokio::test]
    async fn test_oauth_wrong_auth_type() {
        let mut config = create_test_oauth_config();
        config.r#type = AuthType::ApiKey; // Wrong auth type
        let validator = OAuthValidator::new(config);

        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer some_token"))
            .to_http_request();

        let result = validator.validate_request(&req).await.unwrap();
        assert!(result.is_none()); // Should return None for wrong auth type
    }
}

#[cfg(test)]
mod oauth_middleware_tests {
    use super::*;

    #[test]
    fn test_oauth_middleware_creation() {
        let config = create_test_oauth_config();
        let middleware = AuthenticationMiddleware::new(config).unwrap();
        assert!(middleware.is_logging_enabled());
    }

    #[tokio::test]
    async fn test_oauth_middleware_disabled_auth() {
        let mut config = create_test_oauth_config();
        config.enabled = false;
        let middleware = AuthenticationMiddleware::new(config).unwrap();

        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer some_token"))
            .to_http_request();

        let result = middleware.validate_http_request(&req).await.unwrap();
        assert!(result.is_none()); // Should return None when disabled
    }

    #[test]
    fn test_oauth_authentication_result_permissions() {
        use magictunnel::auth::OAuthUserInfo;
        use magictunnel::auth::OAuthValidationResult;

        let user_info = OAuthUserInfo {
            id: "12345".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            login: Some("testuser".to_string()),
        };

        let oauth_result = OAuthValidationResult {
            user_info,
            expires_at: Some(1234567890),
            scopes: vec!["read".to_string(), "write".to_string()],
            audience: None,
            issuer: None,
            resources: Some(Vec::new()),
            access_token: Some("test_access_token".to_string()),
        };

        let auth_result = AuthenticationResult::OAuth(oauth_result);
        
        let permissions = auth_result.get_permissions();
        assert!(permissions.contains(&"read".to_string()));
        assert!(permissions.contains(&"write".to_string()));
        
        let user_id = auth_result.get_user_id();
        assert_eq!(user_id, "12345");
    }

    #[test]
    fn test_oauth_permission_check() {
        use magictunnel::auth::OAuthUserInfo;
        use magictunnel::auth::OAuthValidationResult;

        let config = create_test_oauth_config();
        let middleware = AuthenticationMiddleware::new(config).unwrap();

        let user_info = OAuthUserInfo {
            id: "12345".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            login: Some("testuser".to_string()),
        };

        let oauth_result = OAuthValidationResult {
            user_info,
            expires_at: Some(1234567890),
            scopes: vec!["read".to_string(), "write".to_string()],
            audience: None,
            issuer: None,
            resources: Some(Vec::new()),
            access_token: Some("test_access_token".to_string()),
        };

        let auth_result = AuthenticationResult::OAuth(oauth_result);

        // OAuth users get default read/write permissions
        assert!(middleware.check_permission(&auth_result, "read"));
        assert!(middleware.check_permission(&auth_result, "write"));
        assert!(!middleware.check_permission(&auth_result, "admin")); // No admin by default
    }
}

#[cfg(test)]
mod oauth_config_tests {
    use super::*;

    #[test]
    fn test_oauth_config_validation_success() {
        let config = create_test_oauth_config();
        let oauth_config = config.oauth.unwrap();
        
        // Should validate successfully
        assert!(oauth_config.validate().is_ok());
    }

    #[test]
    fn test_oauth_config_validation_empty_provider() {
        let mut config = create_test_oauth_config();
        if let Some(ref mut oauth_config) = config.oauth {
            oauth_config.provider = "".to_string();
            assert!(oauth_config.validate().is_err());
        }
    }

    #[test]
    fn test_oauth_config_validation_empty_client_id() {
        let mut config = create_test_oauth_config();
        if let Some(ref mut oauth_config) = config.oauth {
            oauth_config.client_id = "".to_string();
            assert!(oauth_config.validate().is_err());
        }
    }

    #[test]
    fn test_oauth_config_validation_empty_urls() {
        let mut config = create_test_oauth_config();
        if let Some(ref mut oauth_config) = config.oauth {
            oauth_config.auth_url = "".to_string();
            assert!(oauth_config.validate().is_err());
        }

        let mut config2 = create_test_oauth_config();
        if let Some(ref mut oauth_config) = config2.oauth {
            oauth_config.token_url = "".to_string();
            assert!(oauth_config.validate().is_err());
        }
    }
}
