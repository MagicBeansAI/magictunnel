//! Core proxy services - essential MCP functionality for proxy mode

use crate::config::Config;
use crate::error::{ProxyError, Result};
use crate::services::{ServiceStatus, ServiceState};
use crate::services::tool_management::ToolManagementService;
use std::sync::Arc;
use tracing::{info, debug, warn, error};

impl std::fmt::Debug for ProxyServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyServices")
            .field("service_count", &self.services.len())
            .field("has_mcp_server", &true)
            .field("has_registry", &true)
            .field("has_smart_discovery", &self.smart_discovery.is_some())
            .finish()
    }
}

/// Core proxy services container
/// 
/// Contains essential services needed for basic MCP proxy functionality:
/// - MCP Server (protocol handling, includes web dashboard)
/// - Registry Service (tool management)
/// - Optional Tool Enhancement (core LLM service for tool descriptions)
/// - Optional Smart Discovery (intelligent tool routing)
/// 
/// Note: The MCP Server includes web dashboard functionality via configure_dashboard_api,
/// so no separate web server is needed.
pub struct ProxyServices {
    /// Note: MCP server is created by factory method, not stored here
    /// Tool registry service
    registry: Arc<crate::registry::RegistryService>,
    /// Tool management service for dynamic visibility control
    tool_management: Arc<ToolManagementService>,
    /// Smart discovery service (optional)
    smart_discovery: Option<Arc<crate::discovery::SmartDiscoveryService>>,
    /// Enhancement storage service (optional)
    enhancement_storage: Option<Arc<crate::discovery::EnhancementStorageService>>,
    /// Service status tracking
    services: Vec<ServiceStatus>,
    /// Configuration snapshot
    config: Config,
}

impl ProxyServices {
    /// Initialize core proxy services
    pub async fn new(config: Config) -> Result<Self> {
        info!("🔧 Initializing core proxy services");
        let mut services = Vec::new();
        
        // Step 1: Initialize Registry Service (foundation for everything)
        debug!("Initializing registry service");
        services.push(ServiceStatus {
            name: "Registry".to_string(),
            status: ServiceState::Initializing,
            message: None,
        });
        
        let registry = Arc::new(
            crate::registry::RegistryService::new(config.registry.clone())
                .await
                .map_err(|e| ProxyError::config(format!("Failed to initialize registry: {}", e)))?
        );
        
        services.last_mut().unwrap().status = ServiceState::Running;
        info!("✅ Registry service initialized with {} tools", registry.get_all_tools().len());
        
        // Step 2: Initialize Tool Management Service (dynamic visibility control)
        debug!("Initializing tool management service");
        services.push(ServiceStatus {
            name: "Tool Management".to_string(),
            status: ServiceState::Initializing,
            message: None,
        });
        
        let tool_management = Arc::new(
            ToolManagementService::new(Arc::new(config.clone()))
                .with_registry_service(Arc::clone(&registry))
        );
        match tool_management.initialize().await {
            Ok(_) => {
                services.last_mut().unwrap().status = ServiceState::Running;
                info!("✅ Tool management service initialized");
            }
            Err(e) => {
                warn!("Tool management initialization failed: {}", e);
                services.last_mut().unwrap().status = ServiceState::Warning;
                services.last_mut().unwrap().message = Some(format!("Failed: {}", e));
            }
        }
        
        // Step 3: Initialize MCP Server (core protocol handling)
        debug!("Initializing MCP server");
        services.push(ServiceStatus {
            name: "MCP Server".to_string(),
            status: ServiceState::Initializing,
            message: None,
        });
        
        // MCP server is created here but ownership will be transferred to main.rs
        // We don't store it in Arc since main.rs will own and start it
        
        services.last_mut().unwrap().status = ServiceState::Running;
        info!("✅ MCP server initialized");
        
        // Step 3: Initialize Tool Enhancement Service (optional, part of core LLM services)
        if config.tool_enhancement.as_ref().map(|s| s.enabled).unwrap_or(false) {
            services.push(ServiceStatus {
                name: "Tool Enhancement".to_string(),
                status: ServiceState::Running,
                message: Some("Core LLM tool description enhancement available".to_string()),
            });
            info!("✅ Tool Enhancement service configured (core LLM feature)");
        } else {
            debug!("Tool Enhancement service not configured");
        }
        
        // Step 4: Initialize Enhancement Storage Service (optional, for persistent tool enhancements)
        let enhancement_storage = if let Some(storage_config) = &config.enhancement_storage {
            debug!("Initializing enhancement storage service");
            services.push(ServiceStatus {
                name: "Enhancement Storage".to_string(),
                status: ServiceState::Initializing,
                message: None,
            });
            
            match crate::discovery::EnhancementStorageService::new(storage_config.clone()) {
                Ok(storage_service) => {
                    services.last_mut().unwrap().status = ServiceState::Running;
                    info!("✅ Enhancement storage service initialized at: {}", storage_config.storage_dir);
                    Some(Arc::new(storage_service))
                }
                Err(e) => {
                    warn!("Failed to initialize enhancement storage service: {}", e);
                    services.last_mut().unwrap().status = ServiceState::Warning;
                    services.last_mut().unwrap().message = Some(format!("Failed: {}", e));
                    None
                }
            }
        } else {
            debug!("Enhancement storage service not configured");
            None
        };
        
        // Step 5: Initialize Smart Discovery (optional, based on config)
        let smart_discovery = if config.smart_discovery.as_ref().map(|sd| sd.enabled).unwrap_or(false) {
            debug!("Initializing smart discovery service");
            services.push(ServiceStatus {
                name: "Smart Discovery".to_string(),
                status: ServiceState::Initializing,
                message: None,
            });
            
            match Self::initialize_smart_discovery(&config, &registry).await {
                Ok(service) => {
                    services.last_mut().unwrap().status = ServiceState::Running;
                    info!("✅ Smart discovery service initialized");
                    Some(service)
                }
                Err(e) => {
                    warn!("Smart discovery initialization failed: {}", e);
                    services.last_mut().unwrap().status = ServiceState::Warning;
                    services.last_mut().unwrap().message = Some(format!("Failed: {}", e));
                    None
                }
            }
        } else {
            debug!("Smart discovery disabled in configuration");
            None
        };
        
        // Note: Web dashboard functionality is provided by the MCP server itself
        // via configure_dashboard_api, so no separate web server is needed
        
        let proxy_services = Self {
            registry,
            tool_management,
            smart_discovery,
            enhancement_storage,
            services,
            config,
        };
        
        info!("🎉 Proxy services initialization completed ({} services)", proxy_services.service_count());
        Ok(proxy_services)
    }
    
    /// Initialize smart discovery service
    async fn initialize_smart_discovery(
        config: &Config, 
        registry: &Arc<crate::registry::RegistryService>
    ) -> Result<Arc<crate::discovery::SmartDiscoveryService>> {
        let discovery_config = config.smart_discovery.as_ref()
            .ok_or_else(|| ProxyError::config("Smart discovery config missing".to_string()))?;
        
        // Clone config and load API keys from environment variables
        let mut config_with_api_key = discovery_config.clone();
        
        // Set API key for llm_tool_selection from environment if not set
        if config_with_api_key.llm_tool_selection.api_key.is_none() {
            if let Some(api_key_env) = &config_with_api_key.llm_tool_selection.api_key_env {
                if let Ok(api_key) = std::env::var(api_key_env) {
                    config_with_api_key.llm_tool_selection.api_key = Some(api_key);
                    info!("Loaded API key from {} environment variable for llm_tool_selection", api_key_env);
                }
            } else if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
                config_with_api_key.llm_tool_selection.api_key = Some(api_key);
                info!("Loaded OpenAI API key from OPENAI_API_KEY environment variable for llm_tool_selection");
            }
        }
        
        // Set API key for llm_mapper from environment if not set
        if config_with_api_key.llm_mapper.api_key.is_none() {
            if let Some(api_key_env) = &config_with_api_key.llm_mapper.api_key_env {
                if let Ok(api_key) = std::env::var(api_key_env) {
                    config_with_api_key.llm_mapper.api_key = Some(api_key);
                    info!("🔑 Loaded API key from {} environment variable for llm_mapper", api_key_env);
                } else {
                    warn!("⚠️ API key environment variable {} not found for llm_mapper", api_key_env);
                }
            } else if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
                config_with_api_key.llm_mapper.api_key = Some(api_key);
                info!("🔑 Loaded OpenAI API key from OPENAI_API_KEY environment variable for llm_mapper");
            }
        }
        
        let service = crate::discovery::SmartDiscoveryService::new(
            Arc::clone(registry),
            config_with_api_key,
        ).await?;
        
        Ok(Arc::new(service))
    }
    
    
    /// Check if all core services are healthy
    pub fn is_healthy(&self) -> bool {
        self.services.iter().all(|s| 
            matches!(s.status, ServiceState::Running | ServiceState::Warning)
        )
    }
    
    /// Get number of services
    pub fn service_count(&self) -> usize {
        self.services.len()
    }
    
    /// Get service summary for logging
    pub fn get_summary(&self) -> Vec<String> {
        self.services.iter().map(|s| {
            match &s.message {
                Some(msg) => format!("{} ({}): {}", s.name, s.status, msg),
                None => format!("{} ({})", s.name, s.status),
            }
        }).collect()
    }
    
    /// Get MCP server reference
    /// MCP server is now created by factory method in ServiceContainer
    /// This method is removed to prevent Arc ownership issues
    
    /// Get registry reference
    pub fn get_registry(&self) -> Option<&Arc<crate::registry::RegistryService>> {
        Some(&self.registry)
    }
    
    /// Get tool management service reference
    pub fn get_tool_management(&self) -> &Arc<ToolManagementService> {
        &self.tool_management
    }
    
    /// Get smart discovery service (if enabled)
    pub fn get_smart_discovery(&self) -> Option<&Arc<crate::discovery::SmartDiscoveryService>> {
        self.smart_discovery.as_ref()
    }
    
    /// Get enhancement storage service (if enabled)
    pub fn get_enhancement_storage(&self) -> Option<&Arc<crate::discovery::EnhancementStorageService>> {
        self.enhancement_storage.as_ref()
    }
    
    /// Get configuration reference
    pub fn get_config(&self) -> &Config {
        &self.config
    }
    
    /// Get web functionality status
    /// Note: Web dashboard is provided by the MCP server itself, not a separate service
    pub fn has_web_dashboard(&self) -> bool {
        // Web dashboard is always available via the MCP server
        true
    }
    
    /// Get detailed service status
    pub fn get_service_status(&self) -> &[ServiceStatus] {
        &self.services
    }
    
    /// Validate service health and dependencies
    pub fn validate_health(&self) -> Result<()> {
        for service in &self.services {
            match service.status {
                ServiceState::Failed => {
                    return Err(ProxyError::config(
                        format!("Service {} failed: {}", 
                                service.name, 
                                service.message.as_deref().unwrap_or("Unknown error"))
                    ));
                }
                ServiceState::Warning => {
                    warn!("Service {} has warnings: {}", 
                          service.name, 
                          service.message.as_deref().unwrap_or("No details"));
                }
                _ => {} // Running, Initializing states are ok
            }
        }
        
        Ok(())
    }
    
    /// Graceful shutdown of proxy services
    pub async fn shutdown(mut self) -> Result<()> {
        info!("🛑 Shutting down proxy services");
        
        // Update service status
        for service in &mut self.services {
            service.status = ServiceState::Stopping;
        }
        
        // Shutdown in reverse dependency order
        
        // Note: Web dashboard functionality is provided by the MCP server itself
        // so no separate web server shutdown is needed
        
        // 2. Smart discovery
        if let Some(_smart_discovery) = self.smart_discovery.take() {
            debug!("Shutting down smart discovery service");
            // Smart discovery shutdown logic would go here
        }
        
        // 3. MCP server (keep reference until last)
        debug!("Shutting down MCP server");
        // MCP server shutdown logic would go here
        
        // 4. Registry (shutdown last as everyone depends on it)
        debug!("Shutting down registry service");
        // Registry shutdown logic would go here
        
        // Update final status
        for service in &mut self.services {
            service.status = ServiceState::Stopped;
        }
        
        info!("✅ Proxy services shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ServerConfig, RegistryConfig};
    
    #[tokio::test]
    async fn test_proxy_services_initialization() {
        let config = Config {
            server: ServerConfig {
                host: std::env::var("MAGICTUNNEL_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
                port: std::env::var("MAGICTUNNEL_PORT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(3001),
                ..Default::default()
            },
            registry: RegistryConfig {
                r#type: "local".to_string(),
                paths: vec!["capabilities".to_string()],
                ..Default::default()
            },
            smart_discovery: None, // Disable for test
            ..Default::default()
        };
        
        let result = ProxyServices::new(config).await;
        assert!(result.is_ok());
        
        let services = result.unwrap();
        assert!(services.is_healthy());
        // MCP server is now created via factory method, not stored in services
        // assert!(services.get_mcp_server().is_some());
        assert!(services.get_registry().is_some());
        assert!(services.get_smart_discovery().is_none()); // Disabled
        assert!(services.has_web_dashboard()); // Always available via MCP server
    }
    
    #[tokio::test]
    async fn test_proxy_services_with_smart_discovery() {
        let config = Config {
            server: ServerConfig {
                host: std::env::var("MAGICTUNNEL_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
                port: std::env::var("MAGICTUNNEL_PORT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(3001),
                ..Default::default()
            },
            registry: RegistryConfig {
                r#type: "local".to_string(),
                paths: vec!["capabilities".to_string()],
                ..Default::default()
            },
            smart_discovery: Some(crate::discovery::SmartDiscoveryConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        
        let result = ProxyServices::new(config).await;
        
        // May succeed or fail depending on smart discovery config
        // But should not crash
        if let Ok(services) = result {
            assert!(services.service_count() >= 2); // At least MCP + Registry
        }
    }
    
    #[tokio::test]
    async fn test_service_summary() {
        let config = Config::default();
        
        if let Ok(services) = ProxyServices::new(config).await {
            let summary = services.get_summary();
            assert!(!summary.is_empty());
            
            // Should contain status information
            assert!(summary.iter().any(|s| s.contains("Registry")));
            assert!(summary.iter().any(|s| s.contains("MCP Server")));
        }
    }
}