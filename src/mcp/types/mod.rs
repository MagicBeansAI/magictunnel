//! MCP types and structures
//!
//! This module contains all type definitions for MCP protocol implementation

use serde::{Deserialize, Serialize};
use serde_json::Value;
use jsonschema::{JSONSchema, ValidationError};
use crate::error::Result;
use crate::mcp::errors::McpError;

pub mod sampling;
pub mod elicitation;
pub mod roots;
pub mod tool_enhancement;
pub mod capabilities;

// Re-export commonly used types for convenience
pub use sampling::*;
pub use elicitation::*;
pub use roots::*;
pub use tool_enhancement::*;
// Re-export client capabilities with distinct names to avoid conflicts
pub use capabilities::{
    ClientCapabilities, 
    ToolsCapability as ClientToolsCapability,
    ResourcesCapability as ClientResourcesCapability,
    PromptsCapability as ClientPromptsCapability,
    SamplingCapability as ClientSamplingCapability,
    ElicitationCapability as ClientElicitationCapability,
};

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name (unique identifier)
    pub name: String,
    /// Human-readable description (optional for compatibility)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Optional human-readable title (MCP enhancement)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// JSON Schema for input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    /// Optional JSON Schema for output validation (MCP enhancement)
    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    /// Optional MCP annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
}

impl Tool {
    /// Create a new Tool with validation
    pub fn new(name: String, description: String, input_schema: Value) -> Result<Self> {
        let tool = Tool {
            name: name.clone(),
            description: Some(description),
            title: None,
            input_schema,
            output_schema: None,
            annotations: None,
        };

        tool.validate()?;
        Ok(tool)
    }

    /// Create a new Tool with annotations
    pub fn with_annotations(
        name: String,
        description: String,
        input_schema: Value,
        annotations: ToolAnnotations
    ) -> Result<Self> {
        let tool = Tool {
            name: name.clone(),
            description: Some(description),
            title: None,
            input_schema,
            output_schema: None,
            annotations: Some(annotations),
        };

        tool.validate()?;
        Ok(tool)
    }

    /// Validate the tool definition
    pub fn validate(&self) -> Result<()> {
        // Validate tool name
        if self.name.trim().is_empty() {
            return Err(crate::error::ProxyError::validation("Tool name cannot be empty"));
        }

        // Validate description (optional but if present, should not be empty)
        if let Some(ref desc) = self.description {
            if desc.trim().is_empty() {
                return Err(crate::error::ProxyError::validation("Tool description cannot be empty"));
            }
        }

        // Validate input schema is a valid JSON Schema
        self.validate_input_schema()?;

        Ok(())
    }

    /// Validate that the input schema is a valid JSON Schema
    pub fn validate_input_schema(&self) -> Result<()> {
        match JSONSchema::compile(&self.input_schema) {
            Ok(_) => Ok(()),
            Err(e) => Err(crate::error::ProxyError::validation(
                format!("Invalid JSON Schema for tool '{}': {}", self.name, e)
            )),
        }
    }

    /// Validate arguments against the input schema
    pub fn validate_arguments(&self, arguments: &Value) -> Result<()> {
        let schema = JSONSchema::compile(&self.input_schema)
            .map_err(|e| crate::error::ProxyError::validation(
                format!("Failed to compile schema for tool '{}': {}", self.name, e)
            ))?;

        let validation_result = schema.validate(arguments);
        match validation_result {
            Ok(_) => Ok(()),
            Err(errors) => {
                let error_messages: Vec<String> = errors
                    .map(|e| format!("  - {}", e))
                    .collect();
                Err(crate::error::ProxyError::validation(
                    format!("Invalid arguments for tool '{}': \n{}",
                        self.name,
                        error_messages.join("\n")
                    )
                ))
            }
        }
    }

    /// Check if the tool is MCP compliant
    pub fn is_mcp_compliant(&self) -> bool {
        // Basic MCP compliance checks
        !self.name.is_empty() &&
        self.description.as_ref().map_or(true, |d| !d.is_empty()) &&
        self.validate_input_schema().is_ok()
    }
}

/// MCP Tool annotations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnnotations {
    /// Display title
    pub title: Option<String>,
    /// Indicates if tool is read-only
    #[serde(rename = "readOnlyHint")]
    pub read_only_hint: Option<bool>,
    /// Indicates if tool is destructive
    #[serde(rename = "destructiveHint")]
    pub destructive_hint: Option<bool>,
    /// Indicates if tool is idempotent
    #[serde(rename = "idempotentHint")]
    pub idempotent_hint: Option<bool>,
    /// Indicates if tool has open-world semantics
    #[serde(rename = "openWorldHint")]
    pub open_world_hint: Option<bool>,
    /// Enhanced sampling metadata (MCP 2025-06-18)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
    /// Enhanced elicitation metadata (MCP 2025-06-18)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation: Option<ElicitationCapability>,
}

impl ToolAnnotations {
    /// Create new annotations with default values
    pub fn new() -> Self {
        Self {
            title: None,
            read_only_hint: None,
            destructive_hint: None,
            idempotent_hint: None,
            open_world_hint: None,
            sampling: None,
            elicitation: None,
        }
    }

    /// Create annotations with title
    pub fn with_title(title: String) -> Self {
        Self {
            title: Some(title),
            read_only_hint: None,
            destructive_hint: None,
            idempotent_hint: None,
            open_world_hint: None,
            sampling: None,
            elicitation: None,
        }
    }

    /// Set read-only hint
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only_hint = Some(read_only);
        self
    }

    /// Set destructive hint
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive_hint = Some(destructive);
        self
    }

    /// Set idempotent hint
    pub fn idempotent(mut self, idempotent: bool) -> Self {
        self.idempotent_hint = Some(idempotent);
        self
    }

    /// Set open-world hint
    pub fn open_world(mut self, open_world: bool) -> Self {
        self.open_world_hint = Some(open_world);
        self
    }

    /// Validate annotations for consistency
    pub fn validate(&self) -> Result<()> {
        // Check for conflicting hints
        if let (Some(true), Some(true)) = (self.read_only_hint, self.destructive_hint) {
            return Err(crate::error::ProxyError::validation(
                "Tool cannot be both read-only and destructive"
            ));
        }

        Ok(())
    }

    /// Check if tool is safe to execute
    pub fn is_safe(&self) -> bool {
        // Tool is considered safe if it's read-only or explicitly not destructive
        self.read_only_hint.unwrap_or(false) ||
        !self.destructive_hint.unwrap_or(false)
    }
}

impl Default for ToolAnnotations {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for MCP compliance - same as ToolAnnotations
pub type MCPAnnotations = ToolAnnotations;

/// MCP 2025-06-18: Sampling capability metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapability {
    /// Whether the tool supports enhanced description sampling
    pub supports_description_enhancement: bool,
    /// Pre-generated enhanced description from external MCP server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhanced_description: Option<String>,
    /// Model used for generating the enhanced description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_used: Option<String>,
    /// Confidence score for the enhancement (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_score: Option<f64>,
    /// When the enhancement was generated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// MCP 2025-06-18: Elicitation capability metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationCapability {
    /// Whether the tool supports parameter elicitation
    pub supports_parameter_elicitation: bool,
    /// Pre-generated enhanced keywords from external MCP server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhanced_keywords: Option<Vec<String>>,
    /// Pre-generated usage patterns from external MCP server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_patterns: Option<Vec<String>>,
    /// Pre-generated parameter examples from external MCP server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_examples: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// When the elicitation metadata was generated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource URI (unique identifier)
    pub uri: String,
    /// Human-readable name
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// MIME type of the resource content
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    /// Optional resource annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ResourceAnnotations>,
}

impl Resource {
    /// Create a new resource
    pub fn new(uri: String, name: String) -> Self {
        Self {
            uri,
            name,
            description: None,
            mime_type: None,
            annotations: None,
        }
    }

    /// Create a resource with description
    pub fn with_description(uri: String, name: String, description: String) -> Self {
        Self {
            uri,
            name,
            description: Some(description),
            mime_type: None,
            annotations: None,
        }
    }

    /// Create a resource with MIME type
    pub fn with_mime_type(uri: String, name: String, mime_type: String) -> Self {
        Self {
            uri,
            name,
            description: None,
            mime_type: Some(mime_type),
            annotations: None,
        }
    }

    /// Create a complete resource with all fields
    pub fn complete(
        uri: String,
        name: String,
        description: Option<String>,
        mime_type: Option<String>,
        annotations: Option<ResourceAnnotations>,
    ) -> Self {
        Self {
            uri,
            name,
            description,
            mime_type,
            annotations,
        }
    }

    /// Validate the resource
    pub fn validate(&self) -> Result<()> {
        if self.uri.is_empty() {
            return Err(crate::error::ProxyError::validation("Resource URI cannot be empty".to_string()));
        }
        if self.name.is_empty() {
            return Err(crate::error::ProxyError::validation("Resource name cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// MCP Resource annotations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAnnotations {
    /// Display title
    pub title: Option<String>,
    /// Indicates if resource is read-only (default: true for MCP resources)
    #[serde(rename = "readOnlyHint")]
    pub read_only_hint: Option<bool>,
    /// Resource size in bytes (if known)
    pub size: Option<u64>,
    /// Last modified timestamp (ISO 8601)
    #[serde(rename = "lastModified")]
    pub last_modified: Option<String>,
    /// Resource tags for categorization
    pub tags: Option<Vec<String>>,
}

impl ResourceAnnotations {
    /// Create new empty resource annotations
    pub fn new() -> Self {
        Self {
            title: None,
            read_only_hint: Some(true), // MCP resources are read-only by default
            size: None,
            last_modified: None,
            tags: None,
        }
    }

    /// Create resource annotations with title
    pub fn with_title(title: String) -> Self {
        Self {
            title: Some(title),
            read_only_hint: Some(true),
            size: None,
            last_modified: None,
            tags: None,
        }
    }

    /// Set read-only hint
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only_hint = Some(read_only);
        self
    }

    /// Set resource size
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    /// Set last modified timestamp
    pub fn with_last_modified(mut self, timestamp: String) -> Self {
        self.last_modified = Some(timestamp);
        self
    }

    /// Set resource tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Check if resource is read-only
    pub fn is_read_only(&self) -> bool {
        self.read_only_hint.unwrap_or(true) // Default to read-only for MCP resources
    }
}

impl Default for ResourceAnnotations {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource content returned by resource read operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    /// Resource URI
    pub uri: String,
    /// MIME type of the content
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    /// Resource content (text or base64-encoded binary)
    pub text: Option<String>,
    /// Base64-encoded binary content (alternative to text)
    pub blob: Option<String>,
}

impl ResourceContent {
    /// Create text resource content
    pub fn text(uri: String, content: String, mime_type: Option<String>) -> Self {
        Self {
            uri,
            mime_type,
            text: Some(content),
            blob: None,
        }
    }

    /// Create binary resource content (base64-encoded)
    pub fn blob(uri: String, content: Vec<u8>, mime_type: Option<String>) -> Self {
        use base64::{Engine as _, engine::general_purpose};
        let blob = general_purpose::STANDARD.encode(content);
        Self {
            uri,
            mime_type,
            text: None,
            blob: Some(blob),
        }
    }

    /// Check if content is text
    pub fn is_text(&self) -> bool {
        self.text.is_some()
    }

    /// Check if content is binary
    pub fn is_blob(&self) -> bool {
        self.blob.is_some()
    }

    /// Get content size in bytes
    pub fn size(&self) -> usize {
        if let Some(text) = &self.text {
            text.len()
        } else if let Some(blob) = &self.blob {
            // Approximate size from base64 (actual size is ~3/4 of base64 length)
            (blob.len() * 3) / 4
        } else {
            0
        }
    }
}

/// Resource list request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListRequest {
    /// Optional cursor for pagination
    pub cursor: Option<String>,
}

/// Resource list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListResponse {
    /// List of available resources
    pub resources: Vec<Resource>,
    /// Next cursor for pagination (if more resources available)
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// Resource read request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadRequest {
    /// Resource URI to read
    pub uri: String,
}

/// Resource read response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadResponse {
    /// Resource contents
    pub contents: Vec<ResourceContent>,
}

/// Tool list request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListRequest {
    /// Optional cursor for pagination
    pub cursor: Option<String>,
}

/// Tool list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListResponse {
    /// List of available tools
    pub tools: Vec<Tool>,
    /// Next cursor for pagination (if more tools available)
    pub next_cursor: Option<String>,
}

/// Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name to call
    pub name: String,
    /// Arguments for the tool
    pub arguments: Value,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(name: String, arguments: Value) -> Self {
        Self { name, arguments }
    }

    /// Validate the tool call
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(crate::error::ProxyError::validation("Tool call name cannot be empty"));
        }

        Ok(())
    }

    /// Validate arguments against a tool definition
    pub fn validate_against_tool(&self, tool: &Tool) -> Result<()> {
        if self.name != tool.name {
            return Err(crate::error::ProxyError::validation(
                format!("Tool call name '{}' does not match tool definition '{}'",
                    self.name, tool.name)
            ));
        }

        tool.validate_arguments(&self.arguments)
    }
}

/// MCP-compliant content types for tool results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// Text content
        text: String,
    },
    /// Image content (base64 encoded)
    #[serde(rename = "image")]
    Image {
        /// Base64 encoded image data
        data: String,
        /// MIME type (e.g., "image/png")
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    /// Resource link content
    #[serde(rename = "resource")]
    Resource {
        /// Resource URI
        uri: String,
        /// Optional resource text content
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// Optional MIME type
        #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
}

impl ToolContent {
    /// Create text content
    pub fn text(text: String) -> Self {
        Self::Text { text }
    }

    /// Create image content with validation
    pub fn image(data: String, mime_type: String) -> crate::error::Result<Self> {
        // Validate MIME type
        Self::validate_image_mime_type(&mime_type)?;

        // Validate base64 encoding
        Self::validate_base64_data(&data)?;

        Ok(Self::Image { data, mime_type })
    }

    /// Create image content without validation (for internal use)
    pub fn image_unchecked(data: String, mime_type: String) -> Self {
        Self::Image { data, mime_type }
    }

    /// Create resource content with validation
    pub fn resource(uri: String) -> crate::error::Result<Self> {
        // Validate URI
        Self::validate_uri(&uri)?;

        Ok(Self::Resource { uri, text: None, mime_type: None })
    }

    /// Create resource content without validation (for internal use)
    pub fn resource_unchecked(uri: String) -> Self {
        Self::Resource { uri, text: None, mime_type: None }
    }

    /// Create resource content with text and validation
    pub fn resource_with_text(uri: String, text: String, mime_type: Option<String>) -> crate::error::Result<Self> {
        // Validate URI
        Self::validate_uri(&uri)?;

        // Validate MIME type if provided
        if let Some(ref mime) = mime_type {
            Self::validate_mime_type(mime)?;
        }

        Ok(Self::Resource { uri, text: Some(text), mime_type })
    }

    /// Create resource content with text without validation (for internal use)
    pub fn resource_with_text_unchecked(uri: String, text: String, mime_type: Option<String>) -> Self {
        Self::Resource { uri, text: Some(text), mime_type }
    }

    /// Validate the content according to its type
    pub fn validate(&self) -> crate::error::Result<()> {
        match self {
            Self::Text { text } => {
                if text.is_empty() {
                    return Err(crate::error::ProxyError::validation("Text content cannot be empty"));
                }
                Ok(())
            }
            Self::Image { data, mime_type } => {
                Self::validate_image_mime_type(mime_type)?;
                Self::validate_base64_data(data)?;
                Ok(())
            }
            Self::Resource { uri, mime_type, .. } => {
                Self::validate_uri(uri)?;
                if let Some(mime) = mime_type {
                    Self::validate_mime_type(mime)?;
                }
                Ok(())
            }
        }
    }

    /// Validate image MIME type
    fn validate_image_mime_type(mime_type: &str) -> crate::error::Result<()> {
        const VALID_IMAGE_TYPES: &[&str] = &[
            "image/png",
            "image/jpeg",
            "image/jpg",
            "image/gif",
            "image/webp",
            "image/svg+xml",
            "image/bmp",
            "image/tiff",
        ];

        if !VALID_IMAGE_TYPES.contains(&mime_type) {
            return Err(crate::error::ProxyError::validation(
                format!("Invalid image MIME type: {}. Supported types: {}",
                    mime_type,
                    VALID_IMAGE_TYPES.join(", ")
                )
            ));
        }

        Ok(())
    }

    /// Validate general MIME type format
    fn validate_mime_type(mime_type: &str) -> crate::error::Result<()> {
        // Basic MIME type validation: type/subtype
        if !mime_type.contains('/') {
            return Err(crate::error::ProxyError::validation(
                format!("Invalid MIME type format: {}. Expected format: type/subtype", mime_type)
            ));
        }

        let parts: Vec<&str> = mime_type.split('/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(crate::error::ProxyError::validation(
                format!("Invalid MIME type format: {}. Expected format: type/subtype", mime_type)
            ));
        }

        Ok(())
    }

    /// Validate base64 encoded data
    fn validate_base64_data(data: &str) -> crate::error::Result<()> {
        use base64::{Engine as _, engine::general_purpose};

        if data.is_empty() {
            return Err(crate::error::ProxyError::validation("Base64 data cannot be empty"));
        }

        // Try to decode base64 data
        match general_purpose::STANDARD.decode(data) {
            Ok(_) => Ok(()),
            Err(e) => Err(crate::error::ProxyError::validation(
                format!("Invalid base64 encoding: {}", e)
            ))
        }
    }

    /// Validate URI format and security
    fn validate_uri(uri: &str) -> crate::error::Result<()> {
        if uri.is_empty() {
            return Err(crate::error::ProxyError::validation("URI cannot be empty"));
        }

        // Check for directory traversal in the original URI before parsing
        if uri.contains("..") {
            return Err(crate::error::ProxyError::validation(
                "URI contains directory traversal attempt"
            ));
        }

        // Basic URI validation
        match url::Url::parse(uri) {
            Ok(parsed_url) => {
                // Security checks
                let scheme = parsed_url.scheme();

                // Allow common safe schemes
                const ALLOWED_SCHEMES: &[&str] = &["http", "https", "file", "data", "ftp", "ftps"];
                if !ALLOWED_SCHEMES.contains(&scheme) {
                    return Err(crate::error::ProxyError::validation(
                        format!("Unsafe URI scheme: {}. Allowed schemes: {}",
                            scheme,
                            ALLOWED_SCHEMES.join(", ")
                        )
                    ));
                }

                // Additional security checks for specific schemes
                if scheme == "file" {
                    // Prevent directory traversal
                    let path = parsed_url.path();
                    if path.contains("..") || path.contains("/../") || path.ends_with("/..") {
                        return Err(crate::error::ProxyError::validation(
                            "File URI contains directory traversal attempt"
                        ));
                    }
                }

                Ok(())
            }
            Err(e) => Err(crate::error::ProxyError::validation(
                format!("Invalid URI format: {}", e)
            ))
        }
    }

    /// Get content type as string
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::Image { .. } => "image",
            Self::Resource { .. } => "resource",
        }
    }

    /// Get MIME type if available
    pub fn mime_type(&self) -> Option<&str> {
        match self {
            Self::Text { .. } => Some("text/plain"),
            Self::Image { mime_type, .. } => Some(mime_type),
            Self::Resource { mime_type, .. } => mime_type.as_deref(),
        }
    }

    /// Check if content is safe for processing
    pub fn is_safe(&self) -> bool {
        match self.validate() {
            Ok(()) => true,
            Err(_) => false,
        }
    }
}

/// Tool call result (MCP-compliant format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the call was successful
    pub success: bool,
    /// MCP-compliant error flag (required by MCP specification)
    #[serde(rename = "isError")]
    pub is_error: bool,
    /// Content array for MCP-compliant responses
    pub content: Vec<ToolContent>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// Legacy data field for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ToolResult {
    /// Create a successful result with text content
    pub fn success(data: Value) -> Self {
        let content = vec![ToolContent::text(
            serde_json::to_string_pretty(&data).unwrap_or_else(|_| data.to_string())
        )];
        Self {
            success: true,
            is_error: false,
            content,
            error: None,
            metadata: None,
            data: Some(data), // Keep for backward compatibility
        }
    }

    /// Create a successful result with custom content
    pub fn success_with_content(content: Vec<ToolContent>) -> Self {
        Self {
            success: true,
            is_error: false,
            content,
            error: None,
            metadata: None,
            data: None,
        }
    }

    /// Create a successful result with metadata
    pub fn success_with_metadata(data: Value, metadata: Value) -> Self {
        let content = vec![ToolContent::text(
            serde_json::to_string_pretty(&data).unwrap_or_else(|_| data.to_string())
        )];
        Self {
            success: true,
            is_error: false,
            content,
            error: None,
            metadata: Some(metadata),
            data: Some(data), // Keep for backward compatibility
        }
    }

    /// Create an error result
    pub fn error(error: String) -> Self {
        let content = vec![ToolContent::text(format!("Error: {}", error))];
        Self {
            success: false,
            is_error: true,
            content,
            error: Some(error),
            metadata: None,
            data: None,
        }
    }

    /// Create an error result with metadata
    pub fn error_with_metadata(error: String, metadata: Value) -> Self {
        let content = vec![ToolContent::text(format!("Error: {}", error))];
        Self {
            success: false,
            is_error: true,
            content,
            error: Some(error),
            metadata: Some(metadata),
            data: None,
        }
    }

    /// Validate the result structure (MCP-compliant)
    pub fn validate(&self) -> Result<()> {
        // Check consistency between success and is_error
        if self.success && self.is_error {
            return Err(crate::error::ProxyError::validation(
                "Result cannot be both successful and error"
            ));
        }

        if !self.success && !self.is_error {
            return Err(crate::error::ProxyError::validation(
                "Failed result must have is_error set to true"
            ));
        }

        // Check error message consistency
        if self.success && self.error.is_some() {
            return Err(crate::error::ProxyError::validation(
                "Successful result cannot have an error message"
            ));
        }

        if !self.success && self.error.is_none() {
            return Err(crate::error::ProxyError::validation(
                "Failed result must have an error message"
            ));
        }

        // Validate content array is not empty
        if self.content.is_empty() {
            return Err(crate::error::ProxyError::validation(
                "Tool result must have at least one content item"
            ));
        }

        Ok(())
    }
}

/// MCP Request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID (can be string, number, or null for notifications)
    pub id: Option<serde_json::Value>,
    /// Method name
    pub method: String,
    /// Parameters
    pub params: Option<Value>,
}

/// MCP Response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID this responds to
    pub id: String,
    /// Result (if successful)
    pub result: Option<Value>,
    /// Error (if failed)
    pub error: Option<McpError>,
}

// ============================================================================
// MCP Logging Types
// ============================================================================

/// MCP Log levels following RFC 5424 syslog severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Detailed debugging information
    Debug,
    /// General informational messages
    Info,
    /// Normal but significant events
    Notice,
    /// Warning conditions
    Warning,
    /// Error conditions
    Error,
    /// Critical conditions
    Critical,
    /// Action must be taken immediately
    Alert,
    /// System is unusable
    Emergency,
}

impl LogLevel {
    /// Convert log level to numeric value for comparison
    pub fn to_numeric(&self) -> u8 {
        match self {
            LogLevel::Debug => 7,
            LogLevel::Info => 6,
            LogLevel::Notice => 5,
            LogLevel::Warning => 4,
            LogLevel::Error => 3,
            LogLevel::Critical => 2,
            LogLevel::Alert => 1,
            LogLevel::Emergency => 0,
        }
    }

    /// Parse log level from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "notice" => Ok(LogLevel::Notice),
            "warning" | "warn" => Ok(LogLevel::Warning),
            "error" => Ok(LogLevel::Error),
            "critical" | "crit" => Ok(LogLevel::Critical),
            "alert" => Ok(LogLevel::Alert),
            "emergency" | "emerg" => Ok(LogLevel::Emergency),
            _ => Err(crate::error::ProxyError::validation(format!("Invalid log level: {}", s))),
        }
    }

    /// Check if this level should be logged given minimum level
    pub fn should_log(&self, min_level: LogLevel) -> bool {
        self.to_numeric() <= min_level.to_numeric()
    }
}

/// MCP logging/setLevel request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSetLevelRequest {
    /// Minimum log level to emit
    pub level: LogLevel,
}

/// MCP log message notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    /// Log severity level
    pub level: LogLevel,
    /// Optional logger name/component
    pub logger: Option<String>,
    /// Arbitrary JSON-serializable log data
    pub data: Value,
}

impl LogMessage {
    /// Create a new log message
    pub fn new(level: LogLevel, data: Value) -> Self {
        Self {
            level,
            logger: None,
            data,
        }
    }

    /// Create a log message with logger name
    pub fn with_logger(level: LogLevel, logger: String, data: Value) -> Self {
        Self {
            level,
            logger: Some(logger),
            data,
        }
    }

    /// Create a debug log message
    pub fn debug(data: Value) -> Self {
        Self::new(LogLevel::Debug, data)
    }

    /// Create an info log message
    pub fn info(data: Value) -> Self {
        Self::new(LogLevel::Info, data)
    }

    /// Create a warning log message
    pub fn warning(data: Value) -> Self {
        Self::new(LogLevel::Warning, data)
    }

    /// Create an error log message
    pub fn error(data: Value) -> Self {
        Self::new(LogLevel::Error, data)
    }
}

/// MCP completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Reference to the resource or prompt for completion
    #[serde(rename = "ref")]
    pub reference: CompletionReference,
    /// Argument for completion (e.g., partial text)
    pub argument: CompletionArgument,
}

/// Reference for completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CompletionReference {
    #[serde(rename = "ref/resource")]
    Resource { uri: String },
    #[serde(rename = "ref/prompt")]
    Prompt { name: String },
}

/// Argument for completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum CompletionArgument {
    #[serde(rename = "name")]
    Name { value: String },
    #[serde(rename = "value")]
    Value { value: String },
}

/// MCP completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// List of completion values
    pub completion: CompletionResult,
}

/// Completion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResult {
    /// Array of completion values
    pub values: Vec<String>,
    /// Total number of available completions (if known)
    pub total: Option<u32>,
    /// Whether there are more completions available
    pub has_more: Option<bool>,
}

// ============================================================================
// MCP Notification Types
// ============================================================================

/// MCP notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpNotification {
    /// Notification method
    pub method: String,
    /// Optional notification parameters
    pub params: Option<Value>,
}

impl McpNotification {
    /// Create a new notification
    pub fn new(method: String) -> Self {
        Self {
            method,
            params: None,
        }
    }

    /// Create a notification with parameters
    pub fn with_params(method: String, params: Value) -> Self {
        Self {
            method,
            params: Some(params),
        }
    }

    /// Create a resources list changed notification
    pub fn resources_list_changed() -> Self {
        Self::new("notifications/resources/list_changed".to_string())
    }

    /// Create a prompts list changed notification
    pub fn prompts_list_changed() -> Self {
        Self::new("notifications/prompts/list_changed".to_string())
    }

    /// Create a tools list changed notification
    pub fn tools_list_changed() -> Self {
        Self::new("notifications/tools/list_changed".to_string())
    }

    /// Create a resource updated notification
    pub fn resource_updated(uri: String) -> Self {
        Self::with_params(
            "notifications/resources/updated".to_string(),
            serde_json::json!({ "uri": uri })
        )
    }

    /// Create a log message notification
    pub fn log_message(log_message: LogMessage) -> Self {
        Self::with_params(
            "notifications/message".to_string(),
            serde_json::to_value(log_message).unwrap_or_default()
        )
    }
}

// ============================================================================
// Prompt Template Types
// ============================================================================

/// Prompt template argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Whether this argument is required
    pub required: bool,
}

/// Prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Unique template name
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Template arguments
    pub arguments: Vec<PromptArgument>,
}

/// Prompt list request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptListRequest {
    /// Optional cursor for pagination
    pub cursor: Option<String>,
}

/// Prompt list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptListResponse {
    /// List of available prompt templates
    pub prompts: Vec<PromptTemplate>,
    /// Next cursor for pagination (if more prompts available)
    pub next_cursor: Option<String>,
}

/// Prompt get request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGetRequest {
    /// Template name to retrieve
    pub name: String,
    /// Arguments for template substitution
    pub arguments: Option<Value>,
}

/// Rendered prompt message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    /// Message role (e.g., "user", "assistant", "system")
    pub role: String,
    /// Message content
    pub content: String,
}

/// Prompt get response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGetResponse {
    /// Rendered prompt messages
    pub messages: Vec<PromptMessage>,
    /// Template description
    pub description: Option<String>,
}

impl PromptArgument {
    /// Create a new required prompt argument
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            required: true,
        }
    }

    /// Create a new prompt argument with description
    pub fn with_description(name: String, description: String) -> Self {
        Self {
            name,
            description: Some(description),
            required: true,
        }
    }

    /// Create an optional prompt argument
    pub fn optional(name: String) -> Self {
        Self {
            name,
            description: None,
            required: false,
        }
    }

    /// Create an optional prompt argument with description
    pub fn optional_with_description(name: String, description: String) -> Self {
        Self {
            name,
            description: Some(description),
            required: false,
        }
    }
}

impl PromptTemplate {
    /// Create a new prompt template
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            arguments: Vec::new(),
        }
    }

    /// Create a prompt template with description
    pub fn with_description(name: String, description: String) -> Self {
        Self {
            name,
            description: Some(description),
            arguments: Vec::new(),
        }
    }

    /// Add an argument to the template
    pub fn with_argument(mut self, argument: PromptArgument) -> Self {
        self.arguments.push(argument);
        self
    }

    /// Add multiple arguments to the template
    pub fn with_arguments(mut self, arguments: Vec<PromptArgument>) -> Self {
        self.arguments.extend(arguments);
        self
    }

    /// Validate the template
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.name.is_empty() {
            return Err(crate::error::ProxyError::validation("Template name cannot be empty"));
        }

        // Check for duplicate argument names
        let mut arg_names = std::collections::HashSet::new();
        for arg in &self.arguments {
            if arg.name.is_empty() {
                return Err(crate::error::ProxyError::validation("Argument name cannot be empty"));
            }
            if !arg_names.insert(&arg.name) {
                return Err(crate::error::ProxyError::validation(
                    format!("Duplicate argument name: {}", arg.name)
                ));
            }
        }

        Ok(())
    }

    /// Get required arguments
    pub fn required_arguments(&self) -> Vec<&PromptArgument> {
        self.arguments.iter().filter(|arg| arg.required).collect()
    }

    /// Get optional arguments
    pub fn optional_arguments(&self) -> Vec<&PromptArgument> {
        self.arguments.iter().filter(|arg| !arg.required).collect()
    }
}

impl PromptMessage {
    /// Create a new prompt message
    pub fn new(role: String, content: String) -> Self {
        Self { role, content }
    }

    /// Create a user message
    pub fn user(content: String) -> Self {
        Self::new("user".to_string(), content)
    }

    /// Create an assistant message
    pub fn assistant(content: String) -> Self {
        Self::new("assistant".to_string(), content)
    }

    /// Create a system message
    pub fn system(content: String) -> Self {
        Self::new("system".to_string(), content)
    }
}