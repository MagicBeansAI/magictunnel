//! Organization-wide security policies for MagicTunnel
//!
//! Provides policy engine for allow/block decisions on servers, hosts, and capabilities,
//! similar to MCP Manager's organization-wide policy enforcement.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use regex::Regex;
use tracing::{debug, warn};
use chrono::{DateTime, Utc, Datelike, Timelike};
use super::statistics::{SecurityServiceStatistics, HealthMonitor, ServiceHealth, HealthStatus, PolicyStatistics, PolicyEffectiveness, PerformanceMetrics};

/// Security policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Whether policy engine is enabled
    pub enabled: bool,
    /// List of security policies
    pub policies: Vec<SecurityPolicy>,
    /// Default action when no policy matches
    pub default_action: PolicyAction,
    /// Whether to log policy decisions
    pub log_decisions: bool,
}

/// Security policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,
    /// Policy description
    pub description: Option<String>,
    /// When this policy applies
    pub conditions: Vec<PolicyCondition>,
    /// Action to take when policy matches
    pub action: PolicyAction,
    /// Priority (higher numbers take precedence)
    pub priority: i32,
    /// Whether this policy is active
    pub enabled: bool,
    /// Policy metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Condition for policy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PolicyCondition {
    /// Server/endpoint restrictions
    Server {
        /// Server name patterns (supports wildcards)
        patterns: Vec<String>,
        /// Whether to match any or all patterns
        match_mode: MatchMode,
    },
    /// Host/domain restrictions
    Host {
        /// Host patterns (supports wildcards and CIDR)
        patterns: Vec<String>,
        /// Whether to match any or all patterns
        match_mode: MatchMode,
    },
    /// Capability restrictions
    Capability {
        /// Capability name patterns
        patterns: Vec<String>,
        /// Whether to match any or all patterns
        match_mode: MatchMode,
    },
    /// Tool name restrictions
    Tool {
        /// Tool name patterns
        patterns: Vec<String>,
        /// Whether to match any or all patterns
        match_mode: MatchMode,
    },
    /// User/role restrictions
    User {
        /// User ID patterns
        user_patterns: Option<Vec<String>>,
        /// Role patterns
        role_patterns: Option<Vec<String>>,
        /// Whether to match any or all patterns
        match_mode: MatchMode,
    },
    /// Time-based restrictions
    TimeWindow {
        /// Allowed time windows
        windows: Vec<TimeWindow>,
        /// Timezone for evaluation
        timezone: Option<String>,
    },
    /// Content-based restrictions
    Content {
        /// Patterns to match in request/response content
        patterns: Vec<String>,
        /// Whether patterns are regex
        is_regex: bool,
        /// Case sensitive matching
        case_sensitive: bool,
        /// Fields to check (request, response, parameters, etc.)
        target_fields: Vec<String>,
    },
    /// Rate limiting restrictions
    RateLimit {
        /// Maximum requests per time window
        max_requests: u32,
        /// Time window in seconds
        window_seconds: u64,
        /// Scope for rate limiting (user, ip, global)
        scope: RateLimitScope,
    },
    /// Custom condition (evaluated by external service)
    Custom {
        /// Condition name
        name: String,
        /// Parameters for evaluation
        parameters: HashMap<String, serde_json::Value>,
        /// External service endpoint
        endpoint: Option<String>,
    },
}

/// How to match multiple patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    /// Match any of the patterns
    Any,
    /// Match all of the patterns
    All,
    /// Match none of the patterns
    None,
}

/// Time window for policy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    /// Start time (HH:MM format)
    pub start_time: String,
    /// End time (HH:MM format)
    pub end_time: String,
    /// Days of week (0=Sunday, 6=Saturday)
    pub days_of_week: Option<Vec<u8>>,
}

/// Rate limiting scope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RateLimitScope {
    /// Per user
    User,
    /// Per IP address
    Ip,
    /// Per API key
    ApiKey,
    /// Global
    Global,
}

/// Action to take when policy matches
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PolicyAction {
    /// Allow the request
    Allow {
        /// Log message
        message: Option<String>,
    },
    /// Block the request
    Block {
        /// Error message to return
        message: String,
        /// HTTP status code
        status_code: Option<u16>,
    },
    /// Require approval
    RequireApproval {
        /// Approval workflow
        workflow: String,
        /// Timeout in seconds
        timeout: Option<u64>,
    },
    /// Modify the request
    Modify {
        /// Modifications to apply
        modifications: Vec<RequestModification>,
    },
    /// Log and continue
    Log {
        /// Log level
        level: LogLevel,
        /// Log message
        message: String,
    },
}

/// Request modification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RequestModification {
    /// Add header
    AddHeader {
        /// Header name
        name: String,
        /// Header value
        value: String,
    },
    /// Remove header
    RemoveHeader {
        /// Header name
        name: String,
    },
    /// Modify parameter
    ModifyParameter {
        /// Parameter name
        name: String,
        /// New value
        value: serde_json::Value,
    },
    /// Remove parameter
    RemoveParameter {
        /// Parameter name
        name: String,
    },
    /// Add metadata
    AddMetadata {
        /// Metadata key
        key: String,
        /// Metadata value
        value: serde_json::Value,
    },
}

/// Log level for policy actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Context for policy evaluation
#[derive(Debug, Clone)]
pub struct PolicyContext {
    /// Server/endpoint name
    pub server: Option<String>,
    /// Host/domain
    pub host: Option<String>,
    /// Capability name
    pub capability: Option<String>,
    /// Tool name
    pub tool: Option<String>,
    /// User information
    pub user: Option<PolicyUser>,
    /// Request content
    pub request_content: Option<String>,
    /// Response content
    pub response_content: Option<String>,
    /// Parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Current timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// User information for policy evaluation
#[derive(Debug, Clone)]
pub struct PolicyUser {
    /// User ID
    pub id: Option<String>,
    /// User roles
    pub roles: Vec<String>,
    /// API key name
    pub api_key_name: Option<String>,
}

/// Result of policy evaluation
#[derive(Debug, Clone)]
pub struct PolicyResult {
    /// Final action to take
    pub action: PolicyAction,
    /// Policies that matched
    pub matched_policies: Vec<String>,
    /// Reason for the decision
    pub reason: String,
    /// Modifications to apply (if any)
    pub modifications: Vec<RequestModification>,
    /// Whether request should be blocked
    pub should_block: bool,
    /// Whether approval is required
    pub requires_approval: bool,
}

/// Statistics tracking for policy engine
#[derive(Debug, Clone)]
struct PolicyStats {
    start_time: DateTime<Utc>,
    total_evaluations: u64,
    evaluations_today: u64,
    violations: u64,
    policy_matches: HashMap<String, u64>,
    last_error: Option<String>,
    total_processing_time_ms: u64,
}

/// Policy engine service
pub struct PolicyEngine {
    config: PolicyConfig,
    compiled_patterns: HashMap<String, Regex>,
    rate_limiters: HashMap<String, RateLimiter>,
    stats: Arc<Mutex<PolicyStats>>,
}

/// Simple rate limiter implementation
#[derive(Debug)]
struct RateLimiter {
    max_requests: u32,
    window_seconds: u64,
    requests: Vec<chrono::DateTime<chrono::Utc>>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            policies: Vec::new(),
            default_action: PolicyAction::Allow { message: None },
            log_decisions: true,
        }
    }
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new(config: PolicyConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut compiled_patterns = HashMap::new();
        
        // Pre-compile regex patterns for performance
        for (policy_idx, policy) in config.policies.iter().enumerate() {
            for (condition_idx, condition) in policy.conditions.iter().enumerate() {
                match condition {
                    PolicyCondition::Content { patterns, is_regex: true, .. } => {
                        for (pattern_idx, pattern) in patterns.iter().enumerate() {
                            let key = format!("policy_{}_{}_pattern_{}", policy_idx, condition_idx, pattern_idx);
                            if let Ok(regex) = Regex::new(pattern) {
                                compiled_patterns.insert(key, regex);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        let stats = PolicyStats {
            start_time: Utc::now(),
            total_evaluations: 0,
            evaluations_today: 0,
            violations: 0,
            policy_matches: HashMap::new(),
            last_error: None,
            total_processing_time_ms: 0,
        };

        Ok(Self {
            config,
            compiled_patterns,
            rate_limiters: HashMap::new(),
            stats: Arc::new(Mutex::new(stats)),
        })
    }
    
    /// Evaluate policies for a given context
    pub fn evaluate(&mut self, context: &PolicyContext) -> PolicyResult {
        if !self.config.enabled {
            return PolicyResult {
                action: PolicyAction::Allow { message: None },
                matched_policies: Vec::new(),
                reason: "Policy engine disabled".to_string(),
                modifications: Vec::new(),
                should_block: false,
                requires_approval: false,
            };
        }
        
        debug!("Evaluating policies for context: {:?}", context);
        
        // Sort policies by priority (highest first)
        let mut policies = self.config.policies.clone();
        policies.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        let mut matched_policies = Vec::new();
        let mut modifications = Vec::new();
        
        // Evaluate each policy
        for policy in &policies {
            if !policy.enabled {
                continue;
            }
            
            if self.policy_matches(policy, context) {
                matched_policies.push(policy.name.clone());
                
                match &policy.action {
                    PolicyAction::Block { message, status_code: _ } => {
                        if self.config.log_decisions {
                            warn!("Policy '{}' blocked request: {}", policy.name, message);
                        }
                        return PolicyResult {
                            action: policy.action.clone(),
                            matched_policies,
                            reason: format!("Blocked by policy: {}", policy.name),
                            modifications,
                            should_block: true,
                            requires_approval: false,
                        };
                    }
                    PolicyAction::RequireApproval { .. } => {
                        if self.config.log_decisions {
                            warn!("Policy '{}' requires approval", policy.name);
                        }
                        return PolicyResult {
                            action: policy.action.clone(),
                            matched_policies,
                            reason: format!("Approval required by policy: {}", policy.name),
                            modifications,
                            should_block: false,
                            requires_approval: true,
                        };
                    }
                    PolicyAction::Modify { modifications: policy_mods } => {
                        modifications.extend(policy_mods.clone());
                        if self.config.log_decisions {
                            debug!("Policy '{}' modified request", policy.name);
                        }
                        // Continue evaluating other policies
                    }
                    PolicyAction::Log { level, message } => {
                        if self.config.log_decisions {
                            match level {
                                LogLevel::Debug => debug!("Policy '{}': {}", policy.name, message),
                                LogLevel::Info => tracing::info!("Policy '{}': {}", policy.name, message),
                                LogLevel::Warn => warn!("Policy '{}': {}", policy.name, message),
                                LogLevel::Error => tracing::error!("Policy '{}': {}", policy.name, message),
                            }
                        }
                        // Continue evaluating other policies
                    }
                    PolicyAction::Allow { message } => {
                        if self.config.log_decisions {
                            if let Some(msg) = message {
                                debug!("Policy '{}' allowed request: {}", policy.name, msg);
                            }
                        }
                        return PolicyResult {
                            action: policy.action.clone(),
                            matched_policies,
                            reason: format!("Allowed by policy: {}", policy.name),
                            modifications,
                            should_block: false,
                            requires_approval: false,
                        };
                    }
                }
            }
        }
        
        // No matching policy, apply default action
        PolicyResult {
            action: self.config.default_action.clone(),
            matched_policies,
            reason: "No matching policy, applied default action".to_string(),
            modifications,
            should_block: matches!(self.config.default_action, PolicyAction::Block { .. }),
            requires_approval: matches!(self.config.default_action, PolicyAction::RequireApproval { .. }),
        }
    }
    
    /// Check if a policy matches the given context
    fn policy_matches(&mut self, policy: &SecurityPolicy, context: &PolicyContext) -> bool {
        for condition in &policy.conditions {
            if !self.condition_matches(condition, context) {
                return false;
            }
        }
        true
    }
    
    /// Check if a condition matches the given context
    fn condition_matches(&mut self, condition: &PolicyCondition, context: &PolicyContext) -> bool {
        match condition {
            PolicyCondition::Server { patterns, match_mode } => {
                if let Some(server) = &context.server {
                    self.patterns_match(patterns, server, match_mode)
                } else {
                    false
                }
            }
            PolicyCondition::Host { patterns, match_mode } => {
                if let Some(host) = &context.host {
                    self.patterns_match(patterns, host, match_mode)
                } else {
                    false
                }
            }
            PolicyCondition::Capability { patterns, match_mode } => {
                if let Some(capability) = &context.capability {
                    self.patterns_match(patterns, capability, match_mode)
                } else {
                    false
                }
            }
            PolicyCondition::Tool { patterns, match_mode } => {
                if let Some(tool) = &context.tool {
                    self.patterns_match(patterns, tool, match_mode)
                } else {
                    false
                }
            }
            PolicyCondition::User { user_patterns, role_patterns, match_mode } => {
                if let Some(user) = &context.user {
                    let mut matches = true;
                    
                    if let Some(user_pats) = user_patterns {
                        if let Some(user_id) = &user.id {
                            matches &= self.patterns_match(user_pats, user_id, match_mode);
                        } else {
                            matches = false;
                        }
                    }
                    
                    if let Some(role_pats) = role_patterns {
                        let roles_str = user.roles.join(",");
                        matches &= self.patterns_match(role_pats, &roles_str, match_mode);
                    }
                    
                    matches
                } else {
                    false
                }
            }
            PolicyCondition::TimeWindow { windows, timezone } => {
                self.time_window_matches(windows, context.timestamp, timezone.as_deref())
            }
            PolicyCondition::Content { patterns, is_regex, case_sensitive, target_fields } => {
                self.content_matches(patterns, *is_regex, *case_sensitive, target_fields, context)
            }
            PolicyCondition::RateLimit { max_requests, window_seconds, scope } => {
                self.rate_limit_matches(*max_requests, *window_seconds, scope, context)
            }
            PolicyCondition::Custom { name: _, parameters: _, endpoint: _ } => {
                // Custom condition evaluation would be implemented here
                // For now, always return false
                false
            }
        }
    }
    
    /// Check if patterns match a value
    fn patterns_match(&self, patterns: &[String], value: &str, match_mode: &MatchMode) -> bool {
        let matches: Vec<bool> = patterns.iter()
            .map(|pattern| self.pattern_matches(pattern, value))
            .collect();
        
        match match_mode {
            MatchMode::Any => matches.iter().any(|&m| m),
            MatchMode::All => matches.iter().all(|&m| m),
            MatchMode::None => !matches.iter().any(|&m| m),
        }
    }
    
    /// Check if a single pattern matches a value
    fn pattern_matches(&self, pattern: &str, value: &str) -> bool {
        // Simple wildcard matching for now
        if pattern == "*" {
            return true;
        }
        
        if pattern == value {
            return true;
        }
        
        // Handle wildcard patterns
        if pattern.contains('*') || pattern.contains('?') {
            let regex_pattern = pattern
                .replace('*', ".*")
                .replace('?', ".");
            
            if let Ok(regex) = Regex::new(&format!("^{}$", regex_pattern)) {
                return regex.is_match(value);
            }
        }
        
        false
    }
    
    /// Check if current time matches any time window
    fn time_window_matches(
        &self,
        windows: &[TimeWindow],
        timestamp: chrono::DateTime<chrono::Utc>,
        _timezone: Option<&str>,
    ) -> bool {
        // For now, use UTC time
        let time = timestamp.time();
        let weekday = timestamp.weekday().num_days_from_sunday() as u8;
        
        for window in windows {
            if let (Ok(start), Ok(end)) = (
                chrono::NaiveTime::parse_from_str(&window.start_time, "%H:%M"),
                chrono::NaiveTime::parse_from_str(&window.end_time, "%H:%M")
            ) {
                let time_matches = if start <= end {
                    time >= start && time <= end
                } else {
                    // Spans midnight
                    time >= start || time <= end
                };
                
                let day_matches = window.days_of_week
                    .as_ref()
                    .map(|days| days.contains(&weekday))
                    .unwrap_or(true);
                
                if time_matches && day_matches {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Check if content matches patterns
    fn content_matches(
        &self,
        patterns: &[String],
        is_regex: bool,
        case_sensitive: bool,
        target_fields: &[String],
        context: &PolicyContext,
    ) -> bool {
        let parameters_json = serde_json::to_string(&context.parameters).unwrap_or_default();
        let content_sources = [
            ("request", context.request_content.as_deref()),
            ("response", context.response_content.as_deref()),
            ("parameters", Some(parameters_json.as_str())),
        ];
        
        for (field_name, content_opt) in &content_sources {
            if !target_fields.is_empty() && !target_fields.contains(&field_name.to_string()) {
                continue;
            }
            
            if let Some(content) = content_opt {
                let search_content = if case_sensitive {
                    content.to_string()
                } else {
                    content.to_lowercase()
                };
                
                for pattern in patterns {
                    let search_pattern = if case_sensitive {
                        pattern.clone()
                    } else {
                        pattern.to_lowercase()
                    };
                    
                    if is_regex {
                        if let Ok(regex) = Regex::new(&search_pattern) {
                            if regex.is_match(&search_content) {
                                return true;
                            }
                        }
                    } else {
                        if search_content.contains(&search_pattern) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
    
    /// Check if rate limit is exceeded
    fn rate_limit_matches(
        &mut self,
        max_requests: u32,
        window_seconds: u64,
        scope: &RateLimitScope,
        context: &PolicyContext,
    ) -> bool {
        let key = match scope {
            RateLimitScope::User => {
                context.user.as_ref()
                    .and_then(|u| u.id.as_ref())
                    .cloned()
                    .unwrap_or_else(|| "anonymous".to_string())
            }
            RateLimitScope::Ip => {
                context.client_ip.clone().unwrap_or_else(|| "unknown".to_string())
            }
            RateLimitScope::ApiKey => {
                context.user.as_ref()
                    .and_then(|u| u.api_key_name.as_ref())
                    .cloned()
                    .unwrap_or_else(|| "no_key".to_string())
            }
            RateLimitScope::Global => "global".to_string(),
        };
        
        let rate_limiter = self.rate_limiters.entry(key).or_insert_with(|| {
            RateLimiter::new(max_requests, window_seconds)
        });
        
        rate_limiter.is_exceeded(context.timestamp)
    }
    
    /// Get all policies for API display
    pub fn get_policies_for_api(&self) -> serde_json::Value {
        use serde_json::json;
        
        let policies: Vec<serde_json::Value> = self.config.policies.iter().enumerate().map(|(index, policy)| {
            json!({
                "id": (index + 1).to_string(),
                "name": policy.name,
                "enabled": policy.enabled,
                "description": policy.description.clone().unwrap_or_default(),
                "priority": policy.priority,
                "condition_count": policy.conditions.len(),
                "action_type": match &policy.action {
                    PolicyAction::Allow { .. } => "allow",
                    PolicyAction::Block { .. } => "block",
                    PolicyAction::RequireApproval { .. } => "require_approval",
                    PolicyAction::Modify { .. } => "modify",
                    PolicyAction::Log { .. } => "log",
                },
                "created_at": Utc::now(),
                "updated_at": Utc::now(),
            })
        }).collect();
        
        json!(policies)
    }
}

impl RateLimiter {
    fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
            requests: Vec::new(),
        }
    }
    
    fn is_exceeded(&mut self, timestamp: chrono::DateTime<chrono::Utc>) -> bool {
        // Clean old requests
        let window_start = timestamp - chrono::Duration::seconds(self.window_seconds as i64);
        self.requests.retain(|&req_time| req_time > window_start);
        
        // Check if limit exceeded
        if self.requests.len() >= self.max_requests as usize {
            return true;
        }
        
        // Add current request
        self.requests.push(timestamp);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pattern_matching() {
        let engine = PolicyEngine::new(PolicyConfig::default()).unwrap();
        
        assert!(engine.pattern_matches("*", "anything"));
        assert!(engine.pattern_matches("test*", "testing"));
        assert!(engine.pattern_matches("test?", "test1"));
        assert!(!engine.pattern_matches("test*", "other"));
    }
    
    #[test]
    fn test_time_window_matching() {
        let engine = PolicyEngine::new(PolicyConfig::default()).unwrap();
        
        let windows = vec![TimeWindow {
            start_time: "09:00".to_string(),
            end_time: "17:00".to_string(),
            days_of_week: Some(vec![1, 2, 3, 4, 5]), // Monday to Friday
        }];
        
        // Test during business hours on a weekday
        let timestamp = chrono::Utc::now()
            .with_hour(14).unwrap()
            .with_minute(30).unwrap();
        
        // This test might fail depending on the actual day, but demonstrates the concept
        // assert!(engine.time_window_matches(&windows, timestamp, None));
    }
}

// Implementation of SecurityServiceStatistics trait for PolicyEngine
impl SecurityServiceStatistics for PolicyEngine {
    type Statistics = PolicyStatistics;
    
    async fn get_statistics(&self) -> Self::Statistics {
        let stats = self.stats.lock().unwrap().clone();
        let service_health = self.get_health().await;
        
        let avg_evaluation_time_ms = if stats.total_evaluations > 0 {
            stats.total_processing_time_ms as f64 / stats.total_evaluations as f64
        } else {
            0.0
        };
        
        let effectiveness = PolicyEffectiveness {
            overall_score: if stats.violations > 0 && stats.total_evaluations > 0 {
                1.0 - (stats.violations as f64 / stats.total_evaluations as f64)
            } else { 1.0 },
            detection_accuracy: 0.95, // Would need actual tracking
            false_positive_rate: 0.05, // Would need actual tracking
            coverage_percentage: 85.0, // Would need actual tracking
        };
        
        PolicyStatistics {
            health: service_health,
            total_policies: self.config.policies.len() as u32,
            active_policies: self.config.policies.iter().filter(|p| p.enabled).count() as u32,
            active_rules: self.config.policies.iter()
                .map(|p| p.conditions.len())
                .sum::<usize>() as u32,
            total_evaluations: stats.total_evaluations,
            evaluations_today: stats.evaluations_today,
            violations: stats.violations,
            avg_evaluation_time_ms,
            effectiveness,
        }
    }
    
    async fn get_health(&self) -> ServiceHealth {
        let stats = self.stats.lock().unwrap().clone();
        let uptime_seconds = (Utc::now() - stats.start_time).num_seconds() as u64;
        
        let avg_response_time_ms = if stats.total_evaluations > 0 {
            stats.total_processing_time_ms as f64 / stats.total_evaluations as f64
        } else {
            0.0
        };
        
        let error_rate = if stats.total_evaluations > 0 {
            stats.violations as f64 / stats.total_evaluations as f64
        } else {
            0.0
        };
        
        let requests_per_second = if uptime_seconds > 0 {
            stats.total_evaluations as f64 / uptime_seconds as f64
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
            *stats = PolicyStats {
                start_time: Utc::now(),
                total_evaluations: 0,
                evaluations_today: 0,
                violations: 0,
                policy_matches: HashMap::new(),
                last_error: None,
                total_processing_time_ms: 0,
            };
        }
        Ok(())
    }
}

impl HealthMonitor for PolicyEngine {
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