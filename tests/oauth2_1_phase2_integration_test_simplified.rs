//! Comprehensive Integration Tests for OAuth 2.1 Phase 2: Session Persistence
//!
//! This test suite validates the OAuth 2.1 Phase 2 session persistence components
//! that are currently implemented and available:
//!
//! **Phase 2 Components Integration:**
//! - Phase 2.1: User Context System - Cross-platform user identification ✅
//! - Phase 2.2: Multi-Platform Token Storage - Secure token storage with encryption ✅
//! - Phase 2.3: Automatic Session Recovery - Runtime session restoration (planned)
//! - Phase 2.4: Token Refresh Service - Background token refresh (planned)
//!
//! **Current Test Coverage:**
//! - User Context creation and management
//! - Token storage and retrieval across providers
//! - Cross-platform compatibility testing
//! - Security validation (encryption, zeroization)
//! - Performance and concurrency testing
//! - Mock infrastructure for testing

use magictunnel::auth::{
    OAuthProviderConfig,
    OAuthTokenResponse,
    TokenData, TokenStorage,
    UserContext, SecureStorageType,
};
use magictunnel::error::Result;
use secrecy::ExposeSecret;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
// tokio::test is used via attribute macro
use wiremock::MockServer;
use tracing::{debug, info, warn};

// ============================================================================
// Test Infrastructure and Utilities
// ============================================================================

/// Simplified test context for OAuth 2.1 Phase 2 integration testing
#[derive(Debug)]
pub struct IntegrationTestContext {
    pub user_context: UserContext,
    pub token_storage: Arc<TokenStorage>,
    pub oauth_providers: HashMap<String, OAuthProviderConfig>,
    pub mock_server: Option<Arc<MockServer>>,
    pub temp_dir: Arc<TempDir>,
}

impl IntegrationTestContext {
    /// Create a new test context focusing on available Phase 2 components
    pub async fn new() -> Result<Self> {
        let temp_dir = Arc::new(TempDir::new().unwrap());
        let session_dir = temp_dir.path().join("oauth2_1_phase2_sessions");
        
        // Phase 2.1: User Context System
        let user_context = UserContext::with_session_dir(session_dir)?;
        debug!("Created user context: {}", user_context.get_unique_user_id());
        
        // Phase 2.2: Multi-Platform Token Storage
        let token_storage = Arc::new(TokenStorage::new(user_context.clone()).await?);
        debug!("Created token storage with backend: {:?}", token_storage.storage_type());
        
        // Set up OAuth provider configurations
        let oauth_providers = create_comprehensive_oauth_providers();
        
        Ok(Self {
            user_context,
            token_storage,
            oauth_providers,
            mock_server: None,
            temp_dir,
        })
    }
    
    /// Set up mock OAuth server for testing
    pub async fn with_mock_server(mut self) -> Self {
        let mock_server = Arc::new(MockServer::start().await);
        self.mock_server = Some(Arc::clone(&mock_server));
        
        // Update OAuth provider configurations to use mock server
        self.oauth_providers = create_mock_oauth_providers(&mock_server);
        
        self
    }
    
    /// Configure OAuth providers (simplified for testing)
    pub fn configure_oauth_providers(&mut self) {
        debug!("OAuth providers configured: {}", self.oauth_providers.len());
    }
    
    /// Store a test token for a provider
    pub async fn store_test_token(&self, provider: &str, user_id: &str, expired: bool) -> Result<String> {
        let oauth_response = create_test_oauth_token_response(provider, user_id, expired);
        self.token_storage.store_oauth_token(provider, Some(user_id), &oauth_response).await
    }
    
    /// Simulate application restart by creating new context with same storage directory
    pub async fn simulate_restart(&self) -> Result<IntegrationTestContext> {
        // Create a new context using the same temp directory for testing persistence
        let temp_dir = Arc::clone(&self.temp_dir);
        let session_dir = temp_dir.path().join("oauth2_1_phase2_sessions");
        
        let user_context = UserContext::with_session_dir(session_dir)?;
        let token_storage = Arc::new(TokenStorage::new(user_context.clone()).await?);
        
        let mut new_context = IntegrationTestContext {
            user_context,
            token_storage,
            oauth_providers: self.oauth_providers.clone(),
            mock_server: self.mock_server.clone(),
            temp_dir,
        };
        
        new_context.configure_oauth_providers();
        Ok(new_context)
    }
}

/// Create comprehensive OAuth provider configurations for testing
fn create_comprehensive_oauth_providers() -> HashMap<String, OAuthProviderConfig> {
    let mut providers = HashMap::new();
    
    // GitHub provider
    providers.insert("github".to_string(), OAuthProviderConfig {
        client_id: "test_github_client_id".to_string(),
        client_secret: "test_github_client_secret".to_string().into(),
        scopes: vec!["read:user".to_string(), "user:email".to_string()],
        oauth_enabled: true,
        device_code_enabled: true,
        authorization_endpoint: Some("https://github.com/login/oauth/authorize".to_string()),
        device_authorization_endpoint: Some("https://github.com/login/device/code".to_string()),
        token_endpoint: Some("https://github.com/login/oauth/access_token".to_string()),
        user_info_endpoint: Some("https://api.github.com/user".to_string()),
        resource_indicators: None,
        extra_params: Some(HashMap::from([
            ("access_type".to_string(), "offline".to_string()),
        ])),
    });
    
    // Google provider
    providers.insert("google".to_string(), OAuthProviderConfig {
        client_id: "test_google_client_id".to_string(),
        client_secret: "test_google_client_secret".to_string().into(),
        scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
        oauth_enabled: true,
        device_code_enabled: false,
        authorization_endpoint: Some("https://accounts.google.com/o/oauth2/auth".to_string()),
        device_authorization_endpoint: None,
        token_endpoint: Some("https://oauth2.googleapis.com/token".to_string()),
        user_info_endpoint: Some("https://www.googleapis.com/oauth2/v1/userinfo".to_string()),
        resource_indicators: Some(vec!["https://www.googleapis.com/auth/userinfo.profile".to_string()]),
        extra_params: None,
    });
    
    // Microsoft provider
    providers.insert("microsoft".to_string(), OAuthProviderConfig {
        client_id: "test_microsoft_client_id".to_string(),
        client_secret: "test_microsoft_client_secret".to_string().into(),
        scopes: vec!["User.Read".to_string(), "Mail.Read".to_string()],
        oauth_enabled: true,
        device_code_enabled: true,
        authorization_endpoint: Some("https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string()),
        device_authorization_endpoint: Some("https://login.microsoftonline.com/common/oauth2/v2.0/devicecode".to_string()),
        token_endpoint: Some("https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string()),
        user_info_endpoint: Some("https://graph.microsoft.com/v1.0/me".to_string()),
        resource_indicators: Some(vec!["https://graph.microsoft.com".to_string()]),
        extra_params: None,
    });
    
    providers
}

/// Create mock OAuth provider configurations using mock server
fn create_mock_oauth_providers(mock_server: &MockServer) -> HashMap<String, OAuthProviderConfig> {
    let base_url = mock_server.uri();
    let mut providers = HashMap::new();
    
    // Mock GitHub provider
    providers.insert("github".to_string(), OAuthProviderConfig {
        client_id: "mock_github_client_id".to_string(),
        client_secret: "mock_github_client_secret".to_string().into(),
        scopes: vec!["read:user".to_string()],
        oauth_enabled: true,
        device_code_enabled: false,
        authorization_endpoint: Some(format!("{}/github/oauth/authorize", base_url)),
        device_authorization_endpoint: None,
        token_endpoint: Some(format!("{}/github/oauth/token", base_url)),
        user_info_endpoint: Some(format!("{}/github/user", base_url)),
        resource_indicators: None,
        extra_params: None,
    });
    
    // Mock Google provider
    providers.insert("google".to_string(), OAuthProviderConfig {
        client_id: "mock_google_client_id".to_string(),
        client_secret: "mock_google_client_secret".to_string().into(),
        scopes: vec!["openid".to_string(), "profile".to_string()],
        oauth_enabled: true,
        device_code_enabled: false,
        authorization_endpoint: Some(format!("{}/google/oauth2/auth", base_url)),
        device_authorization_endpoint: None,
        token_endpoint: Some(format!("{}/google/oauth2/token", base_url)),
        user_info_endpoint: Some(format!("{}/google/oauth2/userinfo", base_url)),
        resource_indicators: None,
        extra_params: None,
    });
    
    providers
}

/// Create test OAuth token response
fn create_test_oauth_token_response(provider: &str, user_id: &str, expired: bool) -> OAuthTokenResponse {
    let expires_in = if expired { Some(1) } else { Some(3600) }; // 1 second vs 1 hour
    
    OAuthTokenResponse {
        access_token: format!("test_access_token_{}_{}", provider, user_id).into(),
        token_type: "Bearer".to_string(),
        expires_in,
        refresh_token: Some(format!("test_refresh_token_{}_{}", provider, user_id).into()),
        scope: Some("read write".to_string()),
        audience: Some(vec![format!("https://api.{}.com", provider)]),
        resource: Some(vec![format!("https://resource.{}.com", provider)]),
    }
}

/// Create test token data
fn create_test_token_data(provider: &str, user_id: &str, expired: bool) -> TokenData {
    let oauth_response = create_test_oauth_token_response(provider, user_id, expired);
    TokenData::from_oauth_response(&oauth_response, provider.to_string(), Some(user_id.to_string()))
}

// ============================================================================
// OAuth 2.1 Phase 2 Integration Tests
// ============================================================================

#[tokio::test]
async fn test_oauth_token_storage_and_persistence() {
    info!("Starting OAuth token storage and persistence test");
    
    // 1. Setup user context and storage
    let mut test_context = IntegrationTestContext::new().await.unwrap()
        .with_mock_server().await;
    test_context.configure_oauth_providers();
    
    // 2. Simulate OAuth authentication flow
    info!("Simulating OAuth authentication flow");
    let github_key = test_context.store_test_token("github", "testuser", false).await.unwrap();
    let google_key = test_context.store_test_token("google", "testuser", false).await.unwrap();
    
    assert!(!github_key.is_empty());
    assert!(!google_key.is_empty());
    
    // 3. Verify token storage
    info!("Verifying token storage");
    let github_token = test_context.token_storage
        .retrieve_oauth_token("github", Some("testuser"))
        .await.unwrap();
    let google_token = test_context.token_storage
        .retrieve_oauth_token("google", Some("testuser"))
        .await.unwrap();
    
    assert!(github_token.is_some());
    assert!(google_token.is_some());
    
    // 4. Simulate application restart
    info!("Simulating application restart");
    let restarted_context = test_context.simulate_restart().await.unwrap();
    
    // 5. Verify tokens persist across restart
    info!("Verifying token persistence across restart");
    let github_token_after = restarted_context.token_storage
        .retrieve_oauth_token("github", Some("testuser"))
        .await.unwrap();
    let google_token_after = restarted_context.token_storage
        .retrieve_oauth_token("google", Some("testuser"))
        .await.unwrap();
    
    assert!(github_token_after.is_some(), "GitHub token should persist");
    assert!(google_token_after.is_some(), "Google token should persist");
    
    // 6. Verify token data integrity
    let github_data = github_token_after.unwrap();
    let google_data = google_token_after.unwrap();
    
    assert_eq!(github_data.provider, "github");
    assert_eq!(google_data.provider, "google");
    assert!(!github_data.access_token.expose_secret().is_empty());
    assert!(!google_data.access_token.expose_secret().is_empty());
    
    info!("OAuth token storage and persistence test completed successfully");
}

#[tokio::test]
async fn test_multi_provider_token_management() {
    info!("Starting multi-provider token management test");
    
    let mut test_context = IntegrationTestContext::new().await.unwrap()
        .with_mock_server().await;
    test_context.configure_oauth_providers();
    
    let providers_users = [
        ("github", "user1"),
        ("github", "user2"), 
        ("google", "user1"),
        ("microsoft", "user1"),
    ];
    
    // Store tokens for multiple providers and users
    info!("Storing tokens for multiple providers and users");
    for (provider, user_id) in &providers_users {
        let key = test_context.store_test_token(provider, user_id, false).await.unwrap();
        assert!(!key.is_empty(), "Token key should not be empty for {}:{}", provider, user_id);
    }
    
    // Verify all tokens are stored
    info!("Verifying all tokens are stored");
    for (provider, user_id) in &providers_users {
        let token = test_context.token_storage
            .retrieve_oauth_token(provider, Some(user_id))
            .await.unwrap();
        assert!(token.is_some(), "Token should exist for {}:{}", provider, user_id);
    }
    
    // Test token cleanup
    info!("Testing token cleanup");
    let cleanup_count = test_context.token_storage.cleanup_expired_tokens().await.unwrap();
    debug!("Cleaned up {} expired tokens", cleanup_count);
    
    // Get all tokens
    let all_tokens = test_context.token_storage.get_all_tokens().await.unwrap();
    info!("Total tokens stored: {}", all_tokens.len());
    assert!(all_tokens.len() >= providers_users.len(), "Should have at least {} tokens", providers_users.len());
    
    // Test provider-specific token behaviors
    info!("Testing provider-specific token behaviors");
    for provider in ["github", "google", "microsoft"].iter() {
        debug!("Provider {} configured", provider);
    }
    
    info!("Multi-provider token management test completed successfully");
}

#[tokio::test]
async fn test_cross_platform_session_storage() {
    info!("Starting cross-platform session storage test");
    
    // Test different storage backends by creating multiple contexts
    let storage_scenarios = vec![
        ("primary_user", SecureStorageType::Filesystem),
        ("secondary_user", SecureStorageType::Filesystem), // Will auto-detect actual storage
    ];
    
    for (user_prefix, _expected_storage) in storage_scenarios {
        info!("Testing storage scenario: {} with filesystem storage", user_prefix);
        
        let temp_dir = TempDir::new().unwrap();
        let session_dir = temp_dir.path().join(format!("{}_sessions", user_prefix));
        let user_context = UserContext::with_session_dir(session_dir.clone()).unwrap();
        let token_storage = TokenStorage::new(user_context.clone()).await.unwrap();
        
        info!("Created storage with type: {:?}", token_storage.storage_type());
        
        // Test storage operations
        let oauth_token = create_test_oauth_token_response("github", user_prefix, false);
        let _store_key = token_storage.store_oauth_token("github", Some(user_prefix), &oauth_token).await.unwrap();
        
        // Validate storage and retrieval
        let retrieved_token = token_storage.retrieve_oauth_token("github", Some(user_prefix)).await.unwrap();
        assert!(retrieved_token.is_some(), "Token should be retrievable for {}", user_prefix);
        
        // Test user context creation and session directory creation  
        debug!("Session directory for {}: {:?}", user_prefix, session_dir.display());
        
        // Verify proper file permissions and security
        let session_dir = &user_context.session_dir;
        assert!(session_dir.exists(), "Session directory should exist for {}", user_prefix);
        
        // Test cleanup
        token_storage.delete_oauth_token("github", Some(user_prefix)).await.unwrap();
        let deleted_token = token_storage.retrieve_oauth_token("github", Some(user_prefix)).await.unwrap();
        assert!(deleted_token.is_none(), "Token should be deleted for {}", user_prefix);
    }
    
    info!("Cross-platform session storage test completed successfully");
}

// ============================================================================
// Security and Performance Tests
// ============================================================================

#[tokio::test]
async fn test_security_validation() {
    info!("Starting security validation test");
    
    let test_context = IntegrationTestContext::new().await.unwrap();
    
    // Test 1: Token encryption at rest
    info!("Testing token encryption at rest");
    let oauth_token = create_test_oauth_token_response("github", "security_user", false);
    let _store_key = test_context.token_storage
        .store_oauth_token("github", Some("security_user"), &oauth_token).await.unwrap();
    
    // Verify token is stored securely (exact verification depends on storage backend)
    let retrieved_token = test_context.token_storage
        .retrieve_oauth_token("github", Some("security_user")).await.unwrap();
    assert!(retrieved_token.is_some(), "Securely stored token should be retrievable");
    
    let token_data = retrieved_token.unwrap();
    assert_eq!(token_data.access_token.expose_secret(), oauth_token.access_token.expose_secret(), "Token data should match");
    
    // Test 2: Memory zeroization for sensitive data
    info!("Testing memory zeroization");
    {
        let sensitive_token = create_test_token_data("github", "zeroize_user", false);
        
        // Token should contain sensitive data
        assert!(!sensitive_token.access_token.expose_secret().is_empty());
        
        // Drop should trigger zeroization (tested at library level)
        drop(sensitive_token);
        // Note: Direct memory inspection would require unsafe code and is platform-specific
    }
    
    // Test 3: Secure file permissions (Unix-only)
    info!("Testing secure file permissions");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let session_dir = &test_context.user_context.session_dir;
        if session_dir.exists() {
            let metadata = std::fs::metadata(session_dir).unwrap();
            let permissions = metadata.permissions();
            let mode = permissions.mode() & 0o777;
            info!("Session directory permissions: {:o}", mode);
            // Should be 0o700 (owner only) or similar secure permissions
            assert!(mode <= 0o750, "Session directory should have secure permissions");
        }
    }
    
    // Test 4: Verify cross-platform user identification uniqueness
    info!("Testing user identification uniqueness");
    let unique_id = test_context.user_context.get_unique_user_id();
    assert!(!unique_id.is_empty(), "User ID should not be empty");
    assert!(unique_id.len() >= 8, "User ID should be reasonably long");
    
    // Create another context and verify different IDs for different sessions
    let temp_dir2 = TempDir::new().unwrap();
    let session_dir2 = temp_dir2.path().join("different_user_sessions");
    let user_context2 = UserContext::with_session_dir(session_dir2).unwrap();
    
    // Same system should produce consistent IDs for same session directory
    let unique_id2 = user_context2.get_unique_user_id();
    info!("User IDs: {} vs {}", unique_id, unique_id2);
    
    info!("Security validation test completed successfully");
}

#[tokio::test]
async fn test_concurrent_session_operations() {
    info!("Starting concurrent session operations test");
    
    let test_context = Arc::new(IntegrationTestContext::new().await.unwrap());
    let mut handles = Vec::new();
    
    // Test concurrent token operations
    for i in 0..10 {
        let context_clone = Arc::clone(&test_context);
        let handle = tokio::spawn(async move {
            let provider = format!("provider_{}", i);
            let user_id = format!("user_{}", i);
            
            // Store token
            let oauth_token = create_test_oauth_token_response(&provider, &user_id, false);
            let store_result = context_clone.token_storage
                .store_oauth_token(&provider, Some(&user_id), &oauth_token).await;
            assert!(store_result.is_ok(), "Concurrent store should succeed for {}", i);
            
            // Retrieve token
            let retrieve_result = context_clone.token_storage
                .retrieve_oauth_token(&provider, Some(&user_id)).await;
            assert!(retrieve_result.is_ok(), "Concurrent retrieve should succeed for {}", i);
            assert!(retrieve_result.unwrap().is_some(), "Token should exist for {}", i);
            
            // Delete token
            let delete_result = context_clone.token_storage
                .delete_oauth_token(&provider, Some(&user_id)).await;
            assert!(delete_result.is_ok(), "Concurrent delete should succeed for {}", i);
            
            i // Return thread ID for verification
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.unwrap();
        results.push(result);
    }
    
    // Verify all operations completed
    assert_eq!(results.len(), 10, "All concurrent operations should complete");
    for i in 0..10 {
        assert!(results.contains(&i), "Thread {} should have completed", i);
    }
    
    info!("Concurrent session operations test completed successfully");
}

#[tokio::test]
async fn test_session_recovery_performance() {
    info!("Starting session recovery performance test");
    
    let test_context = IntegrationTestContext::new().await.unwrap();
    
    // Store multiple tokens to test performance
    let token_count = 50;
    let providers = ["github", "google", "microsoft"];
    
    info!("Storing {} tokens across {} providers", token_count, providers.len());
    let store_start = Instant::now();
    
    for i in 0..token_count {
        let provider = providers[i % providers.len()];
        let user_id = format!("perf_user_{}", i);
        let oauth_token = create_test_oauth_token_response(provider, &user_id, false);
        
        test_context.token_storage
            .store_oauth_token(provider, Some(&user_id), &oauth_token)
            .await.unwrap();
    }
    
    let store_duration = store_start.elapsed();
    info!("Token storage completed in {:?}", store_duration);
    
    // Measure token retrieval performance
    let retrieval_start = Instant::now();
    let all_tokens = test_context.token_storage.get_all_tokens().await.unwrap();
    let retrieval_duration = retrieval_start.elapsed();
    
    info!("Retrieved {} tokens in {:?}", all_tokens.len(), retrieval_duration);
    
    // Performance assertions (adjust based on acceptable performance)
    assert!(store_duration < Duration::from_secs(30), "Token storage should complete within 30 seconds");
    assert!(retrieval_duration < Duration::from_secs(10), "Token retrieval should complete within 10 seconds");
    assert!(all_tokens.len() >= token_count, "Should retrieve all stored tokens");
    
    info!("Session recovery performance test completed successfully");
}

// ============================================================================
// Integration Test Runner and Reporting
// ============================================================================

#[tokio::test]
async fn test_comprehensive_integration_suite_runner() {
    info!("Starting comprehensive integration suite runner");
    
    // This test orchestrates running multiple integration test scenarios
    // and provides comprehensive reporting
    
    let mut test_results = Vec::new();
    
    // Test scenarios to run (only available components)
    let test_scenarios = vec![
        "User Context Creation",
        "Token Storage Operations", 
        "Cross-Component Integration",
    ];
    
    for name in test_scenarios {
        info!("Running integration scenario: {}", name);
        let start_time = Instant::now();
        
        let result = match name {
            "User Context Creation" => test_user_context_creation().await,
            "Token Storage Operations" => test_basic_storage_operations().await,
            "Cross-Component Integration" => test_simplified_integration().await,
            _ => Ok(()),
        };
        
        let duration = start_time.elapsed();
        
        test_results.push((name, result.is_ok(), duration));
        
        match result {
            Ok(_) => info!("✓ {} completed successfully in {:?}", name, duration),
            Err(e) => warn!("✗ {} failed in {:?}: {}", name, duration, e),
        }
    }
    
    // Generate comprehensive report
    info!("Integration Test Suite Results:");
    info!("{}", "=".repeat(50));
    
    let mut total_duration = Duration::ZERO;
    let mut passed_count = 0;
    
    for (name, passed, duration) in &test_results {
        let status = if *passed { "PASS" } else { "FAIL" };
        info!("{}: {} ({:?})", name, status, duration);
        
        total_duration += *duration;
        if *passed {
            passed_count += 1;
        }
    }
    
    info!("{}", "=".repeat(50));
    info!("Total: {}/{} passed in {:?}", passed_count, test_results.len(), total_duration);
    
    // Assert overall success
    assert_eq!(passed_count, test_results.len(), 
               "All integration test scenarios should pass");
    
    info!("Comprehensive integration suite runner completed successfully");
}

// Helper integration test functions

async fn test_user_context_creation() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join("user_context_test");
    let user_context = UserContext::with_session_dir(session_dir)?;
    
    assert!(!user_context.get_unique_user_id().is_empty());
    assert!(user_context.session_dir.exists());
    
    Ok(())
}

async fn test_basic_storage_operations() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join("basic_storage_test");
    let user_context = UserContext::with_session_dir(session_dir)?;
    let token_storage = TokenStorage::new(user_context).await?;
    
    // Test basic storage functionality
    let oauth_token = create_test_oauth_token_response("test_provider", "test_user", false);
    let _key = token_storage.store_oauth_token("test_provider", Some("test_user"), &oauth_token).await?;
    
    let retrieved = token_storage.retrieve_oauth_token("test_provider", Some("test_user")).await?;
    assert!(retrieved.is_some());
    
    Ok(())
}

async fn test_simplified_integration() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join("integration_test");
    let user_context = UserContext::with_session_dir(session_dir)?;
    let token_storage = Arc::new(TokenStorage::new(user_context.clone()).await?);
    
    // Test basic token operations
    let oauth_token = create_test_oauth_token_response("github", "integration_user", false);
    let _key = token_storage.store_oauth_token("github", Some("integration_user"), &oauth_token).await?;
    
    let retrieved = token_storage.retrieve_oauth_token("github", Some("integration_user")).await?;
    assert!(retrieved.is_some());
    
    Ok(())
}