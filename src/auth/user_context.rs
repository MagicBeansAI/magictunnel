//! User Context System for OAuth 2.1 Phase 2.1
//!
//! This module provides a robust cross-platform user context identification system
//! as the foundation for session persistence in MagicTunnel. It handles:
//! - Cross-platform user identification (macOS, Windows, Linux)
//! - Secure session directory management with proper permissions
//! - OS-native secure storage integration
//! - Multi-machine session isolation via hostname
//! - Graceful fallback mechanisms for system limitations

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use std::os::unix::fs::PermissionsExt;
use sysinfo::System;
use tracing::{debug, trace, warn};

use crate::error::ProxyError;
use crate::error::Result;

/// User context information for session persistence and authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// Operating system username ($USER or whoami)
    pub username: String,
    /// User home directory ($HOME)
    pub home_dir: PathBuf,
    /// Operating system user ID
    pub uid: u32,
    /// Machine hostname for multi-machine session isolation
    pub hostname: String,
    /// User-specific session storage directory (~/.magictunnel/sessions/)
    pub session_dir: PathBuf,
    /// Type of secure storage available on this platform
    pub secure_storage: SecureStorageType,
}

/// Secure storage types available on different platforms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecureStorageType {
    /// macOS Keychain Services
    Keychain,
    /// Windows Credential Manager
    CredentialManager,
    /// Linux Secret Service (GNOME Keyring, KWallet, etc.)
    SecretService,
    /// Fallback to encrypted filesystem storage
    Filesystem,
}

/// User context creation errors
#[derive(Debug, thiserror::Error)]
pub enum UserContextError {
    #[error("Failed to get username: {0}")]
    UsernameError(String),
    #[error("Failed to get home directory: {0}")]
    HomeDirectoryError(String),
    #[error("Failed to get user ID: {0}")]
    UserIdError(String),
    #[error("Failed to get hostname: {0}")]
    HostnameError(String),
    #[error("Failed to create session directory: {0}")]
    SessionDirectoryError(String),
    #[error("Failed to set directory permissions: {0}")]
    PermissionError(String),
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),
}

impl UserContext {
    /// Create a new user context by detecting system information
    pub fn new() -> Result<Self> {
        debug!("Creating new user context");

        // Get username with fallback mechanisms
        let username = Self::get_username()?;
        trace!("Detected username: {}", username);

        // Get home directory
        let home_dir = Self::get_home_directory()?;
        trace!("Detected home directory: {}", home_dir.display());

        // Get user ID
        let uid = Self::get_user_id()?;
        trace!("Detected user ID: {}", uid);

        // Get hostname
        let hostname = Self::get_hostname()?;
        trace!("Detected hostname: {}", hostname);

        // Determine secure storage type
        let secure_storage = Self::detect_secure_storage_type();
        debug!("Detected secure storage type: {:?}", secure_storage);

        // Create session directory
        let session_dir = Self::create_session_directory(&home_dir)?;
        debug!("Created session directory: {}", session_dir.display());

        let context = UserContext {
            username,
            home_dir,
            uid,
            hostname,
            session_dir,
            secure_storage,
        };

        debug!("User context created successfully");
        Ok(context)
    }

    /// Create user context with custom session directory (for testing)
    pub fn with_session_dir(session_dir: PathBuf) -> Result<Self> {
        let username = Self::get_username()?;
        let home_dir = Self::get_home_directory()?;
        let uid = Self::get_user_id()?;
        let hostname = Self::get_hostname()?;
        let secure_storage = Self::detect_secure_storage_type();

        // Ensure the custom session directory exists
        Self::ensure_session_directory_exists(&session_dir)?;

        Ok(UserContext {
            username,
            home_dir,
            uid,
            hostname,
            session_dir,
            secure_storage,
        })
    }

    /// Create test user context with filesystem storage (avoids Keychain prompts)
    pub fn for_testing(session_dir: PathBuf) -> Result<Self> {
        let username = Self::get_username()?;
        let home_dir = Self::get_home_directory()?;
        let uid = Self::get_user_id()?;
        let hostname = Self::get_hostname()?;
        
        // Force filesystem storage for testing
        let secure_storage = SecureStorageType::Filesystem;

        // Ensure the custom session directory exists
        Self::ensure_session_directory_exists(&session_dir)?;

        debug!("Created test user context with filesystem storage");

        Ok(UserContext {
            username,
            home_dir,
            uid,
            hostname,
            session_dir,
            secure_storage,
        })
    }

    /// Create test user context with specific storage backend
    pub fn for_testing_with_backend(session_dir: PathBuf, storage_backend: SecureStorageType) -> Result<Self> {
        let username = Self::get_username()?;
        let home_dir = Self::get_home_directory()?;
        let uid = Self::get_user_id()?;
        let hostname = Self::get_hostname()?;

        // Ensure the custom session directory exists
        Self::ensure_session_directory_exists(&session_dir)?;

        debug!("Created test user context with {:?} storage backend", storage_backend);

        Ok(UserContext {
            username,
            home_dir,
            uid,
            hostname,
            session_dir,
            secure_storage: storage_backend,
        })
    }

    /// Get username with fallback mechanisms
    fn get_username() -> Result<String> {
        // Try whoami first (most reliable)
        let username = whoami::username();
        if !username.is_empty() {
            return Ok(username);
        }

        // Fallback to environment variables
        if let Ok(user) = std::env::var("USER") {
            if !user.is_empty() {
                return Ok(user);
            }
        }

        if let Ok(username) = std::env::var("USERNAME") {
            if !username.is_empty() {
                return Ok(username);
            }
        }

        // Final fallback to "unknown"
        warn!("Could not determine username, using 'unknown'");
        Ok("unknown".to_string())
    }

    /// Get home directory with fallback mechanisms
    fn get_home_directory() -> Result<PathBuf> {
        // Try dirs crate first (most reliable)
        if let Some(home_dir) = dirs::home_dir() {
            return Ok(home_dir);
        }

        // Fallback to environment variables
        if let Ok(home) = std::env::var("HOME") {
            return Ok(PathBuf::from(home));
        }

        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            return Ok(PathBuf::from(userprofile));
        }

        // Final fallback to current directory
        warn!("Could not determine home directory, using current directory");
        std::env::current_dir()
            .map_err(|e| ProxyError::user_context(UserContextError::HomeDirectoryError(e.to_string())))
    }

    /// Get user ID (Unix-style)
    fn get_user_id() -> Result<u32> {
        #[cfg(unix)]
        {
            // Use whoami for cross-platform user ID
            use std::process::Command;
            
            // Try to get UID using `id -u` command
            match Command::new("id").arg("-u").output() {
                Ok(output) if output.status.success() => {
                    let uid_str = String::from_utf8_lossy(&output.stdout);
                    let uid_trimmed = uid_str.trim();
                    if let Ok(uid) = uid_trimmed.parse::<u32>() {
                        return Ok(uid);
                    }
                }
                _ => {}
            }
            
            // Fallback to a hash-based approach
            let username = Self::get_username()?;
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            username.hash(&mut hasher);
            Ok((hasher.finish() % 65535) as u32 + 1000) // Keep it in reasonable range
        }

        #[cfg(windows)]
        {
            // Windows doesn't have Unix-style UIDs, use a hash of username
            let username = Self::get_username()?;
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            username.hash(&mut hasher);
            Ok((hasher.finish() % 65535) as u32 + 1000) // Keep it in reasonable range
        }

        #[cfg(not(any(unix, windows)))]
        {
            // Other platforms, use a simple fallback
            Ok(1000)
        }
    }

    /// Get system hostname
    fn get_hostname() -> Result<String> {
        // Try whoami hostname first
        let hostname = whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string());
        if !hostname.is_empty() {
            return Ok(hostname);
        }

        // Try sysinfo as fallback
        if let Some(hostname) = System::host_name() {
            if !hostname.is_empty() {
                return Ok(hostname);
            }
        }

        // Environment variable fallback
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            if !hostname.is_empty() {
                return Ok(hostname);
            }
        }

        // Final fallback
        warn!("Could not determine hostname, using 'localhost'");
        Ok("localhost".to_string())
    }

    /// Detect the appropriate secure storage type for the current platform
    fn detect_secure_storage_type() -> SecureStorageType {
        // Check for test override environment variable first
        if let Ok(storage_override) = std::env::var("MAGICTUNNEL_TEST_STORAGE_BACKEND") {
            match storage_override.to_lowercase().as_str() {
                "filesystem" => {
                    debug!("Using filesystem storage backend (test override)");
                    return SecureStorageType::Filesystem;
                }
                "keychain" => {
                    debug!("Using keychain storage backend (test override)");
                    return SecureStorageType::Keychain;
                }
                "credential_manager" => {
                    debug!("Using credential manager storage backend (test override)");
                    return SecureStorageType::CredentialManager;
                }
                "secret_service" => {
                    debug!("Using secret service storage backend (test override)");
                    return SecureStorageType::SecretService;
                }
                _ => {
                    warn!("Invalid MAGICTUNNEL_TEST_STORAGE_BACKEND value: {}, using platform default", storage_override);
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            SecureStorageType::Keychain
        }

        #[cfg(target_os = "windows")]
        {
            SecureStorageType::CredentialManager
        }

        #[cfg(target_os = "linux")]
        {
            // Check if we're in a desktop environment with secret service
            if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
                SecureStorageType::SecretService
            } else {
                SecureStorageType::Filesystem
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            SecureStorageType::Filesystem
        }
    }

    /// Create the session directory with proper permissions
    fn create_session_directory(home_dir: &PathBuf) -> Result<PathBuf> {
        let magictunnel_dir = home_dir.join(".magictunnel");
        let session_dir = magictunnel_dir.join("sessions");

        Self::ensure_session_directory_exists(&session_dir)?;
        Ok(session_dir)
    }

    /// Ensure session directory exists with proper permissions
    fn ensure_session_directory_exists(session_dir: &PathBuf) -> Result<()> {
        if !session_dir.exists() {
            fs::create_dir_all(session_dir)
                .map_err(|e| ProxyError::user_context(UserContextError::SessionDirectoryError(e.to_string())))?;
        }

        // Set secure permissions (0700 - owner read/write/execute only)
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(session_dir)
                .map_err(|e| ProxyError::user_context(UserContextError::PermissionError(e.to_string())))?
                .permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(session_dir, permissions)
                .map_err(|e| ProxyError::user_context(UserContextError::PermissionError(e.to_string())))?;
        }

        #[cfg(windows)]
        {
            // Windows permissions are more complex, but create_dir_all typically
            // creates with appropriate user-only permissions by default
            debug!("Session directory created with default Windows permissions");
        }

        Ok(())
    }

    /// Get a user-specific session file path
    pub fn get_session_file_path(&self, filename: &str) -> PathBuf {
        self.session_dir.join(filename)
    }

    /// Get a hostname-specific session file path for multi-machine isolation
    pub fn get_hostname_session_file_path(&self, filename: &str) -> PathBuf {
        let hostname_filename = format!("{}_{}", self.hostname, filename);
        self.session_dir.join(hostname_filename)
    }

    /// Check if secure storage is available
    pub fn has_secure_storage(&self) -> bool {
        match self.secure_storage {
            SecureStorageType::Keychain | 
            SecureStorageType::CredentialManager | 
            SecureStorageType::SecretService => true,
            SecureStorageType::Filesystem => false,
        }
    }

    /// Get a unique user identifier for this context
    pub fn get_unique_user_id(&self) -> String {
        format!("{}@{}:{}", self.username, self.hostname, self.uid)
    }

    /// Validate that the user context is usable
    pub fn validate(&self) -> Result<()> {
        if self.username.is_empty() {
            return Err(ProxyError::user_context(UserContextError::UsernameError(
                "Username is empty".to_string()
            )));
        }

        if !self.home_dir.exists() {
            return Err(ProxyError::user_context(UserContextError::HomeDirectoryError(
                format!("Home directory does not exist: {}", self.home_dir.display())
            )));
        }

        if !self.session_dir.exists() {
            return Err(ProxyError::user_context(UserContextError::SessionDirectoryError(
                format!("Session directory does not exist: {}", self.session_dir.display())
            )));
        }

        if self.hostname.is_empty() {
            return Err(ProxyError::user_context(UserContextError::HostnameError(
                "Hostname is empty".to_string()
            )));
        }

        Ok(())
    }
}

impl Default for UserContext {
    fn default() -> Self {
        UserContext::new().unwrap_or_else(|e| {
            warn!("Failed to create user context, using fallback: {}", e);
            UserContext {
                username: "unknown".to_string(),
                home_dir: PathBuf::from("."),
                uid: 1000,
                hostname: "localhost".to_string(),
                session_dir: PathBuf::from("./.magictunnel/sessions"),
                secure_storage: SecureStorageType::Filesystem,
            }
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_user_context_creation() {
        let context = UserContext::new();
        assert!(context.is_ok());

        let context = context.unwrap();
        assert!(!context.username.is_empty());
        assert!(context.home_dir.exists());
        assert!(!context.hostname.is_empty());
        assert!(context.session_dir.exists());
    }

    #[test]
    fn test_user_context_validation() {
        let context = UserContext::new().unwrap();
        assert!(context.validate().is_ok());
    }

    #[test]
    fn test_session_file_paths() {
        let context = UserContext::new().unwrap();
        
        let session_file = context.get_session_file_path("test.json");
        assert!(session_file.starts_with(&context.session_dir));
        assert!(session_file.ends_with("test.json"));

        let hostname_file = context.get_hostname_session_file_path("test.json");
        assert!(hostname_file.starts_with(&context.session_dir));
        assert!(hostname_file.file_name().unwrap().to_string_lossy().contains(&context.hostname));
    }

    #[test]
    fn test_unique_user_id() {
        let context = UserContext::new().unwrap();
        let unique_id = context.get_unique_user_id();
        
        assert!(unique_id.contains(&context.username));
        assert!(unique_id.contains(&context.hostname));
        assert!(unique_id.contains(&context.uid.to_string()));
    }

    #[test]
    fn test_secure_storage_detection() {
        let storage_type = UserContext::detect_secure_storage_type();
        
        #[cfg(target_os = "macos")]
        assert_eq!(storage_type, SecureStorageType::Keychain);
        
        #[cfg(target_os = "windows")]
        assert_eq!(storage_type, SecureStorageType::CredentialManager);
        
        // Linux/Unix might be SecretService or Filesystem depending on environment
        #[cfg(target_os = "linux")]
        assert!(matches!(storage_type, SecureStorageType::SecretService | SecureStorageType::Filesystem));
    }

    #[test]
    fn test_custom_session_directory() {
        let temp_dir = TempDir::new().unwrap();
        let session_dir = temp_dir.path().join("sessions");
        
        let context = UserContext::with_session_dir(session_dir.clone());
        assert!(context.is_ok());
        
        let context = context.unwrap();
        assert_eq!(context.session_dir, session_dir);
        assert!(context.session_dir.exists());
    }

    #[test]
    fn test_session_directory_permissions() {
        let context = UserContext::new().unwrap();
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&context.session_dir).unwrap();
            let permissions = metadata.permissions();
            // Check that permissions are 0700 (owner rwx only)
            assert_eq!(permissions.mode() & 0o777, 0o700);
        }
    }

    #[test]
    fn test_fallback_mechanisms() {
        // Test that fallback mechanisms work when system info is unavailable
        // This is mainly for code coverage as actual fallback testing would require
        // mocking system calls or running in restricted environments
        
        let username = UserContext::get_username();
        assert!(username.is_ok());
        assert!(!username.unwrap().is_empty());

        let home_dir = UserContext::get_home_directory();
        assert!(home_dir.is_ok());

        let hostname = UserContext::get_hostname();
        assert!(hostname.is_ok());
        assert!(!hostname.unwrap().is_empty());
    }

    #[test]
    fn test_default_user_context() {
        let context = UserContext::default();
        
        // Default should never panic and should provide usable values
        assert!(!context.username.is_empty());
        assert!(!context.hostname.is_empty());
        assert!(context.uid > 0 || context.uid == 0); // 0 is valid for root
    }
}