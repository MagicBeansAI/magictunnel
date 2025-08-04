//! Conflict resolution strategies for handling duplicate tool names from multiple sources
//! Conflict resolution for External MCP system

use crate::error::{ProxyError, Result};
use crate::registry::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Simplified capability source for conflict resolution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CapabilitySource {
    /// Local capability from registry
    Local,
    /// Remote capability from remote MCP discovery
    Remote { server_name: String },
}

impl CapabilitySource {
    /// Check if this is a local (direct) capability source
    pub fn is_direct(&self) -> bool {
        matches!(self, CapabilitySource::Local)
    }

    /// Check if this is a remote MCP proxy capability source
    pub fn is_mcp_proxy(&self) -> bool {
        matches!(self, CapabilitySource::Remote { .. })
    }

    /// Get the server name if this is a remote source
    pub fn server_name(&self) -> Option<&str> {
        match self {
            CapabilitySource::Local => None,
            CapabilitySource::Remote { server_name } => Some(server_name),
        }
    }
}

/// Strategy for resolving conflicts when the same tool name exists in multiple sources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictResolutionStrategy {
    /// Local tools take precedence over MCP proxy tools
    LocalFirst,
    /// MCP proxy tools take precedence over local tools
    ProxyFirst,
    /// Use the first tool found (discovery order dependent)
    FirstFound,
    /// Reject tools with conflicting names (error on conflict)
    Reject,
    /// Create prefixed names for conflicting tools (e.g., "local:tool", "server:tool")
    Prefix,
}

/// Configuration for conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConflictResolutionConfig {
    /// Primary resolution strategy
    pub strategy: ConflictResolutionStrategy,
    /// Prefix for local tools when using Prefix strategy
    pub local_prefix: String,
    /// Prefix format for MCP proxy tools when using Prefix strategy
    /// Use {server} placeholder for server name
    pub proxy_prefix_format: String,
    /// Whether to log conflict resolutions
    pub log_conflicts: bool,
    /// Whether to include conflict metadata in tool definitions
    pub include_conflict_metadata: bool,
}

impl Default for ConflictResolutionConfig {
    fn default() -> Self {
        Self {
            strategy: ConflictResolutionStrategy::LocalFirst,
            local_prefix: "local".to_string(),
            proxy_prefix_format: "{server}".to_string(),
            log_conflicts: true,
            include_conflict_metadata: true,
        }
    }
}

/// Information about a tool conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Original tool name that caused the conflict
    pub original_name: String,
    /// Sources involved in the conflict
    pub conflicting_sources: Vec<ConflictSource>,
    /// Resolution strategy used
    pub resolution_strategy: ConflictResolutionStrategy,
    /// Final resolved names
    pub resolved_names: Vec<String>,
}

/// Information about a source involved in a conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictSource {
    /// Type of source (local or mcp_proxy)
    pub source_type: String,
    /// Server name (for MCP proxy sources)
    pub server_name: Option<String>,
    /// Original tool name
    pub original_name: String,
    /// Resolved name after conflict resolution
    pub resolved_name: String,
}

/// Conflict resolver for handling tool name conflicts
pub struct ConflictResolver {
    /// Configuration for conflict resolution
    config: ConflictResolutionConfig,
    /// Track conflicts that have been resolved
    resolved_conflicts: HashMap<String, ConflictInfo>,
}

impl ConflictResolver {
    /// Create a new conflict resolver
    pub fn new(config: ConflictResolutionConfig) -> Self {
        info!("Creating conflict resolver with strategy: {:?}", config.strategy);
        Self {
            config,
            resolved_conflicts: HashMap::new(),
        }
    }

    /// Resolve conflicts in a collection of tool definitions
    pub fn resolve_conflicts(
        &mut self,
        tools: Vec<(String, ToolDefinition, CapabilitySource)>,
    ) -> Result<Vec<(String, ToolDefinition, CapabilitySource)>> {
        debug!("Resolving conflicts for {} tools", tools.len());

        // Group tools by name to identify conflicts, preserving order
        let mut tool_groups: HashMap<String, Vec<(String, ToolDefinition, CapabilitySource)>> = HashMap::new();
        let mut name_order = Vec::new();

        for (name, tool_def, source) in tools {
            if !tool_groups.contains_key(&name) {
                name_order.push(name.clone());
            }
            tool_groups.entry(name.clone()).or_default().push((name, tool_def, source));
        }

        let mut resolved_tools = Vec::new();
        let mut conflicts_found = 0;

        // Process tools in original order
        for original_name in name_order {
            let tool_group = tool_groups.remove(&original_name).unwrap();
            if tool_group.len() == 1 {
                // No conflict, use original tool
                resolved_tools.extend(tool_group);
            } else {
                // Conflict detected
                conflicts_found += 1;
                if self.config.log_conflicts {
                    warn!("Tool name conflict detected for '{}' ({} sources)", original_name, tool_group.len());
                }

                let resolved = self.resolve_single_conflict(original_name, tool_group)?;
                resolved_tools.extend(resolved);
            }
        }

        if conflicts_found > 0 {
            info!("Resolved {} tool name conflicts using strategy: {:?}",
                  conflicts_found, self.config.strategy);
        }

        Ok(resolved_tools)
    }

    /// Resolve a single conflict for tools with the same name
    fn resolve_single_conflict(
        &mut self,
        original_name: String,
        tools: Vec<(String, ToolDefinition, CapabilitySource)>,
    ) -> Result<Vec<(String, ToolDefinition, CapabilitySource)>> {
        match self.config.strategy {
            ConflictResolutionStrategy::LocalFirst => {
                self.resolve_local_first(original_name, tools)
            }
            ConflictResolutionStrategy::ProxyFirst => {
                self.resolve_proxy_first(original_name, tools)
            }
            ConflictResolutionStrategy::FirstFound => {
                self.resolve_first_found(original_name, tools)
            }
            ConflictResolutionStrategy::Reject => {
                self.resolve_reject(original_name, tools)
            }
            ConflictResolutionStrategy::Prefix => {
                self.resolve_prefix(original_name, tools)
            }
        }
    }

    /// Resolve conflict by preferring local tools
    fn resolve_local_first(
        &mut self,
        original_name: String,
        tools: Vec<(String, ToolDefinition, CapabilitySource)>,
    ) -> Result<Vec<(String, ToolDefinition, CapabilitySource)>> {
        // Find the first local tool
        for (name, tool_def, source) in &tools {
            if source.is_direct() {
                debug!("LocalFirst: Using local tool '{}' over {} alternatives", name, tools.len() - 1);
                return Ok(vec![(name.clone(), tool_def.clone(), source.clone())]);
            }
        }

        // No local tool found, use the first proxy tool
        if let Some((name, tool_def, source)) = tools.into_iter().next() {
            debug!("LocalFirst: No local tool found, using first proxy tool '{}'", name);
            Ok(vec![(name, tool_def, source)])
        } else {
            Err(ProxyError::validation("No tools provided for conflict resolution".to_string()))
        }
    }

    /// Resolve conflict by preferring proxy tools
    fn resolve_proxy_first(
        &mut self,
        original_name: String,
        tools: Vec<(String, ToolDefinition, CapabilitySource)>,
    ) -> Result<Vec<(String, ToolDefinition, CapabilitySource)>> {
        // Find the first proxy tool
        for (name, tool_def, source) in &tools {
            if source.is_mcp_proxy() {
                debug!("ProxyFirst: Using proxy tool '{}' over {} alternatives", name, tools.len() - 1);
                return Ok(vec![(name.clone(), tool_def.clone(), source.clone())]);
            }
        }

        // No proxy tool found, use the first local tool
        if let Some((name, tool_def, source)) = tools.into_iter().next() {
            debug!("ProxyFirst: No proxy tool found, using first local tool '{}'", name);
            Ok(vec![(name, tool_def, source)])
        } else {
            Err(ProxyError::validation("No tools provided for conflict resolution".to_string()))
        }
    }

    /// Resolve conflict by using the first tool found
    fn resolve_first_found(
        &mut self,
        original_name: String,
        tools: Vec<(String, ToolDefinition, CapabilitySource)>,
    ) -> Result<Vec<(String, ToolDefinition, CapabilitySource)>> {
        let tool_count = tools.len();
        if let Some((name, tool_def, source)) = tools.into_iter().next() {
            debug!("FirstFound: Using first tool '{}' from {} alternatives", name, tool_count - 1);
            Ok(vec![(name, tool_def, source)])
        } else {
            Err(ProxyError::validation("No tools provided for conflict resolution".to_string()))
        }
    }

    /// Resolve conflict by rejecting all conflicting tools
    fn resolve_reject(
        &mut self,
        original_name: String,
        tools: Vec<(String, ToolDefinition, CapabilitySource)>,
    ) -> Result<Vec<(String, ToolDefinition, CapabilitySource)>> {
        let source_info: Vec<String> = tools.iter()
            .map(|(_, _, source)| match source {
                CapabilitySource::Local => "local".to_string(),
                CapabilitySource::Remote { server_name } => format!("remote:{}", server_name),
            })
            .collect();

        Err(ProxyError::validation(format!(
            "Tool name conflict for '{}' rejected (sources: {}). Use a different conflict resolution strategy.",
            original_name,
            source_info.join(", ")
        )))
    }

    /// Resolve conflict by adding prefixes to tool names
    fn resolve_prefix(
        &mut self,
        original_name: String,
        tools: Vec<(String, ToolDefinition, CapabilitySource)>,
    ) -> Result<Vec<(String, ToolDefinition, CapabilitySource)>> {
        let mut resolved_tools = Vec::new();
        let mut conflict_sources = Vec::new();
        let mut resolved_names = Vec::new();

        for (_, mut tool_def, source) in tools {
            let new_name = match &source {
                CapabilitySource::Local => {
                    format!("{}:{}", self.config.local_prefix, original_name)
                }
                CapabilitySource::Remote { server_name } => {
                    let prefix = self.config.proxy_prefix_format.replace("{server}", server_name);
                    format!("{}:{}", prefix, original_name)
                }
            };

            // Update tool definition name
            tool_def.name = new_name.clone();

            // Add conflict metadata if configured
            if self.config.include_conflict_metadata {
                tool_def.annotations.get_or_insert_with(HashMap::new)
                    .insert("original_name".to_string(), original_name.clone());
                tool_def.annotations.as_mut().unwrap()
                    .insert("conflict_resolved".to_string(), "true".to_string());
            }

            conflict_sources.push(ConflictSource {
                source_type: if source.is_direct() { "local" } else { "remote" }.to_string(),
                server_name: source.server_name().map(|s| s.to_string()),
                original_name: original_name.clone(),
                resolved_name: new_name.clone(),
            });

            resolved_names.push(new_name.clone());
            resolved_tools.push((new_name, tool_def, source));
        }

        // Record the conflict resolution
        let conflict_info = ConflictInfo {
            original_name: original_name.clone(),
            conflicting_sources: conflict_sources,
            resolution_strategy: ConflictResolutionStrategy::Prefix,
            resolved_names,
        };

        self.resolved_conflicts.insert(original_name, conflict_info);

        debug!("Prefix: Resolved conflict by creating {} prefixed tools", resolved_tools.len());
        Ok(resolved_tools)
    }

    /// Get information about resolved conflicts
    pub fn get_conflict_info(&self, original_name: &str) -> Option<&ConflictInfo> {
        self.resolved_conflicts.get(original_name)
    }

    /// Get all resolved conflicts
    pub fn get_all_conflicts(&self) -> &HashMap<String, ConflictInfo> {
        &self.resolved_conflicts
    }

    /// Clear conflict history
    pub fn clear_conflicts(&mut self) {
        self.resolved_conflicts.clear();
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ConflictResolutionConfig) {
        info!("Updating conflict resolution config: {:?}", config.strategy);
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &ConflictResolutionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::RoutingConfig;
    use serde_json::json;

    fn create_test_tool(name: &str, source: CapabilitySource) -> (String, ToolDefinition, CapabilitySource) {
        let tool_def = ToolDefinition {
            name: name.to_string(),
            description: format!("Test tool {}", name),
            input_schema: json!({"type": "object"}),
            routing: RoutingConfig {
                r#type: "subprocess".to_string(),
                config: json!({"command": "echo"}),
            },
            annotations: None,
            hidden: false, // Test tools are visible by default
            enabled: true, // Test tools are enabled by default
            prompt_refs: Vec::new(),
            resource_refs: Vec::new(),
            sampling_strategy: None,
            elicitation_strategy: None,
        };
        (name.to_string(), tool_def, source)
    }

    #[test]
    fn test_no_conflicts() {
        let mut resolver = ConflictResolver::new(ConflictResolutionConfig::default());
        
        let tools = vec![
            create_test_tool("tool1", CapabilitySource::Local),
            create_test_tool("tool2", CapabilitySource::Remote {
                server_name: "server1".to_string(),
            }),
        ];

        let result = resolver.resolve_conflicts(tools).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "tool1");
        assert_eq!(result[1].0, "tool2");
    }

    #[test]
    fn test_local_first_strategy() {
        let mut resolver = ConflictResolver::new(ConflictResolutionConfig {
            strategy: ConflictResolutionStrategy::LocalFirst,
            ..Default::default()
        });

        let tools = vec![
            create_test_tool("tool1", CapabilitySource::Remote {
                server_name: "server1".to_string(),
            }),
            create_test_tool("tool1", CapabilitySource::Local),
        ];

        let result = resolver.resolve_conflicts(tools).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "tool1");
        assert!(result[0].2.is_direct());
    }

    #[test]
    fn test_prefix_strategy() {
        let mut resolver = ConflictResolver::new(ConflictResolutionConfig {
            strategy: ConflictResolutionStrategy::Prefix,
            ..Default::default()
        });

        let tools = vec![
            create_test_tool("tool1", CapabilitySource::Local),
            create_test_tool("tool1", CapabilitySource::Remote {
                server_name: "server1".to_string(),
            }),
        ];

        let result = resolver.resolve_conflicts(tools).unwrap();
        assert_eq!(result.len(), 2);
        
        let names: Vec<&str> = result.iter().map(|(name, _, _)| name.as_str()).collect();
        assert!(names.contains(&"local:tool1"));
        assert!(names.contains(&"server1:tool1"));
    }
}
