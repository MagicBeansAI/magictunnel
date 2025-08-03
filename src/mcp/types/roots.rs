//! MCP Roots types and structures
//! 
//! Implements the roots capability for MCP 2025-06-18 specification
//! Roots allow servers to discover filesystem and URI boundaries that clients can access

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request to list available roots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsListRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Maximum number of roots to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Filter by root type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<RootFilter>,
}

/// Filter criteria for listing roots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootFilter {
    /// Filter by root types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<RootType>>,
    /// Filter by URI schemes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemes: Option<Vec<String>>,
    /// Include only accessible roots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessible_only: Option<bool>,
}

/// Response containing list of available roots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsListResponse {
    /// List of available roots
    pub roots: Vec<Root>,
    /// Optional cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Total number of roots available (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u32>,
}

/// A root represents a filesystem or URI boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Root {
    /// Unique identifier for this root
    pub id: String,
    /// Root type (filesystem, uri, etc.)
    #[serde(rename = "type")]
    pub root_type: RootType,
    /// URI or path representing this root
    pub uri: String,
    /// Human-readable name for this root
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Description of what this root provides access to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this root is currently accessible
    pub accessible: bool,
    /// Permissions available for this root
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<RootPermission>>,
    /// Additional metadata for this root
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Tags for categorizing roots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Types of roots
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootType {
    /// Local filesystem root
    Filesystem,
    /// URI-based root (HTTP, HTTPS, etc.)
    Uri,
    /// Database connection root
    Database,
    /// API endpoint root
    Api,
    /// Cloud storage root
    CloudStorage,
    /// Container or virtual filesystem
    Container,
    /// Network share or remote filesystem
    NetworkShare,
    /// Custom root type
    Custom(String),
}

/// Permissions available for a root
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootPermission {
    /// Read access to the root
    Read,
    /// Write access to the root
    Write,
    /// Execute/traverse access
    Execute,
    /// Delete access
    Delete,
    /// Create new items
    Create,
    /// List contents
    List,
    /// Modify metadata
    Modify,
    /// Full administrative access
    Admin,
}

/// Configuration for the roots service
#[derive(Debug, Clone)]
pub struct RootsConfig {
    /// Whether roots capability is enabled
    pub enabled: bool,
    /// Automatically discover filesystem roots
    pub auto_discover_filesystem: bool,
    /// Predefined roots to always include
    pub predefined_roots: Vec<Root>,
    /// Security restrictions
    pub security: RootsSecurityConfig,
    /// Discovery configuration
    pub discovery: RootsDiscoveryConfig,
}

/// Security configuration for roots
#[derive(Debug, Clone)]
pub struct RootsSecurityConfig {
    /// Whether to enable security checks
    pub enabled: bool,
    /// Blocked paths/URIs (regex patterns)
    pub blocked_patterns: Vec<String>,
    /// Allowed path/URI patterns only
    pub allowed_patterns: Option<Vec<String>>,
    /// Maximum depth for filesystem discovery
    pub max_discovery_depth: usize,
    /// Whether to follow symlinks during discovery
    pub follow_symlinks: bool,
    /// Blocked file extensions
    pub blocked_extensions: Vec<String>,
    /// Maximum number of roots to return
    pub max_roots_count: usize,
}

/// Discovery configuration for roots
#[derive(Debug, Clone)]
pub struct RootsDiscoveryConfig {
    /// Whether to scan common filesystem locations
    pub scan_common_locations: bool,
    /// Custom discovery paths
    pub custom_paths: Vec<String>,
    /// URI schemes to support
    pub supported_schemes: Vec<String>,
    /// Whether to check accessibility during discovery
    pub check_accessibility: bool,
    /// Cache discovery results for this duration (seconds)
    pub cache_duration_seconds: u64,
}

/// Error in roots operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsError {
    /// Error code
    pub code: RootsErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Roots error codes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RootsErrorCode {
    /// Invalid request parameters
    InvalidRequest,
    /// Access denied to requested root
    AccessDenied,
    /// Root not found
    NotFound,
    /// Discovery failed
    DiscoveryFailed,
    /// Security policy violation
    SecurityViolation,
    /// Internal server error
    InternalError,
}

impl Root {
    /// Create a new filesystem root
    pub fn filesystem<S: Into<String>>(id: S, path: S) -> Self {
        Self {
            id: id.into(),
            root_type: RootType::Filesystem,
            uri: format!("file://{}", path.into()),
            name: None,
            description: None,
            accessible: true,
            permissions: Some(vec![RootPermission::Read, RootPermission::List]),
            metadata: None,
            tags: None,
        }
    }

    /// Create a new URI root
    pub fn uri<S: Into<String>>(id: S, uri: S) -> Self {
        Self {
            id: id.into(),
            root_type: RootType::Uri,
            uri: uri.into(),
            name: None,
            description: None,
            accessible: true,
            permissions: Some(vec![RootPermission::Read]),
            metadata: None,
            tags: None,
        }
    }

    /// Create a new API root
    pub fn api<S: Into<String>>(id: S, base_url: S) -> Self {
        Self {
            id: id.into(),
            root_type: RootType::Api,
            uri: base_url.into(),
            name: None,
            description: None,
            accessible: true,
            permissions: Some(vec![RootPermission::Read, RootPermission::Execute]),
            metadata: None,
            tags: None,
        }
    }

    /// Set the name of this root
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description of this root
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the permissions for this root
    pub fn with_permissions(mut self, permissions: Vec<RootPermission>) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Set accessibility status
    pub fn with_accessibility(mut self, accessible: bool) -> Self {
        self.accessible = accessible;
        self
    }

    /// Add metadata to this root
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add tags to this root
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Check if this root has a specific permission
    pub fn has_permission(&self, permission: &RootPermission) -> bool {
        self.permissions.as_ref()
            .map(|perms| perms.contains(permission))
            .unwrap_or(false)
    }

    /// Get the scheme from the URI
    pub fn get_scheme(&self) -> Option<String> {
        if let Some(colon_pos) = self.uri.find(':') {
            Some(self.uri[..colon_pos].to_string())
        } else {
            None
        }
    }

    /// Check if this root matches a filter
    pub fn matches_filter(&self, filter: &RootFilter) -> bool {
        // Check type filter
        if let Some(ref types) = filter.types {
            if !types.contains(&self.root_type) {
                return false;
            }
        }

        // Check scheme filter
        if let Some(ref schemes) = filter.schemes {
            if let Some(scheme) = self.get_scheme() {
                if !schemes.contains(&scheme) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check accessibility filter
        if let Some(accessible_only) = filter.accessible_only {
            if accessible_only && !self.accessible {
                return false;
            }
        }

        true
    }
}

impl RootsListRequest {
    /// Create a new roots list request
    pub fn new() -> Self {
        Self {
            cursor: None,
            limit: None,
            filter: None,
        }
    }

    /// Set cursor for pagination
    pub fn with_cursor<S: Into<String>>(mut self, cursor: S) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Set limit for number of results
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set filter criteria
    pub fn with_filter(mut self, filter: RootFilter) -> Self {
        self.filter = Some(filter);
        self
    }
}

impl RootFilter {
    /// Create a new root filter
    pub fn new() -> Self {
        Self {
            types: None,
            schemes: None,
            accessible_only: None,
        }
    }

    /// Filter by root types
    pub fn with_types(mut self, types: Vec<RootType>) -> Self {
        self.types = Some(types);
        self
    }

    /// Filter by URI schemes
    pub fn with_schemes(mut self, schemes: Vec<String>) -> Self {
        self.schemes = Some(schemes);
        self
    }

    /// Include only accessible roots
    pub fn accessible_only(mut self) -> Self {
        self.accessible_only = Some(true);
        self
    }
}

impl Default for RootsListRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RootFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RootsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_discover_filesystem: true,
            predefined_roots: vec![],
            security: RootsSecurityConfig {
                enabled: true,
                blocked_patterns: vec![
                    r"^/etc/.*".to_string(),
                    r"^/root/.*".to_string(),
                    r"^/proc/.*".to_string(),
                    r"^/sys/.*".to_string(),
                    r".*/\.ssh/.*".to_string(),
                    r".*/\.aws/.*".to_string(),
                ],
                allowed_patterns: None,
                max_discovery_depth: 3,
                follow_symlinks: false,
                blocked_extensions: vec![
                    "key".to_string(),
                    "pem".to_string(),
                    "p12".to_string(),
                    "pfx".to_string(),
                ],
                max_roots_count: 100,
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
        }
    }
}

impl std::fmt::Display for RootsErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RootsErrorCode::InvalidRequest => write!(f, "invalid_request"),
            RootsErrorCode::AccessDenied => write!(f, "access_denied"),
            RootsErrorCode::NotFound => write!(f, "not_found"),
            RootsErrorCode::DiscoveryFailed => write!(f, "discovery_failed"),
            RootsErrorCode::SecurityViolation => write!(f, "security_violation"),
            RootsErrorCode::InternalError => write!(f, "internal_error"),
        }
    }
}

impl std::fmt::Display for RootType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RootType::Filesystem => write!(f, "filesystem"),
            RootType::Uri => write!(f, "uri"),
            RootType::Database => write!(f, "database"),
            RootType::Api => write!(f, "api"),
            RootType::CloudStorage => write!(f, "cloud_storage"),
            RootType::Container => write!(f, "container"),
            RootType::NetworkShare => write!(f, "network_share"),
            RootType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

impl std::error::Error for RootsError {}

impl std::fmt::Display for RootsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_root_creation() {
        let root = Root::filesystem("home", "/home/user")
            .with_name("User Home")
            .with_description("User's home directory")
            .with_permissions(vec![RootPermission::Read, RootPermission::Write]);

        assert_eq!(root.id, "home");
        assert_eq!(root.root_type, RootType::Filesystem);
        assert_eq!(root.uri, "file:///home/user");
        assert_eq!(root.name, Some("User Home".to_string()));
        assert!(root.has_permission(&RootPermission::Read));
        assert!(root.has_permission(&RootPermission::Write));
        assert!(!root.has_permission(&RootPermission::Delete));
    }

    #[test]
    fn test_root_filter() {
        let root = Root::filesystem("test", "/test");
        
        let filter = RootFilter::new()
            .with_types(vec![RootType::Filesystem])
            .accessible_only();

        assert!(root.matches_filter(&filter));

        let uri_filter = RootFilter::new()
            .with_schemes(vec!["http".to_string()]);

        assert!(!root.matches_filter(&uri_filter));
    }

    #[test]
    fn test_roots_request() {
        let request = RootsListRequest::new()
            .with_limit(10)
            .with_filter(RootFilter::new().accessible_only());

        assert_eq!(request.limit, Some(10));
        assert!(request.filter.is_some());
    }

    #[test]
    fn test_serialization() {
        let root = Root::api("test_api", "https://api.example.com")
            .with_name("Test API");

        let json_str = serde_json::to_string(&root).unwrap();
        let deserialized: Root = serde_json::from_str(&json_str).unwrap();

        assert_eq!(root.id, deserialized.id);
        assert_eq!(root.root_type, deserialized.root_type);
        assert_eq!(root.uri, deserialized.uri);
    }

    #[test]
    fn test_uri_scheme_extraction() {
        let root = Root::uri("test", "https://example.com/path");
        assert_eq!(root.get_scheme(), Some("https".to_string()));

        let file_root = Root::filesystem("test", "/path");
        assert_eq!(file_root.get_scheme(), Some("file".to_string()));
    }
}