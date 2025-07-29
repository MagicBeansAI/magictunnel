//! Integration tests for MCP Proxy

use magictunnel::{Config, Result};
use std::path::Path;
use tempfile::tempdir;

#[tokio::test]
async fn test_config_loading() -> Result<()> {
    // Test default configuration
    let config = Config::default();
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 3000);
    assert!(config.server.websocket);
    
    Ok(())
}

#[tokio::test]
async fn test_config_validation() -> Result<()> {
    let mut config = Config::default();
    
    // Valid configuration should pass
    config.validate()?;
    
    // Invalid host should fail
    config.server.host = "".to_string();
    assert!(config.validate().is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_config_file_loading() -> Result<()> {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");
    
    // Create a test config file
    let config_content = r#"
server:
  host: "0.0.0.0"
  port: 8080
  websocket: false
  timeout: 60

registry:
  type: "file"
  paths:
    - "./test_data"
  hot_reload: false
  validation:
    strict: false
    allow_unknown_fields: true
"#;
    
    std::fs::write(&config_path, config_content).unwrap();
    
    // Load the configuration
    let config = Config::load(&config_path, None, None)?;
    
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 8080);
    assert!(!config.server.websocket);
    assert_eq!(config.server.timeout, 60);
    assert_eq!(config.registry.paths, vec!["./test_data"]);
    assert!(!config.registry.hot_reload);
    assert!(!config.registry.validation.strict);
    assert!(config.registry.validation.allow_unknown_fields);
    
    Ok(())
}

#[tokio::test]
async fn test_config_cli_overrides() -> Result<()> {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");
    
    // Create a basic config file
    let config_content = r#"
server:
  host: "127.0.0.1"
  port: 3000
  websocket: true
  timeout: 30

registry:
  type: "file"
  paths:
    - "./data"
  hot_reload: true
  validation:
    strict: true
    allow_unknown_fields: false
"#;
    
    std::fs::write(&config_path, config_content).unwrap();
    
    // Load with CLI overrides
    let config = Config::load(
        &config_path,
        Some("0.0.0.0".to_string()),
        Some(9000),
    )?;
    
    // CLI overrides should take precedence
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 9000);
    
    Ok(())
}

#[tokio::test]
async fn test_nonexistent_config_file() -> Result<()> {
    let nonexistent_path = Path::new("nonexistent_config.yaml");
    
    // Should use defaults when file doesn't exist
    let config = Config::load(nonexistent_path, None, None)?;
    
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 3000);
    
    Ok(())
}
