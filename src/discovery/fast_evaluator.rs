//! Ultra-Fast Permission Evaluator with Bitmap Operations
//!
//! This module provides the fastest possible tool permission evaluation using
//! CPU-optimized bitmap operations and pre-compiled pattern matching.
//! Target: <100μs per evaluation, <1ms for batch operations.

use crate::discovery::permission_cache::ToolId;
use crate::security::SecurityContext;
use ahash::AHashSet;
use regex::RegexSet;
use std::sync::Arc;
use std::time::Instant;

/// Ultra-fast user context optimized for hot path evaluation
#[derive(Debug, Clone)]
pub struct FastUserContext {
    /// Pre-computed hash of user ID for cache keys
    pub user_id_hash: u64,
    
    /// User ID as shared string to avoid allocations
    pub user_id: Arc<str>,
    
    /// User permissions as 64-bit bitmap for O(1) checking
    pub permissions_bitmap: u64,
    
    /// User roles as pre-computed hash set for fast lookup
    pub roles_set: AHashSet<Arc<str>>,
    
    /// API key permissions (if applicable)
    pub api_key_permissions: u64,
}

impl FastUserContext {
    /// Create fast user context from security context
    pub fn from_security_context(security_context: &SecurityContext) -> Option<Self> {
        let user = security_context.user.as_ref()?;
        let user_id = user.id.as_ref()?;
        
        let user_id_hash = Self::hash_user_id(user_id);
        let user_id_arc: Arc<str> = user_id.clone().into();
        
        // Convert roles to hash set of Arc<str> for fast lookup
        let roles_set = user.roles
            .iter()
            .map(|role| -> Arc<str> { role.clone().into() })
            .collect();
        
        // Calculate permissions bitmap (this would integrate with your permission system)
        let permissions_bitmap = Self::calculate_permissions_bitmap(&roles_set);
        
        Some(Self {
            user_id_hash,
            user_id: user_id_arc,
            permissions_bitmap,
            roles_set,
            api_key_permissions: 0, // TODO: Implement API key permissions
        })
    }
    
    /// Generate consistent hash for user ID
    fn hash_user_id(user_id: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = ahash::AHasher::default();
        user_id.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Calculate permissions bitmap from roles
    /// This is a placeholder - integrate with your actual permission system
    fn calculate_permissions_bitmap(roles: &AHashSet<Arc<str>>) -> u64 {
        let mut bitmap = 0u64;
        
        // Example mapping of roles to permission bits
        for (index, role) in roles.iter().enumerate() {
            if index < 64 {
                bitmap |= 1 << index;
            }
        }
        
        // Add some predefined permission mappings
        if roles.iter().any(|role| role.as_ref() == "admin") {
            bitmap |= 0xFFFFFFFFFFFFFFFF; // Admin gets all permissions
        }
        
        if roles.iter().any(|role| role.as_ref() == "user") {
            bitmap |= 0x00000000000000FF; // User gets first 8 permissions
        }
        
        if roles.iter().any(|role| role.as_ref() == "developer") {
            bitmap |= 0x000000000000FF00; // Developer gets permissions 8-15
        }
        
        bitmap
    }
    
    /// Check if user has a specific permission bit
    pub fn has_permission(&self, permission_bit: u8) -> bool {
        if permission_bit >= 64 {
            return false;
        }
        (self.permissions_bitmap & (1 << permission_bit)) != 0
    }
    
    /// Check if user has any of the specified permission bits
    pub fn has_any_permission(&self, permission_mask: u64) -> bool {
        (self.permissions_bitmap & permission_mask) != 0
    }
    
    /// Check if user has all of the specified permission bits
    pub fn has_all_permissions(&self, permission_mask: u64) -> bool {
        (self.permissions_bitmap & permission_mask) == permission_mask
    }
    
    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles_set.iter().any(|r| r.as_ref() == role)
    }
}

/// Compiled rule for ultra-fast evaluation
#[derive(Debug, Clone)]
pub struct CompiledRule {
    /// Rule ID for debugging
    pub rule_id: String,
    
    /// Type of rule for fast dispatch
    pub rule_type: CompiledRuleType,
    
    /// Required permissions bitmap
    pub required_permissions: u64,
    
    /// Required roles (as pre-computed hash set)
    pub required_roles: AHashSet<Arc<str>>,
    
    /// Action to take if rule matches
    pub action: RuleAction,
    
    /// Priority for rule ordering (higher = more important)
    pub priority: u32,
}

/// Type of compiled rule for fast dispatch
#[derive(Debug, Clone)]
pub enum CompiledRuleType {
    /// Explicit tool name match
    ExactTool { tool_id: Arc<str> },
    
    /// Pre-compiled regex pattern
    Pattern { regex_index: usize },
    
    /// Permission bitmap check
    Permission { permission_mask: u64 },
    
    /// Role-based check
    Role { role: Arc<str> },
    
    /// Combination rule (all conditions must match)
    Combined { conditions: Vec<CompiledRuleType> },
}

/// Action to take when rule matches
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAction {
    Allow,
    Deny,
    Inherit,
}

/// Ultra-fast permission evaluator with bitmap operations
pub struct FastPermissionEvaluator {
    /// Pre-compiled rules for ultra-fast evaluation
    compiled_rules: Vec<CompiledRule>,
    
    /// Pre-compiled regex set for pattern matching
    regex_set: Option<RegexSet>,
    
    /// Explicit allow bitmap (tools always allowed)
    explicit_allow_bitmap: u64,
    
    /// Explicit deny bitmap (tools always denied)
    explicit_deny_bitmap: u64,
    
    /// Default action when no rules match
    default_action: RuleAction,
    
    /// Performance statistics
    stats: EvaluatorStats,
}

/// Performance statistics for the evaluator
#[derive(Debug, Clone, Default)]
pub struct EvaluatorStats {
    /// Total evaluations performed
    pub total_evaluations: u64,
    
    /// Fast path hits (bitmap checks)
    pub fast_path_hits: u64,
    
    /// Slow path hits (regex/complex checks)
    pub slow_path_hits: u64,
    
    /// Average evaluation time in nanoseconds
    pub avg_evaluation_time_ns: u64,
    
    /// Last evaluation time
    pub last_evaluation: Option<Instant>,
}

impl FastPermissionEvaluator {
    /// Create a new fast permission evaluator
    pub fn new(default_action: RuleAction) -> Self {
        Self {
            compiled_rules: Vec::new(),
            regex_set: None,
            explicit_allow_bitmap: 0,
            explicit_deny_bitmap: 0,
            default_action,
            stats: EvaluatorStats::default(),
        }
    }
    
    /// Compile rules for ultra-fast evaluation
    pub fn compile_rules(&mut self, rules: Vec<CompiledRule>) -> Result<(), String> {
        // Sort rules by priority (higher priority first)
        let mut sorted_rules = rules;
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // Extract regex patterns for compilation
        let mut patterns = Vec::new();
        for rule in &sorted_rules {
            if let CompiledRuleType::Pattern { .. } = rule.rule_type {
                // In a real implementation, you'd extract the pattern string
                // For now, we'll use placeholder patterns
                patterns.push(".*".to_string());
            }
        }
        
        // Compile regex set if we have patterns
        self.regex_set = if patterns.is_empty() {
            None
        } else {
            Some(RegexSet::new(&patterns).map_err(|e| format!("Regex compilation failed: {}", e))?)
        };
        
        self.compiled_rules = sorted_rules;
        Ok(())
    }
    
    /// Ultra-fast tool permission check
    pub fn is_tool_allowed(&mut self, user_context: &FastUserContext, tool_id: &ToolId) -> EvaluationResult {
        let start_time = Instant::now();
        
        // Fast path 1: Check explicit allow bitmap
        if (user_context.permissions_bitmap & self.explicit_allow_bitmap) != 0 {
            self.stats.fast_path_hits += 1;
            return self.finish_evaluation(start_time, EvaluationResult::new(true, "explicit_allow_bitmap"));
        }
        
        // Fast path 2: Check explicit deny bitmap
        if (user_context.permissions_bitmap & self.explicit_deny_bitmap) != 0 {
            self.stats.fast_path_hits += 1;
            return self.finish_evaluation(start_time, EvaluationResult::new(false, "explicit_deny_bitmap"));
        }
        
        // Fast path 3: Check compiled rules
        for rule in &self.compiled_rules {
            if let Some(result) = self.evaluate_rule_fast(rule, user_context, tool_id) {
                match result.action {
                    RuleAction::Allow => {
                        self.stats.fast_path_hits += 1;
                        return self.finish_evaluation(start_time, EvaluationResult::new(true, &rule.rule_id));
                    }
                    RuleAction::Deny => {
                        self.stats.fast_path_hits += 1;
                        return self.finish_evaluation(start_time, EvaluationResult::new(false, &rule.rule_id));
                    }
                    RuleAction::Inherit => {
                        // Continue to next rule
                        continue;
                    }
                }
            }
        }
        
        // Slow path: Pattern matching
        if let Some(ref regex_set) = self.regex_set {
            if regex_set.is_match(tool_id) {
                self.stats.slow_path_hits += 1;
                return self.finish_evaluation(start_time, EvaluationResult::new(true, "pattern_match"));
            }
        }
        
        // Default action
        let allowed = matches!(self.default_action, RuleAction::Allow);
        self.finish_evaluation(start_time, EvaluationResult::new(allowed, "default"))
    }
    
    /// Fast rule evaluation using bitmap operations
    fn evaluate_rule_fast(&self, rule: &CompiledRule, user_context: &FastUserContext, tool_id: &ToolId) -> Option<RuleEvaluationResult> {
        match &rule.rule_type {
            CompiledRuleType::ExactTool { tool_id: rule_tool_id } => {
                if tool_id == rule_tool_id.as_ref() {
                    Some(RuleEvaluationResult { action: rule.action })
                } else {
                    None
                }
            }
            
            CompiledRuleType::Permission { permission_mask } => {
                if user_context.has_any_permission(*permission_mask) {
                    Some(RuleEvaluationResult { action: rule.action })
                } else {
                    None
                }
            }
            
            CompiledRuleType::Role { role } => {
                if user_context.has_role(role.as_ref()) {
                    Some(RuleEvaluationResult { action: rule.action })
                } else {
                    None
                }
            }
            
            CompiledRuleType::Pattern { .. } => {
                // Pattern matching is handled in slow path
                None
            }
            
            CompiledRuleType::Combined { conditions } => {
                // Check all conditions - if any fail, rule doesn't match
                for condition in conditions {
                    if self.evaluate_rule_fast(&CompiledRule {
                        rule_id: rule.rule_id.clone(),
                        rule_type: condition.clone(),
                        required_permissions: rule.required_permissions,
                        required_roles: rule.required_roles.clone(),
                        action: rule.action,
                        priority: rule.priority,
                    }, user_context, tool_id).is_none() {
                        return None;
                    }
                }
                Some(RuleEvaluationResult { action: rule.action })
            }
        }
    }
    
    /// Finish evaluation and update statistics
    fn finish_evaluation(&mut self, start_time: Instant, result: EvaluationResult) -> EvaluationResult {
        let elapsed = start_time.elapsed();
        
        self.stats.total_evaluations += 1;
        self.stats.last_evaluation = Some(start_time);
        
        // Update rolling average
        let elapsed_ns = elapsed.as_nanos() as u64;
        if self.stats.total_evaluations == 1 {
            self.stats.avg_evaluation_time_ns = elapsed_ns;
        } else {
            // Simple moving average
            self.stats.avg_evaluation_time_ns = 
                (self.stats.avg_evaluation_time_ns * (self.stats.total_evaluations - 1) + elapsed_ns) 
                / self.stats.total_evaluations;
        }
        
        result
    }
    
    /// Batch evaluate multiple tools for a user
    pub fn batch_evaluate(&mut self, user_context: &FastUserContext, tool_ids: &[ToolId]) -> Vec<(ToolId, bool)> {
        tool_ids.iter()
            .map(|tool_id| {
                let result = self.is_tool_allowed(user_context, tool_id);
                (tool_id.clone(), result.allowed)
            })
            .collect()
    }
    
    /// Filter a list of tools to only those the user can access
    pub fn filter_allowed_tools(&mut self, user_context: &FastUserContext, tool_ids: &[ToolId]) -> Vec<ToolId> {
        tool_ids.iter()
            .filter(|tool_id| {
                let result = self.is_tool_allowed(user_context, tool_id);
                result.allowed
            })
            .cloned()
            .collect()
    }
    
    /// Get performance statistics
    pub fn get_stats(&self) -> &EvaluatorStats {
        &self.stats
    }
    
    /// Reset performance statistics
    pub fn reset_stats(&mut self) {
        self.stats = EvaluatorStats::default();
    }
}

/// Result of a rule evaluation
#[derive(Debug, Clone)]
struct RuleEvaluationResult {
    action: RuleAction,
}

/// Final evaluation result with metadata
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    /// Whether the tool is allowed
    pub allowed: bool,
    
    /// Reason for the decision (rule ID or reason string)
    pub reason: String,
    
    /// Evaluation time in nanoseconds
    pub evaluation_time_ns: Option<u64>,
}

impl EvaluationResult {
    /// Create a new evaluation result
    pub fn new(allowed: bool, reason: &str) -> Self {
        Self {
            allowed,
            reason: reason.to_string(),
            evaluation_time_ns: None,
        }
    }
    
    /// Add timing information
    pub fn with_timing(mut self, duration: std::time::Duration) -> Self {
        self.evaluation_time_ns = Some(duration.as_nanos() as u64);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{SecurityUser, SecurityContext};
    use std::collections::HashMap;
    
    fn create_test_security_context(user_id: &str, roles: Vec<String>) -> SecurityContext {
        SecurityContext {
            user: Some(SecurityUser {
                id: Some(user_id.to_string()),
                roles,
                api_key_name: None,
                permissions: vec![],
                auth_method: "test".to_string(),
            }),
            request: crate::security::SecurityRequest {
                id: "req-123".to_string(),
                method: "POST".to_string(),
                path: "/test".to_string(),
                client_ip: Some("127.0.0.1".to_string()),
                user_agent: Some("test".to_string()),
                headers: std::collections::HashMap::new(),
                body: None,
                timestamp: chrono::Utc::now(),
            },
            tool: None,
            resource: None,
            metadata: std::collections::HashMap::new(),
        }
    }
    
    #[test]
    fn test_fast_user_context_creation() {
        let security_context = create_test_security_context("test_user", vec!["admin".to_string()]);
        let fast_context = FastUserContext::from_security_context(&security_context).unwrap();
        
        assert_eq!(fast_context.user_id.as_ref(), "test_user");
        assert!(fast_context.has_role("admin"));
        assert!(fast_context.permissions_bitmap != 0); // Admin should have permissions
    }
    
    #[test]
    fn test_permission_bitmap_operations() {
        let security_context = create_test_security_context("test_user", vec!["user".to_string()]);
        let fast_context = FastUserContext::from_security_context(&security_context).unwrap();
        
        // User role should give permissions 0-7
        assert!(fast_context.has_permission(0));
        assert!(fast_context.has_permission(7));
        assert!(!fast_context.has_permission(8));
    }
    
    #[test]
    fn test_fast_permission_evaluator() {
        let mut evaluator = FastPermissionEvaluator::new(RuleAction::Deny);
        
        let security_context = create_test_security_context("test_user", vec!["admin".to_string()]);
        let fast_context = FastUserContext::from_security_context(&security_context).unwrap();
        
        let result = evaluator.is_tool_allowed(&fast_context, &"test_tool".to_string());
        
        // Should fall back to default (deny)
        assert!(!result.allowed);
        assert_eq!(result.reason, "default");
    }
    
    #[test]
    fn test_batch_evaluation() {
        let mut evaluator = FastPermissionEvaluator::new(RuleAction::Allow);
        
        let security_context = create_test_security_context("test_user", vec!["user".to_string()]);
        let fast_context = FastUserContext::from_security_context(&security_context).unwrap();
        
        let tools = vec!["tool1".to_string(), "tool2".to_string(), "tool3".to_string()];
        let results = evaluator.batch_evaluate(&fast_context, &tools);
        
        assert_eq!(results.len(), 3);
        // All should be allowed with default Allow action
        assert!(results.iter().all(|(_, allowed)| *allowed));
    }
}