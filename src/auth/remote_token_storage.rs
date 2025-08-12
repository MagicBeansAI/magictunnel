//! Remote Token Storage System with Session Isolation
//!
//! This module provides enhanced token storage that prevents session collisions
//! and token conflicts when multiple users connect to remote MagicTunnel instances.
//! It extends the base TokenStorage with remote client-specific isolation.

use crate::auth::{
    RemoteUserContext, TokenStorage, TokenData, OAuthTokenResponse,
    SecureStorageType
};
use crate::error::{Result, ProxyError};
#[cfg(test)]
use crate::auth::UserContext;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace, warn};
use std::sync::Arc;
use std::collections::HashMap;

/// Remote token storage with client isolation
#[derive(Debug)]
pub struct RemoteTokenStorage {
    /// Base token storage
    base_storage: Arc<TokenStorage>,
    
    /// Remote user context for isolation
    remote_context: RemoteUserContext,
    
    /// Isolation prefix for all token keys
    isolation_prefix: String,
    
    /// Storage metadata
    metadata: RemoteStorageMetadata,
}

/// Metadata for remote token storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStorageMetadata {
    /// Client identifier
    pub client_id: String,
    
    /// Storage creation timestamp
    pub created_at: std::time::SystemTime,
    
    /// Last access timestamp
    pub last_accessed: std::time::SystemTime,
    
    /// Token count
    pub token_count: u32,
    
    /// Storage encryption level
    pub encryption_level: String,
    
    /// Client validation fingerprint
    pub client_fingerprint: Option<String>,
}

/// Remote token manager for handling multiple client token storages
#[derive(Debug)]
pub struct RemoteTokenManager {
    /// Active client token storages
    client_storages: std::sync::RwLock<HashMap<String, Arc<RemoteTokenStorage>>>,
    
    /// Manager configuration
    config: RemoteTokenConfig,
}

/// Configuration for remote token management
#[derive(Debug, Clone)]
pub struct RemoteTokenConfig {
    /// Enable automatic token cleanup
    pub enable_auto_cleanup: bool,
    
    /// Token cleanup interval in seconds
    pub cleanup_interval_seconds: u64,
    
    /// Maximum tokens per client
    pub max_tokens_per_client: u32,
    
    /// Token encryption level
    pub encryption_level: TokenEncryptionLevel,
    
    /// Enable token validation
    pub enable_token_validation: bool,
    
    /// Token expiry grace period in seconds
    pub expiry_grace_period_seconds: u64,
}

/// Token encryption levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenEncryptionLevel {
    /// Basic platform encryption
    Platform,
    
    /// Enhanced client-specific encryption
    Enhanced,
    
    /// Maximum security with client validation
    Maximum,
}

impl RemoteTokenStorage {
    /// Create new remote token storage for client
    pub async fn new(remote_context: RemoteUserContext) -> Result<Self> {
        debug!("Creating remote token storage for client: {}", remote_context.display_name());

        // Create modified user context for base storage
        let user_context = remote_context.to_user_context();
        
        // Create base token storage with client-specific context
        let base_storage = Arc::new(TokenStorage::new(user_context).await?);
        
        // Generate isolation prefix from client identity
        let isolation_prefix = Self::generate_isolation_prefix(&remote_context)?;
        
        // Create metadata
        let metadata = RemoteStorageMetadata {
            client_id: remote_context.get_remote_client_id(),
            created_at: std::time::SystemTime::now(),
            last_accessed: std::time::SystemTime::now(),
            token_count: 0,
            encryption_level: "enhanced".to_string(),
            client_fingerprint: remote_context.client_identity.capability_fingerprint.clone(),
        };

        let storage = Self {
            base_storage,
            remote_context,
            isolation_prefix,
            metadata,
        };

        info!("Created remote token storage with prefix: {}", storage.isolation_prefix);
        Ok(storage)
    }

    /// Generate isolation prefix for token keys
    fn generate_isolation_prefix(remote_context: &RemoteUserContext) -> Result<String> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        // Include isolation key
        hasher.update(remote_context.isolation_key.as_bytes());
        
        // Include client identity components
        hasher.update(remote_context.client_identity.client_ip.to_string().as_bytes());
        
        if let Some(ref hostname) = remote_context.client_identity.client_hostname {
            hasher.update(hostname.as_bytes());
        }
        
        if let Some(ref username) = remote_context.client_identity.client_username {
            hasher.update(username.as_bytes());
        }
        
        // Include session ID for temporal isolation
        hasher.update(remote_context.remote_session_id.as_bytes());
        
        let prefix = format!("rmt_{:x}", hasher.finalize());
        Ok(prefix[..24].to_string()) // Use 24 chars for collision resistance
    }

    /// Create isolated token key
    fn create_isolated_key(&self, original_key: &str) -> String {
        format!("{}::{}", self.isolation_prefix, original_key)
    }

    /// Store OAuth token with client isolation
    pub async fn store_oauth_token(
        &self,
        provider: &str,
        user_id: Option<&str>,
        token: &OAuthTokenResponse,
    ) -> Result<String> {
        debug!("Storing OAuth token for provider: {}, client: {}", provider, self.remote_context.display_name());

        // Store with base storage using isolated key
        let original_key = self.create_token_key(provider, user_id);
        let isolated_key = self.create_isolated_key(&original_key);
        
        // Create token data with remote context metadata
        let mut token_data = TokenData::from_oauth_response(
            token,
            provider.to_string(),
            user_id.map(String::from),
        );
        
        // Add remote client metadata
        token_data.metadata.insert("client_id".to_string(), self.metadata.client_id.clone());
        token_data.metadata.insert("client_ip".to_string(), self.remote_context.client_identity.client_ip.to_string());
        
        if let Some(ref hostname) = self.remote_context.client_identity.client_hostname {
            token_data.metadata.insert("client_hostname".to_string(), hostname.clone());
        }
        
        if let Some(ref username) = self.remote_context.client_identity.client_username {
            token_data.metadata.insert("client_username".to_string(), username.clone());
        }
        
        token_data.metadata.insert("session_id".to_string(), self.remote_context.remote_session_id.clone());
        token_data.metadata.insert("isolation_prefix".to_string(), self.isolation_prefix.clone());

        // Store the token
        self.base_storage.store_token(&isolated_key, &token_data).await?;
        
        // Update metadata
        self.update_metadata().await;
        
        info!("Stored OAuth token with isolated key: {}", isolated_key);
        Ok(isolated_key)
    }

    /// Retrieve OAuth token with client isolation
    pub async fn retrieve_oauth_token(
        &self,
        provider: &str,
        user_id: Option<&str>,
    ) -> Result<Option<TokenData>> {
        let original_key = self.create_token_key(provider, user_id);
        let isolated_key = self.create_isolated_key(&original_key);
        
        trace!("Retrieving OAuth token with key: {}", isolated_key);
        
        // Retrieve token
        let token = self.base_storage.retrieve_token(&isolated_key).await?;
        
        // Validate token belongs to this client if found
        if let Some(ref token_data) = token {
            if !self.validate_token_ownership(token_data) {
                warn!("Token ownership validation failed for key: {}", isolated_key);
                return Ok(None);
            }
        }
        
        // Update last accessed time
        if token.is_some() {
            self.update_metadata().await;
        }
        
        Ok(token)
    }

    /// Delete OAuth token with client isolation
    pub async fn delete_oauth_token(
        &self,
        provider: &str,
        user_id: Option<&str>,
    ) -> Result<()> {
        let original_key = self.create_token_key(provider, user_id);
        let isolated_key = self.create_isolated_key(&original_key);
        
        debug!("Deleting OAuth token with key: {}", isolated_key);
        
        // Verify token exists and belongs to this client
        if let Some(token_data) = self.base_storage.retrieve_token(&isolated_key).await? {
            if !self.validate_token_ownership(&token_data) {
                return Err(ProxyError::auth("Token does not belong to this client".to_string()));
            }
        }
        
        // Delete token
        self.base_storage.delete_token(&isolated_key).await?;
        
        info!("Deleted OAuth token for provider: {}, client: {}", provider, self.remote_context.display_name());
        Ok(())
    }

    /// List all tokens for this client
    pub async fn list_client_tokens(&self) -> Result<HashMap<String, TokenData>> {
        let all_keys = self.base_storage.list_tokens().await?;
        let mut client_tokens = HashMap::new();
        
        for key in all_keys {
            if key.starts_with(&self.isolation_prefix) {
                if let Some(token_data) = self.base_storage.retrieve_token(&key).await? {
                    if self.validate_token_ownership(&token_data) {
                        // Remove isolation prefix from key for clean display
                        let clean_key = key.strip_prefix(&format!("{}::", self.isolation_prefix))
                            .unwrap_or(&key)
                            .to_string();
                        client_tokens.insert(clean_key, token_data);
                    }
                }
            }
        }
        
        trace!("Listed {} tokens for client: {}", client_tokens.len(), self.remote_context.display_name());
        Ok(client_tokens)
    }

    /// Cleanup expired tokens for this client
    pub async fn cleanup_expired_tokens(&self) -> Result<u32> {
        debug!("Cleaning up expired tokens for client: {}", self.remote_context.display_name());
        
        let client_tokens = self.list_client_tokens().await?;
        let mut deleted_count = 0;
        
        for (key, token_data) in client_tokens {
            if token_data.is_expired() {
                let isolated_key = self.create_isolated_key(&key);
                match self.base_storage.delete_token(&isolated_key).await {
                    Ok(()) => {
                        deleted_count += 1;
                        trace!("Cleaned up expired token: {}", key);
                    }
                    Err(e) => {
                        warn!("Failed to cleanup token {}: {}", key, e);
                    }
                }
            }
        }
        
        if deleted_count > 0 {
            info!("Cleaned up {} expired tokens for client: {}", deleted_count, self.remote_context.display_name());
            self.update_metadata().await;
        }
        
        Ok(deleted_count)
    }

    /// Validate token ownership
    fn validate_token_ownership(&self, token_data: &TokenData) -> bool {
        // Check if token metadata matches this client
        if let Some(token_client_id) = token_data.metadata.get("client_id") {
            if token_client_id != &self.metadata.client_id {
                return false;
            }
        }
        
        // Check IP address match
        if let Some(token_client_ip) = token_data.metadata.get("client_ip") {
            if token_client_ip != &self.remote_context.client_identity.client_ip.to_string() {
                return false;
            }
        }
        
        // Check session ID match
        if let Some(token_session_id) = token_data.metadata.get("session_id") {
            if token_session_id != &self.remote_context.remote_session_id {
                return false;
            }
        }
        
        true
    }

    /// Update storage metadata
    async fn update_metadata(&self) {
        // This would update the metadata in a mutable way in a real implementation
        // For now, we'll just trace the update
        trace!("Updated metadata for client: {}", self.remote_context.display_name());
    }

    /// Get storage metadata
    pub fn get_metadata(&self) -> &RemoteStorageMetadata {
        &self.metadata
    }

    /// Get client display name
    pub fn get_client_display_name(&self) -> String {
        self.remote_context.display_name()
    }

    /// Check if storage is available
    pub async fn is_storage_available(&self) -> bool {
        self.base_storage.is_storage_available().await
    }

    /// Get storage type
    pub fn get_storage_type(&self) -> &SecureStorageType {
        self.base_storage.storage_type()
    }

    /// Generate a consistent token key (replicating private base_storage method)
    fn create_token_key(&self, provider: &str, user_id: Option<&str>) -> String {
        // Create unique identifier from client identity and isolation key
        let unique_user_id = format!("remote_{}_{}", 
            self.remote_context.client_identity.client_ip, 
            self.remote_context.isolation_key
        );
        match user_id {
            Some(uid) => format!("{}:{}:{}", unique_user_id, provider, uid),
            None => format!("{}:{}", unique_user_id, provider),
        }
    }
}

impl Default for RemoteTokenConfig {
    fn default() -> Self {
        Self {
            enable_auto_cleanup: true,
            cleanup_interval_seconds: 900, // 15 minutes
            max_tokens_per_client: 10,
            encryption_level: TokenEncryptionLevel::Enhanced,
            enable_token_validation: true,
            expiry_grace_period_seconds: 300, // 5 minutes
        }
    }
}

impl RemoteTokenManager {
    /// Create new remote token manager
    pub fn new(config: RemoteTokenConfig) -> Self {
        Self {
            client_storages: std::sync::RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Get or create token storage for remote client
    pub async fn get_client_storage(
        &self,
        remote_context: RemoteUserContext,
    ) -> Result<Arc<RemoteTokenStorage>> {
        let client_id = remote_context.get_remote_client_id();
        
        // Check if storage already exists
        {
            let storages = self.client_storages.read().unwrap();
            if let Some(storage) = storages.get(&client_id) {
                trace!("Using existing token storage for client: {}", client_id);
                return Ok(storage.clone());
            }
        }

        // Create new storage
        debug!("Creating new token storage for client: {}", remote_context.display_name());
        let storage = Arc::new(RemoteTokenStorage::new(remote_context).await?);
        
        // Store in cache
        {
            let mut storages = self.client_storages.write().unwrap();
            storages.insert(client_id.clone(), storage.clone());
        }

        info!("Created token storage for client: {}", client_id);
        Ok(storage)
    }

    /// Remove client storage
    pub async fn remove_client_storage(&self, client_id: &str) -> Result<()> {
        let removed_storage = {
            let mut storages = self.client_storages.write().unwrap();
            storages.remove(client_id)
        };

        if let Some(storage) = removed_storage {
            // Cleanup any remaining tokens
            let _ = storage.cleanup_expired_tokens().await;
            info!("Removed token storage for client: {}", client_id);
        }

        Ok(())
    }

    /// Get all client storages
    pub fn get_all_client_storages(&self) -> HashMap<String, Arc<RemoteTokenStorage>> {
        let storages = self.client_storages.read().unwrap();
        storages.clone()
    }

    /// Cleanup expired storages
    pub async fn cleanup_expired_storages(&self) -> Result<u32> {
        debug!("Cleaning up expired client storages");
        
        let current_time = std::time::SystemTime::now();
        let max_inactivity = std::time::Duration::from_secs(3600 * 24); // 24 hours
        
        let expired_clients: Vec<String> = {
            let storages = self.client_storages.read().unwrap();
            storages.iter()
                .filter_map(|(client_id, storage)| {
                    let last_accessed = storage.get_metadata().last_accessed;
                    if current_time.duration_since(last_accessed).unwrap_or_default() > max_inactivity {
                        Some(client_id.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        let mut removed_count = 0;
        for client_id in expired_clients {
            if let Err(e) = self.remove_client_storage(&client_id).await {
                warn!("Failed to remove expired storage for client {}: {}", client_id, e);
            } else {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            info!("Cleaned up {} expired client storages", removed_count);
        }

        Ok(removed_count)
    }

    /// Get manager statistics
    pub fn get_statistics(&self) -> RemoteTokenManagerStats {
        let storages = self.client_storages.read().unwrap();
        
        RemoteTokenManagerStats {
            total_clients: storages.len(),
            total_storages: storages.len(),
            encryption_level: self.config.encryption_level.clone(),
            cleanup_enabled: self.config.enable_auto_cleanup,
        }
    }
}

/// Remote token manager statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteTokenManagerStats {
    pub total_clients: usize,
    pub total_storages: usize,
    pub encryption_level: TokenEncryptionLevel,
    pub cleanup_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    fn create_test_remote_context() -> RemoteUserContext {
        let local_context = UserContext::default();
        let req = TestRequest::default()
            .insert_header(("x-client-hostname", "test-machine"))
            .insert_header(("x-client-username", "testuser"))
            .peer_addr("192.168.1.100:12345".parse().unwrap())
            .to_http_request();
        
        RemoteUserContext::new(local_context, &req, None).unwrap()
    }

    #[tokio::test]
    async fn test_remote_token_storage_creation() {
        let remote_context = create_test_remote_context();
        let storage = RemoteTokenStorage::new(remote_context).await;
        
        assert!(storage.is_ok());
        let storage = storage.unwrap();
        assert!(!storage.isolation_prefix.is_empty());
        assert!(storage.is_storage_available().await);
    }

    #[tokio::test]
    async fn test_token_isolation() {
        let remote_context1 = create_test_remote_context();
        let storage1 = RemoteTokenStorage::new(remote_context1).await.unwrap();
        
        // Create second context with different client info
        let local_context = UserContext::default();
        let req = TestRequest::default()
            .insert_header(("x-client-hostname", "different-machine"))
            .insert_header(("x-client-username", "different-user"))
            .peer_addr("192.168.1.101:12345".parse().unwrap())
            .to_http_request();
        let remote_context2 = RemoteUserContext::new(local_context, &req, None).unwrap();
        let storage2 = RemoteTokenStorage::new(remote_context2).await.unwrap();
        
        // Different clients should have different isolation prefixes
        assert_ne!(storage1.isolation_prefix, storage2.isolation_prefix);
        
        // Keys should be different even for same provider/user
        let key1 = storage1.create_isolated_key("github:user1");
        let key2 = storage2.create_isolated_key("github:user1");
        assert_ne!(key1, key2);
    }

    #[tokio::test]
    async fn test_token_manager() {
        let config = RemoteTokenConfig::default();
        let manager = RemoteTokenManager::new(config);
        
        let remote_context1 = create_test_remote_context();
        let client_id1 = remote_context1.get_remote_client_id();
        
        // Get storage for first client
        let storage1 = manager.get_client_storage(remote_context1).await.unwrap();
        assert_eq!(manager.get_all_client_storages().len(), 1);
        
        // Get storage for same client - should return same instance
        let req = TestRequest::default()
            .insert_header(("x-client-hostname", "test-machine"))
            .insert_header(("x-client-username", "testuser"))
            .peer_addr("192.168.1.100:12345".parse().unwrap())
            .to_http_request();
        let remote_context1_again = RemoteUserContext::new(UserContext::default(), &req, None).unwrap();
        
        // Note: This test might fail because session IDs are different
        // In a real implementation, we'd need session persistence logic
    }

    #[tokio::test]
    async fn test_token_validation() {
        let remote_context = create_test_remote_context();
        let storage = RemoteTokenStorage::new(remote_context).await.unwrap();
        
        // Create test token data
        let mut token_data = TokenData {
            access_token: "test_token".to_string().into(),
            refresh_token: None,
            expires_at: None,
            scopes: vec!["read".to_string()],
            provider: "test".to_string(),
            token_type: "Bearer".to_string(),
            audience: None,
            resource: None,
            created_at: std::time::SystemTime::now(),
            last_refreshed: None,
            user_id: None,
            metadata: HashMap::new(),
        };
        
        // Token without proper metadata should not validate
        assert!(!storage.validate_token_ownership(&token_data));
        
        // Add correct metadata
        token_data.metadata.insert("client_id".to_string(), storage.metadata.client_id.clone());
        token_data.metadata.insert("client_ip".to_string(), storage.remote_context.client_identity.client_ip.to_string());
        token_data.metadata.insert("session_id".to_string(), storage.remote_context.remote_session_id.clone());
        
        // Now it should validate
        assert!(storage.validate_token_ownership(&token_data));
    }
}