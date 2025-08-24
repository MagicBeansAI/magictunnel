//! Filtered Tool Listing for MCP Protocol
//!
//! This module implements permission-based filtering for MCP tool listing operations,
//! ensuring users only see tools they can actually access.

use crate::discovery::permission_cache::{PermissionCacheManager, ToolId};
use crate::discovery::fast_evaluator::{FastPermissionEvaluator, FastUserContext, RuleAction};
use crate::discovery::audit_trail::{DiscoveryAuditTrail, ExcludedTool, ExclusionReason};
use crate::security::SecurityContext;
use crate::registry::types::{ToolDefinition, CapabilityFile};
use crate::error::Result;
use ahash::{AHashMap, AHashSet};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};
use serde::Serialize;

/// Service for filtering tool listings based on user permissions
pub struct FilteredToolListingService {
    /// Permission cache manager for fast user-specific filtering
    permission_cache: Arc<PermissionCacheManager>,
    
    /// Fast permission evaluator for rule checking
    permission_evaluator: Arc<tokio::sync::RwLock<FastPermissionEvaluator>>,
    
    /// Configuration for filtered listing
    config: FilteredListingConfig,
}

/// Configuration for filtered tool listing
#[derive(Debug, Clone)]
pub struct FilteredListingConfig {
    /// Whether to enable permission pre-filtering for tool lists
    pub enable_permission_filtering: bool,
    
    /// Whether to show anonymized excluded tools count for debugging
    pub show_excluded_count: bool,
    
    /// Whether to generate audit trail for tool listing operations
    pub enable_audit_trail: bool,
    
    /// Maximum time to spend on filtering (ms)
    pub max_filtering_time_ms: u64,
    
    /// Whether to sort allowed tools by relevance
    pub sort_by_relevance: bool,
}

impl Default for FilteredListingConfig {
    fn default() -> Self {
        Self {
            enable_permission_filtering: true,
            show_excluded_count: false, // Don't reveal too much info by default
            enable_audit_trail: true,
            max_filtering_time_ms: 5, // Very fast for tool listing
            sort_by_relevance: false, // Preserve original order by default
        }
    }
}

/// Result of filtered tool listing with metadata
#[derive(Debug, Clone, Serialize)]
pub struct FilteredToolListResult {
    /// Filtered tools that the user can access
    pub allowed_tools: Vec<FilteredToolInfo>,
    
    /// Total number of tools available (before filtering)
    pub total_tools_available: usize,
    
    /// Number of tools excluded due to permissions
    pub tools_excluded: usize,
    
    /// Performance metrics for the filtering operation
    pub filtering_metrics: ListingFilteringMetrics,
    
    /// Audit trail (if enabled)
    pub audit_trail: Option<DiscoveryAuditTrail>,
}

/// Information about a filtered tool
#[derive(Debug, Clone, Serialize)]
pub struct FilteredToolInfo {
    /// Tool identifier
    pub tool_id: ToolId,
    
    /// Tool name for display
    pub name: String,
    
    /// Tool description
    pub description: Option<String>,
    
    /// Tool input schema
    pub input_schema: Option<serde_json::Value>,
    
    /// Why this tool is allowed (for transparency)
    pub access_reason: String,
    
    /// Confidence that user can successfully use this tool
    pub access_confidence: f64,
    
    /// Tool category/tags
    pub categories: Vec<String>,
    
    /// Whether tool is enhanced with AI metadata
    pub is_enhanced: bool,
}

/// Performance metrics for tool listing filtering
#[derive(Debug, Clone, Serialize)]
pub struct ListingFilteringMetrics {
    /// Time spent filtering tools
    pub filtering_time_ms: u64,
    
    /// Tools processed per second
    pub tools_per_second: f64,
    
    /// Cache hit ratio during filtering
    pub cache_hit_ratio: f64,
    
    /// Whether filtering completed within timeout
    pub completed_within_timeout: bool,
}

impl FilteredToolListingService {
    /// Create a new filtered tool listing service
    pub fn new(
        permission_cache: Arc<PermissionCacheManager>,
        config: FilteredListingConfig,
    ) -> Self {
        let permission_evaluator = Arc::new(tokio::sync::RwLock::new(
            FastPermissionEvaluator::new(RuleAction::Deny) // Default deny for safety
        ));
        
        Self {
            permission_cache,
            permission_evaluator,
            config,
        }
    }
    
    /// Filter tools based on user permissions for MCP listing
    pub async fn filter_tools_for_listing(
        &self,
        available_tools: &AHashMap<String, ToolDefinition>,
        security_context: &SecurityContext,
    ) -> Result<FilteredToolListResult> {
        let start_time = Instant::now();
        
        let mut audit_trail = if self.config.enable_audit_trail {
            Some(DiscoveryAuditTrail::new(
                format!("tool_listing_{}", chrono::Utc::now().timestamp()),
                "list_tools".to_string(),
                security_context,
            ))
        } else {
            None
        };
        
        // Track filtering metrics
        let mut metrics = ListingFilteringMetrics {
            filtering_time_ms: 0,
            tools_per_second: 0.0,
            cache_hit_ratio: 0.0,
            completed_within_timeout: true,
        };
        
        let total_tools = available_tools.len();
        
        // Update audit trail with total tools
        if let Some(ref mut audit) = audit_trail {
            audit.total_tools_available = total_tools;
        }
        
        // Short-circuit if filtering is disabled
        if !self.config.enable_permission_filtering {
            let allowed_tools = available_tools.iter()
                .map(|(tool_id, tool_def)| self.create_filtered_tool_info(
                    tool_id.clone(),
                    tool_def,
                    "filtering_disabled".to_string(),
                    1.0,
                ))
                .collect();
            
            return Ok(FilteredToolListResult {
                allowed_tools,
                total_tools_available: total_tools,
                tools_excluded: 0,
                filtering_metrics: metrics,
                audit_trail,
            });
        }
        
        // Get user context for fast evaluation
        let fast_user_context = FastUserContext::from_security_context(security_context);
        if fast_user_context.is_none() {
            warn!("Could not create fast user context for tool listing, showing all tools");
            let allowed_tools = available_tools.iter()
                .map(|(tool_id, tool_def)| self.create_filtered_tool_info(
                    tool_id.clone(),
                    tool_def,
                    "no_user_context".to_string(),
                    0.5,
                ))
                .collect();
            
            return Ok(FilteredToolListResult {
                allowed_tools,
                total_tools_available: total_tools,
                tools_excluded: 0,
                filtering_metrics: metrics,
                audit_trail,
            });
        }
        let fast_user_context = fast_user_context.unwrap();
        
        // Perform filtering with timeout
        let filtering_result = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.max_filtering_time_ms),
            self.do_filtering(&available_tools, &fast_user_context, &mut audit_trail)
        ).await;
        
        let (allowed_tools, excluded_tools) = match filtering_result {
            Ok(result) => result?,
            Err(_) => {
                warn!("Tool listing filtering timed out after {}ms, returning all tools", 
                      self.config.max_filtering_time_ms);
                metrics.completed_within_timeout = false;
                
                let allowed_tools = available_tools.iter()
                    .map(|(tool_id, tool_def)| self.create_filtered_tool_info(
                        tool_id.clone(),
                        tool_def,
                        "timeout_fallback".to_string(),
                        0.8,
                    ))
                    .collect();
                
                (allowed_tools, Vec::new())
            }
        };
        
        // Calculate metrics
        let elapsed = start_time.elapsed();
        metrics.filtering_time_ms = elapsed.as_millis() as u64;
        metrics.tools_per_second = if elapsed.as_secs_f64() > 0.0 {
            total_tools as f64 / elapsed.as_secs_f64()
        } else {
            f64::INFINITY
        };
        
        // Sort tools if requested
        let mut final_allowed_tools = allowed_tools;
        if self.config.sort_by_relevance {
            final_allowed_tools.sort_by(|a, b| {
                b.access_confidence.partial_cmp(&a.access_confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        
        info!("Tool listing filtered: {} allowed, {} excluded, {:.2}ms", 
              final_allowed_tools.len(), excluded_tools.len(), 
              metrics.filtering_time_ms);
        
        Ok(FilteredToolListResult {
            allowed_tools: final_allowed_tools,
            total_tools_available: total_tools,
            tools_excluded: excluded_tools.len(),
            filtering_metrics: metrics,
            audit_trail,
        })
    }
    
    /// Perform the actual filtering logic
    async fn do_filtering(
        &self,
        available_tools: &AHashMap<String, ToolDefinition>,
        fast_user_context: &FastUserContext,
        audit_trail: &mut Option<DiscoveryAuditTrail>,
    ) -> Result<(Vec<FilteredToolInfo>, Vec<ExcludedTool>)> {
        let mut allowed_tools = Vec::new();
        let mut excluded_tools = Vec::new();
        
        // Get a write lock on the evaluator
        let mut evaluator = self.permission_evaluator.write().await;
        
        for (tool_id, tool_def) in available_tools {
            // Skip smart_tool_discovery to avoid recursion
            if tool_id == "smart_tool_discovery" || tool_id == "smart_discovery_tool" {
                continue;
            }
            
            let evaluation_result = evaluator.is_tool_allowed(fast_user_context, tool_id);
            
            if evaluation_result.allowed {
                // Tool is allowed - create filtered tool info
                let access_confidence = self.calculate_access_confidence(&evaluation_result.reason);
                let filtered_tool = self.create_filtered_tool_info(
                    tool_id.clone(),
                    tool_def,
                    evaluation_result.reason.clone(),
                    access_confidence,
                );
                
                allowed_tools.push(filtered_tool);
                
                // Add to audit trail
                if let Some(ref mut audit) = audit_trail {
                    audit.add_considered_tool(crate::discovery::audit_trail::ScoredTool {
                        tool_id: tool_id.clone(),
                        tool_name: tool_def.name.clone(),
                        tool_description: Some(tool_def.description.clone()),
                        discovery_score: 1.0,
                        semantic_score: None,
                        rule_score: None,
                        llm_score: None,
                        final_score: access_confidence,
                        ranking_position: allowed_tools.len(),
                        match_reasoning: format!("Allowed for listing: {}", evaluation_result.reason),
                        parameter_mapping_score: None,
                    });
                }
            } else {
                // Tool is excluded
                let exclusion_reason = self.determine_exclusion_reason(&evaluation_result.reason);
                
                excluded_tools.push(ExcludedTool {
                    tool_id: tool_id.clone(),
                    tool_name: tool_def.name.clone(),
                    tool_description: Some(tool_def.description.clone()),
                    exclusion_reason: exclusion_reason.clone(),
                    blocking_rule: Some(evaluation_result.reason.clone()),
                    potential_match_score: None,
                });
                
                // Add to audit trail
                if let Some(ref mut audit) = audit_trail {
                    match exclusion_reason {
                        ExclusionReason::AllowlistExplicitDeny { .. } |
                        ExclusionReason::AllowlistPatternDeny { .. } |
                        ExclusionReason::AllowlistDefaultDeny => {
                            audit.add_allowlist_exclusion(tool_id.clone(), tool_def.name.clone(), exclusion_reason);
                        }
                        ExclusionReason::RbacRoleRestriction { .. } |
                        ExclusionReason::RbacPermissionRestriction { .. } => {
                            audit.add_rbac_exclusion(tool_id.clone(), tool_def.name.clone(), exclusion_reason);
                        }
                        _ => {
                            audit.add_allowlist_exclusion(tool_id.clone(), tool_def.name.clone(), exclusion_reason);
                        }
                    }
                }
            }
        }
        
        Ok((allowed_tools, excluded_tools))
    }
    
    /// Create filtered tool info from tool definition
    fn create_filtered_tool_info(
        &self,
        tool_id: ToolId,
        tool_def: &ToolDefinition,
        access_reason: String,
        access_confidence: f64,
    ) -> FilteredToolInfo {
        FilteredToolInfo {
            tool_id: tool_id.clone(),
            name: tool_def.name.clone(),
            description: Some(tool_def.description.clone()),
            input_schema: Some(tool_def.input_schema.clone()),
            access_reason,
            access_confidence,
            categories: Vec::new(), // Categories not available on ToolDefinition
            is_enhanced: false, // Enhanced description not available on ToolDefinition
        }
    }
    
    /// Calculate access confidence based on reason
    fn calculate_access_confidence(&self, reason: &str) -> f64 {
        match reason {
            "explicit_allow_bitmap" => 1.0,
            "admin" | "developer" => 0.95,
            "user" => 0.85,
            "pattern_match" => 0.75,
            "default" => 0.6,
            "timeout_fallback" => 0.5,
            "no_user_context" => 0.3,
            "filtering_disabled" => 1.0,
            _ => 0.7, // Default moderate confidence
        }
    }
    
    /// Determine the specific reason for tool exclusion
    fn determine_exclusion_reason(&self, reason: &str) -> ExclusionReason {
        match reason {
            "explicit_deny_bitmap" => ExclusionReason::AllowlistExplicitDeny {
                rule_id: "explicit_deny_bitmap".to_string()
            },
            "pattern_match" => ExclusionReason::AllowlistPatternDeny {
                pattern: "unknown_pattern".to_string()
            },
            "default" => ExclusionReason::AllowlistDefaultDeny,
            reason if reason.contains("role") => ExclusionReason::RbacRoleRestriction {
                required_roles: vec!["unknown".to_string()]
            },
            reason if reason.contains("permission") => ExclusionReason::RbacPermissionRestriction {
                required_permissions: vec!["unknown".to_string()]
            },
            _ => ExclusionReason::AllowlistDefaultDeny,
        }
    }
    
    /// Filter capabilities based on user permissions
    pub async fn filter_capabilities_for_listing(
        &self,
        capabilities: &[CapabilityFile],
        security_context: &SecurityContext,
    ) -> Result<Vec<CapabilityFile>> {
        // Extract tools from capabilities
        let mut all_tools = AHashMap::new();
        for capability in capabilities {
            for tool in &capability.tools {
                all_tools.insert(tool.name.clone(), tool.clone());
            }
        }
        
        // Filter tools
        let filtered_result = self.filter_tools_for_listing(&all_tools, security_context).await?;
        let allowed_tool_ids: AHashSet<_> = filtered_result.allowed_tools
            .into_iter()
            .map(|tool| tool.tool_id)
            .collect();
        
        // Rebuild capabilities with only allowed tools
        let mut filtered_capabilities = Vec::new();
        for capability in capabilities {
            let mut filtered_capability = capability.clone();
            filtered_capability.tools = capability.tools
                .iter()
                .filter(|tool| allowed_tool_ids.contains(&tool.name))
                .cloned()
                .collect();
            
            // Only include capability if it has at least one allowed tool
            if !filtered_capability.tools.is_empty() {
                filtered_capabilities.push(filtered_capability);
            }
        }
        
        debug!("Capability filtering: {} -> {} capabilities with allowed tools",
               capabilities.len(), filtered_capabilities.len());
        
        Ok(filtered_capabilities)
    }
    
    /// Get filtering statistics
    pub async fn get_filtering_stats(&self) -> serde_json::Value {
        let evaluator_stats = {
            let evaluator = self.permission_evaluator.read().await;
            evaluator.get_stats().clone()
        };
        
        serde_json::json!({
            "permission_evaluator": {
                "total_evaluations": evaluator_stats.total_evaluations,
                "fast_path_hits": evaluator_stats.fast_path_hits,
                "slow_path_hits": evaluator_stats.slow_path_hits,
                "avg_evaluation_time_ns": evaluator_stats.avg_evaluation_time_ns,
                "fast_path_ratio": if evaluator_stats.total_evaluations > 0 {
                    evaluator_stats.fast_path_hits as f64 / evaluator_stats.total_evaluations as f64
                } else { 0.0 },
            },
            "config": {
                "permission_filtering_enabled": self.config.enable_permission_filtering,
                "max_filtering_time_ms": self.config.max_filtering_time_ms,
                "sort_by_relevance": self.config.sort_by_relevance,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::permission_cache::PermissionCacheConfig;
    use crate::security::{SecurityUser, SecurityRequest};
    
    fn create_test_security_context(user_id: &str, roles: Vec<String>) -> SecurityContext {
        SecurityContext {
            user: Some(SecurityUser {
                id: Some(user_id.to_string()),
                roles,
                api_key_name: None,
                permissions: vec![],
                auth_method: "test".to_string(),
            }),
            request: SecurityRequest {
                id: "req-123".to_string(),
                method: "POST".to_string(),
                path: "/mcp/tools/list".to_string(),
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
    
    fn create_test_tool_definition(name: &str, description: &str) -> ToolDefinition {
        use crate::registry::types::RoutingConfig;
        use serde_json::Value;
        ToolDefinition {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
            routing: RoutingConfig {
                r#type: "test".to_string(),
                config: Value::Object(serde_json::Map::new()),
            },
            annotations: None,
            hidden: false,
            enabled: true,
            prompt_refs: Vec::new(),
            resource_refs: Vec::new(),
            sampling_strategy: None,
            elicitation_strategy: None,
        }
    }
    
    #[tokio::test]
    async fn test_filtered_tool_listing_service_creation() {
        let cache_config = PermissionCacheConfig::default();
        let permission_cache = Arc::new(PermissionCacheManager::new_with_default_rbac(cache_config));
        
        let listing_config = FilteredListingConfig::default();
        let service = FilteredToolListingService::new(permission_cache, listing_config);
        
        let stats = service.get_filtering_stats().await;
        assert!(stats.get("permission_evaluator").is_some());
    }
    
    #[tokio::test]
    async fn test_tool_filtering_with_permissions() {
        let cache_config = PermissionCacheConfig::default();
        let permission_cache = Arc::new(PermissionCacheManager::new_with_default_rbac(cache_config));
        
        let mut listing_config = FilteredListingConfig::default();
        listing_config.enable_permission_filtering = true;
        
        let service = FilteredToolListingService::new(permission_cache, listing_config);
        
        // Create test tools
        let mut tools = AHashMap::new();
        tools.insert("tool1".to_string(), create_test_tool_definition("tool1", "Test tool 1"));
        tools.insert("tool2".to_string(), create_test_tool_definition("tool2", "Test tool 2"));
        
        let security_context = create_test_security_context("test_user", vec!["user".to_string()]);
        
        let result = service.filter_tools_for_listing(&tools, &security_context).await;
        assert!(result.is_ok());
        
        let filtered_result = result.unwrap();
        assert_eq!(filtered_result.total_tools_available, 2);
        // The actual filtering behavior depends on the FastPermissionEvaluator implementation
    }
    
    #[tokio::test]
    async fn test_tool_filtering_disabled() {
        let cache_config = PermissionCacheConfig::default();
        let permission_cache = Arc::new(PermissionCacheManager::new_with_default_rbac(cache_config));
        
        let mut listing_config = FilteredListingConfig::default();
        listing_config.enable_permission_filtering = false;
        
        let service = FilteredToolListingService::new(permission_cache, listing_config);
        
        let mut tools = AHashMap::new();
        tools.insert("tool1".to_string(), create_test_tool_definition("tool1", "Test tool 1"));
        
        let security_context = create_test_security_context("test_user", vec!["user".to_string()]);
        
        let result = service.filter_tools_for_listing(&tools, &security_context).await.unwrap();
        
        assert_eq!(result.total_tools_available, 1);
        assert_eq!(result.tools_excluded, 0);
        assert_eq!(result.allowed_tools.len(), 1);
        assert_eq!(result.allowed_tools[0].access_reason, "filtering_disabled");
    }
}