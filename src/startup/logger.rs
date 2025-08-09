//! Comprehensive startup logging system

use crate::config::{ConfigResolution, RuntimeMode};
use tracing::{info, warn};

/// Startup logger for comprehensive system information
pub struct StartupLogger;

impl StartupLogger {
    /// Display comprehensive startup information
    pub fn display_startup_info(
        resolution: &ConfigResolution,
        version: &str,
        additional_info: Option<&StartupAdditionalInfo>,
    ) {
        // Main startup banner
        info!("🚀 MagicTunnel v{} starting...", version);
        info!("");

        // Configuration resolution section
        Self::display_configuration_resolution(resolution);
        
        // Environment overrides section
        if resolution.env_overrides.has_overrides() {
            Self::display_environment_overrides(resolution);
        }

        // Feature status section
        Self::display_feature_status(resolution, additional_info);

        // Validation results section
        Self::display_validation_results(resolution);

        // Server information section
        if let Some(info) = additional_info {
            Self::display_server_information(info);
        }

        // Final startup status
        info!("");
        info!("✅ MagicTunnel started successfully in {} mode", resolution.get_runtime_mode());
    }

    /// Display configuration resolution information
    fn display_configuration_resolution(resolution: &ConfigResolution) {
        info!("📁 Configuration Resolution:");
        
        match &resolution.config_path {
            Some(path) => {
                info!("   Config file: {:?} ✅", path);
            }
            None => {
                info!("   Config file: built-in defaults (no config file found)");
            }
        }

        let mode_source = if resolution.env_overrides.runtime_mode.is_some() {
            "via MAGICTUNNEL_RUNTIME_MODE env var"
        } else if resolution.config.deployment.as_ref().map(|d| &d.runtime_mode) != Some(&RuntimeMode::Proxy) {
            "via config file"
        } else {
            "default"
        };

        info!("   Runtime mode: {} ({})", resolution.get_runtime_mode(), mode_source);

        let discovery_source = if resolution.env_overrides.smart_discovery.is_some() {
            "via MAGICTUNNEL_SMART_DISCOVERY env var"
        } else if resolution.is_smart_discovery_enabled() {
            "via config file"
        } else {
            "default (disabled)"
        };

        info!("   Smart discovery: {} ({})", 
               if resolution.is_smart_discovery_enabled() { "enabled" } else { "disabled" },
               discovery_source);
        info!("");
    }

    /// Display environment overrides
    fn display_environment_overrides(resolution: &ConfigResolution) {
        info!("🔧 Environment Overrides:");
        
        let overrides = resolution.env_overrides.get_override_summary();
        for override_info in overrides {
            if override_info.contains("MAGICTUNNEL_RUNTIME_MODE") &&
               resolution.config.deployment.as_ref().map(|d| &d.runtime_mode) != resolution.env_overrides.runtime_mode.as_ref() {
                warn!("   ⚠️  {} (overriding config: {:?})", 
                     override_info,
                     resolution.config.deployment.as_ref().map(|d| &d.runtime_mode));
            } else {
                info!("   ✅ {}", override_info);
            }
        }
        info!("");
    }

    /// Display feature status by category
    fn display_feature_status(resolution: &ConfigResolution, additional_info: Option<&StartupAdditionalInfo>) {
        info!("🎯 Feature Status:");
        
        // Core Services (always available)
        info!("   Core Services:");
        info!("   ✅ MCP Server (proxy mode)");
        
        if let Some(info) = additional_info {
            if info.tools_loaded > 0 {
                info!("   ✅ Tool Registry ({} tools loaded)", info.tools_loaded);
            } else {
                info!("   ⚠️  Tool Registry (no tools loaded)");
            }
        }

        if resolution.is_smart_discovery_enabled() {
            if let Some(info) = additional_info {
                if !info.llm_providers.is_empty() {
                    info!("   ✅ Smart Discovery ({} configured)", info.llm_providers.join(", "));
                } else {
                    warn!("   ⚠️  Smart Discovery enabled but no LLM providers configured");
                }
            } else {
                info!("   ✅ Smart Discovery");
            }
        }

        // Advanced Services (only in advanced mode)
        match resolution.get_runtime_mode() {
            RuntimeMode::Advanced => {
                info!("   ");
                info!("   Advanced Services (runtime_mode: advanced):");
                
                if resolution.config.auth.is_some() {
                    info!("   ✅ Authentication System");
                } else {
                    warn!("   ⚠️  Authentication System (not configured)");
                }

                if resolution.config.security.is_some() {
                    info!("   ✅ Security Framework");
                } else {
                    warn!("   ⚠️  Security Framework (not configured)");
                }

                // Future: Add more advanced service checks here
                info!("   ❌ External Auth (providers not configured)");
            }
            RuntimeMode::Proxy => {
                // In proxy mode, advanced services are not loaded
            }
        }
        info!("");
    }

    /// Display validation results
    fn display_validation_results(resolution: &ConfigResolution) {
        info!("📊 Validation Results:");
        
        let validation = &resolution.validation_result;
        
        // Display overall validation status
        if validation.can_start() {
            info!("   ✅ Configuration validation passed ({} mode)", validation.mode);
        } else {
            warn!("   ❌ Configuration validation failed ({} errors)", validation.errors.len());
        }
        
        // Display errors (if any)
        if !validation.errors.is_empty() {
            info!("   ");
            info!("   Errors that must be fixed:");
            for error in &validation.errors {
                warn!("   ❌ {}", error);
            }
        }
        
        // Display warnings (if any)
        if !validation.warnings.is_empty() {
            if !validation.errors.is_empty() {
                info!("   ");
            }
            info!("   Warnings (non-critical):");
            for warning in &validation.warnings {
                warn!("   ⚠️  {}", warning);
            }
        }
        
        // Display summary
        if validation.errors.is_empty() && validation.warnings.is_empty() {
            info!("   ✨ No issues found - configuration is optimal");
        } else {
            let total_issues = validation.errors.len() + validation.warnings.len();
            info!("   📝 Total issues: {} ({} errors, {} warnings)", 
                  total_issues, validation.errors.len(), validation.warnings.len());
        }

        info!("");
    }

    /// Display server information
    fn display_server_information(info: &StartupAdditionalInfo) {
        info!("🌐 Server Information:");
        info!("   HTTP: http://{}:{}", info.host, info.port);
        info!("   WebSocket: ws://{}:{}/mcp/ws", info.host, info.port);
        
        if let Some(ref dashboard_url) = info.dashboard_url {
            info!("   Web Dashboard: {}", dashboard_url);
        }
        
        info!("");
    }
}

/// Additional startup information for display
pub struct StartupAdditionalInfo {
    pub host: String,
    pub port: u16,
    pub dashboard_url: Option<String>,
    pub tools_loaded: usize,
    pub llm_providers: Vec<String>,
}

impl StartupAdditionalInfo {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            dashboard_url: None,
            tools_loaded: 0,
            llm_providers: Vec::new(),
        }
    }

    pub fn with_dashboard_url(mut self, url: String) -> Self {
        self.dashboard_url = Some(url);
        self
    }

    pub fn with_tools_loaded(mut self, count: usize) -> Self {
        self.tools_loaded = count;
        self
    }

    pub fn with_llm_providers(mut self, providers: Vec<String>) -> Self {
        self.llm_providers = providers;
        self
    }
}

/// Display startup banner with version information
pub fn display_startup_banner(version: &str) {
    info!("");
    info!("╔══════════════════════════════════════════════════════════════╗");
    info!("║                        MagicTunnel v{}                        ║", version);
    info!("║              Intelligent MCP Proxy & Discovery               ║");
    info!("╚══════════════════════════════════════════════════════════════╝");
    info!("");
}