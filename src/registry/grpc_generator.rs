//! gRPC Capability Generator
//! 
//! This module provides functionality to generate MCP tool definitions from gRPC/protobuf service definitions.
//! It supports parsing .proto files and converting gRPC service methods into MCP tools.

use crate::error::{ProxyError, Result};
use crate::registry::types::{
    CapabilityFile, FileMetadata, ToolDefinition, RoutingConfig,
    // MCP 2025-06-18 Enhanced Types
    EnhancedFileMetadata, EnhancedToolDefinition, CoreDefinition, ExecutionConfig,
    DiscoveryEnhancement, MonitoringConfig, AccessConfig, ClassificationMetadata,
    DiscoveryMetadata, McpCapabilities, SandboxConfig, PerformanceConfig,
    AiEnhancedDiscovery, ProgressTrackingConfig, CancellationConfig,
    MetricsConfig, EnhancedRoutingConfig, SemanticContext, WorkflowIntegration
};
use crate::mcp::tool_validation::SecurityClassification;
use crate::utils::{sanitize_capability_name, sanitize_tool_name, ensure_unique_capability_name};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use base64::Engine;

/// Streaming strategy for gRPC streaming methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamingStrategy {
    /// Polling strategy (start/poll/stop tools)
    Polling,
    /// Pagination strategy (paginated tools with tokens)
    Pagination,
    /// Agent-level strategy (transparent streaming)
    AgentLevel,
}

/// Configuration for gRPC capability generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcGeneratorConfig {
    /// Base endpoint for gRPC service
    pub endpoint: String,
    /// Authentication configuration
    pub auth_config: Option<AuthConfig>,
    /// Tool name prefix
    pub tool_prefix: Option<String>,
    /// Filter for specific services
    pub service_filter: Option<Vec<String>>,
    /// Filter for specific methods
    pub method_filter: Option<Vec<String>>,
    /// Streaming strategy for server streaming methods
    pub server_streaming_strategy: StreamingStrategy,
    /// Streaming strategy for client streaming methods
    pub client_streaming_strategy: StreamingStrategy,
    /// Streaming strategy for bidirectional streaming methods
    pub bidirectional_streaming_strategy: StreamingStrategy,
    /// Whether to include method options in tool definitions
    pub include_method_options: bool,
    /// Whether to generate separate tools for streaming methods
    pub separate_streaming_tools: bool,
    /// Whether to generate enhanced MCP 2025-06-18 format
    pub use_enhanced_format: bool,
}

/// Authentication configuration for gRPC endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type
    pub auth_type: AuthType,
    /// Additional headers to include in requests
    pub headers: HashMap<String, String>,
}

/// Supported authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthType {
    /// No authentication
    #[serde(rename = "none")]
    None,
    /// API Key authentication
    #[serde(rename = "api_key")]
    ApiKey {
        /// API key value
        key: String,
        /// Header name for the API key
        header: String,
    },
    /// Bearer token authentication
    #[serde(rename = "bearer")]
    Bearer {
        /// Bearer token value
        token: String,
    },
    /// Basic authentication
    #[serde(rename = "basic")]
    Basic {
        /// Username
        username: String,
        /// Password
        password: String,
    },
    /// OAuth 2.0 authentication
    #[serde(rename = "oauth")]
    OAuth {
        /// OAuth token
        token: String,
        /// Token type (usually "Bearer")
        token_type: String,
    },
}

/// Represents a gRPC service definition
#[derive(Debug, Clone)]
pub struct GrpcService {
    /// Service name
    pub name: String,
    /// Service package
    pub package: String,
    /// Service methods
    pub methods: Vec<GrpcMethod>,
    /// Service options
    pub options: HashMap<String, String>,
}

/// Represents a gRPC method definition
#[derive(Debug, Clone)]
pub struct GrpcMethod {
    /// Method name
    pub name: String,
    /// Input message type
    pub input_type: String,
    /// Output message type
    pub output_type: String,
    /// Whether the method is client streaming
    pub client_streaming: bool,
    /// Whether the method is server streaming
    pub server_streaming: bool,
    /// Method options
    pub options: HashMap<String, String>,
}

/// Represents a gRPC message definition
#[derive(Debug, Clone)]
pub struct GrpcMessage {
    /// Message name
    pub name: String,
    /// Message fields
    pub fields: Vec<GrpcField>,
    /// Nested messages
    pub nested_messages: Vec<GrpcMessage>,
    /// Nested enums
    pub nested_enums: Vec<GrpcEnum>,
    /// Message options
    pub options: HashMap<String, String>,
}

/// Represents a gRPC field definition
#[derive(Debug, Clone)]
pub struct GrpcField {
    /// Field name
    pub name: String,
    /// Field number
    pub number: i32,
    /// Field type
    pub field_type: String,
    /// Whether the field is repeated
    pub repeated: bool,
    /// Whether the field is optional
    pub optional: bool,
    /// Field options
    pub options: HashMap<String, String>,
}

/// Represents a gRPC enum definition
#[derive(Debug, Clone)]
pub struct GrpcEnum {
    /// Enum name
    pub name: String,
    /// Enum values
    pub values: Vec<GrpcEnumValue>,
    /// Enum options
    pub options: HashMap<String, String>,
}

/// Represents a gRPC enum value definition
#[derive(Debug, Clone)]
pub struct GrpcEnumValue {
    /// Enum value name
    pub name: String,
    /// Enum value number
    pub number: i32,
    /// Enum value options
    pub options: HashMap<String, String>,
}

/// gRPC Capability Generator
/// 
/// Generates MCP tool definitions from gRPC/protobuf service definitions.
pub struct GrpcCapabilityGenerator {
    /// Configuration for the generator
    pub config: GrpcGeneratorConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_type: AuthType::None,
            headers: HashMap::new(),
        }
    }
}

impl Default for StreamingStrategy {
    fn default() -> Self {
        StreamingStrategy::Polling
    }
}

impl GrpcCapabilityGenerator {
    /// Create a new gRPC capability generator with the given configuration
    pub fn new(config: GrpcGeneratorConfig) -> Self {
        Self { config }
    }

    /// Generate capability file from protobuf file
    pub fn generate_from_proto_file<P: AsRef<Path>>(&self, proto_path: P) -> Result<CapabilityFile> {
        // Read the protobuf file
        let proto_content = std::fs::read_to_string(proto_path)
            .map_err(|e| ProxyError::config(format!("Failed to read proto file: {}", e)))?;
        
        // Generate from content
        self.generate_from_proto_content(&proto_content)
    }

    /// Generate capability file from protobuf content
    pub fn generate_from_proto_content(&self, proto_content: &str) -> Result<CapabilityFile> {
        // Parse the protobuf content to extract services
        let services = self.parse_proto_content(proto_content)?;
        
        if services.is_empty() {
            return Err(ProxyError::config("No gRPC services found in protobuf content"));
        }
        
        // Create tool definitions for each service and method
        let mut all_tools = Vec::new();
        
        for service in services {
            // Apply service filter if configured
            if let Some(ref service_filter) = self.config.service_filter {
                if !service_filter.contains(&service.name) {
                    continue;
                }
            }
            
            // Apply method filter if configured
            let methods = if let Some(ref method_filter) = self.config.method_filter {
                service.methods.iter()
                    .filter(|m| method_filter.contains(&m.name))
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                service.methods.clone()
            };
            
            // Create tool definitions for each method
            for method in methods {
                match self.method_to_tool_definition(&service, &method) {
                    Ok(tool) => all_tools.push(tool),
                    Err(e) => {
                        // Log the error but continue with other methods
                        eprintln!("Error converting method {} to tool: {:?}", method.name, e);
                    }
                }
            }
        }
        
        if all_tools.is_empty() {
            return Err(ProxyError::config("No tools generated from protobuf content"));
        }
        
        // Generate capability file based on format selection
        if self.config.use_enhanced_format {
            self.generate_enhanced_capability_file(all_tools)
        } else {
            self.generate_legacy_capability_file(all_tools)
        }
    }

    /// Generate enhanced MCP 2025-06-18 format capability file
    fn generate_enhanced_capability_file(&self, tools: Vec<ToolDefinition>) -> Result<CapabilityFile> {
        let mut enhanced_tools = Vec::new();
        let mut existing_names = std::collections::HashSet::new();

        for tool in tools {
            match self.tool_to_enhanced_definition(tool) {
                Ok(mut enhanced_tool) => {
                    // Sanitize tool name to ensure it follows naming conventions
                    enhanced_tool.name = sanitize_tool_name(&enhanced_tool.name);
                    
                    // Ensure uniqueness by checking against existing tool names
                    enhanced_tool.name = ensure_unique_capability_name(&enhanced_tool.name, &existing_names);
                    existing_names.insert(enhanced_tool.name.clone());
                    
                    enhanced_tools.push(enhanced_tool);
                }
                Err(e) => {
                    // Log warning but continue processing other tools
                    tracing::warn!("Failed to convert tool to enhanced definition: {}", e);
                }
            }
        }

        // Generate sanitized capability name from endpoint
        let raw_capability_name = format!("Enhanced gRPC Service Capabilities - {}", 
            self.config.endpoint.replace("https://", "").replace("http://", "").replace("/", " "));
        let sanitized_capability_name = sanitize_capability_name(&raw_capability_name);

        let enhanced_metadata = EnhancedFileMetadata {
            name: sanitized_capability_name,
            description: format!("Auto-generated gRPC service tools for {} - MCP 2025-06-18 compliant with AI enhancement", self.config.endpoint),
            version: "3.0.0".to_string(),
            author: "gRPC Capability Generator".to_string(),
            classification: Some(ClassificationMetadata {
                security_level: "safe".to_string(),
                complexity_level: "moderate".to_string(),
                domain: "rpc".to_string(),
                use_cases: vec!["rpc_integration".to_string(), "grpc_client".to_string()],
            }),
            discovery_metadata: Some(DiscoveryMetadata {
                primary_keywords: vec!["grpc".to_string(), "rpc".to_string(), "protobuf".to_string(), "auto-generated".to_string()],
                semantic_embeddings: true,
                llm_enhanced: true,
                workflow_enabled: true,
            }),
            mcp_capabilities: Some(McpCapabilities {
                version: "2025-06-18".to_string(),
                supports_cancellation: true,
                supports_progress: true,
                supports_sampling: false,
                supports_validation: true,
                supports_elicitation: false,
            }),
        };

        CapabilityFile::new_enhanced(enhanced_metadata, enhanced_tools)
    }

    /// Generate legacy format capability file
    fn generate_legacy_capability_file(&self, mut tools: Vec<ToolDefinition>) -> Result<CapabilityFile> {
        let mut existing_names = std::collections::HashSet::new();
        
        // Sanitize tool names
        for tool in &mut tools {
            // Sanitize tool name to ensure it follows naming conventions
            tool.name = sanitize_tool_name(&tool.name);
            
            // Ensure uniqueness by checking against existing tool names
            tool.name = ensure_unique_capability_name(&tool.name, &existing_names);
            existing_names.insert(tool.name.clone());
        }

        // Generate sanitized capability name from endpoint
        let raw_capability_name = format!("gRPC Service Capabilities - {}", 
            self.config.endpoint.replace("https://", "").replace("http://", "").replace("/", " "));
        let sanitized_capability_name = sanitize_capability_name(&raw_capability_name);
        
        let metadata = FileMetadata::with_name(sanitized_capability_name)
            .description("gRPC service capabilities".to_string())
            .version("1.0.0".to_string())
            .author("gRPC Capability Generator".to_string())
            .tags(vec!["grpc".to_string()]);
        
        Ok(CapabilityFile {
            metadata: Some(metadata),
            tools,
            enhanced_metadata: None,
            enhanced_tools: None,
        })
    }

    /// Convert legacy tool definition to enhanced MCP 2025-06-18 format
    fn tool_to_enhanced_definition(&self, tool: ToolDefinition) -> Result<EnhancedToolDefinition> {
        let enhanced_name = format!("enhanced_{}", tool.name);
        let enhanced_description = format!("AI-enhanced {} with MCP 2025-06-18 compliance", tool.description);

        let input_schema = tool.input_schema.clone();
        let core = CoreDefinition {
            description: enhanced_description.clone(),
            input_schema,
        };

        let execution = ExecutionConfig {
            routing: EnhancedRoutingConfig {
                r#type: "enhanced_grpc".to_string(),
                primary: Some(self.create_enhanced_routing_config(&tool)?),
                fallback: None,
                config: Some(serde_json::json!({
                    "enhanced": true,
                    "grpc_endpoint": self.config.endpoint
                })),
            },
            security: crate::registry::types::SecurityConfig {
                classification: "safe".to_string(),
                sandbox: Some(crate::registry::types::SandboxConfig {
                    resources: Some(crate::registry::types::ResourceLimits {
                        max_memory_mb: Some(512),
                        max_cpu_percent: Some(40),
                        max_execution_seconds: Some(120),
                        max_file_descriptors: Some(200),
                    }),
                    filesystem: None,
                    network: Some(crate::registry::types::NetworkRestrictions {
                        allowed: true,
                        allowed_domains: Some(vec![self.config.endpoint.clone()]),
                        denied_domains: None,
                    }),
                    environment: Some(crate::registry::types::EnvironmentRestrictions {
                        readonly_system: Some(true),
                        env_vars: None,
                    }),
                }),
                requires_approval: Some(false),
                approval_workflow: None,
            },
            performance: PerformanceConfig {
                estimated_duration: Some(serde_json::json!({
                    "simple_operation": 10,
                    "complex_operation": 60
                })),
                complexity: Some("moderate".to_string()),
                supports_cancellation: Some(true),
                supports_progress: Some(false),
                cache_results: Some(false),
                cache_ttl_seconds: Some(0),
                adaptive_optimization: Some(true),
            },
        };

        let discovery = DiscoveryEnhancement {
            ai_enhanced: Some(AiEnhancedDiscovery {
                description: Some(format!("AI-enhanced {} with intelligent processing and security validation", tool.description)),
                usage_patterns: Some(vec![
                    format!("use {} to {{action}}", enhanced_name),
                    format!("help me {{accomplish_task}} with {}", enhanced_name),
                    format!("{} for {{specific_purpose}}", enhanced_name),
                ]),
                semantic_context: Some(SemanticContext {
                    primary_intent: Some("rpc_operation".to_string()),
                    data_types: Some(vec!["structured".to_string(), "protobuf".to_string()]),
                    operations: Some(vec!["grpc_call".to_string()]),
                    security_features: Some(vec!["authenticated_access".to_string()]),
                }),
                ai_capabilities: None,
                workflow_integration: Some(WorkflowIntegration {
                    typically_follows: None,
                    typically_precedes: None,
                    chain_compatibility: Some(vec!["rpc_workflow".to_string()]),
                }),
            }),
            parameter_intelligence: Some(self.generate_parameter_intelligence(&tool)?),
        };

        let monitoring = MonitoringConfig {
            progress_tracking: Some(ProgressTrackingConfig {
                enabled: false,
                granularity: Some("basic".to_string()),
                sub_operations: None,
            }),
            cancellation: Some(CancellationConfig {
                enabled: true,
                graceful_timeout_seconds: Some(30),
                cleanup_required: Some(false),
                cleanup_operations: None,
            }),
            metrics: Some(MetricsConfig {
                track_execution_time: Some(true),
                track_success_rate: Some(true),
                custom_metrics: Some(vec![format!("{}_operations_completed", enhanced_name)]),
            }),
        };

        let access = AccessConfig {
            hidden: true, // gRPC tools are hidden by default
            enabled: true, // gRPC tools are enabled by default
            requires_permissions: Some(vec!["tool:execute".to_string(), "security:validated".to_string()]),
            user_groups: Some(vec!["administrators".to_string()]),
            approval_required: Some(false),
            usage_analytics: Some(true),
        };

        EnhancedToolDefinition::new(enhanced_name, core, execution, discovery, monitoring, access)
    }

    /// Create enhanced routing configuration for gRPC tool
    fn create_enhanced_routing_config(&self, tool: &ToolDefinition) -> Result<Value> {
        let mut config = serde_json::Map::new();

        // Basic gRPC configuration
        config.insert("endpoint".to_string(), json!(self.config.endpoint));
        config.insert("timeout_seconds".to_string(), json!(30));

        // Enhanced features
        config.insert("retry_count".to_string(), json!(3));
        config.insert("retry_backoff_ms".to_string(), json!(1000));

        // Copy existing routing config
        if let serde_json::Value::Object(existing_config) = &tool.routing.config {
            for (key, value) in existing_config {
                config.insert(key.clone(), value.clone());
            }
        }

        // Authentication
        if let Some(ref auth) = self.config.auth_config {
            match &auth.auth_type {
                AuthType::Bearer { token } => {
                    config.insert("headers".to_string(), json!({
                        "Authorization": format!("Bearer {}", token)
                    }));
                }
                AuthType::ApiKey { key, header } => {
                    config.insert("headers".to_string(), json!({
                        header: key
                    }));
                }
                AuthType::Basic { username, password } => {
                    let credentials = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                    config.insert("headers".to_string(), json!({
                        "Authorization": format!("Basic {}", credentials)
                    }));
                }
                _ => {}
            }
        }

        Ok(json!(config))
    }

    /// Generate parameter intelligence for gRPC tool
    fn generate_parameter_intelligence(&self, tool: &ToolDefinition) -> Result<Value> {
        let mut param_map = HashMap::new();

        // Extract parameters from the input schema
        if let Some(properties) = tool.input_schema.get("properties").and_then(|p| p.as_object()) {
            for (param_name, param_schema) in properties {
                let mut param_config = HashMap::new();

                // Set smart default from schema
                if let Some(default_value) = param_schema.get("default") {
                    param_config.insert("smart_default".to_string(), default_value.clone());
                } else {
                    param_config.insert("smart_default".to_string(), Value::Null);
                }

                // Add validation rules
                let mut validations = Vec::new();
                if let Some(required_params) = tool.input_schema.get("required").and_then(|r| r.as_array()) {
                    if required_params.iter().any(|r| r.as_str() == Some(param_name)) {
                        validations.push(json!({
                            "rule": "required_validation",
                            "message": format!("{} must be provided and valid", param_name)
                        }));
                    }
                }
                param_config.insert("validation".to_string(), json!(validations));

                // Add smart suggestions based on parameter type
                let suggestions = vec![json!({
                    "pattern": "*",
                    "description": format!("{} parameter values", param_name),
                    "examples": self.generate_param_examples(param_schema)
                })];
                param_config.insert("smart_suggestions".to_string(), json!(suggestions));

                param_map.insert(param_name.clone(), Value::Object(
                    param_config.into_iter().collect()
                ));
            }
        }

        Ok(json!(param_map))
    }

    /// Generate example values for parameter schema
    fn generate_param_examples(&self, schema: &Value) -> Vec<String> {
        let mut examples = Vec::new();

        if let Some(schema_obj) = schema.as_object() {
            if let Some(param_type) = schema_obj.get("type").and_then(|t| t.as_str()) {
                match param_type {
                    "string" => {
                        examples.push("example_string".to_string());
                        examples.push("test_value".to_string());
                    }
                    "integer" | "number" => {
                        examples.push("123".to_string());
                        examples.push("456".to_string());
                    }
                    "boolean" => {
                        examples.push("true".to_string());
                        examples.push("false".to_string());
                    }
                    "array" => {
                        examples.push("[]".to_string());
                        examples.push("[\"item1\", \"item2\"]".to_string());
                    }
                    "object" => {
                        examples.push("{}".to_string());
                        examples.push("{\"key\": \"value\"}".to_string());
                    }
                    _ => {
                        examples.push("example_value".to_string());
                    }
                }
            }

            // Add enum values if available
            if let Some(enum_values) = schema_obj.get("enum").and_then(|e| e.as_array()) {
                for enum_val in enum_values {
                    if let Some(val_str) = enum_val.as_str() {
                        examples.push(val_str.to_string());
                    }
                }
            }
        }

        if examples.is_empty() {
            examples.push("example_value".to_string());
        }

        examples
    }

    /// Parse protobuf content into gRPC service definitions - actual implementation
    fn parse_proto_content_impl(&self, proto_content: &str) -> Result<Vec<GrpcService>> {
        // This is a simplified parser for demonstration purposes
        // In a real implementation, we would use a proper protobuf parser library
        
        let mut services = Vec::new();
        let mut current_package = String::new();
        
        // Split the content into lines for parsing
        let lines: Vec<&str> = proto_content.lines().collect();
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            
            // Parse package declaration
            if line.starts_with("package ") {
                current_package = line.trim_start_matches("package ")
                    .trim_end_matches(';')
                    .trim()
                    .to_string();
            }
            
            // Parse service declaration
            else if line.starts_with("service ") {
                let service_name = line.trim_start_matches("service ")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                
                // Parse service methods
                let mut methods = Vec::new();
                let mut options = HashMap::new();
                
                // Find the closing brace of the service
                let mut j = i + 1;
                let mut brace_count = 1;
                
                while j < lines.len() && brace_count > 0 {
                    let method_line = lines[j].trim();
                    
                    if method_line.contains("{") {
                        brace_count += 1;
                    }
                    if method_line.contains("}") {
                        brace_count -= 1;
                    }
                    
                    // Parse method declaration
                    if method_line.starts_with("rpc ") && method_line.contains("returns") {
                        let method = self.parse_method_line(method_line)?;
                        methods.push(method);
                    }
                    
                    // Parse service options
                    else if method_line.starts_with("option ") {
                        let (key, value) = self.parse_option_line(method_line)?;
                        options.insert(key, value);
                    }
                    
                    j += 1;
                }
                
                // Create the service
                services.push(GrpcService {
                    name: service_name,
                    package: current_package.clone(),
                    methods,
                    options,
                });
                
                // Skip to after the service definition
                i = j;
                continue;
            }
            
            i += 1;
        }
        
        Ok(services)
    }

    /// Parse protobuf content into gRPC service definitions
    pub fn parse_proto_content(&self, proto_content: &str) -> Result<Vec<GrpcService>> {
        // Use the implementation method
        self.parse_proto_content_impl(proto_content)
    }

    /// Parse a method line into a GrpcMethod
    fn parse_method_line(&self, line: &str) -> Result<GrpcMethod> {
        // Extract method name, input type, and output type
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() < 4 {
            return Err(ProxyError::config(format!("Invalid method line: {}", line)));
        }
        
        let name = parts[1].trim_end_matches('(').to_string();
        
        // Extract input type
        let input_start = line.find('(').ok_or_else(|| ProxyError::config("Missing input type"))?;
        let input_end = line.find(')').ok_or_else(|| ProxyError::config("Missing closing parenthesis for input type"))?;
        let input_type = line[input_start+1..input_end].trim().to_string();
        
        // Check for streaming
        let client_streaming = input_type.starts_with("stream ");
        let input_type = input_type.trim_start_matches("stream ").to_string();
        
        // Extract output type
        let returns_idx = line.find("returns").ok_or_else(|| ProxyError::config("Missing 'returns' keyword"))?;
        let output_start = line[returns_idx..].find('(').ok_or_else(|| ProxyError::config("Missing output type"))?;
        let output_start = returns_idx + output_start;
        let output_end = line[output_start..].find(')').ok_or_else(|| ProxyError::config("Missing closing parenthesis for output type"))?;
        let output_end = output_start + output_end;
        let output_type = line[output_start+1..output_end].trim().to_string();
        
        // Check for streaming
        let server_streaming = output_type.starts_with("stream ");
        let output_type = output_type.trim_start_matches("stream ").to_string();
        
        // Parse method options
        let mut options = HashMap::new();
        if line.contains("{") && line.contains("}") {
            let options_start = line.find('{').unwrap();
            let options_end = line.find('}').unwrap();
            let options_str = &line[options_start+1..options_end];
            
            for option in options_str.split(';') {
                let option = option.trim();
                if !option.is_empty() {
                    if let Ok((key, value)) = self.parse_option_line(&format!("option {}", option)) {
                        options.insert(key, value);
                    }
                }
            }
        }
        
        Ok(GrpcMethod {
            name,
            input_type,
            output_type,
            client_streaming,
            server_streaming,
            options,
        })
    }

    /// Parse an option line into a key-value pair
    fn parse_option_line(&self, line: &str) -> Result<(String, String)> {
        let option_str = line.trim_start_matches("option ").trim_end_matches(';').trim();
        
        if let Some(equals_idx) = option_str.find('=') {
            let key = option_str[..equals_idx].trim().to_string();
            let value = option_str[equals_idx+1..].trim().trim_matches('"').to_string();
            Ok((key, value))
        } else {
            Err(ProxyError::config(format!("Invalid option line: {}", line)))
        }
    }

    /// Convert gRPC service to capability file
    pub fn service_to_capability_file(&self, service: &GrpcService) -> Result<CapabilityFile> {
        let mut tools = Vec::new();
        
        // Convert each method to a tool definition
        for method in &service.methods {
            match self.method_to_tool_definition(service, method) {
                Ok(tool) => tools.push(tool),
                Err(e) => {
                    // Log the error but continue with other methods
                    eprintln!("Error converting method {} to tool: {:?}", method.name, e);
                }
            }
        }
        
        // Create the capability file metadata
        let file_metadata = FileMetadata::with_name(format!("{}-capabilities", service.name.to_lowercase()))
            .description(format!("Capabilities for {} gRPC service", service.name))
            .version("1.0.0".to_string())
            .author("gRPC Capability Generator".to_string())
            .tags(vec!["grpc".to_string(), service.name.to_lowercase()]);
        
        Ok(CapabilityFile {
            metadata: Some(file_metadata),
            tools,
            enhanced_metadata: None,
            enhanced_tools: None,
        })
    }

    /// Convert gRPC method to tool definition
    pub fn method_to_tool_definition(&self, service: &GrpcService, method: &GrpcMethod) -> Result<ToolDefinition> {
        // Generate a tool name
        let tool_name = self.generate_tool_name(service, method);
        
        // Create a description based on the service and method
        let description = if method.client_streaming && method.server_streaming {
            format!("Bidirectional streaming method {} of {} service", method.name, service.name)
        } else if method.client_streaming {
            format!("Client streaming method {} of {} service", method.name, service.name)
        } else if method.server_streaming {
            format!("Server streaming method {} of {} service", method.name, service.name)
        } else {
            format!("Method {} of {} service", method.name, service.name)
        };
        
        // Generate input schema
        let mut input_schema = self.generate_input_schema(method)?;
        
        // Create routing configuration
        let mut routing_config = self.create_routing_config(service, method)?;
        
        // Handle streaming methods
        if method.client_streaming || method.server_streaming {
            let (updated_schema, updated_routing) = self.handle_streaming_method(
                method, input_schema, routing_config)?;
            
            input_schema = updated_schema;
            routing_config = updated_routing;
        }
        
        // Add method options to routing config if configured
        if self.config.include_method_options && !method.options.is_empty() {
            if let serde_json::Value::Object(ref mut config) = routing_config.config {
                let options = serde_json::to_value(&method.options)
                    .map_err(|e| ProxyError::config(format!("Failed to serialize method options: {}", e)))?;
                config.insert("method_options".to_string(), options);
            }
        }
        
        // Create the tool definition
        let tool_def = ToolDefinition::new_with_fields(
            tool_name,
            description,
            input_schema,
            routing_config,
            None, // No annotations for now
        )?;
        
        Ok(tool_def)
    }

    /// Handle streaming method by applying the appropriate streaming strategy
    fn handle_streaming_method(&self, method: &GrpcMethod, mut input_schema: Value, mut routing_config: RoutingConfig)
        -> Result<(Value, RoutingConfig)> {
        
        // Determine the streaming strategy to use
        let strategy = if method.client_streaming && method.server_streaming {
            &self.config.bidirectional_streaming_strategy
        } else if method.server_streaming {
            &self.config.server_streaming_strategy
        } else {
            &self.config.client_streaming_strategy
        };
        
        // Apply the streaming strategy
        match strategy {
            StreamingStrategy::Polling => {
                // For polling, we add polling parameters to the input schema
                if let Value::Object(ref mut obj) = input_schema {
                    if let Some(Value::Object(ref mut props)) = obj.get_mut("properties") {
                        // Add polling parameters
                        props.insert("polling_interval_ms".to_string(), json!({
                            "type": "integer",
                            "description": "Polling interval in milliseconds",
                            "default": 1000
                        }));
                        
                        props.insert("max_poll_count".to_string(), json!({
                            "type": "integer",
                            "description": "Maximum number of polling requests",
                            "default": 10
                        }));
                    }
                }
                
                // Add polling configuration to routing config
                if let Value::Object(ref mut config) = routing_config.config {
                    config.insert("streaming_strategy".to_string(), json!("polling"));
                }
            },
            StreamingStrategy::Pagination => {
                // For pagination, we add pagination parameters to the input schema
                if let Value::Object(ref mut obj) = input_schema {
                    if let Some(Value::Object(ref mut props)) = obj.get_mut("properties") {
                        // Add pagination parameters
                        props.insert("page_size".to_string(), json!({
                            "type": "integer",
                            "description": "Number of items per page",
                            "default": 10
                        }));
                        
                        props.insert("page_token".to_string(), json!({
                            "type": "string",
                            "description": "Token for pagination"
                        }));
                    }
                }
                
                // Add pagination configuration to routing config
                if let Value::Object(ref mut config) = routing_config.config {
                    config.insert("streaming_strategy".to_string(), json!("pagination"));
                    config.insert("page_token_field".to_string(), json!("page_token"));
                    config.insert("next_page_token_field".to_string(), json!("next_page_token"));
                }
            },
            StreamingStrategy::AgentLevel => {
                // For agent-level streaming, we add streaming configuration to routing config
                if let Value::Object(ref mut config) = routing_config.config {
                    config.insert("streaming_strategy".to_string(), json!("agent-level"));
                    config.insert("stream_directly".to_string(), json!(true));
                }
            }
        }
        
        Ok((input_schema, routing_config))
    }

    /// Generate tool name for gRPC method
    pub fn generate_tool_name(&self, service: &GrpcService, method: &GrpcMethod) -> String {
        // This is a placeholder implementation that will be filled in with actual code
        // For now, it returns a simple concatenation of service and method names
        let base_name = format!("{}_{}", service.name.to_lowercase(), method.name.to_lowercase());
        
        if let Some(prefix) = &self.config.tool_prefix {
            format!("{}_{}", prefix, base_name)
        } else {
            base_name
        }
    }

    /// Generate input schema for gRPC method
    pub fn generate_input_schema(&self, method: &GrpcMethod) -> Result<Value> {
        // Create a basic schema based on the method's input type
        let schema = json!({
            "type": "object",
            "properties": {
                // We'll create properties based on the method's input type name
                // In a real implementation, we would parse the actual message definition
                // and create a schema that matches the message structure
            },
            "required": []
        });
        
        // In a real implementation, we would:
        // 1. Parse the input message definition
        // 2. Create properties for each field in the message
        // 3. Set appropriate types, formats, and descriptions
        // 4. Handle nested messages, enums, etc.
        
        // For now, we'll create a simple schema based on the method name
        let mut schema_obj = schema.as_object().unwrap().clone();
        let mut properties = serde_json::Map::new();
        
        // Create properties based on the method name and input type
        if method.input_type.contains("Empty") {
            // No properties for Empty messages
        } else if method.input_type.contains("Request") {
            // Add some common request properties
            if method.name.starts_with("Get") || method.name.starts_with("Retrieve") {
                properties.insert("id".to_string(), json!({
                    "type": "string",
                    "description": "ID of the resource to retrieve"
                }));
            } else if method.name.starts_with("List") || method.name.starts_with("Search") {
                properties.insert("page_size".to_string(), json!({
                    "type": "integer",
                    "description": "Number of items to return",
                    "default": 10
                }));
                
                properties.insert("page_token".to_string(), json!({
                    "type": "string",
                    "description": "Token for pagination"
                }));
                
                properties.insert("filter".to_string(), json!({
                    "type": "string",
                    "description": "Filter expression"
                }));
            } else if method.name.starts_with("Create") || method.name.starts_with("Add") {
                properties.insert("data".to_string(), json!({
                    "type": "object",
                    "description": "Data for the new resource"
                }));
            } else if method.name.starts_with("Update") || method.name.starts_with("Modify") {
                properties.insert("id".to_string(), json!({
                    "type": "string",
                    "description": "ID of the resource to update"
                }));
                
                properties.insert("data".to_string(), json!({
                    "type": "object",
                    "description": "Updated data for the resource"
                }));
            } else if method.name.starts_with("Delete") || method.name.starts_with("Remove") {
                properties.insert("id".to_string(), json!({
                    "type": "string",
                    "description": "ID of the resource to delete"
                }));
            }
        }
        
        // Add the properties to the schema
        schema_obj.insert("properties".to_string(), Value::Object(properties));
        
        Ok(Value::Object(schema_obj))
    }

    /// Create routing configuration for gRPC method
    pub fn create_routing_config(&self, service: &GrpcService, method: &GrpcMethod) -> Result<RoutingConfig> {
        // This is a placeholder implementation that will be filled in with actual code
        // For now, it returns a simple gRPC routing configuration
        let mut config = serde_json::Map::new();
        
        config.insert("endpoint".to_string(), json!(self.config.endpoint));
        config.insert("service".to_string(), json!(format!("{}.{}", service.package, service.name)));
        config.insert("method".to_string(), json!(method.name));
        
        // Add authentication headers if configured
        if let Some(auth_config) = &self.config.auth_config {
            let mut headers = auth_config.headers.clone();
            
            match &auth_config.auth_type {
                AuthType::None => {}
                AuthType::ApiKey { key, header } => {
                    headers.insert(header.clone(), key.clone());
                }
                AuthType::Bearer { token } => {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                }
                AuthType::Basic { username, password } => {
                    let credentials = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                    headers.insert("Authorization".to_string(), format!("Basic {}", credentials));
                }
                AuthType::OAuth { token, token_type } => {
                    headers.insert("Authorization".to_string(), format!("{} {}", token_type, token));
                }
            }
            
            if !headers.is_empty() {
                config.insert("headers".to_string(), json!(headers));
            }
        }
        
        Ok(RoutingConfig::new("grpc".to_string(), Value::Object(config)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_grpc_generator_creation() {
        let config = GrpcGeneratorConfig {
            endpoint: "https://example.com:443".to_string(),
            auth_config: None,
            tool_prefix: None,
            service_filter: None,
            method_filter: None,
            server_streaming_strategy: StreamingStrategy::Polling,
            client_streaming_strategy: StreamingStrategy::Polling,
            bidirectional_streaming_strategy: StreamingStrategy::Polling,
            include_method_options: false,
            separate_streaming_tools: false,
            use_enhanced_format: true,
        };
        
        let generator = GrpcCapabilityGenerator::new(config);
        assert_eq!(generator.config.endpoint, "https://example.com:443");
    }
}