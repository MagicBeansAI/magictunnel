//! Configuration validation system with mode-specific validation

use crate::config::{Config, RuntimeMode};
use crate::error::Result;
use std::collections::HashMap;
use tracing::{debug, info};

/// Configuration validator with mode-specific validation rules
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate configuration for proxy mode
    /// 
    /// Proxy mode requirements:
    /// - Basic server configuration (host, port)
    /// - Registry configuration for tool loading
    /// - Optional smart discovery (but not required)
    /// - Should NOT have advanced features enabled
    pub fn validate_proxy_mode(config: &Config) -> Result<ValidationResult> {
        debug!("Validating proxy mode configuration");
        
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();
        
        // Basic server validation
        if config.server.host.is_empty() {
            errors.push("Server host cannot be empty".to_string());
            suggestions.push("Set server.host to '0.0.0.0' for all interfaces or '127.0.0.1' for localhost only".to_string());
        }
        
        if config.server.port == 0 {
            errors.push("Server port cannot be 0".to_string());
            suggestions.push("Set server.port to a valid port number (e.g., 3001)".to_string());
        }
        
        if config.server.port < 1024 && config.server.port != 0 {
            warnings.push("Server port is in reserved range (<1024), may require root privileges".to_string());
        }
        
        // Registry validation
        if config.registry.paths.is_empty() {
            warnings.push("No capability paths specified, will use default 'capabilities/'".to_string());
        }
        
        // Check for advanced features that shouldn't be enabled in proxy mode
        if config.auth.is_some() {
            warnings.push("Authentication configuration found in proxy mode - will be ignored".to_string());
            suggestions.push("Remove auth configuration or switch to advanced mode to enable authentication".to_string());
        }
        
        if config.security.is_some() {
            warnings.push("Security configuration found in proxy mode - will be ignored".to_string());
            suggestions.push("Remove security configuration or switch to advanced mode to enable security features".to_string());
        }
        
        // Smart discovery validation (optional in proxy mode)
        if let Some(ref smart_discovery) = config.smart_discovery {
            if smart_discovery.enabled {
                info!("Smart discovery enabled in proxy mode");
                
                // Validate smart discovery configuration
                if smart_discovery.tool_selection_mode == "llm_based" && 
                   !smart_discovery.llm_mapper.enabled {
                    warnings.push("Smart discovery using LLM mode but LLM mapper is disabled".to_string());
                    suggestions.push("Enable smart_discovery.llm_mapper for LLM-based tool discovery".to_string());
                }
            }
        }
        
        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
            mode: RuntimeMode::Proxy,
        })
    }
    
    /// Validate configuration for advanced mode
    /// 
    /// Advanced mode requirements:
    /// - All proxy mode requirements
    /// - Security configuration for enterprise features
    /// - Authentication configuration recommended
    /// - Advanced service configurations should be properly structured
    pub fn validate_advanced_mode(config: &Config) -> Result<ValidationResult> {
        debug!("Validating advanced mode configuration");
        
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();
        
        // Start with proxy mode validation
        let proxy_result = Self::validate_proxy_mode(config)?;
        errors.extend(proxy_result.errors);
        warnings.extend(proxy_result.warnings);
        suggestions.extend(proxy_result.suggestions);
        
        // Advanced mode specific validation
        
        // Security configuration validation
        if config.security.is_none() {
            warnings.push("No security configuration found in advanced mode".to_string());
            suggestions.push("Add security configuration to enable enterprise security features".to_string());
        } else if let Some(ref security) = config.security {
            // Validate security configuration structure
            if security.policies.is_none() && security.allowlist.is_none() {
                warnings.push("Security configuration present but no policies or allowlist defined".to_string());
                suggestions.push("Define security policies or allowlist to enable security enforcement".to_string());
            }
        }
        
        // Authentication configuration validation
        if config.auth.is_none() {
            warnings.push("No authentication configuration found in advanced mode".to_string());
            suggestions.push("Add auth configuration to enable user authentication and authorization".to_string());
        } else if let Some(ref auth) = config.auth {
            // Validate auth configuration
            if auth.api_keys.is_none() && auth.oauth.is_none() {
                warnings.push("Authentication enabled but no auth methods configured".to_string());
                suggestions.push("Configure either API keys or OAuth for authentication".to_string());
            }
            
            // OAuth validation
            if let Some(ref oauth) = auth.oauth {
                if oauth.client_id.is_empty() {
                    errors.push("OAuth client_id cannot be empty".to_string());
                }
                if oauth.client_secret.is_empty() {
                    errors.push("OAuth client_secret cannot be empty".to_string());
                }
                if oauth.auth_url.is_empty() {
                    errors.push("OAuth auth_url cannot be empty".to_string());
                }
                if oauth.token_url.is_empty() {
                    errors.push("OAuth token_url cannot be empty".to_string());
                }
            }
        }
        
        // Advanced service configuration validation
        
        // Note: Audit configuration not yet implemented in Config struct
        // This validation will be enabled when audit features are added
        
        // External MCP validation for advanced features
        if let Some(ref external_mcp) = config.external_mcp {
            if external_mcp.config_file.is_empty() {
                warnings.push("External MCP configuration present but no config file specified".to_string());
                suggestions.push("Define external_mcp.config_file to enable external integrations".to_string());
            }
        }
        
        // Note: Performance configuration not yet implemented in Config struct
        // This validation will be enabled when performance features are added
        
        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
            mode: RuntimeMode::Advanced,
        })
    }
    
    /// Generate missing configuration defaults based on runtime mode
    pub fn generate_missing_defaults(config: &mut Config, mode: &RuntimeMode) -> Result<ConfigUpdateSummary> {
        debug!("Generating missing defaults for {:?} mode", mode);
        
        let mut updates = Vec::new();
        
        // Always ensure basic server defaults
        if config.server.host.is_empty() {
            config.server.host = "127.0.0.1".to_string();
            updates.push("Set server.host to '127.0.0.1'".to_string());
        }
        
        if config.server.port == 0 {
            config.server.port = 3001;
            updates.push("Set server.port to 3001".to_string());
        }
        
        // Registry defaults
        if config.registry.paths.is_empty() {
            config.registry.paths = vec!["capabilities".to_string()];
            updates.push("Set registry.paths to ['capabilities']".to_string());
        }
        
        // Mode-specific defaults
        match mode {
            RuntimeMode::Proxy => {
                // Proxy mode: minimal configuration
                if config.smart_discovery.is_none() {
                    config.smart_discovery = Some(crate::discovery::SmartDiscoveryConfig {
                        enabled: false,
                        ..Default::default()
                    });
                    updates.push("Created default smart_discovery configuration (disabled)".to_string());
                }
            }
            RuntimeMode::Advanced => {
                // Advanced mode: more comprehensive defaults
                if config.security.is_none() {
                    config.security = Some(crate::security::SecurityConfig::default());
                    updates.push("Created default security configuration".to_string());
                }
                
                // Note: Config doesn't have audit or performance fields yet
                // These will be added when those features are implemented
            }
        }
        
        Ok(ConfigUpdateSummary {
            updates_applied: updates.len(),
            updates,
            mode: mode.clone(),
        })
    }
    
    /// Suggest configuration fixes based on validation results
    pub fn suggest_config_fixes(validation: &ValidationResult) -> ConfigFixSuggestions {
        let mut fixes = HashMap::new();
        let mut quick_fixes = Vec::new();
        
        // Analyze errors and provide specific fixes
        for error in &validation.errors {
            if error.contains("host cannot be empty") {
                fixes.insert(
                    "server.host".to_string(),
                    "Set to '127.0.0.1' for localhost or '0.0.0.0' for all interfaces".to_string()
                );
                quick_fixes.push(QuickFix {
                    field: "server.host".to_string(),
                    suggested_value: "\"127.0.0.1\"".to_string(),
                    description: "Bind to localhost only".to_string(),
                });
            }
            
            if error.contains("port cannot be 0") {
                fixes.insert(
                    "server.port".to_string(),
                    "Set to a valid port number (e.g., 3001)".to_string()
                );
                quick_fixes.push(QuickFix {
                    field: "server.port".to_string(),
                    suggested_value: "3001".to_string(),
                    description: "Standard MCP proxy port".to_string(),
                });
            }
            
            if error.contains("OAuth") && error.contains("empty") {
                if error.contains("client_id") {
                    fixes.insert(
                        "auth.oauth.client_id".to_string(),
                        "Set your OAuth client ID from your provider".to_string()
                    );
                }
                if error.contains("client_secret") {
                    fixes.insert(
                        "auth.oauth.client_secret".to_string(),
                        "Set your OAuth client secret from your provider".to_string()
                    );
                }
                if error.contains("auth_url") {
                    fixes.insert(
                        "auth.oauth.auth_url".to_string(),
                        "Set the authorization URL from your OAuth provider".to_string()
                    );
                }
                if error.contains("token_url") {
                    fixes.insert(
                        "auth.oauth.token_url".to_string(),
                        "Set the token URL from your OAuth provider".to_string()
                    );
                }
            }
            
            // Note: audit.storage_path validation will be enabled when audit features are added
        }
        
        // Add suggestions for common improvements
        if validation.mode == RuntimeMode::Advanced {
            if !validation.warnings.iter().any(|w| w.contains("security")) {
                quick_fixes.push(QuickFix {
                    field: "security".to_string(),
                    suggested_value: "{}".to_string(),
                    description: "Enable basic security configuration".to_string(),
                });
            }
        }
        
        ConfigFixSuggestions {
            field_fixes: fixes,
            quick_fixes,
            mode_suggestions: Self::generate_mode_suggestions(&validation.mode),
        }
    }
    
    /// Generate mode-specific configuration suggestions
    fn generate_mode_suggestions(mode: &RuntimeMode) -> Vec<String> {
        match mode {
            RuntimeMode::Proxy => vec![
                "Proxy mode provides core MCP proxy functionality with minimal configuration".to_string(),
                "Enable smart_discovery for intelligent tool discovery".to_string(),
                "Use environment variables like MAGICTUNNEL_SMART_DISCOVERY=true for quick testing".to_string(),
            ],
            RuntimeMode::Advanced => vec![
                "Advanced mode provides enterprise security and management features".to_string(),
                "Configure authentication (auth.api_keys or auth.oauth2) for secure access".to_string(),
                "Set up security policies (security.policies) to control tool access".to_string(),
                "Consider external_mcp configuration for integrating with other MCP servers".to_string(),
            ],
        }
    }
}

/// Configuration validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the configuration is valid
    pub is_valid: bool,
    /// Critical errors that prevent startup
    pub errors: Vec<String>,
    /// Non-critical warnings
    pub warnings: Vec<String>,
    /// Helpful suggestions for improvement
    pub suggestions: Vec<String>,
    /// Runtime mode being validated
    pub mode: RuntimeMode,
}

impl ValidationResult {
    /// Check if configuration has any issues (errors or warnings)
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }
    
    /// Get total issue count
    pub fn issue_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
    
    /// Check if configuration can start (no critical errors)
    pub fn can_start(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Configuration update summary after applying defaults
#[derive(Debug)]
pub struct ConfigUpdateSummary {
    /// Number of updates applied
    pub updates_applied: usize,
    /// List of updates that were made
    pub updates: Vec<String>,
    /// Runtime mode for which defaults were generated
    pub mode: RuntimeMode,
}

/// Configuration fix suggestions
#[derive(Debug)]
pub struct ConfigFixSuggestions {
    /// Field-specific fix descriptions
    pub field_fixes: HashMap<String, String>,
    /// Quick-fix suggestions with specific values
    pub quick_fixes: Vec<QuickFix>,
    /// Mode-specific general suggestions
    pub mode_suggestions: Vec<String>,
}

/// Quick fix suggestion with specific value
#[derive(Debug)]
pub struct QuickFix {
    /// Configuration field path (e.g., "server.host")
    pub field: String,
    /// Suggested value in YAML format
    pub suggested_value: String,
    /// Human-readable description
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ServerConfig, RegistryConfig};

    #[test]
    fn test_proxy_mode_validation_valid() {
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
            ..Default::default()
        };
        
        let result = ConfigValidator::validate_proxy_mode(&config).unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }
    
    #[test]
    fn test_proxy_mode_validation_invalid() {
        let config = Config {
            server: ServerConfig {
                host: "".to_string(),  // Invalid: empty host
                port: 0,              // Invalid: zero port
                ..Default::default()
            },
            ..Default::default()
        };
        
        let result = ConfigValidator::validate_proxy_mode(&config).unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.len() >= 2); // Should have host and port errors
        assert!(result.suggestions.len() >= 2); // Should have suggestions
    }
    
    #[test]
    fn test_advanced_mode_validation() {
        let mut config = Config::default();
        config.server.host = "127.0.0.1".to_string();
        config.server.port = 3001;
        
        let result = ConfigValidator::validate_advanced_mode(&config).unwrap();
        assert!(result.can_start()); // Should be able to start despite warnings
        assert!(!result.warnings.is_empty()); // Should have warnings about missing advanced features
    }
    
    #[test]
    fn test_generate_missing_defaults_proxy() {
        let mut config = Config::default();
        
        let summary = ConfigValidator::generate_missing_defaults(&mut config, &RuntimeMode::Proxy).unwrap();
        
        assert!(summary.updates_applied > 0);
        assert!(!config.server.host.is_empty());
        assert!(config.server.port > 0);
    }
    
    #[test]
    fn test_generate_missing_defaults_advanced() {
        let mut config = Config::default();
        
        let summary = ConfigValidator::generate_missing_defaults(&mut config, &RuntimeMode::Advanced).unwrap();
        
        assert!(summary.updates_applied > 0);
        assert!(config.security.is_some());
        // Note: audit configuration not yet implemented in Config struct
    }
    
    #[test]
    fn test_suggest_config_fixes() {
        let validation = ValidationResult {
            is_valid: false,
            errors: vec![
                "Server host cannot be empty".to_string(),
                "Server port cannot be 0".to_string(),
            ],
            warnings: vec![],
            suggestions: vec![],
            mode: RuntimeMode::Proxy,
        };
        
        let suggestions = ConfigValidator::suggest_config_fixes(&validation);
        
        assert!(!suggestions.field_fixes.is_empty());
        assert!(!suggestions.quick_fixes.is_empty());
        assert!(!suggestions.mode_suggestions.is_empty());
    }
}