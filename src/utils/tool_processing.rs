//! Tool processing utilities
//!
//! Shared logic for determining which tools to process, skip, or enhance
//! across different parts of the MagicTunnel system.

use crate::registry::types::ToolDefinition;

/// Tools that should always be excluded from processing
pub const EXCLUDED_TOOL_NAMES: &[&str] = &[
    "smart_discovery_tool",
    "smart_tool_discovery",
];

/// Check if a tool should be excluded from enhancement/embedding processing
///
/// This includes:
/// - Smart discovery tools (to avoid recursion)
/// - Other system tools that shouldn't be processed
///
/// # Arguments
/// * `tool_name` - Name of the tool to check
///
/// # Returns
/// `true` if the tool should be excluded from processing
pub fn should_exclude_tool(tool_name: &str) -> bool {
    EXCLUDED_TOOL_NAMES.contains(&tool_name)
}

/// Check if a tool is from an external MCP server
///
/// External MCP tools are identified by their routing type which indicates
/// they come from external MCP servers rather than local YAML definitions.
///
/// # Arguments
/// * `tool_def` - Tool definition to check
///
/// # Returns
/// `true` if the tool is from an external MCP server
pub fn is_external_mcp_tool(tool_def: &ToolDefinition) -> bool {
    // Primary detection: check routing type
    matches!(tool_def.routing.r#type.as_str(), "external_mcp" | "websocket")
}

/// Check if a tool should be processed for enhancement
///
/// Combines exclusion rules and external MCP detection to determine
/// if a tool is eligible for enhancement processing.
///
/// # Arguments
/// * `tool_name` - Name of the tool
/// * `tool_def` - Tool definition
///
/// # Returns
/// `true` if the tool should be processed for enhancement
pub fn should_process_for_enhancement(tool_name: &str, tool_def: &ToolDefinition) -> bool {
    // Skip excluded tools
    if should_exclude_tool(tool_name) {
        return false;
    }
    
    // Skip external MCP tools - they get enhancements from their source servers
    if is_external_mcp_tool(tool_def) {
        return false;
    }
    
    true
}

/// Check if a tool should be included in embedding processing
///
/// Similar to enhancement processing but may have different rules
/// for semantic search and embedding generation.
///
/// # Arguments
/// * `tool_name` - Name of the tool
/// * `tool_def` - Tool definition
///
/// # Returns
/// `true` if the tool should be processed for embeddings
pub fn should_process_for_embeddings(tool_name: &str, tool_def: &ToolDefinition) -> bool {
    // Skip excluded tools (same rules as enhancement for now)
    should_process_for_enhancement(tool_name, tool_def)
}

/// Get categorized tool counts from a collection of tools
///
/// Provides a breakdown of tools by category for logging and statistics.
///
/// # Arguments
/// * `tools` - Iterator of (tool_name, tool_definition) pairs
///
/// # Returns
/// Tuple of (total_tools, regular_tools, external_mcp_tools, excluded_tools)
pub fn categorize_tools<'a, I>(tools: I) -> (usize, usize, usize, usize)
where
    I: Iterator<Item = (&'a String, &'a ToolDefinition)>,
{
    let mut total_tools = 0;
    let mut regular_tools = 0;
    let mut external_mcp_tools = 0;
    let mut excluded_tools = 0;
    
    for (tool_name, tool_def) in tools {
        total_tools += 1;
        
        if should_exclude_tool(tool_name) {
            excluded_tools += 1;
        } else if is_external_mcp_tool(tool_def) {
            external_mcp_tools += 1;
        } else {
            regular_tools += 1;
        }
    }
    
    (total_tools, regular_tools, external_mcp_tools, excluded_tools)
}

/// Filter tools for processing
///
/// Returns only tools that should be processed based on the given criteria.
///
/// # Arguments
/// * `tools` - Collection of tools to filter
/// * `include_external_mcp` - Whether to include external MCP tools
///
/// # Returns
/// Vector of tools that should be processed
pub fn filter_tools_for_processing(
    tools: &std::collections::HashMap<String, ToolDefinition>,
    include_external_mcp: bool,
) -> Vec<(String, ToolDefinition)> {
    tools
        .iter()
        .filter_map(|(name, def)| {
            if should_exclude_tool(name) {
                None
            } else if !include_external_mcp && is_external_mcp_tool(def) {
                None
            } else {
                Some((name.clone(), def.clone()))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use serde_json::json;

    use crate::registry::types::RoutingConfig;
    
    fn create_test_tool(name: &str, description: &str, routing_type: &str) -> ToolDefinition {
        let routing_config = RoutingConfig {
            r#type: routing_type.to_string(),
            config: match routing_type {
                "external_mcp" => json!({
                    "server_name": "test_server",
                    "tool_name": name
                }),
                "websocket" => json!({
                    "url": "ws://test.com"
                }),
                _ => json!({
                    "method": "GET",
                    "url": "http://test.com"
                }),
            },
        };
        
        ToolDefinition {
            name: name.to_string(),
            description: description.to_string(),
            enabled: true,
            hidden: false,
            routing: routing_config,
            input_schema: json!({}),
            annotations: None,
            prompt_refs: vec![],
            resource_refs: vec![],
            sampling_strategy: None,
            elicitation_strategy: None,
        }
    }

    #[test]
    fn test_should_exclude_tool() {
        assert!(should_exclude_tool("smart_discovery_tool"));
        assert!(should_exclude_tool("smart_tool_discovery"));
        assert!(!should_exclude_tool("regular_tool"));
        assert!(!should_exclude_tool("some_other_tool"));
    }

    #[test]
    fn test_is_external_mcp_tool() {
        // Test external_mcp routing detection
        let external_tool = create_test_tool("test_tool", "A test tool", "external_mcp");
        assert!(is_external_mcp_tool(&external_tool));

        // Test websocket routing detection  
        let websocket_tool = create_test_tool("ws_tool", "A websocket tool", "websocket");
        assert!(is_external_mcp_tool(&websocket_tool));

        // Test regular tool
        let regular_tool = create_test_tool("regular_tool", "A regular tool", "http");
        assert!(!is_external_mcp_tool(&regular_tool));
    }

    #[test]
    fn test_should_process_for_enhancement() {
        // Excluded tool
        let excluded_tool = create_test_tool("smart_discovery_tool", "Smart discovery", "http");
        assert!(!should_process_for_enhancement("smart_discovery_tool", &excluded_tool));

        // External MCP tool
        let external_tool = create_test_tool("external_tool", "External tool", "external_mcp");
        assert!(!should_process_for_enhancement("external_tool", &external_tool));

        // Regular tool
        let regular_tool = create_test_tool("regular_tool", "A regular tool", "http");
        assert!(should_process_for_enhancement("regular_tool", &regular_tool));
    }

    #[test]
    fn test_categorize_tools() {
        let mut tools = HashMap::new();
        
        // Regular tool
        tools.insert("regular".to_string(), create_test_tool("regular", "Regular tool", "http"));
        
        // Excluded tool
        tools.insert("smart_discovery_tool".to_string(), create_test_tool("smart_discovery_tool", "Smart discovery", "http"));
        
        // External MCP tool
        tools.insert("external".to_string(), create_test_tool("external", "External tool", "external_mcp"));

        let (total, regular, external, excluded) = categorize_tools(tools.iter());
        
        assert_eq!(total, 3);
        assert_eq!(regular, 1);
        assert_eq!(external, 1);
        assert_eq!(excluded, 1);
    }
}