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
    /// This avoids the Arc ownership issue by using shared services when available
    pub async fn create_mcp_server_for_main(&self) -> Result<crate::mcp::McpServer> {
        let config = self.get_config()
            .ok_or_else(|| anyhow::anyhow!("Config not available"))?;
        
        // Get enhancement storage from proxy services if available
        let enhancement_storage = self.proxy_services
            .as_ref()
            .and_then(|ps| ps.get_enhancement_storage())
            .cloned();
            
        // Get smart discovery service from proxy services if available
        // This ensures we use the same instance that has the allowlist service set
        let smart_discovery_service = self.proxy_services
            .as_ref()
            .and_then(|ps| ps.get_smart_discovery())
            .map(|sd| Arc::clone(sd));
        
        if smart_discovery_service.is_some() {
            info!("🔄 Using existing smart discovery service from proxy services (maintains allowlist integration)");
        } else {
            info!("ℹ️ No smart discovery service available from proxy services - MCP server will create its own");
        }
        
        // Create MCP server with shared services (including smart discovery with allowlist integration)
        // This ensures the MCP server uses the same services with proper security validation
        let server = crate::mcp::McpServer::with_config_and_services(
            config, 
            enhancement_storage, 
            smart_discovery_service
        ).await?;
        
        Ok(server)
    }
    
    /// Get security services (only available in advanced mode)
    pub fn get_security_services(&self) -> Option<&crate::services::advanced_services::SecurityServices> {
        self.advanced_services.as_ref()?.get_security_services()
    }
    
    /// Integrate security services with smart discovery service (advanced mode only)
    /// This enables nested tool call security validation in smart discovery
    pub async fn integrate_security_with_discovery(&self) -> Result<()> {
        info!("🔍 DEBUG: Starting security integration check");
        
        // Only available in advanced mode with both proxy and advanced services
        if let (Some(proxy), Some(advanced)) = (&self.proxy_services, &self.advanced_services) {
            info!("🔍 DEBUG: Both proxy and advanced services available");
            
            if let Some(smart_discovery) = proxy.get_smart_discovery() {
                info!("🔍 DEBUG: Smart discovery service found");
                
                if let Some(security) = advanced.get_security_services() {
                    info!("🔍 DEBUG: Security services found");
                    
                    if let Some(allowlist_service) = &security.allowlist_service {
                        let instance_id = format!("{:p}", allowlist_service.as_ref());
                        info!("🔍 DEBUG: Allowlist service found - Instance ID: {}", instance_id);
                        info!("🔒 Integrating allowlist service with smart discovery for nested tool call security");
                        smart_discovery.set_allowlist_service(Arc::clone(allowlist_service)).await;
                        info!("✅ Smart discovery now has allowlist service - nested tool calls will be security validated");
                        return Ok(());
                    } else {
                        info!("🔍 DEBUG: No allowlist service in security services");
                    }
                } else {
                    info!("🔍 DEBUG: No security services in advanced services");
                }
            } else {
                info!("🔍 DEBUG: No smart discovery service in proxy services");
            }
        } else {
            info!("🔍 DEBUG: Missing proxy ({}) or advanced ({}) services", 
                  self.proxy_services.is_some(), 
                  self.advanced_services.is_some());
        }
        
        // Not an error - just means we're in proxy mode or services aren't available
        info!("ℹ️ Security integration not needed - proxy mode or services not available");
        Ok(())
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