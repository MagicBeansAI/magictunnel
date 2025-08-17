//! Shared types for MagicTunnel enterprise allowlist system
//!
//! Contains configuration structures, enums, and data types used across
//! the allowlist implementation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};

/// Enterprise allowlist configuration with tool-first architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistConfig {
    /// Whether allowlisting is enabled
    pub enabled: bool,
    /// Default action when no rule matches: allow or deny
    pub default_action: AllowlistAction,
    /// Emergency lockdown state (highest priority)
    pub emergency_lockdown: bool,
    /// Tool-specific allowlist rules (embedded in tool files)
    pub tools: HashMap<String, AllowlistRule>,
    /// Server/File-level rules
    pub servers: HashMap<String, AllowlistRule>,
    /// Capability-level pattern rules (auto-apply to new tools)
    pub capability_patterns: Vec<PatternRule>,
    /// Global-level pattern rules (ultimate fallback)
    pub global_patterns: Vec<PatternRule>,
}

impl Default for AllowlistConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_action: AllowlistAction::Allow,
            emergency_lockdown: false,
            tools: HashMap::new(),
            servers: HashMap::new(),
            capability_patterns: Vec::new(),
            global_patterns: Vec::new(),
        }
    }
}

/// Action to take for allowlist decisions (simplified binary system)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AllowlistAction {
    Allow,
    Deny,
}

/// Unified allowlist rule structure for all levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistRule {
    /// Action to take: allow or deny
    pub action: AllowlistAction,
    /// Optional explanation for the rule
    pub reason: Option<String>,
    /// Pattern matching (only used at capability/global levels)
    pub pattern: Option<AllowlistPattern>,
    /// Rule priority (only used at capability/global levels)
    pub priority: Option<i32>,
    /// Rule identifier
    pub name: Option<String>,
    /// Whether rule is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Pattern rule for capability/global levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRule {
    /// Base allowlist rule
    #[serde(flatten)]
    pub rule: AllowlistRule,
}

/// Pattern matching for allowlist rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AllowlistPattern {
    /// Regular expression (preferred for performance)
    Regex { value: String },
    /// Wildcard pattern (*, ?) - converted to regex internally
    Wildcard { value: String },
    /// Exact string match - converted to regex internally
    Exact { value: String },
}

/// Rule evaluation levels in priority order
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleLevel {
    Emergency = 0,
    Tool = 1,
    Server = 2,
    Capability = 3,
    Global = 4,
    Default = 5,
}

/// High-performance result of allowlist evaluation
#[derive(Debug, Clone)]
pub struct AllowlistResult {
    /// Whether access is allowed
    pub allowed: bool,
    /// Action taken
    pub action: AllowlistAction,
    /// Rule that matched (if any)
    pub matched_rule: Option<String>,
    /// Reason for the decision (shared string for memory efficiency)
    pub reason: Arc<str>,
    /// Rule level that made the decision
    pub rule_level: RuleLevel,
    /// Decision time in nanoseconds (for performance tracking)
    pub decision_time_ns: u64,
    /// Whether approval is required (legacy)
    pub requires_approval: bool,
}

impl AllowlistResult {
    /// Fast deny result for hot path
    pub fn deny_fast(reason: &'static str, level: RuleLevel) -> Self {
        Self {
            allowed: false,
            action: AllowlistAction::Deny,
            matched_rule: None,
            reason: Arc::from(reason),
            rule_level: level,
            decision_time_ns: 0,
            requires_approval: false,
        }
    }
    
    /// Fast allow result for hot path
    pub fn allow_fast(reason: &'static str, level: RuleLevel) -> Self {
        Self {
            allowed: true,
            action: AllowlistAction::Allow,
            matched_rule: None,
            reason: Arc::from(reason),
            rule_level: level,
            decision_time_ns: 0,
            requires_approval: false,
        }
    }
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