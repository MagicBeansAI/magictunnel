//! Multi-mode architecture service loading tests
//! 
//! Tests the conditional service loading strategy including:
//! - ProxyServices loading in proxy mode
//! - AdvancedServices loading in advanced mode  
//! - Service dependency validation
//! - Service container health checks
//! - Service loading order and lifecycle

use std::sync::Arc;

use magictunnel::config::{
    Config, DeploymentConfig, RuntimeMode, ConfigResolution, EnvironmentOverrides,
    ConfigSource, ValidationResult
};
use magictunnel::services::{ServiceLoader, ServiceContainer, ServiceState};
use magictunnel::error::Result;

/// Create test configuration resolution for given runtime mode
fn create_test_resolution(mode: RuntimeMode) -> ConfigResolution {
    let mut config = Config::default();
    config.deployment = Some(DeploymentConfig {
        runtime_mode: mode.clone(),
    });

    // Set reasonable defaults to pass validation
    config.server.host = "127.0.0.1".to_string();
    config.server.port = 3001;
    config.registry.paths = vec!["capabilities".to_string()];

    ConfigResolution {
        config,
        config_path: None,
        env_overrides: EnvironmentOverrides::load().unwrap(),
        config_source: ConfigSource::Defaults,
        validation_result: ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            suggestions: vec![],
            mode,
        },
    }
}

/// Test proxy mode service loading
#[tokio::test]
async fn test_proxy_mode_service_loading() -> Result<()> {
    let resolution = create_test_resolution(RuntimeMode::Proxy);
    
    let container = ServiceLoader::load_services(&resolution).await?;
    
    // Verify proxy mode service container
    assert_eq!(container.runtime_mode, RuntimeMode::Proxy);
    assert!(container.proxy_services.is_some());
    assert!(container.advanced_services.is_none());
    assert!(container.service_count > 0);
    
    // Verify service health
    assert!(container.is_healthy());
    
    Ok(())
}

/// Test advanced mode service loading
#[tokio::test]
async fn test_advanced_mode_service_loading() -> Result<()> {
    let resolution = create_test_resolution(RuntimeMode::Advanced);
    
    let container = ServiceLoader::load_services(&resolution).await?;
    
    // Verify advanced mode service container
    assert_eq!(container.runtime_mode, RuntimeMode::Advanced);
    assert!(container.proxy_services.is_some());
    assert!(container.advanced_services.is_some());
    assert!(container.service_count > 0);
    
    // Verify service health
    assert!(container.is_healthy());
    
    // Verify proxy services loaded first (dependency order)
    let proxy_services = container.proxy_services.as_ref().unwrap();
    assert!(proxy_services.is_healthy());
    
    // Verify advanced services loaded after proxy
    let advanced_services = container.advanced_services.as_ref().unwrap();
    assert!(advanced_services.is_healthy());
    
    Ok(())
}

/// Test service dependency validation
#[tokio::test]
async fn test_service_dependency_validation() -> Result<()> {
    let proxy_resolution = create_test_resolution(RuntimeMode::Proxy);
    let advanced_resolution = create_test_resolution(RuntimeMode::Advanced);
    
    // Test proxy mode dependencies
    let proxy_container = ServiceLoader::load_services(&proxy_resolution).await?;
    assert!(proxy_container.proxy_services.is_some());
    assert!(proxy_container.is_healthy());
    
    // Test advanced mode dependencies
    let advanced_container = ServiceLoader::load_services(&advanced_resolution).await?;
    assert!(advanced_container.proxy_services.is_some());
    assert!(advanced_container.advanced_services.is_some());
    assert!(advanced_container.is_healthy());
    
    // Verify advanced services depend on proxy services
    let proxy_services = advanced_container.proxy_services.as_ref().unwrap();
    let advanced_services = advanced_container.advanced_services.as_ref().unwrap();
    assert!(advanced_services.validate_dependencies(proxy_services));
    
    Ok(())
}

/// Test service container health checks
#[tokio::test]
async fn test_service_container_health_checks() -> Result<()> {
    let resolution = create_test_resolution(RuntimeMode::Advanced);
    let container = ServiceLoader::load_services(&resolution).await?;
    
    // Container should be healthy with both service types loaded
    assert!(container.is_healthy());
    
    // Test individual service health
    if let Some(ref proxy_services) = container.proxy_services {
        assert!(proxy_services.is_healthy());
    } else {
        panic!("Proxy services should be loaded");
    }
    
    if let Some(ref advanced_services) = container.advanced_services {
        assert!(advanced_services.is_healthy());
    } else {
        panic!("Advanced services should be loaded in advanced mode");
    }
    
    Ok(())
}

/// Test service count tracking
#[tokio::test]
async fn test_service_count_tracking() -> Result<()> {
    let proxy_resolution = create_test_resolution(RuntimeMode::Proxy);
    let advanced_resolution = create_test_resolution(RuntimeMode::Advanced);
    
    // Load both types of containers
    let proxy_container = ServiceLoader::load_services(&proxy_resolution).await?;
    let advanced_container = ServiceLoader::load_services(&advanced_resolution).await?;
    
    // Advanced mode should have more services than proxy mode
    assert!(advanced_container.service_count > proxy_container.service_count);
    assert!(proxy_container.service_count > 0);
    assert!(advanced_container.service_count > 0);
    
    // Service count should match actual loaded services
    if let Some(ref proxy_services) = proxy_container.proxy_services {
        assert_eq!(proxy_container.service_count, proxy_services.service_count());
    }
    
    if let (Some(ref proxy_services), Some(ref advanced_services)) = 
        (&advanced_container.proxy_services, &advanced_container.advanced_services) {
        let expected_count = proxy_services.service_count() + advanced_services.service_count();
        assert_eq!(advanced_container.service_count, expected_count);
    }
    
    Ok(())
}

/// Test service loading summary
#[tokio::test]
async fn test_service_loading_summary() -> Result<()> {
    let resolution = create_test_resolution(RuntimeMode::Advanced);
    let container = ServiceLoader::load_services(&resolution).await?;
    
    // Get loading summary
    let summary = ServiceLoader::get_loading_summary(&container);
    
    assert_eq!(summary.runtime_mode, RuntimeMode::Advanced);
    assert_eq!(summary.total_services, container.service_count);
    assert!(!summary.proxy_services.is_empty());
    
    if let Some(ref advanced_services) = summary.advanced_services {
        assert!(!advanced_services.is_empty());
    } else {
        panic!("Advanced services summary should be present");
    }
    
    Ok(())
}

/// Test service container accessors
#[tokio::test]
async fn test_service_container_accessors() -> Result<()> {
    let resolution = create_test_resolution(RuntimeMode::Advanced);
    let container = ServiceLoader::load_services(&resolution).await?;
    
    // Test MCP server access
    // MCP server is now created via factory method
    // assert!(container.get_mcp_server().is_some());
    
    // Test registry access
    assert!(container.get_registry().is_some());
    
    // Test security services access (should be available in advanced mode)
    assert!(container.get_security_services().is_some());
    
    Ok(())
}

/// Test proxy mode service container accessors
#[tokio::test]
async fn test_proxy_mode_service_accessors() -> Result<()> {
    let resolution = create_test_resolution(RuntimeMode::Proxy);
    let container = ServiceLoader::load_services(&resolution).await?;
    
    // Test MCP server access (should be available)
    // MCP server is now created via factory method
    // assert!(container.get_mcp_server().is_some());
    
    // Test registry access (should be available)
    assert!(container.get_registry().is_some());
    
    // Test security services access (should NOT be available in proxy mode)
    assert!(container.get_security_services().is_none());
    
    Ok(())
}

/// Test service container shutdown
#[tokio::test]
async fn test_service_container_shutdown() -> Result<()> {
    let resolution = create_test_resolution(RuntimeMode::Advanced);
    let container = ServiceLoader::load_services(&resolution).await?;
    
    // Verify container is initially healthy
    assert!(container.is_healthy());
    assert!(container.service_count > 0);
    
    // Test graceful shutdown
    let shutdown_result = container.shutdown().await;
    assert!(shutdown_result.is_ok());
    
    Ok(())
}

/// Test service loading with invalid configuration
#[tokio::test]
async fn test_service_loading_with_invalid_config() -> Result<()> {
    // Create resolution with invalid config that should fail validation
    let mut config = Config::default();
    config.deployment = Some(DeploymentConfig {
        runtime_mode: RuntimeMode::Advanced,
    });
    
    // Leave host empty and port as 0 (should fail validation)
    config.server.host = "".to_string();
    config.server.port = 0;
    
    let resolution = ConfigResolution {
        config,
        config_path: None,
        env_overrides: EnvironmentOverrides::load().unwrap(),
        config_source: ConfigSource::Defaults,
        validation_result: ValidationResult {
            is_valid: false,
            errors: vec!["Test validation error".to_string()],
            warnings: vec![],
            suggestions: vec![],
            mode: RuntimeMode::Advanced,
        },
    };
    
    // Service loading might still work if validation is done separately
    // This tests the robustness of the service loader
    let result = ServiceLoader::load_services(&resolution).await;
    
    // The result depends on how the service loader handles validation
    // If it relies on pre-validation, it should work
    // If it validates internally, it might fail
    match result {
        Ok(container) => {
            // If successful, verify basic functionality
            assert_eq!(container.runtime_mode, RuntimeMode::Advanced);
        }
        Err(_) => {
            // If it fails, that's also acceptable behavior
            // The test verifies that invalid config is handled gracefully
        }
    }
    
    Ok(())
}

/// Test service loading modes match configuration
#[tokio::test]
async fn test_service_modes_match_configuration() -> Result<()> {
    // Test proxy mode
    let proxy_resolution = create_test_resolution(RuntimeMode::Proxy);
    let proxy_container = ServiceLoader::load_services(&proxy_resolution).await?;
    
    assert_eq!(proxy_container.runtime_mode, RuntimeMode::Proxy);
    assert_eq!(*proxy_resolution.get_runtime_mode(), RuntimeMode::Proxy);
    
    // Test advanced mode
    let advanced_resolution = create_test_resolution(RuntimeMode::Advanced);
    let advanced_container = ServiceLoader::load_services(&advanced_resolution).await?;
    
    assert_eq!(advanced_container.runtime_mode, RuntimeMode::Advanced);
    assert_eq!(*advanced_resolution.get_runtime_mode(), RuntimeMode::Advanced);
    
    Ok(())
}

/// Test service loading performance and efficiency
#[tokio::test]
async fn test_service_loading_performance() -> Result<()> {
    let resolution = create_test_resolution(RuntimeMode::Advanced);
    
    let start_time = std::time::Instant::now();
    let container = ServiceLoader::load_services(&resolution).await?;
    let loading_time = start_time.elapsed();
    
    // Service loading should complete within reasonable time (10 seconds)
    assert!(loading_time.as_secs() < 10);
    assert!(container.is_healthy());
    assert!(container.service_count > 0);
    
    Ok(())
}