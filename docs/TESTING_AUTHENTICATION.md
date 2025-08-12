# Testing OAuth Authentication Without Keychain Prompts

This guide provides comprehensive solutions for testing MagicTunnel's OAuth authentication system without triggering macOS Keychain prompts or other system-dependent authentication dialogs.

## Quick Summary ✅

**4 Complete Solutions Implemented:**
- ✅ **Environment Variable Override** - `MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem` (Recommended for CI/CD)
- ✅ **Test Helper Functions** - Explicit control with `create_filesystem_test_user_context()` 
- ✅ **Mock Storage Backend** - In-memory storage for pure unit testing
- ✅ **CI/CD Configuration** - Complete GitHub Actions setup with cross-platform testing

**Benefits:**
- No system prompts (Keychain, Credential Manager, Secret Service)
- Cross-platform consistency (macOS/Windows/Linux)
- Complete test isolation and backward compatibility
- Multiple testing strategies for different use cases

## Problem Statement

MagicTunnel's OAuth 2.1 authentication system uses platform-specific secure storage:
- **macOS**: Keychain Services (triggers interactive prompts)
- **Windows**: Credential Manager (may require user interaction)
- **Linux**: Secret Service (may require desktop environment)

These system dependencies break automated testing and CI/CD pipelines.

## Solutions Overview

### 1. Environment Variable Override (Recommended for CI/CD)
### 2. Test Helper Functions (Explicit control in tests)
### 3. Mock Storage Backend (Pure unit testing)
### 4. CI/CD Configuration (Automated testing setup)

---

## Solution 1: Environment Variable Override

Set `MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem` to force filesystem storage for all tests.

### Usage

```bash
# For single test run
export MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem
cargo test

# For CI/CD pipelines
MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem cargo test
```

### Supported Backend Values

- `filesystem` - Encrypted filesystem storage (recommended for testing)
- `keychain` - macOS Keychain (will prompt in interactive environments)
- `credential_manager` - Windows Credential Manager
- `secret_service` - Linux Secret Service

### Example CI Configuration

```yaml
# .github/workflows/test.yml
env:
  MAGICTUNNEL_TEST_STORAGE_BACKEND: filesystem
  RUST_LOG: debug
  RUST_BACKTRACE: 1
```

---

## Solution 2: Test Helper Functions

Use explicit test helper functions for granular control over storage backends.

### Available Helper Functions

```rust
use magictunnel::auth::test_helpers::*;

// Create filesystem-based test context (avoids Keychain)
let user_context = create_filesystem_test_user_context().await?;

// Create mock storage (in-memory, no system dependencies)
let token_storage = create_mock_token_storage().await?;

// Create filesystem storage explicitly
let token_storage = create_filesystem_token_storage().await?;

// Create with specific backend
let user_context = create_test_user_context_with_backend(
    session_dir, 
    SecureStorageType::Filesystem
).await?;
```

### Example Test Implementation

```rust
use magictunnel::auth::test_helpers::*;
use magictunnel::auth::{TokenStorage, OAuthTokenResponse};

#[tokio::test]
async fn test_oauth_without_keychain() {
    // Use filesystem storage to avoid Keychain prompts
    let user_context = create_filesystem_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    
    // Test OAuth token operations
    let oauth_token = OAuthTokenResponse {
        access_token: "test_token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: Some(3600),
        refresh_token: None,
        scope: None,
        audience: None,
        resource: None,
    };
    
    // Store, retrieve, and verify token
    let key = token_storage.store_oauth_token("github", Some("testuser"), &oauth_token).await.unwrap();
    let retrieved = token_storage.retrieve_oauth_token("github", Some("testuser")).await.unwrap();
    
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().access_token, oauth_token.access_token);
}
```

### Convenience Macros

```rust
#[tokio::test]
async fn test_with_filesystem_storage() {
    let result = filesystem_test!(async {
        let user_context = test_user_context!().unwrap();
        assert_eq!(user_context.secure_storage, SecureStorageType::Filesystem);
        Ok::<(), ProxyError>(())
    });
    
    assert!(result.is_ok());
}
```

---

## Solution 3: Mock Storage Backend

Use in-memory mock storage for pure unit testing without any system dependencies.

### Mock Storage Features

- **In-memory storage** - No file system or network dependencies
- **Thread-safe** - Supports concurrent operations
- **Test utilities** - Helper methods for test assertions
- **Automatic cleanup** - Memory is freed when mock is dropped

### Usage Examples

```rust
use magictunnel::auth::{TokenStorage, UserContext, SecureStorageType};

#[tokio::test]
async fn test_with_mock_storage() {
    // Create user context (storage type doesn't matter for mock)
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join("test_sessions");
    let user_context = UserContext::for_testing(session_dir).unwrap();
    
    // Use mock backend
    let token_storage = TokenStorage::new_with_mock_backend(user_context).await.unwrap();
    
    // All operations are in-memory only
    let oauth_token = create_test_oauth_token();
    let key = token_storage.store_oauth_token("github", Some("user"), &oauth_token).await.unwrap();
    let retrieved = token_storage.retrieve_oauth_token("github", Some("user")).await.unwrap();
    
    assert!(retrieved.is_some());
}

// Or use helper function
#[tokio::test] 
async fn test_with_mock_helper() {
    let token_storage = create_mock_token_storage().await.unwrap();
    
    // Test operations without system dependencies
    // ...
}
```

### Direct Mock Storage Access

```rust
use magictunnel::auth::test_helpers::*;

#[tokio::test]
async fn test_mock_storage_directly() {
    let mock_storage = create_mock_storage_backend();
    
    // Direct storage operations
    let token_data = create_test_token_data();
    mock_storage.store_token("test_key", &token_data).await.unwrap();
    
    assert_eq!(mock_storage.len(), 1);
    assert!(!mock_storage.is_empty());
    
    let retrieved = mock_storage.retrieve_token("test_key").await.unwrap();
    assert!(retrieved.is_some());
    
    mock_storage.clear();
    assert!(mock_storage.is_empty());
}
```

---

## Solution 4: CI/CD Configuration

Complete setup for automated testing environments.

### GitHub Actions Configuration

```yaml
name: Tests

on:
  push:
    branches: [ main, master, develop ]
  pull_request:
    branches: [ main, master, develop ]

env:
  CARGO_TERM_COLOR: always
  # Force filesystem storage for automated testing
  MAGICTUNNEL_TEST_STORAGE_BACKEND: filesystem
  # Additional test configuration
  MAGICTUNNEL_DISABLE_NETWORK_TESTS: true
  RUST_LOG: debug
  RUST_BACKTRACE: 1

jobs:
  test:
    name: Test Suite
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable]

    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@v1
      with:
        toolchain: ${{ matrix.rust }}

    - name: Set up test environment
      run: |
        echo "MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem" >> $GITHUB_ENV
        mkdir -p test-data/sessions test-data/tokens
        chmod 700 test-data/sessions test-data/tokens
      shell: bash

    - name: Run OAuth authentication tests
      run: cargo test --test "*oauth*" --test "*auth*" --test "*token_storage*" --verbose
      env:
        MAGICTUNNEL_TEST_STORAGE_BACKEND: filesystem
        MAGICTUNNEL_DISABLE_REAL_OAUTH_TESTS: true
        MAGICTUNNEL_TEST_SESSION_DIR: ${{ github.workspace }}/test-data/sessions

    - name: Clean up test data
      run: rm -rf test-data
      shell: bash
```

### Local Development Script

Create `scripts/test-auth.sh`:

```bash
#!/bin/bash
set -e

echo "Setting up test environment for OAuth authentication tests..."

# Set environment variables
export MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Create test directories
mkdir -p test-data/sessions test-data/tokens
chmod 700 test-data/sessions test-data/tokens

echo "Running OAuth authentication tests..."

# Run auth-specific tests
cargo test --test "*oauth*" --test "*auth*" --test "*token_storage*" --verbose

echo "Running integration tests..."
cargo test --test "*integration*" --verbose --no-default-features

# Cleanup
echo "Cleaning up test data..."
rm -rf test-data

echo "All authentication tests completed successfully!"
```

Make it executable:
```bash
chmod +x scripts/test-auth.sh
./scripts/test-auth.sh
```

---

## Platform-Specific Considerations

### macOS
- **Keychain Access**: Will prompt for keychain access if not using filesystem override
- **Test Isolation**: Use temporary directories to avoid polluting user keychain
- **CI/CD**: GitHub Actions macOS runners work fine with filesystem storage

### Windows
- **Credential Manager**: May require user interaction in desktop environments
- **Service Environments**: Works well in CI/CD with filesystem storage
- **Permissions**: Ensure test directories have proper Windows permissions

### Linux
- **Secret Service**: Requires desktop environment (GNOME Keyring, KWallet)
- **Headless Environments**: Automatically falls back to filesystem storage
- **Docker/CI**: Works seamlessly with filesystem storage

---

## Common Test Patterns

### Pattern 1: Simple OAuth Token Test

```rust
#[tokio::test]
async fn test_oauth_token_lifecycle() {
    setup_filesystem_testing_environment();
    
    let user_context = create_filesystem_test_user_context().await.unwrap();
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    
    let oauth_token = create_test_oauth_token("github", Some("testuser"));
    
    // Store
    let key = token_storage.store_oauth_token("github", Some("testuser"), &oauth_token).await.unwrap();
    assert!(!key.is_empty());
    
    // Retrieve
    let retrieved = token_storage.retrieve_oauth_token("github", Some("testuser")).await.unwrap();
    assert!(retrieved.is_some());
    
    // Delete
    token_storage.delete_oauth_token("github", Some("testuser")).await.unwrap();
    let deleted = token_storage.retrieve_oauth_token("github", Some("testuser")).await.unwrap();
    assert!(deleted.is_none());
    
    cleanup_test_environment();
}
```

### Pattern 2: Multiple Provider Test

```rust
#[tokio::test]
async fn test_multiple_oauth_providers() {
    let token_storage = create_mock_token_storage().await.unwrap();
    
    let providers = [("github", "user1"), ("google", "user2"), ("microsoft", "user3")];
    
    for (provider, user) in &providers {
        let oauth_token = create_test_oauth_token(provider, Some(user));
        token_storage.store_oauth_token(provider, Some(user), &oauth_token).await.unwrap();
    }
    
    // Verify all tokens are stored
    let all_tokens = token_storage.get_all_tokens().await.unwrap();
    assert_eq!(all_tokens.len(), providers.len());
    
    // Verify each token
    for (provider, user) in &providers {
        let retrieved = token_storage.retrieve_oauth_token(provider, Some(user)).await.unwrap();
        assert!(retrieved.is_some());
    }
}
```

### Pattern 3: Concurrent Access Test

```rust
#[tokio::test]
async fn test_concurrent_oauth_operations() {
    let token_storage = Arc::new(create_mock_token_storage().await.unwrap());
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let storage_clone = Arc::clone(&token_storage);
        let handle = tokio::spawn(async move {
            let provider = format!("provider_{}", i);
            let user = format!("user_{}", i);
            let oauth_token = create_test_oauth_token(&provider, Some(&user));
            
            // Concurrent operations
            storage_clone.store_oauth_token(&provider, Some(&user), &oauth_token).await.unwrap();
            let retrieved = storage_clone.retrieve_oauth_token(&provider, Some(&user)).await.unwrap();
            assert!(retrieved.is_some());
            
            storage_clone.delete_oauth_token(&provider, Some(&user)).await.unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
}
```

---

## Troubleshooting

### Common Issues

1. **Keychain Prompts Still Appear**
   - Verify `MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem` is set
   - Use explicit test helpers instead of `UserContext::new()`
   - Check that environment variable is set before importing modules

2. **Permission Errors**
   - Ensure test directories have proper permissions (0700)
   - Use temporary directories for test isolation
   - Clean up test data after tests

3. **CI/CD Failures**
   - Set environment variables at job level, not just step level
   - Use absolute paths for test directories
   - Disable network-dependent tests in CI

4. **Cross-Platform Issues**
   - Use filesystem storage for consistent behavior
   - Test on all target platforms in CI
   - Handle platform-specific path separators

### Debug Commands

```bash
# Check if environment variable is set
echo $MAGICTUNNEL_TEST_STORAGE_BACKEND

# Run specific test with debug logging
RUST_LOG=magictunnel::auth=debug cargo test test_oauth_token_lifecycle -- --nocapture

# Run tests with filesystem storage forced
MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem cargo test --test token_storage_integration_test

# Check test directories
ls -la test-data/
ls -la ~/.magictunnel/sessions/
```

---

## Best Practices

1. **Always Use Environment Override in CI/CD**
   ```yaml
   env:
     MAGICTUNNEL_TEST_STORAGE_BACKEND: filesystem
   ```

2. **Use Test Helpers for Explicit Control**
   ```rust
   let user_context = create_filesystem_test_user_context().await?;
   ```

3. **Prefer Mock Storage for Unit Tests**
   ```rust
   let token_storage = create_mock_token_storage().await?;
   ```

4. **Clean Up Test Data**
   ```rust
   // Use temporary directories that are automatically cleaned up
   let temp_dir = TempDir::new().unwrap();
   ```

5. **Test Cross-Platform Compatibility**
   - Test on macOS, Windows, and Linux
   - Use filesystem storage for consistent behavior
   - Handle platform-specific edge cases

6. **Isolate Tests**
   - Use unique session directories per test
   - Don't share storage between tests
   - Clean up after each test

---

## Migration Guide

### Existing Tests

If you have existing tests using real Keychain:

**Before:**
```rust
#[tokio::test]
async fn test_oauth() {
    let user_context = UserContext::new().unwrap(); // Uses real Keychain
    let token_storage = TokenStorage::new(user_context).await.unwrap();
    // ... test logic
}
```

**After:**
```rust
#[tokio::test]
async fn test_oauth() {
    // Option 1: Use environment variable (recommended)
    setup_filesystem_testing_environment();
    let user_context = UserContext::new().unwrap(); // Now uses filesystem
    
    // Option 2: Use explicit helper
    let user_context = create_filesystem_test_user_context().await.unwrap();
    
    // Option 3: Use mock storage
    let token_storage = create_mock_token_storage().await.unwrap();
    
    // ... test logic remains the same
    
    cleanup_test_environment(); // If using Option 1
}
```

### New Tests

For new tests, prefer explicit test helpers:

```rust
use magictunnel::auth::test_helpers::*;

#[tokio::test]
async fn test_new_oauth_feature() {
    let token_storage = create_mock_token_storage().await.unwrap();
    // ... test logic
}
```

---

This guide ensures that OAuth authentication tests run reliably across all platforms without interactive prompts, making them suitable for automated CI/CD pipelines and development workflows.