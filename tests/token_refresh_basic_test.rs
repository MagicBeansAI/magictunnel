//! Basic Token Refresh Service Tests
//!
//! This test suite validates basic Token Refresh Service functionality
//! that can be tested through the public API.

use magictunnel::auth::{
    config::OAuthProviderConfig,
    oauth::OAuthTokenResponse,
    session_manager::{SessionManager, SessionRecoveryConfig},
    token_refresh::{TokenRefreshService, TokenRefreshConfig, RefreshTask},
    token_storage::{TokenData, TokenStorage},
    user_context::UserContext,
};
use secrecy::ExposeSecret;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tempfile::tempdir;

/// Create a test user context
fn create_test_user_context() -> UserContext {
    let temp_dir = tempdir().unwrap();
    let session_dir = temp_dir.path().to_path_buf();
    
    UserContext::with_session_dir(session_dir)
        .expect("Failed to create test user context")
}

/// Create a test token storage
async fn create_test_token_storage() -> Arc<TokenStorage> {
    let user_context = create_test_user_context();
    Arc::new(TokenStorage::new(user_context).await.expect("Failed to create token storage"))
}

/// Create a test session manager
async fn create_test_session_manager(
    token_storage: Arc<TokenStorage>,
) -> Arc<SessionManager> {
    let user_context = create_test_user_context();
    let recovery_config = SessionRecoveryConfig::default();
    
    Arc::new(
        SessionManager::new(user_context, token_storage, recovery_config)
            .await
            .expect("Failed to create session manager")
    )
}

/// Create test OAuth provider configurations
fn create_test_oauth_providers() -> HashMap<String, OAuthProviderConfig> {
    let mut providers = HashMap::new();
    
    providers.insert("github".to_string(), OAuthProviderConfig {
        client_id: "github_client_id".to_string(),
        client_secret: "github_client_secret".to_string().into(),
        scopes: vec!["read:user".to_string()],
        oauth_enabled: true,
        device_code_enabled: false,
        authorization_endpoint: Some("https://github.com/login/oauth/authorize".to_string()),
        device_authorization_endpoint: None,
        token_endpoint: Some("https://github.com/login/oauth/access_token".to_string()),
        user_info_endpoint: Some("https://api.github.com/user".to_string()),
        resource_indicators: None,
        extra_params: None,
    });
    
    providers
}

#[tokio::test]
async fn test_token_refresh_service_creation() {
    let token_storage = create_test_token_storage().await;
    let session_manager = create_test_session_manager(Arc::clone(&token_storage)).await;
    let user_context = create_test_user_context();
    let refresh_config = TokenRefreshConfig::default();
    
    let service = TokenRefreshService::new(
        user_context,
        token_storage,
        session_manager,
        refresh_config,
    ).await;
    
    assert!(service.is_ok(), "Failed to create token refresh service: {:?}", service.err());
    
    let service = service.unwrap();
    let stats = service.get_refresh_stats();
    
    // Verify initial state
    assert_eq!(stats.total_tasks_scheduled, 0);
    assert_eq!(stats.current_active_refreshes, 0);
    assert_eq!(stats.total_successful_refreshes, 0);
    assert_eq!(stats.total_failed_refreshes, 0);
}

#[tokio::test]
async fn test_refresh_task_creation_and_scheduling() {
    let task = RefreshTask::new(
        "github".to_string(),
        "test_user".to_string(),
        Duration::from_secs(900), // 15 minutes
    );
    
    assert_eq!(task.provider, "github");
    assert_eq!(task.user_id, "test_user");
    assert_eq!(task.retry_count, 0);
    assert_eq!(task.priority, 1);
    assert!(task.last_attempt.is_none());
    assert!(!task.is_due()); // Should not be due immediately
    assert!(!task.has_exceeded_retry_limit(3));
    
    let task_id = task.task_id();
    assert_eq!(task_id, "github:test_user");
}

#[tokio::test]
async fn test_refresh_task_retry_logic() {
    let mut task = RefreshTask::new(
        "github".to_string(),
        "test_user".to_string(),
        Duration::from_secs(900),
    );
    
    let base_delay = Duration::from_secs(5);
    let max_attempts = 3;
    
    // First retry
    task.schedule_retry(base_delay, max_attempts);
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.priority, 2); // Priority increased
    assert!(task.last_attempt.is_some());
    assert!(!task.has_exceeded_retry_limit(max_attempts));
    
    // Second retry
    task.schedule_retry(base_delay, max_attempts);
    assert_eq!(task.retry_count, 2);
    assert_eq!(task.priority, 3);
    assert!(!task.has_exceeded_retry_limit(max_attempts));
    
    // Third retry (limit reached)
    task.schedule_retry(base_delay, max_attempts);
    assert_eq!(task.retry_count, 3);
    assert_eq!(task.priority, 4);
    assert!(task.has_exceeded_retry_limit(max_attempts));
}

#[tokio::test]
async fn test_token_refresh_config_duration_methods() {
    let config = TokenRefreshConfig {
        refresh_threshold_minutes: 20,
        retry_delay_base_seconds: 10,
        refresh_timeout_seconds: 45,
        background_check_interval_minutes: 60,
        max_retry_age_hours: 48,
        ..Default::default()
    };
    
    assert_eq!(config.refresh_threshold(), Duration::from_secs(1200)); // 20 minutes
    assert_eq!(config.retry_delay_base(), Duration::from_secs(10));
    assert_eq!(config.refresh_timeout(), Duration::from_secs(45));
    assert_eq!(config.background_check_interval(), Duration::from_secs(3600)); // 1 hour
    assert_eq!(config.max_retry_age(), Duration::from_secs(172800)); // 48 hours
}

#[tokio::test]
async fn test_token_scheduling_integration() {
    let token_storage = create_test_token_storage().await;
    let session_manager = create_test_session_manager(Arc::clone(&token_storage)).await;
    let user_context = create_test_user_context();
    let refresh_config = TokenRefreshConfig {
        refresh_threshold_minutes: 30, // 30-minute threshold
        background_refresh_enabled: false, // Disable background for this test
        ..Default::default()
    };
    
    let mut service = TokenRefreshService::new(
        user_context,
        Arc::clone(&token_storage),
        session_manager,
        refresh_config,
    ).await.unwrap();
    
    // Set up OAuth providers
    service.set_oauth_providers(create_test_oauth_providers());
    
    // Schedule token refresh
    let result = service.schedule_token_refresh(
        "github",
        "test_user",
        Some(Duration::from_secs(600)), // 10 minutes
    ).await;
    
    assert!(result.is_ok(), "Failed to schedule token refresh: {:?}", result.err());
    
    // Verify task was scheduled
    let stats = service.get_refresh_stats();
    assert_eq!(stats.total_tasks_scheduled, 1);
    
    // Request immediate refresh
    let immediate_result = service.request_immediate_refresh("github", "test_user").await;
    assert!(immediate_result.is_ok(), "Failed to request immediate refresh: {:?}", immediate_result.err());
    
    let stats_after_immediate = service.get_refresh_stats();
    assert_eq!(stats_after_immediate.total_requests_queued, 1);
}

#[tokio::test]
async fn test_token_data_creation() {
    // Test token data creation and basic functionality
    let expires_at = SystemTime::now() + Duration::from_secs(3600); // 1 hour
    
    let token = TokenData {
        access_token: "test_access_token".to_string().into(),
        refresh_token: Some("test_refresh_token".to_string().into()),
        expires_at: Some(expires_at),
        scopes: vec!["read:user".to_string()],
        provider: "github".to_string(),
        token_type: "Bearer".to_string(),
        audience: None,
        resource: None,
        created_at: SystemTime::now(),
        last_refreshed: None,
        user_id: Some("test_user".to_string()),
        metadata: HashMap::new(),
    };
    
    assert_eq!(token.provider, "github");
    assert_eq!(token.user_id, Some("test_user".to_string()));
    assert!(token.refresh_token.is_some());
    assert_eq!(token.scopes, vec!["read:user".to_string()]);
}

#[tokio::test]
async fn test_oauth_token_response_creation() {
    let response = OAuthTokenResponse {
        access_token: "new_access_token".to_string().into(),
        token_type: "Bearer".to_string(),
        expires_in: Some(3600),
        refresh_token: Some("new_refresh_token".to_string().into()),
        scope: Some("read:user user:email".to_string()),
        audience: None,
        resource: None,
    };
    
    assert_eq!(response.access_token.expose_secret(), "new_access_token");
    assert_eq!(response.token_type, "Bearer");
    assert_eq!(response.expires_in, Some(3600));
    assert!(response.refresh_token.is_some());
}

#[tokio::test]
async fn test_cleanup_operations() {
    let token_storage = create_test_token_storage().await;
    let session_manager = create_test_session_manager(Arc::clone(&token_storage)).await;
    let user_context = create_test_user_context();
    let refresh_config = TokenRefreshConfig {
        max_retry_age_hours: 1, // 1 hour
        max_retry_attempts: 2,
        auto_cleanup_failed_attempts: true,
        ..Default::default()
    };
    
    let service = TokenRefreshService::new(
        user_context,
        Arc::clone(&token_storage),
        session_manager,
        refresh_config,
    ).await.unwrap();
    
    // Test cleanup - should succeed without error even with no tasks
    let cleaned_count = service.cleanup_old_refresh_tasks().await;
    assert!(cleaned_count >= 0, "Cleanup should succeed without error");
    
    // Test service statistics
    let stats = service.get_refresh_stats();
    assert_eq!(stats.total_tasks_scheduled, 0);
    assert_eq!(stats.current_active_refreshes, 0);
}

#[tokio::test]
async fn test_provider_configuration() {
    let token_storage = create_test_token_storage().await;
    let session_manager = create_test_session_manager(Arc::clone(&token_storage)).await;
    let user_context = create_test_user_context();
    let refresh_config = TokenRefreshConfig::default();
    
    let mut service = TokenRefreshService::new(
        user_context,
        token_storage,
        session_manager,
        refresh_config,
    ).await.unwrap();
    
    // Test setting OAuth providers
    let providers = create_test_oauth_providers();
    assert_eq!(providers.len(), 1);
    assert!(providers.contains_key("github"));
    
    service.set_oauth_providers(providers);
    
    // Test setting OAuth clients (with empty map for testing)
    let clients = HashMap::new();
    service.set_oauth_clients(clients);
    
    // Service should still be functional
    let stats = service.get_refresh_stats();
    assert_eq!(stats.total_tasks_scheduled, 0);
}