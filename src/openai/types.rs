//! OpenAI API Types and Data Structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenAPI 3.0 Specification Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub servers: Vec<OpenApiServer>,
    pub paths: HashMap<String, OpenApiPathItem>,
    pub components: Option<OpenApiComponents>,
}

/// OpenAPI Info Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiInfo {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub version: String,
}

/// OpenAPI Server Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiServer {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// OpenAPI Path Item Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiPathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<OpenApiOperation>,
}

/// OpenAPI Operation Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiOperation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "operationId", skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(rename = "requestBody", skip_serializing_if = "Option::is_none")]
    pub request_body: Option<OpenApiRequestBody>,
    pub responses: HashMap<String, OpenApiResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// OpenAPI Request Body Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiRequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    pub content: HashMap<String, OpenApiMediaType>,
}

/// OpenAPI Media Type Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiMediaType {
    pub schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<HashMap<String, serde_json::Value>>,
}

/// OpenAPI Response Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiResponse {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, OpenApiMediaType>>,
}

/// OpenAPI Components Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiComponents {
    #[serde(rename = "securitySchemes", skip_serializing_if = "Option::is_none")]
    pub security_schemes: Option<HashMap<String, OpenApiSecurityScheme>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<HashMap<String, serde_json::Value>>,
}

/// OpenAPI Security Scheme Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSecurityScheme {
    #[serde(rename = "type")]
    pub scheme_type: String,
    #[serde(rename = "in", skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl OpenApiSpec {
    /// Create a new OpenAPI 3.1.0 specification
    pub fn new(title: &str, version: &str) -> Self {
        Self {
            openapi: "3.1.0".to_string(),
            info: OpenApiInfo {
                title: title.to_string(),
                description: Some("MagicTunnel - MCP Tools via REST API".to_string()),
                version: version.to_string(),
            },
            servers: vec![
                OpenApiServer {
                    url: "http://localhost:3001".to_string(),
                    description: Some("Local development server".to_string()),
                },
            ],
            paths: HashMap::new(),
            components: Some(OpenApiComponents {
                security_schemes: Some({
                    let mut schemes = HashMap::new();
                    schemes.insert("ApiKeyAuth".to_string(), OpenApiSecurityScheme {
                        scheme_type: "apiKey".to_string(),
                        location: Some("header".to_string()),
                        name: Some("Authorization".to_string()),
                        description: Some("API key authentication".to_string()),
                    });
                    schemes
                }),
                schemas: Some(HashMap::new()), // Empty schemas object for OpenAPI 3.1.0 compliance
            }),
        }
    }

    /// Create a new OpenAPI 3.1.0 specification for smart tool discovery only
    pub fn new_smart_discovery(title: &str, version: &str) -> Self {
        let mut spec = Self::new(title, version);
        
        // Add smart tool discovery endpoint
        spec.add_smart_discovery_endpoint();
        
        spec
    }
    
    /// Add a tool endpoint to the OpenAPI spec
    pub fn add_tool_endpoint(&mut self, tool_name: &str, tool_description: &str, input_schema: &serde_json::Value) {
        let path = format!("/dashboard/api/tools/{}/execute", tool_name);
        
        let operation = OpenApiOperation {
            summary: Some(tool_description.to_string()),
            description: Some(tool_description.to_string()),
            operation_id: Some(format!("execute_{}", tool_name)),
            request_body: Some(OpenApiRequestBody {
                description: Some("Tool execution parameters".to_string()),
                required: Some(true),
                content: {
                    let mut content = HashMap::new();
                    content.insert("application/json".to_string(), OpenApiMediaType {
                        schema: input_schema.clone(),
                        examples: None,
                    });
                    content
                },
            }),
            responses: {
                let mut responses = HashMap::new();
                
                // Success response
                responses.insert("200".to_string(), OpenApiResponse {
                    description: "Tool execution result".to_string(),
                    content: Some({
                        let mut content = HashMap::new();
                        content.insert("application/json".to_string(), OpenApiMediaType {
                            schema: serde_json::json!({
                                "type": "object",
                                "properties": {
                                    "tool": {"type": "string", "description": "Name of the executed tool"},
                                    "result": {
                                        "type": "object",
                                        "properties": {
                                            "status": {"type": "string", "description": "Execution status"},
                                            "output": {"type": "string", "description": "Tool output"},
                                            "execution_time": {"type": "string", "description": "Time taken to execute"}
                                        }
                                    }
                                }
                            }),
                            examples: None,
                        });
                        content
                    }),
                });
                
                // Error response
                responses.insert("400".to_string(), OpenApiResponse {
                    description: "Tool execution failed".to_string(),
                    content: Some({
                        let mut content = HashMap::new();
                        content.insert("application/json".to_string(), OpenApiMediaType {
                            schema: serde_json::json!({
                                "type": "object",
                                "properties": {
                                    "error": {"type": "string", "description": "Error message"},
                                    "tool": {"type": "string", "description": "Tool that failed"}
                                }
                            }),
                            examples: None,
                        });
                        content
                    }),
                });
                
                responses
            },
            tags: Some(vec!["Tools".to_string()]),
        };
        
        let path_item = OpenApiPathItem {
            post: Some(operation),
            get: None, // Will be skipped in serialization due to skip_serializing_if
        };
        
        self.paths.insert(path, path_item);
    }

    /// Add smart tool discovery endpoint to the OpenAPI spec
    pub fn add_smart_discovery_endpoint(&mut self) {
        let path = "/dashboard/api/tools/smart_tool_discovery/execute".to_string();
        
        let operation = OpenApiOperation {
            summary: Some("Smart Tool Discovery and Execution".to_string()),
            description: Some("Intelligently discovers and executes the most appropriate tool for any natural language request. This single endpoint provides access to 100+ tools including file operations, system commands, network requests, data analysis, and more.".to_string()),
            operation_id: Some("smartToolDiscovery".to_string()),
            request_body: Some(OpenApiRequestBody {
                description: Some("Tool discovery parameters".to_string()),
                required: Some(true),
                content: {
                    let mut content = HashMap::new();
                    content.insert("application/json".to_string(), OpenApiMediaType {
                        schema: serde_json::json!({
                            "type": "object",
                            "required": ["request"],
                            "properties": {
                                "request": {
                                    "type": "string",
                                    "description": "Natural language description of what you want to accomplish. Examples: 'check system status', 'list files in current directory', 'ping google.com', 'analyze log files', 'make HTTP request to API'"
                                },
                                "confidence_threshold": {
                                    "type": "number",
                                    "minimum": 0.1,
                                    "maximum": 1.0,
                                    "default": 0.7,
                                    "description": "Minimum confidence score (0.1-1.0) required for tool selection. Lower values are more permissive, higher values are more strict."
                                },
                                "max_results": {
                                    "type": "integer",
                                    "minimum": 1,
                                    "maximum": 10,
                                    "default": 1,
                                    "description": "Maximum number of tool matches to return. Use 1 for execution, higher numbers for exploration."
                                },
                                "mode": {
                                    "type": "string",
                                    "enum": ["execute", "discover"],
                                    "default": "execute",
                                    "description": "Operation mode: 'execute' runs the best tool, 'discover' only finds matching tools without execution"
                                }
                            }
                        }),
                        examples: None,
                    });
                    content
                },
            }),
            responses: {
                let mut responses = HashMap::new();
                
                // Success response
                responses.insert("200".to_string(), OpenApiResponse {
                    description: "Tool discovery and execution results".to_string(),
                    content: Some({
                        let mut content = HashMap::new();
                        content.insert("application/json".to_string(), OpenApiMediaType {
                            schema: serde_json::json!({
                                "type": "object",
                                "properties": {
                                    "success": {
                                        "type": "boolean",
                                        "description": "Whether the operation succeeded"
                                    },
                                    "discovered_tool": {
                                        "type": "object",
                                        "properties": {
                                            "name": {
                                                "type": "string",
                                                "description": "Name of the selected tool"
                                            },
                                            "confidence": {
                                                "type": "number",
                                                "description": "Confidence score for the tool selection"
                                            },
                                            "description": {
                                                "type": "string",
                                                "description": "Description of what the tool does"
                                            }
                                        }
                                    },
                                    "execution_result": {
                                        "type": "object",
                                        "description": "Results from executing the discovered tool (when mode=execute)"
                                    },
                                    "discovered_tools": {
                                        "type": "array",
                                        "description": "List of all matching tools (when mode=discover or max_results > 1)",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "name": {"type": "string"},
                                                "confidence": {"type": "number"},
                                                "description": {"type": "string"}
                                            }
                                        }
                                    },
                                    "next_step": {
                                        "type": "string",
                                        "description": "Suggested next action or follow-up"
                                    },
                                    "error": {
                                        "type": "string",
                                        "description": "Error message if operation failed"
                                    }
                                }
                            }),
                            examples: None,
                        });
                        content
                    }),
                });
                
                // Error responses
                responses.insert("400".to_string(), OpenApiResponse {
                    description: "Bad request - invalid parameters".to_string(),
                    content: None,
                });
                
                responses.insert("500".to_string(), OpenApiResponse {
                    description: "Internal server error".to_string(),
                    content: None,
                });
                
                responses
            },
            tags: Some(vec!["Smart Discovery".to_string()]),
        };
        
        let path_item = OpenApiPathItem {
            post: Some(operation),
            get: None, // Will be skipped in serialization due to skip_serializing_if
        };
        
        self.paths.insert(path, path_item);
    }
}