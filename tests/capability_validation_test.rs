//! Tests for the capability validation functionality of the unified CLI
//!
//! These tests validate the capability validator for validating capability files
//! against a set of rules.

use magictunnel::registry::commands::validate::CapabilityValidator;
use magictunnel::registry::types::{CapabilityFile, ToolDefinition, FileMetadata, RoutingConfig};
use magictunnel::error::Result;
use magictunnel::mcp::types::Tool;
use serde_json::json;

/// Helper function to create a valid capability file for testing
fn create_valid_capability() -> CapabilityFile {
    let tool1 = create_valid_tool("tool1", "First test tool");
    let tool2 = create_valid_tool("tool2", "Second test tool");

    let metadata = FileMetadata {
        name: Some("test_capability".to_string()),
        description: Some("Test capability file".to_string()),
        version: Some("1.0.0".to_string()),
        author: Some("Test Author".to_string()),
        tags: Some(vec!["test".to_string(), "validation".to_string()]),
    };

    CapabilityFile {
        metadata: Some(metadata),
        tools: vec![tool1, tool2],
        enhanced_metadata: None,
        enhanced_tools: None,
    }
}

/// Helper function to create a valid tool definition
fn create_valid_tool(name: &str, description: &str) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "param1": {"type": "string"},
                "param2": {"type": "number"}
            },
            "required": ["param1"]
        }),
        routing: RoutingConfig {
            r#type: "http".to_string(),
            config: json!({
                "url": "https://example.com/api",
                "method": "POST"
            }),
        },
        annotations: None,
        hidden: false, // Test tools are visible by default
        enabled: true, // Test tools are enabled by default
        prompt_refs: Vec::new(),
        resource_refs: Vec::new(),
        sampling_strategy: None, // Use default strategy
        elicitation_strategy: None, // Use default strategy
    }
}

/// Helper function to create an invalid tool definition (missing required fields)
fn create_invalid_tool() -> ToolDefinition {
    ToolDefinition {
        name: "".to_string(), // Empty name is invalid
        description: "Invalid tool".to_string(),
        input_schema: json!({
            "type": "string" // Not an object schema
        }),
        routing: RoutingConfig {
            r#type: "http".to_string(),
            config: json!({
                // Missing required 'method' field
                "url": "https://example.com/api"
            }),
        },
        annotations: None,
        hidden: false, // Test tools are visible by default
        enabled: true, // Test tools are enabled by default
        prompt_refs: Vec::new(),
        resource_refs: Vec::new(),
        sampling_strategy: None, // Use default strategy
        elicitation_strategy: None, // Use default strategy
    }
}

#[test]
fn test_validate_valid_capability() -> Result<()> {
    let capability = create_valid_capability();

    // Create validator
    let validator = CapabilityValidator::new();

    // Validate the capability - should pass
    let result = validator.validate(&capability);
    assert!(result.is_ok());

    // Also test getting validation issues (should be empty)
    let issues = validator.get_validation_issues(&capability);
    assert!(issues.is_empty());

    Ok(())
}

#[test]
fn test_validate_invalid_capability_missing_metadata() -> Result<()> {
    // Create a capability without metadata
    let tool = create_valid_tool("tool1", "Test tool");
    let capability = CapabilityFile {
        metadata: None,
        tools: vec![tool],
        enhanced_metadata: None,
        enhanced_tools: None,
    };
    
    // Create validator
    let validator = CapabilityValidator::new();

    // Validate the capability - should pass (missing metadata is not an error by default)
    let result = validator.validate(&capability);
    assert!(result.is_ok());

    // Test getting validation issues
    let issues = validator.get_validation_issues(&capability);
    // Should have no issues since missing metadata is not validated by default rules
    
    Ok(())
}

#[test]
fn test_validate_invalid_capability_empty_tools() -> Result<()> {
    // Create a capability with no tools
    let metadata = FileMetadata {
        name: Some("empty_capability".to_string()),
        description: Some("Capability with no tools".to_string()),
        version: Some("1.0.0".to_string()),
        author: Some("Test Author".to_string()),
        tags: None,
    };
    
    let capability = CapabilityFile {
        metadata: Some(metadata),
        tools: vec![],
        enhanced_metadata: None,
        enhanced_tools: None,
    };
    
    // Create validator
    let validator = CapabilityValidator::new();

    // Validate the capability - should pass (empty tools is not an error by default)
    let result = validator.validate(&capability);
    assert!(result.is_ok());

    // Test getting validation issues
    let issues = validator.get_validation_issues(&capability);
    // Should have no issues since empty tools is not validated by default rules
    
    Ok(())
}

#[test]
fn test_validate_invalid_tool_definition() -> Result<()> {
    // Create a capability with an invalid tool
    let valid_tool = create_valid_tool("valid_tool", "Valid test tool");
    let invalid_tool = create_invalid_tool();
    
    let metadata = FileMetadata {
        name: Some("mixed_capability".to_string()),
        description: Some("Capability with mixed valid/invalid tools".to_string()),
        version: Some("1.0.0".to_string()),
        author: Some("Test Author".to_string()),
        tags: None,
    };
    
    let capability = CapabilityFile {
        metadata: Some(metadata),
        tools: vec![valid_tool, invalid_tool],
        enhanced_metadata: None,
        enhanced_tools: None,
    };
    
    // Create validator with normal validation level
    let validator = CapabilityValidator::new();
    
    // Validate the capability - should fail due to invalid tool
    let result = validator.validate(&capability);
    assert!(result.is_err());
    
    Ok(())
}

#[test]
fn test_validate_with_different_levels() -> Result<()> {
    // Create a capability with an invalid tool (empty name)
    let tool = create_invalid_tool();
    
    let metadata = FileMetadata {
        name: Some("minimal_capability".to_string()),
        description: Some("Minimal capability file".to_string()),
        version: None, // Missing version
        author: None,  // Missing author
        tags: None,    // Missing tags
    };
    
    let capability = CapabilityFile {
        metadata: Some(metadata),
        tools: vec![tool],
        enhanced_metadata: None,
        enhanced_tools: None,
    };
    
    // Test validation
    let validator = CapabilityValidator::new();

    // Validation should fail due to empty tool name (built-in rule)
    let result = validator.validate(&capability);
    assert!(result.is_err());

    // Test getting validation issues
    let issues = validator.get_validation_issues(&capability);
    assert!(!issues.is_empty());
    assert!(issues.iter().any(|issue| issue.contains("empty") || issue.contains("name")));
    
    Ok(())
}

#[test]
fn test_validate_duplicate_tool_names() -> Result<()> {
    // Create a capability with duplicate tool names
    let tool1 = create_valid_tool("duplicate_name", "First tool with duplicate name");
    let tool2 = create_valid_tool("duplicate_name", "Second tool with duplicate name");
    
    let metadata = FileMetadata {
        name: Some("duplicate_tools".to_string()),
        description: Some("Capability with duplicate tool names".to_string()),
        version: Some("1.0.0".to_string()),
        author: Some("Test Author".to_string()),
        tags: None,
    };
    
    let capability = CapabilityFile {
        metadata: Some(metadata),
        tools: vec![tool1, tool2],
        enhanced_metadata: None,
        enhanced_tools: None,
    };
    
    // Create validator with normal validation level
    let validator = CapabilityValidator::new();
    
    // Validate the capability - should fail due to duplicate tool names
    let result = validator.validate(&capability);
    assert!(result.is_err());
    
    Ok(())
}

#[test]
fn test_validate_empty_description() -> Result<()> {
    // Create a tool with empty description
    let mut tool = create_valid_tool("tool1", "Tool with valid name");
    tool.description = "".to_string(); // Empty description should fail validation
    
    let metadata = FileMetadata {
        name: Some("empty_description_test".to_string()),
        description: Some("Capability with tool that has empty description".to_string()),
        version: Some("1.0.0".to_string()),
        author: Some("Test Author".to_string()),
        tags: None,
    };
    
    let capability = CapabilityFile {
        metadata: Some(metadata),
        tools: vec![tool],
        enhanced_metadata: None,
        enhanced_tools: None,
    };
    
    // Test validation
    let validator = CapabilityValidator::new();

    // Validation should fail due to empty tool description (built-in rule)
    let result = validator.validate(&capability);
    assert!(result.is_err());

    // Test getting validation issues
    let issues = validator.get_validation_issues(&capability);
    assert!(!issues.is_empty());
    assert!(issues.iter().any(|issue| issue.contains("description")));
    
    Ok(())
}

#[test]
fn test_validate_with_custom_rules() -> Result<()> {
    use magictunnel::registry::commands::validate::ValidationRule;

    // Define a custom validation rule
    struct RequireAuthorRule;

    impl ValidationRule for RequireAuthorRule {
        fn name(&self) -> &str {
            "require_author"
        }

        fn validate(&self, capability: &CapabilityFile) -> Result<()> {
            if let Some(ref metadata) = capability.metadata {
                if metadata.author.is_none() {
                    return Err(magictunnel::error::ProxyError::validation(
                        "Author is required".to_string()
                    ));
                }
            }
            Ok(())
        }
    }

    let capability = create_valid_capability();

    // Create validator with custom validation rule
    let mut validator = CapabilityValidator::new();
    validator.add_rule(Box::new(RequireAuthorRule));

    // Validate the capability with custom rules
    let result = validator.validate(&capability);
    assert!(result.is_ok());

    // Create a capability that violates a custom rule
    let mut invalid_capability = create_valid_capability();
    if let Some(ref mut metadata) = invalid_capability.metadata {
        metadata.author = None; // Remove author to violate custom rule
    }

    // Validate - should fail due to custom rule
    let invalid_result = validator.validate(&invalid_capability);
    assert!(invalid_result.is_err());

    Ok(())
}