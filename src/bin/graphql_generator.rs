use clap::{Arg, Command};
use magictunnel::registry::graphql_generator::{GraphQLCapabilityGenerator, AuthConfig, AuthType};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("GraphQL Schema Capability Generator")
        .version("1.0")
        .author("MCP Proxy Team")
        .about("Generates MCP capability files from GraphQL schemas")
        .arg(
            Arg::new("schema")
                .short('s')
                .long("schema")
                .value_name("FILE")
                .help("GraphQL schema file (SDL or JSON introspection)")
                .required(true),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("FORMAT")
                .help("Schema format: sdl or json (auto-detected if not specified)")
                .required(false),
        )
        .arg(
            Arg::new("endpoint")
                .short('e')
                .long("endpoint")
                .value_name("URL")
                .help("GraphQL endpoint URL")
                .required(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output capability file (YAML)")
                .required(true),
        )
        .arg(
            Arg::new("prefix")
                .short('p')
                .long("prefix")
                .value_name("PREFIX")
                .help("Tool name prefix")
                .required(false),
        )
        .arg(
            Arg::new("auth-type")
                .short('a')
                .long("auth-type")
                .value_name("TYPE")
                .help("Authentication type: none, bearer, apikey")
                .required(false)
                .default_value("none"),
        )
        .arg(
            Arg::new("auth-token")
                .short('t')
                .long("auth-token")
                .value_name("TOKEN")
                .help("Authentication token (for bearer or apikey)")
                .required(false),
        )
        .arg(
            Arg::new("auth-header")
                .long("auth-header")
                .value_name("HEADER")
                .help("Authentication header name (for apikey)")
                .required(false)
                .default_value("Authorization"),
        )
        .get_matches();

    // Read schema file
    let schema_file = matches.get_one::<String>("schema").unwrap();
    let schema_content = fs::read_to_string(schema_file)
        .map_err(|e| format!("Failed to read schema file '{}': {}", schema_file, e))?;

    // Get endpoint URL
    let endpoint = matches.get_one::<String>("endpoint").unwrap();

    // Create generator
    let mut generator = GraphQLCapabilityGenerator::new(endpoint.clone());

    // Add prefix if specified
    if let Some(prefix) = matches.get_one::<String>("prefix") {
        generator = generator.with_prefix(prefix.clone());
    }

    // Configure authentication
    let auth_type = matches.get_one::<String>("auth-type").unwrap();
    if auth_type != "none" {
        let auth_token = matches.get_one::<String>("auth-token")
            .ok_or("Authentication token required for non-none auth types")?;

        let auth_config = match auth_type.as_str() {
            "bearer" => AuthConfig {
                auth_type: AuthType::Bearer { token: auth_token.clone() },
                headers: HashMap::new(),
            },
            "apikey" => {
                let header = matches.get_one::<String>("auth-header").unwrap();
                AuthConfig {
                    auth_type: AuthType::ApiKey {
                        header: header.clone(),
                        value: auth_token.clone(),
                    },
                    headers: HashMap::new(),
                }
            }
            _ => return Err(format!("Unsupported auth type: {}", auth_type).into()),
        };

        generator = generator.with_auth(auth_config);
    }

    // Determine format and generate capability file
    let format = matches.get_one::<String>("format");
    let detected_format = if let Some(fmt) = format {
        fmt.as_str()
    } else {
        // Auto-detect format based on file extension or content
        if schema_file.ends_with(".json") || schema_content.trim_start().starts_with('{') {
            "json"
        } else {
            "sdl"
        }
    };

    println!("Parsing GraphQL schema from '{}' (format: {})...", schema_file, detected_format);

    let capability_file = match detected_format {
        "json" => generator.generate_from_introspection(&schema_content)
            .map_err(|e| format!("Failed to generate capability file from JSON introspection: {:?}", e))?,
        "sdl" => generator.generate_from_sdl(&schema_content)
            .map_err(|e| format!("Failed to generate capability file from SDL: {:?}", e))?,
        _ => return Err(format!("Unsupported format: {}. Use 'sdl' or 'json'", detected_format).into()),
    };

    println!("Generated {} tools from GraphQL schema", capability_file.tools.len());

    // Convert to YAML
    let yaml_content = serde_yaml::to_string(&capability_file)
        .map_err(|e| format!("Failed to serialize to YAML: {}", e))?;

    // Write output file
    let output_file = matches.get_one::<String>("output").unwrap();
    fs::write(output_file, yaml_content)
        .map_err(|e| format!("Failed to write output file '{}': {}", output_file, e))?;

    println!("Capability file written to '{}'", output_file);

    // Print summary
    println!("\nSummary:");
    println!("  Schema file: {}", schema_file);
    println!("  Schema format: {}", detected_format);
    println!("  Endpoint: {}", endpoint);
    println!("  Output file: {}", output_file);
    if let Some(prefix) = matches.get_one::<String>("prefix") {
        println!("  Tool prefix: {}", prefix);
    }
    println!("  Auth type: {}", auth_type);
    println!("  Tools generated: {}", capability_file.tools.len());

    // Show first few tools
    println!("\nFirst 5 tools:");
    for (i, tool) in capability_file.tools.iter().take(5).enumerate() {
        println!("  {}. {}: {}", i + 1, tool.name, tool.description);
    }

    if capability_file.tools.len() > 5 {
        println!("  ... and {} more", capability_file.tools.len() - 5);
    }

    Ok(())
}
