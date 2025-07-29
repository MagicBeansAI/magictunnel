//! Configuration file support for capability generators
//!
//! This module provides functionality to load and parse configuration files for capability generators.
//!
//! # Overview
//!
//! The configuration system uses TOML files to define settings for different capability generators.
//! It supports global settings that apply to all generators, as well as generator-specific settings
//! that override the global ones.
//!
//! # Configuration Structure
//!
//! The configuration file has the following main sections:
//!
//! - `[global]`: Settings that apply to all generators
//! - `[graphql]`: Settings specific to GraphQL generators
//! - `[grpc]`: Settings specific to gRPC generators
//! - `[openapi]`: Settings specific to OpenAPI generators
//! - `[output]`: Settings for output file formatting and location
//!
//! # Example Usage
//!
//! ```toml
//! [global]
//! tool_prefix = "mcp"
//! output_dir = "./capabilities"
//!
//! [graphql]
//! endpoint = "https://api.example.com/graphql"
//! tool_prefix = "graphql"
//!
//! [output]
//! format = "yaml"
//! file_pattern = "{name}-capabilities.{ext}"
//! ```
//!
//! # Authentication
//!
//! The configuration supports various authentication methods:
//!
//! - Bearer token: `auth_type = { Bearer = "TOKEN" }`
//! - API key: `auth_type = { ApiKey = { key = "KEY", header = "X-API-Key" } }`
//! - Basic auth: `auth_type = { Basic = { username = "USER", password = "PASS" } }`
//! - OAuth: `auth_type = { OAuth = { token = "TOKEN", token_type = "Bearer" } }`

use crate::error::{ProxyError, Result};
use crate::registry::generator_common::{AuthConfig, AuthType, BaseGeneratorConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Generator configuration file
///
/// This is the main configuration structure that holds settings for all generator types.
/// It can be loaded from a YAML file using the `from_file` or `from_yaml` methods.
///
/// # Example
///
/// ```
/// use magictunnel::registry::GeneratorConfigFile;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = GeneratorConfigFile::from_file("config.yaml")?;
/// // Or from a string
/// let yaml_content = "global:\n  tool_prefix: test";
/// let config = GeneratorConfigFile::from_yaml(yaml_content)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfigFile {
    /// Global configuration
    #[serde(default)]
    pub global: GlobalConfig,
    /// GraphQL generator configuration
    pub graphql: Option<GraphQLGeneratorConfig>,
    /// gRPC generator configuration
    pub grpc: Option<GrpcGeneratorConfig>,
    /// OpenAPI generator configuration
    pub openapi: Option<OpenAPIGeneratorConfig>,
    /// Output configuration
    #[serde(default)]
    pub output: OutputConfig,
}

/// Global configuration for all generators
///
/// These settings apply to all generators unless overridden by generator-specific settings.
///
/// # Fields
///
/// - `tool_prefix`: Optional prefix for all tool names (e.g., "mcp" results in "mcp_toolName")
/// - `auth`: Optional authentication configuration for all generators
/// - `output_dir`: Optional default directory for output files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    /// Tool name prefix for all generators
    pub tool_prefix: Option<String>,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
    /// Default output directory
    pub output_dir: Option<String>,
}

/// Output configuration
///
/// Controls how capability files are formatted and where they are saved.
///
/// # Fields
///
/// - `format`: Output format, always "yaml" (capability files only support YAML)
/// - `pretty`: Whether to pretty-print the output (default: true)
/// - `directory`: Optional output directory (overrides global.output_dir)
/// - `file_pattern`: Pattern for output filenames, supporting {name} and {ext} placeholders
///   (default: "{name}-capabilities.{ext}")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output format (always yaml for capability files)
    #[serde(default = "default_output_format")]
    pub format: String,
    /// Whether to pretty-print output
    #[serde(default = "default_pretty_print")]
    pub pretty: bool,
    /// Output directory
    pub directory: Option<String>,
    /// File naming pattern
    #[serde(default = "default_file_pattern")]
    pub file_pattern: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: default_output_format(),
            pretty: default_pretty_print(),
            directory: None,
            file_pattern: default_file_pattern(),
        }
    }
}

fn default_output_format() -> String {
    "yaml".to_string()
}

fn default_pretty_print() -> bool {
    true
}

fn default_file_pattern() -> String {
    "{name}-capabilities.{ext}".to_string()
}

/// GraphQL generator configuration
///
/// Settings specific to GraphQL capability generation.
///
/// # Fields
///
/// - `endpoint`: GraphQL endpoint URL (required)
/// - `schema`: Optional GraphQL schema file path (SDL or JSON introspection)
/// - `tool_prefix`: Optional prefix for GraphQL tool names (overrides global prefix)
/// - `auth`: Optional authentication configuration (overrides global auth)
/// - `include_deprecated`: Whether to include deprecated fields and operations (default: false)
/// - `include_descriptions`: Whether to include descriptions in schemas (default: true)
/// - `separate_mutation_query`: Whether to generate separate tools for mutations and queries (default: true)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLGeneratorConfig {
    /// GraphQL endpoint URL
    pub endpoint: String,
    /// GraphQL schema file path (SDL or JSON introspection)
    pub schema: Option<String>,
    /// Tool name prefix specific to GraphQL
    pub tool_prefix: Option<String>,
    /// Authentication configuration specific to GraphQL
    pub auth: Option<AuthConfig>,
    /// Whether to include deprecated fields and operations
    #[serde(default)]
    pub include_deprecated: bool,
    /// Whether to include descriptions in schemas
    #[serde(default = "default_true")]
    pub include_descriptions: bool,
    /// Whether to generate separate tools for mutations and queries
    #[serde(default = "default_true")]
    pub separate_mutation_query: bool,
}

/// gRPC generator configuration
///
/// Settings specific to gRPC capability generation.
///
/// # Fields
///
/// - `endpoint`: gRPC service endpoint (required)
/// - `tool_prefix`: Optional prefix for gRPC tool names (overrides global prefix)
/// - `auth`: Optional authentication configuration (overrides global auth)
/// - `service_filter`: Optional list of service names to include
/// - `method_filter`: Optional list of method names to include
/// - `server_streaming_strategy`: Strategy for server streaming methods (polling, pagination, agent-level)
/// - `client_streaming_strategy`: Strategy for client streaming methods
/// - `bidirectional_streaming_strategy`: Strategy for bidirectional streaming methods
/// - `include_method_options`: Whether to include method options in tool definitions
/// - `separate_streaming_tools`: Whether to generate separate tools for streaming methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcGeneratorConfig {
    /// gRPC endpoint
    pub endpoint: String,
    /// Tool name prefix specific to gRPC
    pub tool_prefix: Option<String>,
    /// Authentication configuration specific to gRPC
    pub auth: Option<AuthConfig>,
    /// Service filter
    pub service_filter: Option<Vec<String>>,
    /// Method filter
    pub method_filter: Option<Vec<String>>,
    /// Server streaming strategy
    #[serde(default)]
    pub server_streaming_strategy: String,
    /// Client streaming strategy
    #[serde(default)]
    pub client_streaming_strategy: String,
    /// Bidirectional streaming strategy
    #[serde(default)]
    pub bidirectional_streaming_strategy: String,
    /// Whether to include method options in tool definitions
    #[serde(default)]
    pub include_method_options: bool,
    /// Whether to generate separate tools for streaming methods
    #[serde(default)]
    pub separate_streaming_tools: bool,
}

/// OpenAPI generator configuration
///
/// Settings specific to OpenAPI capability generation.
///
/// # Fields
///
/// - `base_url`: Base URL for the API (required)
/// - `tool_prefix`: Optional prefix for OpenAPI tool names (overrides global prefix)
/// - `auth`: Optional authentication configuration (overrides global auth)
/// - `naming_convention`: Naming convention for tools (operation-id or method-path)
/// - `methods`: Optional list of HTTP methods to include (e.g., GET, POST)
/// - `include_deprecated`: Whether to include deprecated operations (default: false)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPIGeneratorConfig {
    /// Base URL for the API
    pub base_url: String,
    /// Tool name prefix specific to OpenAPI
    pub tool_prefix: Option<String>,
    /// Authentication configuration specific to OpenAPI
    pub auth: Option<AuthConfig>,
    /// Naming convention (operation-id or method-path)
    #[serde(default = "default_naming_convention")]
    pub naming_convention: String,
    /// HTTP methods to include
    pub methods: Option<Vec<String>>,
    /// Whether to include deprecated operations
    #[serde(default)]
    pub include_deprecated: bool,
}

fn default_naming_convention() -> String {
    "operation-id".to_string()
}

fn default_true() -> bool {
    true
}

impl GeneratorConfigFile {
    /// Load configuration from a file
    ///
    /// Reads a YAML configuration file from the specified path and parses it into a
    /// `GeneratorConfigFile` structure.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the YAML configuration file
    ///
    /// # Returns
    ///
    /// A Result containing the parsed configuration or an error
    ///
    /// # Example
    ///
    /// ```
    /// use magictunnel::registry::GeneratorConfigFile;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = GeneratorConfigFile::from_file("config.yaml")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ProxyError::config(format!(
                "Failed to read config file '{}': {}",
                path.as_ref().display(),
                e
            )))?;

        Self::from_yaml(&content)
    }
    
    /// Parse configuration from YAML content
    ///
    /// Parses a YAML string into a `GeneratorConfigFile` structure.
    ///
    /// # Arguments
    ///
    /// * `content` - YAML content as a string
    ///
    /// # Returns
    ///
    /// A Result containing the parsed configuration or an error
    ///
    /// # Example
    ///
    /// ```
    /// use magictunnel::registry::GeneratorConfigFile;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let yaml_content = "global:\n  tool_prefix: test";
    /// let config = GeneratorConfigFile::from_yaml(yaml_content)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_yaml(content: &str) -> Result<Self> {
        serde_yaml::from_str(content)
            .map_err(|e| ProxyError::config(format!("Failed to parse YAML config: {}", e)))
    }


    
    /// Get base configuration for a specific generator
    ///
    /// Creates a base configuration for the specified generator type by combining
    /// global settings with generator-specific overrides.
    ///
    /// # Arguments
    ///
    /// * `generator_type` - The type of generator ("graphql", "grpc", or "openapi")
    ///
    /// # Returns
    ///
    /// A `BaseGeneratorConfig` with the combined settings
    ///
    /// # Example
    ///
    /// ```
    /// use magictunnel::registry::GeneratorConfigFile;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let yaml_content = "global:\n  tool_prefix: test";
    /// let config = GeneratorConfigFile::from_yaml(yaml_content)?;
    /// let base_config = config.get_base_config("graphql");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_base_config(&self, generator_type: &str) -> BaseGeneratorConfig {
        let mut config = BaseGeneratorConfig::default();
        
        // Apply global configuration
        config.tool_prefix = self.global.tool_prefix.clone();
        config.auth_config = self.global.auth.clone();
        
        // Apply generator-specific configuration
        match generator_type {
            "graphql" => {
                if let Some(ref graphql_config) = self.graphql {
                    if graphql_config.tool_prefix.is_some() {
                        config.tool_prefix = graphql_config.tool_prefix.clone();
                    }
                    if graphql_config.auth.is_some() {
                        config.auth_config = graphql_config.auth.clone();
                    }
                }
            },
            "grpc" => {
                if let Some(ref grpc_config) = self.grpc {
                    if grpc_config.tool_prefix.is_some() {
                        config.tool_prefix = grpc_config.tool_prefix.clone();
                    }
                    if grpc_config.auth.is_some() {
                        config.auth_config = grpc_config.auth.clone();
                    }
                }
            },
            "openapi" => {
                if let Some(ref openapi_config) = self.openapi {
                    if openapi_config.tool_prefix.is_some() {
                        config.tool_prefix = openapi_config.tool_prefix.clone();
                    }
                    if openapi_config.auth.is_some() {
                        config.auth_config = openapi_config.auth.clone();
                    }
                }
            },
            _ => {}
        }
        
        // Apply output configuration
        let output_dir = self.output.directory.as_ref()
            .or(self.global.output_dir.as_ref())
            .map(|dir| dir.clone());
        
        if let Some(dir) = output_dir {
            let file_name = self.output.file_pattern
                .replace("{name}", generator_type)
                .replace("{ext}", "yaml"); // Capability files are always YAML

            let path = Path::new(&dir).join(file_name);
            config.output_path = Some(path.to_string_lossy().to_string());
        }
        
        config
    }
    
    /// Validate the configuration
    ///
    /// Checks that the configuration is valid, ensuring that required fields are present
    /// and that values are within acceptable ranges.
    ///
    /// # Returns
    ///
    /// A Result indicating success or an error with details
    ///
    /// # Example
    ///
    /// ```
    /// use magictunnel::registry::GeneratorConfigFile;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let yaml_content = "global:\n  tool_prefix: test";
    /// let config = GeneratorConfigFile::from_yaml(yaml_content)?;
    /// config.validate()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate(&self) -> Result<()> {
        // Validate GraphQL configuration
        if let Some(ref graphql) = self.graphql {
            if graphql.endpoint.is_empty() {
                return Err(ProxyError::config("GraphQL endpoint cannot be empty"));
            }
        }
        
        // Validate gRPC configuration
        if let Some(ref grpc) = self.grpc {
            if grpc.endpoint.is_empty() {
                return Err(ProxyError::config("gRPC endpoint cannot be empty"));
            }
            
            // Validate streaming strategies
            let valid_strategies = ["polling", "pagination", "agent-level"];
            
            if !grpc.server_streaming_strategy.is_empty() && 
               !valid_strategies.contains(&grpc.server_streaming_strategy.as_str()) {
                return Err(ProxyError::config(format!(
                    "Invalid server streaming strategy: {}. Use 'polling', 'pagination', or 'agent-level'",
                    grpc.server_streaming_strategy
                )));
            }
            
            if !grpc.client_streaming_strategy.is_empty() && 
               !valid_strategies.contains(&grpc.client_streaming_strategy.as_str()) {
                return Err(ProxyError::config(format!(
                    "Invalid client streaming strategy: {}. Use 'polling', 'pagination', or 'agent-level'",
                    grpc.client_streaming_strategy
                )));
            }
            
            if !grpc.bidirectional_streaming_strategy.is_empty() && 
               !valid_strategies.contains(&grpc.bidirectional_streaming_strategy.as_str()) {
                return Err(ProxyError::config(format!(
                    "Invalid bidirectional streaming strategy: {}. Use 'polling', 'pagination', or 'agent-level'",
                    grpc.bidirectional_streaming_strategy
                )));
            }
        }
        
        // Validate OpenAPI configuration
        if let Some(ref openapi) = self.openapi {
            if openapi.base_url.is_empty() {
                return Err(ProxyError::config("OpenAPI base URL cannot be empty"));
            }
            
            // Validate naming convention
            if !["operation-id", "method-path"].contains(&openapi.naming_convention.as_str()) {
                return Err(ProxyError::config(format!(
                    "Invalid naming convention: {}. Use 'operation-id' or 'method-path'",
                    openapi.naming_convention
                )));
            }
        }
        
        // Validate output configuration
        if !["yaml", "json"].contains(&self.output.format.as_str()) {
            return Err(ProxyError::config(format!(
                "Invalid output format: {}. Use 'yaml' or 'json'",
                self.output.format
            )));
        }
        
        Ok(())
    }
}

/// Example configuration file in YAML format (recommended)
///
/// Returns a string containing an example YAML configuration file with
/// settings for all generator types. This is used by the `init` command
/// to create a template configuration file.
///
/// # Returns
///
/// A string containing the example configuration in YAML format
///
/// # Example
///
/// ```
/// use magictunnel::registry::generator_config::example_config_yaml;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let example = example_config_yaml();
/// std::fs::write("config.yaml", example)?;
/// # Ok(())
/// # }
/// ```
pub fn example_config_yaml() -> String {
    r#"# MCP Generator Configuration (YAML Format)
# This file demonstrates all available configuration options for the unified capability generator.

# Global settings applied to all generators
global:
  # Global tool prefix for all generators
  tool_prefix: "mcp"
  # Default output directory
  output_dir: "./capabilities"

  # Global authentication configuration (optional)
  # Uncomment and configure as needed:
  # auth:
  #   auth_type:
  #     Bearer: "YOUR_TOKEN"
  #   headers:
  #     X-Custom-Header: "value"

# Output format settings
output:
  # Output format: always yaml (capability files only support YAML)
  format: "yaml"
  # Pretty-print output
  pretty: true
  # Output directory (overrides global.output_dir)
  directory: "./capabilities"
  # File naming pattern
  file_pattern: "{name}-capabilities.{ext}"

# GraphQL generator configuration
graphql:
  # GraphQL endpoint URL (required)
  endpoint: "https://api.example.com/graphql"
  # Override global tool prefix
  tool_prefix: "graphql"
  # Include deprecated fields and operations
  include_deprecated: false
  # Include descriptions in schemas
  include_descriptions: true
  # Generate separate tools for mutations and queries
  separate_mutation_query: true

  # GraphQL-specific authentication (optional)
  # Uncomment and configure as needed:
  # auth:
  #   auth_type:
  #     ApiKey:
  #       key: "YOUR_API_KEY"
  #       header: "X-API-Key"
  #   headers: {}

# gRPC generator configuration
grpc:
  # gRPC service endpoint (required)
  endpoint: "localhost:50051"
  # Override global tool prefix
  tool_prefix: "grpc"
  # Filter services to include
  service_filter:
    - "UserService"
    - "ProductService"
  # Filter methods to include
  method_filter:
    - "GetUser"
    - "ListProducts"
  # Streaming strategies
  server_streaming_strategy: "polling"
  client_streaming_strategy: "agent-level"
  bidirectional_streaming_strategy: "pagination"
  # Include method options
  include_method_options: true
  # Generate separate tools for streaming methods
  separate_streaming_tools: false

# OpenAPI generator configuration
openapi:
  # Base URL for the API (required)
  base_url: "https://api.example.com"
  # Override global tool prefix
  tool_prefix: "api"
  # Naming convention for tools
  naming_convention: "operation-id"
  # HTTP methods to include
  methods:
    - "GET"
    - "POST"
    - "PUT"
    - "DELETE"
  # Include deprecated operations
  include_deprecated: false
"#.to_string()
}

/// Example configuration file (YAML format)
///
/// Returns a string containing an example YAML configuration file with
/// settings for all generator types.
///
/// # Returns
///
/// A string containing the example configuration in YAML format
///
/// # Example
///
/// ```
/// use magictunnel::registry::generator_config::example_config;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let example = example_config();
/// std::fs::write("config.yaml", example)?;
/// # Ok(())
/// # }
/// ```
pub fn example_config() -> String {
    example_config_yaml()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_example_config() {
        let config = GeneratorConfigFile::from_yaml(&example_config()).unwrap();
        assert_eq!(config.global.tool_prefix, Some("mcp".to_string()));
        assert_eq!(config.output.format, "yaml");

        // Auth should be None since it's commented out in the example
        assert!(config.global.auth.is_none());

        if let Some(ref graphql) = config.graphql {
            assert_eq!(graphql.endpoint, "https://api.example.com/graphql");
            assert_eq!(graphql.tool_prefix, Some("graphql".to_string()));
            // Auth should be None since it's commented out
            assert!(graphql.auth.is_none());
        } else {
            panic!("GraphQL config not found");
        }

        if let Some(ref grpc) = config.grpc {
            assert_eq!(grpc.endpoint, "localhost:50051");
            assert_eq!(grpc.tool_prefix, Some("grpc".to_string()));
            assert_eq!(grpc.server_streaming_strategy, "polling");
        } else {
            panic!("gRPC config not found");
        }

        if let Some(ref openapi) = config.openapi {
            assert_eq!(openapi.base_url, "https://api.example.com");
            assert_eq!(openapi.tool_prefix, Some("api".to_string()));
            assert_eq!(openapi.naming_convention, "operation-id");
        } else {
            panic!("OpenAPI config not found");
        }
    }
    
    #[test]
    fn test_get_base_config() {
        let config = GeneratorConfigFile::from_yaml(&example_config()).unwrap();

        let graphql_config = config.get_base_config("graphql");
        assert_eq!(graphql_config.tool_prefix, Some("graphql".to_string()));

        let grpc_config = config.get_base_config("grpc");
        assert_eq!(grpc_config.tool_prefix, Some("grpc".to_string()));

        let openapi_config = config.get_base_config("openapi");
        assert_eq!(openapi_config.tool_prefix, Some("api".to_string()));
    }

    #[test]
    fn test_yaml_config_parsing() {
        let yaml_config = r#"
global:
  tool_prefix: "test"
  output_dir: "./test-output"

openapi:
  base_url: "https://api.example.com"
  tool_prefix: "api"
  naming_convention: "operation-id"
  methods:
    - "GET"
    - "POST"
  include_deprecated: false

output:
  format: "yaml"
  pretty: true
"#;

        let config = GeneratorConfigFile::from_yaml(yaml_config).unwrap();

        // Validate global settings
        assert_eq!(config.global.tool_prefix, Some("test".to_string()));
        assert_eq!(config.global.output_dir, Some("./test-output".to_string()));

        // Validate OpenAPI settings
        assert!(config.openapi.is_some());
        let openapi_config = config.openapi.unwrap();
        assert_eq!(openapi_config.base_url, "https://api.example.com");
        assert_eq!(openapi_config.tool_prefix, Some("api".to_string()));
        assert_eq!(openapi_config.naming_convention, "operation-id".to_string());
        assert_eq!(openapi_config.methods, Some(vec!["GET".to_string(), "POST".to_string()]));
        assert_eq!(openapi_config.include_deprecated, false);

        // Validate output settings
        let output_config = &config.output;
        assert_eq!(output_config.format, "yaml".to_string());
        assert_eq!(output_config.pretty, true);
    }


}