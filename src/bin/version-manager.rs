#!/usr/bin/env cargo

//! Version Manager for MagicTunnel
//! 
//! This utility helps manage version information across the codebase,
//! ensuring consistency between Cargo.toml and all other files.

use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use regex::Regex;

#[derive(Parser)]
#[command(name = "version-manager")]
#[command(about = "Manage version information across MagicTunnel codebase")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Update all files with current Cargo.toml version
    Update,
    /// Check for version inconsistencies
    Check,
    /// Set a new version across all files
    Set { version: String },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Update => update_all_versions()?,
        Commands::Check => check_version_consistency()?,
        Commands::Set { version } => set_version(&version)?,
    }
    
    Ok(())
}

fn get_cargo_version() -> Result<String, Box<dyn std::error::Error>> {
    let cargo_content = fs::read_to_string("Cargo.toml")?;
    let version_regex = Regex::new(r#"version = "([^"]+)""#)?;
    
    if let Some(captures) = version_regex.captures(&cargo_content) {
        Ok(captures[1].to_string())
    } else {
        Err("Could not find version in Cargo.toml".into())
    }
}

fn get_cargo_name() -> Result<String, Box<dyn std::error::Error>> {
    let cargo_content = fs::read_to_string("Cargo.toml")?;
    let name_regex = Regex::new(r#"name = "([^"]+)""#)?;
    
    if let Some(captures) = name_regex.captures(&cargo_content) {
        Ok(captures[1].to_string())
    } else {
        Err("Could not find name in Cargo.toml".into())
    }
}

fn update_all_versions() -> Result<(), Box<dyn std::error::Error>> {
    let version = get_cargo_version()?;
    let name = get_cargo_name()?;

    println!("Updating all files to version: {}", version);

    // Create replacement strings with proper lifetimes
    let readme_replacement = format!("Current Version**: {}", version);
    let claude_replacement = format!("Current Version**: {}", version);
    let claude_current_replacement = format!("Version {} (Current)", version);
    let docs_replacement = format!("Current Version**: {}", version);
    let config_replacement = format!(r#"client_version: "{}""#, version);
    let magictunnel_config_replacement = format!(r#"client_version: "{}""#, version);
    let json_replacement = format!(r#""version": "{}""#, version);

    // Additional replacement strings for comprehensive patterns
    let _version_in_text = format!("v{}", version);
    let version_complete_text = format!("COMPLETE IN v{}", version);
    let version_migration_text = format!("Version {} External MCP Migration", version);
    let version_range_text = format!("Version {}-{}", version, version);
    let version_current_text = format!("(current: {})", version);
    let test_version_replacement = format!(r#"client_version: "{}".to_string()"#, version);
    let test_name_replacement = format!(r#"client_name: "{}".to_string()"#, name);
    let mcp_external_test_version = format!(r#"client_version: "{}".to_string()"#, version);
    let mcp_external_test_name = format!(r#"client_name: "{}-test".to_string()"#, name);
    let server_name_replacement = format!(r#""name": "{}""#, name);
    let server_version_replacement = format!(r#""version": "{}""#, version);
    let service_name_replacement = format!(r#""service": "{}""#, name);
    let user_agent_replacement = format!(r#""{}/{}""#, name, version);

    // Files to update with version patterns
    let version_files = vec![
        ("README.md", r"Current Version\*\*: 0\.2\.[0-9]+", readme_replacement.as_str()),
        ("README.md", r"COMPLETE IN v0\.2\.[0-9]+", version_complete_text.as_str()),
        ("README.md", r"Version 0\.2\.[0-9]+ External MCP Migration", version_migration_text.as_str()),
        ("CLAUDE.md", r"Current Version\*\*: 0\.2\.[0-9]+", claude_replacement.as_str()),
        ("CLAUDE.md", r"Version 0\.2\.[0-9]+ \(Current\)", claude_current_replacement.as_str()),
        ("CLAUDE.md", r"Version 0\.2\.[0-9]+-0\.2\.[0-9]+", version_range_text.as_str()),
        ("CLAUDE.md", r"\(current: 0\.2\.[0-9]+\)", version_current_text.as_str()),
        ("test-resources/documentation.md", r"Current Version\*\*: 0\.2\.[0-9]+", docs_replacement.as_str()),
        ("test-resources/documentation.md", r"COMPLETE IN v0\.2\.[0-9]+", version_complete_text.as_str()),
        ("test-resources/documentation.md", r"Version 0\.2\.[0-9]+ External MCP Migration", version_migration_text.as_str()),
        ("config.yaml.template", r#"client_version: "0\.2\.[0-9]+""#, config_replacement.as_str()),
        ("magictunnel-config.yaml", r#"client_version: "0\.2\.[0-9]+""#, magictunnel_config_replacement.as_str()),
        ("test-resources/info.json", r#""version": "0\.2\.[0-9]+""#, json_replacement.as_str()),
        ("tests/test_config_validation.rs", r#"client_version: "0\.2\.[0-9]+".to_string\(\)"#, test_version_replacement.as_str()),
        ("tests/test_config_validation.rs", r#"client_name: "magictunnel".to_string\(\)"#, test_name_replacement.as_str()),
        ("tests/mcp_external_tests.rs", r#"client_version: "0\.2\.[0-9]+".to_string\(\)"#, mcp_external_test_version.as_str()),
        ("tests/mcp_external_tests.rs", r#"client_name: "magictunnel-test".to_string\(\)"#, mcp_external_test_name.as_str()),
        ("src/mcp/server.rs", r#""name": "magictunnel""#, server_name_replacement.as_str()),
        ("src/mcp/server.rs", r#""version": "0\.2\.[0-9]+""#, server_version_replacement.as_str()),
        ("src/mcp/server.rs", r#""service": "magictunnel""#, service_name_replacement.as_str()),
        ("src/auth/oauth.rs", r#""magictunnel/0\.2\.[0-9]+""#, user_agent_replacement.as_str()),
    ];

    for (file_path, pattern, replacement) in version_files {
        update_file_with_regex(file_path, pattern, replacement)?;
    }

    println!("Version update complete!");
    Ok(())
}

fn update_file_with_regex(file_path: &str, pattern: &str, replacement: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(file_path).exists() {
        println!("Warning: {} not found, skipping", file_path);
        return Ok(());
    }
    
    let content = fs::read_to_string(file_path)?;
    let regex = Regex::new(pattern)?;
    let updated_content = regex.replace_all(&content, replacement);
    
    if content != updated_content {
        fs::write(file_path, updated_content.as_ref())?;
        println!("Updated: {}", file_path);
    } else {
        println!("No changes needed: {}", file_path);
    }
    
    Ok(())
}

fn check_version_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let cargo_version = get_cargo_version()?;
    println!("Cargo.toml version: {}", cargo_version);
    
    // Check various files for version consistency
    let files_to_check = vec![
        "README.md",
        "CLAUDE.md", 
        "test-resources/documentation.md",
        "config.yaml.template",
        "magictunnel-config.yaml",
        "test-resources/info.json",
    ];
    
    let mut inconsistencies = Vec::new();
    
    for file_path in files_to_check {
        if let Ok(content) = fs::read_to_string(file_path) {
            let version_regex = Regex::new(r"0\.2\.[0-9]+")?;
            for version_match in version_regex.find_iter(&content) {
                let found_version = version_match.as_str();
                if found_version != cargo_version {
                    inconsistencies.push(format!("{}: found {}, expected {}", file_path, found_version, cargo_version));
                }
            }
        }
    }
    
    if inconsistencies.is_empty() {
        println!("✅ All versions are consistent!");
    } else {
        println!("❌ Version inconsistencies found:");
        for inconsistency in inconsistencies {
            println!("  {}", inconsistency);
        }
    }
    
    Ok(())
}

fn set_version(new_version: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Update Cargo.toml first
    let cargo_content = fs::read_to_string("Cargo.toml")?;
    let version_regex = Regex::new(r#"version = "[^"]+""#)?;
    let updated_cargo = version_regex.replace(&cargo_content, &format!(r#"version = "{}""#, new_version));
    fs::write("Cargo.toml", updated_cargo.as_ref())?;
    
    println!("Updated Cargo.toml to version: {}", new_version);
    
    // Now update all other files
    update_all_versions()?;
    
    Ok(())
}
