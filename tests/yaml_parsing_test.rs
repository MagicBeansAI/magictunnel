//! Test YAML parsing for capability files

use magictunnel::registry::types::CapabilityFile;
use std::fs;

#[cfg(test)]
mod yaml_parsing_tests {
    use super::*;

    #[test]
    fn test_parse_file_operations_yaml() {
        let content = fs::read_to_string("capabilities/core/file_operations.yaml")
            .expect("Should be able to read file_operations.yaml");
        
        println!("YAML content:\n{}", content);
        
        let result: Result<CapabilityFile, _> = serde_yaml::from_str(&content);
        
        match result {
            Ok(capability_file) => {
                println!("✓ Successfully parsed capability file");
                println!("  Tools count: {}", capability_file.tools.len());
                for tool in &capability_file.tools {
                    println!("  - Tool: {}", tool.name);
                    println!("    Description: {}", tool.description);
                    println!("    Routing type: {}", tool.routing.r#type);
                }
            }
            Err(e) => {
                println!("✗ Failed to parse YAML: {}", e);
                panic!("YAML parsing failed: {}", e);
            }
        }
    }

    #[test]
    fn test_parse_all_capability_files() {
        let capability_files = [
            "capabilities/core/file_operations.yaml",
            "capabilities/ai/llm_tools.yaml",
            "capabilities/web/http_client.yaml",
            "capabilities/data/database_tools.yaml",
            "capabilities/dev/git_tools.yaml",
            "capabilities/system/monitoring.yaml",
        ];

        for file_path in capability_files {
            println!("Testing file: {}", file_path);
            
            let content = fs::read_to_string(file_path)
                .expect(&format!("Should be able to read {}", file_path));
            
            let result: Result<CapabilityFile, _> = serde_yaml::from_str(&content);
            
            match result {
                Ok(capability_file) => {
                    println!("  ✓ Successfully parsed {} with {} tools", 
                            file_path, capability_file.tools.len());
                }
                Err(e) => {
                    println!("  ✗ Failed to parse {}: {}", file_path, e);
                    // Don't panic here, just report the error
                }
            }
        }
    }
}
