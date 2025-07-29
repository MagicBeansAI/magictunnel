//! OpenAPI Capability Generator CLI
//! 
//! Command-line tool for generating MCP capability files from OpenAPI specifications.

use clap::{Arg, Command};
use magictunnel::registry::openapi_generator::{OpenAPICapabilityGenerator, AuthConfig, AuthType, NamingConvention};
use std::collections::HashMap;
use std::fs;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("openapi-generator")
        .about("Generate MCP capability files from OpenAPI specifications")
        .version("1.0.0")
        .arg(
            Arg::new("spec")
                .short('s')
                .long("spec")
                .value_name("FILE")
                .help("OpenAPI specification file (JSON or YAML)")
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
            Arg::new("base-url")
                .short('u')
                .long("base-url")
                .value_name("URL")
                .help("Base URL for the API")
                .required(true)
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
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    let spec_file = matches.get_one::<String>("spec").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();
    let base_url = matches.get_one::<String>("base-url").unwrap();

    // Read the OpenAPI specification
    let spec_content = fs::read_to_string(spec_file)
        .map_err(|e| format!("Failed to read spec file '{}': {}", spec_file, e))?;

    // Detect format
    let detected_format = if spec_file.ends_with(".json") || spec_content.trim_start().starts_with('{') {
        "json"
    } else {
        "yaml"
    };

    println!("Parsing OpenAPI specification from '{}' (format: {})...", spec_file, detected_format);

    // Create generator
    let mut generator = OpenAPICapabilityGenerator::new(base_url.clone());

    // Set prefix if provided
    if let Some(prefix) = matches.get_one::<String>("prefix") {
        generator = generator.with_prefix(prefix.clone());
    }

    // Set naming convention
    let naming = matches.get_one::<String>("naming").unwrap();
    let naming_convention = match naming.as_str() {
        "operation-id" => NamingConvention::OperationId,
        "method-path" => NamingConvention::MethodPath,
        _ => return Err(format!("Invalid naming convention: {}. Use 'operation-id' or 'method-path'", naming).into()),
    };
    generator = generator.with_naming_convention(naming_convention);

    // Set method filter if provided
    if let Some(methods) = matches.get_one::<String>("methods") {
        let method_list: Vec<String> = methods.split(',')
            .map(|m| m.trim().to_uppercase())
            .collect();
        generator = generator.with_method_filter(method_list);
    }

    // Include deprecated if requested
    if matches.get_flag("include-deprecated") {
        generator = generator.include_deprecated();
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
        _ => return Err(format!("Invalid auth type: {}. Use 'none', 'bearer', 'apikey', or 'basic'", auth_type).into()),
    };

    if let Some(auth) = auth_config {
        generator = generator.with_auth(auth);
    }

    // Generate capability file (auto-detect format)
    let capability_file = generator.generate_from_spec(&spec_content)
        .map_err(|e| format!("Failed to generate capability file from OpenAPI/Swagger spec: {:?}", e))?;

    println!("Generated {} tools from OpenAPI specification", capability_file.tools.len());

    // Convert to YAML
    let yaml_content = serde_yaml::to_string(&capability_file)
        .map_err(|e| format!("Failed to serialize to YAML: {}", e))?;

    // Write output file
    fs::write(output_file, yaml_content)
        .map_err(|e| format!("Failed to write output file '{}': {}", output_file, e))?;

    println!("Capability file written to '{}'", output_file);

    // Print summary
    println!("\nGenerated tools:");
    for tool in &capability_file.tools {
        println!("  - {}: {}", tool.name, tool.description);
    }

    Ok(())
}
