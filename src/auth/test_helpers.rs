//! Test Helper Functions for OAuth Authentication Testing
//!
//! This module provides utilities to test OAuth authentication without triggering
//! system keychain prompts or other interactive authentication flows.
//!
//! ## Usage Patterns:
//!
//! ### 1. Environment Variable Override (Recommended for CI/CD)
//! ```bash
//! export MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem
//! cargo test
//! ```
//!
//! ### 2. Test Helper Functions (Explicit control)
//! ```rust
//! use magictunnel::auth::test_helpers::*;
//! 
//! let user_context = create_filesystem_test_user_context().await?;
//! let token_storage = create_mock_token_storage().await?;
//! ```
//!
//! ### 3. Mock Backend (Pure unit testing)
//! ```rust
//! let token_storage = TokenStorage::new_with_mock_backend(user_context).await?;
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use crate::auth::{UserContext, TokenStorage, SecureStorageType, MockStorage, SecureStorage};
use crate::error::Result;
use tempfile::TempDir;
use tracing::debug;

/// Create a test user context that uses filesystem storage (avoids Keychain prompts)
pub async fn create_filesystem_test_user_context() -> Result<UserContext> {
    let temp_dir = TempDir::new().map_err(|e| crate::error::ProxyError::config(format!("Failed to create temp dir: {}", e)))?;
    let session_dir = temp_dir.path().join("test_sessions");
    
    // Use the explicit test creation method
    UserContext::for_testing(session_dir)
}

/// Create a test user context with custom session directory and filesystem storage
pub async fn create_filesystem_test_user_context_with_dir(session_dir: PathBuf) -> Result<UserContext> {
    UserContext::for_testing(session_dir)
}

/// Create a test user context with specific storage backend
pub async fn create_test_user_context_with_backend(session_dir: PathBuf, backend: SecureStorageType) -> Result<UserContext> {
    UserContext::for_testing_with_backend(session_dir, backend)
}

/// Create a TokenStorage instance with mock backend (in-memory, no system dependencies)
pub async fn create_mock_token_storage() -> Result<TokenStorage> {
    let user_context = create_filesystem_test_user_context().await?;
    TokenStorage::new_with_mock_backend(user_context).await
}

/// Create a TokenStorage instance with filesystem backend (encrypted files, no Keychain)
pub async fn create_filesystem_token_storage() -> Result<TokenStorage> {
    let user_context = create_filesystem_test_user_context().await?;
    TokenStorage::new(user_context).await
}

/// Create a TokenStorage instance with custom user context
pub async fn create_token_storage_with_context(user_context: UserContext) -> Result<TokenStorage> {
    TokenStorage::new(user_context).await
}

/// Create a mock storage backend directly
pub fn create_mock_storage_backend() -> Arc<MockStorage> {
    Arc::new(MockStorage::new())
}

/// Set up environment for filesystem-only testing (call this in test setup)
pub fn setup_filesystem_testing_environment() {
    std::env::set_var("MAGICTUNNEL_TEST_STORAGE_BACKEND", "filesystem");
    debug!("Set up filesystem testing environment");
}

/// Clean up test environment (call this in test cleanup)
pub fn cleanup_test_environment() {
    std::env::remove_var("MAGICTUNNEL_TEST_STORAGE_BACKEND");
    debug!("Cleaned up test environment");
}

/// Macro to set up filesystem testing for a test function
#[macro_export]
macro_rules! filesystem_test {
    ($test_fn:expr) => {{
        use $crate::auth::test_helpers::{setup_filesystem_testing_environment, cleanup_test_environment};
        
        setup_filesystem_testing_environment();
        let result = $test_fn.await;
        cleanup_test_environment();
        result
    }}
}

/// Macro to create a test user context with automatic cleanup
#[macro_export] 
macro_rules! test_user_context {
    () => {{
        use tempfile::TempDir;
        use $crate::auth::UserContext;
        
        let temp_dir = TempDir::new().unwrap();
        let session_dir = temp_dir.path().join("test_sessions");
        UserContext::for_testing(session_dir)
    }}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::OAuthTokenResponse;

    #[tokio::test]
    async fn test_filesystem_test_context_creation() {
        let user_context = create_filesystem_test_user_context().await.unwrap();
        assert_eq!(user_context.secure_storage, SecureStorageType::Filesystem);
        assert!(user_context.session_dir.exists());
    }

    #[tokio::test]
    async fn test_mock_token_storage_creation() {
        let token_storage = create_mock_token_storage().await.unwrap();
        assert!(token_storage.is_storage_available().await);
    }

    #[tokio::test]
    async fn test_filesystem_token_storage_creation() {
        let token_storage = create_filesystem_token_storage().await.unwrap();
        assert!(token_storage.is_storage_available().await);
        assert_eq!(*token_storage.storage_type(), SecureStorageType::Filesystem);
    }

    #[tokio::test]
    async fn test_environment_variable_override() {
        setup_filesystem_testing_environment();
        
        let temp_dir = TempDir::new().unwrap();
        let session_dir = temp_dir.path().join("env_test_sessions");
        let user_context = UserContext::with_session_dir(session_dir).unwrap();
        
        assert_eq!(user_context.secure_storage, SecureStorageType::Filesystem);
        
        cleanup_test_environment();
    }

    #[tokio::test]
    async fn test_mock_storage_operations() {
        let mock_storage = create_mock_storage_backend();
        
        // Test basic operations
        let token_data = crate::auth::TokenData::from_oauth_response(
            &OAuthTokenResponse {
                access_token: "test_token".to_string().into(),
                token_type: "Bearer".to_string(),
                expires_in: Some(3600),
                refresh_token: None,
                scope: None,
                audience: None,
                resource: None,
            },
            "test_provider".to_string(),
            Some("test_user".to_string()),
        );

        // Store token
        mock_storage.store_token("test_key", &token_data).await.unwrap();
        assert_eq!(mock_storage.len(), 1);

        // Retrieve token
        let retrieved = mock_storage.retrieve_token("test_key").await.unwrap();
        assert!(retrieved.is_some());

        // Delete token
        mock_storage.delete_token("test_key").await.unwrap();
        assert_eq!(mock_storage.len(), 0);
    }

    #[tokio::test] 
    async fn test_macro_usage() {
        let result = filesystem_test!(async {
            let user_context = test_user_context!().unwrap();
            assert_eq!(user_context.secure_storage, SecureStorageType::Filesystem);
            Ok::<(), crate::error::ProxyError>(())
        });
        
        assert!(result.is_ok());
    }
}