//! Configuration resolution with priority system

use crate::config::{Config, DeploymentConfig, RuntimeMode};
use crate::config::environment::EnvironmentOverrides;
use crate::config::validator::{ConfigValidator, ValidationResult};
use crate::error::{ProxyError, Result};
use std::path::{Path, PathBuf};
use tracing::{info, debug};

/// Configuration file resolution priority
const CONFIG_FILE_PRIORITIES: &[&str] = &[
    "magictunnel-config.yaml", // New default (highest priority)
    "config.yaml",             // Legacy fallback
];

/// Configuration resolver with priority system
pub struct ConfigResolver {
    /// Environment overrides
    env_overrides: EnvironmentOverrides,
}

impl ConfigResolver {
    /// Create a new config resolver
    pub fn new() -> Result<Self> {
        let env_overrides = EnvironmentOverrides::load()?;
        Ok(Self { env_overrides })
    }

    /// Resolve configuration with full priority system
    pub fn resolve_config(&self, cli_config_path: Option<&Path>) -> Result<ConfigResolution> {
        // Step 1: Determine config file path
        let config_path = self.resolve_config_path(cli_config_path);
        
        // Step 2: Load base configuration
        let mut config = self.load_base_config(&config_path)?;
        
        // Step 3: Apply environment overrides
        self.env_overrides.apply_to_config(&mut config);
        
        // Step 4: Validate final configuration
        let validation_result = self.validate_final_config(&config)?;
        
        Ok(ConfigResolution {
            config,
            config_path: Some(config_path.clone()),
            env_overrides: self.env_overrides.clone(),
            config_source: self.determine_config_source(&config_path),
            validation_result,
        })
    }

    /// Resolve the config file path using priority system
    fn resolve_config_path(&self, cli_path: Option<&Path>) -> PathBuf {
        // Priority 1: CLI argument
        if let Some(cli_path) = cli_path {
            debug!("Using CLI-specified config path: {:?}", cli_path);
            return cli_path.to_path_buf();
        }

        // Priority 2: Environment variable
        if let Some(ref env_path) = self.env_overrides.config_path {
            debug!("Using environment config path: {:?}", env_path);
            return env_path.clone();
        }

        // Priority 3: Check default files in order
        for &filename in CONFIG_FILE_PRIORITIES {
            let path = PathBuf::from(filename);
            if path.exists() {
                debug!("Found config file: {:?}", path);
                return path;
            }
        }

        // Priority 4: Default to new standard name (even if it doesn't exist)
        let default_path = PathBuf::from(CONFIG_FILE_PRIORITIES[0]);
        debug!("Using default config path: {:?}", default_path);
        default_path
    }

    /// Load base configuration from file or defaults
    fn load_base_config(&self, config_path: &Path) -> Result<Config> {
        if config_path.exists() {
            info!("Loading configuration from: {:?}", config_path);
            let content = std::fs::read_to_string(config_path)
                .map_err(|e| ProxyError::config(format!("Failed to read config file {:?}: {}", config_path, e)))?;

            serde_yaml::from_str(&content)
                .map_err(|e| ProxyError::config(format!("Failed to parse config file {:?}: {}", config_path, e)))
        } else {
            info!("Config file not found, using built-in proxy mode defaults");
            Ok(Self::create_proxy_mode_defaults())
        }
    }

    /// Create built-in proxy mode defaults
    fn create_proxy_mode_defaults() -> Config {
        let mut config = Config::default();
        
        // Ensure deployment config exists with proxy mode
        config.deployment = Some(DeploymentConfig {
            runtime_mode: RuntimeMode::Proxy,
        });

        // Set smart discovery to disabled by default (can be overridden)
        if let Some(ref mut smart_discovery) = config.smart_discovery {
            smart_discovery.enabled = false;
        }

        config
    }

    /// Validate the final resolved configuration
    fn validate_final_config(&self, config: &Config) -> Result<ValidationResult> {
        // Get runtime mode (should exist after resolution)
        let runtime_mode = config.deployment
            .as_ref()
            .map(|d| &d.runtime_mode)
            .unwrap_or(&RuntimeMode::Proxy);

        // Use the appropriate validator based on runtime mode
        let validation_result = match runtime_mode {
            RuntimeMode::Proxy => ConfigValidator::validate_proxy_mode(config)?,
            RuntimeMode::Advanced => ConfigValidator::validate_advanced_mode(config)?,
        };

        // Log validation results
        if !validation_result.can_start() {
            for error in &validation_result.errors {
                tracing::error!("Configuration error: {}", error);
            }
            return Err(ProxyError::config(format!(
                "Configuration validation failed with {} errors", 
                validation_result.errors.len()
            )));
        }

        // Log warnings but don't fail startup
        for warning in &validation_result.warnings {
            tracing::warn!("Configuration warning: {}", warning);
        }

        // Log suggestions for improvement
        for suggestion in &validation_result.suggestions {
            debug!("Configuration suggestion: {}", suggestion);
        }

        Ok(validation_result)
    }

    /// Determine the configuration source for logging
    fn determine_config_source(&self, config_path: &Path) -> ConfigSource {
        if !config_path.exists() {
            return ConfigSource::Defaults;
        }

        let filename = config_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        match filename {
            "magictunnel-config.yaml" => ConfigSource::MagicTunnelConfig,
            "config.yaml" => ConfigSource::LegacyConfig,
            _ => ConfigSource::Custom(config_path.to_path_buf()),
        }
    }
}

/// Complete configuration resolution result
#[derive(Debug, Clone)]
pub struct ConfigResolution {
    /// Final resolved configuration
    pub config: Config,
    /// Config file path that was used (None if using pure defaults)
    pub config_path: Option<PathBuf>,
    /// Environment overrides that were applied
    pub env_overrides: EnvironmentOverrides,
    /// Source of the configuration
    pub config_source: ConfigSource,
    /// Configuration validation result
    pub validation_result: ValidationResult,
}

impl ConfigResolution {
    /// Get the effective runtime mode
    pub fn get_runtime_mode(&self) -> &RuntimeMode {
        self.config.deployment
            .as_ref()
            .map(|d| &d.runtime_mode)
            .unwrap_or(&RuntimeMode::Proxy)
    }

    /// Check if smart discovery is enabled
    pub fn is_smart_discovery_enabled(&self) -> bool {
        self.config.smart_discovery
            .as_ref()
            .map(|sd| sd.enabled)
            .unwrap_or(false)
    }

    /// Get configuration summary for startup logging
    pub fn get_startup_summary(&self) -> ConfigStartupSummary {
        ConfigStartupSummary {
            config_path: self.config_path.clone(),
            runtime_mode: self.get_runtime_mode().clone(),
            smart_discovery_enabled: self.is_smart_discovery_enabled(),
            has_env_overrides: self.env_overrides.has_overrides(),
            env_override_summary: self.env_overrides.get_override_summary(),
            config_source: self.config_source.clone(),
        }
    }
}

/// Configuration source identification
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Built-in defaults (no config file found)
    Defaults,
    /// magictunnel-config.yaml (new standard)
    MagicTunnelConfig,
    /// config.yaml (legacy)
    LegacyConfig,
    /// Custom path (CLI or env var)
    Custom(PathBuf),
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigSource::Defaults => write!(f, "built-in defaults"),
            ConfigSource::MagicTunnelConfig => write!(f, "magictunnel-config.yaml"),
            ConfigSource::LegacyConfig => write!(f, "config.yaml"),
            ConfigSource::Custom(path) => write!(f, "custom: {:?}", path),
        }
    }
}

/// Configuration startup summary for logging
#[derive(Debug)]
pub struct ConfigStartupSummary {
    pub config_path: Option<PathBuf>,
    pub runtime_mode: RuntimeMode,
    pub smart_discovery_enabled: bool,
    pub has_env_overrides: bool,
    pub env_override_summary: Vec<String>,
    pub config_source: ConfigSource,
}