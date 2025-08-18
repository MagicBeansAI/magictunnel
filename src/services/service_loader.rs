//! Service loading strategy with conditional initialization based on runtime mode

use crate::config::{Config, RuntimeMode, ConfigResolution};
use crate::error::{ProxyError, Result};
use crate::services::{ProxyServices, AdvancedServices, ServiceContainer, ServiceLoadingSummary};
use tracing::{info, debug, warn};

/// Service loading strategy that conditionally loads services based on runtime mode
pub struct ServiceLoader;

impl ServiceLoader {
    /// Load services based on runtime mode from configuration resolution
    pub async fn load_services(resolution: &ConfigResolution) -> Result<ServiceContainer> {
        let runtime_mode = resolution.get_runtime_mode();
        let config = &resolution.config;
        
        info!("🚀 Loading services for {} mode", runtime_mode);
        
        let service_container = match runtime_mode {
            RuntimeMode::Proxy => {
                debug!("Loading proxy mode services (core functionality only)");
                Self::load_proxy_services(config, resolution).await?
            }
            RuntimeMode::Advanced => {
                debug!("Loading advanced mode services (full enterprise features)");
                Self::load_advanced_services(config, resolution).await?
            }
        };
        
        // Log detailed service information
        Self::log_service_details(&service_container);
        
        // Validate service dependencies
        Self::validate_service_dependencies(&service_container, runtime_mode)?;
        
        info!("✅ Service loading completed for {} mode ({} total services)", 
              runtime_mode, service_container.service_count);
        Ok(service_container)
    }
    
    /// Log detailed service information for debugging and monitoring
    fn log_service_details(container: &ServiceContainer) {
        info!("📊 Service Container Details:");
        info!("   Runtime Mode: {}", container.runtime_mode);
        info!("   Total Services: {}", container.service_count);
        
        // Log proxy services
        if let Some(ref proxy_services) = container.proxy_services {
            info!("   📦 Proxy Services ({} services):", proxy_services.service_count());
            let proxy_summary = proxy_services.get_summary();
            for (i, service) in proxy_summary.iter().enumerate() {
                info!("      {}. {}", i + 1, service);
            }
        } else {
            warn!("   ❌ No proxy services loaded");
        }
        
        // Log advanced services
        if let Some(ref advanced_services) = container.advanced_services {
            info!("   🏢 Advanced Services ({} services):", advanced_services.service_count());
            let advanced_summary = advanced_services.get_summary();
            for (i, service) in advanced_summary.iter().enumerate() {
                info!("      {}. {}", i + 1, service);
            }
        } else {
            info!("   📝 No advanced services (proxy mode)");
        }
        
        // Log service health
        if container.is_healthy() {
            info!("   ✅ All services are healthy");
        } else {
            warn!("   ⚠️  Some services may have health issues");
        }
    }
    
    /// Load core proxy services only
    async fn load_proxy_services(config: &Config, resolution: &ConfigResolution) -> Result<ServiceContainer> {
        debug!("Initializing proxy services with minimal footprint");
        
        let proxy_services = ProxyServices::new(config.clone()).await
            .map_err(|e| ProxyError::config(format!("Failed to initialize proxy services: {}", e)))?;
        
        let service_count = proxy_services.service_count();
        debug!("✅ Proxy services loaded ({} services)", service_count);
        debug!("📊 Service count for proxy mode: {}", service_count);
        
        Ok(ServiceContainer {
            proxy_services: Some(proxy_services),
            advanced_services: None,
            runtime_mode: RuntimeMode::Proxy,
            service_count,
            config_file_path: resolution.config_path.clone(),
        })
    }
    
    /// Load all services including advanced enterprise features
    async fn load_advanced_services(config: &Config, resolution: &ConfigResolution) -> Result<ServiceContainer> {
        debug!("Initializing advanced services with enterprise features");
        
        // First load proxy services (always required)
        debug!("Step 1: Loading proxy services for advanced mode");
        let proxy_services = ProxyServices::new(config.clone()).await
            .map_err(|e| ProxyError::config(format!("Failed to initialize proxy services: {}", e)))?;
        debug!("✅ Proxy services loaded ({} services)", proxy_services.service_count());
        
        // Then load advanced services
        debug!("Step 2: Loading advanced enterprise services");
        let advanced_services = AdvancedServices::new(config.clone(), &proxy_services).await
            .map_err(|e| ProxyError::config(format!("Failed to initialize advanced services: {}", e)))?;
        debug!("✅ Advanced services loaded ({} services)", advanced_services.service_count());
        
        let total_services = proxy_services.service_count() + advanced_services.service_count();
        debug!("📊 Total service count: {} + {} = {}", 
               proxy_services.service_count(), 
               advanced_services.service_count(), 
               total_services);
        
        Ok(ServiceContainer {
            proxy_services: Some(proxy_services),
            advanced_services: Some(advanced_services),
            runtime_mode: RuntimeMode::Advanced,
            service_count: total_services,
            config_file_path: resolution.config_path.clone(),
        })
    }
    
    /// Validate service dependencies and loading order
    fn validate_service_dependencies(container: &ServiceContainer, mode: &RuntimeMode) -> Result<()> {
        debug!("Validating service dependencies for {} mode", mode);
        
        // Proxy services are always required
        if container.proxy_services.is_none() {
            return Err(ProxyError::config("Proxy services are required but not loaded".to_string()));
        }
        
        let proxy_services = container.proxy_services.as_ref().unwrap();
        
        // Validate proxy service dependencies
        if !proxy_services.is_healthy() {
            return Err(ProxyError::config("Proxy services failed health check".to_string()));
        }
        
        match mode {
            RuntimeMode::Proxy => {
                // Proxy mode should not have advanced services
                if container.advanced_services.is_some() {
                    warn!("Advanced services loaded in proxy mode - this may indicate a configuration issue");
                }
                debug!("Proxy mode service validation passed");
            }
            RuntimeMode::Advanced => {
                // Advanced mode requires advanced services
                if container.advanced_services.is_none() {
                    return Err(ProxyError::config("Advanced mode requires advanced services".to_string()));
                }
                
                let advanced_services = container.advanced_services.as_ref().unwrap();
                
                // Validate advanced service dependencies
                if !advanced_services.is_healthy() {
                    return Err(ProxyError::config("Advanced services failed health check".to_string()));
                }
                
                // Check interdependencies
                if !advanced_services.validate_dependencies(proxy_services) {
                    return Err(ProxyError::config("Advanced services dependency validation failed".to_string()));
                }
                
                debug!("Advanced mode service validation passed");
            }
        }
        
        Ok(())
    }
    
    /// Get service loading summary for logging
    pub fn get_loading_summary(container: &ServiceContainer) -> ServiceLoadingSummary {
        let proxy_summary = container.proxy_services.as_ref()
            .map(|s| s.get_summary())
            .unwrap_or_default();
        
        let advanced_summary = container.advanced_services.as_ref()
            .map(|s| s.get_summary())
            .unwrap_or_default();
        
        ServiceLoadingSummary {
            runtime_mode: container.runtime_mode.clone(),
            total_services: container.service_count,
            proxy_services: proxy_summary,
            advanced_services: Some(advanced_summary),
            loading_time_ms: 0, // Will be filled by caller
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DeploymentConfig};
    
    fn create_test_resolution(mode: RuntimeMode) -> ConfigResolution {
        let mut config = Config::default();
        config.deployment = Some(DeploymentConfig {
            runtime_mode: mode.clone(),
        });
        
        ConfigResolution {
            config,
            config_path: None,
            env_overrides: crate::config::EnvironmentOverrides::load().unwrap(),
            config_source: crate::config::ConfigSource::Defaults,
            validation_result: crate::config::ValidationResult {
                is_valid: true,
                errors: vec![],
                warnings: vec![],
                suggestions: vec![],
                mode,
            },
        }
    }
    
    #[tokio::test]
    async fn test_proxy_service_loading() {
        let resolution = create_test_resolution(RuntimeMode::Proxy);
        
        let result = ServiceLoader::load_services(&resolution).await;
        
        // Should succeed with proxy services only
        assert!(result.is_ok());
        let container = result.unwrap();
        assert_eq!(container.runtime_mode, RuntimeMode::Proxy);
        assert!(container.proxy_services.is_some());
        assert!(container.advanced_services.is_none());
    }
    
    #[tokio::test]
    async fn test_advanced_service_loading() {
        let resolution = create_test_resolution(RuntimeMode::Advanced);
        
        let result = ServiceLoader::load_services(&resolution).await;
        
        // Should succeed with both proxy and advanced services
        assert!(result.is_ok());
        let container = result.unwrap();
        assert_eq!(container.runtime_mode, RuntimeMode::Advanced);
        assert!(container.proxy_services.is_some());
        assert!(container.advanced_services.is_some());
    }
}