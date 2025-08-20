//! Enhanced allowlist data structures for the new architecture
//! Separates configuration from runtime data with pre-computed decisions

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use super::allowlist_types::{AllowlistAction};

/// Main allowlist data file structure  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistData {
    pub metadata: AllowlistMetadata,
    pub patterns: AllowlistPatterns,
    pub explicit_rules: ExplicitRules,
}

/// Metadata for the allowlist data file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistMetadata {
    pub version: String,
    pub last_updated: DateTime<Utc>,
    pub total_patterns: u32,
    pub total_explicit_rules: u32,
}

/// All pattern-based rules organized by scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistPatterns {
    pub global: Vec<PatternRule>,
    pub tools: Vec<PatternRule>,
    pub capabilities: Vec<PatternRule>,
}

/// Individual pattern rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRule {
    pub name: String,
    pub regex: String,
    pub action: AllowlistAction,
    pub reason: String,
    pub enabled: bool,
}

/// Explicit allow/deny rules for specific items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitRules {
    pub tools: HashMap<String, AllowlistAction>,
    pub capabilities: HashMap<String, AllowlistAction>,
}

/// Pre-computed decision for a specific tool/capability/server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistDecision {
    pub action: AllowlistAction,
    pub rule_source: RuleSource,
    pub rule_name: String,
    pub reason: String,
    pub confidence: f32,
    pub evaluated_at: DateTime<Utc>,
}

/// Source of the allowlist decision for audit trail
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuleSource {
    ExplicitTool,
    ExplicitCapability,
    ToolPattern,
    CapabilityPattern,
    GlobalPattern,
    DefaultAction,
    EmergencyLockdown,
}

/// Audit trail showing how a decision was made
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionAuditTrail {
    pub tool_name: String,
    pub final_decision: AllowlistAction,
    pub rule_source: RuleSource,
    pub rule_name: String,
    pub evaluation_chain: Vec<RuleEvaluation>,
    pub timestamp: DateTime<Utc>,
}

/// Individual step in the decision evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluation {
    pub step: u8,
    pub rule_type: String,
    pub rule_name: Option<String>,
    pub result: EvaluationResult,
    pub reason: Option<String>,
    pub continue_evaluation: bool,
}

/// Result of evaluating a single rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvaluationResult {
    Allow,
    Deny,
    NoMatch,
    NotActive,
    Skip,
}

/// Tool with its allowlist status for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolWithAllowlistStatus {
    pub name: String,
    pub capability: String,
    pub server: String,
    pub allowlist_decision: AllowlistDecision,
    pub audit_available: bool,
}

/// API request for real-time pattern testing
#[derive(Debug, Deserialize)]
pub struct PatternTestRequest {
    pub patterns: AllowlistPatterns,
    pub explicit_rules: ExplicitRules,
    pub test_tools: Vec<String>,
}

/// API response for pattern testing
#[derive(Debug, Serialize)]
pub struct PatternTestResponse {
    pub results: HashMap<String, AllowlistDecision>,
    pub pattern_matches: HashMap<String, Vec<String>>, // pattern_name -> matched_tools
    pub summary: PatternTestSummary,
}

/// Summary of pattern test results
#[derive(Debug, Serialize)]
pub struct PatternTestSummary {
    pub total_tools_tested: u32,
    pub allowed_count: u32,
    pub denied_count: u32,
    pub explicit_rules_applied: u32,
    pub pattern_rules_applied: u32,
    pub default_action_applied: u32,
}

/// Summary statistics for all precomputed allowlist decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistSummary {
    pub total_tools: usize,
    pub allowed_tools: usize,
    pub denied_tools: usize,
    pub explicit_rules: usize,
    pub tool_patterns: usize,
    pub capability_patterns: usize,
    pub global_patterns: usize,
    pub default_actions: usize,
    pub emergency_lockdown: usize,
}

impl Default for AllowlistData {
    fn default() -> Self {
        Self {
            metadata: AllowlistMetadata {
                version: "1.0.0".to_string(),
                last_updated: Utc::now(),
                total_patterns: 0,
                total_explicit_rules: 0,
            },
            patterns: AllowlistPatterns {
                global: Vec::new(),
                tools: Vec::new(),
                capabilities: Vec::new(),
            },
            explicit_rules: ExplicitRules {
                tools: HashMap::new(),
                capabilities: HashMap::new(),
            },
        }
    }
}

impl AllowlistDecision {
    pub fn new(action: AllowlistAction, rule_source: RuleSource, rule_name: String, reason: String) -> Self {
        Self {
            action,
            rule_source,
            rule_name,
            reason,
            confidence: 1.0,
            evaluated_at: Utc::now(),
        }
    }
    
    pub fn allow(rule_source: RuleSource, rule_name: String, reason: String) -> Self {
        Self {
            action: AllowlistAction::Allow,
            rule_source,
            rule_name,
            reason,
            confidence: 1.0,
            evaluated_at: Utc::now(),
        }
    }
    
    pub fn deny(rule_source: RuleSource, rule_name: String, reason: String) -> Self {
        Self {
            action: AllowlistAction::Deny,
            rule_source,
            rule_name,
            reason,
            confidence: 1.0,
            evaluated_at: Utc::now(),
        }
    }
    
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }
}

impl std::fmt::Display for RuleSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleSource::ExplicitTool => write!(f, "explicit_tool"),
            RuleSource::ExplicitCapability => write!(f, "explicit_capability"),
            RuleSource::ToolPattern => write!(f, "tool_pattern"),
            RuleSource::CapabilityPattern => write!(f, "capability_pattern"),
            RuleSource::GlobalPattern => write!(f, "global_pattern"),
            RuleSource::DefaultAction => write!(f, "default_action"),
            RuleSource::EmergencyLockdown => write!(f, "emergency_lockdown"),
        }
    }
}

// ============================================================================
// Real-time Pattern Testing API Types
// ============================================================================

/// Request to test a single pattern in real-time without affecting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimePatternTestRequest {
    /// Pattern to test
    pub pattern: TestPattern,
    /// Tool names to test the pattern against 
    pub test_tools: Vec<String>,
    /// Include full evaluation chain in response
    pub include_evaluation_chain: bool,
}

/// Pattern definition for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPattern {
    /// Pattern name for identification
    pub name: String,
    /// Regular expression pattern
    pub regex: String,
    /// Action to take if pattern matches
    pub action: AllowlistAction,
    /// Pattern scope (global, tools, capabilities)
    pub scope: PatternScope,
    /// Pattern priority (lower = higher priority)
    pub priority: u8,
}

/// Pattern scope for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternScope {
    Global,
    Tools,
    Capabilities,
}

/// Response with pattern test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimePatternTestResponse {
    /// Pattern that was tested
    pub pattern: TestPattern,
    /// Results for each tested tool
    pub tool_results: Vec<PatternToolTestResult>,
    /// Summary statistics
    pub summary: RealTimePatternTestSummary,
    /// Any validation errors with the pattern
    pub validation_errors: Vec<String>,
}

/// Result of testing a pattern against a specific tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternToolTestResult {
    /// Tool name that was tested
    pub tool_name: String,
    /// Whether the pattern matched
    pub pattern_matched: bool,
    /// Final decision after testing the pattern
    pub final_decision: AllowlistAction,
    /// Rule source that made the final decision
    pub rule_source: RuleSource,
    /// Rule name that made the final decision
    pub rule_name: String,
    /// Explanation of the decision
    pub reason: String,
    /// Whether this pattern would change the current decision
    pub decision_would_change: bool,
    /// Current decision without this pattern
    pub current_decision: AllowlistAction,
    /// Full evaluation chain if requested
    pub evaluation_chain: Option<Vec<PatternEvaluationStep>>,
}

/// Individual step in pattern test evaluation chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEvaluationStep {
    /// Step number in evaluation
    pub step: u32,
    /// Type of rule being evaluated
    pub rule_type: String,
    /// Name of the specific rule
    pub rule_name: Option<String>,
    /// Result of this evaluation step (Allow/Deny/NoMatch)
    pub result: EvaluationResult,
    /// Reason for this step's result
    pub reason: Option<String>,
    /// Whether evaluation continues after this step
    pub continue_evaluation: bool,
}


/// Summary statistics for pattern test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimePatternTestSummary {
    /// Total tools tested
    pub total_tools: usize,
    /// Tools where pattern matched
    pub pattern_matches: usize,
    /// Tools where decision would change with this pattern
    pub decisions_changed: usize,
    /// Tools that would be allowed with this pattern
    pub would_allow: usize,
    /// Tools that would be denied with this pattern
    pub would_deny: usize,
    /// Pattern validation successful
    pub pattern_valid: bool,
}

// ============================================================================
// Treeview API Response Types
// ============================================================================

/// Hierarchical treeview response for allowlist status organized by server/capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistTreeviewResponse {
    /// Root level servers in the treeview
    pub servers: Vec<TreeviewServerNode>,
    /// Total number of tools
    pub total_tools: usize,
    /// Number of allowed tools
    pub allowed_tools: usize,
    /// Number of denied tools
    pub denied_tools: usize,
    /// Generation timestamp
    pub generated_at: DateTime<Utc>,
}

/// Server-level node in the treeview hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeviewServerNode {
    /// Server name (e.g., "filesystem", "github")
    pub name: String,
    /// Server-level allowlist status
    pub status: TreeviewNodeStatus,
    /// Child capabilities under this server
    pub capabilities: Vec<TreeviewCapabilityNode>,
    /// Total tools in this server
    pub tool_count: usize,
    /// Allowed tools in this server
    pub allowed_count: usize,
    /// Denied tools in this server
    pub denied_count: usize,
}

/// Capability-level node in the treeview hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeviewCapabilityNode {
    /// Capability name (e.g., "basic", "advanced")
    pub name: String,
    /// Capability-level allowlist status
    pub status: TreeviewNodeStatus,
    /// Individual tools under this capability
    pub tools: Vec<TreeviewToolNode>,
    /// Total tools in this capability
    pub tool_count: usize,
    /// Allowed tools in this capability
    pub allowed_count: usize,
    /// Denied tools in this capability
    pub denied_count: usize,
}

/// Tool-level node in the treeview hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeviewToolNode {
    /// Tool name
    pub name: String,
    /// Tool allowlist decision
    pub status: TreeviewNodeStatus,
    /// Source of the decision (explicit rule, pattern, default, etc.)
    pub decision_source: String,
    /// Explanation of why this decision was made
    pub reason: String,
    /// Whether this tool has an explicit rule
    pub has_explicit_rule: bool,
    /// Priority of rule that made the decision
    pub rule_priority: Option<i32>,
}

/// Status/decision for a node in the treeview
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TreeviewNodeStatus {
    /// All tools under this node are allowed
    Allowed,
    /// All tools under this node are denied
    Denied,
    /// Mix of allowed and denied tools under this node
    Mixed,
    /// Emergency lockdown is active (highest priority)
    Emergency,
    /// Unknown status (shouldn't happen in practice)
    Unknown,
}