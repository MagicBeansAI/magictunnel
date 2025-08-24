//! Role-Based Access Control (RBAC) for MagicTunnel
//!
//! Provides comprehensive role and permission management,
//! similar to MCP Manager's RBAC capabilities.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs;
use chrono::{DateTime, Utc, Timelike};
use tracing::{info, warn, error, debug};
use super::statistics::{SecurityServiceStatistics, HealthMonitor, ServiceHealth, HealthStatus, RbacStatistics, RoleUsage, PerformanceMetrics};

/// RBAC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacConfig {
    /// Whether RBAC is enabled
    pub enabled: bool,
    /// Roles configuration
    pub roles: HashMap<String, Role>,
    /// User role assignments
    pub user_roles: HashMap<String, Vec<String>>,
    /// API key role assignments
    pub api_key_roles: HashMap<String, Vec<String>>,
    /// Default roles for new users
    pub default_roles: Vec<String>,
    /// Whether to inherit parent role permissions
    pub inherit_permissions: bool,
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name
    pub name: String,
    /// Role description
    pub description: Option<String>,
    /// Permissions granted by this role
    pub permissions: Vec<String>,
    /// Parent roles (for inheritance)
    pub parent_roles: Vec<String>,
    /// Whether this role is active
    pub active: bool,
    /// Role metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// When this role was created
    pub created_at: Option<DateTime<Utc>>,
    /// When this role was last modified
    pub modified_at: Option<DateTime<Utc>>,
}

/// Permission definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    /// Permission name
    pub name: String,
    /// Permission description
    pub description: Option<String>,
    /// Resource types this permission applies to
    pub resource_types: Vec<String>,
    /// Actions allowed by this permission
    pub actions: Vec<String>,
    /// Conditions for this permission
    pub conditions: Option<Vec<PermissionCondition>>,
}

/// Condition for permission evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PermissionCondition {
    /// Time-based condition
    TimeRange {
        /// Start time (hour of day, 0-23)
        start_hour: u8,
        /// End time (hour of day, 0-23)
        end_hour: u8,
        /// Days of week (0=Sunday, 6=Saturday)
        days_of_week: Option<Vec<u8>>,
    },
    /// IP address condition
    IpAddress {
        /// Allowed IP ranges (CIDR notation)
        allowed_ranges: Vec<String>,
    },
    /// Resource pattern condition
    ResourcePattern {
        /// Regex pattern for resource matching
        pattern: String,
        /// Whether pattern is case sensitive
        case_sensitive: bool,
    },
    /// Custom condition (evaluated by external service)
    Custom {
        /// Condition name
        name: String,
        /// Parameters for condition evaluation
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// Context for permission evaluation
#[derive(Debug, Clone)]
pub struct PermissionContext {
    /// User ID
    pub user_id: Option<String>,
    /// User roles
    pub user_roles: Vec<String>,
    /// API key name
    pub api_key_name: Option<String>,
    /// Resource being accessed
    pub resource: Option<String>,
    /// Action being performed
    pub action: Option<String>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// Current time
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Result of permission evaluation  
#[derive(Debug, Clone)]
pub struct PermissionResult {
    /// Whether permission is granted
    pub granted: bool,
    /// Roles that granted permission
    pub granting_roles: Vec<String>,
    /// Permissions that were checked
    pub permissions_checked: Vec<String>,
    /// Reason for the decision
    pub reason: String,
    /// Conditions that were evaluated
    pub conditions_evaluated: Vec<String>,
}

/// Statistics tracking for RBAC service
#[derive(Debug, Clone)]
struct RbacStats {
    /// Service start time
    start_time: DateTime<Utc>,
    /// Total authentication attempts
    total_auth_attempts: u64,
    /// Successful authentications
    successful_auth: u64,
    /// Failed authentications
    failed_auth: u64,
    /// Permission evaluation attempts
    permission_evaluations: u64,
    /// Currently active sessions (tracked separately)
    active_sessions: u32,
    /// Role usage tracking
    role_usage: HashMap<String, u64>,
    /// Last error message (if any)
    last_error: Option<String>,
    /// Performance tracking
    total_processing_time_ms: u64,
}

/// RBAC service for managing roles and permissions
pub struct RbacService {
    config: RbacConfig,
    /// Compiled permission conditions
    compiled_conditions: HashMap<String, CompiledCondition>,
    /// Statistics tracking
    stats: Arc<Mutex<RbacStats>>,
    /// File storage path for RBAC data
    storage_path: Option<PathBuf>,
    /// Permissions registry
    permissions_registry: Arc<Mutex<PermissionsRegistry>>,
    /// Mutable config for CRUD operations
    mutable_config: Arc<Mutex<RbacConfig>>,
}

/// Registry for managing dynamic permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsRegistry {
    /// Available permissions
    pub permissions: HashMap<String, Permission>,
    /// Permission categories
    pub categories: HashMap<String, PermissionCategory>,
    /// Last modified timestamp
    pub last_modified: DateTime<Utc>,
}

/// Permission category for organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCategory {
    /// Category ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Associated permissions
    pub permission_ids: Vec<String>,
}

/// Compiled condition for faster evaluation
#[derive(Debug)]
enum CompiledCondition {
    TimeRange {
        start_hour: u8,
        end_hour: u8,
        days_of_week: Option<HashSet<u8>>,
    },
    IpAddress {
        allowed_ranges: Vec<ipnetwork::IpNetwork>,
    },
    ResourcePattern {
        regex: regex::Regex,
    },
    Custom {
        name: String,
        parameters: HashMap<String, serde_json::Value>,
    },
}

impl Default for RbacConfig {
    fn default() -> Self {
        let mut roles = HashMap::new();
        
        // Default admin role
        roles.insert("admin".to_string(), Role {
            name: "admin".to_string(),
            description: Some("Full administrative access".to_string()),
            permissions: vec![
                "read".to_string(),
                "write".to_string(),
                "admin".to_string(),
                "tool:*".to_string(),
                "resource:*".to_string(),
                "prompt:*".to_string(),
            ],
            parent_roles: vec![],
            active: true,
            metadata: HashMap::new(),
            created_at: Some(Utc::now()),
            modified_at: Some(Utc::now()),
        });
        
        // Default user role
        roles.insert("user".to_string(), Role {
            name: "user".to_string(),
            description: Some("Standard user access".to_string()),
            permissions: vec![
                "read".to_string(),
                "tool:read".to_string(),
                "resource:read".to_string(),
                "prompt:read".to_string(),
            ],
            parent_roles: vec![],
            active: true,
            metadata: HashMap::new(),
            created_at: Some(Utc::now()),
            modified_at: Some(Utc::now()),
        });
        
        // Default operator role
        roles.insert("operator".to_string(), Role {
            name: "operator".to_string(),
            description: Some("Tool execution access".to_string()),
            permissions: vec![
                "read".to_string(),
                "write".to_string(),
                "tool:*".to_string(),
                "resource:read".to_string(),
                "prompt:read".to_string(),
            ],
            parent_roles: vec!["user".to_string()],
            active: true,
            metadata: HashMap::new(),
            created_at: Some(Utc::now()),
            modified_at: Some(Utc::now()),
        });
        
        Self {
            enabled: false,
            roles,
            user_roles: HashMap::new(),
            api_key_roles: HashMap::new(),
            default_roles: vec!["user".to_string()],
            inherit_permissions: true,
        }
    }
}

impl Default for PermissionsRegistry {
    fn default() -> Self {
        let mut permissions = HashMap::new();
        let mut categories = HashMap::new();
        
        // Basic permissions
        permissions.insert("read".to_string(), Permission {
            name: "read".to_string(),
            description: Some("Read access to resources".to_string()),
            resource_types: vec!["*".to_string()],
            actions: vec!["read".to_string()],
            conditions: None,
        });
        
        permissions.insert("write".to_string(), Permission {
            name: "write".to_string(),
            description: Some("Write access to resources".to_string()),
            resource_types: vec!["*".to_string()],
            actions: vec!["write", "create", "update"].iter().map(|s| s.to_string()).collect(),
            conditions: None,
        });
        
        permissions.insert("admin".to_string(), Permission {
            name: "admin".to_string(),
            description: Some("Administrative access".to_string()),
            resource_types: vec!["*".to_string()],
            actions: vec!["*".to_string()],
            conditions: None,
        });
        
        // Tool permissions
        permissions.insert("tool:read".to_string(), Permission {
            name: "tool:read".to_string(),
            description: Some("Read tool information and metadata".to_string()),
            resource_types: vec!["tool".to_string()],
            actions: vec!["read".to_string()],
            conditions: None,
        });
        
        permissions.insert("tool:execute".to_string(), Permission {
            name: "tool:execute".to_string(),
            description: Some("Execute tools".to_string()),
            resource_types: vec!["tool".to_string()],
            actions: vec!["execute".to_string()],
            conditions: None,
        });
        
        permissions.insert("tool:*".to_string(), Permission {
            name: "tool:*".to_string(),
            description: Some("Full access to all tools".to_string()),
            resource_types: vec!["tool".to_string()],
            actions: vec!["*".to_string()],
            conditions: None,
        });
        
        // Resource permissions  
        permissions.insert("resource:read".to_string(), Permission {
            name: "resource:read".to_string(),
            description: Some("Read resource content".to_string()),
            resource_types: vec!["resource".to_string()],
            actions: vec!["read".to_string()],
            conditions: None,
        });
        
        permissions.insert("resource:*".to_string(), Permission {
            name: "resource:*".to_string(),
            description: Some("Full access to all resources".to_string()),
            resource_types: vec!["resource".to_string()],
            actions: vec!["*".to_string()],
            conditions: None,
        });
        
        // Prompt permissions
        permissions.insert("prompt:read".to_string(), Permission {
            name: "prompt:read".to_string(),
            description: Some("Read prompt templates".to_string()),
            resource_types: vec!["prompt".to_string()],
            actions: vec!["read".to_string()],
            conditions: None,
        });
        
        permissions.insert("prompt:*".to_string(), Permission {
            name: "prompt:*".to_string(),
            description: Some("Full access to all prompts".to_string()),
            resource_types: vec!["prompt".to_string()],
            actions: vec!["*".to_string()],
            conditions: None,
        });
        
        // Categories
        categories.insert("basic".to_string(), PermissionCategory {
            id: "basic".to_string(),
            name: "Basic Permissions".to_string(),
            description: "Fundamental read/write permissions".to_string(),
            permission_ids: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        });
        
        categories.insert("tools".to_string(), PermissionCategory {
            id: "tools".to_string(),
            name: "Tool Permissions".to_string(),
            description: "Permissions for tool access and execution".to_string(),
            permission_ids: vec!["tool:read".to_string(), "tool:execute".to_string(), "tool:*".to_string()],
        });
        
        categories.insert("resources".to_string(), PermissionCategory {
            id: "resources".to_string(),
            name: "Resource Permissions".to_string(),
            description: "Permissions for accessing resources and files".to_string(),
            permission_ids: vec!["resource:read".to_string(), "resource:*".to_string()],
        });
        
        categories.insert("prompts".to_string(), PermissionCategory {
            id: "prompts".to_string(),
            name: "Prompt Permissions".to_string(),
            description: "Permissions for prompt template access".to_string(),
            permission_ids: vec!["prompt:read".to_string(), "prompt:*".to_string()],
        });
        
        Self {
            permissions,
            categories,
            last_modified: Utc::now(),
        }
    }
}

impl RbacService {
    /// Create a new RBAC service
    pub fn new(config: RbacConfig) -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_storage(config, None)
    }
    
    /// Create a new RBAC service with file storage
    pub fn with_storage(config: RbacConfig, storage_path: Option<PathBuf>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut compiled_conditions = HashMap::new();
        
        // Pre-compile conditions for performance
        for role in config.roles.values() {
            // For now, we don't have conditions directly on roles
            // but this could be extended
        }
        
        let stats = RbacStats {
            start_time: Utc::now(),
            total_auth_attempts: 0,
            successful_auth: 0,
            failed_auth: 0,
            permission_evaluations: 0,
            active_sessions: 0,
            role_usage: HashMap::new(),
            last_error: None,
            total_processing_time_ms: 0,
        };

        // Load or create permissions registry
        let permissions_registry = if let Some(ref path) = storage_path {
            Self::load_permissions_registry(path)
                .unwrap_or_else(|_| {
                    warn!("Failed to load permissions registry, using defaults");
                    PermissionsRegistry::default()
                })
        } else {
            PermissionsRegistry::default()
        };
        
        // Load existing config from file if storage path is provided
        let config = if let Some(ref path) = storage_path {
            Self::load_config_from_file(path, config.clone())
                .unwrap_or_else(|e| {
                    warn!("Failed to load RBAC config from file: {}, using provided config", e);
                    config
                })
        } else {
            config
        };

        Ok(Self {
            config: config.clone(),
            compiled_conditions,
            stats: Arc::new(Mutex::new(stats)),
            storage_path,
            permissions_registry: Arc::new(Mutex::new(permissions_registry)),
            mutable_config: Arc::new(Mutex::new(config)),
        })
    }
    
    /// Load RBAC configuration from file
    fn load_config_from_file(storage_path: &PathBuf, default_config: RbacConfig) -> Result<RbacConfig, Box<dyn std::error::Error>> {
        let config_path = storage_path.join("rbac_config.json");
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: RbacConfig = serde_json::from_str(&content)?;
            info!("Loaded RBAC configuration from {:?}", config_path);
            Ok(config)
        } else {
            // Save default config to file
            Self::save_config_to_file(storage_path, &default_config)?;
            Ok(default_config)
        }
    }
    
    /// Save RBAC configuration to file
    fn save_config_to_file(storage_path: &PathBuf, config: &RbacConfig) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(storage_path)?;
        
        let config_path = storage_path.join("rbac_config.json");
        let content = serde_json::to_string_pretty(config)?;
        fs::write(&config_path, content)?;
        
        info!("Saved RBAC configuration to {:?}", config_path);
        Ok(())
    }
    
    /// Load permissions registry from file
    fn load_permissions_registry(storage_path: &PathBuf) -> Result<PermissionsRegistry, Box<dyn std::error::Error>> {
        let registry_path = storage_path.join("permissions_registry.json");
        
        if registry_path.exists() {
            let content = fs::read_to_string(&registry_path)?;
            let registry: PermissionsRegistry = serde_json::from_str(&content)?;
            info!("Loaded permissions registry from {:?}", registry_path);
            Ok(registry)
        } else {
            let default_registry = PermissionsRegistry::default();
            Self::save_permissions_registry(storage_path, &default_registry)?;
            Ok(default_registry)
        }
    }
    
    /// Save permissions registry to file
    fn save_permissions_registry(storage_path: &PathBuf, registry: &PermissionsRegistry) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(storage_path)?;
        
        let registry_path = storage_path.join("permissions_registry.json");
        let content = serde_json::to_string_pretty(registry)?;
        fs::write(&registry_path, content)?;
        
        info!("Saved permissions registry to {:?}", registry_path);
        Ok(())
    }
    
    /// Save current state to files
    pub fn save_to_storage(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref path) = self.storage_path {
            // Save config
            if let Ok(config) = self.mutable_config.lock() {
                Self::save_config_to_file(path, &config)?;
            }
            
            // Save permissions registry
            if let Ok(registry) = self.permissions_registry.lock() {
                Self::save_permissions_registry(path, &registry)?;
            }
        }
        Ok(())
    }
    
    /// Check if user has permission
    pub fn check_permission(
        &self,
        permission: &str,
        context: &PermissionContext,
    ) -> PermissionResult {
        if !self.config.enabled {
            return PermissionResult {
                granted: true,
                granting_roles: vec![],
                permissions_checked: vec![permission.to_string()],
                reason: "RBAC disabled".to_string(),
                conditions_evaluated: vec![],
            };
        }
        
        let user_roles = self.get_effective_roles(context);
        let mut granting_roles = Vec::new();
        let mut conditions_evaluated = Vec::new();
        
        // Check each role for the permission
        for role_name in &user_roles {
            if let Some(role) = self.config.roles.get(role_name) {
                if !role.active {
                    continue;
                }
                
                let role_permissions = self.get_role_permissions(role_name);
                
                // Check if role has the permission
                if self.permission_matches(permission, &role_permissions) {
                    // Check any conditions
                    if self.check_role_conditions(role, context, &mut conditions_evaluated) {
                        granting_roles.push(role_name.clone());
                    }
                }
            }
        }
        
        let granted = !granting_roles.is_empty();
        let reason = if granted {
            format!("Permission granted by roles: {}", granting_roles.join(", "))
        } else {
            format!("No roles grant permission: {}", permission)
        };
        
        PermissionResult {
            granted,
            granting_roles,
            permissions_checked: vec![permission.to_string()],
            reason,
            conditions_evaluated,
        }
    }
    
    /// Get effective roles for a context (including inherited roles)
    fn get_effective_roles(&self, context: &PermissionContext) -> Vec<String> {
        let mut roles = HashSet::new();
        
        // Get user roles
        if let Some(user_id) = &context.user_id {
            if let Some(user_roles) = self.config.user_roles.get(user_id) {
                for role in user_roles {
                    roles.insert(role.clone());
                    if self.config.inherit_permissions {
                        self.add_parent_roles(role, &mut roles);
                    }
                }
            }
        }
        
        // Get API key roles
        if let Some(api_key) = &context.api_key_name {
            if let Some(api_key_roles) = self.config.api_key_roles.get(api_key) {
                for role in api_key_roles {
                    roles.insert(role.clone());
                    if self.config.inherit_permissions {
                        self.add_parent_roles(role, &mut roles);
                    }
                }
            }
        }
        
        // Add default roles if no other roles assigned
        if roles.is_empty() {
            for role in &self.config.default_roles {
                roles.insert(role.clone());
                if self.config.inherit_permissions {
                    self.add_parent_roles(role, &mut roles);
                }
            }
        }
        
        roles.into_iter().collect()
    }
    
    /// Add parent roles recursively
    fn add_parent_roles(&self, role_name: &str, roles: &mut HashSet<String>) {
        if let Some(role) = self.config.roles.get(role_name) {
            for parent in &role.parent_roles {
                if !roles.contains(parent) {
                    roles.insert(parent.clone());
                    self.add_parent_roles(parent, roles);
                }
            }
        }
    }
    
    /// Get all permissions for a role (including inherited)
    fn get_role_permissions(&self, role_name: &str) -> Vec<String> {
        let mut permissions = HashSet::new();
        self.collect_role_permissions(role_name, &mut permissions, &mut HashSet::new());
        permissions.into_iter().collect()
    }
    
    /// Collect permissions recursively
    fn collect_role_permissions(
        &self,
        role_name: &str,
        permissions: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(role_name) {
            return; // Prevent infinite recursion
        }
        visited.insert(role_name.to_string());
        
        if let Some(role) = self.config.roles.get(role_name) {
            // Add direct permissions
            for permission in &role.permissions {
                permissions.insert(permission.clone());
            }
            
            // Add parent permissions if inheritance is enabled
            if self.config.inherit_permissions {
                for parent in &role.parent_roles {
                    self.collect_role_permissions(parent, permissions, visited);
                }
            }
        }
    }
    
    /// Check if a permission matches any of the role permissions
    fn permission_matches(&self, requested: &str, role_permissions: &[String]) -> bool {
        for role_permission in role_permissions {
            if self.permission_pattern_matches(requested, role_permission) {
                return true;
            }
        }
        false
    }
    
    /// Check if permission matches a pattern (supports wildcards)
    fn permission_pattern_matches(&self, requested: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern == requested {
            return true;
        }
        
        // Handle wildcard patterns like "tool:*", "resource:read", etc.
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            return requested.starts_with(prefix);
        }
        
        false
    }
    
    /// Check role-specific conditions
    fn check_role_conditions(
        &self,
        _role: &Role,
        _context: &PermissionContext,
        _conditions_evaluated: &mut Vec<String>,
    ) -> bool {
        // For now, roles don't have conditions
        // This could be extended in the future
        true
    }
    
    /// Add a new role
    pub fn add_role(&mut self, role: Role) -> Result<(), String> {
        if self.config.roles.contains_key(&role.name) {
            return Err(format!("Role '{}' already exists", role.name));
        }
        
        // Validate parent roles exist
        for parent in &role.parent_roles {
            if !self.config.roles.contains_key(parent) {
                return Err(format!("Parent role '{}' does not exist", parent));
            }
        }
        
        self.config.roles.insert(role.name.clone(), role);
        
        // Auto-save to file storage if available
        if let Err(e) = self.save_to_storage() {
            warn!("Failed to save RBAC changes to storage: {}", e);
        }
        
        Ok(())
    }
    
    /// Update an existing role
    pub fn update_role(&mut self, role: Role) -> Result<(), String> {
        if !self.config.roles.contains_key(&role.name) {
            return Err(format!("Role '{}' does not exist", role.name));
        }
        
        // Validate parent roles exist
        for parent in &role.parent_roles {
            if !self.config.roles.contains_key(parent) {
                return Err(format!("Parent role '{}' does not exist", parent));
            }
        }
        
        self.config.roles.insert(role.name.clone(), role);
        
        // Auto-save to file storage if available
        if let Err(e) = self.save_to_storage() {
            warn!("Failed to save RBAC changes to storage: {}", e);
        }
        
        Ok(())
    }
    
    /// Remove a role
    pub fn remove_role(&mut self, role_name: &str) -> Result<(), String> {
        if !self.config.roles.contains_key(role_name) {
            return Err(format!("Role '{}' does not exist", role_name));
        }
        
        // Check if any other roles depend on this one
        for (name, role) in &self.config.roles {
            if role.parent_roles.contains(&role_name.to_string()) {
                return Err(format!("Cannot remove role '{}' - it is a parent of role '{}'", role_name, name));
            }
        }
        
        // Remove from user assignments
        for user_roles in self.config.user_roles.values_mut() {
            user_roles.retain(|r| r != role_name);
        }
        
        // Remove from API key assignments
        for api_key_roles in self.config.api_key_roles.values_mut() {
            api_key_roles.retain(|r| r != role_name);
        }
        
        // Remove from default roles
        self.config.default_roles.retain(|r| r != role_name);
        
        // Remove the role
        self.config.roles.remove(role_name);
        Ok(())
    }
    
    /// Assign role to user
    pub fn assign_user_role(&mut self, user_id: &str, role_name: &str) -> Result<(), String> {
        if !self.config.roles.contains_key(role_name) {
            return Err(format!("Role '{}' does not exist", role_name));
        }
        
        let user_roles = self.config.user_roles.entry(user_id.to_string()).or_default();
        if !user_roles.contains(&role_name.to_string()) {
            user_roles.push(role_name.to_string());
        }
        
        Ok(())
    }
    
    /// Remove role from user
    pub fn remove_user_role(&mut self, user_id: &str, role_name: &str) -> Result<(), String> {
        if let Some(user_roles) = self.config.user_roles.get_mut(user_id) {
            user_roles.retain(|r| r != role_name);
            if user_roles.is_empty() {
                self.config.user_roles.remove(user_id);
            }
        }
        Ok(())
    }
    
    /// Assign role to API key
    pub fn assign_api_key_role(&mut self, api_key: &str, role_name: &str) -> Result<(), String> {
        if !self.config.roles.contains_key(role_name) {
            return Err(format!("Role '{}' does not exist", role_name));
        }
        
        let api_key_roles = self.config.api_key_roles.entry(api_key.to_string()).or_default();
        if !api_key_roles.contains(&role_name.to_string()) {
            api_key_roles.push(role_name.to_string());
        }
        
        Ok(())
    }
    
    /// Remove role from API key
    pub fn remove_api_key_role(&mut self, api_key: &str, role_name: &str) -> Result<(), String> {
        if let Some(api_key_roles) = self.config.api_key_roles.get_mut(api_key) {
            api_key_roles.retain(|r| r != role_name);
            if api_key_roles.is_empty() {
                self.config.api_key_roles.remove(api_key);
            }
        }
        Ok(())
    }
    
    /// Get all roles
    pub fn get_roles(&self) -> &HashMap<String, Role> {
        &self.config.roles
    }
    
    /// Get roles formatted for API display
    pub fn get_roles_for_api(&self) -> serde_json::Value {
        use serde_json::json;
        
        let roles: Vec<serde_json::Value> = self.config.roles.iter().enumerate().map(|(index, (role_name, role))| {
            json!({
                "id": (index + 1).to_string(),
                "name": role_name,
                "enabled": role.active, // Role uses 'active' field, not 'enabled'
                "description": role.description.clone().unwrap_or_default(),
                "permissions": role.permissions,
                "created_at": Utc::now(),
                "updated_at": Utc::now(),
                "user_count": self.config.user_roles.values().filter(|roles| roles.contains(role_name)).count(),
                "priority": 50 // Role doesn't have priority field, use default
            })
        }).collect();
        
        json!(roles)
    }
    
    /// Get users for API display 
    pub fn get_users_for_api(&self) -> serde_json::Value {
        use serde_json::json;
        
        let users: Vec<serde_json::Value> = self.config.user_roles.iter().enumerate().map(|(index, (user_id, roles))| {
            // Get effective permissions for this user
            let context = PermissionContext {
                user_id: Some(user_id.clone()),
                user_roles: roles.clone(),
                api_key_name: None,
                resource: None,
                action: None,
                client_ip: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };
            
            let effective_roles = self.get_effective_roles(&context);
            let permission_count: usize = effective_roles.iter()
                .map(|role_name| self.get_role_permissions(role_name).len())
                .sum();
            
            json!({
                "id": format!("user_{}", index + 1),
                "username": user_id,
                "assigned_roles": roles,
                "effective_roles": effective_roles,
                "permission_count": permission_count,
                "status": "active",
                "role_count": roles.len(),
                "last_activity": Utc::now() - chrono::Duration::hours(24)
            })
        }).collect();
        
        json!(users)
    }
    
    /// Get role by name
    pub fn get_role(&self, name: &str) -> Option<&Role> {
        self.config.roles.get(name)
    }
    
    /// Get user roles
    pub fn get_user_roles(&self, user_id: &str) -> Vec<String> {
        self.config.user_roles.get(user_id).cloned().unwrap_or_default()
    }
    
    /// Get API key roles
    pub fn get_api_key_roles(&self, api_key: &str) -> Vec<String> {
        self.config.api_key_roles.get(api_key).cloned().unwrap_or_default()
    }
    
    /// Get all permissions from all roles
    pub fn get_all_permissions(&self) -> serde_json::Value {
        use serde_json::json;
        
        if let Ok(registry) = self.permissions_registry.lock() {
            let permissions: Vec<serde_json::Value> = registry.permissions.values().map(|permission| {
                json!({
                    "id": permission.name,
                    "name": Self::to_title_case(&permission.name.replace(':', " ").replace('_', " ")),
                    "description": permission.description.as_ref().unwrap_or(&"No description available".to_string()),
                    "resource_types": permission.resource_types,
                    "actions": permission.actions,
                    "category": self.get_permission_category(&permission.name)
                })
            }).collect();
            
            json!(permissions)
        } else {
            warn!("Failed to acquire permissions registry lock, returning empty permissions");
            json!([])
        }
    }
    
    /// Get category for a permission
    fn get_permission_category(&self, permission_name: &str) -> String {
        if permission_name.starts_with("tool:") {
            "tools".to_string()
        } else if permission_name.starts_with("resource:") {
            "resources".to_string()
        } else if permission_name.starts_with("prompt:") {
            "prompts".to_string()
        } else if permission_name == "admin" {
            "basic".to_string()
        } else {
            "basic".to_string()
        }
    }
    
    /// Get permission categories
    pub fn get_permission_categories(&self) -> serde_json::Value {
        use serde_json::json;
        
        if let Ok(registry) = self.permissions_registry.lock() {
            let categories: Vec<serde_json::Value> = registry.categories.values().map(|category| {
                json!({
                    "id": category.id,
                    "name": category.name,
                    "description": category.description,
                    "permission_count": category.permission_ids.len(),
                    "permissions": category.permission_ids
                })
            }).collect();
            
            json!(categories)
        } else {
            warn!("Failed to acquire permissions registry lock, returning empty categories");
            json!([])
        }
    }
    
    /// Count permissions in a specific category
    fn count_permissions_in_category(&self, category: &str) -> usize {
        let mut count = 0;
        for role in self.config.roles.values() {
            for permission in &role.permissions {
                let perm_category = if permission.starts_with("tool:") {
                    "tools"
                } else if permission.starts_with("resource:") {
                    "resources"
                } else if permission.starts_with("prompt:") {
                    "prompts"
                } else if permission == "admin" {
                    "administrative"
                } else {
                    "basic"
                };
                
                if perm_category == category {
                    count += 1;
                }
            }
        }
        count
    }
    
    /// Convert string to title case
    fn to_title_case(s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    // ============================================================================
    // Thread-safe CRUD operations for SecurityApi
    // ============================================================================
    
    /// Create a new role (thread-safe)
    pub fn create_role_safe(&self, role_data: serde_json::Value) -> Result<serde_json::Value, String> {
        use serde_json::json;
        
        let name = role_data.get("name")
            .and_then(|v| v.as_str())
            .ok_or("Role name is required")?;
            
        let description = role_data.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("New role");
            
        let permissions = role_data.get("permissions")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec!["read".to_string()]);
        
        if let Ok(mut config) = self.mutable_config.lock() {
            // Check if role already exists
            if config.roles.contains_key(name) {
                return Err(format!("Role '{}' already exists", name));
            }
            
            let role = Role {
                name: name.to_string(),
                description: Some(description.to_string()),
                permissions,
                parent_roles: vec![],
                active: true,
                metadata: HashMap::new(),
                created_at: Some(Utc::now()),
                modified_at: Some(Utc::now()),
            };
            
            config.roles.insert(name.to_string(), role.clone());
            drop(config); // Release lock before saving
            
            // Auto-save to file storage
            if let Err(e) = self.save_to_storage() {
                warn!("Failed to save RBAC changes to storage: {}", e);
            }
            
            Ok(json!({
                "id": role.name,
                "name": role.name,
                "description": role.description,
                "permissions": role.permissions,
                "active": role.active,
                "created_at": role.created_at,
                "modified_at": role.modified_at
            }))
        } else {
            Err("Failed to acquire config lock".to_string())
        }
    }
    
    /// Update an existing role (thread-safe)
    pub fn update_role_safe(&self, role_name: &str, role_data: serde_json::Value) -> Result<serde_json::Value, String> {
        use serde_json::json;
        
        if let Ok(mut config) = self.mutable_config.lock() {
            if !config.roles.contains_key(role_name) {
                return Err(format!("Role '{}' does not exist", role_name));
            }
            
            let mut role = config.roles.get(role_name).unwrap().clone();
            
            // Update fields if provided
            if let Some(description) = role_data.get("description").and_then(|v| v.as_str()) {
                role.description = Some(description.to_string());
            }
            
            if let Some(permissions_array) = role_data.get("permissions").and_then(|v| v.as_array()) {
                role.permissions = permissions_array.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
            
            if let Some(parent_roles_array) = role_data.get("parent_roles").and_then(|v| v.as_array()) {
                let parent_roles: Vec<String> = parent_roles_array.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                    
                // Validate parent roles exist
                for parent in &parent_roles {
                    if !config.roles.contains_key(parent) {
                        return Err(format!("Parent role '{}' does not exist", parent));
                    }
                }
                role.parent_roles = parent_roles;
            }
            
            if let Some(active) = role_data.get("active").and_then(|v| v.as_bool()) {
                role.active = active;
            }
            
            role.modified_at = Some(Utc::now());
            
            config.roles.insert(role_name.to_string(), role.clone());
            drop(config); // Release lock before saving
            
            // Auto-save to file storage
            if let Err(e) = self.save_to_storage() {
                warn!("Failed to save RBAC changes to storage: {}", e);
            }
            
            Ok(json!({
                "id": role.name,
                "name": role.name,
                "description": role.description,
                "permissions": role.permissions,
                "parent_roles": role.parent_roles,
                "active": role.active,
                "created_at": role.created_at,
                "modified_at": role.modified_at
            }))
        } else {
            Err("Failed to acquire config lock".to_string())
        }
    }

    /// Delete a role (thread-safe)
    pub fn delete_role_safe(&self, role_name: &str) -> Result<serde_json::Value, String> {
        use serde_json::json;
        
        if let Ok(mut config) = self.mutable_config.lock() {
            if !config.roles.contains_key(role_name) {
                return Err(format!("Role '{}' does not exist", role_name));
            }
            
            let deleted_role = config.roles.remove(role_name).unwrap();
            
            // Remove from default roles if present
            config.default_roles.retain(|r| r != role_name);
            
            drop(config); // Release lock before saving
            
            // Auto-save to file storage
            if let Err(e) = self.save_to_storage() {
                warn!("Failed to save RBAC changes to storage: {}", e);
            }
            
            Ok(json!({
                "success": true,
                "message": format!("Role '{}' deleted successfully", role_name),
                "deleted_role": {
                    "name": deleted_role.name,
                    "description": deleted_role.description
                }
            }))
        } else {
            Err("Failed to acquire config lock".to_string())
        }
    }
    
    /// Get role statistics (thread-safe)
    pub fn get_role_statistics_safe(&self) -> serde_json::Value {
        use serde_json::json;
        
        if let (Ok(config), Ok(stats)) = (self.mutable_config.lock(), self.stats.lock()) {
            let active_roles = config.roles.values().filter(|role| role.active).count();
            let total_permissions: usize = config.roles.values()
                .map(|role| role.permissions.len())
                .sum();
            let users_with_roles = config.user_roles.len();
            
            json!({
                "totalRoles": config.roles.len(),
                "activeRoles": active_roles,
                "totalPermissions": total_permissions,
                "usersWithRoles": users_with_roles,
                "totalAuthAttempts": stats.total_auth_attempts,
                "successfulAuth": stats.successful_auth
            })
        } else {
            json!({
                "totalRoles": 0,
                "activeRoles": 0,
                "error": "Failed to acquire statistics"
            })
        }
    }

    // ============================================================================
    // User Role Assignment Methods (Thread-safe)
    // ============================================================================
    
    /// Assign role to user (thread-safe)
    pub fn assign_user_role_safe(&self, user_id: &str, role_name: &str) -> Result<serde_json::Value, String> {
        use serde_json::json;
        
        if let Ok(mut config) = self.mutable_config.lock() {
            if !config.roles.contains_key(role_name) {
                return Err(format!("Role '{}' does not exist", role_name));
            }
            
            let user_roles = config.user_roles.entry(user_id.to_string()).or_default();
            
            if !user_roles.contains(&role_name.to_string()) {
                user_roles.push(role_name.to_string());
                drop(config); // Release lock before saving
                
                // Auto-save to file storage
                if let Err(e) = self.save_to_storage() {
                    warn!("Failed to save RBAC changes to storage: {}", e);
                }
                
                Ok(json!({
                    "success": true,
                    "message": format!("Role '{}' assigned to user '{}' successfully", role_name, user_id),
                    "user_id": user_id,
                    "role": role_name
                }))
            } else {
                Ok(json!({
                    "success": true,
                    "message": format!("User '{}' already has role '{}'", user_id, role_name),
                    "user_id": user_id,
                    "role": role_name
                }))
            }
        } else {
            Err("Failed to acquire config lock".to_string())
        }
    }
    
    /// Remove role from user (thread-safe)
    pub fn remove_user_role_safe(&self, user_id: &str, role_name: &str) -> Result<serde_json::Value, String> {
        use serde_json::json;
        
        if let Ok(mut config) = self.mutable_config.lock() {
            if let Some(user_roles) = config.user_roles.get_mut(user_id) {
                user_roles.retain(|r| r != role_name);
                
                // Remove user entry if no roles left
                if user_roles.is_empty() {
                    config.user_roles.remove(user_id);
                }
                
                drop(config); // Release lock before saving
                
                // Auto-save to file storage
                if let Err(e) = self.save_to_storage() {
                    warn!("Failed to save RBAC changes to storage: {}", e);
                }
                
                Ok(json!({
                    "success": true,
                    "message": format!("Role '{}' removed from user '{}' successfully", role_name, user_id),
                    "user_id": user_id,
                    "role": role_name
                }))
            } else {
                Ok(json!({
                    "success": true,
                    "message": format!("User '{}' does not have any roles", user_id),
                    "user_id": user_id
                }))
            }
        } else {
            Err("Failed to acquire config lock".to_string())
        }
    }
    
    /// Update user roles (replace all roles for a user)
    pub fn update_user_roles_safe(&self, user_id: &str, new_roles: Vec<String>) -> Result<serde_json::Value, String> {
        use serde_json::json;
        
        if let Ok(mut config) = self.mutable_config.lock() {
            // Validate all roles exist
            for role_name in &new_roles {
                if !config.roles.contains_key(role_name) {
                    return Err(format!("Role '{}' does not exist", role_name));
                }
            }
            
            if new_roles.is_empty() {
                // Remove user if no roles
                config.user_roles.remove(user_id);
            } else {
                // Set new roles
                config.user_roles.insert(user_id.to_string(), new_roles.clone());
            }
            
            drop(config); // Release lock before saving
            
            // Auto-save to file storage
            if let Err(e) = self.save_to_storage() {
                warn!("Failed to save RBAC changes to storage: {}", e);
            }
            
            Ok(json!({
                "success": true,
                "message": format!("User '{}' roles updated successfully", user_id),
                "user_id": user_id,
                "roles": new_roles
            }))
        } else {
            Err("Failed to acquire config lock".to_string())
        }
    }
    
    /// Delete user (remove all role assignments)
    pub fn delete_user_safe(&self, user_id: &str) -> Result<serde_json::Value, String> {
        use serde_json::json;
        
        if let Ok(mut config) = self.mutable_config.lock() {
            let had_roles = config.user_roles.remove(user_id).is_some();
            drop(config); // Release lock before saving
            
            // Auto-save to file storage
            if let Err(e) = self.save_to_storage() {
                warn!("Failed to save RBAC changes to storage: {}", e);
            }
            
            Ok(json!({
                "success": true,
                "message": if had_roles {
                    format!("User '{}' and all role assignments deleted successfully", user_id)
                } else {
                    format!("User '{}' had no role assignments to delete", user_id)
                },
                "user_id": user_id,
                "had_roles": had_roles
            }))
        } else {
            Err("Failed to acquire config lock".to_string())
        }
    }
}

// Implementation of SecurityServiceStatistics trait for RbacService
impl SecurityServiceStatistics for RbacService {
    type Statistics = RbacStatistics;
    
    async fn get_statistics(&self) -> Self::Statistics {
        let stats = match self.stats.lock() {
            Ok(stats) => stats.clone(),
            Err(_) => {
                // If mutex is poisoned, return default stats
                RbacStats {
                    start_time: Utc::now(),
                    total_auth_attempts: 0,
                    successful_auth: 0,
                    failed_auth: 0,
                    permission_evaluations: 0,
                    active_sessions: 0,
                    role_usage: HashMap::new(),
                    last_error: Some("Mutex poisoned".to_string()),
                    total_processing_time_ms: 0,
                }
            }
        };
        let service_health = self.get_health().await;
        
        // Get top active roles
        let mut role_usage: Vec<RoleUsage> = stats.role_usage.iter()
            .map(|(role_name, usage_count)| RoleUsage {
                role_name: role_name.clone(),
                user_count: 0, // Would need to track this separately
                active_sessions: 0, // Would need to track this separately  
                last_used: Utc::now(), // Would need to track this per role
            })
            .collect();
        role_usage.sort_by(|a, b| a.user_count.cmp(&b.user_count));
        role_usage.truncate(10); // Top 10 roles
        
        RbacStatistics {
            health: service_health,
            total_roles: self.config.roles.len() as u32,
            total_users: self.config.user_roles.len() as u32,
            total_permissions: self.config.roles.values()
                .map(|role| role.permissions.len())
                .sum::<usize>() as u32,
            active_sessions: stats.active_sessions,
            total_auth_attempts: stats.total_auth_attempts,
            successful_auth: stats.successful_auth,
            failed_auth: stats.failed_auth,
            permission_evaluations: stats.permission_evaluations,
            top_roles: role_usage,
        }
    }
    
    async fn get_health(&self) -> ServiceHealth {
        let stats = match self.stats.lock() {
            Ok(stats) => stats.clone(),
            Err(_) => {
                // If mutex is poisoned, return default stats
                RbacStats {
                    start_time: Utc::now(),
                    total_auth_attempts: 0,
                    successful_auth: 0,
                    failed_auth: 0,
                    permission_evaluations: 0,
                    active_sessions: 0,
                    role_usage: HashMap::new(),
                    last_error: Some("Mutex poisoned".to_string()),
                    total_processing_time_ms: 0,
                }
            }
        };
        let uptime_seconds = (Utc::now() - stats.start_time).num_seconds() as u64;
        
        let avg_response_time_ms = if stats.permission_evaluations > 0 {
            stats.total_processing_time_ms as f64 / stats.permission_evaluations as f64
        } else {
            0.0
        };
        
        let error_rate = if stats.total_auth_attempts > 0 {
            stats.failed_auth as f64 / stats.total_auth_attempts as f64
        } else {
            0.0
        };
        
        let requests_per_second = if uptime_seconds > 0 {
            stats.permission_evaluations as f64 / uptime_seconds as f64
        } else {
            0.0
        };
        
        let health_status = if stats.last_error.is_some() {
            HealthStatus::Error
        } else if self.config.enabled {
            HealthStatus::Healthy
        } else {
            HealthStatus::Disabled
        };
        
        ServiceHealth {
            status: health_status.clone(),
            is_healthy: matches!(health_status, HealthStatus::Healthy),
            last_checked: Utc::now(),
            error_message: stats.last_error.clone(),
            uptime_seconds,
            performance: PerformanceMetrics {
                avg_response_time_ms,
                requests_per_second,
                error_rate,
                memory_usage_bytes: 0, // Would need actual memory tracking
            },
        }
    }
    
    async fn reset_statistics(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self.stats.lock() {
            Ok(mut stats) => {
                *stats = RbacStats {
                    start_time: Utc::now(),
                    total_auth_attempts: 0,
                    successful_auth: 0,
                    failed_auth: 0,
                    permission_evaluations: 0,
                    active_sessions: 0,
                    role_usage: HashMap::new(),
                    last_error: None,
                    total_processing_time_ms: 0,
                };
                Ok(())
            }
            Err(_) => Err("Mutex poisoned".into())
        }
    }
}

impl HealthMonitor for RbacService {
    async fn is_healthy(&self) -> bool {
        self.config.enabled && 
        self.stats.lock().map(|stats| stats.last_error.is_none()).unwrap_or(false)
    }
    
    async fn health_check(&self) -> ServiceHealth {
        self.get_health().await
    }
    
    fn get_uptime(&self) -> u64 {
        self.stats.lock()
            .map(|stats| (Utc::now() - stats.start_time).num_seconds() as u64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_permission_matching() {
        let rbac = RbacService::new(RbacConfig::default()).unwrap();
        
        let permissions = vec![
            "read".to_string(),
            "tool:*".to_string(),
            "resource:read".to_string(),
        ];
        
        assert!(rbac.permission_matches("read", &permissions));
        assert!(rbac.permission_matches("tool:execute", &permissions));
        assert!(rbac.permission_matches("resource:read", &permissions));
        assert!(!rbac.permission_matches("resource:write", &permissions));
        assert!(!rbac.permission_matches("admin", &permissions));
    }
    
    #[test]
    fn test_role_inheritance() {
        let config = RbacConfig::default();
        let rbac = RbacService::new(config).unwrap();
        
        let context = PermissionContext {
            user_id: Some("test_user".to_string()),
            user_roles: vec!["operator".to_string()],
            api_key_name: None,
            resource: None,
            action: None,
            client_ip: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        
        // Operator should inherit user permissions due to parent_roles
        let result = rbac.check_permission("read", &context);
        assert!(result.granted);
    }
    
    #[test]
    fn test_wildcard_permissions() {
        let rbac = RbacService::new(RbacConfig::default()).unwrap();
        
        // Test various wildcard patterns
        assert!(rbac.permission_pattern_matches("tool:execute", "tool:*"));
        assert!(rbac.permission_pattern_matches("resource:read", "resource:*"));
        assert!(rbac.permission_pattern_matches("anything", "*"));
        assert!(!rbac.permission_pattern_matches("tool:execute", "resource:*"));
    }
}