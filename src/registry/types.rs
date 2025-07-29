//! Registry types and structures

use crate::mcp::Tool;
use crate::mcp::types::ToolAnnotations;
use crate::error::{ProxyError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
        matches!(self.r#type.as_str(), "subprocess" | "http" | "llm" | "websocket" | "database" | "external_mcp")
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
    /// File metadata
    pub metadata: Option<FileMetadata>,
    /// Tool definitions
    pub tools: Vec<ToolDefinition>,
}

impl CapabilityFile {
    /// Create a new capability file
    pub fn new(tools: Vec<ToolDefinition>) -> Result<Self> {
        let file = Self {
            metadata: None,
            tools,
        };
        file.validate()?;
        Ok(file)
    }

    /// Create a new capability file with metadata
    pub fn with_metadata(metadata: FileMetadata, tools: Vec<ToolDefinition>) -> Result<Self> {
        let file = Self {
            metadata: Some(metadata),
            tools,
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
