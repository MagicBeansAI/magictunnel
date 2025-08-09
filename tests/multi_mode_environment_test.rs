//! Multi-mode architecture environment integration tests
//!
//! Tests the complete environment variable integration including:
//! - MAGICTUNNEL_RUNTIME_MODE environment variable behavior
//! - CONFIG_PATH environment variable resolution  
//! - MAGICTUNNEL_SMART_DISCOVERY environment variable override
//! - Environment variable priority over config files
//! - Environment variable validation and error handling

use std::env;
use std::fs;
use tempfile::TempDir;

use magictunnel::config::{
    Config, RuntimeMode, ConfigResolver, EnvironmentOverrides
};
use magictunnel::error::Result;

/// Test helper for managing environment variables
struct EnvTestHelper {
    original_vars: Vec<(String, Option<String>)>,
}

impl EnvTestHelper {
    fn new() -> Self {
        Self {
            original_vars: Vec::new(),
        }
    }

    fn set_var(&mut self, key: &str, value: &str) {
        // Store original value for cleanup
        let original_value = env::var(key).ok();
        self.original_vars.push((key.to_string(), original_value));
        
        // Set new value
        env::set_var(key, value);
    }

    fn remove_var(&mut self, key: &str) {
        // Store original value for cleanup
        let original_value = env::var(key).ok();
        self.original_vars.push((key.to_string(), original_value));
        
        // Remove variable
        env::remove_var(key);
    }
}

impl Drop for EnvTestHelper {
    fn drop(&mut self) {
        // Restore original environment variables
        for (key, original_value) in &self.original_vars {
            match original_value {
                Some(value) => env::set_var(key, value),
                None => env::remove_var(key),
            }
        }
    }
}

/// Test MAGICTUNNEL_RUNTIME_MODE environment variable
#[tokio::test]
async fn test_runtime_mode_environment_variable() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    
    // Test proxy mode
    env_helper.set_var("MAGICTUNNEL_RUNTIME_MODE", "proxy");
    
    let env_overrides = EnvironmentOverrides::load()?;
    assert_eq!(env_overrides.runtime_mode, Some(RuntimeMode::Proxy));
    
    // Test advanced mode
    env_helper.set_var("MAGICTUNNEL_RUNTIME_MODE", "advanced");
    
    let env_overrides = EnvironmentOverrides::load()?;
    assert_eq!(env_overrides.runtime_mode, Some(RuntimeMode::Advanced));
    
    Ok(())
}

/// Test MAGICTUNNEL_SMART_DISCOVERY environment variable
#[tokio::test]
async fn test_smart_discovery_environment_variable() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    
    // Test enabled
    env_helper.set_var("MAGICTUNNEL_SMART_DISCOVERY", "true");
    
    let env_overrides = EnvironmentOverrides::load()?;
    assert_eq!(env_overrides.smart_discovery, Some(true));
    
    // Test disabled
    env_helper.set_var("MAGICTUNNEL_SMART_DISCOVERY", "false");
    
    let env_overrides = EnvironmentOverrides::load()?;
    assert_eq!(env_overrides.smart_discovery, Some(false));
    
    // Test various true values
    for true_value in &["1", "yes", "on", "TRUE", "True"] {
        env_helper.set_var("MAGICTUNNEL_SMART_DISCOVERY", true_value);
        let env_overrides = EnvironmentOverrides::load()?;
        assert_eq!(env_overrides.smart_discovery, Some(true), "Failed for value: {}", true_value);
    }
    
    // Test various false values
    for false_value in &["0", "no", "off", "FALSE", "False"] {
        env_helper.set_var("MAGICTUNNEL_SMART_DISCOVERY", false_value);
        let env_overrides = EnvironmentOverrides::load()?;
        assert_eq!(env_overrides.smart_discovery, Some(false), "Failed for value: {}", false_value);
    }
    
    Ok(())
}

/// Test CONFIG_PATH environment variable
#[tokio::test]
async fn test_config_path_environment_variable() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("my-custom-config.yaml");
    
    // Create a test config file
    fs::write(&config_path, r#"
server:
  host: "0.0.0.0"
  port: 8080
deployment:
  runtime_mode: advanced
"#).unwrap();
    
    // Set CONFIG_PATH environment variable
    env_helper.set_var("CONFIG_PATH", config_path.to_str().unwrap());
    
    let env_overrides = EnvironmentOverrides::load()?;
    assert_eq!(env_overrides.config_path, Some(config_path.clone()));
    
    // Test that resolver uses this path
    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;
    
    assert_eq!(resolution.config_path, Some(config_path));
    assert_eq!(resolution.config.server.port, 8080);
    
    Ok(())
}

/// Test environment variable override priority over config
#[tokio::test]
async fn test_environment_override_priority() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.yaml");
    
    // Create config with proxy mode
    fs::write(&config_path, r#"
server:
  host: "127.0.0.1"
  port: 3001
deployment:
  runtime_mode: proxy
smart_discovery:
  enabled: false
"#).unwrap();
    
    // Set environment to override config
    env_helper.set_var("MAGICTUNNEL_RUNTIME_MODE", "advanced");
    env_helper.set_var("MAGICTUNNEL_SMART_DISCOVERY", "true");
    
    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(Some(&config_path))?;
    
    // Environment should override config values
    assert_eq!(*resolution.get_runtime_mode(), RuntimeMode::Advanced);
    assert!(resolution.is_smart_discovery_enabled());
    
    // Config file values should still be used for non-overridden settings
    assert_eq!(resolution.config.server.port, 3001);
    assert_eq!(resolution.config.server.host, "127.0.0.1");
    
    Ok(())
}

/// Test environment variable validation
#[tokio::test] 
async fn test_environment_variable_validation() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    
    // Test invalid runtime mode
    env_helper.set_var("MAGICTUNNEL_RUNTIME_MODE", "invalid_mode");
    
    let result = EnvironmentOverrides::load();
    assert!(result.is_err(), "Should fail with invalid runtime mode");
    
    // Test invalid smart discovery value
    env_helper.remove_var("MAGICTUNNEL_RUNTIME_MODE");
    env_helper.set_var("MAGICTUNNEL_SMART_DISCOVERY", "maybe");
    
    let result = EnvironmentOverrides::load();
    assert!(result.is_err(), "Should fail with invalid smart discovery value");
    
    Ok(())
}

/// Test environment override summary
#[tokio::test]
async fn test_environment_override_summary() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    
    // Set multiple environment variables
    env_helper.set_var("MAGICTUNNEL_RUNTIME_MODE", "advanced");
    env_helper.set_var("MAGICTUNNEL_SMART_DISCOVERY", "true");
    
    let env_overrides = EnvironmentOverrides::load()?;
    
    assert!(env_overrides.has_overrides());
    
    let summary = env_overrides.get_override_summary();
    assert!(!summary.is_empty());
    
    // Should contain information about both overrides
    let summary_text = summary.join(" ");
    assert!(summary_text.contains("MAGICTUNNEL_RUNTIME_MODE"));
    assert!(summary_text.contains("MAGICTUNNEL_SMART_DISCOVERY"));
    
    Ok(())
}

/// Test applying environment overrides to config
#[tokio::test]
async fn test_applying_environment_overrides_to_config() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    
    // Set environment variables
    env_helper.set_var("MAGICTUNNEL_RUNTIME_MODE", "advanced");
    env_helper.set_var("MAGICTUNNEL_SMART_DISCOVERY", "true");
    
    let env_overrides = EnvironmentOverrides::load()?;
    
    // Create base config
    let mut config = Config::default();
    config.deployment = Some(magictunnel::config::DeploymentConfig {
        runtime_mode: RuntimeMode::Proxy, // Will be overridden
    });
    
    // Smart discovery disabled by default
    if let Some(ref mut sd) = config.smart_discovery {
        sd.enabled = false; // Will be overridden
    }
    
    // Apply overrides
    env_overrides.apply_to_config(&mut config);
    
    // Check that overrides were applied
    assert_eq!(config.deployment.unwrap().runtime_mode, RuntimeMode::Advanced);
    
    if let Some(ref sd) = config.smart_discovery {
        assert!(sd.enabled);
    }
    
    Ok(())
}

/// Test environment variables in complete config resolution flow
#[tokio::test]
async fn test_environment_variables_in_config_resolution() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("magictunnel-config.yaml");
    
    // Create config file
    fs::write(&config_path, r#"
server:
  host: "127.0.0.1"
  port: 3001
deployment:
  runtime_mode: proxy
smart_discovery:
  enabled: false
registry:
  capabilities_dir: "capabilities"
"#).unwrap();
    
    // Set environment overrides
    env_helper.set_var("MAGICTUNNEL_RUNTIME_MODE", "advanced");
    env_helper.set_var("MAGICTUNNEL_SMART_DISCOVERY", "true");
    
    // Change to temp directory and test resolution
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    
    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;
    
    // Verify environment overrides took effect
    assert_eq!(*resolution.get_runtime_mode(), RuntimeMode::Advanced);
    assert!(resolution.is_smart_discovery_enabled());
    assert!(resolution.env_overrides.has_overrides());
    assert!(resolution.validation_result.can_start());
    
    // Config file values should still be present for non-overridden settings
    assert_eq!(resolution.config.server.port, 3001);
    assert_eq!(resolution.config.server.host, "127.0.0.1");
    
    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

/// Test case sensitivity of environment variables
#[tokio::test]
async fn test_environment_variable_case_sensitivity() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    
    // Environment variable names should be case sensitive (uppercase required)
    env_helper.set_var("magictunnel_runtime_mode", "advanced"); // lowercase
    
    let env_overrides = EnvironmentOverrides::load()?;
    assert_eq!(env_overrides.runtime_mode, None); // Should not be recognized
    
    // Correct case should work
    env_helper.set_var("MAGICTUNNEL_RUNTIME_MODE", "advanced");
    
    let env_overrides = EnvironmentOverrides::load()?;
    assert_eq!(env_overrides.runtime_mode, Some(RuntimeMode::Advanced));
    
    Ok(())
}

/// Test environment variable edge cases
#[tokio::test]
async fn test_environment_variable_edge_cases() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    
    // Empty values
    env_helper.set_var("MAGICTUNNEL_RUNTIME_MODE", "");
    let result = EnvironmentOverrides::load();
    assert!(result.is_err(), "Empty runtime mode should be invalid");
    
    env_helper.remove_var("MAGICTUNNEL_RUNTIME_MODE");
    env_helper.set_var("MAGICTUNNEL_SMART_DISCOVERY", "");
    let result = EnvironmentOverrides::load();
    assert!(result.is_err(), "Empty smart discovery should be invalid");
    
    // Whitespace values
    env_helper.remove_var("MAGICTUNNEL_SMART_DISCOVERY");
    env_helper.set_var("MAGICTUNNEL_RUNTIME_MODE", " advanced ");
    let result = EnvironmentOverrides::load();
    // Depending on implementation, this might be trimmed or considered invalid
    match result {
        Ok(overrides) => {
            // If trimming is implemented
            assert_eq!(overrides.runtime_mode, Some(RuntimeMode::Advanced));
        }
        Err(_) => {
            // If strict validation is implemented, this is also acceptable
        }
    }
    
    Ok(())
}

/// Test no environment variables set (baseline)
#[tokio::test]
async fn test_no_environment_variables() -> Result<()> {
    let mut env_helper = EnvTestHelper::new();
    
    // Ensure no MagicTunnel environment variables are set
    env_helper.remove_var("MAGICTUNNEL_RUNTIME_MODE");
    env_helper.remove_var("MAGICTUNNEL_SMART_DISCOVERY");
    env_helper.remove_var("CONFIG_PATH");
    
    let env_overrides = EnvironmentOverrides::load()?;
    
    assert!(!env_overrides.has_overrides());
    assert_eq!(env_overrides.runtime_mode, None);
    assert_eq!(env_overrides.smart_discovery, None);
    assert_eq!(env_overrides.config_path, None);
    
    let summary = env_overrides.get_override_summary();
    assert!(summary.is_empty());
    
    Ok(())
}