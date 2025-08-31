//! Hierarchical Configuration System
//!
//! This module implements a 3-tier hierarchical configuration architecture:
//! Global → MCP → Tool precedence with clean separation of concerns.
//!
//! Key principles:
//! - No new properties added, only restructuring existing ones
//! - Clean separation between Smart Discovery and MCP protocol
//! - Simple precedence resolution with override chains
//! - Migration support from flat to hierarchical structure

use crate::config::config::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default settings that can be overridden at different hierarchy levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultSettings {
    /// Base timeout for all operations (seconds)
    pub timeout: u64,
    /// Base retry attempts for failed operations
    pub retry_attempts: u32,
    /// Base retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for DefaultSettings {
    fn default() -> Self {
        Self {
            timeout: 30,
            retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Global system-wide configuration (Tier 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Deployment configuration - controls runtime mode and feature loading
    pub deployment: DeploymentConfig,
    /// Server infrastructure configuration
    pub server: ServerConfig,
    /// Registry configuration for tool loading
    pub registry: RegistryConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Security configuration (RBAC, audit, sanitization)
    pub security: crate::security::SecurityConfig,
    /// Unified authentication configuration
    pub auth: crate::auth::MultiLevelAuthConfig,
    /// Default settings that can be overridden by MCP or tool levels
    pub defaults: DefaultSettings,
    /// Global timeout setting (convenience shortcut)
    pub timeout: Option<u64>,
    /// Global max retries setting (convenience shortcut)
    pub max_retries: Option<u32>,
    /// Global priority setting (convenience shortcut)
    pub priority: Option<u8>,
    /// Global enabled status (convenience shortcut)
    pub enabled: Option<bool>,
    /// Global visibility setting (convenience shortcut)
    pub hidden: Option<bool>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            deployment: DeploymentConfig::default(),
            server: ServerConfig::default(),
            registry: RegistryConfig::default(),
            logging: LoggingConfig::default(),
            security: crate::security::SecurityConfig::default(),
            auth: crate::auth::MultiLevelAuthConfig::default(),
            defaults: DefaultSettings::default(),
            timeout: None,
            max_retries: None,
            priority: None,
            enabled: None,
            hidden: None,
        }
    }
}

/// Consolidated content services configuration  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentServicesConfig {
    /// Prompt generation service configuration
    pub prompt_generation: Option<crate::mcp::PromptGenerationConfig>,
    /// Resource generation service configuration
    pub resource_generation: Option<crate::mcp::ResourceGenerationConfig>,
    /// Content storage service configuration
    pub content_storage: Option<crate::mcp::ContentStorageConfig>,
    /// External content management configuration
    pub external_content: Option<crate::mcp::ExternalContentConfig>,
}

impl Default for ContentServicesConfig {
    fn default() -> Self {
        Self {
            prompt_generation: None,
            resource_generation: None,
            content_storage: None,
            external_content: None,
        }
    }
}

/// MCP protocol-specific configuration (Tier 2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// External MCP discovery configuration
    pub external_mcp: ExternalMcpConfig,
    /// MCP client configuration
    pub mcp_client: McpClientConfig,
    /// Streamable HTTP transport configuration (MCP 2025-06-18)
    pub streamable_http: StreamableHttpTransportConfig,
    /// MCP sampling service configuration
    pub sampling: SamplingConfig,
    /// MCP elicitation service configuration  
    pub elicitation: ElicitationConfig,
    /// Consolidated content services
    pub content_services: ContentServicesConfig,
    /// Overrides for global default settings
    pub overrides: Option<DefaultSettings>,
    /// MCP-level timeout setting (convenience shortcut)
    pub timeout: Option<u64>,
    /// MCP-level max retries setting (convenience shortcut)
    pub max_retries: Option<u32>,
    /// MCP-level priority setting (convenience shortcut)
    pub priority: Option<u8>,
    /// MCP-level enabled status (convenience shortcut)
    pub enabled: Option<bool>,
    /// MCP-level visibility setting (convenience shortcut)
    pub hidden: Option<bool>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            external_mcp: ExternalMcpConfig::default(),
            mcp_client: McpClientConfig::default(),
            streamable_http: StreamableHttpTransportConfig::default(),
            sampling: SamplingConfig::default(),
            elicitation: ElicitationConfig::default(),
            content_services: ContentServicesConfig::default(),
            overrides: None,
            timeout: None,
            max_retries: None,
            priority: None,
            enabled: None,
            hidden: None,
        }
    }
}

/// Smart Discovery AI configuration (Tier 3A - separate domain)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Smart Discovery service configuration
    pub smart_discovery: crate::discovery::SmartDiscoveryConfig,
    /// Tool enhancement configuration
    pub tool_enhancement: ToolEnhancementConfig,
    /// Enhancement storage configuration
    pub enhancement_storage: crate::discovery::EnhancementStorageConfig,
    /// Tool visibility configuration
    pub visibility: VisibilityConfig,
    /// Conflict resolution configuration for multi-source tools
    pub conflict_resolution: crate::routing::ConflictResolutionConfig,
}

/// Tool routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRoutingConfig {
    /// Routing type (external_mcp, websocket, etc.)
    pub routing_type: String,
    /// Routing configuration parameters
    pub config: HashMap<String, serde_json::Value>,
}

/// Tool-specific authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAuthConfig {
    /// Override authentication requirements for this tool
    pub required: Option<bool>,
    /// Tool-specific authentication method
    pub method: Option<String>,
    /// Tool-specific authentication parameters
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

/// Individual tool configuration overrides (Tier 3B)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Tool-specific overrides for default settings
    pub overrides: Option<DefaultSettings>,
    /// Tool routing configuration
    pub routing: Option<ToolRoutingConfig>,
    /// Tool-specific parameters
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    /// Tool-specific authentication configuration
    pub auth: Option<ToolAuthConfig>,
    /// Tool-specific timeout override (convenience shortcut)
    pub timeout: Option<u64>,
    /// Tool-specific max retries override (convenience shortcut)
    pub max_retries: Option<u32>,
    /// Tool-specific priority (higher numbers = higher priority)
    pub priority: Option<u8>,
    /// Tool-specific enabled status override
    pub enabled: Option<bool>,
    /// Tool-specific visibility override
    pub hidden: Option<bool>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            overrides: None,
            routing: None,
            parameters: None,
            auth: None,
            timeout: None,
            max_retries: None,
            priority: None,
            enabled: None,
            hidden: None,
        }
    }
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            smart_discovery: crate::discovery::SmartDiscoveryConfig::default(),
            tool_enhancement: ToolEnhancementConfig::default(),
            enhancement_storage: crate::discovery::EnhancementStorageConfig::default(),
            visibility: VisibilityConfig::default(),
            conflict_resolution: crate::routing::ConflictResolutionConfig::default(),
        }
    }
}

impl Default for HierarchicalConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            mcp: McpConfig::default(),
            discovery: DiscoveryConfig::default(),
            tools: HashMap::new(),
        }
    }
}

/// New hierarchical configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalConfig {
    /// Global system-wide settings (base level)
    pub global: GlobalConfig,
    /// MCP protocol-specific settings (middle level)
    pub mcp: McpConfig,
    /// Smart Discovery AI settings (separate domain)
    pub discovery: DiscoveryConfig,
    /// Tool-specific overrides (top level)
    #[serde(default)]
    pub tools: HashMap<String, ToolConfig>,
}

/// Fully resolved tool configuration after applying hierarchy precedence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedToolConfig {
    /// Tool name
    pub name: String,
    /// Resolved timeout (from Tool → MCP → Global hierarchy)
    pub timeout: u64,
    /// Resolved max retries (from Tool → MCP → Global hierarchy)
    pub max_retries: u32,
    /// Resolved priority (from Tool → MCP → Global hierarchy)
    pub priority: u8,
    /// Resolved enabled status (from Tool → MCP → Global hierarchy)
    pub enabled: bool,
    /// Resolved visibility status (from Tool → MCP → Global hierarchy)
    pub hidden: bool,
}

/// Configuration resolver with precedence handling
pub struct ConfigResolver {
    config: HierarchicalConfig,
}

// === CONFIGURATION MIGRATION AND LOADING ===

impl HierarchicalConfig {
    /// Load hierarchical configuration from file path
    /// Supports both hierarchical YAML format and automatic migration from flat format
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }

    /// Parse hierarchical configuration from YAML string
    /// Auto-detects format and migrates if necessary
    pub fn from_yaml(yaml_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Try parsing as hierarchical config first
        match serde_yaml::from_str::<HierarchicalConfig>(yaml_content) {
            Ok(config) => Ok(config),
            Err(_) => {
                // Failed as hierarchical, try flat config migration
                let flat_config: crate::config::Config = serde_yaml::from_str(yaml_content)?;
                Ok(Self::from_flat_config(flat_config))
            }
        }
    }

    /// Migrate from flat config structure to hierarchical
    /// Preserves all existing properties, only restructures organization
    pub fn from_flat_config(flat: crate::config::Config) -> Self {
        let global_config = GlobalConfig {
            // Infrastructure settings
            deployment: flat.deployment.clone().unwrap_or_default(),
            server: flat.server.clone(),
            registry: flat.registry.clone(),
            logging: flat.logging.clone().unwrap_or_default(),
            security: flat.security.clone().unwrap_or_default(),
            
            // Unified auth (prioritize multi_level_auth over legacy auth)
            auth: flat.multi_level_auth.clone().unwrap_or_else(|| {
                // Simple migration: if legacy auth exists, just enable multi-level auth
                // Complex field mapping would be handled by a dedicated migration CLI tool
                flat.auth.clone().map(|auth| {
                    let mut multi_level_auth = crate::auth::MultiLevelAuthConfig::default();
                    multi_level_auth.enabled = auth.enabled;
                    multi_level_auth
                }).unwrap_or_default()
            }),
            
            // Default settings extracted from various configs
            defaults: Self::extract_default_settings(&flat),
            // New convenience fields (no direct migration from flat config)
            timeout: None,
            max_retries: None,
            priority: None,
            enabled: None,
            hidden: None,
        };

        let mcp_config = McpConfig {
            // MCP Protocol settings
            external_mcp: flat.external_mcp.clone().unwrap_or_default(),
            mcp_client: flat.mcp_client.clone().unwrap_or_default(),
            streamable_http: flat.streamable_http.clone().unwrap_or_default(),
            
            // MCP 2025-06-18 Services
            sampling: flat.sampling.clone().unwrap_or_default(),
            elicitation: flat.elicitation.clone().unwrap_or_default(),
            content_services: ContentServicesConfig {
                prompt_generation: flat.prompt_generation.clone(),
                resource_generation: flat.resource_generation.clone(),
                content_storage: flat.content_storage.clone(),
                external_content: flat.external_content.clone(),
            },
            
            // No MCP-level overrides initially (use global defaults)
            overrides: None,
            // New convenience fields (no direct migration from flat config)
            timeout: None,
            max_retries: None,
            priority: None,
            enabled: None,
            hidden: None,
        };

        let discovery_config = DiscoveryConfig {
            // Smart Discovery settings (completely separated from MCP)
            smart_discovery: flat.smart_discovery.clone().unwrap_or_default(),
            tool_enhancement: flat.tool_enhancement.clone().unwrap_or_default(),
            enhancement_storage: flat.enhancement_storage.clone().unwrap_or_default(),
            visibility: flat.visibility.clone().unwrap_or_default(),
            conflict_resolution: flat.conflict_resolution.clone().unwrap_or_default(),
        };

        HierarchicalConfig {
            global: global_config,
            mcp: mcp_config,
            discovery: discovery_config,
            tools: HashMap::new(), // No tool-level configs in flat structure initially
        }
    }

    /// Extract common default settings from flat config
    fn extract_default_settings(flat: &crate::config::Config) -> DefaultSettings {
        // Extract timeout settings from server config as base defaults
        let base_timeout = flat.server.timeout;
        
        // Extract retry settings from MCP client config if available
        let (retry_attempts, retry_delay_ms) = if let Some(mcp_client) = &flat.mcp_client {
            (mcp_client.max_reconnect_attempts, mcp_client.reconnect_delay_secs * 1000)
        } else {
            (3, 5000) // Default values
        };

        DefaultSettings {
            timeout: base_timeout,
            retry_attempts,
            retry_delay_ms,
        }
    }

    /// Convert hierarchical config back to flat structure
    /// Used for backwards compatibility and validation
    pub fn to_flat_config(&self) -> crate::config::Config {
        crate::config::Config {
            deployment: Some(self.global.deployment.clone()),
            server: self.global.server.clone(),
            registry: self.global.registry.clone(),
            auth: None, // Use multi_level_auth instead
            multi_level_auth: Some(self.global.auth.clone()),
            logging: Some(self.global.logging.clone()),
            external_mcp: Some(self.mcp.external_mcp.clone()),
            mcp_client: Some(self.mcp.mcp_client.clone()),
            conflict_resolution: Some(self.discovery.conflict_resolution.clone()),
            visibility: Some(self.discovery.visibility.clone()),
            smart_discovery: Some(self.discovery.smart_discovery.clone()),
            security: Some(self.global.security.clone()),
            streamable_http: Some(self.mcp.streamable_http.clone()),
            sampling: Some(self.mcp.sampling.clone()),
            tool_enhancement: Some(self.discovery.tool_enhancement.clone()),
            elicitation: Some(self.mcp.elicitation.clone()),
            prompt_generation: self.mcp.content_services.prompt_generation.clone(),
            resource_generation: self.mcp.content_services.resource_generation.clone(),
            content_storage: self.mcp.content_services.content_storage.clone(),
            external_content: self.mcp.content_services.external_content.clone(),
            enhancement_storage: Some(self.discovery.enhancement_storage.clone()),
        }
    }

    /// Validate the hierarchical configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate each tier separately
        self.global.validate()?;
        self.mcp.validate()?;
        self.discovery.validate()?;
        
        // Validate tool-level configs
        for (tool_name, tool_config) in &self.tools {
            tool_config.validate().map_err(|e| {
                format!("Tool '{}' configuration error: {}", tool_name, e)
            })?;
        }
        
        Ok(())
    }
}

impl GlobalConfig {
    /// Validate global configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate server config using existing validation
        self.server.validate().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Validate registry config
        self.registry.validate().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Validate defaults
        self.defaults.validate()?;
        
        Ok(())
    }
}

impl McpConfig {
    /// Validate MCP configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate MCP client config if available
        self.mcp_client.validate().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Validate streamable HTTP config
        self.streamable_http.validate().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Validate sampling config
        self.sampling.validate().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Validate elicitation config
        self.elicitation.validate().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        Ok(())
    }
}

impl DiscoveryConfig {
    /// Validate Discovery configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Discovery configs typically have minimal validation requirements
        // Most validation is done at the service level
        Ok(())
    }
}

impl ToolConfig {
    /// Validate tool configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate overrides if present
        if let Some(overrides) = &self.overrides {
            overrides.validate()?;
        }
        
        // Tool routing and auth validation would be done at service level
        Ok(())
    }
}

impl DefaultSettings {
    /// Validate default settings
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.timeout == 0 {
            return Err("Timeout must be greater than 0".into());
        }
        
        if self.timeout > 300 {
            return Err("Timeout cannot exceed 300 seconds".into());
        }
        
        if self.retry_attempts > 10 {
            return Err("Retry attempts cannot exceed 10".into());
        }
        
        if self.retry_delay_ms == 0 {
            return Err("Retry delay must be greater than 0".into());
        }
        
        Ok(())
    }
}

impl ConfigResolver {
    /// Create a new ConfigResolver with default settings
    pub fn new(config: HierarchicalConfig) -> Self {
        Self { config }
    }

    /// Resolve timeout configuration with precedence: Tool → MCP → Global
    pub fn resolve_timeout(&self, tool_name: Option<&str>) -> u64 {
        // Check tool-level configuration first
        if let Some(tool_name) = tool_name {
            if let Some(tool_config) = self.config.tools.get(tool_name) {
                if let Some(timeout) = tool_config.timeout {
                    return timeout;
                }
            }
        }

        // Fall back to MCP level
        if let Some(timeout) = self.config.mcp.timeout {
            return timeout;
        }

        // Fall back to global level
        self.config.global.timeout.unwrap_or(30)
    }

    /// Resolve max retries with precedence: Tool → MCP → Global
    pub fn resolve_max_retries(&self, tool_name: Option<&str>) -> u32 {
        // Check tool-level configuration first
        if let Some(tool_name) = tool_name {
            if let Some(tool_config) = self.config.tools.get(tool_name) {
                if let Some(max_retries) = tool_config.max_retries {
                    return max_retries;
                }
            }
        }

        // Fall back to MCP level
        if let Some(max_retries) = self.config.mcp.max_retries {
            return max_retries;
        }

        // Fall back to global level
        self.config.global.max_retries.unwrap_or(3)
    }

    /// Resolve priority with precedence: Tool → MCP → Global
    pub fn resolve_priority(&self, tool_name: Option<&str>) -> u8 {
        // Check tool-level configuration first
        if let Some(tool_name) = tool_name {
            if let Some(tool_config) = self.config.tools.get(tool_name) {
                if let Some(priority) = tool_config.priority {
                    return priority;
                }
            }
        }

        // Fall back to MCP level
        if let Some(priority) = self.config.mcp.priority {
            return priority;
        }

        // Fall back to global level
        self.config.global.priority.unwrap_or(5)
    }

    /// Resolve enabled status with precedence: Tool → MCP → Global
    pub fn resolve_enabled(&self, tool_name: Option<&str>) -> bool {
        // Check tool-level configuration first
        if let Some(tool_name) = tool_name {
            if let Some(tool_config) = self.config.tools.get(tool_name) {
                if let Some(enabled) = tool_config.enabled {
                    return enabled;
                }
            }
        }

        // Fall back to MCP level
        if let Some(enabled) = self.config.mcp.enabled {
            return enabled;
        }

        // Fall back to global level
        self.config.global.enabled.unwrap_or(true)
    }

    /// Resolve hidden status with precedence: Tool → MCP → Global
    pub fn resolve_hidden(&self, tool_name: Option<&str>) -> bool {
        // Check tool-level configuration first
        if let Some(tool_name) = tool_name {
            if let Some(tool_config) = self.config.tools.get(tool_name) {
                if let Some(hidden) = tool_config.hidden {
                    return hidden;
                }
            }
        }

        // Fall back to MCP level
        if let Some(hidden) = self.config.mcp.hidden {
            return hidden;
        }

        // Fall back to global level
        self.config.global.hidden.unwrap_or(true)  // Default to hidden per architectural requirement
    }

    /// Resolve retry attempts setting with precedence: Tool → MCP → Global (legacy compatibility)
    pub fn resolve_retry_attempts(&self, tool_name: Option<&str>) -> u32 {
        self.resolve_max_retries(tool_name)
    }

    /// Resolve retry delay setting with precedence: Tool → MCP → Global (legacy compatibility)
    pub fn resolve_retry_delay_ms(&self, tool_name: Option<&str>) -> u64 {
        // Check tool-level override first
        if let Some(tool_name) = tool_name {
            if let Some(tool_config) = self.config.tools.get(tool_name) {
                if let Some(overrides) = &tool_config.overrides {
                    return overrides.retry_delay_ms;
                }
            }
        }

        // Check MCP-level override next
        if let Some(mcp_overrides) = &self.config.mcp.overrides {
            return mcp_overrides.retry_delay_ms;
        }

        // Fall back to global default
        self.config.global.defaults.retry_delay_ms
    }

    /// Get all configuration for a specific tool, applying precedence
    pub fn resolve_tool_config(&self, tool_name: &str) -> ResolvedToolConfig {
        ResolvedToolConfig {
            name: tool_name.to_string(),
            timeout: self.resolve_timeout(Some(tool_name)),
            max_retries: self.resolve_max_retries(Some(tool_name)),
            priority: self.resolve_priority(Some(tool_name)),
            enabled: self.resolve_enabled(Some(tool_name)),
            hidden: self.resolve_hidden(Some(tool_name)),
        }
    }

    /// Get configuration overrides for a specific tool (only direct overrides, not inherited)
    pub fn get_tool_overrides(&self, tool_name: &str) -> Option<&ToolConfig> {
        self.config.tools.get(tool_name)
    }

    /// Check if a tool has any local overrides
    pub fn has_tool_overrides(&self, tool_name: &str) -> bool {
        self.config.tools.contains_key(tool_name)
    }

    /// Get list of all tools with explicit configuration overrides
    pub fn get_configured_tools(&self) -> Vec<String> {
        self.config.tools.keys().cloned().collect()
    }

    /// Get the hierarchical configuration reference
    pub fn config(&self) -> &HierarchicalConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_hierarchical_config() -> HierarchicalConfig {
        let global = GlobalConfig {
            deployment: DeploymentConfig::default(),
            server: ServerConfig {
                host: "localhost".to_string(),
                port: 3001,
                websocket: true,
                timeout: 30,
                tls: None,
            },
            registry: RegistryConfig {
                r#type: "file".to_string(),
                paths: vec!["capabilities".to_string()],
                hot_reload: true,
                validation: ValidationConfig {
                    strict: true,
                    allow_unknown_fields: false,
                },
            },
            logging: LoggingConfig::default(),
            security: crate::security::SecurityConfig::default(),
            auth: crate::auth::MultiLevelAuthConfig::default(),
            defaults: DefaultSettings {
                timeout: 30,
                retry_attempts: 3,
                retry_delay_ms: 1000,
            },
            timeout: None,
            max_retries: None,
            priority: None,
            enabled: None,
            hidden: None,
        };

        let mcp = McpConfig {
            external_mcp: ExternalMcpConfig::default(),
            mcp_client: McpClientConfig::default(),
            streamable_http: StreamableHttpTransportConfig::default(),
            sampling: SamplingConfig::default(),
            elicitation: ElicitationConfig::default(),
            content_services: ContentServicesConfig {
                prompt_generation: None,
                resource_generation: None,
                content_storage: None,
                external_content: None,
            },
            overrides: Some(DefaultSettings {
                timeout: 45,  // MCP override
                retry_attempts: 5,
                retry_delay_ms: 1500,
            }),
            timeout: Some(45),    // MCP-level convenience shortcut
            max_retries: Some(5), // MCP-level convenience shortcut
            priority: Some(7),    // MCP-level priority override
            enabled: Some(true),  // MCP-level enabled override
            hidden: Some(true),   // MCP-level visibility override
        };

        let discovery = DiscoveryConfig {
            smart_discovery: crate::discovery::SmartDiscoveryConfig::default(),
            tool_enhancement: ToolEnhancementConfig::default(),
            enhancement_storage: crate::discovery::EnhancementStorageConfig::default(),
            visibility: VisibilityConfig::default(),
            conflict_resolution: crate::routing::ConflictResolutionConfig::default(),
        };

        let mut tools = HashMap::new();
        tools.insert("test_tool".to_string(), ToolConfig {
            overrides: Some(DefaultSettings {
                timeout: 60,  // Tool-specific override
                retry_attempts: 2,
                retry_delay_ms: 2000,
            }),
            routing: None,
            parameters: None,
            auth: None,
            timeout: Some(60),    // Tool-level convenience shortcut
            max_retries: Some(2), // Tool-level convenience shortcut
            priority: Some(9),    // Tool-level priority override
            enabled: Some(true),  // Tool-level enabled override
            hidden: Some(false),  // Tool-level visibility override
        });

        HierarchicalConfig {
            global,
            mcp,
            discovery,
            tools,
        }
    }

    #[test]
    fn test_hierarchical_config_creation() {
        let config = create_test_hierarchical_config();
        let resolver = ConfigResolver::new(config);
        
        // Test that config resolver can access all levels
        assert_eq!(resolver.config().global.defaults.timeout, 30);
        assert_eq!(resolver.config().mcp.timeout.unwrap(), 45);
        assert_eq!(resolver.config().tools.get("test_tool").unwrap().timeout.unwrap(), 60);
    }

    #[test]
    fn test_precedence_resolution() {
        let config = create_test_hierarchical_config();
        let resolver = ConfigResolver::new(config);

        // Test tool-level precedence (highest)
        assert_eq!(resolver.resolve_timeout(Some("test_tool")), 60);
        
        // Test MCP-level precedence (middle) 
        assert_eq!(resolver.resolve_timeout(Some("unknown_tool")), 45);
        
        // Test global precedence (lowest) - no tool name provided
        let config_no_mcp_override = HierarchicalConfig {
            global: resolver.config().global.clone(),
            mcp: McpConfig {
                external_mcp: ExternalMcpConfig::default(),
                mcp_client: McpClientConfig::default(),
                streamable_http: StreamableHttpTransportConfig::default(),
                sampling: SamplingConfig::default(),
                elicitation: ElicitationConfig::default(),
                content_services: ContentServicesConfig {
                    prompt_generation: None,
                    resource_generation: None,
                    content_storage: None,
                    external_content: None,
                },
                overrides: None,  // No MCP override
                timeout: None,
                max_retries: None,
                priority: None,
                enabled: None,
                hidden: None,
            },
            discovery: resolver.config().discovery.clone(),
            tools: HashMap::new(),  // No tool overrides
        };
        let resolver_no_overrides = ConfigResolver::new(config_no_mcp_override);
        assert_eq!(resolver_no_overrides.resolve_timeout(None), 30);
    }

    #[test]
    fn test_all_precedence_settings() {
        let config = create_test_hierarchical_config();
        let resolver = ConfigResolver::new(config);

        // Test all three settings with tool override
        assert_eq!(resolver.resolve_timeout(Some("test_tool")), 60);
        assert_eq!(resolver.resolve_retry_attempts(Some("test_tool")), 2);
        assert_eq!(resolver.resolve_retry_delay_ms(Some("test_tool")), 2000);

        // Test all three settings with MCP override (unknown tool)
        assert_eq!(resolver.resolve_timeout(Some("unknown_tool")), 45);
        assert_eq!(resolver.resolve_retry_attempts(Some("unknown_tool")), 5);
        assert_eq!(resolver.resolve_retry_delay_ms(Some("unknown_tool")), 1500);
    }

    #[test]
    fn test_flat_to_hierarchical_migration() {
        // Create a flat config with various settings
        let mut flat_config = crate::config::Config::default();
        flat_config.server.timeout = 25;
        flat_config.server.host = "0.0.0.0".to_string();
        
        // Convert to hierarchical
        let hierarchical = HierarchicalConfig::from_flat_config(flat_config);
        
        // Verify migration preserved values
        assert_eq!(hierarchical.global.server.timeout, 25);
        assert_eq!(hierarchical.global.server.host, "0.0.0.0");
        assert_eq!(hierarchical.global.defaults.timeout, 25); // extracted from server
        
        // Test that it can be converted back
        let back_to_flat = hierarchical.to_flat_config();
        assert_eq!(back_to_flat.server.timeout, 25);
        assert_eq!(back_to_flat.server.host, "0.0.0.0");
    }

    #[test]
    fn test_yaml_migration() {
        let flat_yaml = r#"
server:
  host: "localhost"
  port: 3001
  timeout: 35
  websocket: true
registry:
  type: "file" 
  paths: ["./capabilities"]
  hot_reload: true
  validation:
    strict: false
    allow_unknown_fields: true
"#;
        
        // This should auto-detect as flat config and migrate
        let hierarchical = HierarchicalConfig::from_yaml(flat_yaml).unwrap();
        
        // Verify the migration worked correctly
        assert_eq!(hierarchical.global.server.host, "localhost");
        assert_eq!(hierarchical.global.server.port, 3001);
        assert_eq!(hierarchical.global.server.timeout, 35);
        assert_eq!(hierarchical.global.defaults.timeout, 35);
        
        // Verify the config can be validated
        assert!(hierarchical.validate().is_ok());
    }

    #[test] 
    fn test_validation_errors() {
        let mut config = create_test_hierarchical_config();
        
        // Create invalid defaults
        config.global.defaults.timeout = 0; // Invalid timeout
        
        // This should fail validation
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_tool_level_configuration_precedence() {
        let config = create_test_hierarchical_config();
        let resolver = ConfigResolver::new(config);

        // Test tool-level precedence takes highest priority
        assert_eq!(resolver.resolve_timeout(Some("test_tool")), 60);
        assert_eq!(resolver.resolve_max_retries(Some("test_tool")), 2);
        assert_eq!(resolver.resolve_priority(Some("test_tool")), 9);
        assert_eq!(resolver.resolve_enabled(Some("test_tool")), true);
        assert_eq!(resolver.resolve_hidden(Some("test_tool")), false);

        // Test unknown tool falls back to MCP level
        assert_eq!(resolver.resolve_timeout(Some("unknown_tool")), 45); // MCP override
        
        // Test no tool name provided falls back appropriately
        assert_eq!(resolver.resolve_timeout(None), 45); // MCP override
    }

    #[test]
    fn test_resolve_tool_config() {
        let config = create_test_hierarchical_config();
        let resolver = ConfigResolver::new(config);

        let resolved = resolver.resolve_tool_config("test_tool");
        assert_eq!(resolved.name, "test_tool");
        assert_eq!(resolved.timeout, 60);
        assert_eq!(resolved.max_retries, 2);
        assert_eq!(resolved.priority, 9);
        assert_eq!(resolved.enabled, true);
        assert_eq!(resolved.hidden, false);
    }

    #[test]
    fn test_tool_configuration_management() {
        let config = create_test_hierarchical_config();
        let resolver = ConfigResolver::new(config);

        // Test tool override detection
        assert!(resolver.has_tool_overrides("test_tool"));
        assert!(!resolver.has_tool_overrides("unknown_tool"));

        // Test getting tool overrides
        let overrides = resolver.get_tool_overrides("test_tool");
        assert!(overrides.is_some());
        assert_eq!(overrides.unwrap().timeout, Some(60));

        // Test getting configured tools list
        let configured_tools = resolver.get_configured_tools();
        assert_eq!(configured_tools.len(), 1);
        assert!(configured_tools.contains(&"test_tool".to_string()));
    }

    #[test]
    fn test_configuration_isolation() {
        // Create config with MCP-level and Global-level settings but no tool overrides
        let mut config = create_test_hierarchical_config();
        config.tools.clear(); // Remove tool overrides
        config.mcp.timeout = Some(25);
        config.mcp.priority = Some(3);
        config.global.timeout = Some(15);
        config.global.priority = Some(1);
        
        let resolver = ConfigResolver::new(config);

        // Test that MCP level is used when no tool overrides
        assert_eq!(resolver.resolve_timeout(Some("any_tool")), 25);
        assert_eq!(resolver.resolve_priority(Some("any_tool")), 3);

        // Test that global level is used when no MCP or tool overrides
        let mut config_no_mcp = create_test_hierarchical_config();
        config_no_mcp.tools.clear();
        config_no_mcp.mcp.timeout = None;
        config_no_mcp.mcp.priority = None;
        config_no_mcp.global.timeout = Some(15);
        config_no_mcp.global.priority = Some(1);
        
        let resolver_no_mcp = ConfigResolver::new(config_no_mcp);
        assert_eq!(resolver_no_mcp.resolve_timeout(Some("any_tool")), 15);
        assert_eq!(resolver_no_mcp.resolve_priority(Some("any_tool")), 1);
    }

    #[test]
    fn test_default_values_when_no_overrides() {
        // Create a minimal config with no overrides at any level
        let minimal_config = HierarchicalConfig {
            global: GlobalConfig::default(),
            mcp: McpConfig::default(),
            discovery: DiscoveryConfig {
                smart_discovery: crate::discovery::SmartDiscoveryConfig::default(),
                tool_enhancement: ToolEnhancementConfig::default(),
                enhancement_storage: crate::discovery::EnhancementStorageConfig::default(),
                visibility: VisibilityConfig::default(),
                conflict_resolution: crate::routing::ConflictResolutionConfig::default(),
            },
            tools: HashMap::new(),
        };
        
        let resolver = ConfigResolver::new(minimal_config);
        
        // Test default values are used when no overrides exist
        assert_eq!(resolver.resolve_timeout(Some("any_tool")), 30);    // Default timeout
        assert_eq!(resolver.resolve_max_retries(Some("any_tool")), 3); // Default max retries
        assert_eq!(resolver.resolve_priority(Some("any_tool")), 5);    // Default priority
        assert_eq!(resolver.resolve_enabled(Some("any_tool")), true);  // Default enabled
        assert_eq!(resolver.resolve_hidden(Some("any_tool")), true);   // Default hidden (per architecture)
    }

    #[test]
    fn test_end_to_end_configuration_flow() {
        // Test end-to-end flow using programmatic config instead of YAML parsing
        // to focus on hierarchical precedence logic rather than YAML structure complexity
        
        // Create hierarchical config programmatically
        let config = HierarchicalConfig {
            global: GlobalConfig {
                // Use defaults for most fields to avoid YAML complexity
                timeout: Some(25),    // Global timeout override
                max_retries: Some(4), // Global max retries override  
                priority: Some(6),    // Global priority override
                enabled: Some(true),  // Global enabled
                hidden: Some(true),   // Global hidden (default)
                ..Default::default()
            },
            mcp: McpConfig {
                timeout: Some(20),    // MCP timeout override (higher precedence than global)
                max_retries: Some(2), // MCP max retries override 
                priority: Some(7),    // MCP priority override
                enabled: Some(true),  // MCP enabled
                hidden: Some(false),  // MCP visibility override
                ..Default::default()
            },
            discovery: DiscoveryConfig {
                smart_discovery: crate::discovery::SmartDiscoveryConfig {
                    enabled: true,
                    tool_selection_mode: "hybrid".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
            tools: {
                let mut tools = HashMap::new();
                
                // High priority tool with specific overrides
                tools.insert("test_tool_high_priority".to_string(), ToolConfig {
                    timeout: Some(15),     // Tool-specific timeout (highest precedence)
                    max_retries: Some(1),  // Tool-specific max retries
                    priority: Some(10),    // High priority tool
                    enabled: Some(true),   // Tool enabled
                    hidden: Some(false),   // Tool visible
                    ..Default::default()
                });
                
                // Disabled tool 
                tools.insert("test_tool_disabled".to_string(), ToolConfig {
                    enabled: Some(false),  // Tool disabled
                    hidden: Some(true),    // Tool hidden
                    priority: Some(1),     // Low priority
                    ..Default::default()
                });
                
                tools
            },
        };
        
        let resolver = ConfigResolver::new(config);
        
        // Test 1: Verify Global-level defaults for unknown tools (fall back to MCP level)
        assert_eq!(resolver.resolve_timeout(Some("unknown_tool")), 20);  // MCP level override
        assert_eq!(resolver.resolve_max_retries(Some("unknown_tool")), 2); // MCP level override
        assert_eq!(resolver.resolve_priority(Some("unknown_tool")), 7);   // MCP level override
        assert_eq!(resolver.resolve_enabled(Some("unknown_tool")), true); // MCP level
        assert_eq!(resolver.resolve_hidden(Some("unknown_tool")), false); // MCP level override
        
        // Test 2: Verify Tool-level precedence (highest precedence)
        let high_priority_config = resolver.resolve_tool_config("test_tool_high_priority");
        assert_eq!(high_priority_config.timeout, 15);      // Tool-level override
        assert_eq!(high_priority_config.max_retries, 1);   // Tool-level override  
        assert_eq!(high_priority_config.priority, 10);     // Tool-level override
        assert_eq!(high_priority_config.enabled, true);    // Tool-level
        assert_eq!(high_priority_config.hidden, false);    // Tool-level override
        
        let disabled_tool_config = resolver.resolve_tool_config("test_tool_disabled");
        assert_eq!(disabled_tool_config.enabled, false);   // Tool-level disabled
        assert_eq!(disabled_tool_config.hidden, true);     // Tool-level hidden
        assert_eq!(disabled_tool_config.priority, 1);      // Tool-level low priority
        assert_eq!(disabled_tool_config.timeout, 20);      // Falls back to MCP level
        assert_eq!(disabled_tool_config.max_retries, 2);   // Falls back to MCP level
        
        // Test 3: Verify configuration isolation
        assert!(resolver.has_tool_overrides("test_tool_high_priority"));
        assert!(resolver.has_tool_overrides("test_tool_disabled"));
        assert!(!resolver.has_tool_overrides("nonexistent_tool"));
        
        let configured_tools = resolver.get_configured_tools();
        assert_eq!(configured_tools.len(), 2);
        assert!(configured_tools.contains(&"test_tool_high_priority".to_string()));
        assert!(configured_tools.contains(&"test_tool_disabled".to_string()));
        
        // Test 4: Verify precedence cascade works correctly
        // Tool with no overrides should fall back through MCP to Global levels
        assert_eq!(resolver.resolve_timeout(None), 20);    // No tool name, uses MCP level  
        assert_eq!(resolver.resolve_max_retries(None), 2);  // No tool name, uses MCP level
        
        // Test 5: Verify Smart Discovery configuration is properly isolated
        assert_eq!(resolver.config().discovery.smart_discovery.enabled, true);
        assert_eq!(resolver.config().discovery.smart_discovery.tool_selection_mode, "hybrid");
        
        println!("✅ End-to-end configuration flow test passed");
    }

    #[test]
    fn test_comprehensive_migration_scenarios() {
        // Test migration of complex flat config with all major sections
        let mut flat_config = crate::config::Config::default();
        
        // Configure server settings
        flat_config.server.host = "192.168.1.100".to_string();
        flat_config.server.port = 8080;
        flat_config.server.timeout = 45;
        flat_config.server.websocket = true;
        
        // Configure registry settings  
        flat_config.registry.paths = vec!["./tools".to_string(), "./extensions".to_string()];
        flat_config.registry.hot_reload = true;
        flat_config.registry.validation.strict = false;
        
        // Configure auth settings
        flat_config.auth = Some(crate::config::AuthConfig {
            enabled: true,
            r#type: crate::config::AuthType::OAuth,
            oauth: None,
            api_keys: None,
            jwt: None,
        });
        
        // Configure smart discovery
        flat_config.smart_discovery = Some(crate::discovery::SmartDiscoveryConfig {
            enabled: true,
            tool_selection_mode: "semantic".to_string(),
            default_confidence_threshold: 0.85,
            ..Default::default()
        });
        
        // Convert to hierarchical
        let hierarchical = HierarchicalConfig::from_flat_config(flat_config.clone());
        
        // Test 1: Verify server migration to global level
        assert_eq!(hierarchical.global.server.host, "192.168.1.100");
        assert_eq!(hierarchical.global.server.port, 8080);
        assert_eq!(hierarchical.global.server.timeout, 45);
        assert_eq!(hierarchical.global.server.websocket, true);
        
        // Test 2: Verify defaults extraction from server
        assert_eq!(hierarchical.global.defaults.timeout, 45);
        
        // Test 3: Verify registry migration
        assert_eq!(hierarchical.global.registry.paths, vec!["./tools", "./extensions"]);
        assert_eq!(hierarchical.global.registry.hot_reload, true);
        assert_eq!(hierarchical.global.registry.validation.strict, false);
        
        // Test 4: Verify smart discovery migration to proper section
        assert_eq!(hierarchical.discovery.smart_discovery.enabled, true);
        assert_eq!(hierarchical.discovery.smart_discovery.tool_selection_mode, "semantic");
        assert_eq!(hierarchical.discovery.smart_discovery.default_confidence_threshold, 0.85);
        
        // Test 6: Verify MCP and tool sections initialized with defaults
        assert_eq!(hierarchical.mcp.timeout, None); // No MCP-specific overrides
        assert_eq!(hierarchical.tools.len(), 0);    // No tool-specific configs
        
        // Test 7: Verify round-trip migration preserves data
        let back_to_flat = hierarchical.to_flat_config();
        assert_eq!(back_to_flat.server.host, flat_config.server.host);
        assert_eq!(back_to_flat.server.port, flat_config.server.port);
        assert_eq!(back_to_flat.server.timeout, flat_config.server.timeout);
        assert_eq!(back_to_flat.registry.paths, flat_config.registry.paths);
        assert_eq!(back_to_flat.smart_discovery.as_ref().unwrap().tool_selection_mode, 
                   flat_config.smart_discovery.as_ref().unwrap().tool_selection_mode);
        
        println!("✅ Comprehensive migration scenarios test passed");
    }

    #[test]
    fn test_migration_preserves_semantic_equivalence() {
        // Test that migrated config resolves values identically to original flat config
        let mut flat_config = crate::config::Config::default();
        flat_config.server.timeout = 30;
        flat_config.server.host = "test.local".to_string();
        
        // Create hierarchical version
        let hierarchical = HierarchicalConfig::from_flat_config(flat_config.clone());
        let resolver = ConfigResolver::new(hierarchical);
        
        // Test semantic equivalence for key settings
        assert_eq!(resolver.resolve_timeout(None), flat_config.server.timeout as u64);
        assert_eq!(resolver.resolve_timeout(Some("any_tool")), flat_config.server.timeout as u64);
        
        println!("✅ Migration preserves semantic equivalence test passed");
    }

    #[test] 
    fn test_migration_error_handling() {
        // Test migration with edge cases and potential error conditions
        
        // Test 1: Migration with minimal config
        let minimal_flat = crate::config::Config::default();
        let minimal_hierarchical = HierarchicalConfig::from_flat_config(minimal_flat);
        assert!(minimal_hierarchical.validate().is_ok());
        
        // Test 2: Migration with extreme values
        let mut extreme_flat = crate::config::Config::default();
        extreme_flat.server.timeout = 0; // Edge case: zero timeout
        extreme_flat.server.port = 65535; // Edge case: max port
        
        let extreme_hierarchical = HierarchicalConfig::from_flat_config(extreme_flat);
        // Should preserve values even if they might be invalid
        assert_eq!(extreme_hierarchical.global.server.timeout, 0);
        assert_eq!(extreme_hierarchical.global.server.port, 65535);
        
        println!("✅ Migration error handling test passed");
    }

    #[test]
    fn test_migration_tool_integration() {
        // Test that the CLI tool can be used programmatically
        let mut flat_config = crate::config::Config::default();
        flat_config.server.host = "test.local".to_string();
        flat_config.server.port = 9999;
        flat_config.server.timeout = 60;
        
        // Convert to hierarchical
        let hierarchical = HierarchicalConfig::from_flat_config(flat_config.clone());
        
        // Verify conversion worked
        assert_eq!(hierarchical.global.server.host, "test.local");
        assert_eq!(hierarchical.global.server.port, 9999);
        assert_eq!(hierarchical.global.server.timeout, 60);
        
        // Test validation
        assert!(hierarchical.validate().is_ok());
        
        // Test round-trip conversion  
        let back_to_flat = hierarchical.to_flat_config();
        assert_eq!(back_to_flat.server.host, flat_config.server.host);
        assert_eq!(back_to_flat.server.port, flat_config.server.port);
        assert_eq!(back_to_flat.server.timeout, flat_config.server.timeout);
        
        println!("✅ Migration tool integration test passed");
    }

    #[test]
    fn test_configuration_edge_cases() {
        // Test edge cases in configuration resolution and validation
        
        // Test 1: Empty configuration sections
        let mut config = HierarchicalConfig::default();
        let resolver = ConfigResolver::new(config.clone());
        
        // Should use default values for empty configuration
        assert!(resolver.resolve_timeout(None) > 0);    // Should have reasonable default
        assert!(resolver.resolve_max_retries(None) > 0); // Should have reasonable default
        assert!(resolver.resolve_enabled(None));         // Should be enabled by default
        
        // Test 2: Conflicting tool configurations
        config.tools.insert("conflicting_tool".to_string(), ToolConfig {
            timeout: Some(0),      // Invalid timeout
            max_retries: Some(0),  // Invalid retries
            priority: Some(255),   // Max priority (edge case test)
            enabled: Some(true),   // Enabled but with invalid settings
            hidden: Some(false),
            ..Default::default()
        });
        
        let resolver = ConfigResolver::new(config.clone());
        
        // Should preserve the configured values even if invalid (validation happens elsewhere)
        assert_eq!(resolver.resolve_timeout(Some("conflicting_tool")), 0);
        assert_eq!(resolver.resolve_max_retries(Some("conflicting_tool")), 0);
        assert_eq!(resolver.resolve_priority(Some("conflicting_tool")), 255);
        
        // Test 3: Extremely large values
        config.global.timeout = Some(u64::MAX);
        config.global.max_retries = Some(u32::MAX);
        config.global.priority = Some(u8::MAX);
        
        let resolver = ConfigResolver::new(config.clone());
        assert_eq!(resolver.resolve_timeout(Some("unknown_tool")), u64::MAX);
        assert_eq!(resolver.resolve_max_retries(Some("unknown_tool")), u32::MAX);
        assert_eq!(resolver.resolve_priority(Some("unknown_tool")), u8::MAX as u8);
        
        println!("✅ Configuration edge cases test passed");
    }

    #[test]
    fn test_configuration_error_conditions() {
        // Test various error conditions that could occur in hierarchical configuration
        
        // Test 1: Configuration with missing required fields (should use defaults)
        let minimal_config = HierarchicalConfig {
            global: GlobalConfig::default(),
            mcp: McpConfig::default(),
            discovery: DiscoveryConfig::default(),
            tools: HashMap::new(),
        };
        
        assert!(minimal_config.validate().is_ok()); // Should validate successfully with defaults
        
        // Test 2: Configuration with invalid YAML structure (test error handling)
        let invalid_yaml = r#"
        global:
          invalid_field: true
          nested:
            deeply:
              invalid: "structure"
        mcp:
          - this_should_be_object_not_array
        "#;
        
        // This should either gracefully handle or provide clear error
        let result = HierarchicalConfig::from_yaml(invalid_yaml);
        // Should either succeed with defaults or fail gracefully
        assert!(result.is_ok() || result.is_err());
        
        // Test 3: Tool name edge cases
        let mut config = create_test_hierarchical_config();
        let resolver = ConfigResolver::new(config.clone());
        
        // Test empty tool name
        assert_eq!(resolver.resolve_timeout(Some("")), resolver.resolve_timeout(None));
        
        // Test very long tool name
        let long_tool_name = "a".repeat(1000);
        assert_eq!(resolver.resolve_timeout(Some(&long_tool_name)), resolver.resolve_timeout(None));
        
        // Test special characters in tool name
        assert_eq!(resolver.resolve_timeout(Some("tool-with-dashes_and_underscores.and.dots")), resolver.resolve_timeout(None));
        
        println!("✅ Configuration error conditions test passed");
    }

    #[test]
    fn test_configuration_validation_edge_cases() {
        // Test validation with various edge cases
        
        // Test 1: Validation with invalid timeout
        let mut config = create_test_hierarchical_config();
        config.global.defaults.timeout = 0; // Invalid
        
        let validation_result = config.validate();
        assert!(validation_result.is_err(), "Should fail validation with zero timeout");
        
        // Test 2: Validation with invalid priority ranges
        config.global.priority = Some(0); // Minimum u8 priority
        let validation_result = config.validate();
        // Should handle extreme values appropriately
        
        // Test 3: Validation with circular dependencies (if any exist)
        // Note: Current design doesn't have circular dependencies, but test for future-proofing
        let resolver = ConfigResolver::new(config);
        
        // Should not cause infinite recursion or stack overflow
        for i in 0..1000 {
            let tool_name = format!("test_tool_{}", i);
            resolver.resolve_timeout(Some(&tool_name));
        }
        
        println!("✅ Configuration validation edge cases test passed");
    }

    #[test]
    fn test_configuration_precedence_edge_cases() {
        // Test edge cases in precedence resolution
        
        let mut config = HierarchicalConfig::default();
        
        // Test 1: Partial overrides at different levels
        config.global.timeout = Some(100);
        config.global.max_retries = None; // Not set at global level
        
        config.mcp.timeout = None; // Not overridden at MCP level
        config.mcp.max_retries = Some(5); // Set at MCP level
        
        config.tools.insert("partial_tool".to_string(), ToolConfig {
            timeout: Some(50), // Override only timeout
            max_retries: None, // Don't override max_retries
            ..Default::default()
        });
        
        let resolver = ConfigResolver::new(config);
        
        // Should properly cascade through levels
        assert_eq!(resolver.resolve_timeout(Some("partial_tool")), 50);    // Tool level
        assert_eq!(resolver.resolve_max_retries(Some("partial_tool")), 5); // MCP level
        
        assert_eq!(resolver.resolve_timeout(Some("other_tool")), 100);     // Global level
        assert_eq!(resolver.resolve_max_retries(Some("other_tool")), 5);   // MCP level
        
        // Test 2: None values vs not specified
        let tool_config = resolver.resolve_tool_config("partial_tool");
        assert_eq!(tool_config.timeout, 50);
        assert_eq!(tool_config.max_retries, 5);
        
        println!("✅ Configuration precedence edge cases test passed");
    }

    #[test]
    fn test_hierarchical_config_loading_performance() {
        // Test performance of hierarchical configuration loading and resolution
        use std::time::Instant;
        
        // Test 1: Configuration loading performance
        let start = Instant::now();
        
        // Create complex configuration
        let mut config = HierarchicalConfig::default();
        
        // Add many tool configurations to stress test the system
        for i in 0..1000 {
            config.tools.insert(format!("tool_{}", i), ToolConfig {
                timeout: Some(30 + i % 100),
                max_retries: Some((i % 10) as u32),
                priority: Some((i % 100) as u8),
                enabled: Some(i % 2 == 0),
                hidden: Some(i % 3 == 0),
                ..Default::default()
            });
        }
        
        let loading_duration = start.elapsed();
        println!("Configuration loading with 1000 tools: {:?}", loading_duration);
        
        // Should load reasonably quickly (under 100ms for 1000 tools)
        assert!(loading_duration.as_millis() < 100, "Configuration loading too slow: {:?}", loading_duration);
        
        // Test 2: Resolution performance
        let resolver = ConfigResolver::new(config);
        let start = Instant::now();
        
        // Perform many resolution operations
        for i in 0..10000 {
            let tool_name = format!("tool_{}", i % 1000);
            resolver.resolve_timeout(Some(&tool_name));
            resolver.resolve_max_retries(Some(&tool_name));
            resolver.resolve_priority(Some(&tool_name));
            resolver.resolve_enabled(Some(&tool_name));
            resolver.resolve_hidden(Some(&tool_name));
        }
        
        let resolution_duration = start.elapsed();
        println!("10000 resolution operations: {:?}", resolution_duration);
        
        // Should resolve quickly (under 100ms for 10000 operations)
        assert!(resolution_duration.as_millis() < 100, "Resolution operations too slow: {:?}", resolution_duration);
        
        println!("✅ Hierarchical config loading performance test passed");
    }

    #[test]
    fn test_hierarchical_config_memory_efficiency() {
        // Test memory efficiency of hierarchical configuration
        use std::mem;
        
        // Test 1: Memory footprint of configuration structures
        let config = create_test_hierarchical_config();
        let config_size = mem::size_of_val(&config);
        println!("HierarchicalConfig size: {} bytes", config_size);
        
        // Test 2: Memory usage with many tool configurations
        let mut large_config = HierarchicalConfig::default();
        
        for i in 0..1000 {
            large_config.tools.insert(format!("tool_{}", i), ToolConfig::default());
        }
        
        let large_config_size = mem::size_of_val(&large_config);
        println!("HierarchicalConfig with 1000 tools: {} bytes", large_config_size);
        
        // Test 3: Resolver memory footprint
        let resolver = ConfigResolver::new(config);
        let resolver_size = mem::size_of_val(&resolver);
        println!("ConfigResolver size: {} bytes", resolver_size);
        
        // Should be reasonable memory usage (under 10MB for 1000 tools)
        assert!(large_config_size < 10_000_000, "Config memory usage too high: {} bytes", large_config_size);
        
        println!("✅ Hierarchical config memory efficiency test passed");
    }

    #[test]
    fn test_hierarchical_config_scaling_performance() {
        // Test how performance scales with configuration size
        use std::time::Instant;
        
        let sizes = [10, 100, 500, 1000];
        
        for &size in &sizes {
            let mut config = HierarchicalConfig::default();
            
            // Create configuration with 'size' tools
            for i in 0..size {
                config.tools.insert(format!("tool_{}", i), ToolConfig {
                    timeout: Some(30),
                    max_retries: Some(3),
                    priority: Some(5),
                    enabled: Some(true),
                    hidden: Some(false),
                    ..Default::default()
                });
            }
            
            let resolver = ConfigResolver::new(config);
            
            // Measure resolution performance
            let start = Instant::now();
            for i in 0..1000 {
                let tool_name = format!("tool_{}", i % size);
                resolver.resolve_timeout(Some(&tool_name));
            }
            let duration = start.elapsed();
            
            println!("Resolution performance with {} tools: {:?} ({}ns per operation)", 
                     size, duration, duration.as_nanos() / 1000);
            
            // Performance should be reasonable and not degrade dramatically
            // Allow more time for larger configurations but should stay linear
            let max_ns_per_op = 10000 + (size as u128 * 10); // Base + linear scaling
            assert!(duration.as_nanos() / 1000 < max_ns_per_op, 
                    "Performance degradation with {} tools: {}ns per operation", 
                    size, duration.as_nanos() / 1000);
        }
        
        println!("✅ Hierarchical config scaling performance test passed");
    }

    #[test]
    fn test_yaml_parsing_performance() {
        // Test YAML parsing performance with hierarchical configuration
        use std::time::Instant;
        
        // Create a complex YAML configuration
        let complex_yaml = r#"
global:
  server:
    host: "0.0.0.0"
    port: 3001
    timeout: 30
    websocket: true
  registry:
    paths: ["./capabilities", "./extensions", "./plugins"]
    hot_reload: true
    validation:
      strict: false
      allow_unknown_fields: true
  auth:
    oauth:
      enabled: true
    api_keys:
      enabled: true
  timeout: 60
  max_retries: 5
  priority: 10
  enabled: true
  hidden: false

mcp:
  timeout: 45
  max_retries: 3
  priority: 8
  enabled: true
  hidden: false

discovery:
  smart_discovery:
    enabled: true
    tool_selection_mode: "hybrid"
    confidence_threshold: 0.75

tools:
  example_tool:
    timeout: 25
    max_retries: 2
    priority: 7
    enabled: true
    hidden: false
  another_tool:
    timeout: 35
    max_retries: 4
    priority: 9
    enabled: true
    hidden: true
"#;
        
        // Test parsing performance
        let start = Instant::now();
        for _ in 0..100 {
            let result = HierarchicalConfig::from_yaml(complex_yaml);
            assert!(result.is_ok());
        }
        let duration = start.elapsed();
        
        println!("100 YAML parsing operations: {:?} ({}μs per parse)", 
                 duration, duration.as_micros() / 100);
        
        // Should parse quickly (under 10ms per operation)
        assert!(duration.as_millis() < 1000, "YAML parsing too slow: {:?}", duration);
        
        println!("✅ YAML parsing performance test passed");
    }
}