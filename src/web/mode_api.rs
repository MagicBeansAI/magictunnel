//! Mode detection API for frontend mode awareness

use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, info};
use crate::config::{Config, RuntimeMode, ConfigResolution};
use crate::services::{ServiceContainer, ServiceStatus as ServiceStatusType};

/// Runtime mode information for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeInfo {
    /// Current runtime mode
    pub runtime_mode: RuntimeMode,
    /// Human-readable mode description
    pub mode_description: String,
    /// Available features in this mode
    pub available_features: Vec<String>,
    /// Hidden features in this mode
    pub hidden_features: Vec<String>,
    /// Mode-specific UI configuration
    pub ui_config: ModeUIConfig,
}

/// UI configuration based on runtime mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeUIConfig {
    /// Show security management UI
    pub show_security_ui: bool,
    /// Show authentication settings
    pub show_auth_settings: bool,
    /// Show advanced analytics
    pub show_analytics: bool,
    /// Show audit logs
    pub show_audit_logs: bool,
    /// Show external MCP management
    pub show_external_mcp: bool,
    /// Show RBAC management
    pub show_rbac_management: bool,
    /// Navigation sections to display
    pub navigation_sections: Vec<NavigationSection>,
    /// Status indicators to show
    pub status_indicators: Vec<StatusIndicator>,
}

/// Navigation section configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationSection {
    /// Section identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Section icon
    pub icon: String,
    /// Whether section is visible
    pub visible: bool,
    /// Child pages/items
    pub items: Vec<NavigationItem>,
}

/// Navigation item configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationItem {
    /// Item identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Route path
    pub path: String,
    /// Item icon
    pub icon: String,
    /// Whether item is visible
    pub visible: bool,
    /// Whether item requires advanced mode
    pub requires_advanced: bool,
}

/// Status indicator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusIndicator {
    /// Indicator identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Indicator type
    pub indicator_type: String,
    /// Whether indicator is visible
    pub visible: bool,
}

/// Configuration resolution info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInfo {
    /// Configuration source
    pub config_source: String,
    /// Environment overrides applied
    pub env_overrides: Vec<String>,
    /// Validation status
    pub validation_status: ValidationStatus,
    /// Smart discovery status
    pub smart_discovery_enabled: bool,
}

/// Validation status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStatus {
    /// Whether configuration is valid
    pub is_valid: bool,
    /// Number of errors
    pub error_count: usize,
    /// Number of warnings
    pub warning_count: usize,
    /// Can the system start
    pub can_start: bool,
}

/// Mode API handlers
pub struct ModeApiHandler {
    /// Service container for mode detection
    service_container: Arc<ServiceContainer>,
    /// Configuration resolution
    config_resolution: Arc<ConfigResolution>,
}

impl ModeApiHandler {
    /// Create new mode API handler
    pub fn new(
        service_container: Arc<ServiceContainer>, 
        config_resolution: Arc<ConfigResolution>
    ) -> Self {
        Self {
            service_container,
            config_resolution,
        }
    }

    /// Get current mode information
    pub async fn get_mode_info(&self) -> Result<HttpResponse> {
        debug!("Fetching mode information for frontend");
        
        let runtime_mode = self.config_resolution.get_runtime_mode();
        let mode_info = ModeInfo {
            runtime_mode: runtime_mode.clone(),
            mode_description: Self::get_mode_description(runtime_mode),
            available_features: Self::get_available_features(runtime_mode),
            hidden_features: Self::get_hidden_features(runtime_mode),
            ui_config: Self::create_ui_config(runtime_mode, &self.service_container),
        };

        info!("Mode info requested: {} mode with {} features", 
              runtime_mode, mode_info.available_features.len());

        Ok(HttpResponse::Ok().json(mode_info))
    }

    /// Get configuration information
    pub async fn get_config_info(&self) -> Result<HttpResponse> {
        debug!("Fetching configuration information for frontend");
        
        let resolution = &self.config_resolution;
        let validation = &resolution.validation_result;
        
        let config_info = ConfigInfo {
            config_source: format!("{}", resolution.config_source),
            env_overrides: resolution.env_overrides.get_override_summary(),
            validation_status: ValidationStatus {
                is_valid: validation.is_valid,
                error_count: validation.errors.len(),
                warning_count: validation.warnings.len(),
                can_start: validation.can_start(),
            },
            smart_discovery_enabled: resolution.is_smart_discovery_enabled(),
        };

        Ok(HttpResponse::Ok().json(config_info))
    }

    /// Get service status for frontend
    pub async fn get_service_status(&self) -> Result<HttpResponse> {
        debug!("Fetching service status for frontend");
        
        let container = &self.service_container;
        let runtime_mode = container.runtime_mode.clone();
        
        let mut services = Vec::new();
        
        // Proxy services status
        if let Some(proxy_services) = &container.proxy_services {
            for status in proxy_services.get_service_status() {
                services.push(json!({
                    "name": status.name,
                    "status": status.status.to_string(),
                    "message": status.message,
                    "category": "proxy"
                }));
            }
        }
        
        // Advanced services status
        if let Some(advanced_services) = &container.advanced_services {
            for status in advanced_services.get_service_status() {
                services.push(json!({
                    "name": status.name,
                    "status": status.status.to_string(),
                    "message": status.message,
                    "category": "advanced"
                }));
            }
        }
        
        Ok(HttpResponse::Ok().json(json!({
            "runtime_mode": runtime_mode,
            "total_services": container.service_count,
            "is_healthy": container.is_healthy(),
            "services": services
        })))
    }

    /// Get human-readable mode description
    fn get_mode_description(mode: &RuntimeMode) -> String {
        match mode {
            RuntimeMode::Proxy => "Core MCP proxy functionality with essential tools and basic web interface".to_string(),
            RuntimeMode::Advanced => "Full enterprise features including security, authentication, audit logging, and advanced management".to_string(),
        }
    }

    /// Get available features for runtime mode
    fn get_available_features(mode: &RuntimeMode) -> Vec<String> {
        let mut features = vec![
            "MCP Server".to_string(),
            "Tool Registry".to_string(),
            "Basic Web Dashboard".to_string(),
        ];

        match mode {
            RuntimeMode::Proxy => {
                features.extend([
                    "Smart Discovery (optional)".to_string(),
                    "Tool Management".to_string(),
                ]);
            }
            RuntimeMode::Advanced => {
                features.extend([
                    "Smart Discovery (optional)".to_string(),
                    "Tool Management".to_string(),
                    "Security Framework".to_string(),
                    "Authentication System".to_string(),
                    "Audit Logging".to_string(),
                    "External MCP Integration".to_string(),
                    "Enhanced Web Dashboard".to_string(),
                    "RBAC Management".to_string(),
                    "Analytics & Monitoring".to_string(),
                ]);
            }
        }

        features
    }

    /// Get hidden features for runtime mode
    fn get_hidden_features(mode: &RuntimeMode) -> Vec<String> {
        match mode {
            RuntimeMode::Proxy => vec![
                "Security Management".to_string(),
                "Authentication Settings".to_string(),
                "Audit Logs".to_string(),
                "RBAC Configuration".to_string(),
                "Advanced Analytics".to_string(),
                "External MCP Management".to_string(),
            ],
            RuntimeMode::Advanced => vec![
                // All features available in advanced mode
            ],
        }
    }

    /// Create UI configuration for runtime mode
    fn create_ui_config(mode: &RuntimeMode, service_container: &ServiceContainer) -> ModeUIConfig {
        let is_advanced = matches!(mode, RuntimeMode::Advanced);
        
        ModeUIConfig {
            show_security_ui: is_advanced && service_container.get_security_services().is_some(),
            show_auth_settings: is_advanced,
            show_analytics: is_advanced,
            show_audit_logs: is_advanced,
            show_external_mcp: is_advanced,
            show_rbac_management: is_advanced,
            navigation_sections: Self::create_navigation_sections(mode),
            status_indicators: Self::create_status_indicators(mode),
        }
    }

    /// Create navigation sections based on runtime mode
    fn create_navigation_sections(mode: &RuntimeMode) -> Vec<NavigationSection> {
        let mut sections = vec![
            // Main section (always visible)
            NavigationSection {
                id: "main".to_string(),
                name: "Main".to_string(),
                icon: "home".to_string(),
                visible: true,
                items: vec![
                    NavigationItem {
                        id: "dashboard".to_string(),
                        name: "Dashboard".to_string(),
                        path: "/".to_string(),
                        icon: "dashboard".to_string(),
                        visible: true,
                        requires_advanced: false,
                    },
                    NavigationItem {
                        id: "tools".to_string(),
                        name: "Tools".to_string(),
                        path: "/tools".to_string(),
                        icon: "build".to_string(),
                        visible: true,
                        requires_advanced: false,
                    },
                    NavigationItem {
                        id: "resources".to_string(),
                        name: "Resources".to_string(),
                        path: "/resources".to_string(),
                        icon: "folder".to_string(),
                        visible: true,
                        requires_advanced: false,
                    },
                    NavigationItem {
                        id: "prompts".to_string(),
                        name: "Prompts".to_string(),
                        path: "/prompts".to_string(),
                        icon: "chat".to_string(),
                        visible: true,
                        requires_advanced: false,
                    },
                ],
            },
        ];

        // Add advanced sections in advanced mode
        if matches!(mode, RuntimeMode::Advanced) {
            sections.extend([
                // Security Section with comprehensive sub-options
                NavigationSection {
                    id: "security".to_string(),
                    name: "Security".to_string(),
                    icon: "security".to_string(),
                    visible: true,
                    items: vec![
                        NavigationItem {
                            id: "security-overview".to_string(),
                            name: "Security Overview".to_string(),
                            path: "/security".to_string(),
                            icon: "shield".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "security-policies".to_string(),
                            name: "Security Policies".to_string(),
                            path: "/security/policies".to_string(),
                            icon: "policy".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "security-config".to_string(),
                            name: "Security Config".to_string(),
                            path: "/security/config".to_string(),
                            icon: "settings".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "rbac".to_string(),
                            name: "RBAC".to_string(),
                            path: "/security/rbac".to_string(),
                            icon: "people".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "rbac-users".to_string(),
                            name: "Users".to_string(),
                            path: "/security/rbac/users".to_string(),
                            icon: "user".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "rbac-roles".to_string(),
                            name: "Roles".to_string(),
                            path: "/security/rbac/roles".to_string(),
                            icon: "badge".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "rbac-permissions".to_string(),
                            name: "Permissions".to_string(),
                            path: "/security/rbac/permissions".to_string(),
                            icon: "key".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "audit-logs".to_string(),
                            name: "Audit Logs".to_string(),
                            path: "/security/audit".to_string(),
                            icon: "history".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "audit-search".to_string(),
                            name: "Audit Search".to_string(),
                            path: "/security/audit/search".to_string(),
                            icon: "search".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "audit-violations".to_string(),
                            name: "Violations".to_string(),
                            path: "/security/audit/violations".to_string(),
                            icon: "warning".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "allowlist".to_string(),
                            name: "Allowlist".to_string(),
                            path: "/security/allowlist".to_string(),
                            icon: "check".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "sanitization".to_string(),
                            name: "Sanitization".to_string(),
                            path: "/security/sanitization".to_string(),
                            icon: "clean".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "sanitization-filtering".to_string(),
                            name: "Filtering".to_string(),
                            path: "/security/sanitization/filtering".to_string(),
                            icon: "filter".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "sanitization-secrets".to_string(),
                            name: "Secrets".to_string(),
                            path: "/security/sanitization/secrets".to_string(),
                            icon: "lock".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "sanitization-policies".to_string(),
                            name: "Sanitization Policies".to_string(),
                            path: "/security/sanitization/policies".to_string(),
                            icon: "document".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "sanitization-testing".to_string(),
                            name: "Testing".to_string(),
                            path: "/security/sanitization/testing".to_string(),
                            icon: "test".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                    ],
                },
                // MCP Services Section
                NavigationSection {
                    id: "mcp-services".to_string(),
                    name: "MCP Services".to_string(),
                    icon: "link".to_string(),
                    visible: true,
                    items: vec![
                        NavigationItem {
                            id: "services".to_string(),
                            name: "External MCP".to_string(),
                            path: "/services".to_string(),
                            icon: "link".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "llm-services".to_string(),
                            name: "LLM Services".to_string(),
                            path: "/llm-services".to_string(),
                            icon: "brain".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                    ],
                },
                // Administration Section
                NavigationSection {
                    id: "administration".to_string(),
                    name: "Administration".to_string(),
                    icon: "settings".to_string(),
                    visible: true,
                    items: vec![
                        NavigationItem {
                            id: "config".to_string(),
                            name: "Configuration".to_string(),
                            path: "/config".to_string(),
                            icon: "settings".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "monitoring".to_string(),
                            name: "Monitoring".to_string(),
                            path: "/monitoring".to_string(),
                            icon: "analytics".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "tool-metrics".to_string(),
                            name: "Tool Metrics".to_string(),
                            path: "/tool-metrics".to_string(),
                            icon: "chart".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                        NavigationItem {
                            id: "logs".to_string(),
                            name: "Logs".to_string(),
                            path: "/logs".to_string(),
                            icon: "document".to_string(),
                            visible: true,
                            requires_advanced: true,
                        },
                    ],
                },
            ]);
        } else {
            // In proxy mode, still show some basic sections but fewer advanced features
            sections.extend([
                NavigationSection {
                    id: "administration".to_string(),
                    name: "Administration".to_string(),
                    icon: "settings".to_string(),
                    visible: true,
                    items: vec![
                        NavigationItem {
                            id: "config".to_string(),
                            name: "Configuration".to_string(),
                            path: "/config".to_string(),
                            icon: "settings".to_string(),
                            visible: true,
                            requires_advanced: false,
                        },
                        NavigationItem {
                            id: "logs".to_string(),
                            name: "Logs".to_string(),
                            path: "/logs".to_string(),
                            icon: "document".to_string(),
                            visible: true,
                            requires_advanced: false,
                        },
                    ],
                },
            ]);
        }

        sections
    }

    /// Create status indicators based on runtime mode
    fn create_status_indicators(mode: &RuntimeMode) -> Vec<StatusIndicator> {
        let mut indicators = vec![
            StatusIndicator {
                id: "mcp-server".to_string(),
                name: "MCP Server".to_string(),
                indicator_type: "service".to_string(),
                visible: true,
            },
            StatusIndicator {
                id: "registry".to_string(),
                name: "Registry".to_string(),
                indicator_type: "service".to_string(),
                visible: true,
            },
        ];

        // Add advanced indicators only in advanced mode
        if matches!(mode, RuntimeMode::Advanced) {
            indicators.extend([
                StatusIndicator {
                    id: "security".to_string(),
                    name: "Security".to_string(),
                    indicator_type: "security".to_string(),
                    visible: true,
                },
                StatusIndicator {
                    id: "authentication".to_string(),
                    name: "Authentication".to_string(),
                    indicator_type: "auth".to_string(),
                    visible: true,
                },
            ]);
        }

        indicators
    }
}

/// Register mode API routes
pub fn configure_mode_api(cfg: &mut web::ServiceConfig, handler: Arc<ModeApiHandler>) {
    let handler_clone = Arc::clone(&handler);
    
    cfg.service(
        web::resource("/api/mode")
            .app_data(web::Data::new(handler_clone))
            .route(web::get().to(|data: web::Data<ModeApiHandler>| async move {
                data.get_mode_info().await
            }))
    )
    .service(
        web::resource("/api/config")
            .app_data(web::Data::new(Arc::clone(&handler)))
            .route(web::get().to(|data: web::Data<ModeApiHandler>| async move {
                data.get_config_info().await
            }))
    )
    .service(
        web::resource("/api/services/status")
            .app_data(web::Data::new(Arc::clone(&handler)))
            .route(web::get().to(|data: web::Data<ModeApiHandler>| async move {
                data.get_service_status().await
            }))
    );
}