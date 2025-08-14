//! MCP Roots service implementation
//!
//! Handles roots requests for filesystem and URI boundary discovery according to MCP 2025-06-18 specification

use crate::config::Config;
use crate::error::Result;
use crate::mcp::types::roots::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, error, warn};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

/// Cached discovery results
#[derive(Debug, Clone)]
struct CachedRoots {
    /// Discovered roots
    roots: Vec<Root>,
    /// Cache timestamp
    cached_at: DateTime<Utc>,
    /// Cache expiry time
    expires_at: DateTime<Utc>,
}

/// MCP Roots service
pub struct RootsService {
    /// Service configuration
    config: RootsConfig,
    /// Cached discovery results
    cache: Arc<RwLock<Option<CachedRoots>>>,
    /// Manual root overrides
    manual_roots: Arc<RwLock<HashMap<String, Root>>>,
}

impl RootsService {
    /// Create a new roots service
    pub fn new(config: RootsConfig) -> Result<Self> {
        Ok(Self {
            config,
            cache: Arc::new(RwLock::new(None)),
            manual_roots: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create roots service from main config
    pub fn from_config(config: &Config) -> Result<Self> {
        let roots_config = RootsConfig {
            enabled: config.smart_discovery.as_ref()
                .map(|sd| sd.enabled)
                .unwrap_or(false),
            auto_discover_filesystem: true,
            predefined_roots: Self::create_predefined_roots(config),
            security: RootsSecurityConfig {
                enabled: true,
                blocked_patterns: vec![
                    r"^/etc/.*".to_string(),
                    r"^/root/.*".to_string(),
                    r"^/proc/.*".to_string(),
                    r"^/sys/.*".to_string(),
                    r"^/dev/.*".to_string(),
                    r".*/\.ssh/.*".to_string(),
                    r".*/\.aws/.*".to_string(),
                    r".*/\.env.*".to_string(),
                    r".*/(password|secret|key).*".to_string(),
                ],
                allowed_patterns: Some(vec![
                    r"^/home/.*".to_string(),
                    r"^/Users/.*".to_string(),
                    r"^/tmp/.*".to_string(),
                    r"^/var/tmp/.*".to_string(),
                    r"^\./.*".to_string(),
                    r"^[A-Za-z]:/.*".to_string(), // Windows drives
                ]),
                max_discovery_depth: 2,
                follow_symlinks: false,
                blocked_extensions: vec![
                    "key".to_string(),
                    "pem".to_string(),
                    "p12".to_string(),
                    "pfx".to_string(),
                    "crt".to_string(),
                    "cer".to_string(),
                ],
                max_roots_count: 50,
            },
            discovery: RootsDiscoveryConfig {
                scan_common_locations: true,
                custom_paths: vec![],
                supported_schemes: vec![
                    "file".to_string(),
                    "http".to_string(),
                    "https".to_string(),
                ],
                check_accessibility: true,
                cache_duration_seconds: 300, // 5 minutes
            },
        };

        Self::new(roots_config)
    }

    /// Create predefined roots based on configuration
    fn create_predefined_roots(_config: &Config) -> Vec<Root> {
        let mut roots = Vec::new();

        // Add current working directory
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(cwd_str) = cwd.to_str() {
                roots.push(
                    Root::filesystem("cwd", cwd_str)
                        .with_name("Current Directory")
                        .with_description("Current working directory")
                        .with_permissions(vec![
                            RootPermission::Read,
                            RootPermission::List,
                            RootPermission::Write,
                            RootPermission::Create,
                        ])
                        .with_tags(vec!["filesystem".to_string(), "current".to_string()])
                );
            }
        }

        // Add user home directory
        if let Some(home_dir) = dirs::home_dir() {
            if let Some(home_str) = home_dir.to_str() {
                roots.push(
                    Root::filesystem("home", home_str)
                        .with_name("Home Directory")
                        .with_description("User's home directory")
                        .with_permissions(vec![
                            RootPermission::Read,
                            RootPermission::List,
                            RootPermission::Write,
                            RootPermission::Create,
                        ])
                        .with_tags(vec!["filesystem".to_string(), "home".to_string()])
                );
            }
        }

        // Add temp directory
        let temp_dir = std::env::temp_dir();
        if let Some(temp_str) = temp_dir.to_str() {
            roots.push(
                Root::filesystem("temp", temp_str)
                    .with_name("Temporary Directory")
                    .with_description("System temporary directory")
                    .with_permissions(vec![
                        RootPermission::Read,
                        RootPermission::List,
                        RootPermission::Write,
                        RootPermission::Create,
                        RootPermission::Delete,
                    ])
                    .with_tags(vec!["filesystem".to_string(), "temp".to_string()])
            );
        }

        roots
    }

    /// Handle roots list request
    pub async fn handle_roots_list_request(
        &self,
        request: RootsListRequest,
    ) -> std::result::Result<RootsListResponse, RootsError> {
        if !self.config.enabled {
            return Err(RootsError {
                code: RootsErrorCode::InvalidRequest,
                message: "Roots capability is not enabled".to_string(),
                details: None,
            });
        }

        debug!("Processing roots list request: {:?}", request);

        // Get all available roots
        let mut all_roots = self.get_all_roots().await?;

        // Apply filters if provided
        if let Some(ref filter) = request.filter {
            all_roots.retain(|root| root.matches_filter(filter));
        }

        // Apply pagination
        let (roots, next_cursor) = self.apply_pagination(all_roots, &request);

        let response = RootsListResponse {
            roots: roots.clone(),
            next_cursor,
            total_count: Some(roots.len() as u32),
        };

        info!("Returning {} roots", response.roots.len());
        Ok(response)
    }

    /// Get all available roots (cached or discovered)
    async fn get_all_roots(&self) -> std::result::Result<Vec<Root>, RootsError> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(ref cached) = *cache {
                if Utc::now() < cached.expires_at {
                    debug!("Returning cached roots (count: {})", cached.roots.len());
                    return Ok(cached.roots.clone());
                }
            }
        }

        // Cache miss or expired, discover roots
        let mut discovered_roots = Vec::new();

        // Add predefined roots
        discovered_roots.extend(self.config.predefined_roots.clone());

        // Add manual roots
        {
            let manual_roots = self.manual_roots.read().await;
            discovered_roots.extend(manual_roots.values().cloned());
        }

        // Auto-discover filesystem roots if enabled
        if self.config.auto_discover_filesystem {
            match self.discover_filesystem_roots().await {
                Ok(mut fs_roots) => {
                    discovered_roots.append(&mut fs_roots);
                }
                Err(e) => {
                    warn!("Failed to discover filesystem roots: {}", e.message);
                }
            }
        }

        // Apply security filtering
        let filtered_roots = self.apply_security_filtering(discovered_roots).await?;

        // Limit the number of roots
        let final_roots = if filtered_roots.len() > self.config.security.max_roots_count {
            warn!("Truncating roots from {} to {}", filtered_roots.len(), self.config.security.max_roots_count);
            filtered_roots.into_iter().take(self.config.security.max_roots_count).collect()
        } else {
            filtered_roots
        };

        // Update cache
        let cache_duration = chrono::Duration::seconds(self.config.discovery.cache_duration_seconds as i64);
        let cached = CachedRoots {
            roots: final_roots.clone(),
            cached_at: Utc::now(),
            expires_at: Utc::now() + cache_duration,
        };

        {
            let mut cache = self.cache.write().await;
            *cache = Some(cached);
        }

        info!("Discovered and cached {} roots", final_roots.len());
        Ok(final_roots)
    }

    /// Discover filesystem roots automatically
    async fn discover_filesystem_roots(&self) -> std::result::Result<Vec<Root>, RootsError> {
        let mut roots = Vec::new();

        if self.config.discovery.scan_common_locations {
            // Scan common directories
            let common_paths = self.get_common_filesystem_paths();
            
            for path in common_paths {
                if let Ok(metadata) = tokio::fs::metadata(&path).await {
                    if metadata.is_dir() {
                        if let Some(path_str) = path.to_str() {
                            let root = Root::filesystem(
                                format!("auto_{}", path.file_name().unwrap_or_default().to_string_lossy()),
                                path_str.to_string()
                            )
                            .with_name(format!("Auto-discovered: {}", path.display()))
                            .with_description("Automatically discovered filesystem location")
                            .with_accessibility(self.check_path_accessibility(&path).await)
                            .with_permissions(self.determine_path_permissions(&path).await)
                            .with_tags(vec!["filesystem".to_string(), "auto-discovered".to_string()]);

                            roots.push(root);
                        }
                    }
                }
            }
        }

        // Scan custom paths
        for custom_path in &self.config.discovery.custom_paths {
            let path = PathBuf::from(custom_path);
            if let Ok(metadata) = tokio::fs::metadata(&path).await {
                if metadata.is_dir() {
                    let root = Root::filesystem(
                        format!("custom_{}", path.file_name().unwrap_or_default().to_string_lossy()),
                        custom_path.clone()
                    )
                    .with_name(format!("Custom: {}", path.display()))
                    .with_description("Custom configured path")
                    .with_accessibility(self.check_path_accessibility(&path).await)
                    .with_permissions(self.determine_path_permissions(&path).await)
                    .with_tags(vec!["filesystem".to_string(), "custom".to_string()]);

                    roots.push(root);
                }
            }
        }

        Ok(roots)
    }

    /// Get common filesystem paths to scan
    fn get_common_filesystem_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Cross-platform common directories
        if let Some(home_dir) = dirs::home_dir() {
            paths.push(home_dir.join("Documents"));
            paths.push(home_dir.join("Downloads"));
            paths.push(home_dir.join("Desktop"));
            
            // Add common development directories
            paths.push(home_dir.join("Projects"));
            paths.push(home_dir.join("Development"));
            paths.push(home_dir.join("Code"));
            paths.push(home_dir.join("src"));
        }

        // Add system-specific paths
        #[cfg(unix)]
        {
            paths.push(PathBuf::from("/usr/local"));
            paths.push(PathBuf::from("/opt"));
            paths.push(PathBuf::from("/var/www"));
        }

        #[cfg(windows)]
        {
            paths.push(PathBuf::from("C:\\Program Files"));
            paths.push(PathBuf::from("C:\\Users\\Public"));
        }

        paths
    }

    /// Check if a path is accessible
    async fn check_path_accessibility(&self, path: &Path) -> bool {
        if !self.config.discovery.check_accessibility {
            return true; // Assume accessible if not checking
        }

        // Try to read the directory
        match tokio::fs::read_dir(path).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Determine permissions for a path
    async fn determine_path_permissions(&self, path: &Path) -> Vec<RootPermission> {
        let mut permissions = vec![RootPermission::List];

        // Check read permission
        if tokio::fs::read_dir(path).await.is_ok() {
            permissions.push(RootPermission::Read);
        }

        // Check write permission by attempting to create a temporary file
        let test_file = path.join(".magictunnel_write_test");
        if tokio::fs::write(&test_file, b"test").await.is_ok() {
            permissions.push(RootPermission::Write);
            permissions.push(RootPermission::Create);
            
            // Clean up test file
            let _ = tokio::fs::remove_file(&test_file).await;
        }

        permissions
    }

    /// Apply security filtering to discovered roots
    async fn apply_security_filtering(&self, roots: Vec<Root>) -> std::result::Result<Vec<Root>, RootsError> {
        if !self.config.security.enabled {
            return Ok(roots);
        }

        let mut filtered_roots = Vec::new();

        for root in roots {
            // Check against blocked patterns
            let mut blocked = false;
            for pattern in &self.config.security.blocked_patterns {
                if let Ok(regex) = regex::Regex::new(pattern) {
                    if regex.is_match(&root.uri) {
                        debug!("Root {} blocked by pattern: {}", root.id, pattern);
                        blocked = true;
                        break;
                    }
                }
            }

            if blocked {
                continue;
            }

            // Check against allowed patterns (if specified)
            if let Some(ref allowed_patterns) = self.config.security.allowed_patterns {
                let mut allowed = false;
                for pattern in allowed_patterns {
                    if let Ok(regex) = regex::Regex::new(pattern) {
                        if regex.is_match(&root.uri) {
                            allowed = true;
                            break;
                        }
                    }
                }

                if !allowed {
                    debug!("Root {} not in allowed patterns", root.id);
                    continue;
                }
            }

            // Check file extensions for filesystem roots
            if root.root_type == RootType::Filesystem {
                if let Some(extension) = Path::new(&root.uri).extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if self.config.security.blocked_extensions.contains(&ext_str.to_lowercase()) {
                            debug!("Root {} blocked by extension: {}", root.id, ext_str);
                            continue;
                        }
                    }
                }
            }

            filtered_roots.push(root);
        }

        Ok(filtered_roots)
    }

    /// Apply pagination to results
    fn apply_pagination(&self, mut roots: Vec<Root>, request: &RootsListRequest) -> (Vec<Root>, Option<String>) {
        // Handle cursor-based pagination
        if let Some(ref cursor) = request.cursor {
            // Find the starting position based on cursor
            if let Some(start_idx) = roots.iter().position(|r| r.id == *cursor) {
                roots = roots.into_iter().skip(start_idx + 1).collect();
            }
        }

        // Apply limit
        let limit = request.limit.unwrap_or(self.config.security.max_roots_count as u32) as usize;
        let next_cursor = if roots.len() > limit {
            // Get the last item's ID as the next cursor
            roots.get(limit - 1).map(|r| r.id.clone())
        } else {
            None
        };

        if roots.len() > limit {
            roots.truncate(limit);
        }

        (roots, next_cursor)
    }

    /// Add a manual root
    pub async fn add_manual_root(&self, root: Root) -> std::result::Result<(), RootsError> {
        // Validate the root
        if let Err(e) = self.validate_root(&root).await {
            return Err(e);
        }

        // Save the root ID before moving
        let root_id = root.id.clone();

        // Add to manual roots
        {
            let mut manual_roots = self.manual_roots.write().await;
            manual_roots.insert(root_id.clone(), root);
        }

        // Invalidate cache
        self.invalidate_cache().await;

        info!("Added manual root: {}", root_id);
        Ok(())
    }

    /// Remove a manual root
    pub async fn remove_manual_root(&self, root_id: &str) -> std::result::Result<(), RootsError> {
        {
            let mut manual_roots = self.manual_roots.write().await;
            if manual_roots.remove(root_id).is_none() {
                return Err(RootsError {
                    code: RootsErrorCode::NotFound,
                    message: format!("Manual root '{}' not found", root_id),
                    details: None,
                });
            }
        }

        // Invalidate cache
        self.invalidate_cache().await;

        info!("Removed manual root: {}", root_id);
        Ok(())
    }

    /// Validate a root
    async fn validate_root(&self, root: &Root) -> std::result::Result<(), RootsError> {
        // Check ID is not empty
        if root.id.trim().is_empty() {
            return Err(RootsError {
                code: RootsErrorCode::InvalidRequest,
                message: "Root ID cannot be empty".to_string(),
                details: None,
            });
        }

        // Check URI is valid
        if root.uri.trim().is_empty() {
            return Err(RootsError {
                code: RootsErrorCode::InvalidRequest,
                message: "Root URI cannot be empty".to_string(),
                details: None,
            });
        }

        // Validate scheme is supported
        if let Some(scheme) = root.get_scheme() {
            if !self.config.discovery.supported_schemes.contains(&scheme) {
                return Err(RootsError {
                    code: RootsErrorCode::SecurityViolation,
                    message: format!("Unsupported URI scheme: {}", scheme),
                    details: None,
                });
            }
        }

        Ok(())
    }

    /// Invalidate the cache
    async fn invalidate_cache(&self) {
        let mut cache = self.cache.write().await;
        *cache = None;
        debug!("Roots cache invalidated");
    }

    /// Get service status
    pub async fn get_status(&self) -> Value {
        let cache_info = {
            let cache = self.cache.read().await;
            if let Some(ref cached) = *cache {
                json!({
                    "cached": true,
                    "cached_at": cached.cached_at,
                    "expires_at": cached.expires_at,
                    "cached_count": cached.roots.len()
                })
            } else {
                json!({
                    "cached": false
                })
            }
        };

        let manual_count = self.manual_roots.read().await.len();

        json!({
            "enabled": self.config.enabled,
            "auto_discover_filesystem": self.config.auto_discover_filesystem,
            "predefined_roots": self.config.predefined_roots.len(),
            "manual_roots": manual_count,
            "cache": cache_info,
            "security": {
                "enabled": self.config.security.enabled,
                "max_roots_count": self.config.security.max_roots_count,
                "blocked_patterns": self.config.security.blocked_patterns.len(),
                "allowed_patterns": self.config.security.allowed_patterns.as_ref().map(|p| p.len())
            }
        })
    }

    /// Trigger root discovery manually
    pub async fn trigger_discovery(&self) -> std::result::Result<Value, RootsError> {
        if !self.config.enabled {
            return Err(RootsError {
                code: RootsErrorCode::InvalidRequest,
                message: "Roots capability is not enabled".to_string(),
                details: None,
            });
        }

        info!("Triggering manual root discovery");
        
        // Clear the cache to force rediscovery
        self.invalidate_cache().await;
        
        // Trigger discovery by calling get_all_roots
        let start_time = std::time::Instant::now();
        let roots = self.get_all_roots().await?;
        let discovery_duration = start_time.elapsed();
        
        let result = json!({
            "success": true,
            "discovered_count": roots.len(),
            "discovery_duration_ms": discovery_duration.as_millis(),
            "timestamp": Utc::now()
        });
        
        info!("Manual discovery completed: {} roots found in {}ms", 
              roots.len(), discovery_duration.as_millis());
        
        Ok(result)
    }

    /// Get detailed service status for monitoring
    pub async fn get_service_status(&self) -> Value {
        let cache_info = {
            let cache = self.cache.read().await;
            if let Some(ref cached) = *cache {
                let cache_age_seconds = (Utc::now() - cached.cached_at).num_seconds();
                let cache_remaining_seconds = (cached.expires_at - Utc::now()).num_seconds().max(0);
                
                json!({
                    "status": "active",
                    "cached_at": cached.cached_at,
                    "expires_at": cached.expires_at,
                    "cached_count": cached.roots.len(),
                    "cache_age_seconds": cache_age_seconds,
                    "cache_remaining_seconds": cache_remaining_seconds
                })
            } else {
                json!({
                    "status": "inactive",
                    "cached_count": 0
                })
            }
        };

        let manual_count = self.manual_roots.read().await.len();
        
        // Calculate accessibility metrics
        let roots = match self.get_all_roots().await {
            Ok(roots) => roots,
            Err(_) => vec![], // On error, return empty vec for stats
        };
        
        let total_roots = roots.len();
        let accessible_roots = roots.iter().filter(|r| r.accessible).count();
        
        json!({
            "healthy": self.config.enabled,
            "total_roots": total_roots,
            "accessible_roots": accessible_roots,
            "manual_roots": manual_count,
            "predefined_roots": self.config.predefined_roots.len(),
            "cache_status": cache_info["status"].as_str().unwrap_or("unknown"),
            "last_discovery": cache_info.get("cached_at"),
            "discovery_duration_ms": null, // Will be populated during actual discovery
            "config": {
                "enabled": self.config.enabled,
                "auto_discover_filesystem": self.config.auto_discover_filesystem,
                "security_enabled": self.config.security.enabled,
                "max_roots_count": self.config.security.max_roots_count,
                "cache_duration_seconds": self.config.discovery.cache_duration_seconds
            }
        })
    }

    /// Update service configuration
    pub async fn update_config(&self, updates: Value) -> std::result::Result<Value, RootsError> {
        // Note: This is a read-only implementation since config is immutable in the current design
        // In a production system, you'd want to implement config persistence
        
        warn!("Config update requested but not implemented: {:?}", updates);
        
        Err(RootsError {
            code: RootsErrorCode::InvalidRequest,
            message: "Configuration updates are not supported in the current implementation".to_string(),
            details: Some([
                ("requested_updates".to_string(), updates),
                ("note".to_string(), json!("Configuration is currently read-only from the main config file"))
            ].into_iter().collect()),
        })
    }

    /// Get configuration as JSON
    pub fn get_config(&self) -> Value {
        json!({
            "enabled": self.config.enabled,
            "auto_discover_filesystem": self.config.auto_discover_filesystem,
            "predefined_roots_count": self.config.predefined_roots.len(),
            "security": {
                "enabled": self.config.security.enabled,
                "blocked_patterns_count": self.config.security.blocked_patterns.len(),
                "allowed_patterns_count": self.config.security.allowed_patterns.as_ref().map(|p| p.len()),
                "max_discovery_depth": self.config.security.max_discovery_depth,
                "follow_symlinks": self.config.security.follow_symlinks,
                "blocked_extensions_count": self.config.security.blocked_extensions.len(),
                "max_roots_count": self.config.security.max_roots_count
            },
            "discovery": {
                "scan_common_locations": self.config.discovery.scan_common_locations,
                "custom_paths_count": self.config.discovery.custom_paths.len(),
                "supported_schemes": self.config.discovery.supported_schemes,
                "check_accessibility": self.config.discovery.check_accessibility,
                "cache_duration_seconds": self.config.discovery.cache_duration_seconds
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_roots_service_creation() {
        let config = RootsConfig::default();
        let service = RootsService::new(config).unwrap();
        
        let status = service.get_status().await;
        assert_eq!(status["enabled"], false);
    }

    #[tokio::test]
    async fn test_manual_root_management() {
        let mut config = RootsConfig::default();
        config.enabled = true;
        let service = RootsService::new(config).unwrap();

        let root = Root::filesystem("test", "/tmp/test")
            .with_name("Test Root");

        // Add manual root
        let result = service.add_manual_root(root.clone()).await;
        assert!(result.is_ok());

        // Remove manual root
        let result = service.remove_manual_root("test").await;
        assert!(result.is_ok());

        // Try to remove non-existent root
        let result = service.remove_manual_root("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_security_filtering() {
        let mut config = RootsConfig::default();
        config.enabled = true;
        let service = RootsService::new(config).unwrap();

        let roots = vec![
            Root::filesystem("safe", "/home/user/documents"),
            Root::filesystem("unsafe", "/etc/passwd"),
        ];

        let filtered = service.apply_security_filtering(roots).await.unwrap();
        
        // Should filter out the /etc/ path
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "safe");
    }

    #[tokio::test]
    async fn test_roots_list_request() {
        let mut config = RootsConfig::default();
        config.enabled = true;
        let service = RootsService::new(config).unwrap();

        let request = RootsListRequest::new()
            .with_limit(10)
            .with_filter(RootFilter::new().accessible_only());

        let result = service.handle_roots_list_request(request).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(!response.roots.is_empty());
    }
}