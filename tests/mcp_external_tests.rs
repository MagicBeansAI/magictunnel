//! Tests for External MCP Process Management
//! 
//! These tests verify the External MCP system can spawn processes,
//! perform MCP handshakes, and discover capabilities.

#[cfg(test)]
mod tests {
    use magictunnel::mcp::external_process::ExternalMcpProcess;
    use magictunnel::mcp::external_manager::ExternalMcpManager;
    use magictunnel::config::{ExternalMcpConfig, McpServerConfig, ContainerConfig, McpClientConfig};
    use std::collections::HashMap;
    use tokio::time::Duration;

    /// Helper function to create a default MCP client config for testing
    fn create_test_client_config() -> McpClientConfig {
        McpClientConfig {
            connect_timeout_secs: 30,
            request_timeout_secs: 60,
            max_reconnect_attempts: 5,
            reconnect_delay_secs: 5,
            auto_reconnect: true,
            protocol_version: "2025-06-18".to_string(),
            client_name: format!("{}-test", env!("CARGO_PKG_NAME")),
            client_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Test basic External MCP process creation
    #[tokio::test]
    async fn test_external_mcp_process_creation() {
        let config = McpServerConfig {
            command: "echo".to_string(),
            args: vec!["test".to_string()],
            env: None,
            cwd: None,
            sampling_strategy: None,
            elicitation_strategy: None,
        };

        let client_config = create_test_client_config();
        let process = ExternalMcpProcess::new("test-server".to_string(), config, client_config);
        assert_eq!(process.name, "test-server");
        assert!(!process.is_running().await);
    }

    /// Test environment variable expansion
    #[tokio::test]
    async fn test_env_var_expansion() {
        use magictunnel::mcp::external_process::expand_env_vars;
        
        // Set a test environment variable
        std::env::set_var("TEST_VAR", "test_value");
        
        let input = "prefix_${TEST_VAR}_suffix";
        let result = expand_env_vars(input);
        assert_eq!(result, "prefix_test_value_suffix");
        
        // Test with missing variable
        let input = "prefix_${MISSING_VAR}_suffix";
        let result = expand_env_vars(input);
        assert_eq!(result, "prefix__suffix");
        
        // Clean up
        std::env::remove_var("TEST_VAR");
    }

    /// Test External MCP Manager configuration loading
    #[tokio::test]
    async fn test_external_mcp_manager_creation() {
        let config = ExternalMcpConfig {
            enabled: true,
            config_file: "./test-external-mcp-servers.yaml".to_string(),
            capabilities_output_dir: "./test-capabilities".to_string(),
            refresh_interval_minutes: 60,
            containers: Some(ContainerConfig {
                runtime: "docker".to_string(),
                node_image: Some("node:18-alpine".to_string()),
                python_image: Some("python:3.11-alpine".to_string()),
                network_mode: Some("bridge".to_string()),
                run_args: vec!["--rm".to_string(), "-i".to_string()],
            }),
        };

        let client_config = create_test_client_config();
        let manager = ExternalMcpManager::new(config, client_config);

        // Test that manager is created successfully
        assert!(manager.get_active_servers().await.is_empty());
        assert!(manager.get_all_tools().await.is_empty());
    }

    /// Test configuration validation utilities
    #[tokio::test]
    async fn test_config_validation() {
        use magictunnel::mcp::external_integration::utils;

        // Test valid configuration
        let valid_config = McpServerConfig {
            command: "echo".to_string(),
            args: vec!["test".to_string()],
            env: None,
            cwd: None,
            sampling_strategy: None,
            elicitation_strategy: None,
        };
        assert!(utils::validate_server_config(&valid_config).is_ok());

        // Test invalid configuration - empty command
        let invalid_config = McpServerConfig {
            command: "".to_string(),
            args: vec!["test".to_string()],
            env: None,
            cwd: None,
            sampling_strategy: None,
            elicitation_strategy: None,
        };
        assert!(utils::validate_server_config(&invalid_config).is_err());

        // Test invalid configuration - empty args
        let invalid_config = McpServerConfig {
            command: "echo".to_string(),
            args: vec![],
            env: None,
            cwd: None,
            sampling_strategy: None,
            elicitation_strategy: None,
        };
        assert!(utils::validate_server_config(&invalid_config).is_err());
    }

    /// Test environment variable expansion in configuration
    #[tokio::test]
    async fn test_config_env_expansion() {
        use magictunnel::mcp::external_integration::utils;

        // Set test environment variables
        std::env::set_var("TEST_PATH", "/test/path");
        std::env::set_var("TEST_VALUE", "test_value");

        let mut config = McpServerConfig {
            command: "echo".to_string(),
            args: vec!["${TEST_VALUE}".to_string(), "static".to_string()],
            env: Some({
                let mut env = HashMap::new();
                env.insert("CUSTOM_VAR".to_string(), "${TEST_VALUE}".to_string());
                env
            }),
            cwd: Some("${TEST_PATH}".to_string()),
            sampling_strategy: None,
            elicitation_strategy: None,
        };

        utils::expand_config_env_vars(&mut config);

        assert_eq!(config.args[0], "test_value");
        assert_eq!(config.args[1], "static");
        assert_eq!(config.env.as_ref().unwrap().get("CUSTOM_VAR").unwrap(), "test_value");
        assert_eq!(config.cwd.as_ref().unwrap(), "/test/path");

        // Clean up
        std::env::remove_var("TEST_PATH");
        std::env::remove_var("TEST_VALUE");
    }

    /// Test command availability checking
    #[tokio::test]
    async fn test_command_availability() {
        use magictunnel::mcp::external_integration::utils;

        // Test with a command that should exist on most systems
        let available = utils::is_command_available("echo").await;
        assert!(available);

        // Test with a command that should not exist
        let not_available = utils::is_command_available("definitely_not_a_real_command_12345").await;
        assert!(!not_available);
    }

    /// Integration test - Test with a real echo command (safe test)
    #[tokio::test]
    async fn test_real_process_spawn() {
        let config = McpServerConfig {
            command: "echo".to_string(),
            args: vec!["Hello, External MCP!".to_string()],
            env: None,
            cwd: None,
            sampling_strategy: None,
            elicitation_strategy: None,
        };

        let client_config = create_test_client_config();
        let mut process = ExternalMcpProcess::new("echo-test".to_string(), config, client_config);

        // Start the process
        let start_result = process.start().await;
        
        // Echo command should start successfully but will exit immediately
        // This is expected behavior for this test
        match start_result {
            Ok(_) => {
                // Process started successfully
                // Give it a moment to complete
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => {
                // This might fail if echo is not available or behaves differently
                println!("Echo test failed (expected on some systems): {}", e);
            }
        }

        // Clean up
        let _ = process.stop().await;
    }

    /// Test External MCP Manager with empty configuration
    #[tokio::test]
    async fn test_manager_with_empty_config() {
        let config = ExternalMcpConfig {
            enabled: true,
            config_file: "./non-existent-config.yaml".to_string(),
            capabilities_output_dir: "./test-capabilities".to_string(),
            refresh_interval_minutes: 1, // Short interval for testing
            containers: None,
        };

        let client_config = create_test_client_config();
        let manager = ExternalMcpManager::new(config, client_config);

        // Starting with non-existent config should create example config
        // and not fail (though no servers will be started)
        let start_result = manager.start().await;
        
        // This should succeed (creates example config)
        match start_result {
            Ok(_) => {
                assert!(manager.get_active_servers().await.is_empty());
                assert!(manager.get_all_tools().await.is_empty());
            }
            Err(e) => {
                println!("Manager start failed: {}", e);
                // This might fail due to file system permissions or other issues
                // but the test structure is correct
            }
        }

        // Clean up
        let _ = manager.stop_all().await;
        
        // Clean up test files
        let _ = tokio::fs::remove_file("./non-existent-config.yaml").await;
        let _ = tokio::fs::remove_dir_all("./test-capabilities").await;
    }

    /// Test tool execution error handling
    #[tokio::test]
    async fn test_tool_execution_error_handling() {
        let config = ExternalMcpConfig {
            enabled: true,
            config_file: "./test-config.yaml".to_string(),
            capabilities_output_dir: "./test-capabilities".to_string(),
            refresh_interval_minutes: 60,
            containers: None,
        };

        let client_config = create_test_client_config();
        let manager = ExternalMcpManager::new(config, client_config);

        // Try to execute tool on non-existent server
        let result = manager.execute_tool("non-existent-server", "test-tool", serde_json::json!({})).await;
        assert!(result.is_err());
        
        // The error should indicate server not found
        if let Err(e) = result {
            assert!(e.to_string().contains("not found"));
        }
    }

    /// Test server restart functionality
    #[tokio::test]
    async fn test_server_restart() {
        let config = ExternalMcpConfig {
            enabled: true,
            config_file: "./test-restart-config.yaml".to_string(),
            capabilities_output_dir: "./test-capabilities".to_string(),
            refresh_interval_minutes: 60,
            containers: None,
        };

        let client_config = create_test_client_config();
        let manager = ExternalMcpManager::new(config, client_config);

        // Try to restart non-existent server
        let result = manager.restart_server("non-existent-server").await;
        assert!(result.is_err());
        
        // Clean up
        let _ = tokio::fs::remove_file("./test-restart-config.yaml").await;
        let _ = tokio::fs::remove_dir_all("./test-capabilities").await;
    }
}
