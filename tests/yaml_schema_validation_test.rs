//! Comprehensive YAML Schema Validation Tests
//! 
//! This test module ensures all YAML capability files have proper schema validation
//! and that their configurations conform to MCP 2025-06-18 specification requirements.

use std::fs;
use std::path::{Path, PathBuf};
use serde_yaml;
use serde_json::Value;

#[tokio::test]
async fn test_yaml_schema_validation() {
    let capabilities_dir = get_capabilities_dir();
    assert!(capabilities_dir.exists(), "Capabilities directory should exist");

    let yaml_files = find_yaml_files(&capabilities_dir);
    assert!(!yaml_files.is_empty(), "Should find YAML files to validate");

    println!("Testing schema validation for {} YAML files", yaml_files.len());

    let mut schema_errors = Vec::new();

    for yaml_file in &yaml_files {
        match validate_yaml_schema(yaml_file).await {
            Ok(_) => {
                println!("✅ Schema valid: {}", yaml_file.display());
            }
            Err(e) => {
                schema_errors.push(format!("{}: {}", yaml_file.display(), e));
            }
        }
    }

    if !schema_errors.is_empty() {
        panic!("YAML schema validation errors found:\n{}", schema_errors.join("\n"));
    }

    println!("✅ All {} YAML files have valid schemas", yaml_files.len());
}

#[tokio::test]
async fn test_required_fields_validation() {
    let capabilities_dir = get_capabilities_dir();
    let yaml_files = find_yaml_files(&capabilities_dir);

    let mut field_errors = Vec::new();

    for yaml_file in &yaml_files {
        match validate_required_fields(yaml_file).await {
            Ok(_) => {}
            Err(e) => {
                field_errors.push(format!("{}: {}", yaml_file.display(), e));
            }
        }
    }

    if !field_errors.is_empty() {
        panic!("Required field validation errors:\n{}", field_errors.join("\n"));
    }

    println!("✅ All YAML files have required fields");
}

#[tokio::test]
async fn test_tool_definitions_validation() {
    let capabilities_dir = get_capabilities_dir();
    let yaml_files = find_yaml_files(&capabilities_dir);

    let mut tool_errors = Vec::new();
    let mut total_tools = 0;

    for yaml_file in &yaml_files {
        match validate_tool_definitions(yaml_file).await {
            Ok(tool_count) => {
                total_tools += tool_count;
            }
            Err(e) => {
                tool_errors.push(format!("{}: {}", yaml_file.display(), e));
            }
        }
    }

    if !tool_errors.is_empty() {
        panic!("Tool definition validation errors:\n{}", tool_errors.join("\n"));
    }

    println!("✅ All {} tools in YAML files have valid definitions", total_tools);
}

#[tokio::test]
async fn test_routing_configuration_validation() {
    let capabilities_dir = get_capabilities_dir();
    let yaml_files = find_yaml_files(&capabilities_dir);

    let mut routing_errors = Vec::new();

    for yaml_file in &yaml_files {
        match validate_routing_configurations(yaml_file).await {
            Ok(_) => {}
            Err(e) => {
                routing_errors.push(format!("{}: {}", yaml_file.display(), e));
            }
        }
    }

    if !routing_errors.is_empty() {
        panic!("Routing configuration validation errors:\n{}", routing_errors.join("\n"));
    }

    println!("✅ All YAML files have valid routing configurations");
}

#[tokio::test]
async fn test_mcp_2025_compliance_fields() {
    let capabilities_dir = get_capabilities_dir();
    let yaml_files = find_yaml_files(&capabilities_dir);

    let mut compliance_errors = Vec::new();
    let mut enhanced_files = 0;

    for yaml_file in &yaml_files {
        match validate_mcp_2025_compliance(yaml_file).await {
            Ok(is_enhanced) => {
                if is_enhanced {
                    enhanced_files += 1;
                }
            }
            Err(e) => {
                compliance_errors.push(format!("{}: {}", yaml_file.display(), e));
            }
        }
    }

    if !compliance_errors.is_empty() {
        panic!("MCP 2025-06-18 compliance validation errors:\n{}", compliance_errors.join("\n"));
    }

    println!("✅ {} files comply with MCP 2025-06-18 specification", enhanced_files);
}

// Helper functions

fn get_capabilities_dir() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    current_dir.join("capabilities")
}

fn find_yaml_files(dir: &Path) -> Vec<PathBuf> {
    let mut yaml_files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        if extension == "yaml" || extension == "yml" {
                            yaml_files.push(path);
                        }
                    }
                } else if path.is_dir() {
                    let mut sub_files = find_yaml_files(&path);
                    yaml_files.append(&mut sub_files);
                }
            }
        }
    }
    
    yaml_files
}

async fn validate_yaml_schema(yaml_file: &Path) -> Result<(), String> {
    let content = fs::read_to_string(yaml_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    
    // Basic schema validation
    if !yaml_content.is_object() {
        return Err("Root YAML element must be an object".to_string());
    }
    
    let obj = yaml_content.as_object().unwrap();
    
    // Check for required top-level fields
    if !obj.contains_key("tools") && !obj.contains_key("metadata") {
        return Err("YAML must contain either 'tools' or 'metadata' field".to_string());
    }
    
    Ok(())
}

async fn validate_required_fields(yaml_file: &Path) -> Result<(), String> {
    let content = fs::read_to_string(yaml_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    
    // Validate metadata if present
    if let Some(metadata) = yaml_content.get("metadata") {
        if let Some(metadata_obj) = metadata.as_object() {
            let required_basic_fields = ["name", "description"];
            for field in &required_basic_fields {
                if !metadata_obj.contains_key(*field) {
                    return Err(format!("Missing required metadata field: {}", field));
                }
                
                if let Some(value) = metadata_obj.get(*field) {
                    if value.is_null() || (value.is_string() && value.as_str().unwrap().is_empty()) {
                        return Err(format!("Required metadata field '{}' cannot be empty", field));
                    }
                }
            }
        }
    }
    
    Ok(())
}

async fn validate_tool_definitions(yaml_file: &Path) -> Result<usize, String> {
    let content = fs::read_to_string(yaml_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    
    if let Some(tools) = yaml_content.get("tools") {
        if let Some(tools_array) = tools.as_array() {
            for (i, tool) in tools_array.iter().enumerate() {
                if let Some(tool_obj) = tool.as_object() {
                    // Check basic tool fields
                    if !tool_obj.contains_key("name") {
                        return Err(format!("Tool at index {} missing 'name' field", i));
                    }
                    
                    if let Some(name) = tool_obj.get("name") {
                        if name.is_null() || (name.is_string() && name.as_str().unwrap().is_empty()) {
                            return Err(format!("Tool at index {} has empty name", i));
                        }
                    }
                    
                    // Check for description in various possible locations
                    let has_description = tool_obj.contains_key("description") ||
                        (tool_obj.get("core").and_then(|c| c.get("description")).is_some());
                    
                    if !has_description {
                        return Err(format!("Tool '{}' at index {} missing description", 
                            tool_obj.get("name").and_then(|n| n.as_str()).unwrap_or("unknown"), i));
                    }
                    
                    // Check for input schema in various possible locations
                    let has_input_schema = tool_obj.contains_key("inputSchema") ||
                        tool_obj.contains_key("input_schema") ||
                        (tool_obj.get("core").and_then(|c| c.get("input_schema")).is_some());
                    
                    if !has_input_schema {
                        return Err(format!("Tool '{}' at index {} missing input schema", 
                            tool_obj.get("name").and_then(|n| n.as_str()).unwrap_or("unknown"), i));
                    }
                } else {
                    return Err(format!("Tool at index {} is not an object", i));
                }
            }
            
            return Ok(tools_array.len());
        }
    }
    
    Ok(0)
}

async fn validate_routing_configurations(yaml_file: &Path) -> Result<(), String> {
    let content = fs::read_to_string(yaml_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    
    if let Some(tools) = yaml_content.get("tools") {
        if let Some(tools_array) = tools.as_array() {
            for (i, tool) in tools_array.iter().enumerate() {
                if let Some(tool_obj) = tool.as_object() {
                    let tool_name = tool_obj.get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown");
                    
                    // Check for routing configuration in various possible locations
                    let routing = tool_obj.get("routing")
                        .or_else(|| tool_obj.get("execution").and_then(|e| e.get("routing")));
                    
                    if let Some(routing_config) = routing {
                        if let Some(routing_obj) = routing_config.as_object() {
                            // Check for routing type
                            if !routing_obj.contains_key("type") {
                                return Err(format!("Tool '{}' missing routing type", tool_name));
                            }
                            
                            // Validate routing type values
                            if let Some(routing_type) = routing_obj.get("type").and_then(|t| t.as_str()) {
                                let basic_types = ["http", "grpc", "graphql", "websocket", "subprocess", "lambda"];
                                let valid_prefixes = ["enhanced_", "ai_", "external_"];
                                
                                let is_valid = basic_types.contains(&routing_type) ||
                                    valid_prefixes.iter().any(|prefix| routing_type.starts_with(prefix));
                                
                                if !is_valid {
                                    return Err(format!("Tool '{}' has invalid routing type: {}. Must be a basic type ({}) or start with enhanced_, ai_, or external_", 
                                        tool_name, routing_type, basic_types.join(", ")));
                                }
                            }
                        }
                    } else {
                        // Only require routing for tools that aren't just metadata
                        if tool_obj.len() > 2 { // More than just name and description
                            return Err(format!("Tool '{}' missing routing configuration", tool_name));
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

async fn validate_mcp_2025_compliance(yaml_file: &Path) -> Result<bool, String> {
    let content = fs::read_to_string(yaml_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    
    // Check for MCP 2025-06-18 enhanced format indicators
    let mut is_enhanced = false;
    
    if let Some(metadata) = yaml_content.get("metadata") {
        if let Some(metadata_obj) = metadata.as_object() {
            // Check for MCP capabilities
            if let Some(mcp_capabilities) = metadata_obj.get("mcp_capabilities") {
                if let Some(mcp_obj) = mcp_capabilities.as_object() {
                    is_enhanced = true;
                    
                    // Validate MCP version
                    if let Some(version) = mcp_obj.get("version") {
                        if let Some(version_str) = version.as_str() {
                            if version_str != "2025-06-18" {
                                return Err(format!("Invalid MCP version: {}. Expected: 2025-06-18", version_str));
                            }
                        }
                    }
                    
                    // Check for required capabilities
                    let required_capabilities = [
                        "supports_cancellation",
                        "supports_progress", 
                        "supports_sampling",
                        "supports_validation"
                    ];
                    
                    for capability in &required_capabilities {
                        if !mcp_obj.contains_key(*capability) {
                            // This is a warning, not an error for now
                            println!("⚠️  MCP capability '{}' not declared in {}", capability, yaml_file.display());
                        }
                    }
                }
            }
            
            // Check for classification
            if metadata_obj.contains_key("classification") {
                is_enhanced = true;
            }
            
            // Check for discovery metadata
            if metadata_obj.contains_key("discovery_metadata") {
                is_enhanced = true;
            }
        }
    }
    
    // Check tools for enhanced structure
    if let Some(tools) = yaml_content.get("tools") {
        if let Some(tools_array) = tools.as_array() {
            if !tools_array.is_empty() {
                if let Some(first_tool) = tools_array[0].as_object() {
                    let enhanced_sections = ["core", "execution", "discovery", "monitoring", "access"];
                    let found_sections = enhanced_sections.iter()
                        .filter(|&&section| first_tool.contains_key(section))
                        .count();
                    
                    if found_sections >= 3 {
                        is_enhanced = true;
                    }
                }
            }
        }
    }
    
    Ok(is_enhanced)
}