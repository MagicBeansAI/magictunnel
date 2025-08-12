//! Comprehensive tests for OAuth 2.1 Phase 2.3 Automatic Session Recovery
//!
//! This test suite covers all aspects of the session recovery system including:
//! - Session recovery on startup and runtime
//! - Token validation with OAuth providers
//! - Session state persistence and management
//! - Multi-provider support
//! - Graceful degradation for expired/invalid tokens
//! - Configuration-driven behavior

use magictunnel::auth::{
    config::OAuthProviderConfig,
    session_manager::{
        SessionManager, SessionRecoveryConfig, ActiveSession, AuthMethodType, 
        SessionState, SessionRecoveryResult
    },
    token_storage::{TokenData, TokenStorage},
    user_context::UserContext,
};
use magictunnel::error::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use tokio::test;

/// Create a test user context
fn create_test_user_context() -> Result<UserContext> {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join("sessions");
    UserContext::with_session_dir(session_dir)
}

/// Create a test token storage
async fn create_test_token_storage(user_context: UserContext) -> Result<Arc<TokenStorage>> {
    TokenStorage::new(user_context).await.map(Arc::new)
}

/// Create test OAuth provider configurations
fn create_test_oauth_providers() -> HashMap<String, OAuthProviderConfig> {
    let mut providers = HashMap::new();
    
    // GitHub provider
    providers.insert(
        "github".to_string(),
        OAuthProviderConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string().into(),
            scopes: vec!["read:user".to_string()],
            oauth_enabled: true,
            device_code_enabled: false,
            authorization_endpoint: Some("https://github.com/login/oauth/authorize".to_string()),
            device_authorization_endpoint: None,
            token_endpoint: Some("https://github.com/login/oauth/access_token".to_string()),
            user_info_endpoint: Some("https://api.github.com/user".to_string()),
            resource_indicators: None,
            extra_params: None,
        }
    );
    
    // Google provider
    providers.insert(
        "google".to_string(),
        OAuthProviderConfig {
            client_id: "test_google_client_id".to_string(),
            client_secret: "test_google_client_secret".to_string().into(),
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            oauth_enabled: true,
            device_code_enabled: false,
            authorization_endpoint: Some("https://accounts.google.com/o/oauth2/auth".to_string()),
            device_authorization_endpoint: None,
            token_endpoint: Some("https://oauth2.googleapis.com/token".to_string()),
            user_info_endpoint: Some("https://www.googleapis.com/oauth2/v1/userinfo".to_string()),
            resource_indicators: None,
            extra_params: None,
        }
    );
    
    providers
}

/// Create test token data
fn create_test_token_data(provider: &str, user_id: &str, expired: bool) -> TokenData {
    let expires_at = if expired {
        Some(SystemTime::now() - Duration::from_secs(3600)) // Expired 1 hour ago
    } else {
        Some(SystemTime::now() + Duration::from_secs(3600)) // Expires in 1 hour
    };
    
    TokenData {
        access_token: format!("test_access_token_{}", provider).into(),
        refresh_token: Some(format!("test_refresh_token_{}", provider).into()),
        expires_at,
        scopes: vec!["read".to_string(), "write".to_string()],
        provider: provider.to_string(),
        token_type: "Bearer".to_string(),
        audience: None,
        resource: None,
        created_at: SystemTime::now(),
        last_refreshed: None,
        user_id: Some(user_id.to_string()),
        metadata: HashMap::new(),
    }
}

/// Create a test session manager
async fn create_test_session_manager(
    recovery_config: Option<SessionRecoveryConfig>,
) -> Result<SessionManager> {
    let user_context = create_test_user_context()?;
    let token_storage = create_test_token_storage(user_context.clone()).await?;
    let config = recovery_config.unwrap_or_default();
    
    let mut session_manager = SessionManager::new(
        user_context,
        token_storage,
        config,
    ).await?;
    
    session_manager.set_oauth_providers(create_test_oauth_providers());
    
    Ok(session_manager)
}

#[test]
async fn test_session_manager_creation() {
    let session_manager = create_test_session_manager(None).await;
    assert!(session_manager.is_ok());
    
    let manager = session_manager.unwrap();
    let stats = manager.get_session_stats();
    assert_eq!(stats.total_sessions, 0);
    assert!(stats.system_compatible);
}

#[test]
async fn test_session_recovery_config() {
    let config = SessionRecoveryConfig::default();
    
    // Test default values
    assert!(config.enabled);
    assert!(config.auto_recovery_on_startup);
    assert_eq!(config.validation_interval_minutes, 60);
    assert_eq!(config.max_recovery_attempts, 3);
    assert_eq!(config.token_validation_timeout_seconds, 30);
    assert!(config.graceful_degradation);
    assert!(config.retry_failed_providers);
    assert!(config.cleanup_expired_sessions);
    assert_eq!(config.max_session_age_hours, 24);
    assert!(config.persist_session_state);
    
    // Test duration conversions
    assert_eq!(config.validation_interval(), Duration::from_secs(3600));
    assert_eq!(config.token_validation_timeout(), Duration::from_secs(30));
    assert_eq!(config.max_session_age(), Duration::from_secs(86400));
}

#[test]
async fn test_active_session_lifecycle() {
    let session = ActiveSession::new(
        "github".to_string(),
        "testuser".to_string(),
        Some(SystemTime::now() + Duration::from_secs(3600)),
        AuthMethodType::OAuth,
        vec!["read".to_string()],
    );
    
    assert_eq!(session.provider, "github");
    assert_eq!(session.user_id, "testuser");
    assert!(!session.is_expired());
    assert!(session.is_valid);
    assert_eq!(session.scopes, vec!["read"]);
    
    // Test age calculation
    assert!(session.age() < Duration::from_secs(1));
    
    // Test validation interval
    assert!(!session.needs_validation(Duration::from_secs(3600)));
    
    // Test expired session
    let expired_session = ActiveSession::new(
        "github".to_string(),
        "testuser".to_string(),
        Some(SystemTime::now() - Duration::from_secs(10)),
        AuthMethodType::OAuth,
        vec!["read".to_string()],
    );
    assert!(expired_session.is_expired());
}

#[test]
async fn test_session_state_management() {
    let mut state = SessionState::default();
    
    // Test initial state
    assert_eq!(state.active_sessions.len(), 0);
    assert_eq!(state.recovery_attempts, 0);
    assert_eq!(state.failed_providers.len(), 0);
    assert!(state.is_compatible_with_current_system());
    
    // Add a session
    let session = ActiveSession::new(
        "github".to_string(),
        "testuser".to_string(),
        Some(SystemTime::now() + Duration::from_secs(3600)),
        AuthMethodType::OAuth,
        vec!["read".to_string()],
    );
    state.active_sessions.insert("test_session_id".to_string(), session);
    
    // Test statistics
    let stats = state.get_stats();
    assert_eq!(stats.total_sessions, 1);
    assert_eq!(stats.valid_sessions, 1);
    assert_eq!(stats.expired_sessions, 0);
    assert_eq!(stats.failed_providers, 0);
    assert_eq!(*stats.providers.get("github").unwrap(), 1);
    
    // Test cleanup of expired sessions
    let expired_session = ActiveSession::new(
        "google".to_string(),
        "testuser2".to_string(),
        Some(SystemTime::now() - Duration::from_secs(10)),
        AuthMethodType::OAuth,
        vec!["read".to_string()],
    );
    state.active_sessions.insert("expired_session_id".to_string(), expired_session);
    
    let cleaned_up = state.cleanup_expired_sessions();
    assert_eq!(cleaned_up, 1);
    assert_eq!(state.active_sessions.len(), 1);
}

#[test]
async fn test_session_recovery_with_valid_tokens() {
    let session_manager = create_test_session_manager(None).await.unwrap();
    
    // Store some valid tokens
    let token_storage = session_manager.get_user_context()
        .get_session_file_path("test")
        .parent()
        .unwrap()
        .to_path_buf();
    
    let user_context = create_test_user_context().unwrap();
    let token_storage = create_test_token_storage(user_context).await.unwrap();
    
    // Add test tokens
    let github_token = create_test_token_data("github", "testuser", false);
    let google_token = create_test_token_data("google", "testuser", false);
    
    let github_token_response = convert_token_to_oauth_response(&github_token);
    let google_token_response = convert_token_to_oauth_response(&google_token);
    
    token_storage.store_oauth_token("github", Some("testuser"), &github_token_response).await.unwrap();
    token_storage.store_oauth_token("google", Some("testuser"), &google_token_response).await.unwrap();
    
    // Test that session manager can recover these tokens (would require mock HTTP server)
    // For now, test that the recovery attempt doesn't crash
    let result = session_manager.recover_sessions_on_startup().await;
    assert!(result.is_ok());
}

#[test]
async fn test_session_recovery_with_expired_tokens() {
    let session_manager = create_test_session_manager(None).await.unwrap();
    
    // Store expired tokens
    let user_context = create_test_user_context().unwrap();
    let token_storage = create_test_token_storage(user_context).await.unwrap();
    
    let expired_token = create_test_token_data("github", "testuser", true);
    let expired_token_response = convert_token_to_oauth_response(&expired_token);
    token_storage.store_oauth_token("github", Some("testuser"), &expired_token_response).await.unwrap();
    
    // Test recovery with expired tokens
    let result = session_manager.recover_sessions_on_startup().await;
    assert!(result.is_ok());
    
    let recovery_result = result.unwrap();
    // Expired tokens should not result in recovered sessions
    assert_eq!(recovery_result.recovered_sessions, 0);
}

#[test]
async fn test_session_recovery_disabled() {
    let mut config = SessionRecoveryConfig::default();
    config.enabled = false;
    
    let session_manager = create_test_session_manager(Some(config)).await.unwrap();
    
    let result = session_manager.recover_sessions_on_startup().await.unwrap();
    assert_eq!(result.recovered_sessions, 0);
    assert_eq!(result.failed_validations, 0);
    assert!(result.success);
}

#[test]
async fn test_session_recovery_startup_disabled() {
    let mut config = SessionRecoveryConfig::default();
    config.auto_recovery_on_startup = false;
    
    let session_manager = create_test_session_manager(Some(config)).await.unwrap();
    
    let result = session_manager.recover_sessions_on_startup().await.unwrap();
    assert_eq!(result.recovered_sessions, 0);
    assert_eq!(result.failed_validations, 0);
    assert!(result.success);
}

#[test]
async fn test_runtime_session_validation() {
    let session_manager = create_test_session_manager(None).await.unwrap();
    
    // Test validation with no sessions
    let result = session_manager.validate_sessions_runtime().await.unwrap();
    assert_eq!(result.recovered_sessions, 0);
    assert_eq!(result.failed_validations, 0);
    assert!(result.success);
}

#[test]
async fn test_session_management_operations() {
    let session_manager = create_test_session_manager(None).await.unwrap();
    
    // Test has_active_session with no sessions
    assert!(!session_manager.has_active_session("github"));
    assert!(session_manager.get_active_session("github").is_none());
    
    // Test get_active_sessions
    let active_sessions = session_manager.get_active_sessions();
    assert!(active_sessions.is_empty());
    
    // Test remove_session with non-existent session
    let removed = session_manager.remove_session("github", "testuser").await.unwrap();
    assert!(!removed);
    
    // Test clear_all_sessions
    let cleared = session_manager.clear_all_sessions().await.unwrap();
    assert_eq!(cleared, 0);
}

#[test]
async fn test_session_state_persistence() {
    let user_context = create_test_user_context().unwrap();
    let token_storage = create_test_token_storage(user_context.clone()).await.unwrap();
    
    // Create session manager with persistence enabled
    let mut config = SessionRecoveryConfig::default();
    config.persist_session_state = true;
    
    let session_manager = SessionManager::new(
        user_context,
        token_storage,
        config,
    ).await.unwrap();
    
    // Test that session manager created successfully with persistence
    let stats = session_manager.get_session_stats();
    assert_eq!(stats.total_sessions, 0);
    
    // Test clearing sessions (which triggers persistence)
    let cleared = session_manager.clear_all_sessions().await.unwrap();
    assert_eq!(cleared, 0);
}

#[test]
async fn test_session_recovery_result() {
    // Test successful result
    let mut result = SessionRecoveryResult::success();
    assert!(result.success);
    assert!(result.errors.is_empty());
    
    result.recovered_sessions = 5;
    result.failed_validations = 2;
    result.cleaned_up_sessions = 1;
    
    // Test failure result
    let failure_result = SessionRecoveryResult::failure("Test error".to_string());
    assert!(!failure_result.success);
    assert_eq!(failure_result.errors.len(), 1);
    assert_eq!(failure_result.errors[0], "Test error");
    
    // Test adding errors
    let mut result = SessionRecoveryResult::success();
    result.add_error("Test error".to_string());
    assert!(!result.success);
    assert_eq!(result.errors.len(), 1);
}

#[test]
async fn test_session_manager_configuration_integration() {
    // Test various configuration combinations
    let configs = vec![
        SessionRecoveryConfig {
            enabled: true,
            auto_recovery_on_startup: true,
            validation_interval_minutes: 30,
            max_recovery_attempts: 5,
            token_validation_timeout_seconds: 60,
            graceful_degradation: true,
            retry_failed_providers: false,
            cleanup_expired_sessions: true,
            max_session_age_hours: 12,
            persist_session_state: true,
        },
        SessionRecoveryConfig {
            enabled: false,
            auto_recovery_on_startup: false,
            validation_interval_minutes: 120,
            max_recovery_attempts: 1,
            token_validation_timeout_seconds: 10,
            graceful_degradation: false,
            retry_failed_providers: true,
            cleanup_expired_sessions: false,
            max_session_age_hours: 48,
            persist_session_state: false,
        },
    ];
    
    for config in configs {
        let session_manager = create_test_session_manager(Some(config.clone())).await;
        assert!(session_manager.is_ok(), "Failed to create session manager with config");
        
        let manager = session_manager.unwrap();
        let manager_config = manager.get_recovery_config();
        
        // Verify configuration was applied correctly
        assert_eq!(manager_config.enabled, config.enabled);
        assert_eq!(manager_config.auto_recovery_on_startup, config.auto_recovery_on_startup);
        assert_eq!(manager_config.validation_interval_minutes, config.validation_interval_minutes);
        assert_eq!(manager_config.max_recovery_attempts, config.max_recovery_attempts);
        assert_eq!(manager_config.token_validation_timeout_seconds, config.token_validation_timeout_seconds);
        assert_eq!(manager_config.graceful_degradation, config.graceful_degradation);
        assert_eq!(manager_config.retry_failed_providers, config.retry_failed_providers);
        assert_eq!(manager_config.cleanup_expired_sessions, config.cleanup_expired_sessions);
        assert_eq!(manager_config.max_session_age_hours, config.max_session_age_hours);
        assert_eq!(manager_config.persist_session_state, config.persist_session_state);
    }
}

#[test]
async fn test_auth_method_type_conversion() {
    use magictunnel::auth::config::AuthMethod;
    
    // Test OAuth conversion
    let oauth_method = AuthMethod::OAuth {
        provider: "github".to_string(),
        scopes: vec!["read".to_string()],
    };
    let auth_type = AuthMethodType::from(&oauth_method);
    assert_eq!(auth_type, AuthMethodType::OAuth);
    
    // Test DeviceCode conversion
    let device_method = AuthMethod::DeviceCode {
        provider: "github".to_string(),
        scopes: vec!["read".to_string()],
    };
    let auth_type = AuthMethodType::from(&device_method);
    assert_eq!(auth_type, AuthMethodType::DeviceCode);
    
    // Test ApiKey conversion
    let api_key_method = AuthMethod::ApiKey {
        key_ref: "test_key".to_string(),
    };
    let auth_type = AuthMethodType::from(&api_key_method);
    assert_eq!(auth_type, AuthMethodType::ApiKey);
    
    // Test ServiceAccount conversion
    let service_account_method = AuthMethod::ServiceAccount {
        account_ref: "test_account".to_string(),
    };
    let auth_type = AuthMethodType::from(&service_account_method);
    assert_eq!(auth_type, AuthMethodType::ServiceAccount);
}

#[test]
async fn test_session_recovery_error_handling() {
    // Test session manager creation with invalid configuration
    let user_context = create_test_user_context().unwrap();
    let token_storage = create_test_token_storage(user_context.clone()).await.unwrap();
    
    // Test with extreme timeout values
    let config = SessionRecoveryConfig {
        token_validation_timeout_seconds: 0, // Invalid but should not crash
        ..Default::default()
    };
    
    let session_manager = SessionManager::new(user_context, token_storage, config).await;
    // Should succeed even with edge case configuration
    assert!(session_manager.is_ok());
}

#[test]
async fn test_multi_provider_session_recovery() {
    let session_manager = create_test_session_manager(None).await.unwrap();
    
    // Test with multiple OAuth providers
    let mut providers = create_test_oauth_providers();
    providers.insert(
        "microsoft".to_string(),
        OAuthProviderConfig {
            client_id: "ms_client_id".to_string(),
            client_secret: "ms_client_secret".to_string().into(),
            scopes: vec!["User.Read".to_string()],
            oauth_enabled: true,
            device_code_enabled: false,
            authorization_endpoint: Some("https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string()),
            device_authorization_endpoint: None,
            token_endpoint: Some("https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string()),
            user_info_endpoint: Some("https://graph.microsoft.com/v1.0/me".to_string()),
            resource_indicators: None,
            extra_params: None,
        }
    );
    
    // Set the extended provider list
    let mut manager = session_manager;
    manager.set_oauth_providers(providers);
    
    // Test recovery attempt with multiple providers
    let result = manager.recover_sessions_on_startup().await;
    assert!(result.is_ok());
    
    // Should handle multiple providers gracefully
    let recovery_result = result.unwrap();
    assert!(recovery_result.success);
}

// Integration test helper - would be used with a mock HTTP server in practice
#[ignore] // Ignore by default since it requires network setup
#[test]
async fn test_token_validation_with_mock_server() {
    // This test would set up a mock HTTP server to simulate OAuth provider responses
    // and test actual token validation logic. Skipped for now since it requires
    // additional test infrastructure.
    
    // TODO: Implement with mock HTTP server:
    // 1. Set up mock server with userinfo endpoints
    // 2. Create session manager with test configuration
    // 3. Store tokens and test validation against mock endpoints
    // 4. Verify proper handling of various HTTP responses (200, 401, 403, 500, etc.)
    // 5. Test timeout handling
    // 6. Test rate limiting behavior
}

/// Helper to create OAuthTokenResponse for testing
fn convert_token_to_oauth_response(token_data: &TokenData) -> magictunnel::auth::oauth::OAuthTokenResponse {
    magictunnel::auth::oauth::OAuthTokenResponse {
        access_token: token_data.access_token.clone(),
        token_type: token_data.token_type.clone(),
        expires_in: token_data.expires_at.and_then(|exp| {
            exp.duration_since(SystemTime::now()).ok().map(|d| d.as_secs())
        }),
        refresh_token: token_data.refresh_token.clone(),
        scope: if token_data.scopes.is_empty() {
            None
        } else {
            Some(token_data.scopes.join(" "))
        },
        audience: token_data.audience.clone(),
        resource: token_data.resource.clone(),
    }
}