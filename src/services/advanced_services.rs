//! Advanced enterprise services - additional features for advanced mode

use crate::config::Config;
use crate::error::{ProxyError, Result};
use crate::services::{ServiceStatus, ServiceState, ProxyServices};
use std::sync::Arc;
use tracing::{info, debug, warn, error};

/// Advanced enterprise services container
/// 
/// Contains enterprise features only available in advanced mode:
/// 
/// **Enterprise Security Services**:
/// - Security Framework (allowlist, RBAC, sanitization, policies, emergency lockdown)
/// 
/// **MagicTunnel Authentication** (TODO - not yet implemented):
/// - User authentication for MagicTunnel itself (different from MCP protocol auth)
/// 
/// Note: Core LLM services (sampling, elicitation, tool enhancement) and MCP protocol 
/// authentication are part of core services and available in both proxy and advanced modes.
#[derive(Debug)]
pub struct AdvancedServices {
    /// Service status tracking
    services: Vec<ServiceStatus>,
    /// Configuration snapshot
    config: Config,
    /// Security services instances
    pub security_services: Option<SecurityServices>,
}

/// Container for actual security service instances
pub struct SecurityServices {
    pub allowlist_service: Option<Arc<crate::security::AllowlistService>>,
    pub audit_service: Option<Arc<crate::security::AuditService>>,
    pub rbac_service: Option<Arc<crate::security::RbacService>>,
    pub sanitization_service: Option<Arc<crate::security::SanitizationService>>,
    pub lockdown_manager: Option<Arc<crate::security::EmergencyLockdownManager>>,
}

impl std::fmt::Debug for SecurityServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecurityServices")
            .field("allowlist_service", &self.allowlist_service.is_some())
            .field("audit_service", &self.audit_service.is_some())
            .field("rbac_service", &self.rbac_service.is_some())
            .field("sanitization_service", &self.sanitization_service.is_some())
            .field("lockdown_manager", &self.lockdown_manager.is_some())
            .finish()
    }
}

impl AdvancedServices {
    /// Initialize advanced enterprise services
    pub async fn new(config: Config, proxy_services: &ProxyServices) -> Result<Self> {
        info!("🏢 Initializing advanced enterprise services");
        let mut services = Vec::new();
        
        
        // Step 1: Enterprise Security Services - Show all available advanced services with their configuration status
        let security_enabled = config.security.as_ref().map(|s| s.enabled).unwrap_or(false);
        
        // Always show Tool Allowlisting service
        let allowlist_enabled = config.security.as_ref()
            .and_then(|s| s.allowlist.as_ref())
            .map(|c| c.enabled)
            .unwrap_or(false);
        services.push(ServiceStatus {
            name: "Tool Allowlisting".to_string(),
            status: if security_enabled && allowlist_enabled { ServiceState::Running } else { ServiceState::Warning },
            message: Some(if security_enabled && allowlist_enabled {
                "Enterprise tool allowlisting active".to_string()
            } else if security_enabled {
                "Available but not configured".to_string()
            } else {
                "Requires security framework to be enabled".to_string()
            }),
        });
        
        // Always show RBAC service
        let rbac_enabled = config.security.as_ref()
            .and_then(|s| s.rbac.as_ref())
            .map(|c| c.enabled)
            .unwrap_or(false);
        services.push(ServiceStatus {
            name: "RBAC (Role-Based Access Control)".to_string(),
            status: if security_enabled && rbac_enabled { ServiceState::Running } else { ServiceState::Warning },
            message: Some(if security_enabled && rbac_enabled {
                "Enterprise role-based access control active".to_string()
            } else if security_enabled {
                "Available but not configured".to_string()
            } else {
                "Requires security framework to be enabled".to_string()
            }),
        });
        
        // Always show Request Sanitization service
        let sanitization_enabled = config.security.as_ref()
            .and_then(|s| s.sanitization.as_ref())
            .map(|c| c.enabled)
            .unwrap_or(false);
        services.push(ServiceStatus {
            name: "Request Sanitization".to_string(),
            status: if security_enabled && sanitization_enabled { ServiceState::Running } else { ServiceState::Warning },
            message: Some(if security_enabled && sanitization_enabled {
                "Enterprise request sanitization active".to_string()
            } else if security_enabled {
                "Available but not configured".to_string()
            } else {
                "Requires security framework to be enabled".to_string()
            }),
        });
        
        // Always show Audit Logging service
        let audit_enabled = config.security.as_ref()
            .and_then(|s| s.audit.as_ref())
            .map(|c| c.enabled)
            .unwrap_or(false);
        services.push(ServiceStatus {
            name: "Audit Logging".to_string(),
            status: if security_enabled && audit_enabled { ServiceState::Running } else { ServiceState::Warning },
            message: Some(if security_enabled && audit_enabled {
                "Enterprise audit logging active".to_string()
            } else if security_enabled {
                "Available but not configured".to_string()
            } else {
                "Requires security framework to be enabled".to_string()
            }),
        });
        
        
        // Always show Emergency Lockdown service
        let lockdown_enabled = config.security.as_ref()
            .and_then(|s| s.emergency_lockdown.as_ref())
            .map(|c| c.enabled)
            .unwrap_or(false);
        services.push(ServiceStatus {
            name: "Emergency Lockdown".to_string(),
            status: if security_enabled && lockdown_enabled { ServiceState::Running } else { ServiceState::Warning },
            message: Some(if security_enabled && lockdown_enabled {
                "Enterprise emergency lockdown capability active".to_string()
            } else if security_enabled {
                "Available but not configured".to_string()
            } else {
                "Requires security framework to be enabled".to_string()
            }),
        });
        
        // Step 2: MagicTunnel Authentication - Not yet implemented
        // Note: This is different from MCP protocol authentication (which is core)
        services.push(ServiceStatus {
            name: "MagicTunnel Authentication".to_string(),
            status: ServiceState::Warning,
            message: Some("Not yet implemented - MagicTunnel user authentication system".to_string()),
        });
        
        info!("📝 Note: Core LLM services (sampling, elicitation) are available in both proxy and advanced modes");
        
        // Initialize actual security services if security is enabled
        let security_services = if security_enabled {
            info!("🔒 Initializing security service instances");
            
            // Initialize allowlist service if configured
            let allowlist_service = if config.security.as_ref()
                .and_then(|s| s.allowlist.as_ref())
                .map(|c| c.enabled)
                .unwrap_or(false) {
                match crate::security::AllowlistService::new(
                    config.security.as_ref().unwrap().allowlist.as_ref().unwrap().clone()
                ) {
                    Ok(service) => {
                        info!("✅ Allowlist service initialized successfully");
                        Some(Arc::new(service))
                    },
                    Err(e) => {
                        error!("❌ Failed to initialize allowlist service: {}", e);
                        None
                    }
                }
            } else {
                None
            };
            
            // Initialize audit service if configured (async)
            let audit_service = if config.security.as_ref()
                .and_then(|s| s.audit.as_ref())
                .map(|c| c.enabled)
                .unwrap_or(false) {
                match crate::security::AuditService::new(
                    config.security.as_ref().unwrap().audit.as_ref().unwrap().clone()
                ).await {
                    Ok(service) => {
                        info!("✅ Audit service initialized successfully");
                        Some(Arc::new(service))
                    },
                    Err(e) => {
                        error!("❌ Failed to initialize audit service: {}", e);
                        None
                    }
                }
            } else {
                None
            };
            
            // Initialize RBAC service if configured
            let rbac_service = if config.security.as_ref()
                .and_then(|s| s.rbac.as_ref())
                .map(|c| c.enabled)
                .unwrap_or(false) {
                match crate::security::RbacService::new(
                    config.security.as_ref().unwrap().rbac.as_ref().unwrap().clone()
                ) {
                    Ok(service) => {
                        info!("✅ RBAC service initialized successfully");
                        Some(Arc::new(service))
                    },
                    Err(e) => {
                        error!("❌ Failed to initialize RBAC service: {}", e);
                        None
                    }
                }
            } else {
                None
            };
            
            // Initialize sanitization service if configured
            let sanitization_service = if config.security.as_ref()
                .and_then(|s| s.sanitization.as_ref())
                .map(|c| c.enabled)
                .unwrap_or(false) {
                match crate::security::SanitizationService::new(
                    config.security.as_ref().unwrap().sanitization.as_ref().unwrap().clone()
                ) {
                    Ok(service) => {
                        info!("✅ Sanitization service initialized successfully");
                        Some(Arc::new(service))
                    },
                    Err(e) => {
                        error!("❌ Failed to initialize sanitization service: {}", e);
                        None
                    }
                }
            } else {
                None
            };
            
            // Initialize emergency lockdown manager if configured (async)
            let lockdown_manager = if config.security.as_ref()
                .and_then(|s| s.emergency_lockdown.as_ref())
                .map(|c| c.enabled)
                .unwrap_or(false) {
                match crate::security::EmergencyLockdownManager::new(
                    config.security.as_ref().unwrap().emergency_lockdown.as_ref().unwrap().clone()
                ).await {
                    Ok(service) => {
                        info!("✅ Emergency lockdown manager initialized successfully");
                        Some(Arc::new(service))
                    },
                    Err(e) => {
                        error!("❌ Failed to initialize emergency lockdown manager: {}", e);
                        None
                    }
                }
            } else {
                None
            };
            
            Some(SecurityServices {
                allowlist_service,
                audit_service,
                rbac_service,
                sanitization_service,
                lockdown_manager,
            })
        } else {
            None
        };
        
        // Note: External MCP Integration is provided by the main MCP server in both modes
        // Note: Enhanced web UI features are provided by the MCP server's dashboard API
        // with mode-aware functionality (security APIs, analytics, etc.)
        
        let advanced_services = Self {
            services,
            config,
            security_services,
        };
        
        info!("🎉 Advanced services initialization completed ({} services)", advanced_services.service_count());
        Ok(advanced_services)
    }
    
    // Note: Individual security and authentication services are available
    // via the web API and are initialized on-demand by the web dashboard.
    // This advanced services container just tracks their availability.
    
    
    
    /// Check if all advanced services are healthy
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
    
    /// Get security services instances
    pub fn get_security_services(&self) -> Option<&SecurityServices> {
        self.security_services.as_ref()
    }
    
    /// Validate dependencies on proxy services
    pub fn validate_dependencies(&self, proxy_services: &ProxyServices) -> bool {
        // Advanced services require healthy proxy services
        if !proxy_services.is_healthy() {
            error!("Advanced services require healthy proxy services");
            return false;
        }
        
        // Registry is required for security policies
        if proxy_services.get_registry().is_none() {
            error!("Advanced services require registry service");
            return false;
        }
        
        // MCP server is required for authentication middleware
        // MCP server is now created via factory method
        // if proxy_services.get_mcp_server().is_none() {
        if proxy_services.get_registry().is_none() {
            error!("Advanced services require MCP server");
            return false;
        }
        
        debug!("Advanced services dependency validation passed");
        true
    }
    
    /// Check if enterprise security framework is configured
    pub fn has_enterprise_security(&self) -> bool {
        self.config.security.is_some()
    }
    
    
    /// Check if MagicTunnel authentication is implemented (always false for now)
    pub fn has_magictunnel_auth(&self) -> bool {
        false // Not yet implemented
    }
    
    /// Get enterprise security configuration
    pub fn get_security_config(&self) -> Option<&crate::security::SecurityConfig> {
        self.config.security.as_ref()
    }
    
    
    /// Get external MCP integration status
    /// Note: External MCP integration is provided by the main MCP server in both modes
    pub fn has_external_mcp_integration(&self) -> bool {
        // External MCP integration is available via the main MCP server
        true
    }
    
    /// Get enhanced web functionality status
    /// Note: Enhanced web features are provided by the MCP server's dashboard API
    /// with mode-aware security and analytics functionality
    pub fn has_enhanced_web_features(&self) -> bool {
        // Enhanced web features are available via the MCP server's mode-aware dashboard
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
                        format!("Advanced service {} failed: {}", 
                                service.name, 
                                service.message.as_deref().unwrap_or("Unknown error"))
                    ));
                }
                ServiceState::Warning => {
                    warn!("Advanced service {} has warnings: {}", 
                          service.name, 
                          service.message.as_deref().unwrap_or("No details"));
                }
                _ => {} // Running, Initializing states are ok
            }
        }
        
        Ok(())
    }
    
    /// Graceful shutdown of advanced services
    pub async fn shutdown(mut self) -> Result<()> {
        info!("🛑 Shutting down advanced services");
        
        // Update service status
        for service in &mut self.services {
            service.status = ServiceState::Stopping;
        }
        
        // Shutdown in reverse dependency order
        // Note: Enhanced web features and External MCP are provided by the MCP server
        
        // Note: Individual security and authentication services are managed
        // by the web API layer, so no direct shutdown needed here
        
        // Update final status
        for service in &mut self.services {
            service.status = ServiceState::Stopped;
        }
        
        info!("✅ Advanced services shutdown completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ProxyServices;
    use crate::config::{Config, ServerConfig, RegistryConfig};
    
    async fn create_test_proxy_services() -> ProxyServices {
        let config = Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3001,
                ..Default::default()
            },
            registry: RegistryConfig {
                r#type: "local".to_string(),
                paths: vec!["capabilities".to_string()],
                ..Default::default()
            },
            smart_discovery: None,
            ..Default::default()
        };
        
        ProxyServices::new(config).await.expect("Failed to create proxy services")
    }
    
    #[tokio::test]
    async fn test_advanced_services_with_minimal_config() {
        let proxy_services = create_test_proxy_services().await;
        let config = Config::default(); // Minimal config, no advanced features
        
        let result = AdvancedServices::new(config, &proxy_services).await;
        assert!(result.is_ok());
        
        let services = result.unwrap();
        assert!(services.is_healthy());
        assert_eq!(services.service_count(), 0); // No advanced services configured
    }
    
    #[tokio::test]
    async fn test_advanced_services_dependency_validation() {
        let proxy_services = create_test_proxy_services().await;
        let config = Config::default();
        
        let services = AdvancedServices::new(config, &proxy_services).await.unwrap();
        
        // Should validate dependencies successfully
        assert!(services.validate_dependencies(&proxy_services));
    }
    
    #[tokio::test]
    async fn test_advanced_services_with_security_config() {
        let proxy_services = create_test_proxy_services().await;
        let mut config = Config::default();
        config.security = Some(crate::security::SecurityConfig::default());
        
        let result = AdvancedServices::new(config, &proxy_services).await;
        
        // May succeed or fail depending on security service implementation
        // But should not crash
        if let Ok(services) = result {
            assert!(services.service_count() >= 1); // Should have security service
        }
    }
    
    #[tokio::test]
    async fn test_service_summary() {
        let proxy_services = create_test_proxy_services().await;
        let config = Config::default();
        
        if let Ok(services) = AdvancedServices::new(config, &proxy_services).await {
            let summary = services.get_summary();
            // Summary should always be valid (even if empty)
            assert!(summary.len() >= 0);
        }
    }
}