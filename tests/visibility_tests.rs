//! Tests for the tool visibility system
//! 
//! This module tests the hidden flag functionality, CLI commands, and visibility management.

use magictunnel::registry::types::{ToolDefinition, CapabilityFile, RoutingConfig};
use magictunnel::registry::service::RegistryService;
use magictunnel::config::{Config, RegistryConfig, VisibilityConfig, ValidationConfig};
use serde_json::json;
use tempfile::TempDir;
use std::fs;

#[tokio::test]
async fn test_tool_definition_hidden_field() {
    // Test that ToolDefinition properly handles the hidden field
    let routing = RoutingConfig {
        r#type: "subprocess".to_string(),
        config: json!({"command": "echo", "args": ["test"]}),
    };

    // Test default hidden value (false)
    let tool = ToolDefinition::new_with_fields(
        "test_tool".to_string(),
        "Test tool".to_string(),
        json!({"type": "object", "properties": {}}),
        routing.clone(),
        None,
    ).unwrap();
    
    assert!(!tool.is_hidden(), "Tool should be visible by default");

    // Test explicit hidden value
    let hidden_tool = ToolDefinition::new_with_hidden(
        "hidden_tool".to_string(),
        "Hidden test tool".to_string(),
        json!({"type": "object", "properties": {}}),
        routing,
        None,
        true,
    ).unwrap();
    
    assert!(hidden_tool.is_hidden(), "Tool should be hidden when explicitly set");

    // Test visibility helper methods
    let mut mutable_tool = tool.clone();
    mutable_tool.set_hidden(true);
    assert!(mutable_tool.is_hidden(), "Tool should be hidden after set_hidden(true)");

    let visible_copy = mutable_tool.with_hidden(false);
    assert!(!visible_copy.is_hidden(), "Copied tool should be visible");
    assert!(mutable_tool.is_hidden(), "Original tool should still be hidden");
}

#[tokio::test]
async fn test_capability_file_visibility_methods() {
    // Create a test capability file with mixed visibility
    let routing = RoutingConfig {
        r#type: "subprocess".to_string(),
        config: json!({"command": "echo", "args": ["test"]}),
    };

    let visible_tool = ToolDefinition::new_with_hidden(
        "visible_tool".to_string(),
        "Visible tool".to_string(),
        json!({"type": "object", "properties": {}}),
        routing.clone(),
        None,
        false,
    ).unwrap();

    let hidden_tool = ToolDefinition::new_with_hidden(
        "hidden_tool".to_string(),
        "Hidden tool".to_string(),
        json!({"type": "object", "properties": {}}),
        routing,
        None,
        true,
    ).unwrap();

    let mut capability_file = CapabilityFile {
        metadata: None,
        tools: vec![visible_tool, hidden_tool],
    };

    // Test visibility counting methods
    assert_eq!(capability_file.tool_count(), 2);
    assert_eq!(capability_file.visible_tool_count(), 1);
    assert_eq!(capability_file.hidden_tool_count(), 1);

    // Test visibility filtering methods
    let visible_tools = capability_file.visible_tools();
    assert_eq!(visible_tools.len(), 1);
    assert_eq!(visible_tools[0].name, "visible_tool");

    let hidden_tools = capability_file.hidden_tools();
    assert_eq!(hidden_tools.len(), 1);
    assert_eq!(hidden_tools[0].name, "hidden_tool");

    // Test bulk visibility changes
    capability_file.set_all_tools_hidden(true);
    assert_eq!(capability_file.visible_tool_count(), 0);
    assert_eq!(capability_file.hidden_tool_count(), 2);

    capability_file.set_all_tools_hidden(false);
    assert_eq!(capability_file.visible_tool_count(), 2);
    assert_eq!(capability_file.hidden_tool_count(), 0);

    // Test individual tool visibility changes
    capability_file.set_tool_hidden("visible_tool", true).unwrap();
    assert_eq!(capability_file.visible_tool_count(), 1);
    assert_eq!(capability_file.hidden_tool_count(), 1);

    // Test tool visibility query
    assert_eq!(capability_file.is_tool_hidden("visible_tool"), Some(true));
    assert_eq!(capability_file.is_tool_hidden("hidden_tool"), Some(false));
    assert_eq!(capability_file.is_tool_hidden("nonexistent"), None);

    // Test error handling for nonexistent tool
    let result = capability_file.set_tool_hidden("nonexistent", true);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_registry_service_visibility_filtering() {
    // Create a temporary directory for test capability files
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    // Create test capability file with mixed visibility
    let test_capability = r#"
metadata:
  name: "Test Visibility"
  description: "Test capability file for visibility"
  version: "1.0.0"

tools:
  - name: "visible_tool"
    description: "A visible tool"
    inputSchema:
      type: "object"
      properties: {}
    routing:
      type: "subprocess"
      config:
        command: "echo"
        args: ["visible"]
    hidden: false

  - name: "hidden_tool"
    description: "A hidden tool"
    inputSchema:
      type: "object"
      properties: {}
    routing:
      type: "subprocess"
      config:
        command: "echo"
        args: ["hidden"]
    hidden: true
"#;

    let test_file_path = temp_dir.path().join("test_visibility.yaml");
    fs::write(&test_file_path, test_capability).unwrap();

    // Create registry configuration
    let registry_config = RegistryConfig {
        r#type: "file".to_string(),
        paths: vec![temp_path.to_string()],
        hot_reload: false,
        validation: ValidationConfig::default(),
    };

    let config = Config {
        registry: registry_config,
        ..Default::default()
    };

    // Create registry service
    let registry_service = RegistryService::new(config.registry.clone()).await.unwrap();

    // Test that only visible tools are returned by default
    let visible_tools = registry_service.list_tools();
    assert_eq!(visible_tools.len(), 1);
    assert!(visible_tools.contains(&"visible_tool".to_string()));
    assert!(!visible_tools.contains(&"hidden_tool".to_string()));

    // Test that all tools are returned when explicitly requested
    let all_tools = registry_service.list_all_tools();
    assert_eq!(all_tools.len(), 2);
    assert!(all_tools.contains(&"visible_tool".to_string()));
    assert!(all_tools.contains(&"hidden_tool".to_string()));

    // Test hidden tools list
    let hidden_tools = registry_service.list_hidden_tools();
    assert_eq!(hidden_tools.len(), 1);
    assert!(hidden_tools.contains(&"hidden_tool".to_string()));

    // Test visibility statistics
    let (total, visible, hidden) = registry_service.visibility_stats();
    assert_eq!(total, 2);
    assert_eq!(visible, 1);
    assert_eq!(hidden, 1);

    // Test individual tool visibility check
    assert_eq!(registry_service.is_tool_hidden("visible_tool"), Some(false));
    assert_eq!(registry_service.is_tool_hidden("hidden_tool"), Some(true));
    assert_eq!(registry_service.is_tool_hidden("nonexistent"), None);
}

#[tokio::test]
async fn test_visibility_config_defaults() {
    // Test VisibilityConfig default values
    let visibility_config = VisibilityConfig::default();
    
    assert!(!visibility_config.hide_individual_tools);
    assert!(!visibility_config.expose_smart_discovery_only);
    assert!(visibility_config.allow_override);
    assert!(!visibility_config.default_hidden);
}

#[tokio::test]
async fn test_serde_hidden_field() {
    // Test that the hidden field is properly serialized/deserialized
    let routing = RoutingConfig {
        r#type: "subprocess".to_string(),
        config: json!({"command": "echo", "args": ["test"]}),
    };

    let tool = ToolDefinition::new_with_hidden(
        "test_tool".to_string(),
        "Test tool".to_string(),
        json!({"type": "object", "properties": {}}),
        routing,
        None,
        true,
    ).unwrap();

    // Serialize to YAML
    let yaml_str = serde_yaml::to_string(&tool).unwrap();
    assert!(yaml_str.contains("hidden: true"));

    // Deserialize from YAML
    let deserialized_tool: ToolDefinition = serde_yaml::from_str(&yaml_str).unwrap();
    assert!(deserialized_tool.is_hidden());

    // Test default value when hidden field is missing
    let yaml_without_hidden = r#"
name: "test_tool"
description: "Test tool"
inputSchema:
  type: "object"
  properties: {}
routing:
  type: "subprocess"
  config:
    command: "echo"
    args: ["test"]
annotations: null
"#;

    let tool_without_hidden: ToolDefinition = serde_yaml::from_str(yaml_without_hidden).unwrap();
    assert!(!tool_without_hidden.is_hidden(), "Tool should default to visible when hidden field is missing");
}
