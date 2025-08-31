//! Configuration Migration CLI
//!
//! This CLI tool provides commands for migrating MagicTunnel configurations
//! from the flat structure to the new hierarchical structure.

use clap::{Parser, Subcommand};
use magictunnel::config::Config;
use magictunnel::config::hierarchical::HierarchicalConfig;
use magictunnel::error::{ProxyError, Result};
use std::path::PathBuf;
use std::fs;
use tracing::{info, warn, error};
use serde_yaml;

#[derive(Parser)]
#[command(name = "magictunnel-config-migrate")]
#[command(about = "MagicTunnel Configuration Migration Utility")]
#[command(version)]
struct Cli {
    /// Configuration file path (flat structure)
    #[arg(short, long, default_value = "config.yaml")]
    input: PathBuf,

    /// Output file path (hierarchical structure) 
    #[arg(short, long, default_value = "magictunnel-config.yaml")]
    output: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert flat config to hierarchical structure
    Convert {
        /// Overwrite output file if it exists
        #[arg(short, long)]
        force: bool,
        /// Validate the converted config
        #[arg(short, long)]
        validate: bool,
    },
    /// Validate an existing hierarchical config
    Validate {
        /// Path to hierarchical config file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Show preview of conversion without writing file
    Preview,
    /// Show differences between flat and hierarchical structures
    Diff,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_level(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    match cli.command {
        Commands::Convert { force, validate } => {
            convert_config(&cli.input, &cli.output, force, validate).await
        },
        Commands::Validate { config } => {
            let config_path = config.unwrap_or(cli.output);
            validate_hierarchical_config(&config_path).await
        },
        Commands::Preview => {
            preview_conversion(&cli.input).await
        },
        Commands::Diff => {
            show_diff(&cli.input).await
        },
    }
}

/// Convert flat configuration to hierarchical structure
async fn convert_config(
    input_path: &PathBuf, 
    output_path: &PathBuf, 
    force: bool, 
    validate: bool
) -> Result<()> {
    info!("🔄 Starting configuration migration");
    info!("📥 Input: {}", input_path.display());
    info!("📤 Output: {}", output_path.display());
    
    // Check if input file exists
    if !input_path.exists() {
        error!("❌ Input configuration file does not exist: {}", input_path.display());
        return Err(ProxyError::config(
            format!("Input file not found: {}", input_path.display())
        ));
    }
    
    // Check if output file exists (unless force is specified)
    if output_path.exists() && !force {
        error!("❌ Output file already exists: {}", output_path.display());
        error!("💡 Use --force to overwrite or choose a different output path");
        return Err(ProxyError::config(
            "Output file exists and --force not specified".to_string()
        ));
    }
    
    // Load flat configuration
    info!("📖 Loading flat configuration...");
    let flat_config = Config::load(input_path, None, None)?;
    
    // Convert to hierarchical
    info!("🔄 Converting to hierarchical structure...");
    let hierarchical_config = HierarchicalConfig::from_flat_config(flat_config);
    
    // Validate if requested
    if validate {
        info!("✅ Validating converted configuration...");
        if let Err(e) = hierarchical_config.validate() {
            error!("❌ Validation failed: {}", e);
            return Err(ProxyError::validation(e.to_string()));
        }
        info!("✅ Validation passed");
    }
    
    // Write hierarchical configuration
    info!("💾 Writing hierarchical configuration...");
    let yaml_content = serde_yaml::to_string(&hierarchical_config)
        .map_err(|e| ProxyError::config(format!("YAML serialization failed: {}", e)))?;
    
    fs::write(output_path, yaml_content)
        .map_err(|e| ProxyError::config(format!("Failed to write output file: {}", e)))?;
    
    info!("✅ Configuration migration completed successfully!");
    info!("📁 New hierarchical config written to: {}", output_path.display());
    
    // Show summary
    show_migration_summary(&hierarchical_config);
    
    Ok(())
}

/// Validate hierarchical configuration
async fn validate_hierarchical_config(config_path: &PathBuf) -> Result<()> {
    info!("🔍 Validating hierarchical configuration: {}", config_path.display());
    
    if !config_path.exists() {
        error!("❌ Configuration file does not exist: {}", config_path.display());
        return Err(ProxyError::config(
            format!("Config file not found: {}", config_path.display())
        ));
    }
    
    // Load and parse
    let content = fs::read_to_string(config_path)
        .map_err(|e| ProxyError::config(format!("Failed to read file: {}", e)))?;
    
    let config: HierarchicalConfig = serde_yaml::from_str(&content)
        .map_err(|e| ProxyError::config(format!("YAML parsing failed: {}", e)))?;
    
    // Validate
    match config.validate() {
        Ok(_) => {
            info!("✅ Configuration is valid!");
            show_config_summary(&config);
            Ok(())
        },
        Err(e) => {
            error!("❌ Configuration validation failed: {}", e);
            Err(ProxyError::validation(e.to_string()))
        }
    }
}

/// Preview conversion without writing file
async fn preview_conversion(input_path: &PathBuf) -> Result<()> {
    info!("👀 Previewing configuration conversion");
    
    if !input_path.exists() {
        error!("❌ Input configuration file does not exist: {}", input_path.display());
        return Err(ProxyError::config(
            format!("Input file not found: {}", input_path.display())
        ));
    }
    
    // Load and convert
    let flat_config = Config::load(input_path, None, None)?;
    let hierarchical_config = HierarchicalConfig::from_flat_config(flat_config);
    
    // Show YAML preview
    let yaml_content = serde_yaml::to_string(&hierarchical_config)
        .map_err(|e| ProxyError::config(format!("YAML serialization failed: {}", e)))?;
    
    println!("📄 Hierarchical Configuration Preview:");
    println!("=====================================");
    println!("{}", yaml_content);
    println!("=====================================");
    
    show_migration_summary(&hierarchical_config);
    
    Ok(())
}

/// Show differences between structures
async fn show_diff(input_path: &PathBuf) -> Result<()> {
    info!("📊 Analyzing configuration structure differences");
    
    if !input_path.exists() {
        error!("❌ Input configuration file does not exist: {}", input_path.display());
        return Err(ProxyError::config(
            format!("Input file not found: {}", input_path.display())
        ));
    }
    
    let flat_config = Config::load(input_path, None, None)?;
    let hierarchical_config = HierarchicalConfig::from_flat_config(flat_config.clone());
    
    println!("🔄 Configuration Structure Analysis");
    println!("===================================");
    println!();
    
    println!("📊 STRUCTURE MAPPING:");
    println!("• Server settings        → Global.server");
    println!("• Registry settings      → Global.registry");  
    println!("• Authentication         → Global.auth");
    println!("• Logging                → Global.logging");
    println!("• Security               → Global.security");
    println!("• MCP protocol           → MCP.*");
    println!("• Smart Discovery        → Discovery.smart_discovery");
    println!("• Tool Enhancement       → Discovery.tool_enhancement");
    println!("• Visibility             → Discovery.visibility");
    println!("• Tool-specific configs  → Tools[tool_name]");
    println!();
    
    // Show configuration counts
    println!("📈 CONFIGURATION STATISTICS:");
    if let Some(smart_discovery) = &flat_config.smart_discovery {
        println!("• Smart Discovery: Enabled = {}", smart_discovery.enabled);
    }
    if let Some(security) = &flat_config.security {
        println!("• Security features: {} configured", 
                if security.allowlist.is_some() { 1 } else { 0 } +
                if security.rbac.is_some() { 1 } else { 0 } +
                if security.audit.is_some() { 1 } else { 0 } +
                if security.sanitization.is_some() { 1 } else { 0 });
    }
    println!("• Tool overrides: {}", hierarchical_config.tools.len());
    
    // Show validation status
    match hierarchical_config.validate() {
        Ok(_) => println!("✅ Converted config would be valid"),
        Err(e) => {
            warn!("⚠️ Converted config has validation issues: {}", e);
        }
    }
    
    Ok(())
}

/// Show migration summary
fn show_migration_summary(config: &HierarchicalConfig) {
    println!();
    println!("📊 Migration Summary:");
    println!("====================");
    println!("🌍 Global Configuration:");
    println!("  • Server: {}:{}", config.global.server.host, config.global.server.port);
    println!("  • Registry paths: {:?}", config.global.registry.paths);
    println!("  • Auth enabled: {}", config.global.auth.enabled);
    
    println!("📡 MCP Configuration:");
    println!("  • Timeout override: {:?}", config.mcp.timeout);
    println!("  • Max retries: {:?}", config.mcp.max_retries);
    
    println!("🤖 Discovery Configuration:");
    println!("  • Smart Discovery enabled: {}", config.discovery.smart_discovery.enabled);
    println!("  • Selection mode: {}", config.discovery.smart_discovery.tool_selection_mode);
    
    println!("🛠️ Tool Configurations:");
    println!("  • Tool overrides: {}", config.tools.len());
    
    if !config.tools.is_empty() {
        println!("  • Tools with overrides:");
        for tool_name in config.tools.keys().take(5) {
            println!("    - {}", tool_name);
        }
        if config.tools.len() > 5 {
            println!("    ... and {} more", config.tools.len() - 5);
        }
    }
}

/// Show configuration summary
fn show_config_summary(config: &HierarchicalConfig) {
    println!();
    println!("📊 Configuration Summary:");
    println!("========================");
    println!("🌍 Global: Server on {}:{}", config.global.server.host, config.global.server.port);
    println!("📡 MCP: {} protocol services configured", 
             (if config.mcp.timeout.is_some() { 1 } else { 0 }) +
             (if config.mcp.max_retries.is_some() { 1 } else { 0 }));
    println!("🤖 Discovery: Smart Discovery {}", 
             if config.discovery.smart_discovery.enabled { "enabled" } else { "disabled" });
    println!("🛠️ Tools: {} tool configurations", config.tools.len());
}