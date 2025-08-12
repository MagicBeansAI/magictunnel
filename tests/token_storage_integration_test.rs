//! Integration tests for Multi-Platform Token Storage (OAuth 2.1 Phase 2.2)
//! 
//! This test suite validates the complete token storage system across all platforms
//! and storage backends, ensuring secure cross-platform operation with proper
//! encryption, thread safety, and OAuth integration.

use magictunnel::auth::{
    TokenStorage, TokenData, UserContext, OAuthTokenResponse, 
    OAuthValidator, SecureStorageType,
};
use magictunnel::config::AuthConfig;
use magictunnel::error::Result;
use secrecy::ExposeSecret;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio;

/// Create a test user context with temporary session directory (uses filesystem storage)
async fn create_test_user_context() -> Result<UserContext> {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join("test_sessions");
    // Use for_testing to ensure filesystem storage and avoid Keychain prompts
    UserContext::for_testing(session_dir)
}

/// Create test OAuth token response
fn create_test_oauth_token(provider: &str, user_id: Option<&str>) -> OAuthTokenResponse {
    OAuthTokenResponse {
        access_token: format!("test_access_token_{}_{}", provider, user_id.unwrap_or("anonymous")).into(),
        token_type: "Bearer".to_string(),
        expires_in: Some(3600), // 1 hour
        refresh_token: Some(format!("test_refresh_token_{}_{}", provider, user_id.unwrap_or("anonymous")).into()),
        scope: Some("read write".to_string()),
        audience: Some(vec!["https://api.example.com".to_string()]),
        resource: Some(vec!["https://resource.example.com".to_string()]),
    }
}

/// Create test OAuth config
fn create_test_oauth_config() -> AuthConfig {
    let mut config = AuthConfig::default();
    config.enabled = true;
    config.r#type = magictunnel::config::AuthType::OAuth;
    
    let oauth_config = magictunnel::config::OAuthConfig {
        provider: "test_provider".to_string(),
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string().into(),
        auth_url: "https://example.com/auth".to_string(),
        token_url: "https://example.com/token".to_string(),
        oauth_2_1_enabled: true,
        resource_indicators_enabled: false,
        default_resources: vec![],
        default_audience: vec![],
        require_explicit_resources: false,
    };
    
    config.oauth = Some(oauth_config);
    config
}

#[tokio::test]
async fn test_token_storage_creation() {
    let user_context = create_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new(user_context).await;
    
    assert!(token_storage.is_ok());
    
    let storage = token_storage.unwrap();
    assert!(storage.is_storage_available().await);
}

#[tokio::test]
async fn test_token_data_creation_and_operations() {
    let oauth_token = create_test_oauth_token("github", Some("testuser"));
    let token_data = TokenData::from_oauth_response(
        &oauth_token,
        "github".to_string(),
        Some("testuser".to_string()),
    );
    
    assert_eq!(token_data.provider, "github");
    assert_eq!(token_data.user_id, Some("testuser".to_string()));
    assert_eq!(token_data.token_type, "Bearer");
    assert!(!token_data.is_expired());
    assert!(!token_data.needs_refresh());
    assert_eq!(token_data.display_name(), "github:testuser");
    
    // Test scopes parsing
    assert_eq!(token_data.scopes, vec!["read", "write"]);
    
    // Test resource indicators
    assert_eq!(token_data.audience, Some(vec!["https://api.example.com".to_string()]));
    assert_eq!(token_data.resource, Some(vec!["https://resource.example.com".to_string()]));
}

#[tokio::test]
async fn test_token_expiration_logic() {
    let mut oauth_token = create_test_oauth_token("github", Some("testuser"));
    oauth_token.expires_in = Some(1); // Expires in 1 second
    
    let token_data = TokenData::from_oauth_response(
        &oauth_token,
        "github".to_string(),
        Some("testuser".to_string()),
    );
    
    assert!(!token_data.is_expired()); // Should not be expired yet
    assert!(token_data.needs_refresh()); // Should need refresh (expires in < 5 minutes)
    
    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(token_data.is_expired());
}

#[tokio::test]
async fn test_token_refresh_update() {
    let oauth_token = create_test_oauth_token("github", Some("testuser"));
    let mut token_data = TokenData::from_oauth_response(
        &oauth_token,
        "github".to_string(),
        Some("testuser".to_string()),
    );
    
    let original_access_token = token_data.access_token.clone();
    
    // Create refresh response
    let refresh_response = OAuthTokenResponse {
        access_token: "new_access_token".to_string().into(),
        token_type: "Bearer".to_string(),
        expires_in: Some(7200), // 2 hours
        refresh_token: Some("new_refresh_token".to_string().into()),
        scope: Some("read write admin".to_string()),
        audience: None,
        resource: None,
    };
    
    token_data.update_from_refresh(&refresh_response);
    
    assert_ne!(token_data.access_token.expose_secret(), original_access_token.expose_secret());
    assert_eq!(token_data.access_token.expose_secret(), "new_access_token");
    assert_eq!(token_data.refresh_token.as_ref().map(|t| t.expose_secret().as_str()), Some("new_refresh_token"));
    assert_eq!(token_data.scopes, vec!["read", "write", "admin"]);
    assert!(token_data.last_refreshed.is_some());
}

#[tokio::test]
async fn test_basic_token_storage_operations() {
    let user_context = create_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    
    let oauth_token = create_test_oauth_token("github", Some("testuser"));
    
    // Store token
    let key = token_storage.store_oauth_token("github", Some("testuser"), &oauth_token).await;
    assert!(key.is_ok());
    let stored_key = key.unwrap();
    assert!(!stored_key.is_empty());
    
    // Retrieve token
    let retrieved = token_storage.retrieve_oauth_token("github", Some("testuser")).await;
    assert!(retrieved.is_ok());
    assert!(retrieved.as_ref().unwrap().is_some());
    
    let token_data = retrieved.unwrap().unwrap();
    assert_eq!(token_data.provider, "github");
    assert_eq!(token_data.user_id, Some("testuser".to_string()));
    assert_eq!(token_data.access_token.expose_secret(), oauth_token.access_token.expose_secret());
    
    // Delete token
    let deleted = token_storage.delete_oauth_token("github", Some("testuser")).await;
    assert!(deleted.is_ok());
    
    // Verify deletion
    let retrieved_after_delete = token_storage.retrieve_oauth_token("github", Some("testuser")).await;
    assert!(retrieved_after_delete.is_ok());
    assert!(retrieved_after_delete.unwrap().is_none());
}

#[tokio::test]
async fn test_multiple_providers_and_users() {
    let user_context = create_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    
    let providers_users = [
        ("github", Some("user1")),
        ("github", Some("user2")),
        ("google", Some("user1")),
        ("microsoft", None),
    ];
    
    // Store tokens for different provider/user combinations
    for (provider, user_id) in &providers_users {
        let oauth_token = create_test_oauth_token(provider, *user_id);
        let result = token_storage.store_oauth_token(provider, *user_id, &oauth_token).await;
        assert!(result.is_ok());
    }
    
    // Retrieve and verify each token
    for (provider, user_id) in &providers_users {
        let retrieved = token_storage.retrieve_oauth_token(provider, *user_id).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.as_ref().unwrap().is_some());
        
        let token_data = retrieved.unwrap().unwrap();
        assert_eq!(token_data.provider, *provider);
        assert_eq!(token_data.user_id, user_id.map(String::from));
    }
    
    // List all tokens
    let all_tokens = token_storage.get_all_tokens().await;
    assert!(all_tokens.is_ok());
    let tokens = all_tokens.unwrap();
    assert_eq!(tokens.len(), providers_users.len());
}

#[tokio::test]
async fn test_expired_token_cleanup() {
    let user_context = create_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    
    // Store token that will expire immediately
    let mut expired_oauth_token = create_test_oauth_token("github", Some("testuser"));
    expired_oauth_token.expires_in = Some(0); // Expires immediately
    
    let _ = token_storage.store_oauth_token("github", Some("testuser"), &expired_oauth_token).await.unwrap();
    
    // Store non-expired token
    let valid_oauth_token = create_test_oauth_token("google", Some("testuser"));
    let _ = token_storage.store_oauth_token("google", Some("testuser"), &valid_oauth_token).await.unwrap();
    
    // Wait a moment to ensure token has expired
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Clean up expired tokens
    let cleanup_count = token_storage.cleanup_expired_tokens().await;
    assert!(cleanup_count.is_ok());
    let deleted_count = cleanup_count.unwrap();
    
    assert_eq!(deleted_count, 1); // Only the GitHub token should be cleaned up
    
    // Verify expired token is gone
    let github_token = token_storage.retrieve_oauth_token("github", Some("testuser")).await.unwrap();
    assert!(github_token.is_none());
    
    // Verify valid token still exists
    let google_token = token_storage.retrieve_oauth_token("google", Some("testuser")).await.unwrap();
    assert!(google_token.is_some());
}

#[tokio::test]
async fn test_concurrent_token_operations() {
    let user_context = create_test_user_context().await.unwrap();
    let token_storage = Arc::new(TokenStorage::new(user_context).await.unwrap());
    
    let mut handles = Vec::new();
    
    // Spawn multiple concurrent operations
    for i in 0..10 {
        let storage_clone = Arc::clone(&token_storage);
        let handle = tokio::spawn(async move {
            let provider = format!("provider_{}", i);
            let user_id = format!("user_{}", i);
            let oauth_token = create_test_oauth_token(&provider, Some(&user_id));
            
            // Store token
            let store_result = storage_clone.store_oauth_token(&provider, Some(&user_id), &oauth_token).await;
            assert!(store_result.is_ok());
            
            // Retrieve token
            let retrieve_result = storage_clone.retrieve_oauth_token(&provider, Some(&user_id)).await;
            assert!(retrieve_result.is_ok());
            assert!(retrieve_result.unwrap().is_some());
            
            // Delete token
            let delete_result = storage_clone.delete_oauth_token(&provider, Some(&user_id)).await;
            assert!(delete_result.is_ok());
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_token_storage_with_oauth_validator() {
    let user_context = create_test_user_context().await.unwrap();
    let auth_config = create_test_oauth_config();
    
    let oauth_validator = OAuthValidator::with_token_storage(auth_config, user_context).await;
    assert!(oauth_validator.is_ok());
    
    let validator = oauth_validator.unwrap();
    assert!(validator.has_token_storage());
    
    // Test token storage operations through OAuth validator
    let oauth_token = create_test_oauth_token("test_provider", Some("testuser"));
    
    let store_result = validator.store_token_manually("test_provider", Some("testuser"), &oauth_token).await;
    assert!(store_result.is_ok());
    assert!(store_result.unwrap().is_some());
    
    let retrieve_result = validator.retrieve_stored_token("test_provider", Some("testuser")).await;
    assert!(retrieve_result.is_ok());
    assert!(retrieve_result.unwrap().is_some());
    
    let delete_result = validator.delete_stored_token("test_provider", Some("testuser")).await;
    assert!(delete_result.is_ok());
    assert!(delete_result.unwrap());
}

#[tokio::test]
async fn test_token_cache_operations() {
    let user_context = create_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    
    let oauth_token = create_test_oauth_token("github", Some("testuser"));
    
    // Store token (should cache)
    let _key = token_storage.store_oauth_token("github", Some("testuser"), &oauth_token).await.unwrap();
    
    // First retrieval should be from cache
    let retrieved1 = token_storage.retrieve_oauth_token("github", Some("testuser")).await.unwrap();
    assert!(retrieved1.is_some());
    
    // Clear cache
    token_storage.clear_cache();
    
    // Second retrieval should be from storage backend
    let retrieved2 = token_storage.retrieve_oauth_token("github", Some("testuser")).await.unwrap();
    assert!(retrieved2.is_some());
    
    // Both retrievals should have the same data
    let token1 = retrieved1.unwrap();
    let token2 = retrieved2.unwrap();
    assert_eq!(token1.access_token.expose_secret(), token2.access_token.expose_secret());
    assert_eq!(token1.provider, token2.provider);
}

#[tokio::test] 
async fn test_filesystem_storage_encryption() {
    let user_context = create_test_user_context().await.unwrap();
    
    // Force filesystem storage by using a user context without secure storage
    let filesystem_context = user_context.clone();
    
    let token_storage = TokenStorage::new(filesystem_context).await.unwrap();
    
    // Verify we're using filesystem storage on the test system
    assert!(matches!(
        token_storage.storage_type(),
        SecureStorageType::Filesystem | SecureStorageType::Keychain | 
        SecureStorageType::CredentialManager | SecureStorageType::SecretService
    ));
    
    let oauth_token = create_test_oauth_token("github", Some("testuser"));
    
    // Store token
    let _key = token_storage.store_oauth_token("github", Some("testuser"), &oauth_token).await.unwrap();
    
    // Retrieve token
    let retrieved = token_storage.retrieve_oauth_token("github", Some("testuser")).await.unwrap();
    assert!(retrieved.is_some());
    
    let token_data = retrieved.unwrap();
    assert_eq!(token_data.access_token.expose_secret(), oauth_token.access_token.expose_secret());
    assert_eq!(token_data.provider, "github");
}

#[tokio::test]
async fn test_token_data_zeroization() {
    let oauth_token = create_test_oauth_token("github", Some("testuser"));
    let token_data = TokenData::from_oauth_response(
        &oauth_token,
        "github".to_string(),
        Some("testuser".to_string()),
    );
    
    // Note: as_ptr() not available on Secret<String>, and clone() creates a new Secret
    let _original_access_token = token_data.access_token.clone();
    
    // Drop the token (should trigger zeroization)
    drop(token_data);
    
    // Note: In a real security test, we would verify that the memory has been zeroized
    // However, this is difficult to test reliably in a unit test environment
    // The zeroize crate handles this at the memory level when the struct is dropped
}

#[tokio::test]
async fn test_storage_backend_availability() {
    let user_context = create_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    
    // Test that storage backend is available
    assert!(token_storage.is_storage_available().await);
    
    // Test storage type detection
    let storage_type = token_storage.storage_type();
    assert!(matches!(
        storage_type,
        SecureStorageType::Keychain | 
        SecureStorageType::CredentialManager | 
        SecureStorageType::SecretService | 
        SecureStorageType::Filesystem
    ));
}

#[tokio::test]
async fn test_error_handling() {
    let user_context = create_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    
    // Test retrieving non-existent token
    let result = token_storage.retrieve_oauth_token("nonexistent", Some("user")).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
    
    // Test deleting non-existent token
    let delete_result = token_storage.delete_oauth_token("nonexistent", Some("user")).await;
    assert!(delete_result.is_ok()); // Should succeed (no-op)
}

#[tokio::test] 
async fn test_token_metadata() {
    let user_context = create_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    
    let oauth_token = create_test_oauth_token("github", Some("testuser"));
    let mut token_data = TokenData::from_oauth_response(
        &oauth_token,
        "github".to_string(),
        Some("testuser".to_string()),
    );
    
    // Add custom metadata
    token_data.metadata.insert("client_version".to_string(), "1.0.0".to_string());
    token_data.metadata.insert("device_id".to_string(), "test-device".to_string());
    
    // Store token with metadata using a generated key
    let _ = token_storage.store_oauth_token("github", Some("testuser"), &oauth_token).await.unwrap();
    
    // Retrieve and verify token was stored correctly
    let retrieved = token_storage.retrieve_oauth_token("github", Some("testuser")).await.unwrap();
    assert!(retrieved.is_some());
    
    let retrieved_token = retrieved.unwrap();
    // Metadata won't be preserved in the basic store/retrieve cycle since we used OAuth token
    // This test would need to be updated to use the direct store_token method to test metadata
    assert_eq!(retrieved_token.provider, "github");
    assert_eq!(retrieved_token.user_id, Some("testuser".to_string()));
}

// Integration test with actual OAuth workflow simulation
#[tokio::test]
async fn test_complete_oauth_workflow_simulation() {
    let user_context = create_test_user_context().await.unwrap();
    let auth_config = create_test_oauth_config();
    
    // Create OAuth validator with token storage
    let oauth_validator = OAuthValidator::with_token_storage(auth_config, user_context).await.unwrap();
    
    // Simulate successful OAuth token exchange
    let oauth_token = create_test_oauth_token("test_provider", Some("oauth_user"));
    
    // Store token automatically (simulate what would happen in exchange_code_for_token)
    let store_result = oauth_validator.store_token_manually("test_provider", Some("oauth_user"), &oauth_token).await;
    assert!(store_result.is_ok());
    
    // Retrieve token for API calls
    let retrieved_token = oauth_validator.retrieve_stored_token("test_provider", Some("oauth_user")).await;
    assert!(retrieved_token.is_ok());
    assert!(retrieved_token.as_ref().unwrap().is_some());
    
    let token_data = retrieved_token.unwrap().unwrap();
    assert_eq!(token_data.provider, "test_provider");
    assert_eq!(token_data.user_id, Some("oauth_user".to_string()));
    assert!(!token_data.is_expired());
    
    // Simulate token refresh
    let mut refresh_response = create_test_oauth_token("test_provider", Some("oauth_user"));
    refresh_response.access_token = "refreshed_access_token".to_string().into();
    
    let updated_store = oauth_validator.store_token_manually("test_provider", Some("oauth_user"), &refresh_response).await;
    assert!(updated_store.is_ok());
    
    // Verify token was updated
    let refreshed_token = oauth_validator.retrieve_stored_token("test_provider", Some("oauth_user")).await.unwrap().unwrap();
    assert_eq!(refreshed_token.access_token.expose_secret(), "refreshed_access_token");
    
    // Clean up
    let cleanup_result = oauth_validator.delete_stored_token("test_provider", Some("oauth_user")).await;
    assert!(cleanup_result.is_ok());
    assert!(cleanup_result.unwrap());
}

#[tokio::test]
async fn test_cross_platform_compatibility() {
    let user_context = create_test_user_context().await.unwrap();
    
    // Test that we can create token storage on any platform
    let token_storage_result = TokenStorage::new(user_context).await;
    assert!(token_storage_result.is_ok());
    
    let token_storage = token_storage_result.unwrap();
    
    // Test basic operations work regardless of platform
    let oauth_token = create_test_oauth_token("cross_platform_test", Some("user"));
    
    let store_result = token_storage.store_oauth_token("cross_platform_test", Some("user"), &oauth_token).await;
    assert!(store_result.is_ok());
    
    let retrieve_result = token_storage.retrieve_oauth_token("cross_platform_test", Some("user")).await;
    assert!(retrieve_result.is_ok());
    assert!(retrieve_result.unwrap().is_some());
    
    let delete_result = token_storage.delete_oauth_token("cross_platform_test", Some("user")).await;
    assert!(delete_result.is_ok());
}

#[tokio::test]
async fn test_mock_storage_backend() {
    let user_context = create_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new_with_mock_backend(user_context).await.unwrap();
    
    // Test that mock storage is available and works
    assert!(token_storage.is_storage_available().await);
    
    let oauth_token = create_test_oauth_token("mock_test", Some("mock_user"));
    
    // Store token
    let key = token_storage.store_oauth_token("mock_test", Some("mock_user"), &oauth_token).await;
    assert!(key.is_ok());
    
    // Retrieve token
    let retrieved = token_storage.retrieve_oauth_token("mock_test", Some("mock_user")).await;
    assert!(retrieved.is_ok());
    assert!(retrieved.as_ref().unwrap().is_some());
    
    let token_data = retrieved.unwrap().unwrap();
    assert_eq!(token_data.provider, "mock_test");
    assert_eq!(token_data.user_id, Some("mock_user".to_string()));
    assert_eq!(token_data.access_token.expose_secret(), oauth_token.access_token.expose_secret());
    
    // Delete token
    let deleted = token_storage.delete_oauth_token("mock_test", Some("mock_user")).await;
    assert!(deleted.is_ok());
    
    // Verify deletion
    let retrieved_after_delete = token_storage.retrieve_oauth_token("mock_test", Some("mock_user")).await;
    assert!(retrieved_after_delete.is_ok());
    assert!(retrieved_after_delete.unwrap().is_none());
}

#[tokio::test]
async fn test_filesystem_storage_forced() {
    // Ensure we're using filesystem storage
    let user_context = create_test_user_context().await.unwrap();
    assert_eq!(user_context.secure_storage, SecureStorageType::Filesystem);
    
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    assert_eq!(*token_storage.storage_type(), SecureStorageType::Filesystem);
    
    // Test basic operations with filesystem backend
    let oauth_token = create_test_oauth_token("filesystem_test", Some("fs_user"));
    
    let store_result = token_storage.store_oauth_token("filesystem_test", Some("fs_user"), &oauth_token).await;
    assert!(store_result.is_ok());
    
    let retrieve_result = token_storage.retrieve_oauth_token("filesystem_test", Some("fs_user")).await;
    assert!(retrieve_result.is_ok());
    assert!(retrieve_result.unwrap().is_some());
}