//! Tests for the capability merging functionality of the unified CLI
//!
//! These tests validate the merging of multiple capability files with
//! different merge strategies.

use magictunnel::registry::commands::{CapabilityMerger, MergeStrategy};
use magictunnel::registry::types::{CapabilityFile, ToolDefinition, FileMetadata};
use magictunnel::error::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::tempdir;
use std::fs;
use serde_json::json;

/// Helper function to create a simple capability file for testing
fn create_test_capability(name: &str, tools: Vec<ToolDefinition>) -> CapabilityFile {
    CapabilityFile {
        metadata: Some(FileMetadata {
            name: Some(name.to_string()),
            description: Some(format!("Test capability for {}", name)),
            version: Some("1.0.0".to_string()),
            author: Some("Test Author".to_string()),
            tags: Some(vec!["test".to_string()]),
        }),
        tools,
    }
}

/// Helper function to create a test tool
fn create_test_tool(name: &str, description: &str) -> ToolDefinition {
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
        routing: magictunnel::registry::types::RoutingConfig {
            r#type: "http".to_string(),
            config: json!({
                "url": "https://example.com/api",
                "method": "POST"
            }),
        },
        annotations: None,
        hidden: false, // Test tools are visible by default
        enabled: true, // Test tools are enabled by default
    }
}

/// Helper function to write a capability file to disk and read it back
fn write_and_read_capability(
    capability: &CapabilityFile,
    dir: &tempfile::TempDir,
    filename: &str,
) -> Result<CapabilityFile> {
    let path = dir.path().join(filename);
    let content = serde_json::to_string_pretty(capability).unwrap();
    fs::write(&path, content).unwrap();
    
    let file_content = fs::read_to_string(&path)?;
    let parsed_capability: CapabilityFile = serde_json::from_str(&file_content)?;
    
    Ok(parsed_capability)
}

#[test]
fn test_basic_merge() -> Result<()> {
    // Create a temporary directory for our test files
    let temp_dir = tempdir().unwrap();
    
    // Create two capability files with different tools
    let tool1 = create_test_tool("tool1", "First test tool");
    let tool2 = create_test_tool("tool2", "Second test tool");
    
    let capability1 = create_test_capability("capability1", vec![tool1]);
    let capability2 = create_test_capability("capability2", vec![tool2]);
    
    // Write and read back the capabilities to ensure they're valid
    let cap1 = write_and_read_capability(&capability1, &temp_dir, "capability1.json")?;
    let cap2 = write_and_read_capability(&capability2, &temp_dir, "capability2.json")?;
    
    // Create a merger
    let merger = CapabilityMerger::new();
    
    // Perform the merge
    let merged = merger.merge(vec![cap1, cap2], MergeStrategy::Error)?;
    
    // Verify the merged capability
    assert_eq!(merged.tools.len(), 2);
    
    // Verify tool names
    let tool_names: Vec<String> = merged.tools.iter()
        .map(|t| t.name.clone())
        .collect();
    assert!(tool_names.contains(&"tool1".to_string()));
    assert!(tool_names.contains(&"tool2".to_string()));
    
    // Verify metadata
    let metadata = merged.metadata.unwrap();
    assert_eq!(metadata.name, Some("capability1".to_string())); // Uses first non-empty name
    let description = metadata.description.unwrap();
    assert!(description.contains("Test capability for capability1"));
    assert!(description.contains("Test capability for capability2"));
    
    Ok(())
}

#[test]
fn test_merge_with_conflicts_error_strategy() -> Result<()> {
    // Create a temporary directory for our test files
    let temp_dir = tempdir().unwrap();
    
    // Create two capability files with the same tool name but different descriptions
    let tool1a = create_test_tool("tool1", "First test tool - version A");
    let tool1b = create_test_tool("tool1", "First test tool - version B");
    
    let capability1 = create_test_capability("capability1", vec![tool1a]);
    let capability2 = create_test_capability("capability2", vec![tool1b]);
    
    // Write and read back the capabilities to ensure they're valid
    let cap1 = write_and_read_capability(&capability1, &temp_dir, "capability1.json")?;
    let cap2 = write_and_read_capability(&capability2, &temp_dir, "capability2.json")?;
    
    // Create a merger
    let merger = CapabilityMerger::new();
    
    // Perform the merge - this should fail due to the conflict
    let result = merger.merge(vec![cap1, cap2], MergeStrategy::Error);
    assert!(result.is_err());
    
    Ok(())
}

#[test]
fn test_merge_with_conflicts_keep_first_strategy() -> Result<()> {
    // Create a temporary directory for our test files
    let temp_dir = tempdir().unwrap();
    
    // Create two capability files with the same tool name but different descriptions
    let tool1a = create_test_tool("tool1", "First test tool - version A");
    let tool1b = create_test_tool("tool1", "First test tool - version B");
    let tool2 = create_test_tool("tool2", "Second test tool");
    
    let capability1 = create_test_capability("capability1", vec![tool1a]);
    let capability2 = create_test_capability("capability2", vec![tool1b, tool2]);
    
    // Write and read back the capabilities to ensure they're valid
    let cap1 = write_and_read_capability(&capability1, &temp_dir, "capability1.json")?;
    let cap2 = write_and_read_capability(&capability2, &temp_dir, "capability2.json")?;
    
    // Create a merger
    let merger = CapabilityMerger::new();
    
    // Perform the merge
    let merged = merger.merge(vec![cap1, cap2], MergeStrategy::KeepFirst)?;
    
    // Verify the merged capability
    assert_eq!(merged.tools.len(), 2);
    
    // Verify the tool description matches the first file's version
    let tool1 = merged.tools.iter().find(|t| t.name == "tool1").unwrap();
    assert_eq!(tool1.description, "First test tool - version A");
    
    Ok(())
}

#[test]
fn test_merge_with_conflicts_keep_last_strategy() -> Result<()> {
    // Create a temporary directory for our test files
    let temp_dir = tempdir().unwrap();
    
    // Create two capability files with the same tool name but different descriptions
    let tool1a = create_test_tool("tool1", "First test tool - version A");
    let tool1b = create_test_tool("tool1", "First test tool - version B");
    let tool2 = create_test_tool("tool2", "Second test tool");
    
    let capability1 = create_test_capability("capability1", vec![tool1a]);
    let capability2 = create_test_capability("capability2", vec![tool1b, tool2]);
    
    // Write and read back the capabilities to ensure they're valid
    let cap1 = write_and_read_capability(&capability1, &temp_dir, "capability1.json")?;
    let cap2 = write_and_read_capability(&capability2, &temp_dir, "capability2.json")?;
    
    // Create a merger
    let merger = CapabilityMerger::new();
    
    // Perform the merge
    let merged = merger.merge(vec![cap1, cap2], MergeStrategy::KeepLast)?;
    
    // Verify the merged capability
    assert_eq!(merged.tools.len(), 2);
    
    // Verify the tool description matches the second file's version
    let tool1 = merged.tools.iter().find(|t| t.name == "tool1").unwrap();
    assert_eq!(tool1.description, "First test tool - version B");
    
    Ok(())
}

#[test]
fn test_merge_with_conflicts_rename_strategy() -> Result<()> {
    // Create a temporary directory for our test files
    let temp_dir = tempdir().unwrap();
    
    // Create two capability files with the same tool name but different descriptions
    let tool1a = create_test_tool("tool1", "First test tool - version A");
    let tool1b = create_test_tool("tool1", "First test tool - version B");
    let tool2 = create_test_tool("tool2", "Second test tool");
    
    let capability1 = create_test_capability("capability1", vec![tool1a]);
    let capability2 = create_test_capability("capability2", vec![tool1b, tool2]);
    
    // Write and read back the capabilities to ensure they're valid
    let cap1 = write_and_read_capability(&capability1, &temp_dir, "capability1.json")?;
    let cap2 = write_and_read_capability(&capability2, &temp_dir, "capability2.json")?;
    
    // Create a merger
    let merger = CapabilityMerger::new();
    
    // Perform the merge
    let merged = merger.merge(vec![cap1, cap2], MergeStrategy::Rename)?;
    
    // Verify the merged capability
    assert_eq!(merged.tools.len(), 3);
    
    // Verify the renamed tools exist
    let tool_names: Vec<String> = merged.tools.iter()
        .map(|t| t.name.clone())
        .collect();
    assert!(tool_names.contains(&"tool1".to_string()));
    assert!(tool_names.contains(&"tool1_v1".to_string()));
    assert!(tool_names.contains(&"tool2".to_string()));
    
    Ok(())
}

#[test]
fn test_merge_empty_files_list() -> Result<()> {
    // Create a merger
    let merger = CapabilityMerger::new();
    
    // Perform the merge with an empty list - this should fail
    let result = merger.merge(vec![], MergeStrategy::Error);
    assert!(result.is_err());
    
    Ok(())
}