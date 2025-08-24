//! High-Performance Permission-Aware Tool Cache System
//!
//! This module implements ultra-fast permission-based tool filtering for smart discovery.
//! Target performance: <1ms for 100k tools, <100μs for 10k tools per user.

use crate::security::SecurityContext;
use crate::security::rbac::{RbacService, PermissionContext, PermissionResult};
use ahash::{AHashMap, AHashSet};
use arc_swap::ArcSwap;
use chrono::Utc;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use serde::{Deserialize, Serialize};

/// Unique identifier for a tool
pub type ToolId = String;

/// Unique identifier for a role
pub type RoleId = String;

/// Unique identifier for a user
pub type UserId = String;

/// Per-user cached tool permissions for ultra-fast lookups
#[derive(Debug, Clone)]
pub struct UserToolCache {
    /// Hash of user ID for fast cache key generation
    pub user_id_hash: u64,
    
    /// User ID (for debugging and audit)
    pub user_id: UserId,
    
    /// Set of tools this user is allowed to access
    pub allowed_tools: AHashSet<ToolId>,
    
    /// Bitmap representing user's permissions (up to 64 permissions)
    pub permissions_bitmap: u64,
    
    /// User's roles for pattern-based evaluation
    pub user_roles: Vec<RoleId>,
    
    /// When this cache entry was created
    pub cache_timestamp: Instant,
    
    /// Time-to-live for this cache entry
    pub ttl: Duration,
    
    /// Statistics for performance monitoring
    pub stats: UserCacheStats,
}

/// Statistics for user cache performance monitoring
#[derive(Debug, Clone, Default)]
pub struct UserCacheStats {
    /// Number of cache hits
    pub hits: u64,
    
    /// Number of cache misses
    pub misses: u64,
    
    /// Total number of tool lookups
    pub lookups: u64,
    
    /// Last access time
    pub last_access: Option<Instant>,
    
    /// Number of cache refreshes
    pub refreshes: u64,
}

impl UserToolCache {
    /// Create a new user tool cache
    pub fn new(user_id: UserId, allowed_tools: AHashSet<ToolId>, permissions_bitmap: u64, user_roles: Vec<RoleId>) -> Self {
        let user_id_hash = Self::hash_user_id(&user_id);
        
        Self {
            user_id_hash,
            user_id,
            allowed_tools,
            permissions_bitmap,
            user_roles,
            cache_timestamp: Instant::now(),
            ttl: Duration::from_secs(300), // 5 minute default TTL
            stats: UserCacheStats::default(),
        }
    }
    
    /// Generate consistent hash for user ID
    fn hash_user_id(user_id: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = ahash::AHasher::default();
        user_id.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Check if a tool is allowed for this user (O(1) operation)
    pub fn is_tool_allowed(&self, tool_id: &ToolId) -> bool {
        self.allowed_tools.contains(tool_id)
    }
    
    /// Check if a tool is allowed for this user with stats tracking
    pub fn is_tool_allowed_with_stats(&mut self, tool_id: &ToolId) -> bool {
        self.stats.lookups += 1;
        self.stats.last_access = Some(Instant::now());
        
        let is_allowed = self.allowed_tools.contains(tool_id);
        
        if is_allowed {
            self.stats.hits += 1;
        } else {
            self.stats.misses += 1;
        }
        
        is_allowed
    }
    
    /// Check if this cache entry is expired
    pub fn is_expired(&self) -> bool {
        self.cache_timestamp.elapsed() > self.ttl
    }
    
    /// Get cache hit ratio for performance monitoring
    pub fn hit_ratio(&self) -> f64 {
        if self.stats.lookups == 0 {
            0.0
        } else {
            self.stats.hits as f64 / self.stats.lookups as f64
        }
    }
    
    /// Refresh the cache timestamp (extend TTL)
    pub fn refresh(&mut self) {
        self.cache_timestamp = Instant::now();
        self.stats.refreshes += 1;
    }
}

/// Global index mapping permissions and roles to tools
#[derive(Debug, Clone)]
pub struct PermissionIndex {
    /// Map from role ID to set of allowed tools
    pub role_to_tools: AHashMap<RoleId, AHashSet<ToolId>>,
    
    /// Map from tool ID to required permissions bitmap
    pub tool_to_permissions: AHashMap<ToolId, u64>,
    
    /// Map from tool ID to required roles
    pub tool_to_roles: AHashMap<ToolId, Vec<RoleId>>,
    
    /// Reverse index: permission bit to tools requiring it
    pub permission_to_tools: AHashMap<u8, AHashSet<ToolId>>,
    
    /// Statistics for performance monitoring
    pub stats: PermissionIndexStats,
}

/// Statistics for permission index performance
#[derive(Debug, Clone, Default)]
pub struct PermissionIndexStats {
    /// Number of index lookups
    pub lookups: u64,
    
    /// Number of index updates
    pub updates: u64,
    
    /// Last update time
    pub last_update: Option<Instant>,
    
    /// Number of tools indexed
    pub tools_count: usize,
    
    /// Number of roles indexed
    pub roles_count: usize,
}

impl PermissionIndex {
    /// Create a new empty permission index
    pub fn new() -> Self {
        Self {
            role_to_tools: AHashMap::new(),
            tool_to_permissions: AHashMap::new(),
            tool_to_roles: AHashMap::new(),
            permission_to_tools: AHashMap::new(),
            stats: PermissionIndexStats::default(),
        }
    }
    
    /// Add a tool with its required permissions and roles
    pub fn add_tool(&mut self, tool_id: ToolId, required_permissions: u64, required_roles: Vec<RoleId>) {
        // Update tool-to-permissions mapping
        self.tool_to_permissions.insert(tool_id.clone(), required_permissions);
        
        // Update tool-to-roles mapping
        self.tool_to_roles.insert(tool_id.clone(), required_roles.clone());
        
        // Update role-to-tools mapping
        for role in &required_roles {
            self.role_to_tools
                .entry(role.clone())
                .or_insert_with(AHashSet::new)
                .insert(tool_id.clone());
        }
        
        // Update permission-to-tools reverse index
        for bit_position in 0..64 {
            if (required_permissions & (1 << bit_position)) != 0 {
                self.permission_to_tools
                    .entry(bit_position)
                    .or_insert_with(AHashSet::new)
                    .insert(tool_id.clone());
            }
        }
        
        self.stats.updates += 1;
        self.stats.last_update = Some(Instant::now());
        self.update_counts();
    }
    
    /// Remove a tool from the index
    pub fn remove_tool(&mut self, tool_id: &ToolId) {
        // Remove from tool-to-permissions
        if let Some(permissions) = self.tool_to_permissions.remove(tool_id) {
            // Remove from permission-to-tools reverse index
            for bit_position in 0..64 {
                if (permissions & (1 << bit_position)) != 0 {
                    if let Some(tools) = self.permission_to_tools.get_mut(&bit_position) {
                        tools.remove(tool_id);
                        if tools.is_empty() {
                            self.permission_to_tools.remove(&bit_position);
                        }
                    }
                }
            }
        }
        
        // Remove from tool-to-roles and role-to-tools
        if let Some(roles) = self.tool_to_roles.remove(tool_id) {
            for role in roles {
                if let Some(tools) = self.role_to_tools.get_mut(&role) {
                    tools.remove(tool_id);
                    if tools.is_empty() {
                        self.role_to_tools.remove(&role);
                    }
                }
            }
        }
        
        self.stats.updates += 1;
        self.stats.last_update = Some(Instant::now());
        self.update_counts();
    }
    
    /// Get all tools accessible by a user with given permissions and roles
    pub fn get_user_tools(&mut self, permissions_bitmap: u64, user_roles: &[RoleId]) -> AHashSet<ToolId> {
        self.stats.lookups += 1;
        
        let mut allowed_tools = AHashSet::new();
        
        // Add tools based on role membership
        for role in user_roles {
            if let Some(role_tools) = self.role_to_tools.get(role) {
                allowed_tools.extend(role_tools.iter().cloned());
            }
        }
        
        // Add tools based on permission bitmap
        for bit_position in 0..64 {
            if (permissions_bitmap & (1 << bit_position)) != 0 {
                if let Some(permission_tools) = self.permission_to_tools.get(&bit_position) {
                    allowed_tools.extend(permission_tools.iter().cloned());
                }
            }
        }
        
        allowed_tools
    }
    
    /// Update internal statistics
    fn update_counts(&mut self) {
        self.stats.tools_count = self.tool_to_permissions.len();
        self.stats.roles_count = self.role_to_tools.len();
    }
}

impl Default for PermissionIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance cache manager for user tool permissions
pub struct PermissionCacheManager {
    /// Per-user tool caches
    user_caches: Arc<RwLock<AHashMap<u64, UserToolCache>>>,
    
    /// Global permission index
    permission_index: Arc<ArcSwap<PermissionIndex>>,
    
    /// RBAC service for permission checking
    rbac_service: Arc<RbacService>,
    
    /// Configuration
    config: PermissionCacheConfig,
    
    /// Overall statistics
    stats: PermissionCacheStats,
}

/// Configuration for permission cache manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCacheConfig {
    /// Maximum number of user caches to keep in memory
    pub max_user_caches: usize,
    
    /// Default TTL for user caches
    pub default_user_cache_ttl: Duration,
    
    /// How often to clean up expired caches
    pub cleanup_interval: Duration,
    
    /// Whether to enable performance monitoring
    pub enable_stats: bool,
}

impl Default for PermissionCacheConfig {
    fn default() -> Self {
        Self {
            max_user_caches: 10_000,
            default_user_cache_ttl: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(60), // 1 minute
            enable_stats: true,
        }
    }
}

/// Statistics for permission cache manager
#[derive(Debug, Clone, Default)]
pub struct PermissionCacheStats {
    /// Total cache hits across all users
    pub total_hits: u64,
    
    /// Total cache misses across all users
    pub total_misses: u64,
    
    /// Number of cache evictions
    pub evictions: u64,
    
    /// Number of cache cleanups performed
    pub cleanups: u64,
    
    /// Current number of cached users
    pub cached_users: usize,
    
    /// Memory usage estimate in bytes
    pub estimated_memory_bytes: usize,
}

impl PermissionCacheManager {
    /// Create a new permission cache manager with RBAC service
    pub fn new(config: PermissionCacheConfig, rbac_service: Arc<RbacService>) -> Self {
        Self {
            user_caches: Arc::new(RwLock::new(AHashMap::new())),
            permission_index: Arc::new(ArcSwap::new(Arc::new(PermissionIndex::new()))),
            rbac_service,
            config,
            stats: PermissionCacheStats::default(),
        }
    }
    
    /// Create a new permission cache manager with default/mock RBAC service for testing
    /// This is a temporary compatibility method - use `new` with proper RBAC service in production
    pub fn new_with_default_rbac(config: PermissionCacheConfig) -> Self {
        use crate::security::rbac::RbacConfig;
        
        // Create a minimal RBAC config for compatibility
        let rbac_config = RbacConfig {
            enabled: false, // Disabled for compatibility
            roles: std::collections::HashMap::new(),
            user_roles: std::collections::HashMap::new(),
            api_key_roles: std::collections::HashMap::new(),
            default_roles: Vec::new(),
            inherit_permissions: false,
        };
        
        // Create mock RBAC service (note: in production this should be a real service)
        let rbac_service = Arc::new(RbacService::new(rbac_config).expect("Failed to create mock RBAC service"));
        
        Self::new(config, rbac_service)
    }
    
    /// Get or create user tool cache
    pub async fn get_user_cache(&self, security_context: &SecurityContext) -> Option<UserToolCache> {
        let user_id = security_context.user.as_ref()?.id.clone()?;
        let user_id_hash = UserToolCache::hash_user_id(&user_id);
        
        // Try to get from cache first
        {
            let caches = self.user_caches.read().unwrap();
            if let Some(cache) = caches.get(&user_id_hash) {
                if !cache.is_expired() {
                    debug!("Cache hit for user: {}", user_id);
                    return Some(cache.clone());
                }
            }
        }
        
        // Cache miss or expired - rebuild cache
        debug!("Cache miss for user: {}, rebuilding...", user_id);
        self.rebuild_user_cache(security_context).await
    }
    
    /// Rebuild user cache from current permissions
    pub async fn rebuild_user_cache(&self, security_context: &SecurityContext) -> Option<UserToolCache> {
        let user = security_context.user.as_ref()?;
        let user_id = user.id.clone()?;
        
        // Get user's permissions and roles
        let permissions_bitmap = self.calculate_user_permissions_bitmap(security_context);
        let user_roles = user.roles.clone();
        
        // Get all available tools from permission index to check against
        let permission_index = self.permission_index.load_full();
        let all_tools: Vec<ToolId> = permission_index.tool_to_permissions.keys().cloned().collect();
        
        // Use RBAC service to determine which tools are actually allowed
        let allowed_tools = self.bulk_check_tool_permissions(security_context, &all_tools);
        
        // Create new cache entry
        let mut user_cache = UserToolCache::new(user_id.clone(), allowed_tools, permissions_bitmap, user_roles);
        user_cache.ttl = self.config.default_user_cache_ttl;
        
        // Store in cache
        let user_id_hash = user_cache.user_id_hash;
        {
            let mut caches = self.user_caches.write().unwrap();
            
            // Ensure we don't exceed cache limit
            if caches.len() >= self.config.max_user_caches {
                self.evict_oldest_cache(&mut caches);
            }
            
            caches.insert(user_id_hash, user_cache.clone());
        }
        
        info!("Rebuilt cache for user: {} with {} allowed tools", user_id, user_cache.allowed_tools.len());
        Some(user_cache)
    }
    
    /// Calculate user permissions bitmap from security context using RBAC service
    fn calculate_user_permissions_bitmap(&self, security_context: &SecurityContext) -> u64 {
        let user = match security_context.user.as_ref() {
            Some(user) => user,
            None => return 0, // Anonymous user has no permissions
        };
        
        // Get user roles from RBAC service
        let user_roles = if let Some(user_id) = &user.id {
            self.rbac_service.get_user_roles(user_id)
        } else {
            Vec::new()
        };
        
        // Also check API key roles if present
        let api_key_roles = if let Some(api_key_name) = security_context.user.as_ref().and_then(|u| u.api_key_name.as_ref()) {
            self.rbac_service.get_api_key_roles(api_key_name)
        } else {
            Vec::new()
        };
        
        // Combine all roles
        let mut all_roles = user_roles;
        all_roles.extend(api_key_roles);
        
        let mut bitmap = 0u64;
        
        // Convert role names to bit positions for fast bitmap operations
        // This is a simplified approach - in production you might want a more sophisticated mapping
        for (index, _role) in all_roles.iter().enumerate() {
            if index < 64 {
                bitmap |= 1 << index;
                debug!("Added role '{}' to permission bitmap at position {}", _role, index);
            }
        }
        
        // Special handling for admin roles - grant all permissions
        if all_roles.iter().any(|role| role.contains("admin") || role.contains("superuser")) {
            bitmap = u64::MAX; // All permissions granted
            debug!("Admin role detected, granting all permissions");
        }
        
        debug!("Calculated permission bitmap {:016x} for user {:?} with roles: {:?}", 
               bitmap, user.id, all_roles);
        
        bitmap
    }
    
    /// Evict the oldest cache entry to make room
    fn evict_oldest_cache(&self, caches: &mut AHashMap<u64, UserToolCache>) {
        if let Some((&oldest_key, _)) = caches
            .iter()
            .min_by_key(|(_, cache)| cache.cache_timestamp)
        {
            caches.remove(&oldest_key);
            // Note: In a real implementation, you'd update stats here
        }
    }
    
    /// Update the global permission index
    pub fn update_permission_index(&self, new_index: PermissionIndex) {
        self.permission_index.store(Arc::new(new_index));
        
        // Invalidate all user caches since permissions may have changed
        let mut caches = self.user_caches.write().unwrap();
        caches.clear();
        
        info!("Permission index updated, invalidated {} user caches", caches.len());
    }
    
    /// Check if a user can access a specific tool
    pub async fn is_tool_allowed(&self, security_context: &SecurityContext, tool_id: &ToolId) -> bool {
        // First try to use the cache for fast lookup
        if let Some(mut user_cache) = self.get_user_cache(security_context).await {
            return user_cache.is_tool_allowed(tool_id);
        }
        
        // Fallback to direct RBAC check if cache is not available
        debug!("Cache not available, performing direct RBAC check for tool: {}", tool_id);
        
        // Create permission context for RBAC check
        let permission_context = PermissionContext {
            user_id: security_context.user.as_ref().and_then(|u| u.id.clone()),
            user_roles: if let Some(user) = &security_context.user {
                if let Some(user_id) = &user.id {
                    self.rbac_service.get_user_roles(user_id)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            },
            api_key_name: security_context.user.as_ref().and_then(|u| u.api_key_name.clone()),
            resource: Some(format!("tool:{}", tool_id)),
            action: Some("execute".to_string()),
            client_ip: security_context.request.client_ip.clone(),
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        };
        
        // Check if user has execute permission for this specific tool
        let result = self.rbac_service.check_permission("tool:execute", &permission_context);
        
        debug!("Direct RBAC check result for tool {}: granted={}", tool_id, result.granted);
        
        result.granted
    }
    
    /// Bulk check permissions for multiple tools using RBAC service
    /// This is used to populate the cache efficiently
    pub fn bulk_check_tool_permissions(&self, security_context: &SecurityContext, tool_ids: &[ToolId]) -> AHashSet<ToolId> {
        let mut allowed_tools = AHashSet::new();
        
        // Create base permission context
        let base_permission_context = PermissionContext {
            user_id: security_context.user.as_ref().and_then(|u| u.id.clone()),
            user_roles: if let Some(user) = &security_context.user {
                if let Some(user_id) = &user.id {
                    self.rbac_service.get_user_roles(user_id)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            },
            api_key_name: security_context.user.as_ref().and_then(|u| u.api_key_name.clone()),
            resource: None, // Will be set per tool
            action: Some("execute".to_string()),
            client_ip: security_context.request.client_ip.clone(),
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        };
        
        // Check permission for each tool
        for tool_id in tool_ids {
            let mut permission_context = base_permission_context.clone();
            permission_context.resource = Some(format!("tool:{}", tool_id));
            
            let result = self.rbac_service.check_permission("tool:execute", &permission_context);
            
            if result.granted {
                allowed_tools.insert(tool_id.clone());
                debug!("Tool {} allowed for user {:?} via RBAC", tool_id, permission_context.user_id);
            } else {
                debug!("Tool {} denied for user {:?} via RBAC", tool_id, permission_context.user_id);
            }
        }
        
        debug!("Bulk permission check: {}/{} tools allowed for user {:?}", 
               allowed_tools.len(), tool_ids.len(), base_permission_context.user_id);
        
        allowed_tools
    }
    
    /// Get all allowed tools for a user
    pub async fn get_user_allowed_tools(&self, security_context: &SecurityContext) -> AHashSet<ToolId> {
        if let Some(user_cache) = self.get_user_cache(security_context).await {
            user_cache.allowed_tools.clone()
        } else {
            AHashSet::new()
        }
    }
    
    /// Clean up expired caches
    pub fn cleanup_expired_caches(&mut self) {
        let mut caches = self.user_caches.write().unwrap();
        let initial_count = caches.len();
        
        caches.retain(|_, cache| !cache.is_expired());
        
        let removed_count = initial_count - caches.len();
        if removed_count > 0 {
            debug!("Cleaned up {} expired user caches", removed_count);
        }
        
        self.stats.cleanups += 1;
        self.stats.cached_users = caches.len();
    }
    
    /// Invalidate cache for a specific user
    pub async fn invalidate_user_cache(&self, user_id: &UserId) {
        let user_id_hash = UserToolCache::hash_user_id(user_id);
        let mut caches = self.user_caches.write().unwrap();
        
        if caches.remove(&user_id_hash).is_some() {
            debug!("Invalidated cache for user: {}", user_id);
        }
    }
    
    /// Invalidate all user caches
    pub async fn invalidate_all_user_caches(&self) {
        let mut caches = self.user_caches.write().unwrap();
        let count = caches.len();
        caches.clear();
        
        if count > 0 {
            info!("Invalidated all {} user caches", count);
        }
    }
    
    /// Invalidate caches for users with a specific role
    pub async fn invalidate_caches_by_role(&self, role_id: &str) {
        let mut caches = self.user_caches.write().unwrap();
        let initial_count = caches.len();
        
        caches.retain(|_, cache| {
            !cache.user_roles.iter().any(|role| role == role_id)
        });
        
        let removed_count = initial_count - caches.len();
        if removed_count > 0 {
            info!("Invalidated {} user caches for role: {}", removed_count, role_id);
        }
    }
    
    /// Emergency clear all caches
    pub async fn emergency_clear_all_caches(&self) {
        let mut caches = self.user_caches.write().unwrap();
        let count = caches.len();
        caches.clear();
        
        // Also clear the permission index to force complete rebuild
        self.permission_index.store(Arc::new(PermissionIndex::new()));
        
        warn!("Emergency cache clear completed - {} caches and permission index cleared", count);
    }
    
    /// Cleanup expired caches and return list of expired user IDs (async version)
    pub async fn cleanup_expired_caches_async(&self) -> Vec<UserId> {
        let mut caches = self.user_caches.write().unwrap();
        let mut expired_users = Vec::new();
        
        // Collect expired user IDs before removing them
        for (_, cache) in caches.iter() {
            if cache.is_expired() {
                expired_users.push(cache.user_id.clone());
            }
        }
        
        // Remove expired caches
        caches.retain(|_, cache| !cache.is_expired());
        
        if !expired_users.is_empty() {
            debug!("Cleaned up {} expired user caches", expired_users.len());
        }
        
        expired_users
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> PermissionCacheStats {
        let caches = self.user_caches.read().unwrap();
        
        let mut stats = self.stats.clone();
        stats.cached_users = caches.len();
        
        // Calculate total hits/misses from all user caches
        stats.total_hits = caches.values().map(|cache| cache.stats.hits).sum();
        stats.total_misses = caches.values().map(|cache| cache.stats.misses).sum();
        
        // Estimate memory usage (rough calculation)
        stats.estimated_memory_bytes = caches.len() * std::mem::size_of::<UserToolCache>()
            + caches.values()
                .map(|cache| cache.allowed_tools.len() * std::mem::size_of::<ToolId>())
                .sum::<usize>();
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_tool_cache_creation() {
        let user_id = "test_user".to_string();
        let mut allowed_tools = AHashSet::new();
        allowed_tools.insert("tool1".to_string());
        allowed_tools.insert("tool2".to_string());
        
        let cache = UserToolCache::new(user_id.clone(), allowed_tools, 0b1010, vec!["admin".to_string()]);
        
        assert_eq!(cache.user_id, user_id);
        assert_eq!(cache.permissions_bitmap, 0b1010);
        assert_eq!(cache.allowed_tools.len(), 2);
        assert!(cache.is_tool_allowed(&"tool1".to_string()));
        assert!(!cache.is_tool_allowed(&"tool3".to_string()));
    }
    
    #[test]
    fn test_permission_index() {
        let mut index = PermissionIndex::new();
        
        // Add tool with permissions and roles
        index.add_tool(
            "test_tool".to_string(),
            0b1010, // Permissions: bits 1 and 3
            vec!["admin".to_string(), "user".to_string()]
        );
        
        // Test role-based lookup
        let admin_tools = index.get_user_tools(0, &["admin".to_string()]);
        assert!(admin_tools.contains("test_tool"));
        
        // Test permission-based lookup
        let permission_tools = index.get_user_tools(0b1010, &[]);
        assert!(permission_tools.contains("test_tool"));
        
        // Test combined lookup
        let combined_tools = index.get_user_tools(0b1000, &["admin".to_string()]);
        assert!(combined_tools.contains("test_tool"));
    }
}