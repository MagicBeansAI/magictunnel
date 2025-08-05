//! Registry types and structures

use crate::mcp::Tool;
use crate::mcp::types::ToolAnnotations;
use crate::config::SamplingElicitationStrategy;
use crate::error::{ProxyError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Default value for the enabled field
fn default_enabled() -> bool {
    true
}

/// Routing configuration for a tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingConfig {
    /// Routing type (subprocess, http, llm, etc.)
    pub r#type: String,
    /// Configuration specific to the routing type
    pub config: Value,
}

impl RoutingConfig {
    /// Create a new routing configuration
    pub fn new(routing_type: String, config: Value) -> Self {
        Self {
            r#type: routing_type,
            config,
        }
    }

    /// Validate the routing configuration
    pub fn validate(&self) -> Result<()> {
        // Validate routing type
        if self.r#type.trim().is_empty() {
            return Err(crate::error::ProxyError::validation("Routing type cannot be empty"));
        }

        // Validate known routing types
        match self.r#type.as_str() {
            "subprocess" => self.validate_subprocess_config(),
            "http" => self.validate_http_config(),
            "llm" => self.validate_llm_config(),
            "websocket" => self.validate_websocket_config(),
            "external_mcp" => self.validate_external_mcp_config(),
            // MCP 2025-06-18 enhanced routing types
            "smart_discovery" => Ok(()), // Smart discovery is always valid
            "enhanced_subprocess" => self.validate_enhanced_subprocess_config(),
            "ai_enhanced_discovery" => self.validate_ai_enhanced_discovery_config(),
            "enhanced_system_monitor" => self.validate_enhanced_system_monitor_config(),
            "ai_memory_analyzer" => self.validate_ai_memory_analyzer_config(),
            "ai_process_monitor" => self.validate_ai_process_monitor_config(),
            "ai_network_diagnostics" => self.validate_ai_network_diagnostics_config(),
            "ai_service_monitor" => self.validate_ai_service_monitor_config(),
            "ai_enhanced_processor" => self.validate_ai_enhanced_processor_config(),
            "enhanced" => Ok(()), // Generic enhanced type
            _ => {
                // Allow unknown types but warn
                tracing::warn!("Unknown routing type: {}", self.r#type);
                Ok(())
            }
        }
    }

    /// Validate subprocess routing configuration
    fn validate_subprocess_config(&self) -> Result<()> {
        let config = &self.config;

        if config.get("command").is_none() {
            return Err(crate::error::ProxyError::validation(
                "Subprocess routing requires 'command' field"
            ));
        }

        Ok(())
    }

    /// Validate HTTP routing configuration
    fn validate_http_config(&self) -> Result<()> {
        let config = &self.config;

        if config.get("url").is_none() {
            return Err(crate::error::ProxyError::validation(
                "HTTP routing requires 'url' field"
            ));
        }

        if config.get("method").is_none() {
            return Err(crate::error::ProxyError::validation(
                "HTTP routing requires 'method' field"
            ));
        }

        Ok(())
    }

    /// Validate LLM routing configuration
    fn validate_llm_config(&self) -> Result<()> {
        let config = &self.config;

        if config.get("provider").is_none() {
            return Err(crate::error::ProxyError::validation(
                "LLM routing requires 'provider' field"
            ));
        }

        if config.get("model").is_none() {
            return Err(crate::error::ProxyError::validation(
                "LLM routing requires 'model' field"
            ));
        }

        Ok(())
    }

    /// Validate WebSocket routing configuration
    fn validate_websocket_config(&self) -> Result<()> {
        let config = &self.config;

        if config.get("url").is_none() {
            return Err(crate::error::ProxyError::validation(
                "WebSocket routing requires 'url' field"
            ));
        }

        Ok(())
    }

    /// Validate External MCP routing configuration
    fn validate_external_mcp_config(&self) -> Result<()> {
        let config = &self.config;

        if config.get("server_name").is_none() && config.get("endpoint").is_none() {
            return Err(crate::error::ProxyError::validation(
                "External MCP routing requires 'server_name' or 'endpoint' field"
            ));
        }

        Ok(())
    }

    /// Get the routing type
    pub fn routing_type(&self) -> &str {
        &self.r#type
    }

    /// Check if routing type is supported
    pub fn is_supported_type(&self) -> bool {
        matches!(self.r#type.as_str(), 
            "subprocess" | "http" | "llm" | "websocket" | "database" | "external_mcp" |
            // MCP 2025-06-18 enhanced routing types
            "smart_discovery" | "enhanced_subprocess" | "ai_enhanced_discovery" |
            "enhanced_system_monitor" | "ai_memory_analyzer" | "ai_process_monitor" |
            "ai_network_diagnostics" | "ai_service_monitor" | "ai_enhanced_processor" | "enhanced"
        )
    }

    // MCP 2025-06-18 Enhanced routing validation methods
    
    /// Validate enhanced subprocess routing configuration
    fn validate_enhanced_subprocess_config(&self) -> Result<()> {
        // Enhanced subprocess allows more flexible configuration
        Ok(())
    }

    /// Validate AI-enhanced discovery routing configuration
    fn validate_ai_enhanced_discovery_config(&self) -> Result<()> {
        // AI discovery routing is always valid
        Ok(())
    }

    /// Validate enhanced system monitor routing configuration
    fn validate_enhanced_system_monitor_config(&self) -> Result<()> {
        // Enhanced system monitoring is always valid
        Ok(())
    }

    /// Validate AI memory analyzer routing configuration
    fn validate_ai_memory_analyzer_config(&self) -> Result<()> {
        // AI memory analyzer is always valid
        Ok(())
    }

    /// Validate AI process monitor routing configuration
    fn validate_ai_process_monitor_config(&self) -> Result<()> {
        // AI process monitor is always valid
        Ok(())
    }

    /// Validate AI network diagnostics routing configuration
    fn validate_ai_network_diagnostics_config(&self) -> Result<()> {
        // AI network diagnostics is always valid
        Ok(())
    }

    /// Validate AI service monitor routing configuration
    fn validate_ai_service_monitor_config(&self) -> Result<()> {
        // AI service monitor is always valid
        Ok(())
    }

    /// Validate AI enhanced processor routing configuration
    fn validate_ai_enhanced_processor_config(&self) -> Result<()> {
        // AI enhanced processor is always valid
        Ok(())
    }
}

/// Tool definition with routing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (unique identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    /// Routing configuration
    pub routing: RoutingConfig,
    /// Optional annotations for metadata
    pub annotations: Option<std::collections::HashMap<String, String>>,
    /// Whether this tool should be hidden from main tool lists (default: false)
    /// Hidden tools are still available for discovery and execution but not exposed in tools/list
    #[serde(default)]
    pub hidden: bool,
    /// Whether this tool is enabled for routing and execution (default: true)
    /// Disabled tools are not considered for routing or execution, regardless of visibility
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Generated prompt template references for this tool
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompt_refs: Vec<PromptReference>,
    /// Generated resource references for this tool
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_refs: Vec<ResourceReference>,
    /// Sampling strategy override for this specific tool
    pub sampling_strategy: Option<SamplingElicitationStrategy>,
    /// Elicitation strategy override for this specific tool
    pub elicitation_strategy: Option<SamplingElicitationStrategy>,
}

impl ToolDefinition {
    /// Create a new tool definition
    pub fn new(tool: Tool, routing: RoutingConfig) -> Result<Self> {
        let definition = Self {
            name: tool.name.clone(),
            description: tool.description.clone().unwrap_or_else(|| format!("Tool: {}", tool.name)),
            input_schema: tool.input_schema.clone(),
            routing,
            annotations: tool.annotations.as_ref().map(|_ann| {
                // Convert MCP annotations to simple string map
                std::collections::HashMap::new()
            }),
            hidden: true, // Default to hidden (consistent with other tools)
            enabled: true, // Default to enabled
            prompt_refs: Vec::new(), // No prompts initially
            resource_refs: Vec::new(), // No resources initially
            sampling_strategy: None, // Use server default
            elicitation_strategy: None, // Use server default
        };
        definition.validate()?;
        Ok(definition)
    }

    /// Create a new tool definition with explicit fields
    pub fn new_with_fields(
        name: String,
        description: String,
        input_schema: serde_json::Value,
        routing: RoutingConfig,
        annotations: Option<std::collections::HashMap<String, String>>,
    ) -> Result<Self> {
        let definition = Self {
            name,
            description,
            input_schema,
            routing,
            annotations,
            hidden: false, // Default to visible
            enabled: true, // Default to enabled
            prompt_refs: Vec::new(), // No prompts initially
            resource_refs: Vec::new(), // No resources initially
            sampling_strategy: None, // Use server default
            elicitation_strategy: None, // Use server default
        };
        definition.validate()?;
        Ok(definition)
    }

    /// Create a new tool definition with all fields including hidden and enabled
    pub fn new_with_all_fields(
        name: String,
        description: String,
        input_schema: serde_json::Value,
        routing: RoutingConfig,
        annotations: Option<std::collections::HashMap<String, String>>,
        hidden: bool,
        enabled: bool,
    ) -> Result<Self> {
        let definition = Self {
            name,
            description,
            input_schema,
            routing,
            annotations,
            hidden,
            enabled,
            prompt_refs: Vec::new(), // No prompts initially
            resource_refs: Vec::new(), // No resources initially
            sampling_strategy: None, // Use server default
            elicitation_strategy: None, // Use server default
        };
        definition.validate()?;
        Ok(definition)
    }

    /// Create a new tool definition with hidden field (enabled defaults to true)
    pub fn new_with_hidden(
        name: String,
        description: String,
        input_schema: serde_json::Value,
        routing: RoutingConfig,
        annotations: Option<std::collections::HashMap<String, String>>,
        hidden: bool,
    ) -> Result<Self> {
        Self::new_with_all_fields(name, description, input_schema, routing, annotations, hidden, true)
    }

    /// Convert to MCP Tool
    pub fn to_mcp_tool(&self) -> Tool {
        Tool {
            name: self.name.clone(),
            description: Some(self.description.clone()),
            title: None,
            input_schema: self.input_schema.clone(),
            output_schema: None,
            annotations: self.annotations.as_ref().map(|_ann| {
                ToolAnnotations {
                    title: None,
                    read_only_hint: None,
                    destructive_hint: None,
                    idempotent_hint: None,
                    open_world_hint: None,
                    sampling: None,
                    elicitation: None,
                }
            }),
        }
    }

    /// Validate the tool definition
    pub fn validate(&self) -> Result<()> {
        // Validate the tool name
        if self.name.is_empty() {
            return Err(ProxyError::validation("Tool name cannot be empty".to_string()));
        }

        // Validate the description
        if self.description.is_empty() {
            return Err(ProxyError::validation("Tool description cannot be empty".to_string()));
        }

        // Validate the input schema
        if !self.input_schema.is_object() {
            return Err(ProxyError::validation("Input schema must be a JSON object".to_string()));
        }

        // Validate the routing configuration
        self.routing.validate()?;

        Ok(())
    }

    /// Get the tool name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the tool description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the routing type
    pub fn routing_type(&self) -> &str {
        self.routing.routing_type()
    }

    /// Check if the tool is safe to execute
    pub fn is_safe(&self) -> bool {
        // Check if tool has destructive annotations
        if let Some(annotations) = &self.annotations {
            // If explicitly marked as destructive, it's not safe
            if let Some(destructive) = annotations.get("destructive") {
                if destructive.parse::<bool>().unwrap_or(false) {
                    return false;
                }
            }
        }

        // Check routing type for inherently dangerous operations
        match self.routing.routing_type() {
            "subprocess" => {
                // Subprocess tools are potentially dangerous
                // Check if the command contains dangerous operations
                if let Some(config) = self.routing.config.as_object() {
                    if let Some(command) = config.get("command").and_then(|v| v.as_str()) {
                        // List of dangerous commands
                        let dangerous_commands = ["rm", "del", "format", "fdisk", "dd", "shutdown", "reboot"];
                        return !dangerous_commands.iter().any(|&cmd| command.contains(cmd));
                    }
                }
                false // Default to unsafe for subprocess without proper validation
            }
            _ => true // Other routing types are generally safe
        }
    }

    /// Check if this tool is hidden
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Set the hidden status of this tool
    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    /// Create a copy of this tool with hidden status changed
    pub fn with_hidden(&self, hidden: bool) -> Self {
        let mut tool = self.clone();
        tool.hidden = hidden;
        tool
    }

    /// Check if this tool is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set the enabled status of this tool
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Create a copy of this tool with enabled status changed
    pub fn with_enabled(&self, enabled: bool) -> Self {
        let mut tool = self.clone();
        tool.enabled = enabled;
        tool
    }

    /// Validate arguments for this tool
    pub fn validate_arguments(&self, arguments: &Value) -> Result<()> {
        // Create a temporary Tool to validate arguments
        let tool = self.to_mcp_tool();
        tool.validate_arguments(arguments)
    }
}

/// Capability file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityFile {
    /// File metadata (legacy format)
    pub metadata: Option<FileMetadata>,
    /// Tool definitions (legacy format)
    pub tools: Vec<ToolDefinition>,
    /// Enhanced file metadata (MCP 2025-06-18 format)
    pub enhanced_metadata: Option<EnhancedFileMetadata>,
    /// Enhanced tool definitions (MCP 2025-06-18 format)
    pub enhanced_tools: Option<Vec<EnhancedToolDefinition>>,
}

impl CapabilityFile {
    /// Create a new capability file
    pub fn new(tools: Vec<ToolDefinition>) -> Result<Self> {
        let file = Self {
            metadata: None,
            tools,
            enhanced_metadata: None,
            enhanced_tools: None,
        };
        file.validate()?;
        Ok(file)
    }

    /// Create a new capability file with metadata
    pub fn with_metadata(metadata: FileMetadata, tools: Vec<ToolDefinition>) -> Result<Self> {
        let file = Self {
            metadata: Some(metadata),
            tools,
            enhanced_metadata: None,
            enhanced_tools: None,
        };
        file.validate()?;
        Ok(file)
    }

    /// Validate the capability file
    pub fn validate(&self) -> Result<()> {
        // Validate metadata if present
        if let Some(ref metadata) = self.metadata {
            metadata.validate()?;
        }

        // Validate all tool definitions
        for tool_def in &self.tools {
            tool_def.validate()?;
        }

        // Check for duplicate tool names
        let mut tool_names = std::collections::HashSet::new();
        for tool_def in &self.tools {
            if !tool_names.insert(tool_def.name()) {
                return Err(crate::error::ProxyError::validation(
                    format!("Duplicate tool name: {}", tool_def.name())
                ));
            }
        }

        Ok(())
    }

    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.iter().find(|tool| tool.name() == name)
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.iter().map(|tool| tool.name()).collect()
    }

    /// Get tools by routing type
    pub fn tools_by_routing_type(&self, routing_type: &str) -> Vec<&ToolDefinition> {
        self.tools.iter()
            .filter(|tool| tool.routing_type() == routing_type)
            .collect()
    }

    /// Get safe tools only
    pub fn safe_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.iter()
            .filter(|tool| tool.is_safe())
            .collect()
    }

    /// Count tools
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Get visible tools only (not hidden)
    pub fn visible_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.iter()
            .filter(|tool| !tool.is_hidden())
            .collect()
    }

    /// Get hidden tools only
    pub fn hidden_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.iter()
            .filter(|tool| tool.is_hidden())
            .collect()
    }

    /// Count visible tools
    pub fn visible_tool_count(&self) -> usize {
        self.tools.iter().filter(|tool| !tool.is_hidden()).count()
    }

    /// Count hidden tools
    pub fn hidden_tool_count(&self) -> usize {
        self.tools.iter().filter(|tool| tool.is_hidden()).count()
    }

    /// Set all tools in this file to hidden/visible
    pub fn set_all_tools_hidden(&mut self, hidden: bool) {
        for tool in &mut self.tools {
            tool.set_hidden(hidden);
        }
    }

    /// Set specific tool hidden status by name
    pub fn set_tool_hidden(&mut self, tool_name: &str, hidden: bool) -> Result<()> {
        if let Some(tool) = self.tools.iter_mut().find(|t| t.name == tool_name) {
            tool.set_hidden(hidden);
            Ok(())
        } else {
            Err(crate::error::ProxyError::registry(format!("Tool '{}' not found in capability file", tool_name)))
        }
    }

    /// Get tool visibility status by name
    pub fn is_tool_hidden(&self, tool_name: &str) -> Option<bool> {
        self.tools.iter()
            .find(|t| t.name == tool_name)
            .map(|t| t.is_hidden())
    }

    /// Get enabled tools only
    pub fn enabled_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.iter()
            .filter(|tool| tool.is_enabled())
            .collect()
    }

    /// Get disabled tools only
    pub fn disabled_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.iter()
            .filter(|tool| !tool.is_enabled())
            .collect()
    }

    /// Count enabled tools
    pub fn enabled_tool_count(&self) -> usize {
        self.tools.iter().filter(|tool| tool.is_enabled()).count()
    }

    /// Count disabled tools
    pub fn disabled_tool_count(&self) -> usize {
        self.tools.iter().filter(|tool| !tool.is_enabled()).count()
    }

    /// Set all tools in this file to enabled/disabled
    pub fn set_all_tools_enabled(&mut self, enabled: bool) {
        for tool in &mut self.tools {
            tool.set_enabled(enabled);
        }
    }

    /// Set specific tool enabled status by name
    pub fn set_tool_enabled(&mut self, tool_name: &str, enabled: bool) -> Result<()> {
        if let Some(tool) = self.tools.iter_mut().find(|t| t.name == tool_name) {
            tool.set_enabled(enabled);
            Ok(())
        } else {
            Err(crate::error::ProxyError::registry(format!("Tool '{}' not found in capability file", tool_name)))
        }
    }

    /// Get tool enabled status by name
    pub fn is_tool_enabled(&self, tool_name: &str) -> Option<bool> {
        self.tools.iter()
            .find(|t| t.name == tool_name)
            .map(|t| t.is_enabled())
    }

    /// Get tools that are both visible and enabled (ready for normal use)
    pub fn active_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.iter()
            .filter(|tool| !tool.is_hidden() && tool.is_enabled())
            .collect()
    }

    /// Get tools that are enabled but hidden (smart discovery only)
    pub fn discoverable_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.iter()
            .filter(|tool| tool.is_hidden() && tool.is_enabled())
            .collect()
    }
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File name/identifier
    pub name: Option<String>,
    /// File description
    pub description: Option<String>,
    /// File version
    pub version: Option<String>,
    /// File author/team
    pub author: Option<String>,
    /// Tags for organization
    pub tags: Option<Vec<String>>,
}

impl FileMetadata {
    /// Create new metadata
    pub fn new() -> Self {
        Self {
            name: None,
            description: None,
            version: None,
            author: None,
            tags: None,
        }
    }

    /// Create metadata with name
    pub fn with_name(name: String) -> Self {
        Self {
            name: Some(name),
            description: None,
            version: None,
            author: None,
            tags: None,
        }
    }

    /// Set description
    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set version
    pub fn version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Set author
    pub fn author(mut self, author: String) -> Self {
        self.author = Some(author);
        self
    }

    /// Set tags
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Validate metadata
    pub fn validate(&self) -> Result<()> {
        // Validate version format if present
        if let Some(ref version) = self.version {
            if version.trim().is_empty() {
                return Err(crate::error::ProxyError::validation("Version cannot be empty"));
            }
        }

        // Validate tags if present
        if let Some(ref tags) = self.tags {
            for tag in tags {
                if tag.trim().is_empty() {
                    return Err(crate::error::ProxyError::validation("Tag cannot be empty"));
                }
            }
        }

        Ok(())
    }

    /// Check if metadata has required fields
    pub fn is_complete(&self) -> bool {
        self.name.is_some() && self.description.is_some()
    }
}

impl Default for FileMetadata {
    fn default() -> Self {
        Self::new()
    }
}

// Enhanced MCP 2025-06-18 Type Definitions
// These types support the full MCP 2025-06-18 specification with AI enhancement,
// security sandboxing, progress tracking, and comprehensive monitoring

/// Enhanced file metadata for MCP 2025-06-18 format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedFileMetadata {
    /// File name/identifier  
    pub name: String,
    /// File description
    pub description: String,
    /// File version
    pub version: String,
    /// File author/team
    pub author: String,
    /// Enhanced classification metadata
    pub classification: Option<ClassificationMetadata>,
    /// Discovery enhancement metadata
    pub discovery_metadata: Option<DiscoveryMetadata>,
    /// MCP 2025-06-18 capabilities
    pub mcp_capabilities: Option<McpCapabilities>,
}

/// Classification metadata for security and complexity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationMetadata {
    /// Security level classification
    pub security_level: String,
    /// Complexity level classification  
    pub complexity_level: String,
    /// Domain classification
    pub domain: String,
    /// Use cases
    pub use_cases: Vec<String>,
}

/// Discovery metadata for AI-enhanced discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryMetadata {
    /// Primary keywords for discovery
    pub primary_keywords: Vec<String>,
    /// Whether semantic embeddings are enabled
    pub semantic_embeddings: bool,
    /// Whether LLM enhancement is enabled
    pub llm_enhanced: bool,
    /// Whether workflow integration is enabled
    pub workflow_enabled: bool,
}

/// MCP 2025-06-18 capabilities specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapabilities {
    /// MCP specification version
    pub version: String,
    /// Whether cancellation is supported
    pub supports_cancellation: bool,
    /// Whether progress tracking is supported
    pub supports_progress: bool,
    /// Whether sampling is supported
    pub supports_sampling: bool,
    /// Whether validation is supported
    pub supports_validation: bool,
    /// Whether elicitation is supported
    pub supports_elicitation: bool,
}

/// Enhanced tool definition for MCP 2025-06-18 format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedToolDefinition {
    /// Tool name (unique identifier)
    pub name: String,
    /// Core tool definition
    pub core: CoreDefinition,
    /// Execution configuration
    pub execution: ExecutionConfig,
    /// Discovery enhancement
    pub discovery: DiscoveryEnhancement,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Access control configuration
    pub access: AccessConfig,
    /// Sampling strategy override for this specific tool
    pub sampling_strategy: Option<SamplingElicitationStrategy>,
    /// Elicitation strategy override for this specific tool
    pub elicitation_strategy: Option<SamplingElicitationStrategy>,
}

/// Core tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreDefinition {
    /// Human-readable description
    pub description: String,
    /// JSON Schema for input parameters
    pub input_schema: serde_json::Value,
}

/// Execution configuration with security and performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Enhanced routing configuration
    pub routing: EnhancedRoutingConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
}

/// Enhanced routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedRoutingConfig {
    /// Routing type
    pub r#type: String,
    /// Primary routing configuration
    pub primary: Option<serde_json::Value>,
    /// Fallback routing configuration
    pub fallback: Option<serde_json::Value>,
    /// Additional routing configuration
    pub config: Option<serde_json::Value>,
}

/// Security configuration with sandboxing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Security classification
    pub classification: String,
    /// Sandbox configuration
    pub sandbox: Option<SandboxConfig>,
    /// Whether approval is required
    pub requires_approval: Option<bool>,
    /// Approval workflow
    pub approval_workflow: Option<String>,
}

/// Sandbox configuration for security isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Resource limits
    pub resources: Option<ResourceLimits>,
    /// Filesystem restrictions
    pub filesystem: Option<FilesystemRestrictions>,
    /// Network restrictions
    pub network: Option<NetworkRestrictions>,
    /// Environment restrictions
    pub environment: Option<EnvironmentRestrictions>,
}

/// Resource limits for sandboxing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in MB
    pub max_memory_mb: Option<u64>,
    /// Maximum CPU percentage
    pub max_cpu_percent: Option<u64>,
    /// Maximum execution seconds
    pub max_execution_seconds: Option<u64>,
    /// Maximum file descriptors
    pub max_file_descriptors: Option<u64>,
}

/// Filesystem restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemRestrictions {
    /// Allowed read paths
    pub allowed_read_paths: Option<Vec<String>>,
    /// Allowed write paths  
    pub allowed_write_paths: Option<Vec<String>>,
    /// Denied read patterns
    pub denied_read_patterns: Option<Vec<String>>,
    /// Denied write patterns
    pub denied_write_patterns: Option<Vec<String>>,
}

/// Network restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRestrictions {
    /// Whether network access is allowed
    pub allowed: bool,
    /// Allowed domains
    pub allowed_domains: Option<Vec<String>>,
    /// Denied domains
    pub denied_domains: Option<Vec<String>>,
}

/// Environment restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentRestrictions {
    /// Whether system is readonly
    pub readonly_system: Option<bool>,
    /// Environment variables
    pub env_vars: Option<serde_json::Value>,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Estimated duration for different operation types
    pub estimated_duration: Option<serde_json::Value>,
    /// Complexity level
    pub complexity: Option<String>,
    /// Whether cancellation is supported
    pub supports_cancellation: Option<bool>,
    /// Whether progress tracking is supported
    pub supports_progress: Option<bool>,
    /// Whether results should be cached
    pub cache_results: Option<bool>,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: Option<u64>,
    /// Whether adaptive optimization is enabled
    pub adaptive_optimization: Option<bool>,
}

/// Discovery enhancement configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryEnhancement {
    /// AI-enhanced discovery metadata
    pub ai_enhanced: Option<AiEnhancedDiscovery>,
    /// Parameter intelligence
    pub parameter_intelligence: Option<serde_json::Value>,
}

/// AI-enhanced discovery metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEnhancedDiscovery {
    /// Enhanced description
    pub description: Option<String>,
    /// Usage patterns
    pub usage_patterns: Option<Vec<String>>,
    /// Semantic context
    pub semantic_context: Option<SemanticContext>,
    /// AI capabilities
    pub ai_capabilities: Option<serde_json::Value>,
    /// Workflow integration
    pub workflow_integration: Option<WorkflowIntegration>,
}

/// Semantic context for AI understanding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticContext {
    /// Primary intent
    pub primary_intent: Option<String>,
    /// Data types
    pub data_types: Option<Vec<String>>,
    /// Operations
    pub operations: Option<Vec<String>>,
    /// Security features
    pub security_features: Option<Vec<String>>,
}

/// Workflow integration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowIntegration {
    /// Tools that typically follow this one
    pub typically_follows: Option<Vec<String>>,
    /// Tools that typically precede this one
    pub typically_precedes: Option<Vec<String>>,
    /// Chain compatibility
    pub chain_compatibility: Option<Vec<String>>,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Progress tracking configuration
    pub progress_tracking: Option<ProgressTrackingConfig>,
    /// Cancellation configuration
    pub cancellation: Option<CancellationConfig>,
    /// Metrics configuration
    pub metrics: Option<MetricsConfig>,
}

/// Progress tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressTrackingConfig {
    /// Whether progress tracking is enabled
    pub enabled: bool,
    /// Progress granularity
    pub granularity: Option<String>,
    /// Sub-operations for detailed progress
    pub sub_operations: Option<Vec<SubOperation>>,
}

/// Sub-operation for progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubOperation {
    /// Sub-operation ID
    pub id: String,
    /// Sub-operation name
    pub name: String,
    /// Estimated percentage of total work
    pub estimated_percentage: u8,
}

/// Cancellation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationConfig {
    /// Whether cancellation is enabled
    pub enabled: bool,
    /// Graceful timeout in seconds
    pub graceful_timeout_seconds: Option<u64>,
    /// Whether cleanup is required
    pub cleanup_required: Option<bool>,
    /// Cleanup operations
    pub cleanup_operations: Option<Vec<String>>,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether to track execution time
    pub track_execution_time: Option<bool>,
    /// Whether to track success rate
    pub track_success_rate: Option<bool>,
    /// Custom metrics to track
    pub custom_metrics: Option<Vec<String>>,
}

/// Access control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessConfig {
    /// Whether this tool should be hidden from main tool lists
    pub hidden: bool,
    /// Whether this tool is enabled for execution
    pub enabled: bool,
    /// Required permissions
    pub requires_permissions: Option<Vec<String>>,
    /// Allowed user groups
    pub user_groups: Option<Vec<String>>,
    /// Whether approval is required
    pub approval_required: Option<bool>,
    /// Whether to track usage analytics
    pub usage_analytics: Option<bool>,
}

// Enhanced MCP 2025-06-18 implementations
impl EnhancedFileMetadata {
    /// Create new enhanced metadata
    pub fn new(name: String, description: String, version: String, author: String) -> Self {
        Self {
            name,
            description,
            version,
            author,
            classification: None,
            discovery_metadata: None,
            mcp_capabilities: None,
        }
    }

    /// Validate enhanced metadata
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(ProxyError::validation("Enhanced metadata name cannot be empty"));
        }
        if self.description.trim().is_empty() {
            return Err(ProxyError::validation("Enhanced metadata description cannot be empty"));
        }
        if self.version.trim().is_empty() {
            return Err(ProxyError::validation("Enhanced metadata version cannot be empty"));
        }
        if self.author.trim().is_empty() {
            return Err(ProxyError::validation("Enhanced metadata author cannot be empty"));
        }
        Ok(())
    }
}

impl EnhancedToolDefinition {
    /// Create a new enhanced tool definition
    pub fn new(
        name: String,
        core: CoreDefinition,
        execution: ExecutionConfig,
        discovery: DiscoveryEnhancement,
        monitoring: MonitoringConfig,
        access: AccessConfig,
    ) -> Result<Self> {
        let tool = Self {
            name,
            core,
            execution,
            discovery,
            monitoring,
            access,
            sampling_strategy: None, // Use server default
            elicitation_strategy: None, // Use server default
        };
        tool.validate()?;
        Ok(tool)
    }

    /// Validate the enhanced tool definition
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(ProxyError::validation("Enhanced tool name cannot be empty"));
        }
        if self.core.description.trim().is_empty() {
            return Err(ProxyError::validation("Enhanced tool description cannot be empty"));
        }
        if !self.core.input_schema.is_object() {
            return Err(ProxyError::validation("Enhanced tool input schema must be a JSON object"));
        }
        Ok(())
    }

    /// Convert to legacy ToolDefinition for compatibility
    pub fn to_legacy_tool(&self) -> Result<ToolDefinition> {
        // Create a basic routing config from the enhanced execution config
        let routing_config = RoutingConfig::new(
            self.execution.routing.r#type.clone(),
            self.execution.routing.config.clone().unwrap_or_else(|| serde_json::json!({})),
        );

        ToolDefinition::new_with_all_fields(
            self.name.clone(),
            self.core.description.clone(),
            self.core.input_schema.clone(),
            routing_config,
            None, // annotations
            self.access.hidden,
            self.access.enabled,
        )
    }
}

impl From<&EnhancedToolDefinition> for ToolDefinition {
    fn from(enhanced: &EnhancedToolDefinition) -> Self {
        enhanced.to_legacy_tool().unwrap_or_else(|_| {
            // Fallback with minimal configuration
            ToolDefinition {
                name: enhanced.name.clone(),
                description: enhanced.core.description.clone(),
                input_schema: enhanced.core.input_schema.clone(),
                routing: RoutingConfig::new("enhanced".to_string(), serde_json::json!({})),
                annotations: None,
                hidden: enhanced.access.hidden,
                enabled: enhanced.access.enabled,
                prompt_refs: Vec::new(), // No prompts initially
                resource_refs: Vec::new(), // No resources initially
                sampling_strategy: enhanced.sampling_strategy.clone(),
                elicitation_strategy: enhanced.elicitation_strategy.clone(),
            }
        })
    }
}

// Enhanced CapabilityFile methods
impl CapabilityFile {
    /// Create a new enhanced capability file (MCP 2025-06-18 format)
    pub fn new_enhanced(metadata: EnhancedFileMetadata, tools: Vec<EnhancedToolDefinition>) -> Result<Self> {
        let file = Self {
            metadata: None,
            enhanced_metadata: Some(metadata),
            tools: Vec::new(), // No legacy tools in enhanced format
            enhanced_tools: Some(tools),
        };
        file.validate_enhanced()?;
        Ok(file)
    }


    /// Get effective tools (enhanced or legacy)
    pub fn effective_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.iter().collect()
    }

    /// Get enhanced tools if available
    pub fn get_enhanced_tools(&self) -> Option<&Vec<EnhancedToolDefinition>> {
        self.enhanced_tools.as_ref()
    }

    /// Validate enhanced capability file
    pub fn validate_enhanced(&self) -> Result<()> {
        // Validate enhanced metadata if present
        if let Some(ref enhanced_metadata) = self.enhanced_metadata {
            enhanced_metadata.validate()?;
        }

        // Validate enhanced tool definitions if present
        if let Some(ref enhanced_tools) = self.enhanced_tools {
            for tool_def in enhanced_tools {
                tool_def.validate()?;
            }

            // Check for duplicate tool names in enhanced tools
            let mut enhanced_tool_names = std::collections::HashSet::new();
            for tool_def in enhanced_tools {
                if !enhanced_tool_names.insert(&tool_def.name) {
                    return Err(crate::error::ProxyError::validation(
                        format!("Duplicate enhanced tool name: {}", tool_def.name)
                    ));
                }
            }

            if enhanced_tools.is_empty() {
                return Err(crate::error::ProxyError::validation(
                    "Enhanced capability file must contain at least one tool".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Count enhanced tools only
    pub fn enhanced_tool_count(&self) -> usize {
        self.enhanced_tools.as_ref().map_or(0, |t| t.len())
    }

    /// Get enhanced tool by name
    pub fn get_enhanced_tool(&self, name: &str) -> Option<&EnhancedToolDefinition> {
        self.enhanced_tools.as_ref()?.iter().find(|tool| tool.name == name)
    }
}

/// Reference to a generated prompt template stored separately
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptReference {
    /// Unique prompt ID/name
    pub name: String,
    /// Prompt type (usage, parameter_validation, troubleshooting, etc.)
    pub prompt_type: String,
    /// Short description of the prompt
    pub description: Option<String>,
    /// Storage location/path for the prompt content
    pub storage_path: Option<String>,
    /// Generation metadata
    pub generation_metadata: Option<GenerationReferenceMetadata>,
}

/// Reference to a generated resource stored separately  
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceReference {
    /// Unique resource ID/name
    pub name: String,
    /// Resource type (documentation, examples, schema, configuration, etc.)
    pub resource_type: String,
    /// Resource URI for MCP clients
    pub uri: String,
    /// MIME type of the resource
    pub mime_type: Option<String>,
    /// Short description of the resource
    pub description: Option<String>,
    /// Storage location/path for the resource content
    pub storage_path: Option<String>,
    /// Generation metadata
    pub generation_metadata: Option<GenerationReferenceMetadata>,
}

/// Metadata for generated content references
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationReferenceMetadata {
    /// Model used for generation
    pub model_used: Option<String>,
    /// Confidence score of the generation
    pub confidence_score: Option<f32>,
    /// Generation timestamp
    pub generated_at: Option<String>,
    /// Generation time in milliseconds
    pub generation_time_ms: Option<u64>,
    /// Version of the content
    pub version: Option<String>,
    /// Whether this is from external MCP server
    pub external_source: Option<String>,
}

/// Raw enhanced capability file format for parsing (before conversion to legacy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedCapabilityFileRaw {
    /// Enhanced file metadata (unified structure)
    pub metadata: Option<EnhancedFileMetadata>,
    /// Enhanced tool definitions
    pub tools: Vec<EnhancedToolDefinitionRaw>,
}

/// Raw enhanced tool definition for parsing (before conversion to legacy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedToolDefinitionRaw {
    /// Tool name (unique identifier)
    pub name: String,
    /// Core tool definition
    pub core: CoreDefinition,
    /// Execution configuration (optional)
    pub execution: Option<ExecutionConfigRaw>,
    /// Discovery enhancement (optional)
    pub discovery: Option<DiscoveryEnhancement>,
    /// Monitoring configuration (optional)
    pub monitoring: Option<MonitoringConfig>,
    /// Access control configuration (optional)
    pub access: Option<AccessConfigRaw>,
    /// Routing configuration (optional, will use default if not specified)
    pub routing: Option<RoutingConfig>,
    /// Optional annotations for metadata
    pub annotations: Option<std::collections::HashMap<String, String>>,
    /// Whether this tool should be hidden from main tool lists
    pub hidden: Option<bool>,
    /// Whether this tool is enabled for routing and execution
    pub enabled: Option<bool>,
    /// Generated prompt template references for this tool
    pub prompt_refs: Option<Vec<PromptReference>>,
    /// Generated resource references for this tool
    pub resource_refs: Option<Vec<ResourceReference>>,
    /// Sampling strategy override for this specific tool
    pub sampling_strategy: Option<SamplingElicitationStrategy>,
    /// Elicitation strategy override for this specific tool
    pub elicitation_strategy: Option<SamplingElicitationStrategy>,
}

/// Raw execution configuration for parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfigRaw {
    /// Enhanced routing configuration
    pub routing: Option<EnhancedRoutingConfigRaw>,
    /// Security configuration
    pub security: Option<serde_json::Value>,
    /// Performance configuration
    pub performance: Option<serde_json::Value>,
}

/// Raw enhanced routing configuration for parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedRoutingConfigRaw {
    /// Routing type
    pub r#type: String,
    /// Primary routing configuration
    pub primary: Option<serde_json::Value>,
    /// Fallback routing configuration
    pub fallback: Option<serde_json::Value>,
    /// Additional routing configuration
    pub config: Option<serde_json::Value>,
}

/// Raw access configuration for parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessConfigRaw {
    /// Whether this tool should be hidden from main tool lists
    pub hidden: Option<bool>,
    /// Whether this tool is enabled for routing and execution
    pub enabled: Option<bool>,
    /// Required permissions
    pub requires_permissions: Option<Vec<String>>,
    /// User groups with access
    pub user_groups: Option<Vec<String>>,
    /// Whether approval is required
    pub approval_required: Option<bool>,
}

// ================================================================================================
// Custom Validation Extensions
// ================================================================================================

/// Custom validation extensions for MagicTunnel capability schemas
/// These extend standard JSON Schema with domain-specific validation rules
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ValidationExtensions {
    // Range validation
    /// Optimal range for numeric values [min, max]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimal_range: Option<[f64; 2]>,
    
    // Security validation
    /// Enable privacy scanning for content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_scan: Option<bool>,
    /// Enable content filtering for malicious content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_filter: Option<bool>,
    /// Enable injection protection for user input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injection_protection: Option<bool>,
    /// Enable semantic analysis of content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_analysis: Option<bool>,
    /// Enable path traversal protection for file paths
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_traversal_protection: Option<bool>,
    /// Enable general security scanning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_scan: Option<bool>,
    /// Enable relevance checking for context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance_check: Option<bool>,
    
    // Tool validation
    /// Verify tool accessibility before use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_accessible: Option<bool>,
    /// Verify tool existence before use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_exists: Option<bool>,
    
    // Size validation
    /// Maximum size in megabytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size_mb: Option<u64>,
    
    // Rule-based validation
    /// List of validation rules with messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<ValidationRule>>,
}

/// A validation rule with a specific rule type and error message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationRule {
    /// Rule type (e.g., "required_validation", "format_validation")
    pub rule: String,
    /// Error message to display when validation fails
    pub message: String,
}

impl ValidationExtensions {
    /// Create a new empty ValidationExtensions
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create ValidationExtensions with security validation enabled
    pub fn with_security() -> Self {
        Self {
            privacy_scan: Some(true),
            content_filter: Some(true),
            injection_protection: Some(true),
            security_scan: Some(true),
            ..Default::default()
        }
    }
    
    /// Create ValidationExtensions with file path validation
    pub fn with_file_path_validation() -> Self {
        Self {
            path_traversal_protection: Some(true),
            security_scan: Some(true),
            ..Default::default()
        }
    }
    
    /// Create ValidationExtensions with tool validation
    pub fn with_tool_validation() -> Self {
        Self {
            tool_accessible: Some(true),
            tool_exists: Some(true),
            ..Default::default()
        }
    }
    
    /// Create ValidationExtensions with range validation
    pub fn with_range_validation(min: f64, max: f64) -> Self {
        Self {
            optimal_range: Some([min, max]),
            ..Default::default()
        }
    }
    
    /// Check if any validation extensions are enabled
    pub fn has_validations(&self) -> bool {
        self.optimal_range.is_some() ||
        self.privacy_scan.is_some() ||
        self.content_filter.is_some() ||
        self.injection_protection.is_some() ||
        self.semantic_analysis.is_some() ||
        self.path_traversal_protection.is_some() ||
        self.security_scan.is_some() ||
        self.relevance_check.is_some() ||
        self.tool_accessible.is_some() ||
        self.tool_exists.is_some() ||
        self.max_size_mb.is_some() ||
        self.rules.is_some()
    }
}

/// Utility functions for working with validation extensions in JSON Schema
impl ValidationExtensions {
    /// Extract validation extensions from a JSON Schema value
    pub fn from_schema(schema: &Value) -> Option<Self> {
        schema.get("validation")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    
    /// Extract validation extensions from x-validation property (JSON Schema convention)
    pub fn from_x_validation(schema: &Value) -> Option<Self> {
        schema.get("x-validation")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    
    /// Inject validation extensions into a JSON Schema as x-validation property
    pub fn inject_into_schema(&self, schema: &mut Value) {
        if self.has_validations() {
            if let Some(obj) = schema.as_object_mut() {
                obj.insert("x-validation".to_string(), serde_json::to_value(self).unwrap());
            }
        }
    }
    
    /// Inject validation extensions into a JSON Schema as validation property (MagicTunnel convention)
    pub fn inject_as_validation(&self, schema: &mut Value) {
        if self.has_validations() {
            if let Some(obj) = schema.as_object_mut() {
                obj.insert("validation".to_string(), serde_json::to_value(self).unwrap());
            }
        }
    }
}
