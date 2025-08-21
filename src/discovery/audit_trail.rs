//! Discovery Audit Trail for Transparency and Debugging
//!
//! This module provides comprehensive audit trails for smart discovery operations,
//! showing which tools were considered, excluded, and why.

use crate::discovery::permission_cache::ToolId;
use crate::security::SecurityContext;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Comprehensive audit trail for a discovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryAuditTrail {
    /// Request ID for correlation
    pub request_id: String,
    
    /// Original user request
    pub user_request: String,
    
    /// User context for this discovery
    pub user_context: AuditUserContext,
    
    /// Total tools available in registry
    pub total_tools_available: usize,
    
    /// Tools excluded by allowlist rules
    pub tools_excluded_by_allowlist: Vec<ExcludedTool>,
    
    /// Tools excluded by RBAC rules
    pub tools_excluded_by_rbac: Vec<ExcludedTool>,
    
    /// Tools that passed filtering and were considered
    pub allowed_tools_considered: Vec<ScoredTool>,
    
    /// Final selected tool (if any)
    pub selected_tool: Option<SelectedTool>,
    
    /// Reasoning for the final selection
    pub selection_reasoning: String,
    
    /// Discovery method used (hybrid, semantic, etc.)
    pub discovery_method: String,
    
    /// Performance metrics
    pub performance_metrics: DiscoveryPerformanceMetrics,
    
    /// Timestamp of discovery operation
    pub timestamp: DateTime<Utc>,
    
    /// Cache hit information
    pub cache_info: CacheHitInfo,
}

/// User context information for audit (anonymized if needed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditUserContext {
    /// User ID (may be anonymized for privacy)
    pub user_id: String,
    
    /// User roles at time of request
    pub user_roles: Vec<String>,
    
    /// Permission level (summarized)
    pub permission_level: String,
    
    /// Whether user is authenticated
    pub is_authenticated: bool,
    
    /// API key used (if applicable, anonymized)
    pub api_key_info: Option<String>,
}

/// Information about a tool that was excluded from consideration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludedTool {
    /// Tool identifier
    pub tool_id: ToolId,
    
    /// Tool name for display
    pub tool_name: String,
    
    /// Tool description
    pub tool_description: Option<String>,
    
    /// Reason for exclusion
    pub exclusion_reason: ExclusionReason,
    
    /// Specific rule that caused exclusion
    pub blocking_rule: Option<String>,
    
    /// Would this tool have been a good match?
    pub potential_match_score: Option<f64>,
}

/// Reason why a tool was excluded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExclusionReason {
    /// Excluded by allowlist explicit deny rule
    AllowlistExplicitDeny { rule_id: String },
    
    /// Excluded by allowlist pattern rule
    AllowlistPatternDeny { pattern: String },
    
    /// Excluded by allowlist default deny policy
    AllowlistDefaultDeny,
    
    /// Excluded by RBAC role restrictions
    RbacRoleRestriction { required_roles: Vec<String> },
    
    /// Excluded by RBAC permission restrictions
    RbacPermissionRestriction { required_permissions: Vec<String> },
    
    /// Excluded by emergency lockdown
    EmergencyLockdown,
    
    /// Excluded due to service unavailability
    ServiceUnavailable { service: String },
}

/// Information about a tool that was considered and scored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredTool {
    /// Tool identifier
    pub tool_id: ToolId,
    
    /// Tool name for display
    pub tool_name: String,
    
    /// Tool description
    pub tool_description: Option<String>,
    
    /// Discovery confidence score (0.0 - 1.0)
    pub discovery_score: f64,
    
    /// Semantic similarity score (if applicable)
    pub semantic_score: Option<f64>,
    
    /// Rule-based score (if applicable)
    pub rule_score: Option<f64>,
    
    /// LLM-based score (if applicable)
    pub llm_score: Option<f64>,
    
    /// Combined weighted score used for ranking
    pub final_score: f64,
    
    /// Ranking position in candidate list
    pub ranking_position: usize,
    
    /// Why this tool was considered a match
    pub match_reasoning: String,
    
    /// Parameter mapping confidence (if applicable)
    pub parameter_mapping_score: Option<f64>,
}

/// Information about the final selected tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedTool {
    /// Tool that was selected
    pub scored_tool: ScoredTool,
    
    /// Parameter mapping results
    pub parameter_mapping: ParameterMappingResult,
    
    /// Confidence in the selection
    pub selection_confidence: f64,
    
    /// Whether this was a fallback choice
    pub is_fallback: bool,
    
    /// Alternative tools that were close
    pub alternatives: Vec<ScoredTool>,
}

/// Results of parameter mapping for the selected tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterMappingResult {
    /// Successfully mapped parameters
    pub mapped_parameters: serde_json::Value,
    
    /// Parameters that couldn't be mapped
    pub unmapped_parameters: Vec<String>,
    
    /// Parameters that used default values
    pub defaulted_parameters: Vec<String>,
    
    /// Confidence in parameter mapping (0.0 - 1.0)
    pub mapping_confidence: f64,
    
    /// Whether LLM was used for parameter mapping
    pub used_llm_mapping: bool,
}

/// Performance metrics for the discovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryPerformanceMetrics {
    /// Total time for discovery operation
    pub total_time_ms: u64,
    
    /// Time spent on permission filtering
    pub permission_filtering_time_ms: u64,
    
    /// Time spent on tool scoring
    pub tool_scoring_time_ms: u64,
    
    /// Time spent on parameter mapping
    pub parameter_mapping_time_ms: u64,
    
    /// Time spent on LLM calls (if any)
    pub llm_time_ms: u64,
    
    /// Cache lookup time
    pub cache_lookup_time_ms: u64,
    
    /// Number of tools evaluated
    pub tools_evaluated: usize,
    
    /// Memory usage estimate (bytes)
    pub estimated_memory_bytes: usize,
}

/// Information about cache hits during discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHitInfo {
    /// Whether user permission cache was hit
    pub permission_cache_hit: bool,
    
    /// Whether discovery result was cached
    pub discovery_cache_hit: bool,
    
    /// Whether parameter mapping was cached
    pub parameter_cache_hit: bool,
    
    /// Cache keys used (for debugging)
    pub cache_keys: Vec<String>,
}

impl DiscoveryAuditTrail {
    /// Create a new audit trail for a discovery operation
    pub fn new(request_id: String, user_request: String, security_context: &SecurityContext) -> Self {
        Self {
            request_id,
            user_request,
            user_context: AuditUserContext::from_security_context(security_context),
            total_tools_available: 0,
            tools_excluded_by_allowlist: Vec::new(),
            tools_excluded_by_rbac: Vec::new(),
            allowed_tools_considered: Vec::new(),
            selected_tool: None,
            selection_reasoning: String::new(),
            discovery_method: String::new(),
            performance_metrics: DiscoveryPerformanceMetrics::default(),
            timestamp: Utc::now(),
            cache_info: CacheHitInfo::default(),
        }
    }
    
    /// Add a tool that was excluded by allowlist
    pub fn add_allowlist_exclusion(&mut self, tool_id: ToolId, tool_name: String, reason: ExclusionReason) {
        self.tools_excluded_by_allowlist.push(ExcludedTool {
            tool_id,
            tool_name,
            tool_description: None,
            exclusion_reason: reason,
            blocking_rule: None,
            potential_match_score: None,
        });
    }
    
    /// Add a tool that was excluded by RBAC
    pub fn add_rbac_exclusion(&mut self, tool_id: ToolId, tool_name: String, reason: ExclusionReason) {
        self.tools_excluded_by_rbac.push(ExcludedTool {
            tool_id,
            tool_name,
            tool_description: None,
            exclusion_reason: reason,
            blocking_rule: None,
            potential_match_score: None,
        });
    }
    
    /// Add a tool that was considered and scored
    pub fn add_considered_tool(&mut self, scored_tool: ScoredTool) {
        self.allowed_tools_considered.push(scored_tool);
    }
    
    /// Set the final selected tool
    pub fn set_selected_tool(&mut self, selected_tool: SelectedTool) {
        self.selected_tool = Some(selected_tool);
    }
    
    /// Calculate summary statistics
    pub fn get_summary(&self) -> DiscoveryAuditSummary {
        DiscoveryAuditSummary {
            total_tools: self.total_tools_available,
            excluded_tools: self.tools_excluded_by_allowlist.len() + self.tools_excluded_by_rbac.len(),
            considered_tools: self.allowed_tools_considered.len(),
            success: self.selected_tool.is_some(),
            total_time_ms: self.performance_metrics.total_time_ms,
            cache_hit_ratio: self.calculate_cache_hit_ratio(),
        }
    }
    
    /// Calculate overall cache hit ratio
    fn calculate_cache_hit_ratio(&self) -> f64 {
        let hits = [
            self.cache_info.permission_cache_hit,
            self.cache_info.discovery_cache_hit,
            self.cache_info.parameter_cache_hit,
        ].iter().filter(|&&hit| hit).count();
        
        hits as f64 / 3.0
    }
    
    /// Get exclusion reasons summary
    pub fn get_exclusion_summary(&self) -> Vec<(String, usize)> {
        let mut summary = std::collections::HashMap::new();
        
        for excluded in &self.tools_excluded_by_allowlist {
            let reason = format!("Allowlist: {:?}", excluded.exclusion_reason);
            *summary.entry(reason).or_insert(0) += 1;
        }
        
        for excluded in &self.tools_excluded_by_rbac {
            let reason = format!("RBAC: {:?}", excluded.exclusion_reason);
            *summary.entry(reason).or_insert(0) += 1;
        }
        
        let mut result: Vec<_> = summary.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending
        result
    }
}

/// Summary statistics for a discovery audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryAuditSummary {
    pub total_tools: usize,
    pub excluded_tools: usize,
    pub considered_tools: usize,
    pub success: bool,
    pub total_time_ms: u64,
    pub cache_hit_ratio: f64,
}

impl AuditUserContext {
    /// Create audit user context from security context
    pub fn from_security_context(security_context: &SecurityContext) -> Self {
        let user = security_context.user.as_ref();
        
        Self {
            user_id: user
                .and_then(|u| u.id.as_ref())
                .map(|id| Self::anonymize_user_id(id))
                .unwrap_or_else(|| "anonymous".to_string()),
            user_roles: user
                .map(|u| u.roles.clone())
                .unwrap_or_default(),
            permission_level: user
                .map(|u| Self::summarize_permission_level(u))
                .unwrap_or_else(|| "none".to_string()),
            is_authenticated: user.is_some(),
            api_key_info: user
                .and_then(|u| u.api_key_name.as_ref())
                .map(|key| Self::anonymize_api_key(key)),
        }
    }
    
    /// Anonymize user ID for privacy (keep first 4 chars + hash)
    fn anonymize_user_id(user_id: &str) -> String {
        if user_id.len() <= 4 {
            user_id.to_string()
        } else {
            use std::hash::{Hash, Hasher};
            let prefix = &user_id[..4];
            let mut hasher = ahash::AHasher::default();
            user_id.hash(&mut hasher);
            let hash = format!("{:08x}", hasher.finish());
            format!("{}...{}", prefix, &hash[..4])
        }
    }
    
    /// Summarize permission level
    fn summarize_permission_level(user: &crate::security::SecurityUser) -> String {
        let roles = &user.roles;
        if roles.contains(&"admin".to_string()) {
            "admin".to_string()
        } else if roles.contains(&"developer".to_string()) {
            "developer".to_string()
        } else if roles.contains(&"user".to_string()) {
            "user".to_string()
        } else if !roles.is_empty() {
            "custom".to_string()
        } else {
            "none".to_string()
        }
    }
    
    /// Anonymize API key (show first 4 chars only)
    fn anonymize_api_key(api_key: &str) -> String {
        if api_key.len() <= 4 {
            "*".repeat(api_key.len())
        } else {
            format!("{}...{}", &api_key[..4], "*".repeat(4))
        }
    }
}

impl Default for DiscoveryPerformanceMetrics {
    fn default() -> Self {
        Self {
            total_time_ms: 0,
            permission_filtering_time_ms: 0,
            tool_scoring_time_ms: 0,
            parameter_mapping_time_ms: 0,
            llm_time_ms: 0,
            cache_lookup_time_ms: 0,
            tools_evaluated: 0,
            estimated_memory_bytes: 0,
        }
    }
}

impl Default for CacheHitInfo {
    fn default() -> Self {
        Self {
            permission_cache_hit: false,
            discovery_cache_hit: false,
            parameter_cache_hit: false,
            cache_keys: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{SecurityUser, SecurityRequest};
    
    fn create_test_security_context() -> SecurityContext {
        SecurityContext {
            user: Some(SecurityUser {
                id: Some("test_user_12345".to_string()),
                roles: vec!["user".to_string(), "developer".to_string()],
                api_key_name: Some("test_api_key".to_string()),
                permissions: vec![],
                auth_method: "api_key".to_string(),
            }),
            request: SecurityRequest {
                id: "req-123".to_string(),
                method: "POST".to_string(),
                path: "/mcp/call".to_string(),
                client_ip: Some("127.0.0.1".to_string()),
                user_agent: Some("test-agent".to_string()),
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
    fn test_audit_trail_creation() {
        let security_context = create_test_security_context();
        let audit_trail = DiscoveryAuditTrail::new(
            "req-123".to_string(),
            "find a tool to read files".to_string(),
            &security_context,
        );
        
        assert_eq!(audit_trail.request_id, "req-123");
        assert_eq!(audit_trail.user_request, "find a tool to read files");
        assert_eq!(audit_trail.user_context.permission_level, "developer");
        assert!(audit_trail.user_context.is_authenticated);
    }
    
    #[test]
    fn test_user_id_anonymization() {
        let anonymized = AuditUserContext::anonymize_user_id("test_user_12345");
        assert!(anonymized.starts_with("test"));
        assert!(anonymized.contains("..."));
        assert_eq!(anonymized.len(), "test...abcd".len());
    }
    
    #[test]
    fn test_api_key_anonymization() {
        let anonymized = AuditUserContext::anonymize_api_key("abcd_1234567890");
        assert_eq!(anonymized, "abcd...****");
    }
    
    #[test]
    fn test_exclusion_summary() {
        let security_context = create_test_security_context();
        let mut audit_trail = DiscoveryAuditTrail::new(
            "req-123".to_string(),
            "test request".to_string(),
            &security_context,
        );
        
        audit_trail.add_allowlist_exclusion(
            "tool1".to_string(),
            "Tool 1".to_string(),
            ExclusionReason::AllowlistDefaultDeny,
        );
        
        audit_trail.add_rbac_exclusion(
            "tool2".to_string(),
            "Tool 2".to_string(),
            ExclusionReason::RbacRoleRestriction { required_roles: vec!["admin".to_string()] },
        );
        
        let summary = audit_trail.get_exclusion_summary();
        assert_eq!(summary.len(), 2);
    }
}