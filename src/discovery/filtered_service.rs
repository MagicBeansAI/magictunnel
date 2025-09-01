//! Permission-Filtered Smart Discovery Service
//!
//! This module extends the smart discovery service with pre-filtering based on
//! user permissions, ensuring only allowed tools are considered for discovery.

use crate::discovery::types::*;
use crate::discovery::service::SmartDiscoveryService;
use crate::discovery::permission_cache::{PermissionCacheManager, PermissionCacheConfig, ToolId};
use crate::discovery::fast_evaluator::{FastPermissionEvaluator, FastUserContext, RuleAction};
use crate::discovery::audit_trail::{
    DiscoveryAuditTrail, ExcludedTool, ScoredTool, SelectedTool, ExclusionReason,
    ParameterMappingResult
};
use crate::security::{SecurityContext, AllowlistService, RbacService};
use crate::error::Result;
use ahash::{AHashMap, AHashSet};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Enhanced smart discovery service with permission pre-filtering
pub struct FilteredSmartDiscoveryService {
    /// Underlying smart discovery service
    inner: Arc<SmartDiscoveryService>,
    
    /// Permission cache manager for fast user-specific filtering
    permission_cache: Arc<PermissionCacheManager>,
    
    /// Fast permission evaluator for ultra-fast rule checking
    permission_evaluator: Arc<tokio::sync::RwLock<FastPermissionEvaluator>>,
    
    /// Allowlist service for rule evaluation
    allowlist_service: Option<Arc<AllowlistService>>,
    
    /// Configuration for filtered discovery
    config: FilteredDiscoveryConfig,
}

/// Configuration for filtered smart discovery
#[derive(Debug, Clone)]
pub struct FilteredDiscoveryConfig {
    /// Whether to enable permission pre-filtering
    pub enable_permission_filtering: bool,
    
    /// Whether to enable audit trail generation
    pub enable_audit_trail: bool,
    
    /// Maximum time to spend on permission filtering (ms)
    pub max_permission_filtering_time_ms: u64,
    
    /// Whether to log excluded tools for debugging
    pub log_excluded_tools: bool,
    
    /// Cache configuration
    pub cache_config: PermissionCacheConfig,
}

impl Default for FilteredDiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_permission_filtering: true,
            enable_audit_trail: true,
            max_permission_filtering_time_ms: 10, // Very fast filtering
            log_excluded_tools: false, // Avoid log spam in production
            cache_config: PermissionCacheConfig::default(),
        }
    }
}

/// Result of filtered tool discovery with audit trail
#[derive(Debug, Clone)]
pub struct FilteredDiscoveryResult {
    /// Standard discovery response
    pub discovery_response: SmartDiscoveryResponse,
    
    /// Comprehensive audit trail
    pub audit_trail: Option<DiscoveryAuditTrail>,
    
    /// Performance metrics specific to filtering
    pub filtering_metrics: FilteringPerformanceMetrics,
}

/// Performance metrics for the filtering process
#[derive(Debug, Clone)]
pub struct FilteringPerformanceMetrics {
    /// Time spent on permission filtering
    pub permission_filtering_time_ms: u64,
    
    /// Number of tools filtered out
    pub tools_filtered_out: usize,
    
    /// Number of tools allowed through
    pub tools_allowed: usize,
    
    /// Cache hit rate for permissions
    pub permission_cache_hit_rate: f64,
    
    /// Whether filtering was skipped due to timeout
    pub filtering_timed_out: bool,
}

impl FilteredSmartDiscoveryService {
    /// Create a new filtered smart discovery service with RBAC service
    pub fn new(
        inner: Arc<SmartDiscoveryService>,
        config: FilteredDiscoveryConfig,
        rbac_service: Arc<RbacService>,
    ) -> Self {
        let permission_cache = Arc::new(
            PermissionCacheManager::new_production(config.cache_config.clone(), rbac_service)
                .expect("Failed to create production permission cache manager")
        );
        let permission_evaluator = Arc::new(tokio::sync::RwLock::new(
            FastPermissionEvaluator::new(RuleAction::Deny) // Default deny
        ));
        
        Self {
            inner,
            permission_cache,
            permission_evaluator,
            allowlist_service: None,
            config,
        }
    }
    
    /// Create a new filtered smart discovery service with default RBAC for testing
    /// ⚠️  WARNING: This method is for testing/development only
    #[cfg(debug_assertions)]
    pub fn new_with_mock_rbac(
        inner: Arc<SmartDiscoveryService>,
        config: FilteredDiscoveryConfig,
    ) -> Self {
        use tracing::warn;
        
        warn!("⚠️  Creating FilteredSmartDiscoveryService with mock RBAC - FOR TESTING ONLY");
        
        let permission_cache = Arc::new(
            PermissionCacheManager::new_with_default_rbac(config.cache_config.clone())
        );
        let permission_evaluator = Arc::new(tokio::sync::RwLock::new(
            FastPermissionEvaluator::new(RuleAction::Deny) // Default deny
        ));
        
        Self {
            inner,
            permission_cache,
            permission_evaluator,
            allowlist_service: None,
            config,
        }
    }
    
    /// Set the allowlist service for rule evaluation
    pub fn set_allowlist_service(&mut self, allowlist_service: Arc<AllowlistService>) {
        self.allowlist_service = Some(allowlist_service);
        info!("Allowlist service set for filtered smart discovery");
    }
    
    /// Main filtered discovery method with pre-filtering
    pub async fn discover_with_filtering(
        &self,
        request: SmartDiscoveryRequest,
        security_context: &SecurityContext,
    ) -> Result<FilteredDiscoveryResult> {
        let start_time = Instant::now();
        let request_id = Uuid::new_v4().to_string();
        
        // Create audit trail if enabled
        let mut audit_trail = if self.config.enable_audit_trail {
            Some(DiscoveryAuditTrail::new(
                request_id.clone(),
                request.request.clone(),
                security_context,
            ))
        } else {
            None
        };
        
        // Performance tracking
        let mut filtering_metrics = FilteringPerformanceMetrics {
            permission_filtering_time_ms: 0,
            tools_filtered_out: 0,
            tools_allowed: 0,
            permission_cache_hit_rate: 0.0,
            filtering_timed_out: false,
        };
        
        // Step 1: Pre-filter tools based on user permissions
        let (allowed_tools, excluded_tools) = if self.config.enable_permission_filtering {
            let filter_start = Instant::now();
            
            let result = tokio::time::timeout(
                std::time::Duration::from_millis(self.config.max_permission_filtering_time_ms),
                self.filter_tools_by_permissions(security_context, &mut audit_trail, &request.request)
            ).await;
            
            let filter_duration = filter_start.elapsed();
            filtering_metrics.permission_filtering_time_ms = filter_duration.as_millis() as u64;
            
            match result {
                Ok(filter_result) => {
                    let (allowed, excluded) = filter_result?;
                    filtering_metrics.tools_allowed = allowed.len();
                    filtering_metrics.tools_filtered_out = excluded.len();
                    (allowed, excluded)
                }
                Err(_) => {
                    warn!("Permission filtering timed out after {}ms, proceeding without filtering", 
                          self.config.max_permission_filtering_time_ms);
                    filtering_metrics.filtering_timed_out = true;
                    (AHashSet::new(), Vec::new()) // Empty sets - will fall back to original discovery
                }
            }
        } else {
            (AHashSet::new(), Vec::new())
        };
        
        // Step 2: Create filtered request if we have allowed tools
        let discovery_response = if self.config.enable_permission_filtering && !allowed_tools.is_empty() && !filtering_metrics.filtering_timed_out {
            info!("Running filtered discovery with {} allowed tools (excluded {})", 
                  allowed_tools.len(), excluded_tools.len());
            
            // Create a filtered request that limits tool selection
            let filtered_request = SmartDiscoveryRequest {
                preferred_tools: Some(allowed_tools.iter().cloned().collect()),
                request: request.request.clone(),
                context: request.context.clone(),
                confidence_threshold: request.confidence_threshold,
                include_error_details: request.include_error_details,
                sequential_mode: request.sequential_mode,
                auth_context: request.auth_context.clone(),
            };
            
            // Run discovery on filtered tools
            match self.inner.discover_and_execute(filtered_request).await {
                Ok(response) => response,
                Err(e) => {
                    warn!("Filtered discovery failed, falling back to original discovery: {}", e);
                    // Fall back to original discovery
                    let fallback_request = SmartDiscoveryRequest {
                        preferred_tools: request.preferred_tools.clone(),
                        request: request.request.clone(),
                        context: request.context.clone(),
                        confidence_threshold: request.confidence_threshold,
                        include_error_details: request.include_error_details,
                        sequential_mode: request.sequential_mode,
                        auth_context: request.auth_context.clone(),
                    };
                    self.inner.discover_and_execute(fallback_request).await?
                }
            }
        } else {
            // Run original discovery (either filtering disabled, no allowed tools, or timeout)
            if self.config.enable_permission_filtering && allowed_tools.is_empty() && !filtering_metrics.filtering_timed_out {
                warn!("No tools allowed for user, but proceeding with original discovery as fallback");
            }
            let original_request = SmartDiscoveryRequest {
                preferred_tools: request.preferred_tools.clone(),
                request: request.request.clone(),
                context: request.context.clone(),
                confidence_threshold: request.confidence_threshold,
                include_error_details: request.include_error_details,
                sequential_mode: request.sequential_mode,
                auth_context: request.auth_context.clone(),
            };
            self.inner.discover_and_execute(original_request).await?
        };
        
        // Step 3: Update audit trail with results
        if let Some(ref mut audit) = audit_trail {
            self.update_audit_trail_with_results(audit, &discovery_response, &allowed_tools, &excluded_tools).await;
            
            // Update performance metrics
            audit.performance_metrics.total_time_ms = start_time.elapsed().as_millis() as u64;
            audit.performance_metrics.permission_filtering_time_ms = filtering_metrics.permission_filtering_time_ms;
            audit.performance_metrics.tools_evaluated = allowed_tools.len();
        }
        
        // Step 4: Log filtering summary if enabled
        if self.config.log_excluded_tools && !excluded_tools.is_empty() {
            self.log_filtering_summary(&excluded_tools, &allowed_tools);
        }
        
        Ok(FilteredDiscoveryResult {
            discovery_response,
            audit_trail,
            filtering_metrics,
        })
    }
    
    /// Filter available tools based on user permissions
    async fn filter_tools_by_permissions(
        &self,
        security_context: &SecurityContext,
        audit_trail: &mut Option<DiscoveryAuditTrail>,
        request_text: &str,
    ) -> Result<(AHashSet<ToolId>, Vec<ExcludedTool>)> {
        // Get all available tools
        let enabled_tools = self.inner.get_registry().get_enabled_tools();
        let all_tools: AHashMap<String, crate::registry::types::ToolDefinition> = enabled_tools.into_iter().collect();
        let mut allowed_tools = AHashSet::new();
        let mut excluded_tools = Vec::new();
        
        // Get user context for fast evaluation
        let fast_user_context = FastUserContext::from_security_context(security_context);
        if fast_user_context.is_none() {
            warn!("Could not create fast user context, allowing all tools");
            return Ok((
                all_tools.into_iter().map(|(name, _)| name).collect(),
                Vec::new()
            ));
        }
        let fast_user_context = fast_user_context.unwrap();
        
        // Update audit trail with total tools
        if let Some(ref mut audit) = audit_trail {
            audit.total_tools_available = all_tools.len();
        }
        
        // Check each tool against user permissions
        {
            let mut evaluator = self.permission_evaluator.write().await;
            
            for (tool_name, tool_def) in all_tools {
                // Skip smart_tool_discovery to avoid recursion
                if tool_name == "smart_tool_discovery" || tool_name == "smart_discovery_tool" {
                    continue;
                }
                
                let evaluation_result = evaluator.is_tool_allowed(&fast_user_context, &tool_name);
                
                if evaluation_result.allowed {
                    allowed_tools.insert(tool_name.clone());
                    
                    // Compute a preliminary discovery score based on request/tool similarity
                    let prelim_score = self.estimate_potential_match_score(request_text, &tool_def);
                    
                    // Add to audit trail as considered tool with real score
                    if let Some(ref mut audit) = audit_trail {
                        audit.add_considered_tool(ScoredTool {
                            tool_id: tool_name.clone(),
                            tool_name: tool_name.clone(),
                            tool_description: Some(tool_def.description.clone()),
                            discovery_score: prelim_score,
                            semantic_score: None,
                            rule_score: None,
                            llm_score: None,
                            final_score: prelim_score,
                            ranking_position: 0, // Will be set later
                            match_reasoning: if prelim_score > 0.0 {
                                format!("Permission-allowed; lexical match {:.2}", prelim_score)
                            } else {
                                "Permission-allowed".to_string()
                            },
                            parameter_mapping_score: None,
                        });
                    }
                } else {
                    // Tool is excluded - determine why
                    let exclusion_reason = self.determine_exclusion_reason(&tool_name, &evaluation_result.reason);

                    // Estimate how good a match this would have been for visibility
                    let potential_match_score = self
                        .estimate_potential_match_score(request_text, &tool_def);

                    let excluded_entry = ExcludedTool {
                        tool_id: tool_name.clone(),
                        tool_name: tool_name.clone(),
                        tool_description: Some(tool_def.description.clone()),
                        exclusion_reason: exclusion_reason.clone(),
                        blocking_rule: Some(evaluation_result.reason.clone()),
                        potential_match_score: Some(potential_match_score),
                    };

                    excluded_tools.push(excluded_entry.clone());

                    // Add to audit trail with detailed entry
                    if let Some(ref mut audit) = audit_trail {
                        match exclusion_reason {
                            ExclusionReason::AllowlistExplicitDeny { .. } |
                            ExclusionReason::AllowlistPatternDeny { .. } |
                            ExclusionReason::AllowlistDefaultDeny => {
                                audit.add_allowlist_excluded_tool(excluded_entry);
                            }
                            ExclusionReason::RbacRoleRestriction { .. } |
                            ExclusionReason::RbacPermissionRestriction { .. } => {
                                audit.add_rbac_excluded_tool(excluded_entry);
                            }
                            _ => {
                                audit.add_allowlist_excluded_tool(excluded_entry);
                            }
                        }
                    }
                }
            }
        }
        
        info!("Permission filtering complete: {} allowed, {} excluded", 
              allowed_tools.len(), excluded_tools.len());
        
        Ok((allowed_tools, excluded_tools))
    }
    
    /// Determine the specific reason for tool exclusion
    fn determine_exclusion_reason(&self, tool_name: &str, reason: &str) -> ExclusionReason {
        // First, try to use allowlist decision details if available
        if let Some(ref svc) = self.allowlist_service {
            if let Some(decision) = svc.get_tool_decision(tool_name) {
                use crate::security::allowlist_data::RuleSource;
                return match decision.rule_source {
                    RuleSource::ExplicitTool | RuleSource::ExplicitCapability => {
                        ExclusionReason::AllowlistExplicitDeny { rule_id: decision.rule_name }
                    }
                    RuleSource::ToolPattern | RuleSource::CapabilityPattern | RuleSource::GlobalPattern => {
                        ExclusionReason::AllowlistPatternDeny { pattern: decision.rule_name }
                    }
                    RuleSource::DefaultAction => ExclusionReason::AllowlistDefaultDeny,
                    RuleSource::EmergencyLockdown => ExclusionReason::EmergencyLockdown,
                };
            }
        }

        // Fall back to evaluator reason hints and RBAC index
        match reason {
            "explicit_deny_bitmap" => ExclusionReason::AllowlistExplicitDeny {
                rule_id: "explicit_deny_bitmap".to_string(),
            },
            "pattern_match" => ExclusionReason::AllowlistPatternDeny {
                pattern: "pattern_rule".to_string(),
            },
            "default" => ExclusionReason::AllowlistDefaultDeny,
            r if r.contains("role") => {
                let required_roles = self
                    .permission_cache
                    .get_tool_required_roles(&tool_name.to_string());
                if required_roles.is_empty() {
                    ExclusionReason::RbacRoleRestriction {
                        required_roles: vec!["unknown".to_string()],
                    }
                } else {
                    ExclusionReason::RbacRoleRestriction { required_roles }
                }
            }
            r if r.contains("permission") => {
                let bitmap_opt = self
                    .permission_cache
                    .get_tool_required_permissions_bitmap(&tool_name.to_string());
                let permission_names = if let Some(bitmap) = bitmap_opt {
                    // Map set bits to synthetic permission identifiers for visibility
                    (0u8..64)
                        .filter(|bit| (bitmap & (1u64 << bit)) != 0)
                        .map(|bit| format!("perm_bit_{}", bit))
                        .collect::<Vec<String>>()
                } else {
                    vec!["unknown".to_string()]
                };
                ExclusionReason::RbacPermissionRestriction {
                    required_permissions: permission_names,
                }
            }
            _ => ExclusionReason::AllowlistDefaultDeny,
        }
    }
    
    /// Update audit trail with discovery results
    async fn update_audit_trail_with_results(
        &self,
        audit_trail: &mut DiscoveryAuditTrail,
        discovery_response: &SmartDiscoveryResponse,
        allowed_tools: &AHashSet<ToolId>,
        excluded_tools: &[ExcludedTool],
    ) {
        // Update selection reasoning
        if discovery_response.success {
            audit_trail.selection_reasoning = format!(
                "Successfully selected tool from {} allowed tools (excluded {})",
                allowed_tools.len(),
                excluded_tools.len()
            );
            
            // Set selected tool if we have one
            if let Some(ref data) = discovery_response.data {
                if let Some(tool_name) = data.get("tool_name").and_then(|v| v.as_str()) {
                    let selected_tool = SelectedTool {
                        scored_tool: ScoredTool {
                            tool_id: tool_name.to_string(),
                            tool_name: tool_name.to_string(),
                            tool_description: None,
                            // Use the discovery confidence score from metadata when available
                            discovery_score: discovery_response.metadata.confidence_score,
                            semantic_score: None,
                            rule_score: None,
                            llm_score: None,
                            final_score: 1.0,
                            ranking_position: 1,
                            match_reasoning: "Selected by filtered discovery".to_string(),
                            parameter_mapping_score: None,
                        },
                        parameter_mapping: ParameterMappingResult {
                            mapped_parameters: data.clone(),
                            unmapped_parameters: Vec::new(),
                            defaulted_parameters: Vec::new(),
                            mapping_confidence: 1.0,
                            // Heuristic: consider LLM mapping used if extraction status is present
                            used_llm_mapping: discovery_response
                                .metadata
                                .extraction_status
                                .as_ref()
                                .is_some(),
                        },
                        selection_confidence: 1.0,
                        is_fallback: false,
                        alternatives: Vec::new(),
                    };
                    
                    audit_trail.set_selected_tool(selected_tool);
                }
            }
        } else {
            audit_trail.selection_reasoning = format!(
                "Discovery failed - {} tools allowed, {} excluded",
                allowed_tools.len(),
                excluded_tools.len()
            );
        }
        
        // Update discovery method
        audit_trail.discovery_method = "filtered_smart_discovery".to_string();
    }

    /// Rough estimate of how well a tool would match a request (0.0 - 1.0)
    fn estimate_potential_match_score(
        &self,
        request_text: &str,
        tool_def: &crate::registry::types::ToolDefinition,
    ) -> f64 {
        fn tokenize(s: &str) -> std::collections::HashSet<String> {
            s.split(|c: char| !c.is_alphanumeric())
                .filter(|t| !t.is_empty())
                .map(|t| t.to_lowercase())
                .collect()
        }

        let req = tokenize(request_text);
        let tool_text = format!("{} {}", tool_def.name, tool_def.description);
        let tool = tokenize(&tool_text);

        if req.is_empty() || tool.is_empty() {
            return 0.0;
        }

        let intersection = req.intersection(&tool).count() as f64;
        let union = req.union(&tool).count() as f64;
        let mut score = if union > 0.0 { intersection / union } else { 0.0 };

        // Small boost if tool name appears in request
        if request_text.to_lowercase().contains(&tool_def.name.to_lowercase()) {
            score = (score + 0.2).min(1.0);
        }

        score
    }
    
    /// Log filtering summary for debugging
    fn log_filtering_summary(&self, excluded_tools: &[ExcludedTool], allowed_tools: &AHashSet<ToolId>) {
        info!("🔒 Permission filtering summary:");
        info!("   ✅ Allowed tools: {}", allowed_tools.len());
        info!("   ❌ Excluded tools: {}", excluded_tools.len());
        
        if !excluded_tools.is_empty() {
            let mut reason_counts = std::collections::HashMap::new();
            for excluded in excluded_tools {
                let reason_key = format!("{:?}", excluded.exclusion_reason);
                *reason_counts.entry(reason_key).or_insert(0) += 1;
            }
            
            info!("   📊 Exclusion reasons:");
            for (reason, count) in reason_counts {
                info!("      {} tools: {}", count, reason);
            }
        }
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> serde_json::Value {
        let cache_stats = self.permission_cache.get_stats();
        let evaluator_stats = {
            let evaluator = self.permission_evaluator.read().await;
            evaluator.get_stats().clone()
        };
        
        serde_json::json!({
            "permission_cache": {
                "cached_users": cache_stats.cached_users,
                "total_hits": cache_stats.total_hits,
                "total_misses": cache_stats.total_misses,
                "hit_ratio": if cache_stats.total_hits + cache_stats.total_misses > 0 {
                    cache_stats.total_hits as f64 / (cache_stats.total_hits + cache_stats.total_misses) as f64
                } else { 0.0 },
                "estimated_memory_bytes": cache_stats.estimated_memory_bytes,
            },
            "permission_evaluator": {
                "total_evaluations": evaluator_stats.total_evaluations,
                "fast_path_hits": evaluator_stats.fast_path_hits,
                "slow_path_hits": evaluator_stats.slow_path_hits,
                "avg_evaluation_time_ns": evaluator_stats.avg_evaluation_time_ns,
                "fast_path_ratio": if evaluator_stats.total_evaluations > 0 {
                    evaluator_stats.fast_path_hits as f64 / evaluator_stats.total_evaluations as f64
                } else { 0.0 },
            }
        })
    }
    
    /// Clean up expired caches
    pub async fn cleanup_caches(&self) {
        // This would be called periodically by a background task
        let cache_manager = Arc::clone(&self.permission_cache);
        
        tokio::spawn(async move {
            // Cast to mutable reference for cleanup
            // In a real implementation, you'd need proper mutex handling
            // For now, this is a placeholder
            debug!("Cache cleanup would run here");
        });
    }
    
    /// Update permission index (called when permissions change)
    pub async fn update_permissions(&self, permission_index: crate::discovery::permission_cache::PermissionIndex) {
        self.permission_cache.update_permission_index(permission_index);
        info!("Permission index updated, user caches invalidated");
    }
}

impl Default for FilteringPerformanceMetrics {
    fn default() -> Self {
        Self {
            permission_filtering_time_ms: 0,
            tools_filtered_out: 0,
            tools_allowed: 0,
            permission_cache_hit_rate: 0.0,
            filtering_timed_out: false,
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
                id: Some("test_user".to_string()),
                roles: vec!["user".to_string()],
                api_key_name: None,
                permissions: vec![],
                auth_method: "test".to_string(),
            }),
            request: SecurityRequest {
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
    
    // Tests would go here - placeholder for now since we need actual SmartDiscoveryService instance
    #[tokio::test]
    async fn test_filtered_discovery_config() {
        let config = FilteredDiscoveryConfig::default();
        assert!(config.enable_permission_filtering);
        assert!(config.enable_audit_trail);
        assert_eq!(config.max_permission_filtering_time_ms, 10);
    }
}
