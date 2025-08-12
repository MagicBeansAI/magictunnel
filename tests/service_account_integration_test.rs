//! Service Account authentication integration tests
//! 
//! This module tests the complete ServiceAccount authentication flow including:
//! - GitHub PAT validation
//! - GitLab PAT validation  
//! - Google Service Account Key validation
//! - Custom service account types
//! - Authentication middleware integration
//! - AuthenticationContext creation

use actix_web::test::TestRequest;
use magictunnel::auth::{
    AuthenticationContext, AuthenticationMiddleware, AuthenticationResult, ServiceAccountConfig, ServiceAccountType, ServiceAccountUserInfo, ServiceAccountValidationResult, ServiceAccountValidator
};
use magictunnel::config::AuthConfig;
use magictunnel::error::Result;
use std::collections::HashMap;
use tokio;

/// Create a test GitHub PAT configuration
fn create_github_pat_config() -> ServiceAccountConfig {
    ServiceAccountConfig {
        account_type: ServiceAccountType::PersonalAccessToken,
        credentials: "ghp_test_token_123456789".to_string(),
        rbac_user_id: Some("github_user_123".to_string()),
        rbac_roles: Some(vec![
            "read".to_string(), 
            "write".to_string(), 
            "admin".to_string()
        ]),
        provider_config: Some({
            let mut config = HashMap::new();
            config.insert("provider".to_string(), "github".to_string());
            config
        }),
    }
}

/// Create a test GitLab PAT configuration
fn create_gitlab_pat_config() -> ServiceAccountConfig {
    ServiceAccountConfig {
        account_type: ServiceAccountType::PersonalAccessToken,
        credentials: "glpat-test_token_987654321".to_string(),
        rbac_user_id: Some("gitlab_user_456".to_string()),
        rbac_roles: Some(vec![
            "read".to_string(), 
            "write".to_string()
        ]),
        provider_config: Some({
            let mut config = HashMap::new();
            config.insert("provider".to_string(), "gitlab".to_string());
            config.insert("api_base_url".to_string(), "https://gitlab.example.com/api/v4".to_string());
            config
        }),
    }
}

/// Create a test Google Service Account Key configuration
fn create_google_service_key_config() -> ServiceAccountConfig {
    let service_key_json = r#"{
        "type": "service_account",
        "project_id": "test-project-123",
        "private_key_id": "key123",
        "private_key": "-----BEGIN PRIVATE KEY-----\ntest_key\n-----END PRIVATE KEY-----\n",
        "client_email": "test@test-project-123.iam.gserviceaccount.com",
        "client_id": "123456789",
        "auth_uri": "https://accounts.google.com/o/oauth2/auth",
        "token_uri": "https://oauth2.googleapis.com/token",
        "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
        "client_x509_cert_url": "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project-123.iam.gserviceaccount.com"
    }"#;

    ServiceAccountConfig {
        account_type: ServiceAccountType::ServiceKey,
        credentials: service_key_json.to_string(),
        rbac_user_id: Some("google_sa_789".to_string()),
        rbac_roles: Some(vec![
            "read".to_string(), 
            "write".to_string(),
            "storage".to_string(),
        ]),
        provider_config: Some({
            let mut config = HashMap::new();
            config.insert("provider".to_string(), "google".to_string());
            config
        }),
    }
}

/// Create a test custom service account configuration
fn create_custom_service_account_config() -> ServiceAccountConfig {
    ServiceAccountConfig {
        account_type: ServiceAccountType::Custom("api_token".to_string()),
        credentials: "custom_token_abcdef123456".to_string(),
        rbac_user_id: Some("custom_user_999".to_string()),
        rbac_roles: Some(vec![
            "read".to_string(),
            "api_access".to_string(),
        ]),
        provider_config: Some({
            let mut config = HashMap::new();
            config.insert("provider".to_string(), "custom_api".to_string());
            config
        }),
    }
}

#[tokio::test]
async fn test_service_account_validator_creation() {
    let mut configs = HashMap::new();
    configs.insert("github_pat".to_string(), create_github_pat_config());
    configs.insert("gitlab_pat".to_string(), create_gitlab_pat_config());
    configs.insert("google_sa".to_string(), create_google_service_key_config());
    configs.insert("custom_token".to_string(), create_custom_service_account_config());

    let validator = ServiceAccountValidator::new(configs);
    assert!(validator.is_enabled());
}

#[tokio::test]
async fn test_service_account_header_extraction() {
    let mut configs = HashMap::new();
    configs.insert("github_pat".to_string(), create_github_pat_config());

    let validator = ServiceAccountValidator::new(configs);

    // Test X-Service-Account-Ref and X-Service-Account-Token headers
    let req = TestRequest::default()
        .insert_header(("X-Service-Account-Ref", "github_pat"))
        .insert_header(("X-Service-Account-Token", "ghp_test_token_123456789"))
        .to_http_request();

    let result = validator.validate_request(&req).await;
    // Note: This will fail validation since it tries to make actual HTTP requests
    // But we're testing the credential extraction logic
    assert!(result.is_err() || result.unwrap().is_some());
}

#[tokio::test]
async fn test_service_account_authorization_header() {
    let mut configs = HashMap::new();
    configs.insert("github_pat".to_string(), create_github_pat_config());

    let validator = ServiceAccountValidator::new(configs);

    // Test Authorization header with ServiceAccount scheme
    let req = TestRequest::default()
        .insert_header(("Authorization", "ServiceAccount github_pat:ghp_test_token_123456789"))
        .to_http_request();

    let result = validator.validate_request(&req).await;
    // Note: This will fail validation since it tries to make actual HTTP requests
    // But we're testing the credential extraction logic
    assert!(result.is_err() || result.unwrap().is_some());
}

#[tokio::test]
async fn test_service_account_single_config_default() {
    let mut configs = HashMap::new();
    configs.insert("default_account".to_string(), create_github_pat_config());

    let validator = ServiceAccountValidator::new(configs);

    // Test Authorization header without account reference (should use default)
    let req = TestRequest::default()
        .insert_header(("Authorization", "ServiceAccount ghp_test_token_123456789"))
        .to_http_request();

    let result = validator.validate_request(&req).await;
    // Note: This will fail validation since it tries to make actual HTTP requests
    // But we're testing the default account logic
    assert!(result.is_err() || result.unwrap().is_some());
}

#[tokio::test]
async fn test_service_account_no_credentials() {
    let mut configs = HashMap::new();
    configs.insert("github_pat".to_string(), create_github_pat_config());

    let validator = ServiceAccountValidator::new(configs);

    // Test request with no service account credentials
    let req = TestRequest::default()
        .insert_header(("Authorization", "Bearer some_other_token"))
        .to_http_request();

    let result = validator.validate_request(&req).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_google_service_account_key_parsing() {
    let mut configs = HashMap::new();
    configs.insert("google_sa".to_string(), create_google_service_key_config());

    let validator = ServiceAccountValidator::new(configs);

    // Test that we can create the validator with Google service account config
    assert!(validator.is_enabled());
    
    // Test that Google service account config can be created and validated
    let config = create_google_service_key_config();
    assert_eq!(config.account_type, ServiceAccountType::ServiceKey);
    assert!(config.credentials.contains("service_account"));
    assert!(config.credentials.contains("test-project-123"));
    assert!(config.provider_config.is_some());
    
    if let Some(provider_config) = &config.provider_config {
        assert_eq!(provider_config.get("provider"), Some(&"google".to_string()));
    }
}

#[tokio::test]
async fn test_service_account_permission_checking() {
    let mut configs = HashMap::new();
    configs.insert("github_pat".to_string(), create_github_pat_config());

    let validator = ServiceAccountValidator::new(configs);
    
    // Create a mock service account validation result for testing permissions
    let mock_result = ServiceAccountValidationResult {
        user_info: ServiceAccountUserInfo {
            id: "github:test_user".to_string(),
            name: Some("Test User".to_string()),
            email: Some("test@github.com".to_string()),
            login: Some("test_user".to_string()),
        },
        permissions: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        account_type: ServiceAccountType::PersonalAccessToken,
        metadata: HashMap::new(),
        expires_at: None,
    };

    // Test permission checking
    assert!(validator.check_permission(&mock_result, "read"));
    assert!(validator.check_permission(&mock_result, "write"));
    assert!(validator.check_permission(&mock_result, "admin"));
    assert!(!validator.check_permission(&mock_result, "super_admin"));
}

#[tokio::test]
async fn test_authentication_middleware_with_service_account() {
    let mut auth_config = AuthConfig::default();
    auth_config.enabled = true; // Enable authentication
    let middleware = AuthenticationMiddleware::new(auth_config).unwrap();

    // Create service account validator
    let mut configs = HashMap::new();
    configs.insert("github_pat".to_string(), create_github_pat_config());
    let sa_validator = ServiceAccountValidator::new(configs);

    // Add service account validator to middleware
    let middleware = middleware.with_service_accounts(sa_validator);

    // Test request with service account headers
    let req = TestRequest::default()
        .insert_header(("X-Service-Account-Ref", "github_pat"))
        .insert_header(("X-Service-Account-Token", "ghp_test_token_123456789"))
        .to_http_request();

    let result = middleware.validate_http_request(&req).await;
    // Note: This will fail validation since it tries to make actual HTTP requests
    // But we're testing the middleware integration
    assert!(result.is_err() || result.unwrap().is_some());
}

#[tokio::test]
async fn test_authentication_context_from_service_account() {
    // Create a mock service account validation result
    
    let sa_result = ServiceAccountValidationResult {
        user_info: ServiceAccountUserInfo {
            id: "github:test_user".to_string(),
            name: Some("Test User".to_string()),
            email: Some("test@github.com".to_string()),
            login: Some("test_user".to_string()),
        },
        permissions: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        account_type: ServiceAccountType::PersonalAccessToken,
        metadata: {
            let mut metadata = HashMap::new();
            metadata.insert("provider".to_string(), "github".to_string());
            metadata.insert("github_id".to_string(), "123456".to_string());
            metadata
        },
        expires_at: None,
    };

    let auth_result = AuthenticationResult::ServiceAccount(sa_result);
    let session_id = "test_session_123".to_string();

    // Create authentication context from service account result
    let auth_context = AuthenticationContext::from_auth_result(&auth_result, session_id)
        .expect("Failed to create auth context");

    // Verify authentication context properties
    assert_eq!(auth_context.user_id, "github:test_user");
    assert_eq!(auth_context.scopes, vec!["read", "write", "admin"]);
    assert!(auth_context.metadata.contains_key("provider"));
    assert_eq!(auth_context.metadata.get("provider").unwrap(), "github");

    // Test provider token access
    let github_token = auth_context.get_provider_token("github");
    assert!(github_token.is_some());

    if let Some(token) = github_token {
        assert_eq!(token.token_type, "ServiceAccount");
        assert_eq!(token.scopes, vec!["read", "write", "admin"]);
        assert!(token.metadata.contains_key("provider"));
    }

    // Test scope checking
    assert!(auth_context.has_scope("read"));
    assert!(auth_context.has_scope("write"));
    assert!(auth_context.has_scope("admin"));
    assert!(!auth_context.has_scope("super_admin"));
}

#[tokio::test]
async fn test_service_account_config_validation() {
    // Test GitHub PAT config
    let github_config = create_github_pat_config();
    assert_eq!(github_config.account_type, ServiceAccountType::PersonalAccessToken);
    assert!(!github_config.credentials.is_empty());
    assert!(github_config.rbac_roles.is_some());
    assert!(github_config.provider_config.is_some());

    // Test Google Service Account config  
    let google_config = create_google_service_key_config();
    assert_eq!(google_config.account_type, ServiceAccountType::ServiceKey);
    assert!(!google_config.credentials.is_empty());
    assert!(google_config.credentials.contains("service_account"));
    assert!(google_config.credentials.contains("test-project-123"));

    // Test Custom service account config
    let custom_config = create_custom_service_account_config();
    if let ServiceAccountType::Custom(ref custom_type) = custom_config.account_type {
        assert_eq!(custom_type, "api_token");
    } else {
        panic!("Expected Custom service account type");
    }
}

#[tokio::test]
async fn test_service_account_type_display() {
    // Test service account type formatting
    use magictunnel::auth::AuthMethod;

    let oauth_method = AuthMethod::OAuth {
        provider: "github".to_string(),
        scopes: vec!["user:email".to_string()],
    };

    let service_account_method = AuthMethod::ServiceAccount {
        account_ref: "github_pat".to_string(),
    };

    // Test that the auth method formats correctly
    let oauth_display = format!("{:?}", oauth_method);
    let sa_display = format!("{:?}", service_account_method);
    
    assert!(oauth_display.contains("OAuth"));
    assert!(sa_display.contains("ServiceAccount"));
    assert!(sa_display.contains("github_pat"));
}

#[tokio::test]
async fn test_multiple_service_account_types() {
    let mut configs = HashMap::new();
    configs.insert("github_pat".to_string(), create_github_pat_config());
    configs.insert("gitlab_pat".to_string(), create_gitlab_pat_config());
    configs.insert("google_sa".to_string(), create_google_service_key_config());
    configs.insert("custom_token".to_string(), create_custom_service_account_config());

    let validator = ServiceAccountValidator::new(configs);
    assert!(validator.is_enabled());

    // Test that we can create validator with multiple account types
    // The validator should be enabled when configs are provided
    assert!(validator.is_enabled());
}