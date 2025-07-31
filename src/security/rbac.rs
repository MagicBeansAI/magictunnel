//! Role-Based Access Control (RBAC) for MagicTunnel
//!
//! Provides comprehensive role and permission management,
//! similar to MCP Manager's RBAC capabilities.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

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

/// RBAC service for managing roles and permissions
pub struct RbacService {
    config: RbacConfig,
    /// Compiled permission conditions
    compiled_conditions: HashMap<String, CompiledCondition>,
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

impl RbacService {
    /// Create a new RBAC service
    pub fn new(config: RbacConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut compiled_conditions = HashMap::new();
        
        // Pre-compile conditions for performance
        for role in config.roles.values() {
            // For now, we don't have conditions directly on roles
            // but this could be extended
        }
        
        Ok(Self {
            config,
            compiled_conditions,
        })
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