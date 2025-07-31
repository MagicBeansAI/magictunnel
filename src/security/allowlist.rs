//! Tool allowlisting system for MagicTunnel
//!
//! Provides explicit control over which tools, resources, and prompts can be accessed
//! by different users and API keys. Similar to MCP Manager's allowlist filtering.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, warn};
use regex::Regex;

/// Configuration for tool allowlisting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistConfig {
    /// Whether allowlisting is enabled
    pub enabled: bool,
    /// Default action when no rule matches: allow or deny
    pub default_action: AllowlistAction,
    /// Tool-specific allowlist rules
    pub tools: HashMap<String, ToolAllowlistRule>,
    /// Resource-specific allowlist rules  
    pub resources: HashMap<String, ResourceAllowlistRule>,
    /// Prompt-specific allowlist rules
    pub prompts: HashMap<String, PromptAllowlistRule>,
    /// Global rules that apply to all items
    pub global_rules: Vec<GlobalAllowlistRule>,
}

/// Action to take for allowlist decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AllowlistAction {
    Allow,
    Deny,
    RequireApproval,
}

/// Tool-specific allowlist rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAllowlistRule {
    /// Tool name (supports wildcards)
    pub name: String,
    /// Action to take
    pub action: AllowlistAction,
    /// Required permissions to access this tool
    pub required_permissions: Vec<String>,
    /// API keys that can access this tool
    pub allowed_api_keys: Option<Vec<String>>, 
    /// User roles that can access this tool
    pub allowed_roles: Option<Vec<String>>,
    /// Parameters that are allowed/blocked
    pub parameter_rules: Option<ParameterRules>,
    /// Rate limiting for this tool
    pub rate_limit: Option<RateLimit>,
}

/// Resource-specific allowlist rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllowlistRule {
    /// Resource URI pattern (supports wildcards)
    pub uri_pattern: String,
    /// Action to take
    pub action: AllowlistAction,
    /// Required permissions
    pub required_permissions: Vec<String>,
    /// Allowed API keys
    pub allowed_api_keys: Option<Vec<String>>,
    /// Allowed roles
    pub allowed_roles: Option<Vec<String>>,
}

/// Prompt-specific allowlist rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAllowlistRule {
    /// Prompt name pattern (supports wildcards)
    pub name_pattern: String,
    /// Action to take
    pub action: AllowlistAction,
    /// Required permissions
    pub required_permissions: Vec<String>,
    /// Allowed API keys
    pub allowed_api_keys: Option<Vec<String>>,
    /// Allowed roles
    pub allowed_roles: Option<Vec<String>>,
}

/// Global allowlist rule that applies to all items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalAllowlistRule {
    /// Rule name for logging
    pub name: String,
    /// Pattern to match against
    pub pattern: AllowlistPattern,
    /// Action to take
    pub action: AllowlistAction,
    /// Priority (higher numbers override lower)
    pub priority: i32,
}

/// Pattern matching for allowlist rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AllowlistPattern {
    /// Exact string match
    Exact { value: String },
    /// Wildcard pattern (*, ?)
    Wildcard { pattern: String },
    /// Regular expression
    Regex { pattern: String },
    /// Source server/endpoint match
    Source { server: String },
    /// User/role match
    User { user_id: String, roles: Vec<String> },
}

/// Parameter filtering rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterRules {
    /// Parameters that are explicitly allowed 
    pub allowed: Option<Vec<String>>,
    /// Parameters that are explicitly blocked
    pub blocked: Option<Vec<String>>,
    /// Regex patterns for parameter values that are blocked
    pub blocked_value_patterns: Option<Vec<String>>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum requests per time window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Action when limit exceeded
    pub action: RateLimitAction,
}

/// Action when rate limit is exceeded
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RateLimitAction {
    Block,
    Delay,
    RequireApproval,
}

/// Context for allowlist evaluation
#[derive(Debug, Clone)]
pub struct AllowlistContext {
    /// User ID (from JWT/OAuth)
    pub user_id: Option<String>,
    /// User roles
    pub user_roles: Vec<String>,
    /// API key name (if using API key auth)
    pub api_key_name: Option<String>,
    /// User permissions
    pub permissions: Vec<String>,
    /// Source server/endpoint
    pub source: Option<String>,
    /// Request IP address
    pub client_ip: Option<String>,
}

/// Result of allowlist evaluation
#[derive(Debug, Clone)]
pub struct AllowlistResult {
    /// Whether access is allowed
    pub allowed: bool,
    /// Action taken
    pub action: AllowlistAction,
    /// Rule that matched (if any)
    pub matched_rule: Option<String>,
    /// Reason for the decision
    pub reason: String,
    /// Whether approval is required
    pub requires_approval: bool,
}

/// Tool allowlist service
pub struct AllowlistService {
    config: AllowlistConfig,
    compiled_patterns: HashMap<String, Regex>,
}

impl Default for AllowlistConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_action: AllowlistAction::Allow,
            tools: HashMap::new(),
            resources: HashMap::new(),
            prompts: HashMap::new(),
            global_rules: Vec::new(),
        }
    }
}

impl AllowlistService {
    /// Create a new allowlist service
    pub fn new(config: AllowlistConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut compiled_patterns = HashMap::new();
        
        // Pre-compile regex patterns for performance
        for rule in &config.global_rules {
            if let AllowlistPattern::Regex { pattern } = &rule.pattern {
                let regex = Regex::new(pattern)?;
                compiled_patterns.insert(format!("global_{}", rule.name), regex);
            }
        }
        
        // Compile parameter blocking patterns
        for (name, tool_rule) in &config.tools {
            if let Some(param_rules) = &tool_rule.parameter_rules {
                if let Some(blocked_patterns) = &param_rules.blocked_value_patterns {
                    for (i, pattern) in blocked_patterns.iter().enumerate() {
                        let regex = Regex::new(pattern)?;
                        compiled_patterns.insert(format!("tool_{}_param_{}", name, i), regex);
                    }
                }
            }
        }
        
        Ok(Self {
            config,
            compiled_patterns,
        })
    }
    
    /// Check if a tool access is allowed
    pub fn check_tool_access(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
        context: &AllowlistContext,
    ) -> AllowlistResult {
        if !self.config.enabled {
            return AllowlistResult {
                allowed: true,
                action: AllowlistAction::Allow,
                matched_rule: None,
                reason: "Allowlist disabled".to_string(),
                requires_approval: false,
            };
        }
        
        debug!("Checking tool access for: {} with context: {:?}", tool_name, context);
        
        // Check global rules first (by priority)
        let mut global_rules = self.config.global_rules.clone();
        global_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        for rule in &global_rules {
            if self.pattern_matches(&rule.pattern, tool_name, context) {
                return AllowlistResult {
                    allowed: matches!(rule.action, AllowlistAction::Allow),
                    action: rule.action.clone(),
                    matched_rule: Some(rule.name.clone()),
                    reason: format!("Matched global rule: {}", rule.name),
                    requires_approval: matches!(rule.action, AllowlistAction::RequireApproval),
                };
            }
        }
        
        // Check tool-specific rules
        for (pattern, rule) in &self.config.tools {
            if self.wildcard_match(pattern, tool_name) {
                // Check permissions
                if !rule.required_permissions.is_empty() {
                    if !rule.required_permissions.iter().all(|perm| context.permissions.contains(perm)) {
                        return AllowlistResult {
                            allowed: false,
                            action: AllowlistAction::Deny,
                            matched_rule: Some(pattern.clone()),
                            reason: "Insufficient permissions".to_string(),
                            requires_approval: false,
                        };
                    }
                }
                
                // Check API key allowlist
                if let Some(allowed_keys) = &rule.allowed_api_keys {
                    if let Some(api_key) = &context.api_key_name {
                        if !allowed_keys.contains(api_key) {
                            return AllowlistResult {
                                allowed: false,
                                action: AllowlistAction::Deny,
                                matched_rule: Some(pattern.clone()),
                                reason: "API key not in allowlist".to_string(),
                                requires_approval: false,
                            };
                        }
                    }
                }
                
                // Check role allowlist
                if let Some(allowed_roles) = &rule.allowed_roles {
                    if !context.user_roles.iter().any(|role| allowed_roles.contains(role)) {
                        return AllowlistResult {
                            allowed: false,
                            action: AllowlistAction::Deny,
                            matched_rule: Some(pattern.clone()),
                            reason: "User role not in allowlist".to_string(),
                            requires_approval: false,
                        };
                    }
                }
                
                // Check parameter rules
                if let Some(param_rules) = &rule.parameter_rules {
                    if let Err(reason) = self.check_parameters(parameters, param_rules, tool_name) {
                        return AllowlistResult {
                            allowed: false,
                            action: AllowlistAction::Deny,
                            matched_rule: Some(pattern.clone()),
                            reason,
                            requires_approval: false,
                        };
                    }
                }
                
                return AllowlistResult {
                    allowed: matches!(rule.action, AllowlistAction::Allow),
                    action: rule.action.clone(),
                    matched_rule: Some(pattern.clone()),
                    reason: format!("Matched tool rule: {}", pattern),
                    requires_approval: matches!(rule.action, AllowlistAction::RequireApproval),
                };
            }
        }
        
        // Apply default action
        AllowlistResult {
            allowed: matches!(self.config.default_action, AllowlistAction::Allow),
            action: self.config.default_action.clone(),
            matched_rule: None,
            reason: "No matching rule, applied default action".to_string(),
            requires_approval: matches!(self.config.default_action, AllowlistAction::RequireApproval),
        }
    }
    
    /// Check if resource access is allowed
    pub fn check_resource_access(
        &self,
        resource_uri: &str,
        context: &AllowlistContext,
    ) -> AllowlistResult {
        if !self.config.enabled {
            return AllowlistResult {
                allowed: true,
                action: AllowlistAction::Allow,
                matched_rule: None,
                reason: "Allowlist disabled".to_string(),
                requires_approval: false,
            };
        }
        
        // Check resource-specific rules
        for (pattern, rule) in &self.config.resources {
            if self.wildcard_match(pattern, resource_uri) {
                // Check permissions
                if !rule.required_permissions.is_empty() {
                    if !rule.required_permissions.iter().all(|perm| context.permissions.contains(perm)) {
                        return AllowlistResult {
                            allowed: false,
                            action: AllowlistAction::Deny,
                            matched_rule: Some(pattern.clone()),
                            reason: "Insufficient permissions for resource".to_string(),
                            requires_approval: false,
                        };
                    }
                }
                
                return AllowlistResult {
                    allowed: matches!(rule.action, AllowlistAction::Allow),
                    action: rule.action.clone(),
                    matched_rule: Some(pattern.clone()),
                    reason: format!("Matched resource rule: {}", pattern),
                    requires_approval: matches!(rule.action, AllowlistAction::RequireApproval),
                };
            }
        }
        
        // Apply default action
        AllowlistResult {
            allowed: matches!(self.config.default_action, AllowlistAction::Allow),
            action: self.config.default_action.clone(),
            matched_rule: None,
            reason: "No matching resource rule, applied default action".to_string(),
            requires_approval: matches!(self.config.default_action, AllowlistAction::RequireApproval),
        }
    }
    
    /// Check if prompt access is allowed
    pub fn check_prompt_access(
        &self,
        prompt_name: &str,
        context: &AllowlistContext,
    ) -> AllowlistResult {
        if !self.config.enabled {
            return AllowlistResult {
                allowed: true,
                action: AllowlistAction::Allow,
                matched_rule: None,
                reason: "Allowlist disabled".to_string(),
                requires_approval: false,
            };
        }
        
        // Check prompt-specific rules
        for (pattern, rule) in &self.config.prompts {
            if self.wildcard_match(pattern, prompt_name) {
                // Check permissions
                if !rule.required_permissions.is_empty() {
                    if !rule.required_permissions.iter().all(|perm| context.permissions.contains(perm)) {
                        return AllowlistResult {
                            allowed: false,
                            action: AllowlistAction::Deny,
                            matched_rule: Some(pattern.clone()),
                            reason: "Insufficient permissions for prompt".to_string(),
                            requires_approval: false,
                        };
                    }
                }
                
                return AllowlistResult {
                    allowed: matches!(rule.action, AllowlistAction::Allow),
                    action: rule.action.clone(),
                    matched_rule: Some(pattern.clone()),
                    reason: format!("Matched prompt rule: {}", pattern),
                    requires_approval: matches!(rule.action, AllowlistAction::RequireApproval),
                };
            }
        }
        
        // Apply default action
        AllowlistResult {
            allowed: matches!(self.config.default_action, AllowlistAction::Allow),
            action: self.config.default_action.clone(),
            matched_rule: None,
            reason: "No matching prompt rule, applied default action".to_string(),
            requires_approval: matches!(self.config.default_action, AllowlistAction::RequireApproval),
        }
    }
    
    /// Check if pattern matches
    fn pattern_matches(
        &self,
        pattern: &AllowlistPattern,
        value: &str,
        context: &AllowlistContext,
    ) -> bool {
        match pattern {
            AllowlistPattern::Exact { value: pattern_value } => pattern_value == value,
            AllowlistPattern::Wildcard { pattern } => self.wildcard_match(pattern, value),
            AllowlistPattern::Regex { pattern } => {
                // This would need the pre-compiled regex
                // For now, compile on the fly (should be optimized)
                if let Ok(regex) = Regex::new(pattern) {
                    regex.is_match(value)
                } else {
                    false
                }
            }
            AllowlistPattern::Source { server } => {
                context.source.as_ref().map_or(false, |s| s == server)
            }
            AllowlistPattern::User { user_id, roles } => {
                if let Some(ctx_user_id) = &context.user_id {
                    if ctx_user_id == user_id {
                        return true;
                    }
                }
                roles.iter().any(|role| context.user_roles.contains(role))
            }
        }
    }
    
    /// Simple wildcard matching (* and ?)
    fn wildcard_match(&self, pattern: &str, value: &str) -> bool {
        // Convert wildcard pattern to regex
        let regex_pattern = pattern
            .replace('*', ".*")
            .replace('?', ".");
        
        if let Ok(regex) = Regex::new(&format!("^{}$", regex_pattern)) {
            regex.is_match(value)
        } else {
            pattern == value
        }
    }
    
    /// Check parameter rules
    fn check_parameters(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
        rules: &ParameterRules,
        tool_name: &str,
    ) -> Result<(), String> {
        // Check allowed parameters
        if let Some(allowed) = &rules.allowed {
            for param_name in parameters.keys() {
                if !allowed.contains(param_name) {
                    return Err(format!("Parameter '{}' not in allowlist", param_name));
                }
            }
        }
        
        // Check blocked parameters
        if let Some(blocked) = &rules.blocked {
            for param_name in parameters.keys() {
                if blocked.contains(param_name) {
                    return Err(format!("Parameter '{}' is blocked", param_name));
                }
            }
        }
        
        // Check blocked value patterns
        if let Some(blocked_patterns) = &rules.blocked_value_patterns {
            for (param_name, param_value) in parameters {
                let value_str = param_value.to_string();
                for (i, _pattern) in blocked_patterns.iter().enumerate() {
                    let regex_key = format!("tool_{}_param_{}", tool_name, i);
                    if let Some(regex) = self.compiled_patterns.get(&regex_key) {
                        if regex.is_match(&value_str) {
                            return Err(format!(
                                "Parameter '{}' value matches blocked pattern",
                                param_name
                            ));
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wildcard_matching() {
        let service = AllowlistService::new(AllowlistConfig::default()).unwrap();
        
        assert!(service.wildcard_match("test*", "testing"));
        assert!(service.wildcard_match("test*", "test"));
        assert!(!service.wildcard_match("test*", "other"));
        
        assert!(service.wildcard_match("test?", "test1"));
        assert!(service.wildcard_match("test?", "testa"));
        assert!(!service.wildcard_match("test?", "test12"));
    }
    
    #[test]
    fn test_tool_allowlist_disabled() {
        let config = AllowlistConfig {
            enabled: false,
            ..Default::default()
        };
        let service = AllowlistService::new(config).unwrap();
        let context = AllowlistContext {
            user_id: None,
            user_roles: vec![],
            api_key_name: None,
            permissions: vec![],
            source: None,
            client_ip: None,
        };
        
        let result = service.check_tool_access("any_tool", &HashMap::new(), &context);
        assert!(result.allowed);
    }
}