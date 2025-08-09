//! Environment variable integration for MagicTunnel configuration

use crate::config::{RuntimeMode, Config};
use crate::error::Result;
use std::env;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Environment variable names used by MagicTunnel
pub struct EnvVars;

impl EnvVars {
    pub const RUNTIME_MODE: &'static str = "MAGICTUNNEL_RUNTIME_MODE";
    pub const CONFIG_PATH: &'static str = "MAGICTUNNEL_CONFIG_PATH";
    pub const SMART_DISCOVERY: &'static str = "MAGICTUNNEL_SMART_DISCOVERY";
}

/// Environment configuration overrides
#[derive(Debug, Clone, Default)]
pub struct EnvironmentOverrides {
    /// Runtime mode override
    pub runtime_mode: Option<RuntimeMode>,
    /// Config file path override
    pub config_path: Option<PathBuf>,
    /// Smart discovery override
    pub smart_discovery: Option<bool>,
}

impl EnvironmentOverrides {
    /// Load environment variable overrides
    pub fn load() -> Result<Self> {
        let mut overrides = EnvironmentOverrides::default();

        // Load runtime mode
        if let Ok(mode_str) = env::var(EnvVars::RUNTIME_MODE) {
            match mode_str.parse::<RuntimeMode>() {
                Ok(mode) => {
                    debug!("Environment override: {}={}", EnvVars::RUNTIME_MODE, mode);
                    overrides.runtime_mode = Some(mode);
                }
                Err(e) => {
                    warn!("Invalid {}: {} ({})", EnvVars::RUNTIME_MODE, mode_str, e);
                    return Err(crate::error::ProxyError::config(format!(
                        "Invalid {}: {} (valid options: proxy, advanced)",
                        EnvVars::RUNTIME_MODE,
                        mode_str
                    )));
                }
            }
        }

        // Load config path
        if let Ok(path_str) = env::var(EnvVars::CONFIG_PATH) {
            let path = PathBuf::from(path_str);
            debug!("Environment override: {}={:?}", EnvVars::CONFIG_PATH, path);
            overrides.config_path = Some(path);
        }

        // Load smart discovery
        if let Ok(discovery_str) = env::var(EnvVars::SMART_DISCOVERY) {
            match discovery_str.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => {
                    debug!("Environment override: {}=true", EnvVars::SMART_DISCOVERY);
                    overrides.smart_discovery = Some(true);
                }
                "false" | "0" | "no" | "off" => {
                    debug!("Environment override: {}=false", EnvVars::SMART_DISCOVERY);
                    overrides.smart_discovery = Some(false);
                }
                _ => {
                    warn!("Invalid {}: {} (expected: true/false)", EnvVars::SMART_DISCOVERY, discovery_str);
                    return Err(crate::error::ProxyError::config(format!(
                        "Invalid {}: {} (valid options: true, false)",
                        EnvVars::SMART_DISCOVERY,
                        discovery_str
                    )));
                }
            }
        }

        Ok(overrides)
    }

    /// Apply environment overrides to a config
    pub fn apply_to_config(&self, config: &mut Config) {
        // Apply runtime mode override
        if let Some(ref runtime_mode) = self.runtime_mode {
            if config.deployment.is_none() {
                config.deployment = Some(crate::config::DeploymentConfig::default());
            }
            if let Some(ref mut deployment) = config.deployment {
                let original_mode = &deployment.runtime_mode;
                if original_mode != runtime_mode {
                    info!(
                        "Environment override: runtime_mode changed from {} to {}",
                        original_mode, runtime_mode
                    );
                }
                deployment.runtime_mode = runtime_mode.clone();
            }
        }

        // Apply smart discovery override
        if let Some(smart_discovery_enabled) = self.smart_discovery {
            if config.smart_discovery.is_none() {
                config.smart_discovery = Some(crate::discovery::SmartDiscoveryConfig::default());
            }
            if let Some(ref mut smart_discovery) = config.smart_discovery {
                if smart_discovery.enabled != smart_discovery_enabled {
                    info!(
                        "Environment override: smart_discovery.enabled changed from {} to {}",
                        smart_discovery.enabled, smart_discovery_enabled
                    );
                }
                smart_discovery.enabled = smart_discovery_enabled;
            }
        }
    }

    /// Get the effective config file path (with environment override)
    pub fn get_config_path(&self, default_path: &std::path::Path) -> PathBuf {
        self.config_path
            .clone()
            .unwrap_or_else(|| default_path.to_path_buf())
    }

    /// Check if any environment overrides are active
    pub fn has_overrides(&self) -> bool {
        self.runtime_mode.is_some() || self.config_path.is_some() || self.smart_discovery.is_some()
    }

    /// Get summary of active overrides for logging
    pub fn get_override_summary(&self) -> Vec<String> {
        let mut summary = Vec::new();

        if let Some(ref mode) = self.runtime_mode {
            summary.push(format!("{}={}", EnvVars::RUNTIME_MODE, mode));
        }

        if let Some(ref path) = self.config_path {
            summary.push(format!("{}={:?}", EnvVars::CONFIG_PATH, path));
        }

        if let Some(discovery) = self.smart_discovery {
            summary.push(format!("{}={}", EnvVars::SMART_DISCOVERY, discovery));
        }

        summary
    }
}