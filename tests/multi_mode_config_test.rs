//! Multi-mode architecture configuration resolution tests
//! 
//! Tests the complete configuration resolution system including:
//! - Environment variable override behavior
//! - Config file priority resolution (magictunnel-config.yaml vs config.yaml)
//! - Built-in proxy mode defaults
//! - Configuration validation for both modes
//! - Priority system: CLI > Environment > Config File > Defaults

use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use magictunnel::config::{
    Config, DeploymentConfig, RuntimeMode, ConfigResolver, ConfigResolution,
    EnvironmentOverrides, ConfigSource
};
use magictunnel::error::Result;

/// Test helper for creating temporary config files
struct ConfigTestHelper {
    temp_dir: TempDir,
}

impl ConfigTestHelper {
    fn new() -> Result<Self> {
        let temp_dir = TempDir::new()
            .map_err(|e| magictunnel::error::ProxyError::config(format!("Failed to create temp dir: {}", e)))?;
        Ok(Self { temp_dir })
    }

    fn create_config_file(&self, filename: &str, content: &str) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        fs::write(&path, content).expect("Failed to write test config file");
        path
    }

    fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }
}

/// Test proxy mode configuration with built-in defaults
#[tokio::test]
async fn test_proxy_mode_builtin_defaults() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    // Change to temp directory (no config files exist)
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(helper.path()).unwrap();

    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;

    // Should use built-in defaults with proxy mode
    assert_eq!(*resolution.get_runtime_mode(), RuntimeMode::Proxy);
    assert!(matches!(resolution.config_source, ConfigSource::Defaults));
    assert!(!resolution.is_smart_discovery_enabled());
    assert!(resolution.validation_result.can_start());

    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

/// Test magictunnel-config.yaml priority over config.yaml
#[tokio::test]
async fn test_config_file_priority() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    // Create both config files in temp directory
    helper.create_config_file("config.yaml", r#"
server:
  host: "0.0.0.0"
  port: 8080
deployment:
  runtime_mode: proxy
"#);

    helper.create_config_file("magictunnel-config.yaml", r#"
server:
  host: "127.0.0.1"
  port: 3001
deployment:
  runtime_mode: advanced
"#);

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(helper.path()).unwrap();

    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;

    // Should use magictunnel-config.yaml (higher priority)
    assert!(matches!(resolution.config_source, ConfigSource::MagicTunnelConfig));
    assert_eq!(*resolution.get_runtime_mode(), RuntimeMode::Advanced);
    assert_eq!(resolution.config.server.port, 3001);
    assert_eq!(resolution.config.server.host, "127.0.0.1");

    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

/// Test environment variable overrides
#[tokio::test]
async fn test_environment_variable_overrides() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    // Create config file with proxy mode
    helper.create_config_file("magictunnel-config.yaml", r#"
server:
  host: "127.0.0.1"
  port: 3001
deployment:
  runtime_mode: proxy
smart_discovery:
  enabled: false
"#);

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(helper.path()).unwrap();

    // Set environment variables
    env::set_var("MAGICTUNNEL_RUNTIME_MODE", "advanced");
    env::set_var("MAGICTUNNEL_SMART_DISCOVERY", "true");

    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;

    // Environment should override config
    assert_eq!(*resolution.get_runtime_mode(), RuntimeMode::Advanced);
    assert!(resolution.is_smart_discovery_enabled());
    assert!(resolution.env_overrides.has_overrides());

    // Cleanup
    env::remove_var("MAGICTUNNEL_RUNTIME_MODE");
    env::remove_var("MAGICTUNNEL_SMART_DISCOVERY");
    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

/// Test CONFIG_PATH environment variable
#[tokio::test] 
async fn test_config_path_environment_override() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    // Create config file with specific name
    let custom_config = helper.create_config_file("custom-config.yaml", r#"
server:
  host: "0.0.0.0"
  port: 9000
deployment:
  runtime_mode: advanced
"#);

    // Set CONFIG_PATH environment variable
    env::set_var("CONFIG_PATH", custom_config.to_str().unwrap());

    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;

    // Should use custom config path
    assert!(matches!(resolution.config_source, ConfigSource::Custom(_)));
    assert_eq!(resolution.config.server.port, 9000);
    assert_eq!(*resolution.get_runtime_mode(), RuntimeMode::Advanced);

    // Cleanup
    env::remove_var("CONFIG_PATH");
    Ok(())
}

/// Test CLI config path override (highest priority)
#[tokio::test]
async fn test_cli_config_override() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    // Create multiple config files
    helper.create_config_file("magictunnel-config.yaml", r#"
deployment:
  runtime_mode: proxy
"#);

    let cli_config = helper.create_config_file("cli-config.yaml", r#"
server:
  host: "192.168.1.1"
  port: 4000
deployment:
  runtime_mode: advanced
"#);

    // Set environment that would normally override
    env::set_var("MAGICTUNNEL_RUNTIME_MODE", "proxy");

    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(Some(&cli_config))?;

    // CLI should have highest priority (env override should be ignored for file path)
    assert!(matches!(resolution.config_source, ConfigSource::Custom(_)));
    assert_eq!(*resolution.get_runtime_mode(), RuntimeMode::Proxy); // Env override still applies to content
    assert_eq!(resolution.config.server.port, 4000);

    // Cleanup
    env::remove_var("MAGICTUNNEL_RUNTIME_MODE");
    Ok(())
}

/// Test configuration validation for proxy mode
#[tokio::test]
async fn test_proxy_mode_validation() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    // Create valid proxy config
    helper.create_config_file("magictunnel-config.yaml", r#"
server:
  host: "127.0.0.1"
  port: 3001
deployment:
  runtime_mode: proxy
registry:
  capabilities_dir: "capabilities"
"#);

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(helper.path()).unwrap();

    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;

    // Should validate successfully
    assert!(resolution.validation_result.can_start());
    assert!(resolution.validation_result.errors.is_empty());
    assert_eq!(resolution.validation_result.mode, RuntimeMode::Proxy);

    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

/// Test configuration validation for advanced mode
#[tokio::test]
async fn test_advanced_mode_validation() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    // Create advanced config with security settings
    helper.create_config_file("magictunnel-config.yaml", r#"
server:
  host: "127.0.0.1"
  port: 3001
deployment:
  runtime_mode: advanced
registry:
  capabilities_dir: "capabilities"
security:
  policies: []
  allowlists: []
auth:
  api_keys: ["test-key"]
"#);

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(helper.path()).unwrap();

    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;

    // Should validate successfully for advanced mode
    assert!(resolution.validation_result.can_start());
    assert_eq!(resolution.validation_result.mode, RuntimeMode::Advanced);

    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

/// Test configuration validation errors
#[tokio::test]
async fn test_configuration_validation_errors() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    // Create invalid config (empty host, zero port)
    helper.create_config_file("magictunnel-config.yaml", r#"
server:
  host: ""
  port: 0
deployment:
  runtime_mode: proxy
"#);

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(helper.path()).unwrap();

    let resolver = ConfigResolver::new()?;
    
    // Should fail with validation errors
    let result = resolver.resolve_config(None);
    assert!(result.is_err());

    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

/// Test environment override summary
#[tokio::test]
async fn test_environment_override_summary() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    helper.create_config_file("magictunnel-config.yaml", r#"
server:
  host: "127.0.0.1"
  port: 3001
deployment:
  runtime_mode: proxy
"#);

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(helper.path()).unwrap();

    // Set multiple environment variables
    env::set_var("MAGICTUNNEL_RUNTIME_MODE", "advanced");
    env::set_var("MAGICTUNNEL_SMART_DISCOVERY", "true");

    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;

    let summary = resolution.get_startup_summary();
    assert!(summary.has_env_overrides);
    assert!(!summary.env_override_summary.is_empty());
    assert_eq!(summary.runtime_mode, RuntimeMode::Advanced);
    assert!(summary.smart_discovery_enabled);

    // Cleanup
    env::remove_var("MAGICTUNNEL_RUNTIME_MODE");
    env::remove_var("MAGICTUNNEL_SMART_DISCOVERY");
    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

/// Test smart discovery configuration
#[tokio::test]
async fn test_smart_discovery_configuration() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    // Test config with smart discovery enabled
    helper.create_config_file("magictunnel-config.yaml", r#"
server:
  host: "127.0.0.1"
  port: 3001
deployment:
  runtime_mode: proxy
smart_discovery:
  enabled: true
  mode: hybrid
  confidence_threshold: 0.7
"#);

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(helper.path()).unwrap();

    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;

    assert!(resolution.is_smart_discovery_enabled());
    assert!(resolution.validation_result.can_start());

    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

/// Test legacy config.yaml fallback
#[tokio::test]
async fn test_legacy_config_fallback() -> Result<()> {
    let helper = ConfigTestHelper::new()?;

    // Only create legacy config.yaml (no magictunnel-config.yaml)
    helper.create_config_file("config.yaml", r#"
server:
  host: "0.0.0.0"
  port: 8080
deployment:
  runtime_mode: advanced
"#);

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(helper.path()).unwrap();

    let resolver = ConfigResolver::new()?;
    let resolution = resolver.resolve_config(None)?;

    // Should use legacy config
    assert!(matches!(resolution.config_source, ConfigSource::LegacyConfig));
    assert_eq!(*resolution.get_runtime_mode(), RuntimeMode::Advanced);
    assert_eq!(resolution.config.server.port, 8080);

    env::set_current_dir(original_dir).unwrap();
    Ok(())
}