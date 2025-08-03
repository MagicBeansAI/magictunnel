//! Unified Capability Generator CLI
//!
//! This is the main entry point for the unified capability generation CLI.
//! It supports generating capability files from various sources (GraphQL, gRPC, OpenAPI)
//! using a common interface and configuration format.
//!
//! # Overview
//!
//! The `magictunnel-cli` provides a unified interface for generating MCP capability files
//! from different API definition formats. It supports:
//!
//! - GraphQL schemas (SDL or JSON introspection)
//! - gRPC/protobuf service definitions
//! - OpenAPI specifications (v3.0, JSON or YAML)
//!
//! # Features
//!
//! - **Configuration File Support**: Use TOML configuration files for complex setups
//! - **Multiple Generator Types**: Support for GraphQL, gRPC, and OpenAPI
//! - **Authentication**: Various authentication methods (Bearer, API Key, Basic, OAuth)
//! - **Customization**: Prefix, naming conventions, filtering, and more
//! - **Utility Commands**: Initialize config files, merge capability files, validate
//!
//! # Usage
//!
//! ```bash
//! # Generate from GraphQL schema
//! magictunnel-cli graphql --schema schema.graphql --endpoint https://api.example.com/graphql --output capabilities.yaml
//!
//! # Generate from gRPC protobuf
//! magictunnel-cli grpc --proto service.proto --endpoint localhost:50051 --output capabilities.yaml
//!
//! # Generate from OpenAPI specification
//! magictunnel-cli openapi --spec openapi.json --base-url https://api.example.com --output capabilities.yaml
//!
//! # Initialize a configuration file
//! magictunnel-cli init --output config.yaml
//!
//! # Merge capability files
//! magictunnel-cli merge --input file1.yaml,file2.yaml --output merged.yaml
//!
//! # Validate capability files
//! magictunnel-cli validate --input capabilities.yaml --strict
//! ```

use clap::{Arg, ArgMatches, Command, ArgAction};
use magictunnel::error::{ProxyError, Result};
use magictunnel::registry::{
    generator_common::{
        read_file_content, write_capability_file, CapabilityGeneratorBase,
        AuthConfig, AuthType
    },
    generator_config::{GeneratorConfigFile, example_config_yaml},
    graphql_generator::{AuthConfig as GraphQLAuthConfig, AuthType as GraphQLAuthType},
    grpc_generator::{GrpcCapabilityGenerator, GrpcGeneratorConfig, StreamingStrategy, AuthConfig as GrpcAuthConfig, AuthType as GrpcAuthType},
    openapi_generator::{OpenAPICapabilityGenerator, NamingConvention, AuthConfig as OpenAPIAuthConfig, AuthType as OpenAPIAuthType},
    types::CapabilityFile,
    commands::{
        GraphQLGeneratorAdapter, GrpcGeneratorAdapter, OpenAPIGeneratorAdapter,
        CapabilityMerger, CapabilityValidator, merge::MergeStrategy
    },
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use reqwest::Client;
use serde_json::{json, Value};
use std::io::{self, Write};
use base64::{Engine as _, engine::general_purpose};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("magictunnel-cli")
        .about("MagicTunnel CLI - Unified MCP Capability Generator")
        .version("1.0.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("graphql")
                .about("Generate capabilities from GraphQL schema")
                .arg(
                    Arg::new("schema")
                        .short('s')
                        .long("schema")
                        .value_name("FILE")
                        .help("GraphQL schema file (SDL or JSON introspection)")
                        .required_unless_present("config")
                )
                .arg(
                    Arg::new("endpoint")
                        .short('e')
                        .long("endpoint")
                        .value_name("URL")
                        .help("GraphQL endpoint URL")
                        .required_unless_present("config")
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Output capability file (YAML)")
                        .required_unless_present("config")
                )
                .arg(
                    Arg::new("prefix")
                        .short('p')
                        .long("prefix")
                        .value_name("PREFIX")
                        .help("Tool name prefix")
                )
                .arg(
                    Arg::new("auth-type")
                        .short('a')
                        .long("auth-type")
                        .value_name("TYPE")
                        .help("Authentication type: none, bearer, apikey")
                        .default_value("none")
                )
                .arg(
                    Arg::new("auth-token")
                        .short('t')
                        .long("auth-token")
                        .value_name("TOKEN")
                        .help("Authentication token (for bearer or apikey)")
                )
                .arg(
                    Arg::new("auth-header")
                        .long("auth-header")
                        .value_name("HEADER")
                        .help("Authentication header name (for apikey)")
                        .default_value("Authorization")
                )
                .arg(
                    Arg::new("config")
                        .short('c')
                        .long("config")
                        .value_name("FILE")
                        .help("Configuration file (TOML)")
                )
        )
        .subcommand(
            Command::new("grpc")
                .about("Generate capabilities from gRPC/protobuf")
                .arg(
                    Arg::new("proto")
                        .short('p')
                        .long("proto")
                        .value_name("FILE")
                        .help("Protobuf (.proto) file containing service definitions")
                        .required_unless_present("config")
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Output capability file (YAML)")
                        .required_unless_present("config")
                )
                .arg(
                    Arg::new("endpoint")
                        .short('e')
                        .long("endpoint")
                        .value_name("ENDPOINT")
                        .help("gRPC service endpoint (e.g., localhost:50051)")
                        .required_unless_present("config")
                )
                .arg(
                    Arg::new("prefix")
                        .long("prefix")
                        .value_name("PREFIX")
                        .help("Tool name prefix")
                )
                .arg(
                    Arg::new("service-filter")
                        .long("service-filter")
                        .value_name("SERVICES")
                        .help("Comma-separated list of service names to include")
                )
                .arg(
                    Arg::new("method-filter")
                        .long("method-filter")
                        .value_name("METHODS")
                        .help("Comma-separated list of method names to include")
                )
                .arg(
                    Arg::new("server-streaming")
                        .long("server-streaming")
                        .value_name("STRATEGY")
                        .help("Strategy for server streaming methods (polling, pagination, agent-level)")
                        .default_value("polling")
                )
                .arg(
                    Arg::new("client-streaming")
                        .long("client-streaming")
                        .value_name("STRATEGY")
                        .help("Strategy for client streaming methods (polling, pagination, agent-level)")
                        .default_value("polling")
                )
                .arg(
                    Arg::new("bidirectional-streaming")
                        .long("bidirectional-streaming")
                        .value_name("STRATEGY")
                        .help("Strategy for bidirectional streaming methods (polling, pagination, agent-level)")
                        .default_value("polling")
                )
                .arg(
                    Arg::new("include-method-options")
                        .long("include-method-options")
                        .help("Include method options in tool definitions")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("separate-streaming-tools")
                        .long("separate-streaming-tools")
                        .help("Generate separate tools for streaming methods")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("auth-type")
                        .short('a')
                        .long("auth-type")
                        .value_name("TYPE")
                        .help("Authentication type (none, bearer, apikey, basic, oauth)")
                        .default_value("none")
                )
                .arg(
                    Arg::new("auth-token")
                        .long("auth-token")
                        .value_name("TOKEN")
                        .help("Authentication token (for bearer/apikey/oauth auth)")
                )
                .arg(
                    Arg::new("auth-header")
                        .long("auth-header")
                        .value_name("HEADER")
                        .help("Authentication header name (for apikey auth)")
                        .default_value("X-API-Key")
                )
                .arg(
                    Arg::new("auth-username")
                        .long("auth-username")
                        .value_name("USERNAME")
                        .help("Username (for basic auth)")
                )
                .arg(
                    Arg::new("auth-password")
                        .long("auth-password")
                        .value_name("PASSWORD")
                        .help("Password (for basic auth)")
                )
                .arg(
                    Arg::new("auth-token-type")
                        .long("auth-token-type")
                        .value_name("TYPE")
                        .help("Token type (for oauth auth)")
                        .default_value("Bearer")
                )
                .arg(
                    Arg::new("config")
                        .short('c')
                        .long("config")
                        .value_name("FILE")
                        .help("Configuration file (TOML)")
                )
        )
        .subcommand(
            Command::new("openapi")
                .about("Generate capabilities from OpenAPI specification")
                .arg(
                    Arg::new("spec")
                        .short('s')
                        .long("spec")
                        .value_name("FILE")
                        .help("OpenAPI specification file (JSON or YAML)")
                        .required_unless_present("config")
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Output capability file (YAML)")
                        .required_unless_present("config")
                )
                .arg(
                    Arg::new("base-url")
                        .short('u')
                        .long("base-url")
                        .value_name("URL")
                        .help("Base URL for the API")
                        .required_unless_present("config")
                )
                .arg(
                    Arg::new("prefix")
                        .short('p')
                        .long("prefix")
                        .value_name("PREFIX")
                        .help("Tool name prefix")
                )
                .arg(
                    Arg::new("auth-type")
                        .short('a')
                        .long("auth-type")
                        .value_name("TYPE")
                        .help("Authentication type (none, bearer, apikey, basic)")
                        .default_value("none")
                )
                .arg(
                    Arg::new("auth-token")
                        .short('t')
                        .long("auth-token")
                        .value_name("TOKEN")
                        .help("Authentication token (for bearer/apikey auth)")
                )
                .arg(
                    Arg::new("auth-header")
                        .long("auth-header")
                        .value_name("HEADER")
                        .help("Authentication header name (for apikey auth)")
                        .default_value("X-API-Key")
                )
                .arg(
                    Arg::new("auth-username")
                        .long("auth-username")
                        .value_name("USERNAME")
                        .help("Username (for basic auth)")
                )
                .arg(
                    Arg::new("auth-password")
                        .long("auth-password")
                        .value_name("PASSWORD")
                        .help("Password (for basic auth)")
                )
                .arg(
                    Arg::new("naming")
                        .short('n')
                        .long("naming")
                        .value_name("CONVENTION")
                        .help("Naming convention (operation-id, method-path)")
                        .default_value("operation-id")
                )
                .arg(
                    Arg::new("methods")
                        .short('m')
                        .long("methods")
                        .value_name("METHODS")
                        .help("Comma-separated list of HTTP methods to include (e.g., GET,POST)")
                )
                .arg(
                    Arg::new("include-deprecated")
                        .long("include-deprecated")
                        .help("Include deprecated operations")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("config")
                        .short('c')
                        .long("config")
                        .value_name("FILE")
                        .help("Configuration file (TOML)")
                )
        )
        .subcommand(
            Command::new("init")
                .about("Initialize a new configuration file")
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Output configuration file")
                        .default_value("magictunnel-cli.yaml")
                )
        )
        .subcommand(
            Command::new("merge")
                .about("Merge multiple capability files into one")
                .arg(
                    Arg::new("input")
                        .short('i')
                        .long("input")
                        .value_name("FILES")
                        .help("Input capability files (comma-separated)")
                        .required(true)
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Output merged capability file")
                        .required(true)
                )
                .arg(
                    Arg::new("strategy")
                        .short('s')
                        .long("strategy")
                        .value_name("STRATEGY")
                        .help("Merge strategy for handling duplicates (keep-first, keep-last, rename, error)")
                        .default_value("error")
                )
        )
        .subcommand(
            Command::new("validate")
                .about("Validate capability files")
                .arg(
                    Arg::new("input")
                        .short('i')
                        .long("input")
                        .value_name("FILES")
                        .help("Input capability files to validate (comma-separated)")
                        .required(true)
                )
                .arg(
                    Arg::new("strict")
                        .short('s')
                        .long("strict")
                        .help("Enable strict validation")
                        .action(ArgAction::SetTrue)
                )
        )
        // MCP Resources Management
        .subcommand(
            Command::new("resources")
                .about("Manage MCP resources")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .arg(
                    Arg::new("server")
                        .long("server")
                        .value_name("URL")
                        .help("MagicTunnel server URL")
                        .default_value("http://localhost:3001")
                )
                .subcommand(
                    Command::new("list")
                        .about("List all available resources")
                )
                .subcommand(
                    Command::new("read")
                        .about("Read resource content by URI")
                        .arg(
                            Arg::new("uri")
                                .value_name("URI")
                                .help("Resource URI to read")
                                .required(true)
                        )
                )
                .subcommand(
                    Command::new("export")
                        .about("Export resource content to file")
                        .arg(
                            Arg::new("uri")
                                .value_name("URI")
                                .help("Resource URI to export")
                                .required(true)
                        )
                        .arg(
                            Arg::new("output")
                                .short('o')
                                .long("output")
                                .value_name("FILE")
                                .help("Output file path")
                                .required(true)
                        )
                )
        )
        // MCP Prompts Management
        .subcommand(
            Command::new("prompts")
                .about("Manage MCP prompt templates")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .arg(
                    Arg::new("server")
                        .long("server")
                        .value_name("URL")
                        .help("MagicTunnel server URL")
                        .default_value("http://localhost:3001")
                )
                .subcommand(
                    Command::new("list")
                        .about("List all prompt templates")
                )
                .subcommand(
                    Command::new("execute")
                        .about("Execute a prompt template")
                        .arg(
                            Arg::new("name")
                                .value_name("NAME")
                                .help("Prompt template name")
                                .required(true)
                        )
                        .arg(
                            Arg::new("args")
                                .long("args")
                                .value_name("JSON")
                                .help("Arguments as JSON object")
                        )
                )
                .subcommand(
                    Command::new("export")
                        .about("Export prompt template to file")
                        .arg(
                            Arg::new("name")
                                .value_name("NAME")
                                .help("Prompt template name")
                                .required(true)
                        )
                        .arg(
                            Arg::new("output")
                                .short('o')
                                .long("output")
                                .value_name("FILE")
                                .help("Output file path")
                                .required(true)
                        )
                )
        )
        // Tools Management
        .subcommand(
            Command::new("tools")
                .about("Manage MCP tools")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .arg(
                    Arg::new("server")
                        .long("server")
                        .value_name("URL")
                        .help("MagicTunnel server URL")
                        .default_value("http://localhost:3001")
                )
                .subcommand(
                    Command::new("list")
                        .about("List all available tools")
                        .arg(
                            Arg::new("filter")
                                .long("filter")
                                .value_name("TYPE")
                                .help("Filter tools by type (enabled, disabled, hidden, all)")
                                .default_value("enabled")
                        )
                )
                .subcommand(
                    Command::new("execute")
                        .about("Execute a tool")
                        .arg(
                            Arg::new("name")
                                .value_name("NAME")
                                .help("Tool name to execute")
                                .required(true)
                        )
                        .arg(
                            Arg::new("args")
                                .long("args")
                                .value_name("JSON")
                                .help("Arguments as JSON object")
                        )
                )
                .subcommand(
                    Command::new("info")
                        .about("Show detailed tool information")
                        .arg(
                            Arg::new("name")
                                .value_name("NAME")
                                .help("Tool name")
                                .required(true)
                        )
                )
        )
        // Services Management
        .subcommand(
            Command::new("services")
                .about("Manage MCP services")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .arg(
                    Arg::new("server")
                        .long("server")
                        .value_name("URL")
                        .help("MagicTunnel server URL")
                        .default_value("http://localhost:3001")
                )
                .subcommand(
                    Command::new("list")
                        .about("List all MCP services")
                )
                .subcommand(
                    Command::new("restart")
                        .about("Restart a service")
                        .arg(
                            Arg::new("name")
                                .value_name("NAME")
                                .help("Service name to restart")
                                .required(true)
                        )
                )
                .subcommand(
                    Command::new("start")
                        .about("Start a service")
                        .arg(
                            Arg::new("name")
                                .value_name("NAME")
                                .help("Service name to start")
                                .required(true)
                        )
                )
                .subcommand(
                    Command::new("stop")
                        .about("Stop a service")
                        .arg(
                            Arg::new("name")
                                .value_name("NAME")
                                .help("Service name to stop")
                                .required(true)
                        )
                )
        )
        // Server Management
        .subcommand(
            Command::new("server")
                .about("Manage MagicTunnel server")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .arg(
                    Arg::new("server")
                        .long("server")
                        .value_name("URL")
                        .help("MagicTunnel server URL")
                        .default_value("http://localhost:3001")
                )
                .subcommand(
                    Command::new("status")
                        .about("Get server status")
                )
                .subcommand(
                    Command::new("restart")
                        .about("Restart the server")
                        .arg(
                            Arg::new("args")
                                .long("args")
                                .value_name("ARGS")
                                .help("Startup arguments")
                        )
                )
                .subcommand(
                    Command::new("health")
                        .about("Perform health check")
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("graphql", sub_matches)) => {
            if let Some(config_file) = sub_matches.get_one::<String>("config") {
                generate_from_config(config_file, "graphql")?;
            } else {
                generate_graphql_from_args(sub_matches)?;
            }
        },
        Some(("grpc", sub_matches)) => {
            if let Some(config_file) = sub_matches.get_one::<String>("config") {
                generate_from_config(config_file, "grpc")?;
            } else {
                generate_grpc_from_args(sub_matches)?;
            }
        },
        Some(("openapi", sub_matches)) => {
            if let Some(config_file) = sub_matches.get_one::<String>("config") {
                generate_from_config(config_file, "openapi")?;
            } else {
                generate_openapi_from_args(sub_matches)?;
            }
        },
        Some(("merge", sub_matches)) => {
            merge_capability_files(sub_matches)?;
        },
        Some(("validate", sub_matches)) => {
            validate_capability_files(sub_matches)?;
        },
        Some(("init", sub_matches)) => {
            let output_file = sub_matches.get_one::<String>("output").unwrap();
            initialize_config_file(output_file)?;
        },
        Some(("resources", sub_matches)) => {
            let server_url = sub_matches.get_one::<String>("server").unwrap();
            handle_resources_command(sub_matches, server_url).await?;
        },
        Some(("prompts", sub_matches)) => {
            let server_url = sub_matches.get_one::<String>("server").unwrap();
            handle_prompts_command(sub_matches, server_url).await?;
        },
        Some(("tools", sub_matches)) => {
            let server_url = sub_matches.get_one::<String>("server").unwrap();
            handle_tools_command(sub_matches, server_url).await?;
        },
        Some(("services", sub_matches)) => {
            let server_url = sub_matches.get_one::<String>("server").unwrap();
            handle_services_command(sub_matches, server_url).await?;
        },
        Some(("server", sub_matches)) => {
            let server_url = sub_matches.get_one::<String>("server").unwrap();
            handle_server_command(sub_matches, server_url).await?;
        },
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }

    Ok(())
}

/// Convert common AuthConfig to GraphQLAuthConfig
///
/// This function converts the generic AuthConfig to the GraphQL-specific AuthConfig format.
/// It handles all authentication types and performs necessary transformations for types
/// that aren't directly supported by GraphQL (like Basic auth and OAuth).
///
/// # Arguments
///
/// * `auth` - The common AuthConfig to convert
///
/// # Returns
///
/// A GraphQLAuthConfig with the equivalent authentication settings
fn convert_to_graphql_auth(auth: &AuthConfig) -> GraphQLAuthConfig {
    match &auth.auth_type {
        AuthType::None => GraphQLAuthConfig {
            auth_type: GraphQLAuthType::None,
            headers: auth.headers.clone(),
        },
        AuthType::Bearer { token } => GraphQLAuthConfig {
            auth_type: GraphQLAuthType::Bearer { token: token.clone() },
            headers: auth.headers.clone(),
        },
        AuthType::ApiKey { key, header } => GraphQLAuthConfig {
            auth_type: GraphQLAuthType::ApiKey {
                value: key.clone(),
                header: header.clone(),
            },
            headers: auth.headers.clone(),
        },
        AuthType::Basic { username, password } => {
            // GraphQL doesn't support Basic auth directly, so we'll use a header
            let mut headers = auth.headers.clone();
            let auth_value = format!("Basic {}", base64::encode(format!("{}:{}", username, password)));
            headers.insert("Authorization".to_string(), auth_value);
            GraphQLAuthConfig {
                auth_type: GraphQLAuthType::None,
                headers,
            }
        },
        AuthType::OAuth { token, token_type } => {
            // GraphQL doesn't support OAuth directly, so we'll use a Bearer token
            GraphQLAuthConfig {
                auth_type: GraphQLAuthType::Bearer { token: token.clone() },
                headers: auth.headers.clone(),
            }
        },
    }
}

/// Convert common AuthConfig to GrpcAuthConfig
///
/// This function converts the generic AuthConfig to the gRPC-specific AuthConfig format.
/// It preserves all authentication types and headers.
///
/// # Arguments
///
/// * `auth` - The common AuthConfig to convert
///
/// # Returns
///
/// A GrpcAuthConfig with the equivalent authentication settings
fn convert_to_grpc_auth(auth: &AuthConfig) -> GrpcAuthConfig {
    match &auth.auth_type {
        AuthType::None => GrpcAuthConfig {
            auth_type: GrpcAuthType::None,
            headers: auth.headers.clone(),
        },
        AuthType::Bearer { token } => GrpcAuthConfig {
            auth_type: GrpcAuthType::Bearer { token: token.clone() },
            headers: auth.headers.clone(),
        },
        AuthType::ApiKey { key, header } => GrpcAuthConfig {
            auth_type: GrpcAuthType::ApiKey {
                key: key.clone(),
                header: header.clone(),
            },
            headers: auth.headers.clone(),
        },
        AuthType::Basic { username, password } => GrpcAuthConfig {
            auth_type: GrpcAuthType::Basic {
                username: username.clone(),
                password: password.clone(),
            },
            headers: auth.headers.clone(),
        },
        AuthType::OAuth { token, token_type } => GrpcAuthConfig {
            auth_type: GrpcAuthType::OAuth {
                token: token.clone(),
                token_type: token_type.clone(),
            },
            headers: auth.headers.clone(),
        },
    }
}

/// Convert common AuthConfig to OpenAPIAuthConfig
///
/// This function converts the generic AuthConfig to the OpenAPI-specific AuthConfig format.
/// It handles all authentication types and performs necessary transformations for types
/// that aren't directly supported by OpenAPI.
///
/// # Arguments
///
/// * `auth` - The common AuthConfig to convert
///
/// # Returns
///
/// An OpenAPIAuthConfig with the equivalent authentication settings
fn convert_to_openapi_auth(auth: &AuthConfig) -> OpenAPIAuthConfig {
    match &auth.auth_type {
        AuthType::None => OpenAPIAuthConfig {
            auth_type: OpenAPIAuthType::None,
            headers: auth.headers.clone(),
        },
        AuthType::Bearer { token } => OpenAPIAuthConfig {
            auth_type: OpenAPIAuthType::Bearer { token: token.clone() },
            headers: auth.headers.clone(),
        },
        AuthType::ApiKey { key, header } => OpenAPIAuthConfig {
            auth_type: OpenAPIAuthType::ApiKey {
                key: key.clone(),
                header: header.clone(),
            },
            headers: auth.headers.clone(),
        },
        AuthType::Basic { username, password } => OpenAPIAuthConfig {
            auth_type: OpenAPIAuthType::Basic {
                username: username.clone(),
                password: password.clone(),
            },
            headers: auth.headers.clone(),
        },
        AuthType::OAuth { token, token_type } => {
            // OpenAPI doesn't have OAuth directly, so we'll use Bearer
            OpenAPIAuthConfig {
                auth_type: OpenAPIAuthType::Bearer { token: token.clone() },
                headers: auth.headers.clone(),
            }
        },
    }
}

/// Generate capabilities from a configuration file
///
/// This function reads a YAML or TOML configuration file and generates capability files
/// based on the specified generator type (GraphQL, gRPC, or OpenAPI).
/// Auto-detects format based on file extension.
///
/// # Arguments
///
/// * `config_file` - Path to the YAML or TOML configuration file
/// * `generator_type` - The type of generator to use ("graphql", "grpc", or "openapi")
///
/// # Returns
///
/// A Result indicating success or an error with details
///
/// # Example
///
/// ```
/// generate_from_config("config.yaml", "graphql")?;
/// generate_from_config("config.toml", "graphql")?; // Still supported
/// ```
fn generate_from_config(config_file: &str, generator_type: &str) -> Result<()> {
    println!("Loading configuration from '{}'...", config_file);
    let config = GeneratorConfigFile::from_file(config_file)?;
    
    // Validate the configuration
    config.validate()?;
    
    match generator_type {
        "graphql" => {
            if let Some(graphql_config) = &config.graphql {
                let endpoint = &graphql_config.endpoint;
                println!("Generating GraphQL capabilities for endpoint: {}", endpoint);
                

                
                // Determine output path
                let output_path = determine_output_path(&config, "graphql")?;

                // Get schema file path
                let schema_file = if let Some(schema) = &graphql_config.schema {
                    schema.clone()
                } else {
                    return Err(ProxyError::config("GraphQL schema file not specified in configuration. Please add 'schema' field to the [graphql] section."));
                };

                // Read the schema file
                println!("Reading GraphQL schema from '{}'...", schema_file);
                let schema_content = fs::read_to_string(&schema_file)
                    .map_err(|e| ProxyError::config(format!("Failed to read schema file '{}': {}", schema_file, e)))?;

                // Create generator adapter
                let mut adapter = GraphQLGeneratorAdapter::new(endpoint.clone());

                // Add prefix if specified
                if let Some(prefix) = &graphql_config.tool_prefix {
                    adapter = adapter.with_prefix(prefix.clone());
                } else if let Some(prefix) = &config.global.tool_prefix {
                    adapter = adapter.with_prefix(prefix.clone());
                }

                // Apply authentication
                if let Some(auth) = &graphql_config.auth {
                    // Convert common AuthConfig to GraphQLAuthConfig
                    let graphql_auth = convert_to_graphql_auth(auth);
                    adapter = adapter.with_auth(graphql_auth);
                } else if let Some(auth) = &config.global.auth {
                    // Convert common AuthConfig to GraphQLAuthConfig
                    let graphql_auth = convert_to_graphql_auth(auth);
                    adapter = adapter.with_auth(graphql_auth);
                }

                // Generate capability file
                println!("Parsing GraphQL schema...");
                let capability_file = adapter.generate_from_content(&schema_content)
                    .map_err(|e| ProxyError::config(format!("Failed to generate capability file from GraphQL schema: {:?}", e)))?;

                println!("Generated {} tools from GraphQL schema", capability_file.tools.len());

                // Write output file
                write_capability_file(&capability_file, &output_path)?;

                println!("Capability file written to '{}'", output_path.display());

                // Print summary
                println!("\nSummary:");
                println!("  Schema file: {}", schema_file);
                // Determine schema format
                let schema_format = if schema_content.trim_start().starts_with('{') {
                    "JSON Introspection"
                } else {
                    "GraphQL SDL"
                };
                println!("  Schema format: {}", schema_format);
                println!("  Endpoint: {}", endpoint);
                println!("  Output file: {}", output_path.display());
                if let Some(prefix) = &graphql_config.tool_prefix {
                    println!("  Tool prefix: {}", prefix);
                } else if let Some(prefix) = &config.global.tool_prefix {
                    println!("  Tool prefix: {}", prefix);
                }
                println!("  Tools generated: {}", capability_file.tools.len());
            } else {
                return Err(ProxyError::config("GraphQL configuration not found in config file"));
            }
        },
        "grpc" => {
            if let Some(grpc_config) = &config.grpc {
                let endpoint = &grpc_config.endpoint;
                println!("Generating gRPC capabilities for endpoint: {}", endpoint);
                
                // Create generator config
                let mut generator_config = GrpcGeneratorConfig {
                    endpoint: endpoint.clone(),
                    auth_config: None,
                    tool_prefix: None,
                    service_filter: grpc_config.service_filter.clone(),
                    method_filter: grpc_config.method_filter.clone(),
                    server_streaming_strategy: parse_streaming_strategy(&grpc_config.server_streaming_strategy)?,
                    client_streaming_strategy: parse_streaming_strategy(&grpc_config.client_streaming_strategy)?,
                    bidirectional_streaming_strategy: parse_streaming_strategy(&grpc_config.bidirectional_streaming_strategy)?,
                    include_method_options: grpc_config.include_method_options,
                    separate_streaming_tools: grpc_config.separate_streaming_tools,
                    use_enhanced_format: true, // Always use enhanced format
                };
                
                // Apply tool prefix
                if let Some(prefix) = &grpc_config.tool_prefix {
                    generator_config.tool_prefix = Some(prefix.clone());
                } else if let Some(prefix) = &config.global.tool_prefix {
                    generator_config.tool_prefix = Some(prefix.clone());
                }
                
                // Apply authentication
                if let Some(auth) = &grpc_config.auth {
                    // Convert common AuthConfig to GrpcAuthConfig
                    let grpc_auth = convert_to_grpc_auth(auth);
                    generator_config.auth_config = Some(grpc_auth);
                } else if let Some(auth) = &config.global.auth {
                    // Convert common AuthConfig to GrpcAuthConfig
                    let grpc_auth = convert_to_grpc_auth(auth);
                    generator_config.auth_config = Some(grpc_auth);
                }
                
                // Create generator
                let generator = GrpcCapabilityGenerator::new(generator_config);
                
                // Determine output path
                let output_path = determine_output_path(&config, "grpc")?;
                
                // Find proto files
                // For now, we'll just use a placeholder
                println!("Error: Proto file not specified in configuration");
                println!("Please specify a proto file using the --proto option or add it to the configuration file");
                return Err(ProxyError::config("Proto file not specified"));
            } else {
                return Err(ProxyError::config("gRPC configuration not found in config file"));
            }
        },
        "openapi" => {
            if let Some(openapi_config) = &config.openapi {
                let base_url = &openapi_config.base_url;
                println!("Generating OpenAPI capabilities for base URL: {}", base_url);
                
                // Create generator
                let mut generator = OpenAPICapabilityGenerator::new(base_url.clone());
                
                // Apply configuration
                if let Some(prefix) = &openapi_config.tool_prefix {
                    generator = generator.with_prefix(prefix.clone());
                } else if let Some(prefix) = &config.global.tool_prefix {
                    generator = generator.with_prefix(prefix.clone());
                }
                
                // Apply authentication
                if let Some(auth) = &openapi_config.auth {
                    // Convert common AuthConfig to OpenAPIAuthConfig
                    let openapi_auth = convert_to_openapi_auth(auth);
                    generator = generator.with_auth(openapi_auth);
                } else if let Some(auth) = &config.global.auth {
                    // Convert common AuthConfig to OpenAPIAuthConfig
                    let openapi_auth = convert_to_openapi_auth(auth);
                    generator = generator.with_auth(openapi_auth);
                }
                
                // Apply naming convention
                let naming_convention = match openapi_config.naming_convention.as_str() {
                    "operation-id" => NamingConvention::OperationId,
                    "method-path" => NamingConvention::MethodPath,
                    _ => NamingConvention::OperationId,
                };
                generator = generator.with_naming_convention(naming_convention);
                
                // Apply method filter
                if let Some(methods) = &openapi_config.methods {
                    generator = generator.with_method_filter(methods.clone());
                }
                
                // Apply deprecated flag
                if openapi_config.include_deprecated {
                    generator = generator.include_deprecated();
                }
                
                // Determine output path
                let output_path = determine_output_path(&config, "openapi")?;
                
                // Find OpenAPI spec files
                // For now, we'll just use a placeholder
                println!("Error: OpenAPI spec file not specified in configuration");
                println!("Please specify a spec file using the --spec option or add it to the configuration file");
                return Err(ProxyError::config("OpenAPI spec file not specified"));
            } else {
                return Err(ProxyError::config("OpenAPI configuration not found in config file"));
            }
        },
        _ => {
            return Err(ProxyError::config(format!("Unsupported generator type: {}", generator_type)));
        }
    }
    
    // This code is unreachable due to the previous return statements
    // but we'll keep it to maintain the function signature
    Ok(())
}

/// Determine output path from configuration
///
/// This function calculates the output file path based on the configuration settings.
/// It uses the output directory and file pattern from the configuration, creating
/// directories if they don't exist.
///
/// # Arguments
///
/// * `config` - The generator configuration file
/// * `generator_type` - The type of generator ("graphql", "grpc", or "openapi")
///
/// # Returns
///
/// A Result containing the output path or an error
///
/// # Example
///
/// ```
/// let output_path = determine_output_path(&config, "graphql")?;
/// ```
fn determine_output_path(config: &GeneratorConfigFile, generator_type: &str) -> Result<PathBuf> {
    // Check for output directory in configuration
    let output_dir = config.output.directory.as_ref()
        .or(config.global.output_dir.as_ref())
        .map(|dir| dir.clone())
        .unwrap_or_else(|| ".".to_string());
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(&output_dir)
        .map_err(|e| ProxyError::config(format!(
            "Failed to create output directory '{}': {}", 
            output_dir, 
            e
        )))?;
    
    // Generate file name
    let file_name = config.output.file_pattern
        .replace("{name}", generator_type)
        .replace("{ext}", match config.output.format.as_str() {
            "json" => "json",
            _ => "yaml",
        });
    
    Ok(Path::new(&output_dir).join(file_name))
}

/// Generate GraphQL capabilities from command-line arguments
///
/// This function processes command-line arguments for the GraphQL generator
/// and generates capability files from a GraphQL schema.
///
/// # Arguments
///
/// * `matches` - The command-line arguments for the GraphQL subcommand
///
/// # Returns
///
/// A Result indicating success or an error with details
///
/// # Example
///
/// ```
/// generate_graphql_from_args(sub_matches)?;
/// ```
fn generate_graphql_from_args(matches: &clap::ArgMatches) -> Result<()> {
    use magictunnel::registry::generator_common::CapabilityGeneratorBase;
    let schema_file = matches.get_one::<String>("schema").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();
    let endpoint = matches.get_one::<String>("endpoint").unwrap();
    
    // Read the schema file
    println!("Reading GraphQL schema from '{}'...", schema_file);
    let schema_content = fs::read_to_string(schema_file)
        .map_err(|e| ProxyError::config(format!("Failed to read schema file '{}': {}", schema_file, e)))?;
    
    // Handle format selection (enhanced is default)
    let use_enhanced = true; // Always use enhanced format
    let format_msg = if use_enhanced { "enhanced MCP 2025-06-18" } else { "legacy" };
    println!("Using {} format", format_msg);

    // Create generator adapter
    let mut adapter = GraphQLGeneratorAdapter::new(endpoint.clone());
    
    // Add prefix if specified
    if let Some(prefix) = matches.get_one::<String>("prefix") {
        adapter = adapter.with_prefix(prefix.clone());
    }
    
    // Configure authentication
    let auth_type = matches.get_one::<String>("auth-type").unwrap();
    if auth_type != "none" {
        let auth_token = matches.get_one::<String>("auth-token")
            .ok_or_else(|| ProxyError::config("Authentication token required for non-none auth types"))?;
        
        let auth_config = match auth_type.as_str() {
            "bearer" => GraphQLAuthConfig {
                auth_type: GraphQLAuthType::Bearer { token: auth_token.clone() },
                headers: HashMap::new(),
            },
            "apikey" => {
                let header = matches.get_one::<String>("auth-header").unwrap();
                GraphQLAuthConfig {
                    auth_type: GraphQLAuthType::ApiKey {
                        value: auth_token.clone(),
                        header: header.clone(),
                    },
                    headers: HashMap::new(),
                }
            }
            _ => return Err(ProxyError::config(format!("Unsupported auth type: {}", auth_type))),
        };
        
        // Convert GraphQLAuthConfig back to common AuthConfig for the adapter
        let common_auth = AuthConfig {
            auth_type: match &auth_config.auth_type {
                GraphQLAuthType::None => AuthType::None,
                GraphQLAuthType::Bearer { token } => AuthType::Bearer { token: token.clone() },
                GraphQLAuthType::ApiKey { value, header } => AuthType::ApiKey {
                    key: value.clone(),
                    header: header.clone(),
                },
                // GraphQL doesn't have other auth types, but we need to handle all cases
                _ => AuthType::None,
            },
            headers: auth_config.headers.clone(),
        };
        // Convert common AuthConfig to GraphQLAuthConfig before passing to adapter
        let graphql_auth = convert_to_graphql_auth(&common_auth);
        adapter = adapter.with_auth(graphql_auth);
    }
    
    // Generate capability file
    println!("Parsing GraphQL schema...");
    let capability_file = adapter.generate_from_content(&schema_content)
        .map_err(|e| ProxyError::config(format!("Failed to generate capability file from GraphQL schema: {:?}", e)))?;
    
    let tools_count = capability_file.enhanced_tools.as_ref().map(|t| t.len()).unwrap_or(0);
    
    println!("Generated {} {} tools from GraphQL schema", 
        tools_count, 
"enhanced");
    
    // Write output file
    write_capability_file(&capability_file, output_file)?;
    
    println!("Capability file written to '{}'", output_file);
    
    // Print summary
    println!("\nSummary:");
    println!("  Schema file: {}", schema_file);
    // Determine schema format
    let schema_format = if schema_content.trim_start().starts_with('{') {
        "JSON Introspection"
    } else {
        "GraphQL SDL"
    };
    println!("  Schema format: {}", schema_format);
    println!("  Endpoint: {}", endpoint);
    println!("  Output file: {}", output_file);
    if let Some(prefix) = matches.get_one::<String>("prefix") {
        println!("  Tool prefix: {}", prefix);
    }
    println!("  Auth type: {}", auth_type);
    println!("  Format: {}", format_msg);
    println!("  Tools generated: {}", tools_count);
    
    Ok(())
}

/// Generate gRPC capabilities from command-line arguments
///
/// This function processes command-line arguments for the gRPC generator
/// and generates capability files from a protobuf service definition.
///
/// # Arguments
///
/// * `matches` - The command-line arguments for the gRPC subcommand
///
/// # Returns
///
/// A Result indicating success or an error with details
///
/// # Example
///
/// ```
/// generate_grpc_from_args(sub_matches)?;
/// ```
fn generate_grpc_from_args(matches: &clap::ArgMatches) -> Result<()> {
    use magictunnel::registry::generator_common::CapabilityGeneratorBase;
    let proto_file = matches.get_one::<String>("proto").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();
    let endpoint = matches.get_one::<String>("endpoint").unwrap();
    
    // Read the protobuf file
    println!("Reading protobuf file '{}'...", proto_file);
    let proto_content = fs::read_to_string(proto_file)
        .map_err(|e| ProxyError::config(format!("Failed to read proto file '{}': {}", proto_file, e)))?;
    
    // Handle format selection (enhanced is default)
    let use_enhanced = true; // Always use enhanced format
    let format_msg = if use_enhanced { "enhanced MCP 2025-06-18" } else { "legacy" };
    println!("Using {} format", format_msg);

    // Create generator adapter
    let mut adapter = GrpcGeneratorAdapter::new(endpoint.clone());
    
    // Set tool prefix if provided
    if let Some(prefix) = matches.get_one::<String>("prefix") {
        adapter = adapter.with_prefix(prefix.clone());
    }
    
    // Set service filter if provided
    if let Some(services) = matches.get_one::<String>("service-filter") {
        let service_list: Vec<String> = services.split(',')
            .map(|s| s.trim().to_string())
            .collect();
        adapter = adapter.with_service_filter(service_list);
    }
    
    // Set method filter if provided
    if let Some(methods) = matches.get_one::<String>("method-filter") {
        let method_list: Vec<String> = methods.split(',')
            .map(|m| m.trim().to_string())
            .collect();
        adapter = adapter.with_method_filter(method_list);
    }
    
    // Parse streaming strategies
    let server_streaming = parse_streaming_strategy(
        matches.get_one::<String>("server-streaming").unwrap()
    )?;
    adapter = adapter.with_server_streaming_strategy(server_streaming);
    
    let client_streaming = parse_streaming_strategy(
        matches.get_one::<String>("client-streaming").unwrap()
    )?;
    adapter = adapter.with_client_streaming_strategy(client_streaming);
    
    let bidirectional_streaming = parse_streaming_strategy(
        matches.get_one::<String>("bidirectional-streaming").unwrap()
    )?;
    adapter = adapter.with_bidirectional_streaming_strategy(bidirectional_streaming);
    
    // Set method options and separate streaming tools
    if matches.get_flag("include-method-options") {
        adapter = adapter.with_include_method_options(true);
    }
    
    if matches.get_flag("separate-streaming-tools") {
        adapter = adapter.with_separate_streaming_tools(true);
    }
    
    // Set up authentication
    let auth_type = matches.get_one::<String>("auth-type").unwrap();
    if auth_type != "none" {
        let auth_config = match auth_type.as_str() {
            "bearer" => {
                let token = matches.get_one::<String>("auth-token")
                    .ok_or_else(|| ProxyError::config("Bearer authentication requires --auth-token"))?;
                GrpcAuthConfig {
                    auth_type: GrpcAuthType::Bearer { token: token.clone() },
                    headers: HashMap::new(),
                }
            }
            "apikey" => {
                let token = matches.get_one::<String>("auth-token")
                    .ok_or_else(|| ProxyError::config("API key authentication requires --auth-token"))?;
                let header = matches.get_one::<String>("auth-header").unwrap();
                GrpcAuthConfig {
                    auth_type: GrpcAuthType::ApiKey {
                        key: token.clone(),
                        header: header.clone(),
                    },
                    headers: HashMap::new(),
                }
            }
            "basic" => {
                let username = matches.get_one::<String>("auth-username")
                    .ok_or_else(|| ProxyError::config("Basic authentication requires --auth-username"))?;
                let password = matches.get_one::<String>("auth-password")
                    .ok_or_else(|| ProxyError::config("Basic authentication requires --auth-password"))?;
                GrpcAuthConfig {
                    auth_type: GrpcAuthType::Basic {
                        username: username.clone(),
                        password: password.clone(),
                    },
                    headers: HashMap::new(),
                }
            }
            "oauth" => {
                let token = matches.get_one::<String>("auth-token")
                    .ok_or_else(|| ProxyError::config("OAuth authentication requires --auth-token"))?;
                let token_type = matches.get_one::<String>("auth-token-type").unwrap();
                GrpcAuthConfig {
                    auth_type: GrpcAuthType::OAuth {
                        token: token.clone(),
                        token_type: token_type.clone(),
                    },
                    headers: HashMap::new(),
                }
            }
            _ => return Err(ProxyError::config(format!("Invalid auth type: {}. Use 'none', 'bearer', 'apikey', 'basic', or 'oauth'", auth_type))),
        };
        
        // Convert GrpcAuthConfig back to common AuthConfig for the adapter
        let common_auth = AuthConfig {
            auth_type: match &auth_config.auth_type {
                GrpcAuthType::None => AuthType::None,
                GrpcAuthType::Bearer { token } => AuthType::Bearer { token: token.clone() },
                GrpcAuthType::ApiKey { key, header } => AuthType::ApiKey {
                    key: key.clone(),
                    header: header.clone(),
                },
                GrpcAuthType::Basic { username, password } => AuthType::Basic {
                    username: username.clone(),
                    password: password.clone(),
                },
                GrpcAuthType::OAuth { token, token_type } => AuthType::OAuth {
                    token: token.clone(),
                    token_type: token_type.clone(),
                },
            },
            headers: auth_config.headers.clone(),
        };
        adapter = adapter.with_auth(common_auth);
    }
    
    // Generate capability file
    println!("Generating capability file from protobuf...");
    let capability_file = adapter.generate_from_content(&proto_content)
        .map_err(|e| ProxyError::config(format!("Failed to generate capability file from protobuf: {:?}", e)))?;
    
    let tools_count = capability_file.enhanced_tools.as_ref().map(|t| t.len()).unwrap_or(0);
    
    println!("Generated {} {} tools from protobuf service definitions", 
        tools_count, 
"enhanced");
    
    // Write output file
    write_capability_file(&capability_file, output_file)?;
    
    println!("Capability file written to '{}'", output_file);
    
    // Print summary
    println!("\nGenerated tools:");
    if let Some(enhanced_tools) = &capability_file.enhanced_tools {
        for tool in enhanced_tools {
            println!("  - {}: {}", tool.name, tool.core.description);
        }
    }
    
    Ok(())
}

/// Generate OpenAPI capabilities from command-line arguments
///
/// This function processes command-line arguments for the OpenAPI generator
/// and generates capability files from an OpenAPI specification.
///
/// # Arguments
///
/// * `matches` - The command-line arguments for the OpenAPI subcommand
///
/// # Returns
///
/// A Result indicating success or an error with details
///
/// # Example
///
/// ```
/// generate_openapi_from_args(sub_matches)?;
/// ```
fn generate_openapi_from_args(matches: &clap::ArgMatches) -> Result<()> {
    use magictunnel::registry::generator_common::CapabilityGeneratorBase;
    let spec_file = matches.get_one::<String>("spec").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();
    let base_url = matches.get_one::<String>("base-url").unwrap();
    
    // Read the OpenAPI specification
    let spec_content = fs::read_to_string(spec_file)
        .map_err(|e| ProxyError::config(format!("Failed to read spec file '{}': {}", spec_file, e)))?;
    
    println!("Parsing OpenAPI specification from '{}'...", spec_file);
    
    // Handle format selection (enhanced is default)
    let use_enhanced = true; // Always use enhanced format
    let format_msg = if use_enhanced { "enhanced MCP 2025-06-18" } else { "legacy" };
    println!("Using {} format", format_msg);

    // Create generator adapter
    let mut adapter = OpenAPIGeneratorAdapter::new(base_url.clone());
    
    // Set prefix if provided
    if let Some(prefix) = matches.get_one::<String>("prefix") {
        adapter = adapter.with_prefix(prefix.clone());
    }
    
    // Set naming convention
    let naming = matches.get_one::<String>("naming").unwrap();
    let naming_convention = match naming.as_str() {
        "operation-id" => NamingConvention::OperationId,
        "method-path" => NamingConvention::MethodPath,
        _ => return Err(ProxyError::config(format!("Invalid naming convention: {}. Use 'operation-id' or 'method-path'", naming))),
    };
    adapter = adapter.with_naming_convention(naming_convention);
    
    // Set method filter if provided
    if let Some(methods) = matches.get_one::<String>("methods") {
        let method_list: Vec<String> = methods.split(',')
            .map(|m| m.trim().to_uppercase())
            .collect();
        adapter = adapter.with_method_filter(method_list);
    }
    
    // Include deprecated if requested
    if matches.get_flag("include-deprecated") {
        adapter = adapter.with_include_deprecated(true);
    }
    
    // Set up authentication
    let auth_type = matches.get_one::<String>("auth-type").unwrap();
    if auth_type != "none" {
        let auth_config = match auth_type.as_str() {
            "bearer" => {
                let token = matches.get_one::<String>("auth-token")
                    .ok_or_else(|| ProxyError::config("Bearer authentication requires --auth-token"))?;
                OpenAPIAuthConfig {
                    auth_type: OpenAPIAuthType::Bearer { token: token.clone() },
                    headers: HashMap::new(),
                }
            }
            "apikey" => {
                let token = matches.get_one::<String>("auth-token")
                    .ok_or_else(|| ProxyError::config("API key authentication requires --auth-token"))?;
                let header = matches.get_one::<String>("auth-header").unwrap();
                OpenAPIAuthConfig {
                    auth_type: OpenAPIAuthType::ApiKey {
                        key: token.clone(),
                        header: header.clone(),
                    },
                    headers: HashMap::new(),
                }
            }
            "basic" => {
                let username = matches.get_one::<String>("auth-username")
                    .ok_or_else(|| ProxyError::config("Basic authentication requires --auth-username"))?;
                let password = matches.get_one::<String>("auth-password")
                    .ok_or_else(|| ProxyError::config("Basic authentication requires --auth-password"))?;
                OpenAPIAuthConfig {
                    auth_type: OpenAPIAuthType::Basic {
                        username: username.clone(),
                        password: password.clone(),
                    },
                    headers: HashMap::new(),
                }
            }
            _ => return Err(ProxyError::config(format!("Invalid auth type: {}. Use 'none', 'bearer', 'apikey', or 'basic'", auth_type))),
        };
        
        // Convert OpenAPIAuthConfig back to common AuthConfig for the adapter
        let common_auth = AuthConfig {
            auth_type: match &auth_config.auth_type {
                OpenAPIAuthType::None => AuthType::None,
                OpenAPIAuthType::Bearer { token } => AuthType::Bearer { token: token.clone() },
                OpenAPIAuthType::ApiKey { key, header } => AuthType::ApiKey {
                    key: key.clone(),
                    header: header.clone(),
                },
                OpenAPIAuthType::Basic { username, password } => AuthType::Basic {
                    username: username.clone(),
                    password: password.clone(),
                },
                // OpenAPI doesn't have OAuth directly, but we need to handle it for completeness
                _ => AuthType::None,
            },
            headers: auth_config.headers.clone(),
        };
        // Convert common AuthConfig to OpenAPIAuthConfig before passing to adapter
        let openapi_auth = convert_to_openapi_auth(&common_auth);
        adapter = adapter.with_auth(openapi_auth);
    }
    
    // Generate capability file
    let capability_file = adapter.generate_from_content(&spec_content)
        .map_err(|e| ProxyError::config(format!("Failed to generate capability file from OpenAPI spec: {:?}", e)))?;
    
    let tools_count = capability_file.enhanced_tools.as_ref().map(|t| t.len()).unwrap_or(0);
    
    println!("Generated {} {} tools from OpenAPI specification", 
        tools_count, 
"enhanced");
    
    // Write output file
    write_capability_file(&capability_file, output_file)?;
    
    println!("Capability file written to '{}'", output_file);
    
    // Print summary
    println!("\nGenerated tools:");
    if let Some(enhanced_tools) = &capability_file.enhanced_tools {
        for tool in enhanced_tools {
            println!("  - {}: {}", tool.name, tool.core.description);
        }
    }
    
    Ok(())
}


/// Merge capability files
///
/// This function merges multiple capability files into a single file,
/// handling duplicate tool names according to the specified strategy.
///
/// # Arguments
///
/// * `matches` - The command-line arguments for the merge subcommand
///
/// # Returns
///
/// A Result indicating success or an error with details
///
/// # Example
///
/// ```
/// merge_capability_files(sub_matches)?;
/// ```
fn merge_capability_files(matches: &clap::ArgMatches) -> Result<()> {
    let input_files_str = matches.get_one::<String>("input").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();
    let strategy_str = matches.get_one::<String>("strategy").unwrap();
    
    // Handle format selection (enhanced is default)
    let use_enhanced = true; // Always use enhanced format
    let format_msg = if use_enhanced { "enhanced MCP 2025-06-18" } else { "legacy" };
    println!("Output format: {}", format_msg);
    
    // Parse merge strategy
    let strategy = match strategy_str.to_lowercase().as_str() {
        "keep-first" => MergeStrategy::KeepFirst,
        "keep-last" => MergeStrategy::KeepLast,
        "rename" => MergeStrategy::Rename,
        "error" => MergeStrategy::Error,
        _ => return Err(ProxyError::config(format!(
            "Invalid merge strategy: {}. Use 'keep-first', 'keep-last', 'rename', or 'error'",
            strategy_str
        ))),
    };
    
    // Split input files
    let input_files: Vec<&str> = input_files_str.split(',')
        .map(|s| s.trim())
        .collect();
    
    if input_files.is_empty() {
        return Err(ProxyError::config("No input files specified"));
    }
    
    println!("Merging {} capability files...", input_files.len());
    
    // Load all input files
    let mut capability_files = Vec::new();
    for file_path in &input_files {
        println!("Reading '{}'...", file_path);
        let content = read_file_content(file_path)?;
        
        let file: CapabilityFile = serde_yaml::from_str(&content)
            .map_err(|e| ProxyError::config(format!(
                "Failed to parse capability file '{}': {}", file_path, e
            )))?;
        
        capability_files.push(file);
    }
    
    // Create merger and merge files
    let merger = CapabilityMerger::new();
    let merged_file = merger.merge(capability_files, strategy)?;
    
    let tools_count = merged_file.enhanced_tools.as_ref().map(|t| t.len()).unwrap_or(0);
    
    println!("Successfully merged {} {} tools from {} files",
             tools_count,
             "enhanced",
             input_files.len());
    
    // Write output file
    write_capability_file(&merged_file, output_file)?;
    
    println!("Merged capability file written to '{}'", output_file);
    
    // Print summary
    println!("\nSummary:");
    println!("  Input files: {}", input_files_str);
    println!("  Output file: {}", output_file);
    println!("  Merge strategy: {}", strategy_str);
    println!("  Output format: {}", format_msg);
    println!("  Total tools: {}", tools_count);
    
    Ok(())
}

/// Validate capability files
///
/// This function validates one or more capability files against a set of rules,
/// checking for issues like duplicate tool names, empty descriptions, etc.
///
/// # Arguments
///
/// * `matches` - The command-line arguments for the validate subcommand
///
/// # Returns
///
/// A Result indicating success or an error with details
///
/// # Example
///
/// ```
/// validate_capability_files(sub_matches)?;
/// ```
fn validate_capability_files(matches: &clap::ArgMatches) -> Result<()> {
    let input_files_str = matches.get_one::<String>("input").unwrap();
    let strict_mode = matches.get_flag("strict");
    
    // Split input files
    let input_files: Vec<&str> = input_files_str.split(',')
        .map(|s| s.trim())
        .collect();
    
    if input_files.is_empty() {
        return Err(ProxyError::config("No input files specified"));
    }
    
    println!("Validating {} capability files{}...",
             input_files.len(),
             if strict_mode { " (strict mode)" } else { "" });
    
    // Create validator
    let validator = CapabilityValidator::new();
    
    // Load and validate each file
    let mut all_valid = true;
    let mut all_issues = Vec::new();
    
    for file_path in &input_files {
        println!("Validating '{}'...", file_path);
        let content = read_file_content(file_path)?;
        
        let file: CapabilityFile = serde_yaml::from_str(&content)
            .map_err(|e| ProxyError::config(format!(
                "Failed to parse capability file '{}': {}", file_path, e
            )))?;
        
        // Get validation issues
        let issues = validator.get_validation_issues(&file);
        
        if issues.is_empty() {
            println!("  ✓ File is valid");
        } else {
            all_valid = false;
            println!("  ✗ File has {} validation issues:", issues.len());
            for issue in &issues {
                println!("    - {}", issue);
                all_issues.push(format!("{}: {}", file_path, issue));
            }
        }
    }
    
    // Print summary
    println!("\nValidation Summary:");
    println!("  Files validated: {}", input_files.len());
    println!("  Valid files: {}", input_files.len() - (if all_valid { 0 } else { 1 }));
    println!("  Total issues: {}", all_issues.len());
    
    // In strict mode, fail if any issues were found
    if strict_mode && !all_valid {
        return Err(ProxyError::validation(format!(
            "Validation failed with {} issues", all_issues.len()
        )));
    }
    
    Ok(())
}


/// Parse streaming strategy from string
///
/// This function converts a string representation of a streaming strategy
/// to the corresponding StreamingStrategy enum value.
///
/// # Arguments
///
/// * `strategy` - The streaming strategy as a string ("polling", "pagination", or "agent-level")
///
/// # Returns
///
/// A Result containing the StreamingStrategy enum value or an error
///
/// # Example
///
/// ```
/// let strategy = parse_streaming_strategy("polling")?;
/// ```
fn parse_streaming_strategy(strategy: &str) -> Result<StreamingStrategy> {
    match strategy.to_lowercase().as_str() {
        "polling" => Ok(StreamingStrategy::Polling),
        "pagination" => Ok(StreamingStrategy::Pagination),
        "agent-level" | "agentlevel" => Ok(StreamingStrategy::AgentLevel),
        _ => Err(ProxyError::config(format!("Invalid streaming strategy: {}. Use 'polling', 'pagination', or 'agent-level'", strategy))),
    }
}

/// Initialize a new configuration file
///
/// This function creates a new configuration file with example settings
/// for all generator types. Creates YAML format by default for consistency
/// with the main MCP Proxy configuration.
///
/// # Arguments
///
/// * `output_file` - The path where the configuration file should be created
///
/// # Returns
///
/// A Result indicating success or an error with details
///
/// # Example
///
/// ```
/// initialize_config_file("config.yaml")?;
/// initialize_config_file("config.toml")?; // Still supported
/// ```
fn initialize_config_file(output_file: &str) -> Result<()> {
    println!("Initializing new configuration file at '{}'...", output_file);

    // Generate YAML configuration
    let config_content = example_config_yaml();

    // Write to file
    fs::write(output_file, config_content)
        .map_err(|e| ProxyError::config(format!("Failed to write config file '{}': {}", output_file, e)))?;
    
    println!("Configuration file created at '{}'", output_file);
    println!("Edit this file to configure your generators, then run:");
    println!("  magictunnel-cli graphql --config {}", output_file);
    println!("  magictunnel-cli grpc --config {}", output_file);
    println!("  magictunnel-cli openapi --config {}", output_file);
    
    Ok(())
}

// CLI Management Command Handlers

async fn handle_resources_command(matches: &ArgMatches, server_url: &str) -> Result<()> {
    let client = Client::new();
    
    match matches.subcommand() {
        Some(("list", _)) => {
            println!("📚 Fetching MCP resources from {}...", server_url);
            
            let response = client
                .get(&format!("{}/dashboard/api/resources", server_url))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to fetch resources: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            if let Some(resources) = data.get("resources").and_then(|r| r.as_array()) {
                if resources.is_empty() {
                    println!("ℹ️  No MCP resources found");
                } else {
                    println!("✅ Found {} MCP resource(s):", resources.len());
                    for resource in resources {
                        if let (Some(uri), Some(name)) = (resource.get("uri"), resource.get("name")) {
                            println!("  📄 {} ({})", name.as_str().unwrap_or("Unknown"), uri.as_str().unwrap_or("Unknown"));
                            if let Some(description) = resource.get("description") {
                                println!("     {}", description.as_str().unwrap_or(""));
                            }
                        }
                    }
                }
            } else {
                println!("⚠️  Invalid response format");
            }
        },
        Some(("read", sub_matches)) => {
            let uri = sub_matches.get_one::<String>("uri").unwrap();
            println!("📖 Reading resource content for: {}", uri);
            
            let response = client
                .post(&format!("{}/dashboard/api/resources/read", server_url))
                .json(&json!({ "uri": uri }))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to read resource: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            if let Some(content) = data.get("contents").and_then(|c| c.as_array()).and_then(|arr| arr.first()) {
                if let Some(text) = content.get("text") {
                    println!("📄 Content:\n{}", text.as_str().unwrap_or(""));
                } else if let Some(blob) = content.get("blob") {
                    println!("🔢 Binary content (base64): {}", blob.as_str().unwrap_or(""));
                }
            } else {
                println!("⚠️  No content found or invalid format");
            }
        },
        Some(("export", sub_matches)) => {
            let uri = sub_matches.get_one::<String>("uri").unwrap();
            let output_file = sub_matches.get_one::<String>("output").unwrap();
            
            println!("💾 Exporting resource {} to {}...", uri, output_file);
            
            let response = client
                .post(&format!("{}/dashboard/api/resources/read", server_url))
                .json(&json!({ "uri": uri }))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to read resource: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            if let Some(content) = data.get("contents").and_then(|c| c.as_array()).and_then(|arr| arr.first()) {
                if let Some(text) = content.get("text") {
                    fs::write(output_file, text.as_str().unwrap_or(""))
                        .map_err(|e| ProxyError::config(format!("Failed to write file: {}", e)))?;
                    println!("✅ Text content exported to {}", output_file);
                } else if let Some(blob) = content.get("blob") {
                    // Decode base64 and write as binary
                    let decoded = general_purpose::STANDARD.decode(blob.as_str().unwrap_or(""))
                        .map_err(|e| ProxyError::config(format!("Failed to decode base64: {}", e)))?;
                    fs::write(output_file, decoded)
                        .map_err(|e| ProxyError::config(format!("Failed to write file: {}", e)))?;
                    println!("✅ Binary content exported to {}", output_file);
                }
            } else {
                return Err(ProxyError::config("No content found".to_string()));
            }
        },
        _ => {
            println!("❌ Unknown resources subcommand");
        }
    }
    
    Ok(())
}

async fn handle_prompts_command(matches: &ArgMatches, server_url: &str) -> Result<()> {
    let client = Client::new();
    
    match matches.subcommand() {
        Some(("list", _)) => {
            println!("💬 Fetching MCP prompts from {}...", server_url);
            
            let response = client
                .get(&format!("{}/dashboard/api/prompts", server_url))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to fetch prompts: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            if let Some(prompts) = data.get("prompts").and_then(|p| p.as_array()) {
                if prompts.is_empty() {
                    println!("ℹ️  No MCP prompts found");
                } else {
                    println!("✅ Found {} MCP prompt(s):", prompts.len());
                    for prompt in prompts {
                        if let Some(name) = prompt.get("name") {
                            print!("  💬 {}", name.as_str().unwrap_or("Unknown"));
                            if let Some(description) = prompt.get("description") {
                                print!(" - {}", description.as_str().unwrap_or(""));
                            }
                            println!();
                            if let Some(args) = prompt.get("arguments").and_then(|a| a.as_array()) {
                                for arg in args {
                                    if let Some(arg_name) = arg.get("name") {
                                        let required = arg.get("required").and_then(|r| r.as_bool()).unwrap_or(false);
                                        let req_str = if required { " (required)" } else { "" };
                                        println!("     📝 {}{}", arg_name.as_str().unwrap_or(""), req_str);
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                println!("⚠️  Invalid response format");
            }
        },
        Some(("execute", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            let args_json = sub_matches.get_one::<String>("arguments");
            
            println!("🚀 Executing prompt: {}", name);
            
            let mut request_body = json!({ "name": name });
            
            if let Some(args_str) = args_json {
                let arguments: Value = serde_json::from_str(args_str)
                    .map_err(|e| ProxyError::config(format!("Invalid JSON arguments: {}", e)))?;
                request_body["arguments"] = arguments;
            }
            
            let response = client
                .post(&format!("{}/dashboard/api/prompts/execute", server_url))
                .json(&request_body)
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to execute prompt: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            if let Some(response_data) = data.get("response") {
                if let Some(description) = response_data.get("description") {
                    println!("📝 Description: {}", description.as_str().unwrap_or(""));
                }
                
                if let Some(messages) = response_data.get("messages").and_then(|m| m.as_array()) {
                    println!("💬 Generated {} message(s):", messages.len());
                    for (i, message) in messages.iter().enumerate() {
                        if let (Some(role), Some(content)) = (message.get("role"), message.get("content")) {
                            println!("  Message {} ({}):", i + 1, role.as_str().unwrap_or("unknown"));
                            if let Some(text) = content.get("text") {
                                println!("    {}", text.as_str().unwrap_or(""));
                            }
                        }
                    }
                } else {
                    println!("⚠️  No messages in response");
                }
            } else {
                println!("⚠️  Invalid response format");
            }
        },
        Some(("export", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            let output_file = sub_matches.get_one::<String>("output").unwrap();
            let args_json = sub_matches.get_one::<String>("arguments");
            
            println!("💾 Exporting prompt execution {} to {}...", name, output_file);
            
            let mut request_body = json!({ "name": name });
            
            if let Some(args_str) = args_json {
                let arguments: Value = serde_json::from_str(args_str)
                    .map_err(|e| ProxyError::config(format!("Invalid JSON arguments: {}", e)))?;
                request_body["arguments"] = arguments;
            }
            
            let response = client
                .post(&format!("{}/dashboard/api/prompts/execute", server_url))
                .json(&request_body)
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to execute prompt: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            // Export the full response as JSON
            let export_content = serde_json::to_string_pretty(&data)
                .map_err(|e| ProxyError::config(format!("Failed to serialize response: {}", e)))?;
            
            fs::write(output_file, export_content)
                .map_err(|e| ProxyError::config(format!("Failed to write file: {}", e)))?;
            
            println!("✅ Prompt execution result exported to {}", output_file);
        },
        _ => {
            println!("❌ Unknown prompts subcommand");
        }
    }
    
    Ok(())
}

async fn handle_tools_command(matches: &ArgMatches, server_url: &str) -> Result<()> {
    let client = Client::new();
    
    match matches.subcommand() {
        Some(("list", _)) => {
            println!("🔧 Fetching tools from {}...", server_url);
            
            let response = client
                .get(&format!("{}/dashboard/api/tools", server_url))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to fetch tools: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            if let Some(tools) = data.get("tools").and_then(|t| t.as_array()) {
                if tools.is_empty() {
                    println!("ℹ️  No tools found");
                } else {
                    println!("✅ Found {} tool(s):", tools.len());
                    for tool in tools {
                        if let Some(name) = tool.get("name") {
                            print!("  🔧 {}", name.as_str().unwrap_or("Unknown"));
                            if let Some(description) = tool.get("description") {
                                print!(" - {}", description.as_str().unwrap_or(""));
                            }
                            println!();
                        }
                    }
                }
            } else {
                println!("⚠️  Invalid response format");
            }
        },
        Some(("execute", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            let args_json = sub_matches.get_one::<String>("arguments");
            
            println!("🚀 Executing tool: {}", name);
            
            let mut request_body = json!({ "name": name });
            
            if let Some(args_str) = args_json {
                let arguments: Value = serde_json::from_str(args_str)
                    .map_err(|e| ProxyError::config(format!("Invalid JSON arguments: {}", e)))?;
                request_body["arguments"] = arguments;
            } else {
                request_body["arguments"] = json!({});
            }
            
            let response = client
                .post(&format!("{}/v1/mcp/call", server_url))
                .json(&request_body)
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to execute tool: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            println!("✅ Tool execution result:");
            println!("{}", serde_json::to_string_pretty(&data).unwrap_or_else(|_| "Invalid JSON".to_string()));
        },
        Some(("info", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            
            println!("ℹ️  Getting tool info: {}", name);
            
            let response = client
                .get(&format!("{}/dashboard/api/tools", server_url))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to fetch tools: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            if let Some(tools) = data.get("tools").and_then(|t| t.as_array()) {
                if let Some(tool) = tools.iter().find(|t| t.get("name").and_then(|n| n.as_str()) == Some(name)) {
                    println!("🔧 Tool Information:");
                    println!("  Name: {}", tool.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown"));
                    if let Some(description) = tool.get("description") {
                        println!("  Description: {}", description.as_str().unwrap_or(""));
                    }
                    if let Some(schema) = tool.get("inputSchema") {
                        println!("  Input Schema:");
                        println!("{}", serde_json::to_string_pretty(schema).unwrap_or_else(|_| "Invalid JSON".to_string()));
                    }
                } else {
                    println!("❌ Tool '{}' not found", name);
                }
            } else {
                println!("⚠️  Invalid response format");
            }
        },
        _ => {
            println!("❌ Unknown tools subcommand");
        }
    }
    
    Ok(())
}

async fn handle_services_command(matches: &ArgMatches, server_url: &str) -> Result<()> {
    let client = Client::new();
    
    match matches.subcommand() {
        Some(("list", _)) => {
            println!("🛠️  Fetching services from {}...", server_url);
            
            let response = client
                .get(&format!("{}/dashboard/api/services", server_url))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to fetch services: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            if let Some(services) = data.get("services").and_then(|s| s.as_array()) {
                if services.is_empty() {
                    println!("ℹ️  No services found");
                } else {
                    println!("✅ Found {} service(s):", services.len());
                    for service in services {
                        if let Some(name) = service.get("name") {
                            let status = service.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
                            let status_emoji = match status {
                                "running" => "🟢",
                                "stopped" => "🔴",
                                "error" => "❌",
                                _ => "⚪",
                            };
                            println!("  {} {} ({})", status_emoji, name.as_str().unwrap_or("Unknown"), status);
                        }
                    }
                }
            } else {
                println!("⚠️  Invalid response format");
            }
        },
        Some(("restart", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            
            println!("🔄 Restarting service: {}", name);
            
            let response = client
                .post(&format!("{}/dashboard/api/services/{}/restart", server_url, name))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to restart service: {}", e)))?;
            
            if response.status().is_success() {
                println!("✅ Service '{}' restarted successfully", name);
            } else {
                return Err(ProxyError::connection(format!("Failed to restart service: {}", response.status())));
            }
        },
        Some(("start", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            
            println!("▶️  Starting service: {}", name);
            
            let response = client
                .post(&format!("{}/dashboard/api/services/{}/start", server_url, name))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to start service: {}", e)))?;
            
            if response.status().is_success() {
                println!("✅ Service '{}' started successfully", name);
            } else {
                return Err(ProxyError::connection(format!("Failed to start service: {}", response.status())));
            }
        },
        Some(("stop", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            
            println!("⏹️  Stopping service: {}", name);
            
            let response = client
                .post(&format!("{}/dashboard/api/services/{}/stop", server_url, name))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to stop service: {}", e)))?;
            
            if response.status().is_success() {
                println!("✅ Service '{}' stopped successfully", name);
            } else {
                return Err(ProxyError::connection(format!("Failed to stop service: {}", response.status())));
            }
        },
        _ => {
            println!("❌ Unknown services subcommand");
        }
    }
    
    Ok(())
}

async fn handle_server_command(matches: &ArgMatches, server_url: &str) -> Result<()> {
    let client = Client::new();
    
    match matches.subcommand() {
        Some(("status", _)) => {
            println!("🩺 Checking server status at {}...", server_url);
            
            let response = client
                .get(&format!("{}/dashboard/api/status", server_url))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to get server status: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ProxyError::connection(format!("Server returned status: {}", response.status())));
            }
            
            let data: Value = response.json().await
                .map_err(|e| ProxyError::connection(format!("Failed to parse response: {}", e)))?;
            
            println!("✅ Server Status:");
            println!("{}", serde_json::to_string_pretty(&data).unwrap_or_else(|_| "Invalid JSON".to_string()));
        },
        Some(("restart", _)) => {
            println!("🔄 Restarting server at {}...", server_url);
            
            let response = client
                .post(&format!("{}/dashboard/api/restart", server_url))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to restart server: {}", e)))?;
            
            if response.status().is_success() {
                println!("✅ Server restart initiated");
            } else {
                return Err(ProxyError::connection(format!("Failed to restart server: {}", response.status())));
            }
        },
        Some(("health", _)) => {
            println!("💓 Checking server health at {}...", server_url);
            
            let response = client
                .get(&format!("{}/health", server_url))
                .send()
                .await
                .map_err(|e| ProxyError::connection(format!("Failed to check health: {}", e)))?;
            
            if response.status().is_success() {
                let status_text = response.text().await.unwrap_or_else(|_| "Unknown".to_string());
                println!("✅ Server is healthy: {}", status_text);
            } else {
                println!("❌ Server health check failed: {}", response.status());
            }
        },
        _ => {
            println!("❌ Unknown server subcommand");
        }
    }
    
    Ok(())
}