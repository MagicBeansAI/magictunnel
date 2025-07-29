//! Tests for the unified capability generation CLI configuration parsing
//!
//! These tests validate the configuration file parsing functionality of the unified CLI,
//! including validation, environment variable overrides, and cross-generator settings.

use magictunnel::registry::generator_config::{
    GeneratorConfigFile, GraphQLGeneratorConfig, GrpcGeneratorConfig, OpenAPIGeneratorConfig,
    GlobalConfig, OutputConfig
};
use magictunnel::registry::generator_common::{AuthConfig, AuthType};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;





#[test]
fn test_config_file_parsing() {
    // Create a test YAML configuration
    let config_content = r#"
global:
  tool_prefix: "test"
  output_dir: "./test-output"
  auth:
    auth_type:
      type: "bearer"
      token: "global-token"
    headers:
      X-Global-Header: "global-value"

graphql:
  endpoint: "https://graphql.example.com"
  tool_prefix: "gql"
  include_deprecated: true
  auth:
    auth_type:
      type: "api_key"
      key: "graphql-key"
      header: "X-API-Key"
    headers: {}

grpc:
  endpoint: "grpc.example.com:50051"
  tool_prefix: "rpc"
  server_streaming_strategy: "polling"
  client_streaming_strategy: "pagination"
  bidirectional_streaming_strategy: "agent-level"

openapi:
  base_url: "https://api.example.com"
  tool_prefix: "api"
  naming_convention: "method-path"
  methods:
    - "GET"
    - "POST"
  include_deprecated: false

output:
  format: "yaml"
  pretty: true
  directory: "./output"
  file_pattern: "{name}_capabilities.{ext}"
    "#;

    // Parse the configuration
    let config = GeneratorConfigFile::from_yaml(config_content).unwrap();

    // Validate global settings
    assert_eq!(config.global.tool_prefix, Some("test".to_string()));
    assert_eq!(config.global.output_dir, Some("./test-output".to_string()));
    
    // Validate global auth
    if let Some(ref auth) = config.global.auth {
        match &auth.auth_type {
            AuthType::Bearer { token } => assert_eq!(token, "global-token"),
            _ => panic!("Expected Bearer auth type"),
        }
        assert_eq!(auth.headers.get("X-Global-Header").unwrap(), "global-value");
    } else {
        panic!("Global auth not found");
    }

    // Validate GraphQL settings
    if let Some(ref graphql) = config.graphql {
        assert_eq!(graphql.endpoint, "https://graphql.example.com");
        assert_eq!(graphql.tool_prefix, Some("gql".to_string()));
        assert_eq!(graphql.include_deprecated, true);
        
        // Validate GraphQL auth
        if let Some(ref auth) = graphql.auth {
            match &auth.auth_type {
                AuthType::ApiKey { key, header } => {
                    assert_eq!(key, "graphql-key");
                    assert_eq!(header, "X-API-Key");
                },
                _ => panic!("Expected ApiKey auth type"),
            }
        } else {
            panic!("GraphQL auth not found");
        }
    } else {
        panic!("GraphQL config not found");
    }

    // Validate gRPC settings
    if let Some(ref grpc) = config.grpc {
        assert_eq!(grpc.endpoint, "grpc.example.com:50051");
        assert_eq!(grpc.tool_prefix, Some("rpc".to_string()));
        assert_eq!(grpc.server_streaming_strategy, "polling");
        assert_eq!(grpc.client_streaming_strategy, "pagination");
        assert_eq!(grpc.bidirectional_streaming_strategy, "agent-level");
    } else {
        panic!("gRPC config not found");
    }

    // Validate OpenAPI settings
    if let Some(ref openapi) = config.openapi {
        assert_eq!(openapi.base_url, "https://api.example.com");
        assert_eq!(openapi.tool_prefix, Some("api".to_string()));
        assert_eq!(openapi.naming_convention, "method-path");
        assert_eq!(openapi.methods, Some(vec!["GET".to_string(), "POST".to_string()]));
        assert_eq!(openapi.include_deprecated, false);
    } else {
        panic!("OpenAPI config not found");
    }

    // Validate output settings
    assert_eq!(config.output.format, "yaml");
    assert_eq!(config.output.pretty, true);
    assert_eq!(config.output.directory, Some("./output".to_string()));
    assert_eq!(config.output.file_pattern, "{name}_capabilities.{ext}");
}

#[test]
fn test_config_validation() {
    // Test valid configuration
    let valid_config = GeneratorConfigFile {
        global: GlobalConfig {
            tool_prefix: Some("test".to_string()),
            auth: None,
            output_dir: Some("./output".to_string()),
        },
        graphql: Some(GraphQLGeneratorConfig {
            endpoint: "https://graphql.example.com".to_string(),
            schema: None,
            tool_prefix: None,
            auth: None,
            include_deprecated: false,
            include_descriptions: true,
            separate_mutation_query: true,
        }),
        grpc: Some(GrpcGeneratorConfig {
            endpoint: "grpc.example.com:50051".to_string(),
            tool_prefix: None,
            auth: None,
            service_filter: None,
            method_filter: None,
            server_streaming_strategy: "polling".to_string(),
            client_streaming_strategy: "polling".to_string(),
            bidirectional_streaming_strategy: "polling".to_string(),
            include_method_options: false,
            separate_streaming_tools: false,
        }),
        openapi: Some(OpenAPIGeneratorConfig {
            base_url: "https://api.example.com".to_string(),
            tool_prefix: None,
            auth: None,
            naming_convention: "operation-id".to_string(),
            methods: None,
            include_deprecated: false,
        }),
        output: OutputConfig {
            format: "yaml".to_string(),
            pretty: true,
            directory: None,
            file_pattern: "{name}-capabilities.{ext}".to_string(),
        },
    };
    assert!(valid_config.validate().is_ok());

    // Test invalid GraphQL endpoint
    let mut invalid_config = valid_config.clone();
    if let Some(ref mut graphql) = invalid_config.graphql {
        graphql.endpoint = "".to_string();
    }
    assert!(invalid_config.validate().is_err());

    // Test invalid gRPC streaming strategy
    let mut invalid_config = valid_config.clone();
    if let Some(ref mut grpc) = invalid_config.grpc {
        grpc.server_streaming_strategy = "invalid".to_string();
    }
    assert!(invalid_config.validate().is_err());

    // Test invalid OpenAPI naming convention
    let mut invalid_config = valid_config.clone();
    if let Some(ref mut openapi) = invalid_config.openapi {
        openapi.naming_convention = "invalid".to_string();
    }
    assert!(invalid_config.validate().is_err());

    // Test invalid output format
    let mut invalid_config = valid_config.clone();
    invalid_config.output.format = "invalid".to_string();
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_base_config_generation() {
    // Create a test configuration
    let config = GeneratorConfigFile {
        global: GlobalConfig {
            tool_prefix: Some("global".to_string()),
            auth: Some(AuthConfig {
                auth_type: AuthType::Bearer { token: "global-token".to_string() },
                headers: HashMap::new(),
            }),
            output_dir: Some("./global-output".to_string()),
        },
        graphql: Some(GraphQLGeneratorConfig {
            endpoint: "https://graphql.example.com".to_string(),
            schema: None,
            tool_prefix: Some("graphql".to_string()),
            auth: Some(AuthConfig {
                auth_type: AuthType::Bearer { token: "graphql-token".to_string() },
                headers: HashMap::new(),
            }),
            include_deprecated: false,
            include_descriptions: true,
            separate_mutation_query: true,
        }),
        grpc: Some(GrpcGeneratorConfig {
            endpoint: "grpc.example.com:50051".to_string(),
            tool_prefix: None,
            auth: None,
            service_filter: None,
            method_filter: None,
            server_streaming_strategy: "polling".to_string(),
            client_streaming_strategy: "polling".to_string(),
            bidirectional_streaming_strategy: "polling".to_string(),
            include_method_options: false,
            separate_streaming_tools: false,
        }),
        openapi: Some(OpenAPIGeneratorConfig {
            base_url: "https://api.example.com".to_string(),
            tool_prefix: None,
            auth: None,
            naming_convention: "operation-id".to_string(),
            methods: None,
            include_deprecated: false,
        }),
        output: OutputConfig {
            format: "yaml".to_string(),
            pretty: true,
            directory: Some("./output".to_string()),
            file_pattern: "{name}-capabilities.{ext}".to_string(),
        },
    };

    // Test GraphQL base config (should use GraphQL-specific settings)
    let graphql_base = config.get_base_config("graphql");
    assert_eq!(graphql_base.tool_prefix, Some("graphql".to_string()));
    if let Some(ref auth) = graphql_base.auth_config {
        match &auth.auth_type {
            AuthType::Bearer { token } => assert_eq!(token, "graphql-token"),
            _ => panic!("Expected Bearer auth type"),
        }
    } else {
        panic!("GraphQL auth not found");
    }

    // Test gRPC base config (should fall back to global settings for tool_prefix and auth)
    let grpc_base = config.get_base_config("grpc");
    assert_eq!(grpc_base.tool_prefix, Some("global".to_string()));
    if let Some(ref auth) = grpc_base.auth_config {
        match &auth.auth_type {
            AuthType::Bearer { token } => assert_eq!(token, "global-token"),
            _ => panic!("Expected Bearer auth type"),
        }
    } else {
        panic!("gRPC auth not found");
    }

    // Test OpenAPI base config (should fall back to global settings for tool_prefix and auth)
    let openapi_base = config.get_base_config("openapi");
    assert_eq!(openapi_base.tool_prefix, Some("global".to_string()));
    if let Some(ref auth) = openapi_base.auth_config {
        match &auth.auth_type {
            AuthType::Bearer { token } => assert_eq!(token, "global-token"),
            _ => panic!("Expected Bearer auth type"),
        }
    } else {
        panic!("OpenAPI auth not found");
    }
}

#[test]
fn test_config_file_loading() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.yaml");

    // Create a test configuration file
    let config_content = r#"
global:
  tool_prefix: "test"

graphql:
  endpoint: "https://graphql.example.com"
    "#;

    fs::write(&config_path, config_content).unwrap();

    // Load the configuration file
    let config = GeneratorConfigFile::from_file(&config_path).unwrap();
    assert_eq!(config.global.tool_prefix, Some("test".to_string()));
    if let Some(ref graphql) = config.graphql {
        assert_eq!(graphql.endpoint, "https://graphql.example.com");
    } else {
        panic!("GraphQL config not found");
    }

    // Test loading a non-existent file
    let result = GeneratorConfigFile::from_file("non-existent-file.toml");
    assert!(result.is_err());
}