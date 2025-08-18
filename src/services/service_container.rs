//! Service container for holding all loaded services

use crate::config::RuntimeMode;
use crate::error::Result;
use crate::services::{ProxyServices, AdvancedServices};
use std::sync::Arc;
use tracing::{info, error};

/// Container holding all loaded services
#[derive(Debug)]
pub struct ServiceContainer {
    /// Core proxy services (always present)
    pub proxy_services: Option<ProxyServices>,
    /// Advanced enterprise services (only in advanced mode)
    pub advanced_services: Option<AdvancedServices>,
    /// Runtime mode for this container
    pub runtime_mode: RuntimeMode,
    /// Total number of loaded services
    pub service_count: usize,
    /// Configuration file path for persistence
    pub config_file_path: Option<std::path::PathBuf>,
}

impl ServiceContainer {
    /// Check if all services are healthy
    pub fn is_healthy(&self) -> bool {
        let proxy_healthy = self.proxy_services.as_ref()
            .map(|s| s.is_healthy())
            .unwrap_or(false);
        
        let advanced_healthy = self.advanced_services.as_ref()
            .map(|s| s.is_healthy())
            .unwrap_or(true); // No advanced services is ok
        
        proxy_healthy && advanced_healthy
    }
    
    /// MCP server is now created by factory method to avoid Arc ownership issues
    /// Use create_mcp_server_for_main() instead
    
    /// Get the registry service from proxy services
    pub fn get_registry(&self) -> Option<&Arc<crate::registry::RegistryService>> {
        self.proxy_services.as_ref()?.get_registry()
    }

    /// Get the tool management service from proxy services
    pub fn get_tool_management(&self) -> Option<&Arc<crate::services::tool_management::ToolManagementService>> {
        Some(self.proxy_services.as_ref()?.get_tool_management())
    }

    /// Get the configuration from proxy services
    pub fn get_config(&self) -> Option<&crate::config::Config> {
        self.proxy_services.as_ref().map(|ps| ps.get_config())
    }

    /// Create a new MCP server instance for main.rs to own and start
    /// This avoids the Arc ownership issue by creating a fresh server
    pub async fn create_mcp_server_for_main(&self) -> Result<crate::mcp::McpServer> {
        let config = self.get_config()
            .ok_or_else(|| anyhow::anyhow!("Config not available"))?;
        
        // Get enhancement storage from proxy services if available
        let enhancement_storage = self.proxy_services
            .as_ref()
            .and_then(|ps| ps.get_enhancement_storage())
            .cloned();
        
        // Create MCP server with full configuration (including enhancement services)
        // This ensures the dashboard has access to all configured services
        let server = crate::mcp::McpServer::with_config_and_storage(config, enhancement_storage).await?;
        
        Ok(server)
    }
    
    /// Get security services (only available in advanced mode)
    pub fn get_security_services(&self) -> Option<&crate::services::advanced_services::SecurityServices> {
        self.advanced_services.as_ref()?.get_security_services()
    }
    
    /// Graceful shutdown of all services
    pub async fn shutdown(self) -> Result<()> {
        info!("🛑 Shutting down services gracefully");
        
        // Shutdown in reverse order (advanced first, then proxy)
        if let Some(advanced_services) = self.advanced_services {
            if let Err(e) = advanced_services.shutdown().await {
                error!("Failed to shutdown advanced services: {}", e);
            }
        }
        
        if let Some(proxy_services) = self.proxy_services {
            if let Err(e) = proxy_services.shutdown().await {
                error!("Failed to shutdown proxy services: {}", e);
            }
        }
        
        info!("✅ Service shutdown completed");
        Ok(())
    }
}