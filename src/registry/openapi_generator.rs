//! OpenAPI Capability Generator
//!
//! This module provides functionality to generate MCP tool definitions from OpenAPI specifications.
//! It supports both OpenAPI 3.0 and Swagger 2.0 formats, parsing JSON and YAML specifications
//! to create corresponding MCP tools for REST API endpoints.

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
use openapiv3::{OpenAPI, Operation, Parameter, RequestBody, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};

/// Authentication configuration for OpenAPI endpoints
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

/// Represents an OpenAPI operation that can be converted to an MCP tool
#[derive(Debug, Clone)]
pub struct OpenAPIOperation {
    /// HTTP method (GET, POST, PUT, PATCH, DELETE)
    pub method: String,
    /// API path (e.g., "/users/{id}")
    pub path: String,
    /// Operation ID from OpenAPI spec
    pub operation_id: Option<String>,
    /// Operation summary
    pub summary: Option<String>,
    /// Operation description
    pub description: Option<String>,
    /// Parameters (path, query, header, cookie)
    pub parameters: Vec<OpenAPIParameter>,
    /// Request body schema
    pub request_body: Option<OpenAPIRequestBody>,
    /// Response schemas
    pub responses: HashMap<String, OpenAPIResponse>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Whether the operation is deprecated
    pub deprecated: bool,
}

/// Represents an OpenAPI parameter
#[derive(Debug, Clone)]
pub struct OpenAPIParameter {
    /// Parameter name
    pub name: String,
    /// Parameter location (path, query, header, cookie)
    pub location: String,
    /// Parameter description
    pub description: Option<String>,
    /// Whether the parameter is required
    pub required: bool,
    /// Parameter schema
    pub schema: Value,
    /// Default value
    pub default: Option<Value>,
}

/// Represents an OpenAPI request body
#[derive(Debug, Clone)]
pub struct OpenAPIRequestBody {
    /// Request body description
    pub description: Option<String>,
    /// Whether the request body is required
    pub required: bool,
    /// Content types and their schemas
    pub content: HashMap<String, Value>,
}

/// Represents an OpenAPI response
#[derive(Debug, Clone)]
pub struct OpenAPIResponse {
    /// Response description
    pub description: String,
    /// Response content schemas
    pub content: HashMap<String, Value>,
    /// Response headers
    pub headers: HashMap<String, Value>,
}

/// OpenAPI Capability Generator
/// 
/// Generates MCP tool definitions from OpenAPI specifications.
/// Supports OpenAPI 3.0 and Swagger 2.0 formats.
pub struct OpenAPICapabilityGenerator {
    /// Base URL for the API
    pub base_url: String,
    /// Authentication configuration
    pub auth_config: Option<AuthConfig>,
    /// Tool name prefix
    pub tool_prefix: Option<String>,
    /// Filter for specific operations (by operation ID or tag)
    pub operation_filter: Option<Vec<String>>,
    /// Filter for specific paths (regex patterns)
    pub path_filter: Option<Vec<String>>,
    /// Filter for specific HTTP methods
    pub method_filter: Option<Vec<String>>,
    /// Custom naming convention for tools
    pub naming_convention: NamingConvention,
    /// Whether to include deprecated operations
    pub include_deprecated: bool,
    /// Whether to generate enhanced MCP 2025-06-18 format
    pub use_enhanced_format: bool,
}

/// Naming convention for generated tools
#[derive(Debug, Clone)]
pub enum NamingConvention {
    /// Use operation ID as tool name
    OperationId,
    /// Use method + path combination
    MethodPath,
    /// Use custom format string
    Custom(String),
}

impl Default for NamingConvention {
    fn default() -> Self {
        NamingConvention::OperationId
    }
}

// ===== Swagger 2.0 Data Structures =====

/// Swagger 2.0 specification root
#[derive(Debug, Clone, Deserialize)]
pub struct Swagger2Spec {
    pub swagger: String,
    pub info: Swagger2Info,
    pub host: Option<String>,
    #[serde(rename = "basePath")]
    pub base_path: Option<String>,
    pub schemes: Option<Vec<String>>,
    pub paths: HashMap<String, Swagger2PathItem>,
    pub definitions: Option<HashMap<String, Swagger2Schema>>,
    pub parameters: Option<HashMap<String, Swagger2Parameter>>,
    pub responses: Option<HashMap<String, Swagger2Response>>,
}

/// Swagger 2.0 info object
#[derive(Debug, Clone, Deserialize)]
pub struct Swagger2Info {
    pub title: String,
    pub description: Option<String>,
    pub version: String,
}

/// Swagger 2.0 path item
#[derive(Debug, Clone, Deserialize)]
pub struct Swagger2PathItem {
    pub get: Option<Swagger2Operation>,
    pub post: Option<Swagger2Operation>,
    pub put: Option<Swagger2Operation>,
    pub delete: Option<Swagger2Operation>,
    pub options: Option<Swagger2Operation>,
    pub head: Option<Swagger2Operation>,
    pub patch: Option<Swagger2Operation>,
    pub parameters: Option<Vec<Swagger2Parameter>>,
}

/// Swagger 2.0 operation
#[derive(Debug, Clone, Deserialize)]
pub struct Swagger2Operation {
    pub summary: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "operationId")]
    pub operation_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub parameters: Option<Vec<Swagger2Parameter>>,
    pub responses: HashMap<String, Swagger2Response>,
    pub deprecated: Option<bool>,
}

/// Swagger 2.0 parameter
#[derive(Debug, Clone, Deserialize)]
pub struct Swagger2Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    pub description: Option<String>,
    pub required: Option<bool>,
    #[serde(rename = "type")]
    pub param_type: Option<String>,
    pub format: Option<String>,
    pub schema: Option<Box<Swagger2Schema>>,
    pub default: Option<Value>,
}

/// Swagger 2.0 response
#[derive(Debug, Clone, Deserialize)]
pub struct Swagger2Response {
    pub description: String,
    pub schema: Option<Box<Swagger2Schema>>,
    pub headers: Option<HashMap<String, Swagger2Header>>,
}

/// Swagger 2.0 header
#[derive(Debug, Clone, Deserialize)]
pub struct Swagger2Header {
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub header_type: String,
    pub format: Option<String>,
}

/// Swagger 2.0 schema
#[derive(Debug, Clone, Deserialize)]
pub struct Swagger2Schema {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub format: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
    pub properties: Option<HashMap<String, Swagger2Schema>>,
    pub items: Option<Box<Swagger2Schema>>,
    pub required: Option<Vec<String>>,
    pub default: Option<Value>,
    #[serde(rename = "enum")]
    pub enumeration: Option<Vec<Value>>,
    // Extensions field to fix the SchemaData struct issue mentioned in TODO
    #[serde(flatten)]
    pub extensions: HashMap<String, Value>,
}

impl OpenAPICapabilityGenerator {
    /// Create a new OpenAPI capability generator
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            auth_config: None,
            tool_prefix: None,
            operation_filter: None,
            path_filter: None,
            method_filter: None,
            naming_convention: NamingConvention::default(),
            include_deprecated: false,
            use_enhanced_format: true, // Default to enhanced format
        }
    }

    /// Set authentication configuration
    pub fn with_auth(mut self, auth_config: AuthConfig) -> Self {
        self.auth_config = Some(auth_config);
        self
    }

    /// Set tool name prefix
    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.tool_prefix = Some(prefix);
        self
    }

    /// Set operation filter (by operation ID or tag)
    pub fn with_operation_filter(mut self, filter: Vec<String>) -> Self {
        self.operation_filter = Some(filter);
        self
    }

    /// Set path filter (regex patterns)
    pub fn with_path_filter(mut self, filter: Vec<String>) -> Self {
        self.path_filter = Some(filter);
        self
    }

    /// Set HTTP method filter
    pub fn with_method_filter(mut self, filter: Vec<String>) -> Self {
        self.method_filter = Some(filter);
        self
    }

    /// Set naming convention
    pub fn with_naming_convention(mut self, convention: NamingConvention) -> Self {
        self.naming_convention = convention;
        self
    }

    /// Include deprecated operations
    pub fn include_deprecated(mut self) -> Self {
        self.include_deprecated = true;
        self
    }

    /// Use enhanced MCP 2025-06-18 format
    pub fn with_enhanced_format(mut self, enhanced: bool) -> Self {
        self.use_enhanced_format = enhanced;
        self
    }

    /// Generate capability file from OpenAPI 3.0 specification
    pub fn generate_from_openapi3(&mut self, spec_content: &str) -> Result<CapabilityFile> {
        let openapi_spec = self.parse_openapi3_spec(spec_content)?;
        let operations = self.extract_operations_from_openapi3(&openapi_spec)?;
        self.generate_capability_file(operations)
    }

    /// Generate capability file from Swagger 2.0 specification
    pub fn generate_from_swagger2(&mut self, spec_content: &str) -> Result<CapabilityFile> {
        let swagger_spec = self.parse_swagger2_spec(spec_content)?;
        let operations = self.extract_operations_from_swagger2(&swagger_spec)?;
        self.generate_capability_file(operations)
    }

    /// Auto-detect format and generate capability file
    pub fn generate_from_spec(&mut self, spec_content: &str) -> Result<CapabilityFile> {
        // Try to detect format by parsing as JSON first
        if let Ok(spec_json) = serde_json::from_str::<Value>(spec_content) {
            if spec_json.get("openapi").is_some() {
                return self.generate_from_openapi3(spec_content);
            } else if spec_json.get("swagger").is_some() {
                return self.generate_from_swagger2(spec_content);
            }
        }

        // Try YAML parsing
        if let Ok(spec_yaml) = serde_yaml::from_str::<Value>(spec_content) {
            if spec_yaml.get("openapi").is_some() {
                return self.generate_from_openapi3(spec_content);
            } else if spec_yaml.get("swagger").is_some() {
                return self.generate_from_swagger2(spec_content);
            }
        }

        Err(ProxyError::config("Unable to detect OpenAPI/Swagger format"))
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_type: AuthType::None,
            headers: HashMap::new(),
        }
    }
}

impl OpenAPICapabilityGenerator {
    /// Parse OpenAPI 3.0 specification from JSON or YAML
    fn parse_openapi3_spec(&self, spec_content: &str) -> Result<OpenAPI> {
        // Try JSON first
        if let Ok(spec) = serde_json::from_str::<OpenAPI>(spec_content) {
            return Ok(spec);
        }

        // Try YAML
        serde_yaml::from_str::<OpenAPI>(spec_content)
            .map_err(|e| ProxyError::validation(format!("Failed to parse OpenAPI specification: {}", e)))
    }

    /// Extract operations from OpenAPI 3.0 specification
    fn extract_operations_from_openapi3(&self, spec: &OpenAPI) -> Result<Vec<OpenAPIOperation>> {
        let mut operations = Vec::new();

        for (path, path_item) in &spec.paths.paths {
            if let Some(path_item) = path_item.as_item() {
                // Extract operations for each HTTP method
                self.extract_operation_if_present(&mut operations, path, "GET", &path_item.get)?;
                self.extract_operation_if_present(&mut operations, path, "POST", &path_item.post)?;
                self.extract_operation_if_present(&mut operations, path, "PUT", &path_item.put)?;
                self.extract_operation_if_present(&mut operations, path, "PATCH", &path_item.patch)?;
                self.extract_operation_if_present(&mut operations, path, "DELETE", &path_item.delete)?;
                self.extract_operation_if_present(&mut operations, path, "HEAD", &path_item.head)?;
                self.extract_operation_if_present(&mut operations, path, "OPTIONS", &path_item.options)?;
                self.extract_operation_if_present(&mut operations, path, "TRACE", &path_item.trace)?;
            }
        }

        // Apply filters
        self.apply_filters(operations)
    }

    /// Extract operation if present and convert to OpenAPIOperation
    fn extract_operation_if_present(
        &self,
        operations: &mut Vec<OpenAPIOperation>,
        path: &str,
        method: &str,
        operation: &Option<Operation>,
    ) -> Result<()> {
        if let Some(op) = operation {
            let openapi_operation = self.convert_operation(path, method, op)?;
            operations.push(openapi_operation);
        }
        Ok(())
    }

    /// Convert OpenAPI Operation to OpenAPIOperation
    fn convert_operation(&self, path: &str, method: &str, operation: &Operation) -> Result<OpenAPIOperation> {
        // Extract parameters
        let mut parameters = Vec::new();
        for param_ref in &operation.parameters {
            if let Some(param) = param_ref.as_item() {
                parameters.push(self.convert_parameter(param)?);
            }
        }

        // Extract request body
        let request_body = if let Some(req_body_ref) = &operation.request_body {
            if let Some(req_body) = req_body_ref.as_item() {
                Some(self.convert_request_body(req_body)?)
            } else {
                None
            }
        } else {
            None
        };

        // Extract responses
        let mut responses = HashMap::new();
        for (status_code, response_ref) in &operation.responses.responses {
            if let Some(response) = response_ref.as_item() {
                responses.insert(status_code.to_string(), self.convert_response(response)?);
            }
        }

        Ok(OpenAPIOperation {
            method: method.to_uppercase(),
            path: path.to_string(),
            operation_id: operation.operation_id.clone(),
            summary: operation.summary.clone(),
            description: operation.description.clone(),
            parameters,
            request_body,
            responses,
            tags: operation.tags.clone(),
            deprecated: operation.deprecated,
        })
    }

    /// Convert OpenAPI Parameter to OpenAPIParameter
    fn convert_parameter(&self, parameter: &Parameter) -> Result<OpenAPIParameter> {
        let (name, location, description, required, schema) = match parameter {
            Parameter::Query { parameter_data, .. } => (
                parameter_data.name.clone(),
                "query".to_string(),
                parameter_data.description.clone(),
                parameter_data.required,
                self.extract_parameter_schema(parameter)?,
            ),
            Parameter::Header { parameter_data, .. } => (
                parameter_data.name.clone(),
                "header".to_string(),
                parameter_data.description.clone(),
                parameter_data.required,
                self.extract_parameter_schema(parameter)?,
            ),
            Parameter::Path { parameter_data, .. } => (
                parameter_data.name.clone(),
                "path".to_string(),
                parameter_data.description.clone(),
                true, // Path parameters are always required
                self.extract_parameter_schema(parameter)?,
            ),
            Parameter::Cookie { parameter_data, .. } => (
                parameter_data.name.clone(),
                "cookie".to_string(),
                parameter_data.description.clone(),
                parameter_data.required,
                self.extract_parameter_schema(parameter)?,
            ),
        };

        Ok(OpenAPIParameter {
            name,
            location,
            description,
            required,
            schema,
            default: None, // TODO: Extract default values
        })
    }

    /// Extract schema from parameter
    fn extract_parameter_schema(&self, parameter: &Parameter) -> Result<Value> {
        let parameter_data = parameter.parameter_data_ref();

        // Extract actual schema from parameter if available
        match parameter {
            Parameter::Query { parameter_data, allow_reserved: _, style: _, allow_empty_value: _ } => {
                match &parameter_data.format {
                    openapiv3::ParameterSchemaOrContent::Schema(schema_ref) => {
                        match schema_ref {
                            openapiv3::ReferenceOr::Reference { reference } => {
                                self.resolve_schema_reference(reference)
                            },
                            openapiv3::ReferenceOr::Item(schema) => {
                                self.convert_schema_with_inheritance(schema)
                            },
                        }
                    },
                    openapiv3::ParameterSchemaOrContent::Content(_) => {
                        self.create_basic_parameter_schema(parameter_data)
                    },
                }
            },
            Parameter::Header { parameter_data, style: _ } => {
                match &parameter_data.format {
                    openapiv3::ParameterSchemaOrContent::Schema(schema_ref) => {
                        match schema_ref {
                            openapiv3::ReferenceOr::Reference { reference } => {
                                self.resolve_schema_reference(reference)
                            },
                            openapiv3::ReferenceOr::Item(schema) => {
                                self.convert_openapi_schema_to_json_schema(schema)
                            },
                        }
                    },
                    openapiv3::ParameterSchemaOrContent::Content(_) => {
                        self.create_basic_parameter_schema(parameter_data)
                    },
                }
            },
            Parameter::Path { parameter_data, style: _ } => {
                match &parameter_data.format {
                    openapiv3::ParameterSchemaOrContent::Schema(schema_ref) => {
                        match schema_ref {
                            openapiv3::ReferenceOr::Reference { reference } => {
                                self.resolve_schema_reference(reference)
                            },
                            openapiv3::ReferenceOr::Item(schema) => {
                                self.convert_openapi_schema_to_json_schema(schema)
                            },
                        }
                    },
                    openapiv3::ParameterSchemaOrContent::Content(_) => {
                        self.create_basic_parameter_schema(parameter_data)
                    },
                }
            },
            Parameter::Cookie { parameter_data, style: _ } => {
                match &parameter_data.format {
                    openapiv3::ParameterSchemaOrContent::Schema(schema_ref) => {
                        match schema_ref {
                            openapiv3::ReferenceOr::Reference { reference } => {
                                self.resolve_schema_reference(reference)
                            },
                            openapiv3::ReferenceOr::Item(schema) => {
                                self.convert_openapi_schema_to_json_schema(schema)
                            },
                        }
                    },
                    openapiv3::ParameterSchemaOrContent::Content(_) => {
                        self.create_basic_parameter_schema(parameter_data)
                    },
                }
            },
        }
    }

    /// Create basic parameter schema when no complex schema is available
    fn create_basic_parameter_schema(&self, parameter_data: &openapiv3::ParameterData) -> Result<Value> {
        Ok(json!({
            "type": "string",
            "description": parameter_data.description.clone().unwrap_or_else(|| "Parameter".to_string())
        }))
    }

    /// Resolve schema reference (handles $ref and components)
    fn resolve_schema_reference(&self, schema_ref: &str) -> Result<Value> {
        // Handle $ref references
        if schema_ref.starts_with("#/") {
            // This is a JSON Pointer reference
            let reference_path = schema_ref.trim_start_matches("#/");

            // Parse the reference path to understand what it's pointing to
            if reference_path.starts_with("components/schemas/") {
                let schema_name = reference_path.trim_start_matches("components/schemas/");
                Ok(json!({
                    "$ref": schema_ref,
                    "description": format!("Reference to schema component: {}", schema_name),
                    "type": "object",
                    "x-component-name": schema_name,
                    "x-component-type": "schema"
                }))
            } else if reference_path.starts_with("components/parameters/") {
                let param_name = reference_path.trim_start_matches("components/parameters/");
                Ok(json!({
                    "$ref": schema_ref,
                    "description": format!("Reference to parameter component: {}", param_name),
                    "type": "string",
                    "x-component-name": param_name,
                    "x-component-type": "parameter"
                }))
            } else if reference_path.starts_with("components/responses/") {
                let response_name = reference_path.trim_start_matches("components/responses/");
                Ok(json!({
                    "$ref": schema_ref,
                    "description": format!("Reference to response component: {}", response_name),
                    "type": "object",
                    "x-component-name": response_name,
                    "x-component-type": "response"
                }))
            } else if reference_path.starts_with("definitions/") {
                // Swagger 2.0 style reference
                let definition_name = reference_path.trim_start_matches("definitions/");
                Ok(json!({
                    "$ref": schema_ref,
                    "description": format!("Reference to definition: {}", definition_name),
                    "type": "object",
                    "x-definition-name": definition_name,
                    "x-swagger-version": "2.0"
                }))
            } else {
                // Generic JSON Pointer reference
                Ok(json!({
                    "$ref": schema_ref,
                    "description": format!("Reference to {}", reference_path),
                    "type": "object",
                    "x-reference-path": reference_path
                }))
            }
        } else if schema_ref.starts_with("http://") || schema_ref.starts_with("https://") {
            // Handle external HTTP references
            Ok(json!({
                "$ref": schema_ref,
                "type": "object",
                "description": format!("External HTTP reference: {}", schema_ref),
                "x-external-reference": true
            }))
        } else {
            // Handle other external references or relative references
            Ok(json!({
                "$ref": schema_ref,
                "type": "object",
                "description": format!("External reference: {}", schema_ref),
                "x-external-reference": true
            }))
        }
    }

    /// Convert complex OpenAPI schema to JSON Schema
    fn convert_openapi_schema_to_json_schema(&self, schema: &openapiv3::Schema) -> Result<Value> {
        let mut json_schema = serde_json::Map::new();

        // Extract schema data
        if let Some(description) = &schema.schema_data.description {
            json_schema.insert("description".to_string(), json!(description));
        }

        if let Some(default) = &schema.schema_data.default {
            json_schema.insert("default".to_string(), default.clone());
        }

        // Handle schema kind
        match &schema.schema_kind {
            openapiv3::SchemaKind::Type(type_schema) => {
                self.convert_type_schema_to_json_schema(type_schema, &mut json_schema)?;
            },
            openapiv3::SchemaKind::OneOf { one_of } => {
                let mut one_of_schemas = Vec::new();
                for schema_ref in one_of {
                    let resolved_schema = match schema_ref {
                        openapiv3::ReferenceOr::Reference { reference } => {
                            self.resolve_schema_reference(reference)?
                        },
                        openapiv3::ReferenceOr::Item(schema) => {
                            self.convert_openapi_schema_to_json_schema(schema)?
                        },
                    };
                    one_of_schemas.push(resolved_schema);
                }
                json_schema.insert("oneOf".to_string(), json!(one_of_schemas));
                json_schema.insert("description".to_string(), json!("OneOf schema - exactly one of the schemas must match"));
            },
            openapiv3::SchemaKind::AllOf { all_of } => {
                let mut all_of_schemas = Vec::new();
                for schema_ref in all_of {
                    let resolved_schema = match schema_ref {
                        openapiv3::ReferenceOr::Reference { reference } => {
                            self.resolve_schema_reference(reference)?
                        },
                        openapiv3::ReferenceOr::Item(schema) => {
                            self.convert_openapi_schema_to_json_schema(schema)?
                        },
                    };
                    all_of_schemas.push(resolved_schema);
                }
                json_schema.insert("allOf".to_string(), json!(all_of_schemas));
                json_schema.insert("description".to_string(), json!("AllOf schema - all schemas must match"));
            },
            openapiv3::SchemaKind::AnyOf { any_of } => {
                let mut any_of_schemas = Vec::new();
                for schema_ref in any_of {
                    let resolved_schema = match schema_ref {
                        openapiv3::ReferenceOr::Reference { reference } => {
                            self.resolve_schema_reference(reference)?
                        },
                        openapiv3::ReferenceOr::Item(schema) => {
                            self.convert_openapi_schema_to_json_schema(schema)?
                        },
                    };
                    any_of_schemas.push(resolved_schema);
                }
                json_schema.insert("anyOf".to_string(), json!(any_of_schemas));
                json_schema.insert("description".to_string(), json!("AnyOf schema - any of the schemas may match"));
            },
            openapiv3::SchemaKind::Not { not: _ } => {
                json_schema.insert("type".to_string(), json!("object"));
                json_schema.insert("description".to_string(), json!("Not schema"));
            },
            openapiv3::SchemaKind::Any(_) => {
                json_schema.insert("type".to_string(), json!("object"));
                json_schema.insert("description".to_string(), json!("Any schema"));
            },
        }

        Ok(Value::Object(json_schema))
    }

    /// Convert OpenAPI Type schema to JSON Schema
    fn convert_type_schema_to_json_schema(&self, type_schema: &openapiv3::Type, json_schema: &mut serde_json::Map<String, Value>) -> Result<()> {
        match type_schema {
            openapiv3::Type::String(string_type) => {
                json_schema.insert("type".to_string(), json!("string"));
                match &string_type.format {
                    openapiv3::VariantOrUnknownOrEmpty::Item(string_format) => {
                        json_schema.insert("format".to_string(), json!(format!("{:?}", string_format)));
                    },
                    openapiv3::VariantOrUnknownOrEmpty::Unknown(format_str) => {
                        json_schema.insert("format".to_string(), json!(format_str));
                    },
                    openapiv3::VariantOrUnknownOrEmpty::Empty => {},
                }
                if !string_type.enumeration.is_empty() {
                    json_schema.insert("enum".to_string(), json!(string_type.enumeration));
                }
            },
            openapiv3::Type::Number(number_type) => {
                json_schema.insert("type".to_string(), json!("number"));
                match &number_type.format {
                    openapiv3::VariantOrUnknownOrEmpty::Item(number_format) => {
                        json_schema.insert("format".to_string(), json!(format!("{:?}", number_format)));
                    },
                    openapiv3::VariantOrUnknownOrEmpty::Unknown(format_str) => {
                        json_schema.insert("format".to_string(), json!(format_str));
                    },
                    openapiv3::VariantOrUnknownOrEmpty::Empty => {},
                }
                if let Some(minimum) = number_type.minimum {
                    json_schema.insert("minimum".to_string(), json!(minimum));
                }
                if let Some(maximum) = number_type.maximum {
                    json_schema.insert("maximum".to_string(), json!(maximum));
                }
            },
            openapiv3::Type::Integer(integer_type) => {
                json_schema.insert("type".to_string(), json!("integer"));
                match &integer_type.format {
                    openapiv3::VariantOrUnknownOrEmpty::Item(integer_format) => {
                        json_schema.insert("format".to_string(), json!(format!("{:?}", integer_format)));
                    },
                    openapiv3::VariantOrUnknownOrEmpty::Unknown(format_str) => {
                        json_schema.insert("format".to_string(), json!(format_str));
                    },
                    openapiv3::VariantOrUnknownOrEmpty::Empty => {},
                }
                if let Some(minimum) = integer_type.minimum {
                    json_schema.insert("minimum".to_string(), json!(minimum));
                }
                if let Some(maximum) = integer_type.maximum {
                    json_schema.insert("maximum".to_string(), json!(maximum));
                }
            },
            openapiv3::Type::Object(object_type) => {
                json_schema.insert("type".to_string(), json!("object"));

                // Handle properties with recursive schema resolution
                if !object_type.properties.is_empty() {
                    let mut properties = serde_json::Map::new();
                    for (prop_name, prop_schema_ref) in &object_type.properties {
                        // Recursively resolve property schemas
                        let prop_schema = match prop_schema_ref {
                            openapiv3::ReferenceOr::Reference { reference } => {
                                self.resolve_schema_reference(reference)?
                            },
                            openapiv3::ReferenceOr::Item(schema) => {
                                self.convert_openapi_schema_to_json_schema(schema)?
                            },
                        };
                        properties.insert(prop_name.clone(), prop_schema);
                    }
                    json_schema.insert("properties".to_string(), Value::Object(properties));
                }

                // Handle required fields
                if !object_type.required.is_empty() {
                    json_schema.insert("required".to_string(), json!(object_type.required));
                }
            },
            openapiv3::Type::Array(array_type) => {
                json_schema.insert("type".to_string(), json!("array"));

                // Handle array items with recursive schema resolution
                if let Some(items_ref) = &array_type.items {
                    let items_schema = match items_ref {
                        openapiv3::ReferenceOr::Reference { reference } => {
                            self.resolve_schema_reference(reference)?
                        },
                        openapiv3::ReferenceOr::Item(schema) => {
                            self.convert_openapi_schema_to_json_schema(schema)?
                        },
                    };
                    json_schema.insert("items".to_string(), items_schema);
                }

                if let Some(min_items) = array_type.min_items {
                    json_schema.insert("minItems".to_string(), json!(min_items));
                }
                if let Some(max_items) = array_type.max_items {
                    json_schema.insert("maxItems".to_string(), json!(max_items));
                }
                if array_type.unique_items {
                    json_schema.insert("uniqueItems".to_string(), json!(true));
                }
            },
            openapiv3::Type::Boolean(_) => {
                json_schema.insert("type".to_string(), json!("boolean"));
            },
        }

        Ok(())
    }

    /// Validate and enhance schema conversion
    fn validate_and_enhance_schema(&self, mut schema: Value) -> Result<Value> {
        // Add validation metadata
        if let Some(schema_obj) = schema.as_object_mut() {
            // Add schema validation metadata
            schema_obj.insert("x-mcp-generated".to_string(), json!(true));
            schema_obj.insert("x-openapi-version".to_string(), json!("3.0"));

            // Ensure required fields have proper defaults
            if !schema_obj.contains_key("type") {
                schema_obj.insert("type".to_string(), json!("object"));
            }

            // Add description if missing
            if !schema_obj.contains_key("description") {
                schema_obj.insert("description".to_string(), json!("Auto-generated schema from OpenAPI specification"));
            }

            // Validate and fix common schema issues
            if let Some(schema_type) = schema_obj.get("type") {
                match schema_type.as_str() {
                    Some("object") => {
                        // Ensure object schemas have properties field
                        if !schema_obj.contains_key("properties") {
                            schema_obj.insert("properties".to_string(), json!({}));
                        }
                    },
                    Some("array") => {
                        // Ensure array schemas have items field
                        if !schema_obj.contains_key("items") {
                            schema_obj.insert("items".to_string(), json!({
                                "type": "string",
                                "description": "Array item"
                            }));
                        }
                    },
                    Some("string") => {
                        // Add default format for string types if not present
                        if !schema_obj.contains_key("format") {
                            schema_obj.insert("format".to_string(), json!("text"));
                        }
                    },
                    _ => {}
                }
            }

            // Add additional validation constraints
            if schema_obj.contains_key("$ref") {
                schema_obj.insert("x-reference-resolved".to_string(), json!(false));
                schema_obj.insert("x-reference-type".to_string(), json!("openapi"));
            }
        }

        Ok(schema)
    }

    /// Enhanced schema conversion with inheritance support
    fn convert_schema_with_inheritance(&self, schema: &openapiv3::Schema) -> Result<Value> {
        let mut base_schema = self.convert_openapi_schema_to_json_schema(schema)?;

        // Handle inheritance patterns
        match &schema.schema_kind {
            openapiv3::SchemaKind::AllOf { all_of } => {
                // AllOf typically represents inheritance
                let mut merged_properties = serde_json::Map::new();
                let mut merged_required = Vec::new();

                for schema_ref in all_of {
                    let resolved_schema = match schema_ref {
                        openapiv3::ReferenceOr::Reference { reference } => {
                            self.resolve_schema_reference(reference)?
                        },
                        openapiv3::ReferenceOr::Item(schema) => {
                            self.convert_openapi_schema_to_json_schema(schema)?
                        },
                    };

                    // Merge properties from each schema
                    if let Some(properties) = resolved_schema.get("properties").and_then(|p| p.as_object()) {
                        for (prop_name, prop_value) in properties {
                            merged_properties.insert(prop_name.clone(), prop_value.clone());
                        }
                    }

                    // Merge required fields
                    if let Some(required) = resolved_schema.get("required").and_then(|r| r.as_array()) {
                        for req_field in required {
                            if let Some(field_name) = req_field.as_str() {
                                if !merged_required.contains(&field_name.to_string()) {
                                    merged_required.push(field_name.to_string());
                                }
                            }
                        }
                    }
                }

                // Update the base schema with merged properties
                if let Some(base_obj) = base_schema.as_object_mut() {
                    base_obj.insert("properties".to_string(), Value::Object(merged_properties));
                    if !merged_required.is_empty() {
                        base_obj.insert("required".to_string(), json!(merged_required));
                    }
                    base_obj.insert("x-inheritance-pattern".to_string(), json!("allOf"));
                }
            },
            _ => {
                // No special inheritance handling needed
            }
        }

        self.validate_and_enhance_schema(base_schema)
    }

    /// Convert OpenAPI RequestBody to OpenAPIRequestBody
    fn convert_request_body(&self, request_body: &RequestBody) -> Result<OpenAPIRequestBody> {
        let mut content = HashMap::new();

        for (media_type, media_type_object) in &request_body.content {
            // Enhanced schema extraction with proper schema parsing
            let schema = if let Some(schema_ref) = &media_type_object.schema {
                match schema_ref {
                    openapiv3::ReferenceOr::Reference { reference } => {
                        self.resolve_schema_reference(reference)?
                    },
                    openapiv3::ReferenceOr::Item(schema) => {
                        self.convert_schema_with_inheritance(schema)?
                    },
                }
            } else {
                json!({
                    "type": "object",
                    "description": format!("Request body for {}", media_type)
                })
            };
            content.insert(media_type.clone(), schema);
        }

        Ok(OpenAPIRequestBody {
            description: request_body.description.clone(),
            required: request_body.required,
            content,
        })
    }

    /// Convert OpenAPI Response to OpenAPIResponse
    fn convert_response(&self, response: &Response) -> Result<OpenAPIResponse> {
        let mut content = HashMap::new();
        let mut headers = HashMap::new();

        // Enhanced content schema extraction
        for (media_type, media_type_object) in &response.content {
            let schema = if let Some(schema_ref) = &media_type_object.schema {
                match schema_ref {
                    openapiv3::ReferenceOr::Reference { reference } => {
                        self.resolve_schema_reference(reference)?
                    },
                    openapiv3::ReferenceOr::Item(schema) => {
                        self.convert_schema_with_inheritance(schema)?
                    },
                }
            } else {
                json!({
                    "type": "object",
                    "description": format!("Response content for {}", media_type)
                })
            };
            content.insert(media_type.clone(), schema);
        }

        // Enhanced header extraction
        for (header_name, header_ref) in &response.headers {
            let header_schema = match header_ref {
                openapiv3::ReferenceOr::Reference { reference } => {
                    self.resolve_schema_reference(reference)?
                },
                openapiv3::ReferenceOr::Item(header) => {
                    // Headers in OpenAPI 3.0 don't have a schema field like parameters do
                    // Instead, they have format information directly
                    let mut header_json = serde_json::Map::new();
                    header_json.insert("type".to_string(), json!("string"));

                    if let Some(description) = &header.description {
                        header_json.insert("description".to_string(), json!(description));
                    } else {
                        header_json.insert("description".to_string(), json!(format!("Response header: {}", header_name)));
                    }

                    // Add format information if available
                    match &header.format {
                        openapiv3::ParameterSchemaOrContent::Schema(schema_ref) => {
                            // Handle schema-based header format
                            match schema_ref {
                                openapiv3::ReferenceOr::Reference { reference } => {
                                    if let Ok(resolved_schema) = self.resolve_schema_reference(reference) {
                                        if let Some(format_value) = resolved_schema.get("format") {
                                            header_json.insert("format".to_string(), format_value.clone());
                                        }
                                    }
                                },
                                openapiv3::ReferenceOr::Item(schema) => {
                                    if let Ok(converted_schema) = self.convert_openapi_schema_to_json_schema(schema) {
                                        if let Some(format_value) = converted_schema.get("format") {
                                            header_json.insert("format".to_string(), format_value.clone());
                                        }
                                    }
                                },
                            }
                        },
                        openapiv3::ParameterSchemaOrContent::Content(_) => {
                            // Handle content-based header format (less common)
                            header_json.insert("format".to_string(), json!("content"));
                        },
                    }

                    Value::Object(header_json)
                },
            };
            headers.insert(header_name.clone(), header_schema);
        }

        Ok(OpenAPIResponse {
            description: response.description.clone(),
            content,
            headers,
        })
    }

    /// Apply filters to operations
    fn apply_filters(&self, mut operations: Vec<OpenAPIOperation>) -> Result<Vec<OpenAPIOperation>> {
        // Filter by deprecated status
        if !self.include_deprecated {
            operations.retain(|op| !op.deprecated);
        }

        // Filter by HTTP methods
        if let Some(method_filter) = &self.method_filter {
            operations.retain(|op| method_filter.contains(&op.method));
        }

        // Filter by operation ID or tags
        if let Some(operation_filter) = &self.operation_filter {
            operations.retain(|op| {
                // Check operation ID
                if let Some(op_id) = &op.operation_id {
                    if operation_filter.contains(op_id) {
                        return true;
                    }
                }
                // Check tags
                op.tags.iter().any(|tag| operation_filter.contains(tag))
            });
        }

        // Filter by path patterns
        if let Some(path_filter) = &self.path_filter {
            operations.retain(|op| {
                path_filter.iter().any(|pattern| {
                    // Simple pattern matching - could be enhanced with regex
                    op.path.contains(pattern)
                })
            });
        }

        Ok(operations)
    }

    // ===== Swagger 2.0 Parsing and Conversion Methods =====

    /// Parse Swagger 2.0 specification from JSON or YAML
    fn parse_swagger2_spec(&self, spec_content: &str) -> Result<Swagger2Spec> {
        // Try JSON first
        if let Ok(spec) = serde_json::from_str::<Swagger2Spec>(spec_content) {
            return Ok(spec);
        }

        // Try YAML
        serde_yaml::from_str::<Swagger2Spec>(spec_content)
            .map_err(|e| ProxyError::validation(format!("Failed to parse Swagger 2.0 specification: {}", e)))
    }

    /// Extract operations from Swagger 2.0 specification
    fn extract_operations_from_swagger2(&self, spec: &Swagger2Spec) -> Result<Vec<OpenAPIOperation>> {
        let mut operations = Vec::new();

        for (path, path_item) in &spec.paths {
            // Extract operations for each HTTP method
            self.extract_swagger2_operation_if_present(&mut operations, path, "GET", &path_item.get)?;
            self.extract_swagger2_operation_if_present(&mut operations, path, "POST", &path_item.post)?;
            self.extract_swagger2_operation_if_present(&mut operations, path, "PUT", &path_item.put)?;
            self.extract_swagger2_operation_if_present(&mut operations, path, "PATCH", &path_item.patch)?;
            self.extract_swagger2_operation_if_present(&mut operations, path, "DELETE", &path_item.delete)?;
            self.extract_swagger2_operation_if_present(&mut operations, path, "HEAD", &path_item.head)?;
            self.extract_swagger2_operation_if_present(&mut operations, path, "OPTIONS", &path_item.options)?;
        }

        // Apply filters
        self.apply_filters(operations)
    }

    /// Extract Swagger 2.0 operation if present and convert to OpenAPIOperation
    fn extract_swagger2_operation_if_present(
        &self,
        operations: &mut Vec<OpenAPIOperation>,
        path: &str,
        method: &str,
        operation: &Option<Swagger2Operation>,
    ) -> Result<()> {
        if let Some(op) = operation {
            let openapi_operation = self.convert_swagger2_operation(path, method, op)?;
            operations.push(openapi_operation);
        }
        Ok(())
    }

    /// Convert Swagger 2.0 Operation to OpenAPIOperation
    fn convert_swagger2_operation(&self, path: &str, method: &str, operation: &Swagger2Operation) -> Result<OpenAPIOperation> {
        // Extract parameters
        let mut parameters = Vec::new();
        if let Some(params) = &operation.parameters {
            for param in params {
                parameters.push(self.convert_swagger2_parameter(param)?);
            }
        }

        // Extract request body (from body parameters in Swagger 2.0)
        let request_body = self.extract_swagger2_request_body(&parameters)?;

        // Extract responses
        let mut responses = HashMap::new();
        for (status_code, response) in &operation.responses {
            responses.insert(status_code.clone(), self.convert_swagger2_response(response)?);
        }

        Ok(OpenAPIOperation {
            method: method.to_uppercase(),
            path: path.to_string(),
            operation_id: operation.operation_id.clone(),
            summary: operation.summary.clone(),
            description: operation.description.clone(),
            parameters,
            request_body,
            responses,
            tags: operation.tags.clone().unwrap_or_default(),
            deprecated: operation.deprecated.unwrap_or(false),
        })
    }

    /// Convert Swagger 2.0 Parameter to OpenAPIParameter
    fn convert_swagger2_parameter(&self, parameter: &Swagger2Parameter) -> Result<OpenAPIParameter> {
        // Convert Swagger 2.0 parameter type to JSON schema
        let schema = if let Some(schema) = &parameter.schema {
            self.convert_swagger2_schema_to_json_schema(schema)?
        } else {
            // Create schema from type and format
            let mut schema_obj = serde_json::Map::new();
            if let Some(param_type) = &parameter.param_type {
                schema_obj.insert("type".to_string(), json!(param_type));
            } else {
                schema_obj.insert("type".to_string(), json!("string"));
            }

            if let Some(format) = &parameter.format {
                schema_obj.insert("format".to_string(), json!(format));
            }

            if let Some(description) = &parameter.description {
                schema_obj.insert("description".to_string(), json!(description));
            }

            if let Some(default) = &parameter.default {
                schema_obj.insert("default".to_string(), default.clone());
            }

            Value::Object(schema_obj)
        };

        Ok(OpenAPIParameter {
            name: parameter.name.clone(),
            location: parameter.location.clone(),
            description: parameter.description.clone(),
            required: parameter.required.unwrap_or(false),
            schema,
            default: parameter.default.clone(),
        })
    }

    /// Extract request body from Swagger 2.0 body parameters
    fn extract_swagger2_request_body(&self, parameters: &[OpenAPIParameter]) -> Result<Option<OpenAPIRequestBody>> {
        // In Swagger 2.0, request body is represented as a parameter with location "body"
        for param in parameters {
            if param.location == "body" {
                let mut content = HashMap::new();
                content.insert("application/json".to_string(), param.schema.clone());

                return Ok(Some(OpenAPIRequestBody {
                    description: param.description.clone(),
                    required: param.required,
                    content,
                }));
            }
        }
        Ok(None)
    }

    /// Convert Swagger 2.0 Response to OpenAPIResponse
    fn convert_swagger2_response(&self, response: &Swagger2Response) -> Result<OpenAPIResponse> {
        let mut content = HashMap::new();
        let mut headers = HashMap::new();

        // Convert schema to content
        if let Some(schema) = &response.schema {
            let json_schema = self.convert_swagger2_schema_to_json_schema(schema)?;
            content.insert("application/json".to_string(), json_schema);
        }

        // Convert headers
        if let Some(response_headers) = &response.headers {
            for (header_name, header) in response_headers {
                headers.insert(header_name.clone(), json!({
                    "type": header.header_type,
                    "format": header.format,
                    "description": header.description
                }));
            }
        }

        Ok(OpenAPIResponse {
            description: response.description.clone(),
            content,
            headers,
        })
    }

    /// Convert Swagger 2.0 Schema to JSON Schema
    fn convert_swagger2_schema_to_json_schema(&self, schema: &Swagger2Schema) -> Result<Value> {
        let mut json_schema = serde_json::Map::new();

        // Handle reference
        if let Some(reference) = &schema.reference {
            return Ok(json!({
                "$ref": reference,
                "description": schema.description.clone().unwrap_or_else(|| "Referenced schema".to_string())
            }));
        }

        // Handle type
        if let Some(schema_type) = &schema.schema_type {
            json_schema.insert("type".to_string(), json!(schema_type));
        }

        // Handle format
        if let Some(format) = &schema.format {
            json_schema.insert("format".to_string(), json!(format));
        }

        // Handle description
        if let Some(description) = &schema.description {
            json_schema.insert("description".to_string(), json!(description));
        }

        // Handle default value
        if let Some(default) = &schema.default {
            json_schema.insert("default".to_string(), default.clone());
        }

        // Handle enum
        if let Some(enumeration) = &schema.enumeration {
            json_schema.insert("enum".to_string(), json!(enumeration));
        }

        // Handle properties (for object types)
        if let Some(properties) = &schema.properties {
            let mut props = serde_json::Map::new();
            for (prop_name, prop_schema) in properties {
                props.insert(prop_name.clone(), self.convert_swagger2_schema_to_json_schema(prop_schema)?);
            }
            json_schema.insert("properties".to_string(), Value::Object(props));
        }

        // Handle required fields
        if let Some(required) = &schema.required {
            json_schema.insert("required".to_string(), json!(required));
        }

        // Handle array items
        if let Some(items) = &schema.items {
            json_schema.insert("items".to_string(), self.convert_swagger2_schema_to_json_schema(items)?);
        }

        // Handle extensions (this fixes the SchemaData struct issue mentioned in TODO)
        for (key, value) in &schema.extensions {
            if key.starts_with("x-") {
                json_schema.insert(key.clone(), value.clone());
            }
        }

        Ok(Value::Object(json_schema))
    }

    /// Generate capability file from operations
    fn generate_capability_file(&self, operations: Vec<OpenAPIOperation>) -> Result<CapabilityFile> {
        if self.use_enhanced_format {
            self.generate_enhanced_capability_file(operations)
        } else {
            self.generate_legacy_capability_file(operations)
        }
    }

    /// Generate enhanced MCP 2025-06-18 format capability file
    fn generate_enhanced_capability_file(&self, operations: Vec<OpenAPIOperation>) -> Result<CapabilityFile> {
        let mut enhanced_tools = Vec::new();

        for operation in operations {
            match self.operation_to_enhanced_tool_definition(operation) {
                Ok(tool) => enhanced_tools.push(tool),
                Err(e) => {
                    // Log warning but continue processing other operations
                    tracing::warn!("Failed to convert operation to enhanced tool: {}", e);
                }
            }
        }

        let enhanced_metadata = EnhancedFileMetadata {
            name: "Enhanced OpenAPI REST API".to_string(),
            description: format!("Auto-generated REST API tools for {} - MCP 2025-06-18 compliant with AI enhancement", self.base_url),
            version: "3.0.0".to_string(),
            author: "OpenAPI Schema Generator".to_string(),
            classification: Some(ClassificationMetadata {
                security_level: "safe".to_string(),
                complexity_level: "simple".to_string(),
                domain: "api".to_string(),
                use_cases: vec!["api_integration".to_string(), "rest_client".to_string()],
            }),
            discovery_metadata: Some(DiscoveryMetadata {
                primary_keywords: vec!["openapi".to_string(), "rest".to_string(), "api".to_string(), "auto-generated".to_string()],
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
    fn generate_legacy_capability_file(&self, operations: Vec<OpenAPIOperation>) -> Result<CapabilityFile> {
        let mut tools = Vec::new();

        for operation in operations {
            match self.operation_to_tool_definition(operation) {
                Ok(tool) => tools.push(tool),
                Err(e) => {
                    // Log warning but continue processing other operations
                    tracing::warn!("Failed to convert operation to tool: {}", e);
                }
            }
        }

        let metadata = FileMetadata {
            name: Some("OpenAPI REST API".to_string()),
            description: Some(format!("Auto-generated REST API tools for {}", self.base_url)),
            version: Some("1.0.0".to_string()),
            author: Some("OpenAPI Schema Generator".to_string()),
            tags: Some(vec!["openapi".to_string(), "rest".to_string(), "auto-generated".to_string()]),
        };

        CapabilityFile::with_metadata(metadata, tools)
    }

    /// Convert OpenAPI operation to enhanced MCP 2025-06-18 tool definition
    fn operation_to_enhanced_tool_definition(&self, operation: OpenAPIOperation) -> Result<EnhancedToolDefinition> {
        let tool_name = format!("enhanced_{}", self.generate_tool_name(&operation)?);
        let description = format!("AI-enhanced {} with MCP 2025-06-18 compliance", self.generate_tool_description(&operation));
        let input_schema = self.generate_input_schema(&operation)?;

        let core = CoreDefinition {
            description,
            input_schema,
        };

        let execution = ExecutionConfig {
            routing: EnhancedRoutingConfig {
                r#type: "enhanced_http".to_string(),
                primary: Some(self.create_enhanced_routing_config(&operation)?),
                fallback: None,
                config: Some(serde_json::json!({
                    "enhanced": true,
                    "base_url": self.base_url
                })),
            },
            security: crate::registry::types::SecurityConfig {
                classification: "safe".to_string(),
                sandbox: Some(crate::registry::types::SandboxConfig {
                    resources: Some(crate::registry::types::ResourceLimits {
                        max_memory_mb: Some(256),
                        max_cpu_percent: Some(30), 
                        max_execution_seconds: Some(60),
                        max_file_descriptors: Some(100),
                    }),
                    filesystem: None,
                    network: Some(crate::registry::types::NetworkRestrictions {
                        allowed: true,
                        allowed_domains: Some(vec![self.base_url.clone()]),
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
                    "simple_operation": 5,
                    "complex_operation": 30
                })),
                complexity: Some("moderate".to_string()),
                supports_cancellation: Some(true),
                supports_progress: Some(false),
                cache_results: Some(operation.method.to_lowercase() == "get"),
                cache_ttl_seconds: if operation.method.to_lowercase() == "get" { Some(300) } else { Some(0) },
                adaptive_optimization: Some(true),
            },
        };

        let discovery = DiscoveryEnhancement {
            ai_enhanced: Some(AiEnhancedDiscovery {
                description: Some(format!("AI-enhanced {} with intelligent processing and security validation", 
                    self.generate_tool_description(&operation))),
                usage_patterns: Some(vec![
                    format!("use {} to {{action}}", tool_name),
                    format!("help me {{accomplish_task}} with {}", tool_name),
                    format!("{} for {{specific_purpose}}", tool_name),
                ]),
                semantic_context: Some(SemanticContext {
                    primary_intent: Some(match operation.method.to_lowercase().as_str() {
                        "get" => "data_retrieval",
                        "post" => "data_creation",
                        "put" | "patch" => "data_modification",
                        "delete" => "data_deletion",
                        _ => "general_operation"
                    }.to_string()),
                    operations: Some(vec![operation.method.to_lowercase()]),
                    data_types: Some(vec!["structured".to_string(), "unstructured".to_string()]),
                    security_features: Some(vec!["authentication".to_string(), "authorization".to_string()]),
                }),
                ai_capabilities: None,
                workflow_integration: Some(WorkflowIntegration {
                    typically_follows: Some(vec![]),
                    typically_precedes: Some(vec![]),
                    chain_compatibility: Some(vec!["api_workflow".to_string()]),
                }),
            }),
            parameter_intelligence: Some(self.generate_parameter_intelligence(&operation)?),
        };

        let monitoring = MonitoringConfig {
            progress_tracking: Some(ProgressTrackingConfig {
                enabled: false,
                granularity: Some("basic".to_string()),
                sub_operations: None,
            }),
            cancellation: Some(CancellationConfig {
                enabled: true,
                graceful_timeout_seconds: Some(10),
                cleanup_required: Some(false),
                cleanup_operations: None,
            }),
            metrics: Some(MetricsConfig {
                track_execution_time: Some(true),
                track_success_rate: Some(true),
                custom_metrics: Some(vec![format!("{}_operations_completed", tool_name)]),
            }),
        };

        let access = AccessConfig {
            hidden: true, // OpenAPI tools are hidden by default
            enabled: true, // OpenAPI tools are enabled by default
            requires_permissions: Some(vec!["tool:execute".to_string(), "security:validated".to_string()]),
            user_groups: Some(vec!["administrators".to_string()]),
            approval_required: Some(false),
            usage_analytics: Some(true),
        };

        EnhancedToolDefinition::new(tool_name, core, execution, discovery, monitoring, access)
    }

    /// Convert OpenAPI operation to legacy MCP tool definition
    fn operation_to_tool_definition(&self, operation: OpenAPIOperation) -> Result<ToolDefinition> {
        let tool_name = self.generate_tool_name(&operation)?;
        let description = self.generate_tool_description(&operation);
        let input_schema = self.generate_input_schema(&operation)?;
        let routing = self.create_routing_config(&operation)?;

        Ok(ToolDefinition {
            name: tool_name,
            description,
            input_schema,
            routing,
            annotations: None, // TODO: Add annotations support
            hidden: true, // OpenAPI tools are hidden by default (consistent with other tools)
            enabled: true, // OpenAPI tools are enabled by default
            prompt_refs: Vec::new(),
            resource_refs: Vec::new(),
        })
    }

    /// Generate tool name based on naming convention
    fn generate_tool_name(&self, operation: &OpenAPIOperation) -> Result<String> {
        let base_name = match &self.naming_convention {
            NamingConvention::OperationId => {
                operation.operation_id.clone()
                    .unwrap_or_else(|| format!("{}_{}", operation.method.to_lowercase(),
                        operation.path.replace('/', "_").replace('{', "").replace('}', "")))
            }
            NamingConvention::MethodPath => {
                format!("{}_{}", operation.method.to_lowercase(),
                    operation.path.replace('/', "_").replace('{', "").replace('}', ""))
            }
            NamingConvention::Custom(format) => {
                // Simple template substitution
                format.replace("{method}", &operation.method.to_lowercase())
                      .replace("{path}", &operation.path.replace('/', "_"))
                      .replace("{operationId}", &operation.operation_id.clone().unwrap_or_default())
            }
        };

        let tool_name = if let Some(prefix) = &self.tool_prefix {
            format!("{}_{}", prefix, base_name)
        } else {
            base_name
        };

        Ok(tool_name)
    }

    /// Generate tool description
    fn generate_tool_description(&self, operation: &OpenAPIOperation) -> String {
        let mut description = operation.description.clone()
            .or_else(|| operation.summary.clone())
            .unwrap_or_else(|| format!("{} {}", operation.method, operation.path));

        // Ensure description is not empty
        if description.trim().is_empty() {
            description = format!("{} {}", operation.method, operation.path);
        }

        if operation.deprecated {
            description = format!("⚠️ DEPRECATED: {}", description);
        }

        description
    }

    /// Generate JSON schema for input parameters
    fn generate_input_schema(&self, operation: &OpenAPIOperation) -> Result<Value> {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        // Add path parameters
        for param in &operation.parameters {
            if param.location == "path" {
                properties.insert(param.name.clone(), param.schema.clone());
                if param.required {
                    required.push(param.name.clone());
                }
            }
        }

        // Add query parameters
        for param in &operation.parameters {
            if param.location == "query" {
                properties.insert(param.name.clone(), param.schema.clone());
                if param.required {
                    required.push(param.name.clone());
                }
            }
        }

        // Add request body parameters
        if let Some(request_body) = &operation.request_body {
            if request_body.content.contains_key("application/json") {
                properties.insert("body".to_string(), json!({
                    "type": "object",
                    "description": request_body.description.clone().unwrap_or_else(|| "Request body".to_string())
                }));
                if request_body.required {
                    required.push("body".to_string());
                }
            }
        }

        Ok(json!({
            "type": "object",
            "properties": properties,
            "required": required
        }))
    }

    /// Create routing configuration for HTTP request
    fn create_routing_config(&self, operation: &OpenAPIOperation) -> Result<RoutingConfig> {
        let mut config = serde_json::Map::new();

        config.insert("method".to_string(), json!(operation.method));
        config.insert("url".to_string(), json!(format!("{}{}", self.base_url, operation.path)));

        // Add authentication headers
        if let Some(auth_config) = &self.auth_config {
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
                    let credentials = general_purpose::STANDARD.encode(format!("{}:{}", username, password));
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

        // Add parameter mapping
        let mut path_params = Vec::new();
        let mut query_params = Vec::new();

        for param in &operation.parameters {
            match param.location.as_str() {
                "path" => path_params.push(&param.name),
                "query" => query_params.push(&param.name),
                _ => {}
            }
        }

        if !path_params.is_empty() {
            config.insert("path_params".to_string(), json!(path_params));
        }
        if !query_params.is_empty() {
            config.insert("query_params".to_string(), json!(query_params));
        }

        // Add request body handling
        if operation.request_body.is_some() {
            config.insert("body_param".to_string(), json!("body"));
        }

        Ok(RoutingConfig::new("http".to_string(), Value::Object(config)))
    }

    /// Create enhanced routing configuration for OpenAPI operation
    fn create_enhanced_routing_config(&self, operation: &OpenAPIOperation) -> Result<Value> {
        let mut config = serde_json::Map::new();

        // Basic HTTP configuration
        config.insert("method".to_string(), json!(operation.method.to_uppercase()));
        config.insert("url".to_string(), json!(format!("{}{}", self.base_url, operation.path)));
        config.insert("timeout_seconds".to_string(), json!(30));

        // Enhanced features
        config.insert("retry_count".to_string(), json!(3));
        config.insert("cache_responses".to_string(), json!(operation.method.to_lowercase() == "get"));

        // Authentication
        if let Some(ref auth) = self.auth_config {
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
                    let credentials = general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                    config.insert("headers".to_string(), json!({
                        "Authorization": format!("Basic {}", credentials)
                    }));
                }
                _ => {}
            }
        }

        // Request body handling
        if operation.request_body.is_some() {
            config.insert("body_param".to_string(), json!("body"));
        }

        Ok(json!(config))
    }

    /// Generate parameter intelligence for OpenAPI operation
    fn generate_parameter_intelligence(&self, operation: &OpenAPIOperation) -> Result<Value> {
        let mut param_map = HashMap::new();

        for param in &operation.parameters {
            let mut param_config = HashMap::new();

            // Set smart default
            if let Some(ref default_value) = param.default {
                param_config.insert("smart_default".to_string(), default_value.clone());
            } else {
                param_config.insert("smart_default".to_string(), Value::Null);
            }

            // Add validation rules
            let mut validations = Vec::new();
            if param.required {
                validations.push(json!({
                    "rule": "required_validation",
                    "message": format!("{} must be provided and valid", param.name)
                }));
            }
            param_config.insert("validation".to_string(), json!(validations));

            // Add smart suggestions based on parameter type
            if param.location == "path" || param.location == "query" {
                let suggestions = vec![json!({
                    "pattern": "*",
                    "description": format!("{} parameter values", param.name),
                    "examples": self.generate_param_examples(&param.schema)
                })];
                param_config.insert("smart_suggestions".to_string(), json!(suggestions));
            }

            param_map.insert(param.name.clone(), Value::Object(
                param_config.into_iter().collect()
            ));
        }

        // Handle request body parameters
        if let Some(ref request_body) = operation.request_body {
            param_map.insert("body".to_string(), json!({
                "smart_default": null,
                "validation": if request_body.required {
                    vec![json!({
                        "rule": "required_validation",
                        "message": "Request body must be provided and valid"
                    })]
                } else {
                    vec![]
                },
                "content_types": request_body.content.keys().collect::<Vec<_>>()
            }));
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
}

// Include Swagger 2.0 tests
#[cfg(test)]
mod swagger2_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_openapi_generator_creation() {
        let generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string());
        assert_eq!(generator.base_url, "https://api.example.com");
        assert!(generator.auth_config.is_none());
        assert!(generator.tool_prefix.is_none());
    }

    #[test]
    fn test_openapi_generator_with_auth() {
        let auth_config = AuthConfig {
            auth_type: AuthType::Bearer { token: "test-token".to_string() },
            headers: HashMap::new(),
        };

        let generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string())
            .with_auth(auth_config);

        assert!(generator.auth_config.is_some());
        if let Some(auth) = &generator.auth_config {
            match &auth.auth_type {
                AuthType::Bearer { token } => assert_eq!(token, "test-token"),
                _ => panic!("Expected Bearer token"),
            }
        }
    }

    #[test]
    fn test_openapi_generator_with_prefix() {
        let generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string())
            .with_prefix("test".to_string());

        assert_eq!(generator.tool_prefix, Some("test".to_string()));
    }

    #[test]
    fn test_simple_openapi3_spec() {
        let mut generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string());

        let openapi_spec = r#"
        {
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "getUsers",
                        "summary": "Get all users",
                        "responses": {
                            "200": {
                                "description": "Success"
                            }
                        }
                    }
                }
            }
        }
        "#;

        let result = generator.generate_from_openapi3(openapi_spec);
        assert!(result.is_ok(), "Failed to generate from OpenAPI spec: {:?}", result.err());

        let capability_file = result.unwrap();
        assert_eq!(capability_file.tools.len(), 1);
        assert_eq!(capability_file.tools[0].name, "getUsers");
        assert_eq!(capability_file.tools[0].description, "Get all users");
    }

    #[test]
    fn test_openapi3_spec_with_parameters() {
        let mut generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string());

        let openapi_spec = r#"
        {
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users/{id}": {
                    "get": {
                        "operationId": "getUserById",
                        "summary": "Get user by ID",
                        "parameters": [
                            {
                                "name": "id",
                                "in": "path",
                                "required": true,
                                "schema": {
                                    "type": "string"
                                }
                            }
                        ],
                        "responses": {
                            "200": {
                                "description": "Success"
                            }
                        }
                    }
                }
            }
        }
        "#;

        let result = generator.generate_from_openapi3(openapi_spec);
        assert!(result.is_ok(), "Failed to generate from OpenAPI spec: {:?}", result.err());

        let capability_file = result.unwrap();
        assert_eq!(capability_file.tools.len(), 1);

        let tool = &capability_file.tools[0];
        assert_eq!(tool.name, "getUserById");

        // Check that the input schema includes the path parameter
        let properties = tool.input_schema["properties"].as_object().unwrap();
        assert!(properties.contains_key("id"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("id")));
    }

    #[test]
    fn test_naming_convention_method_path() {
        let mut generator = OpenAPICapabilityGenerator::new("https://api.example.com".to_string())
            .with_naming_convention(NamingConvention::MethodPath);

        let openapi_spec = r#"
        {
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "post": {
                        "summary": "Create user",
                        "responses": {
                            "201": {
                                "description": "Created"
                            }
                        }
                    }
                }
            }
        }
        "#;

        let result = generator.generate_from_openapi3(openapi_spec);
        assert!(result.is_ok());

        let capability_file = result.unwrap();
        assert_eq!(capability_file.tools.len(), 1);
        assert_eq!(capability_file.tools[0].name, "post__users");
    }
}
