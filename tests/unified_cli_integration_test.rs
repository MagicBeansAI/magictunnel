//! Integration tests for the unified capability generation CLI
//!
//! These tests validate the end-to-end functionality of the unified CLI,
//! including command-line argument parsing, configuration file loading,
//! and execution of the different generators.

use magictunnel::registry::generator_config::{
    GeneratorConfigFile, GlobalConfig, GraphQLGeneratorConfig, GrpcGeneratorConfig, OpenAPIGeneratorConfig, OutputConfig
};
use magictunnel::registry::generator_common::{AuthConfig, AuthType};
use magictunnel::error::Result;
use std::path::{Path, PathBuf};
use std::fs;
use tempfile::tempdir;
use std::process::Command;
use std::env;

/// Helper function to create a test configuration file
fn create_test_config(dir: &Path, config: &GeneratorConfigFile) -> PathBuf {
    let config_path = dir.join("test_config.yaml");
    let config_str = serde_yaml::to_string(config).unwrap();
    fs::write(&config_path, config_str).unwrap();
    config_path
}

/// Helper function to create a GraphQL schema file
fn create_graphql_schema(dir: &Path, schema: &str) -> PathBuf {
    let schema_path = dir.join("schema.graphql");
    fs::write(&schema_path, schema).unwrap();
    schema_path
}

/// Helper function to create a gRPC proto file
fn create_grpc_proto(dir: &Path, proto: &str) -> PathBuf {
    let proto_path = dir.join("service.proto");
    fs::write(&proto_path, proto).unwrap();
    proto_path
}

/// Helper function to create an OpenAPI spec file
fn create_openapi_spec(dir: &Path, spec: &str) -> PathBuf {
    let spec_path = dir.join("openapi.yaml");
    fs::write(&spec_path, spec).unwrap();
    spec_path
}

/// Helper function to run the CLI with arguments
fn run_cli(args: &[&str]) -> std::process::Output {
    let cli_path = env::current_dir().unwrap().join("target/debug/magictunnel-cli");
    
    Command::new(cli_path)
        .args(args)
        .output()
        .expect("Failed to execute CLI")
}

#[test]
fn test_cli_help() {
    let output = run_cli(&["--help"]);
    
    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("magictunnel-cli"));
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("Options:"));
    assert!(stdout.contains("Commands:"));
}

#[test]
fn test_cli_version() {
    let output = run_cli(&["--version"]);
    
    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("magictunnel-cli"));
    assert!(stdout.contains("1.0.0"));
}

#[test]
fn test_graphql_generator_cli() -> Result<()> {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create a simple GraphQL schema
    let schema = r#"
    type Query {
        hello: String
        user(id: ID!): User
    }
    
    type User {
        id: ID!
        name: String!
        email: String
    }
    "#;
    
    let schema_path = create_graphql_schema(temp_dir.path(), schema);
    let output_path = temp_dir.path().join("graphql_capability.yaml");
    
    // Run the CLI with GraphQL generator
    let output = run_cli(&[
        "graphql",
        "--endpoint", "https://example.com/graphql",
        "--schema", schema_path.to_str().unwrap(),
        "--output", output_path.to_str().unwrap(),
        "--prefix", "test",
    ]);
    
    assert!(output.status.success());
    
    // Verify the output file exists
    assert!(output_path.exists());
    
    // Verify the content of the output file
    let content = fs::read_to_string(output_path)?;
    println!("Working test file content: {}", content);
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

    assert!(yaml.is_mapping());
    assert!(yaml.get("tools").is_some());
    assert!(yaml["tools"].is_sequence());

    // Verify tool names have the prefix
    for tool in yaml["tools"].as_sequence().unwrap() {
        assert!(tool["name"].as_str().unwrap().starts_with("test_"));
    }
    
    Ok(())
}

#[test]
fn test_grpc_generator_cli() -> Result<()> {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create a simple gRPC proto file
    let proto = r#"
    syntax = "proto3";
    
    package test;
    
    service UserService {
        rpc GetUser (GetUserRequest) returns (User);
        rpc ListUsers (ListUsersRequest) returns (ListUsersResponse);
    }
    
    message GetUserRequest {
        string user_id = 1;
    }
    
    message User {
        string user_id = 1;
        string name = 2;
        string email = 3;
    }
    
    message ListUsersRequest {
        int32 page_size = 1;
        int32 page_token = 2;
    }
    
    message ListUsersResponse {
        repeated User users = 1;
        string next_page_token = 2;
    }
    "#;
    
    let proto_path = create_grpc_proto(temp_dir.path(), proto);
    let output_path = temp_dir.path().join("grpc_capability.yaml");
    
    // Run the CLI with gRPC generator
    let output = run_cli(&[
        "grpc",
        "--endpoint", "localhost:50051",
        "--proto", proto_path.to_str().unwrap(),
        "--output", output_path.to_str().unwrap(),
        "--prefix", "test",
    ]);
    
    assert!(output.status.success());
    
    // Verify the output file exists
    assert!(output_path.exists());
    
    // Verify the content of the output file
    let content = fs::read_to_string(output_path)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

    assert!(yaml.is_mapping());
    assert!(yaml.get("tools").is_some());
    assert!(yaml["tools"].is_sequence());

    // Verify tool names have the prefix
    for tool in yaml["tools"].as_sequence().unwrap() {
        assert!(tool["name"].as_str().unwrap().starts_with("test_"));
    }
    
    Ok(())
}

#[test]
fn test_openapi_generator_cli() -> Result<()> {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create a simple OpenAPI spec
    let spec = r#"
    openapi: 3.0.0
    info:
      title: Test API
      version: 1.0.0
    paths:
      /users:
        get:
          operationId: listUsers
          summary: List users
          responses:
            '200':
              description: A list of users
              content:
                application/json:
                  schema:
                    type: array
                    items:
                      $ref: '#/components/schemas/User'
        post:
          operationId: createUser
          summary: Create a user
          requestBody:
            content:
              application/json:
                schema:
                  $ref: '#/components/schemas/User'
          responses:
            '201':
              description: User created
    components:
      schemas:
        User:
          type: object
          properties:
            id:
              type: string
            name:
              type: string
            email:
              type: string
    "#;
    
    let spec_path = create_openapi_spec(temp_dir.path(), spec);
    let output_path = temp_dir.path().join("openapi_capability.yaml");
    
    // Run the CLI with OpenAPI generator
    let output = run_cli(&[
        "openapi",
        "--base-url", "https://example.com/api",
        "--spec", spec_path.to_str().unwrap(),
        "--output", output_path.to_str().unwrap(),
        "--prefix", "test",
    ]);
    
    assert!(output.status.success());
    
    // Verify the output file exists
    assert!(output_path.exists());
    
    // Verify the content of the output file
    let content = fs::read_to_string(output_path)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

    assert!(yaml.is_mapping());
    assert!(yaml.get("tools").is_some());
    assert!(yaml["tools"].is_sequence());

    // Verify tool names have the prefix
    for tool in yaml["tools"].as_sequence().unwrap() {
        assert!(tool["name"].as_str().unwrap().starts_with("test_"));
    }
    
    Ok(())
}

#[test]
fn test_cli_with_config_file() -> Result<()> {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create a simple GraphQL schema
    let schema = r#"
    type Query {
        hello: String
    }
    "#;
    
    let schema_path = create_graphql_schema(temp_dir.path(), schema);
    let output_path = temp_dir.path().join("graphql-capabilities.yaml");
    
    // Create a configuration file
    let config = GeneratorConfigFile {
        global: GlobalConfig {
            output_dir: Some(temp_dir.path().to_str().unwrap().to_string()),
            tool_prefix: Some("config_test".to_string()),
            ..Default::default()
        },
        graphql: Some(GraphQLGeneratorConfig {
            endpoint: "https://example.com/graphql".to_string(),
            schema: Some(schema_path.to_str().unwrap().to_string()),
            tool_prefix: None,
            auth: Some(AuthConfig {
                auth_type: AuthType::Bearer { token: "test-token".to_string() },
                headers: std::collections::HashMap::new(),
            }),
            include_deprecated: false,
            include_descriptions: true,
            separate_mutation_query: true,
        }),
        grpc: None,
        openapi: None,
        output: OutputConfig {
            format: "yaml".to_string(),
            pretty: true,
            directory: None,
            file_pattern: "{name}-capabilities.{ext}".to_string(),
        },
    };
    
    let config_path = create_test_config(temp_dir.path(), &config);
    
    // Run the CLI with config file
    let output = run_cli(&[
        "graphql",
        "--config", config_path.to_str().unwrap(),
    ]);
    
    if !output.status.success() {
        eprintln!("CLI command failed!");
        eprintln!("Exit code: {:?}", output.status.code());
        eprintln!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());

    // Verify the output file exists
    assert!(output_path.exists());

    // Verify the content of the output file
    let content = fs::read_to_string(&output_path)?;
    println!("File content: {}", content);
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

    assert!(yaml.is_mapping());
    assert!(yaml.get("tools").is_some());

    // Verify tool names have the prefix from config
    for tool in yaml["tools"].as_sequence().unwrap() {
        assert!(tool["name"].as_str().unwrap().starts_with("config_test_"));
    }
    
    Ok(())
}

#[test]
fn test_merge_command() -> Result<()> {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create two simple capability files
    let capability1 = r#"
    {
        "metadata": {
            "name": "capability1",
            "description": "First test capability"
        },
        "tools": [
            {
                "name": "tool1",
                "description": "First tool",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "param1": {"type": "string"}
                    }
                },
                "routing": {
                    "type": "http",
                    "config": {
                        "url": "https://example.com/api/tool1",
                        "method": "POST"
                    }
                }
            }
        ]
    }
    "#;
    
    let capability2 = r#"
    {
        "metadata": {
            "name": "capability2",
            "description": "Second test capability"
        },
        "tools": [
            {
                "name": "tool2",
                "description": "Second tool",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "param2": {"type": "string"}
                    }
                },
                "routing": {
                    "type": "http",
                    "config": {
                        "url": "https://example.com/api/tool2",
                        "method": "POST"
                    }
                }
            }
        ]
    }
    "#;
    
    let file1_path = temp_dir.path().join("capability1.json");
    let file2_path = temp_dir.path().join("capability2.json");
    let output_path = temp_dir.path().join("merged.yaml");
    
    fs::write(&file1_path, capability1)?;
    fs::write(&file2_path, capability2)?;
    
    // Run the CLI with merge command (comma-separated input files)
    let input_files = format!("{},{}", file1_path.to_str().unwrap(), file2_path.to_str().unwrap());
    let output = run_cli(&[
        "merge",
        "--input", &input_files,
        "--output", output_path.to_str().unwrap(),
        "--strategy", "rename",
    ]);

    assert!(output.status.success());
    
    // Verify the output file exists
    assert!(output_path.exists());
    
    // Verify the content of the output file
    let content = fs::read_to_string(output_path)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

    assert!(yaml.is_mapping());
    assert!(yaml.get("tools").is_some());
    assert_eq!(yaml["tools"].as_sequence().unwrap().len(), 2);
    
    Ok(())
}

#[test]
fn test_validate_command() -> Result<()> {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create a valid capability file
    let capability = r#"
    {
        "metadata": {
            "name": "test_capability",
            "description": "Test capability file"
        },
        "tools": [
            {
                "name": "test_tool",
                "description": "Test tool",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "param": {"type": "string"}
                    }
                },
                "routing": {
                    "type": "http",
                    "config": {
                        "url": "https://example.com/api",
                        "method": "POST"
                    }
                }
            }
        ]
    }
    "#;
    
    let file_path = temp_dir.path().join("capability.json");
    fs::write(&file_path, capability)?;
    
    // Run the CLI with validate command
    let output = run_cli(&[
        "validate",
        "--input", file_path.to_str().unwrap(),
        "--strict",
    ]);

    assert!(output.status.success());
    
    // Create an invalid capability file
    let invalid_capability = r#"
    {
        "metadata": {
            "name": "invalid_capability"
            // Missing description
        },
        "tools": [
            {
                "name": "", // Empty name is invalid
                "description": "Invalid tool",
                "inputSchema": {
                    "type": "string" // Not an object schema
                },
                "routing": {
                    "type": "http",
                    "config": {
                        // Missing required 'method' field
                        "url": "https://example.com/api"
                    }
                }
            }
        ]
    }
    "#;
    
    let invalid_file_path = temp_dir.path().join("invalid_capability.json");
    fs::write(&invalid_file_path, invalid_capability)?;
    
    // Run the CLI with validate command on invalid file
    let invalid_output = run_cli(&[
        "validate",
        "--input", invalid_file_path.to_str().unwrap(),
    ]);
    
    // Should fail validation
    assert!(!invalid_output.status.success());
    
    Ok(())
}