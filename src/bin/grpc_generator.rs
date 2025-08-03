//! gRPC Capability Generator CLI
//! 
//! Command-line tool for generating MCP capability files from gRPC/protobuf service definitions.

use clap::{Arg, Command};
use magictunnel::registry::grpc_generator::{GrpcCapabilityGenerator, GrpcGeneratorConfig, AuthConfig, AuthType, StreamingStrategy};
use std::collections::HashMap;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("grpc-generator")
        .about("Generate MCP capability files from gRPC/protobuf service definitions")
        .version("1.0.0")
        .arg(
            Arg::new("proto")
                .short('p')
                .long("proto")
                .value_name("FILE")
                .help("Protobuf (.proto) file containing service definitions")
                .required(true)
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output capability file (YAML)")
                .required(true)
        )
        .arg(
            Arg::new("endpoint")
                .short('e')
                .long("endpoint")
                .value_name("ENDPOINT")
                .help("gRPC service endpoint (e.g., localhost:50051)")
                .required(true)
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
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("separate-streaming-tools")
                .long("separate-streaming-tools")
                .help("Generate separate tools for streaming methods")
                .action(clap::ArgAction::SetTrue)
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
        .get_matches();

    let proto_file = matches.get_one::<String>("proto").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();
    let endpoint = matches.get_one::<String>("endpoint").unwrap();

    // Read the protobuf file
    println!("Reading protobuf file '{}'...", proto_file);
    let proto_content = fs::read_to_string(proto_file)
        .map_err(|e| format!("Failed to read proto file '{}': {}", proto_file, e))?;

    // Parse streaming strategies
    let server_streaming = parse_streaming_strategy(
        matches.get_one::<String>("server-streaming").unwrap()
    )?;
    
    let client_streaming = parse_streaming_strategy(
        matches.get_one::<String>("client-streaming").unwrap()
    )?;
    
    let bidirectional_streaming = parse_streaming_strategy(
        matches.get_one::<String>("bidirectional-streaming").unwrap()
    )?;

    // Create generator config
    let mut config = GrpcGeneratorConfig {
        endpoint: endpoint.clone(),
        auth_config: None,
        tool_prefix: None,
        service_filter: None,
        method_filter: None,
        server_streaming_strategy: server_streaming,
        client_streaming_strategy: client_streaming,
        bidirectional_streaming_strategy: bidirectional_streaming,
        include_method_options: matches.get_flag("include-method-options"),
        separate_streaming_tools: matches.get_flag("separate-streaming-tools"),
        use_enhanced_format: true, // Always use enhanced format
    };

    // Set tool prefix if provided
    if let Some(prefix) = matches.get_one::<String>("prefix") {
        config.tool_prefix = Some(prefix.clone());
    }

    // Set service filter if provided
    if let Some(services) = matches.get_one::<String>("service-filter") {
        let service_list: Vec<String> = services.split(',')
            .map(|s| s.trim().to_string())
            .collect();
        config.service_filter = Some(service_list);
    }

    // Set method filter if provided
    if let Some(methods) = matches.get_one::<String>("method-filter") {
        let method_list: Vec<String> = methods.split(',')
            .map(|m| m.trim().to_string())
            .collect();
        config.method_filter = Some(method_list);
    }

    // Set up authentication
    let auth_type = matches.get_one::<String>("auth-type").unwrap();
    let auth_config = match auth_type.as_str() {
        "none" => None,
        "bearer" => {
            let token = matches.get_one::<String>("auth-token")
                .ok_or("Bearer authentication requires --auth-token")?;
            Some(AuthConfig {
                auth_type: AuthType::Bearer { token: token.clone() },
                headers: HashMap::new(),
            })
        }
        "apikey" => {
            let token = matches.get_one::<String>("auth-token")
                .ok_or("API key authentication requires --auth-token")?;
            let header = matches.get_one::<String>("auth-header").unwrap();
            Some(AuthConfig {
                auth_type: AuthType::ApiKey {
                    key: token.clone(),
                    header: header.clone(),
                },
                headers: HashMap::new(),
            })
        }
        "basic" => {
            let username = matches.get_one::<String>("auth-username")
                .ok_or("Basic authentication requires --auth-username")?;
            let password = matches.get_one::<String>("auth-password")
                .ok_or("Basic authentication requires --auth-password")?;
            Some(AuthConfig {
                auth_type: AuthType::Basic {
                    username: username.clone(),
                    password: password.clone(),
                },
                headers: HashMap::new(),
            })
        }
        "oauth" => {
            let token = matches.get_one::<String>("auth-token")
                .ok_or("OAuth authentication requires --auth-token")?;
            let token_type = matches.get_one::<String>("auth-token-type").unwrap();
            Some(AuthConfig {
                auth_type: AuthType::OAuth {
                    token: token.clone(),
                    token_type: token_type.clone(),
                },
                headers: HashMap::new(),
            })
        }
        _ => return Err(format!("Invalid auth type: {}. Use 'none', 'bearer', 'apikey', 'basic', or 'oauth'", auth_type).into()),
    };

    if let Some(auth) = auth_config {
        config.auth_config = Some(auth);
    }

    // Always using enhanced format
    println!("Using enhanced MCP 2025-06-18 format");

    // Create generator
    let generator = GrpcCapabilityGenerator::new(config);

    // Generate capability file
    println!("Generating capability file from protobuf...");
    let capability_file = generator.generate_from_proto_content(&proto_content)
        .map_err(|e| format!("Failed to generate capability file from protobuf: {:?}", e))?;

    let tools_count = capability_file.enhanced_tools.as_ref().map(|t| t.len()).unwrap_or(0);
    
    println!("Generated {} enhanced tools from protobuf service definitions", tools_count);

    // Convert to YAML
    let yaml_content = serde_yaml::to_string(&capability_file)
        .map_err(|e| format!("Failed to serialize to YAML: {}", e))?;

    // Write output file
    fs::write(output_file, yaml_content)
        .map_err(|e| format!("Failed to write output file '{}': {}", output_file, e))?;

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

/// Parse streaming strategy from string
fn parse_streaming_strategy(strategy: &str) -> Result<StreamingStrategy, String> {
    match strategy.to_lowercase().as_str() {
        "polling" => Ok(StreamingStrategy::Polling),
        "pagination" => Ok(StreamingStrategy::Pagination),
        "agent-level" | "agentlevel" => Ok(StreamingStrategy::AgentLevel),
        _ => Err(format!("Invalid streaming strategy: {}. Use 'polling', 'pagination', or 'agent-level'", strategy)),
    }
}