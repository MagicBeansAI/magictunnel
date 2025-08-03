//! YAML Migration Validation Tests
//! 
//! This test module validates the YAML migration from legacy format to MCP 2025-06-18
//! enhanced format. It ensures all capability files are properly formatted and compliant.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use serde_yaml;
use serde_json::Value;

#[tokio::test]
async fn test_yaml_syntax_validation() {
    let capabilities_dir = get_capabilities_dir();
    assert!(capabilities_dir.exists(), "Capabilities directory should exist");

    let yaml_files = find_yaml_files(&capabilities_dir);
    assert!(!yaml_files.is_empty(), "Should find YAML files to validate");

    println!("Found {} YAML files to validate", yaml_files.len());

    let mut syntax_errors = Vec::new();

    for yaml_file in &yaml_files {
        match fs::read_to_string(yaml_file) {
            Ok(content) => {
                if let Err(e) = serde_yaml::from_str::<Value>(&content) {
                    syntax_errors.push(format!("{}: {}", yaml_file.display(), e));
                }
            }
            Err(e) => {
                syntax_errors.push(format!("{}: Failed to read file: {}", yaml_file.display(), e));
            }
        }
    }

    if !syntax_errors.is_empty() {
        panic!("YAML syntax errors found:\n{}", syntax_errors.join("\n"));
    }

    println!("✅ All {} YAML files have valid syntax", yaml_files.len());
}

#[tokio::test]
async fn test_mcp_2025_compliance_validation() {
    let script_path = get_python_validation_script();
    
    if !script_path.exists() {
        println!("⚠️ Python validation script not found, skipping MCP 2025-06-18 compliance test");
        return;
    }

    let capabilities_dir = get_capabilities_dir();
    
    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&capabilities_dir)
        .output()
        .expect("Failed to execute Python validation script");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "MCP 2025-06-18 compliance validation failed:\nSTDOUT:\n{}\nSTDERR:\n{}", 
            stdout, stderr
        );
    }

    println!("✅ MCP 2025-06-18 compliance validation passed");
}

#[tokio::test]
async fn test_format_detection() {
    let capabilities_dir = get_capabilities_dir();
    let yaml_files = find_yaml_files(&capabilities_dir);

    let mut legacy_count = 0;
    let mut enhanced_count = 0;
    let mut detection_errors = Vec::new();

    for yaml_file in &yaml_files {
        match analyze_yaml_format(yaml_file).await {
            Ok(format_type) => {
                match format_type.as_str() {
                    "legacy" => legacy_count += 1,
                    "enhanced" => enhanced_count += 1,
                    _ => detection_errors.push(format!("{}: Unknown format type: {}", yaml_file.display(), format_type)),
                }
            }
            Err(e) => {
                detection_errors.push(format!("{}: {}", yaml_file.display(), e));
            }
        }
    }

    if !detection_errors.is_empty() {
        panic!("Format detection errors:\n{}", detection_errors.join("\n"));
    }

    let total_files = legacy_count + enhanced_count;
    let enhanced_percentage = if total_files > 0 {
        (enhanced_count * 100) / total_files
    } else {
        0
    };

    println!("📊 Format Distribution:");
    println!("  Legacy format: {} files", legacy_count);
    println!("  Enhanced MCP 2025-06-18 format: {} files", enhanced_count);
    println!("  Migration progress: {}%", enhanced_percentage);

    // Assert that we have at least some files
    assert!(total_files > 0, "Should have at least one capability file");
}

#[tokio::test]
async fn test_enhanced_format_structure() {
    let capabilities_dir = get_capabilities_dir();
    let yaml_files = find_yaml_files(&capabilities_dir);

    let mut enhanced_files = Vec::new();
    let mut structure_errors = Vec::new();

    // Find enhanced format files
    for yaml_file in &yaml_files {
        if let Ok(format_type) = analyze_yaml_format(yaml_file).await {
            if format_type == "enhanced" {
                enhanced_files.push(yaml_file);
            }
        }
    }

    if enhanced_files.is_empty() {
        println!("⚠️ No enhanced format files found, skipping structure validation");
        return;
    }

    println!("Validating structure of {} enhanced format files", enhanced_files.len());

    for yaml_file in &enhanced_files {
        if let Err(e) = validate_enhanced_structure(yaml_file).await {
            structure_errors.push(format!("{}: {}", yaml_file.display(), e));
        }
    }

    if !structure_errors.is_empty() {
        panic!("Enhanced format structure errors:\n{}", structure_errors.join("\n"));
    }

    println!("✅ All enhanced format files have valid structure");
}

#[tokio::test]
async fn test_security_configuration_presence() {
    let capabilities_dir = get_capabilities_dir();
    let yaml_files = find_yaml_files(&capabilities_dir);

    let mut security_configured_count = 0;
    let mut total_enhanced_files = 0;

    for yaml_file in &yaml_files {
        if let Ok(format_type) = analyze_yaml_format(yaml_file).await {
            if format_type == "enhanced" {
                total_enhanced_files += 1;
                if has_security_configuration(yaml_file).await.unwrap_or(false) {
                    security_configured_count += 1;
                }
            }
        }
    }

    if total_enhanced_files > 0 {
        let security_percentage = (security_configured_count * 100) / total_enhanced_files;
        println!("🔒 Security Configuration:");
        println!("  Enhanced files with security config: {} / {}", security_configured_count, total_enhanced_files);
        println!("  Security coverage: {}%", security_percentage);

        // Expect that enhanced files have security configuration
        assert!(
            security_configured_count > 0, 
            "Enhanced format files should have security configuration"
        );
    } else {
        println!("⚠️ No enhanced format files found for security validation");
    }
}

// Helper functions

fn get_capabilities_dir() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    current_dir.join("capabilities")
}

fn get_python_validation_script() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    current_dir.join("scripts").join("validate_yaml_migration.py")
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
                    // Recursively search subdirectories
                    let mut sub_files = find_yaml_files(&path);
                    yaml_files.append(&mut sub_files);
                }
            }
        }
    }
    
    yaml_files
}

async fn analyze_yaml_format(yaml_file: &Path) -> Result<String, String> {
    let content = fs::read_to_string(yaml_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    
    // Check for enhanced format indicators
    if let Some(metadata) = yaml_content.get("metadata") {
        if metadata.get("classification").is_some() ||
           metadata.get("discovery_metadata").is_some() ||
           metadata.get("mcp_capabilities").is_some() {
            return Ok("enhanced".to_string());
        }
    }
    
    // Check tools for enhanced structure
    if let Some(tools) = yaml_content.get("tools") {
        if let Some(tools_array) = tools.as_array() {
            if !tools_array.is_empty() {
                if let Some(first_tool) = tools_array[0].as_object() {
                    let enhanced_sections = [
                        "core", "execution", "discovery", "monitoring", "access"
                    ];
                    let found_sections = enhanced_sections.iter()
                        .filter(|&&section| first_tool.contains_key(section))
                        .count();
                    
                    if found_sections >= 3 {
                        return Ok("enhanced".to_string());
                    }
                }
            }
        }
    }
    
    Ok("legacy".to_string())
}

async fn validate_enhanced_structure(yaml_file: &Path) -> Result<(), String> {
    let content = fs::read_to_string(yaml_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    
    // Validate metadata structure
    if let Some(metadata) = yaml_content.get("metadata") {
        let required_fields = ["name", "description", "version", "author"];
        for field in &required_fields {
            if metadata.get(field).is_none() {
                return Err(format!("Missing required metadata field: {}", field));
            }
        }
    }
    
    // Validate tools structure
    if let Some(tools) = yaml_content.get("tools") {
        if let Some(tools_array) = tools.as_array() {
            for (i, tool) in tools_array.iter().enumerate() {
                if let Some(tool_obj) = tool.as_object() {
                    let required_sections = ["name", "core", "execution", "discovery", "monitoring", "access"];
                    for section in &required_sections {
                        if tool_obj.get(*section).is_none() {
                            return Err(format!("Tool {}: Missing required section: {}", i, section));
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

async fn has_security_configuration(yaml_file: &Path) -> Result<bool, String> {
    let content = fs::read_to_string(yaml_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let yaml_content: Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    
    // Check metadata for classification
    if let Some(metadata) = yaml_content.get("metadata") {
        if metadata.get("classification").is_some() {
            return Ok(true);
        }
    }
    
    // Check tools for security configuration
    if let Some(tools) = yaml_content.get("tools") {
        if let Some(tools_array) = tools.as_array() {
            for tool in tools_array {
                if let Some(tool_obj) = tool.as_object() {
                    if let Some(execution) = tool_obj.get("execution") {
                        if execution.get("security").is_some() {
                            return Ok(true);
                        }
                    }
                }
            }
        }
    }
    
    Ok(false)
}

#[tokio::test]
async fn test_validation_script_integration() {
    let bash_script = get_bash_validation_script();
    
    if !bash_script.exists() {
        println!("⚠️ Bash validation script not found, skipping integration test");
        return;
    }

    let capabilities_dir = get_capabilities_dir();
    
    let output = Command::new("bash")
        .arg(&bash_script)
        .arg("capabilities") // relative path instead of absolute
        .arg("false") // non-strict mode for testing
        .arg("false") // non-verbose mode
        .current_dir(&capabilities_dir.parent().unwrap()) // set working directory
        .output()
        .expect("Failed to execute bash validation script");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Bash validation script failed:\nSTDOUT:\n{}\nSTDERR:\n{}", 
            stdout, stderr
        );
    }

    println!("✅ Bash validation script integration test passed");
}

fn get_bash_validation_script() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    current_dir.join("scripts").join("validate_yaml_migration.sh")
}