//! Simple YAML Loading Test for Migrated Files
//! 
//! This test verifies that the registry can successfully load both legacy and
//! migrated MCP 2025-06-18 enhanced YAML files.

use std::path::PathBuf;
use std::fs;
use serde_yaml;
use serde_json::Value;

#[tokio::test]
async fn test_load_migrated_smart_discovery() {
    let smart_discovery_path = get_capabilities_path("ai/smart_discovery.yaml");
    
    // Test that the file can be parsed as YAML
    let content = fs::read_to_string(&smart_discovery_path)
        .expect("Should be able to read smart_discovery.yaml");
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .expect("smart_discovery.yaml should be valid YAML");
    
    // Verify it has the enhanced format structure
    assert!(yaml_content.get("metadata").is_some(), "Should have metadata section");
    assert!(yaml_content.get("tools").is_some(), "Should have tools section");
    
    // Check for enhanced metadata indicators
    let metadata = yaml_content.get("metadata").unwrap();
    let has_classification = metadata.get("classification").is_some();
    let has_discovery_metadata = metadata.get("discovery_metadata").is_some();
    let has_mcp_capabilities = metadata.get("mcp_capabilities").is_some();
    
    assert!(
        has_classification || has_discovery_metadata || has_mcp_capabilities,
        "Should have enhanced metadata indicators"
    );
    
    // Verify tools have enhanced structure
    let tools = yaml_content.get("tools").unwrap().as_array().unwrap();
    assert!(!tools.is_empty(), "Should have at least one tool");
    
    let first_tool = &tools[0];
    let tool_obj = first_tool.as_object().unwrap();
    
    // Check for enhanced tool sections
    let enhanced_sections = ["core", "execution", "discovery", "monitoring", "access"];
    let found_sections: Vec<_> = enhanced_sections.iter()
        .filter(|&&section| tool_obj.contains_key(section))
        .collect();
    
    assert!(
        found_sections.len() >= 3,
        "Enhanced tool should have at least 3 enhanced sections, found: {:?}",
        found_sections
    );
    
    println!("✅ smart_discovery.yaml successfully validated as enhanced format");
}

#[tokio::test]
async fn test_load_migrated_monitoring() {
    let monitoring_path = get_capabilities_path("system/monitoring.yaml");
    
    // Test that the file can be parsed as YAML
    let content = fs::read_to_string(&monitoring_path)
        .expect("Should be able to read monitoring.yaml");
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .expect("monitoring.yaml should be valid YAML");
    
    // Verify it has the enhanced format structure
    assert!(yaml_content.get("metadata").is_some(), "Should have metadata section");
    assert!(yaml_content.get("tools").is_some(), "Should have tools section");
    
    // Check for enhanced metadata indicators
    let metadata = yaml_content.get("metadata").unwrap();
    let has_classification = metadata.get("classification").is_some();
    let has_discovery_metadata = metadata.get("discovery_metadata").is_some();
    let has_mcp_capabilities = metadata.get("mcp_capabilities").is_some();
    
    assert!(
        has_classification || has_discovery_metadata || has_mcp_capabilities,
        "Should have enhanced metadata indicators"
    );
    
    // Count tools and verify they have enhanced structure
    let tools = yaml_content.get("tools").unwrap().as_array().unwrap();
    assert!(!tools.is_empty(), "Should have at least one tool");
    
    let mut enhanced_tools = 0;
    for tool in tools {
        let tool_obj = tool.as_object().unwrap();
        let enhanced_sections = ["core", "execution", "discovery", "monitoring", "access"];
        let found_sections = enhanced_sections.iter()
            .filter(|&&section| tool_obj.contains_key(section))
            .count();
        
        if found_sections >= 3 {
            enhanced_tools += 1;
        }
    }
    
    assert!(enhanced_tools > 0, "Should have at least one enhanced tool");
    
    println!("✅ monitoring.yaml successfully validated as enhanced format with {} enhanced tools", enhanced_tools);
}

#[tokio::test]
async fn test_load_legacy_format_files() {
    let legacy_files = [
        "core/file_operations.yaml",
        "web/http_client.yaml",
        "ai/llm_tools.yaml",
    ];
    
    for file_path in &legacy_files {
        let full_path = get_capabilities_path(file_path);
        
        if !full_path.exists() {
            println!("⚠️ Skipping {} - file not found", file_path);
            continue;
        }
        
        // Test that the file can be parsed as YAML
        let content = fs::read_to_string(&full_path)
            .expect(&format!("Should be able to read {}", file_path));
        
        let yaml_content: Value = serde_yaml::from_str(&content)
            .expect(&format!("{} should be valid YAML", file_path));
        
        // Verify it has the basic legacy structure
        assert!(yaml_content.get("tools").is_some(), "Should have tools section");
        
        let tools = yaml_content.get("tools").unwrap().as_array().unwrap();
        assert!(!tools.is_empty(), "Should have at least one tool");
        
        // Verify legacy tool structure
        for tool in tools {
            let tool_obj = tool.as_object().unwrap();
            assert!(tool_obj.get("name").is_some(), "Tool should have name");
            assert!(tool_obj.get("description").is_some(), "Tool should have description");
            assert!(tool_obj.get("inputSchema").is_some(), "Tool should have inputSchema");
            assert!(tool_obj.get("routing").is_some(), "Tool should have routing");
        }
        
        println!("✅ {} successfully validated as legacy format", file_path);
    }
}

#[tokio::test]
async fn test_validation_script_integration() {
    let python_script = get_python_validation_script();
    
    if !python_script.exists() {
        println!("⚠️ Python validation script not found, skipping integration test");
        return;
    }

    // Test validation of smart_discovery.yaml
    let smart_discovery_path = get_capabilities_path("ai/smart_discovery.yaml");
    
    let output = std::process::Command::new("python3")
        .arg(&python_script)
        .arg(&smart_discovery_path)
        .output()
        .expect("Failed to execute Python validation script");

    assert!(
        output.status.success(),
        "Python validation should pass for smart_discovery.yaml: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Enhanced MCP 2025-06-18"), "Should detect enhanced format");
    assert!(stdout.contains("VALIDATION PASSED"), "Should pass validation");
    
    println!("✅ Python validation script integration test passed");
}

#[tokio::test]
async fn test_migration_statistics() {
    let capabilities_dir = get_capabilities_dir();
    let yaml_files = find_yaml_files(&capabilities_dir);
    
    let mut legacy_count = 0;
    let mut enhanced_count = 0;
    let mut total_tools = 0;
    
    for yaml_file in &yaml_files {
        match load_and_analyze_yaml(yaml_file).await {
            Ok((format_type, tool_count)) => {
                total_tools += tool_count;
                match format_type.as_str() {
                    "legacy" => legacy_count += 1,
                    "enhanced" => enhanced_count += 1,
                    _ => {}
                }
            }
            Err(e) => {
                println!("⚠️ Error analyzing {}: {}", yaml_file.display(), e);
            }
        }
    }
    
    let total_files = legacy_count + enhanced_count;
    let migration_percentage = if total_files > 0 {
        (enhanced_count * 100) / total_files
    } else {
        0
    };
    
    println!("📊 Migration Statistics:");
    println!("  Total YAML files: {}", total_files);
    println!("  Legacy format: {} files", legacy_count);
    println!("  Enhanced format: {} files", enhanced_count);
    println!("  Migration progress: {}%", migration_percentage);
    println!("  Total tools: {}", total_tools);
    
    // Basic assertions
    assert!(total_files > 0, "Should have at least one YAML file");
    assert!(enhanced_count >= 2, "Should have at least 2 enhanced files (smart_discovery and monitoring)");
    assert!(total_tools > 0, "Should have at least one tool");
    
    // Expect at least some migration progress
    assert!(migration_percentage > 0, "Should have some migration progress");
    
    println!("✅ Migration statistics test passed");
}

// Helper functions

fn get_capabilities_dir() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    current_dir.join("capabilities")
}

fn get_capabilities_path(relative_path: &str) -> PathBuf {
    get_capabilities_dir().join(relative_path)
}

fn get_python_validation_script() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    current_dir.join("scripts").join("validate_yaml_migration.py")
}

fn find_yaml_files(dir: &PathBuf) -> Vec<PathBuf> {
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

async fn load_and_analyze_yaml(yaml_file: &PathBuf) -> Result<(String, usize), String> {
    let content = fs::read_to_string(yaml_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    
    // Count tools
    let tool_count = yaml_content.get("tools")
        .and_then(|t| t.as_array())
        .map(|t| t.len())
        .unwrap_or(0);
    
    // Determine format
    let format_type = if is_enhanced_format(&yaml_content) {
        "enhanced".to_string()
    } else {
        "legacy".to_string()
    };
    
    Ok((format_type, tool_count))
}

fn is_enhanced_format(yaml_content: &Value) -> bool {
    // Check metadata for enhanced indicators
    if let Some(metadata) = yaml_content.get("metadata") {
        if metadata.get("classification").is_some() ||
           metadata.get("discovery_metadata").is_some() ||
           metadata.get("mcp_capabilities").is_some() {
            return true;
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
                        return true;
                    }
                }
            }
        }
    }
    
    false
}