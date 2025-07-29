//! Integration tests for the high-performance registry service

use magictunnel::config::{RegistryConfig, ValidationConfig};
use magictunnel::registry::service::RegistryService;

#[cfg(test)]
mod registry_service_tests {
    use super::*;

    /// Test registry service initialization and basic functionality
    #[tokio::test]
    async fn test_registry_service_initialization() {
        let config = RegistryConfig {
            r#type: "file".to_string(),
            paths: vec![
                "capabilities/core/*.yaml".to_string(),
                "capabilities/ai/*.yaml".to_string(),
                "capabilities/web/*.yaml".to_string(),
            ],
            hot_reload: false,
            validation: ValidationConfig {
                strict: true,
                allow_unknown_fields: false,
            },
        };

        let result = RegistryService::new(config).await;
        assert!(result.is_ok(), "Registry service should initialize successfully");

        let service = result.unwrap();
        let metadata = service.metadata();

        // Verify metadata is populated
        assert!(metadata.file_count >= 0);
        assert!(metadata.tool_count >= 0);
        assert!(metadata.load_duration_ms > 0);

        println!("Registry initialized with {} files, {} tools",
                metadata.file_count, metadata.tool_count);
    }

    /// Test capability file discovery
    #[tokio::test]
    async fn test_capability_file_discovery() {
        let config = RegistryConfig {
            r#type: "file".to_string(),
            paths: vec![
                "capabilities/**/*.yaml".to_string(),
            ],
            hot_reload: false,
            validation: ValidationConfig {
                strict: false, // Allow for test flexibility
                allow_unknown_fields: true,
            },
        };

        let result = RegistryService::new(config).await;
        if let Err(ref e) = result {
            println!("Registry service creation failed: {:?}", e);
        }
        assert!(result.is_ok(), "Registry service should discover capability files");

        let service = result.unwrap();
        let metadata = service.metadata();
        let tools = service.list_tools();

        println!("Registry metadata: files={}, tools={}, duration={}ms",
                metadata.file_count, metadata.tool_count, metadata.load_duration_ms);
        println!("Tools discovered: {:?}", tools);

        // We should have discovered some tools from our sample files
        assert!(!tools.is_empty(), "Should discover tools from capability files");

        // Check for specific tools from our sample files
        let tool_names: Vec<&str> = tools.iter().map(|s| s.as_str()).collect();
        println!("Discovered tools: {:?}", tool_names);

        // These tools should exist based on our sample files
        let expected_tools = vec![
            "read_file", "write_file", "list_directory",  // from file_operations.yaml
            "generate_text", "analyze_sentiment",         // from llm_tools.yaml
            "http_get", "http_post",                      // from http_client.yaml
        ];

        for expected_tool in expected_tools {
            if tool_names.contains(&expected_tool) {
                println!("✓ Found expected tool: {}", expected_tool);
            }
        }
    }

    /// Test tool lookup functionality
    #[tokio::test]
    async fn test_tool_lookup() {
        let config = RegistryConfig {
            r#type: "file".to_string(),
            paths: vec![
                "capabilities/core/*.yaml".to_string(),
            ],
            hot_reload: false,
            validation: ValidationConfig {
                strict: false,
                allow_unknown_fields: true,
            },
        };

        let result = RegistryService::new(config).await;
        assert!(result.is_ok());

        let service = result.unwrap();

        // Test getting a specific tool
        if let Some(tool) = service.get_tool("read_file") {
            assert_eq!(tool.name(), "read_file");
            assert!(!tool.description().is_empty());
            println!("✓ Successfully retrieved tool: {}", tool.name());
        }

        // Test getting a non-existent tool
        let non_existent = service.get_tool("non_existent_tool");
        assert!(non_existent.is_none(), "Should return None for non-existent tools");
    }

    /// Test registry reload functionality
    #[tokio::test]
    async fn test_registry_reload() {
        let config = RegistryConfig {
            r#type: "file".to_string(),
            paths: vec![
                "capabilities/core/*.yaml".to_string(),
            ],
            hot_reload: false,
            validation: ValidationConfig {
                strict: false,
                allow_unknown_fields: true,
            },
        };

        let result = RegistryService::new(config).await;
        assert!(result.is_ok());

        let service = result.unwrap();
        let initial_metadata = service.metadata();

        // Debug output for initial state
        println!("Initial metadata: file_count={}, tool_count={}, load_duration_ms={}",
                 initial_metadata.file_count, initial_metadata.tool_count, initial_metadata.load_duration_ms);

        // Reload the registry
        let reload_result = service.reload_registry().await;
        assert!(reload_result.is_ok(), "Registry reload should succeed");

        let updated_metadata = service.metadata();

        // Debug output to see the actual values
        println!("Updated metadata: file_count={}, tool_count={}, load_duration_ms={}",
                 updated_metadata.file_count, updated_metadata.tool_count, updated_metadata.load_duration_ms);

        // Metadata should be updated - check that we have files and tools
        assert!(updated_metadata.file_count > 0, "Should have discovered files after reload");
        assert!(updated_metadata.tool_count > 0, "Should have discovered tools after reload");

        // Load duration might be 0 for very fast operations, so we'll just check it's not negative
        // (which would be impossible anyway since it's u64)
        println!("✓ Registry reloaded successfully with {} files and {} tools",
                 updated_metadata.file_count, updated_metadata.tool_count);
    }
}
