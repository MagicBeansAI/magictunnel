//! Tool allowlisting system for MagicTunnel
//!
//! Provides explicit control over which tools, resources, and prompts can be accessed
//! by different users and API keys. Similar to MCP Manager's allowlist filtering.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::debug;
use regex::Regex;
use chrono::{DateTime, Utc, Timelike};
use super::statistics::{SecurityServiceStatistics, HealthMonitor, ServiceHealth, HealthStatus, AllowlistStatistics, HourlyMetric, RuleMatch, PerformanceMetrics};

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

/// Statistics tracking for allowlist service
#[derive(Debug, Clone)]
struct AllowlistStats {
    /// Service start time
    start_time: DateTime<Utc>,
    /// Total requests processed
    total_requests: u64,
    /// Requests that were allowed
    allowed_requests: u64,
    /// Requests that were blocked
    blocked_requests: u64,
    /// Requests that required approval
    approval_required_requests: u64,
    /// Rule match counts
    rule_matches: HashMap<String, u64>,
    /// Last error message (if any)
    last_error: Option<String>,
    /// Performance tracking
    total_processing_time_ms: u64,
    /// Hourly request patterns (last 24 hours)
    hourly_stats: Vec<HourlyMetric>,
}

/// Tool allowlist service
pub struct AllowlistService {
    config: AllowlistConfig,
    compiled_patterns: HashMap<String, Regex>,
    stats: Arc<Mutex<AllowlistStats>>,
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
        
        let stats = AllowlistStats {
            start_time: Utc::now(),
            total_requests: 0,
            allowed_requests: 0,
            blocked_requests: 0,
            approval_required_requests: 0,
            rule_matches: HashMap::new(),
            last_error: None,
            total_processing_time_ms: 0,
            hourly_stats: Vec::new(),
        };

        Ok(Self {
            config,
            compiled_patterns,
            stats: Arc::new(Mutex::new(stats)),
        })
    }
    
    /// Create a result and update statistics
    fn create_result_with_stats(&self, result: AllowlistResult, start_time: std::time::Instant) -> AllowlistResult {
        let processing_time_ms = start_time.elapsed().as_millis() as u64;
        self.update_stats(&result, processing_time_ms);
        result
    }
    
    /// Update statistics for a request
    fn update_stats(&self, result: &AllowlistResult, processing_time_ms: u64) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_requests += 1;
            stats.total_processing_time_ms += processing_time_ms;
            
            match result.action {
                AllowlistAction::Allow => stats.allowed_requests += 1,
                AllowlistAction::Deny => stats.blocked_requests += 1,
                AllowlistAction::RequireApproval => stats.approval_required_requests += 1,
            }
            
            if let Some(rule_name) = &result.matched_rule {
                *stats.rule_matches.entry(rule_name.clone()).or_insert(0) += 1;
            }
            
            // Update hourly stats (simplified - just track current hour)
            let current_hour = Utc::now().date_naive().and_hms_opt(Utc::now().hour(), 0, 0)
                .unwrap_or_default().and_utc();
            
            if let Some(last_metric) = stats.hourly_stats.last_mut() {
                if last_metric.hour == current_hour {
                    // Update existing hour
                    last_metric.request_count += 1;
                    match result.action {
                        AllowlistAction::Allow => last_metric.allowed_count += 1,
                        AllowlistAction::Deny => last_metric.blocked_count += 1,
                        AllowlistAction::RequireApproval => {} // Could track separately
                    }
                } else {
                    // New hour
                    stats.hourly_stats.push(HourlyMetric {
                        hour: current_hour,
                        request_count: 1,
                        blocked_count: if matches!(result.action, AllowlistAction::Deny) { 1 } else { 0 },
                        allowed_count: if matches!(result.action, AllowlistAction::Allow) { 1 } else { 0 },
                    });
                }
            } else {
                // First metric
                stats.hourly_stats.push(HourlyMetric {
                    hour: current_hour,
                    request_count: 1,
                    blocked_count: if matches!(result.action, AllowlistAction::Deny) { 1 } else { 0 },
                    allowed_count: if matches!(result.action, AllowlistAction::Allow) { 1 } else { 0 },
                });
            }
            
            // Keep only last 24 hours of stats
            let cutoff = Utc::now() - chrono::Duration::hours(24);
            stats.hourly_stats.retain(|metric| metric.hour >= cutoff);
        }
    }
    
    /// Check if a tool access is allowed
    pub fn check_tool_access(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
        context: &AllowlistContext,
    ) -> AllowlistResult {
        let start_time = std::time::Instant::now();
        if !self.config.enabled {
            let result = AllowlistResult {
                allowed: true,
                action: AllowlistAction::Allow,
                matched_rule: None,
                reason: "Allowlist disabled".to_string(),
                requires_approval: false,
            };
            let processing_time_ms = start_time.elapsed().as_millis() as u64;
            self.update_stats(&result, processing_time_ms);
            return result;
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
        let result = AllowlistResult {
            allowed: matches!(self.config.default_action, AllowlistAction::Allow),
            action: self.config.default_action.clone(),
            matched_rule: None,
            reason: "No matching rule, applied default action".to_string(),
            requires_approval: matches!(self.config.default_action, AllowlistAction::RequireApproval),
        };
        
        // Track statistics
        let processing_time_ms = start_time.elapsed().as_millis() as u64;
        self.update_stats(&result, processing_time_ms);
        
        result
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
    
    /// Get all configured rules for API display
    pub fn get_configured_rules(&self) -> serde_json::Value {
        use serde_json::json;
        
        let mut rules = Vec::new();
        let mut rule_id = 1;
        
        // Add tool rules
        for (pattern, rule) in &self.config.tools {
            rules.push(json!({
                "id": rule_id.to_string(),
                "name": rule.name.clone(),
                "enabled": true, // ToolAllowlistRule doesn't have enabled field, assume true if configured
                "rule_type": "tool",
                "pattern": pattern,
                "action": format!("{:?}", rule.action).to_lowercase(),
                "priority": 50, // Default priority
                "created_at": Utc::now(),
                "updated_at": Utc::now(),
                "description": format!("Tool allowlist rule for {}", rule.name),
                "required_permissions": rule.required_permissions
            }));
            rule_id += 1;
        }
        
        // Add resource rules
        for (pattern, rule) in &self.config.resources {
            rules.push(json!({
                "id": rule_id.to_string(),
                "name": format!("Resource: {}", pattern),
                "enabled": true, // ResourceAllowlistRule doesn't have enabled field
                "rule_type": "resource",
                "pattern": pattern,
                "action": format!("{:?}", rule.action).to_lowercase(),
                "priority": 50, // Default priority since no priority field
                "created_at": Utc::now(),
                "updated_at": Utc::now(),
                "description": format!("Resource allowlist rule for {}", pattern)
            }));
            rule_id += 1;
        }
        
        // Add prompt rules  
        for (pattern, rule) in &self.config.prompts {
            rules.push(json!({
                "id": rule_id.to_string(),
                "name": format!("Prompt: {}", pattern),
                "enabled": true, // PromptAllowlistRule doesn't have enabled field
                "rule_type": "prompt", 
                "pattern": pattern,
                "action": format!("{:?}", rule.action).to_lowercase(),
                "priority": 50, // Default priority since no priority field
                "created_at": Utc::now(),
                "updated_at": Utc::now(),
                "description": format!("Prompt allowlist rule for {}", pattern)
            }));
            rule_id += 1;
        }
        
        json!(rules)
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

// Implementation of SecurityServiceStatistics trait for AllowlistService
impl SecurityServiceStatistics for AllowlistService {
    type Statistics = AllowlistStatistics;
    
    async fn get_statistics(&self) -> Self::Statistics {
        let stats = self.stats.lock().unwrap().clone();
        let service_health = self.get_health().await;
        
        // Get top matched rules (sorted by frequency)
        let mut rule_matches: Vec<RuleMatch> = stats.rule_matches.iter()
            .map(|(rule_name, count)| RuleMatch {
                rule_name: rule_name.clone(),
                match_count: *count,
                action: "unknown".to_string(), // Would need to track this per rule
                last_matched: Utc::now(), // Would need to track this per rule
            })
            .collect();
        rule_matches.sort_by(|a, b| b.match_count.cmp(&a.match_count));
        rule_matches.truncate(10); // Top 10 rules
        
        AllowlistStatistics {
            health: service_health,
            total_rules: (self.config.tools.len() + 
                         self.config.resources.len() + 
                         self.config.prompts.len() + 
                         self.config.global_rules.len()) as u32,
            active_rules: (self.config.tools.len() + 
                          self.config.resources.len() + 
                          self.config.prompts.len() + 
                          self.config.global_rules.len()) as u32, // All rules are considered active
            total_requests: stats.total_requests,
            allowed_requests: stats.allowed_requests,
            blocked_requests: stats.blocked_requests,
            approval_required_requests: stats.approval_required_requests,
            top_matched_rules: rule_matches,
            hourly_patterns: stats.hourly_stats,
        }
    }
    
    async fn get_health(&self) -> ServiceHealth {
        let stats = self.stats.lock().unwrap().clone();
        let uptime_seconds = (Utc::now() - stats.start_time).num_seconds() as u64;
        
        let avg_response_time_ms = if stats.total_requests > 0 {
            stats.total_processing_time_ms as f64 / stats.total_requests as f64
        } else {
            0.0
        };
        
        let error_rate = if stats.total_requests > 0 {
            stats.blocked_requests as f64 / stats.total_requests as f64
        } else {
            0.0
        };
        
        let requests_per_second = if uptime_seconds > 0 {
            stats.total_requests as f64 / uptime_seconds as f64
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
        if let Ok(mut stats) = self.stats.lock() {
            *stats = AllowlistStats {
                start_time: Utc::now(),
                total_requests: 0,
                allowed_requests: 0,
                blocked_requests: 0,
                approval_required_requests: 0,
                rule_matches: HashMap::new(),
                last_error: None,
                total_processing_time_ms: 0,
                hourly_stats: Vec::new(),
            };
        }
        Ok(())
    }
}

impl HealthMonitor for AllowlistService {
    async fn is_healthy(&self) -> bool {
        self.config.enabled && self.stats.lock().unwrap().last_error.is_none()
    }
    
    async fn health_check(&self) -> ServiceHealth {
        self.get_health().await
    }
    
    fn get_uptime(&self) -> u64 {
        let stats = self.stats.lock().unwrap();
        (Utc::now() - stats.start_time).num_seconds() as u64
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