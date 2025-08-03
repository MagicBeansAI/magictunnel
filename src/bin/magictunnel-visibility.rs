//! Visibility Management CLI
//!
//! This CLI tool provides commands for managing tool visibility in capability files.
//! It allows you to hide/show tools globally, per-file, or individually.

use clap::{Parser, Subcommand};
use magictunnel::config::Config;
use magictunnel::registry::types::CapabilityFile;
use magictunnel::registry::types::ToolDefinition;
use magictunnel::discovery::enhancement::ToolEnhancementConfig;
use magictunnel::error::{ProxyError, Result};
use std::path::PathBuf;
use std::fs;
use tracing::info;

#[derive(Parser)]
#[command(name = "magictunnel-visibility")]
#[command(about = "MagicTunnel Tool Visibility Management")]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show visibility status of all tools
    Status {
        /// Show detailed information for each tool
        #[arg(short, long)]
        detailed: bool,
    },
    /// Hide all tools globally
    HideAll {
        /// Confirm the action
        #[arg(short, long)]
        confirm: bool,
    },
    /// Show all tools globally
    ShowAll {
        /// Confirm the action
        #[arg(short, long)]
        confirm: bool,
    },
    /// Hide all tools in a specific capability file
    HideFile {
        /// Path to the capability file
        file: PathBuf,
        /// Confirm the action
        #[arg(short, long)]
        confirm: bool,
    },
    /// Show all tools in a specific capability file
    ShowFile {
        /// Path to the capability file
        file: PathBuf,
        /// Confirm the action
        #[arg(short, long)]
        confirm: bool,
    },
    /// Hide a specific tool
    HideTool {
        /// Name of the tool to hide
        tool_name: String,
        /// Capability file containing the tool (optional, will search if not provided)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Show a specific tool
    ShowTool {
        /// Name of the tool to show
        tool_name: String,
        /// Capability file containing the tool (optional, will search if not provided)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// List all capability files
    ListFiles,
    /// Enable all tools globally
    EnableAll {
        /// Confirm the action
        #[arg(short, long)]
        confirm: bool,
    },
    /// Disable all tools globally
    DisableAll {
        /// Confirm the action
        #[arg(short, long)]
        confirm: bool,
    },
    /// Enable all tools in a specific capability file
    EnableFile {
        /// Path to the capability file
        file: PathBuf,
        /// Confirm the action
        #[arg(short, long)]
        confirm: bool,
    },
    /// Disable all tools in a specific capability file
    DisableFile {
        /// Path to the capability file
        file: PathBuf,
        /// Confirm the action
        #[arg(short, long)]
        confirm: bool,
    },
    /// Enable a specific tool
    EnableTool {
        /// Name of the tool to enable
        tool_name: String,
        /// Capability file containing the tool (optional, will search if not provided)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Disable a specific tool
    DisableTool {
        /// Name of the tool to disable
        tool_name: String,
        /// Capability file containing the tool (optional, will search if not provided)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Show MCP capability override warnings
    ShowMcpWarnings {
        /// Show detailed information about external MCP tools and their capabilities
        #[arg(short, long)]
        detailed: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(cli.log_level.parse().unwrap_or(tracing::Level::INFO))
        .init();

    // Load configuration
    let config = Config::load(&cli.config, None, None)?;

    match cli.command {
        Commands::Status { detailed } => show_status(&config, detailed).await,
        Commands::HideAll { confirm } => hide_all_tools(&config, confirm).await,
        Commands::ShowAll { confirm } => show_all_tools(&config, confirm).await,
        Commands::HideFile { file, confirm } => hide_file_tools(&file, confirm).await,
        Commands::ShowFile { file, confirm } => show_file_tools(&file, confirm).await,
        Commands::HideTool { tool_name, file } => hide_tool(&config, &tool_name, file.as_ref()).await,
        Commands::ShowTool { tool_name, file } => show_tool(&config, &tool_name, file.as_ref()).await,
        Commands::ListFiles => list_files(&config).await,
        Commands::EnableAll { confirm } => enable_all_tools(&config, confirm).await,
        Commands::DisableAll { confirm } => disable_all_tools(&config, confirm).await,
        Commands::EnableFile { file, confirm } => enable_file_tools(&file, confirm).await,
        Commands::DisableFile { file, confirm } => disable_file_tools(&file, confirm).await,
        Commands::EnableTool { tool_name, file } => enable_tool(&config, &tool_name, file.as_ref()).await,
        Commands::DisableTool { tool_name, file } => disable_tool(&config, &tool_name, file.as_ref()).await,
        Commands::ShowMcpWarnings { detailed } => show_mcp_warnings(&config, detailed).await,
    }
}

async fn show_status(config: &Config, detailed: bool) -> Result<()> {
    info!("Showing tool visibility and enabled status");

    let capability_files = discover_capability_files(config).await?;
    let mut total_tools = 0;
    let mut hidden_tools = 0;
    let mut visible_tools = 0;
    let mut enabled_tools = 0;
    let mut disabled_tools = 0;
    let mut active_tools = 0; // visible + enabled

    println!("Tool Visibility & Enabled Status");
    println!("================================");

    for file_path in &capability_files {
        let capability_file = load_capability_file(file_path)?;
        let file_total = capability_file.tool_count();
        let file_hidden = capability_file.hidden_tool_count();
        let file_visible = capability_file.visible_tool_count();
        let file_enabled = capability_file.enabled_tool_count();
        let file_disabled = capability_file.disabled_tool_count();
        let file_active = capability_file.active_tools().len();

        total_tools += file_total;
        hidden_tools += file_hidden;
        visible_tools += file_visible;
        enabled_tools += file_enabled;
        disabled_tools += file_disabled;
        active_tools += file_active;

        println!("\nFile: {}", file_path.display());
        println!("  Total: {}, Visible: {}, Hidden: {}", file_total, file_visible, file_hidden);
        println!("  Enabled: {}, Disabled: {}, Active: {}", file_enabled, file_disabled, file_active);

        if detailed {
            for tool in &capability_file.tools {
                let visibility = if tool.is_hidden() { "HIDDEN" } else { "VISIBLE" };
                let enabled_status = if tool.is_enabled() { "ENABLED" } else { "DISABLED" };
                println!("    {} - {} ({}, {})", tool.name, tool.description, visibility, enabled_status);
            }
        }
    }

    println!("\nOverall Summary");
    println!("===============");
    println!("Total tools: {}", total_tools);
    println!("Visible tools: {}", visible_tools);
    println!("Hidden tools: {}", hidden_tools);
    println!("Enabled tools: {}", enabled_tools);
    println!("Disabled tools: {}", disabled_tools);
    println!("Active tools: {} (visible + enabled)", active_tools);
    println!("Capability files: {}", capability_files.len());

    Ok(())
}

async fn hide_all_tools(config: &Config, confirm: bool) -> Result<()> {
    if !confirm {
        println!("This will hide ALL tools in ALL capability files.");
        println!("Use --confirm to proceed with this action.");
        return Ok(());
    }

    info!("Hiding all tools globally");
    let capability_files = discover_capability_files(config).await?;
    let mut modified_count = 0;

    for file_path in &capability_files {
        if modify_file_visibility(file_path, true).await? {
            modified_count += 1;
            println!("Modified: {}", file_path.display());
        }
    }

    println!("Successfully hid all tools in {} files", modified_count);
    Ok(())
}

async fn show_all_tools(config: &Config, confirm: bool) -> Result<()> {
    if !confirm {
        println!("This will show ALL tools in ALL capability files.");
        println!("Use --confirm to proceed with this action.");
        return Ok(());
    }

    info!("Showing all tools globally");
    let capability_files = discover_capability_files(config).await?;
    let mut modified_count = 0;

    for file_path in &capability_files {
        if modify_file_visibility(file_path, false).await? {
            modified_count += 1;
            println!("Modified: {}", file_path.display());
        }
    }

    println!("Successfully showed all tools in {} files", modified_count);
    Ok(())
}

async fn hide_file_tools(file_path: &PathBuf, confirm: bool) -> Result<()> {
    if !confirm {
        println!("This will hide ALL tools in file: {}", file_path.display());
        println!("Use --confirm to proceed with this action.");
        return Ok(());
    }

    info!("Hiding all tools in file: {}", file_path.display());
    if modify_file_visibility(file_path, true).await? {
        println!("Successfully hid all tools in: {}", file_path.display());
    } else {
        println!("No changes needed in: {}", file_path.display());
    }
    Ok(())
}

async fn show_file_tools(file_path: &PathBuf, confirm: bool) -> Result<()> {
    if !confirm {
        println!("This will show ALL tools in file: {}", file_path.display());
        println!("Use --confirm to proceed with this action.");
        return Ok(());
    }

    info!("Showing all tools in file: {}", file_path.display());
    if modify_file_visibility(file_path, false).await? {
        println!("Successfully showed all tools in: {}", file_path.display());
    } else {
        println!("No changes needed in: {}", file_path.display());
    }
    Ok(())
}

async fn hide_tool(config: &Config, tool_name: &str, file_path: Option<&PathBuf>) -> Result<()> {
    info!("Hiding tool: {}", tool_name);
    
    if let Some(file_path) = file_path {
        modify_tool_visibility(file_path, tool_name, true).await
    } else {
        // Search for the tool in all capability files
        let capability_files = discover_capability_files(config).await?;
        for file_path in &capability_files {
            let capability_file = load_capability_file(file_path)?;
            // Check both enhanced and legacy tools
            if capability_file.get_enhanced_tool(tool_name).is_some() || capability_file.get_tool(tool_name).is_some() {
                return modify_tool_visibility(file_path, tool_name, true).await;
            }
        }
        Err(ProxyError::registry(format!("Tool '{}' not found in any capability file", tool_name)))
    }
}

async fn show_tool(config: &Config, tool_name: &str, file_path: Option<&PathBuf>) -> Result<()> {
    info!("Showing tool: {}", tool_name);
    
    if let Some(file_path) = file_path {
        modify_tool_visibility(file_path, tool_name, false).await
    } else {
        // Search for the tool in all capability files
        let capability_files = discover_capability_files(config).await?;
        for file_path in &capability_files {
            let capability_file = load_capability_file(file_path)?;
            // Check both enhanced and legacy tools
            if capability_file.get_enhanced_tool(tool_name).is_some() || capability_file.get_tool(tool_name).is_some() {
                return modify_tool_visibility(file_path, tool_name, false).await;
            }
        }
        Err(ProxyError::registry(format!("Tool '{}' not found in any capability file", tool_name)))
    }
}

async fn list_files(config: &Config) -> Result<()> {
    info!("Listing capability files");
    
    let capability_files = discover_capability_files(config).await?;
    
    println!("Capability Files");
    println!("================");
    
    for file_path in &capability_files {
        let capability_file = load_capability_file(file_path)?;
        let total = capability_file.tool_count();
        let visible = capability_file.visible_tool_count();
        let hidden = capability_file.hidden_tool_count();
        
        println!("{}", file_path.display());
        println!("  Tools: {} total, {} visible, {} hidden", total, visible, hidden);
    }
    
    println!("\nTotal files: {}", capability_files.len());
    Ok(())
}

// Helper functions

async fn discover_capability_files(config: &Config) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path_str in &config.registry.paths {
        let path = PathBuf::from(path_str);

        if path.is_file() && is_yaml_file(&path) {
            files.push(path);
        } else if path.is_dir() {
            let dir_files = discover_directory_files(&path).await?;
            files.extend(dir_files);
        }
    }

    Ok(files)
}

async fn discover_directory_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await.map_err(|e| {
        ProxyError::registry(format!("Failed to read directory {}: {}", dir.display(), e))
    })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        ProxyError::registry(format!("Failed to read directory entry: {}", e))
    })? {
        let path = entry.path();
        if path.is_file() && is_yaml_file(&path) {
            files.push(path);
        } else if path.is_dir() {
            // Recursive discovery with boxing to avoid infinite size
            let sub_files = Box::pin(discover_directory_files(&path)).await?;
            files.extend(sub_files);
        }
    }

    Ok(files)
}

fn is_yaml_file(path: &PathBuf) -> bool {
    if let Some(extension) = path.extension() {
        matches!(extension.to_str(), Some("yaml") | Some("yml"))
    } else {
        false
    }
}

fn load_capability_file(path: &PathBuf) -> Result<CapabilityFile> {
    let content = fs::read_to_string(path).map_err(|e| {
        ProxyError::registry(format!("Failed to read file {}: {}", path.display(), e))
    })?;
    
    // Try to parse as enhanced format first, then fall back to legacy format
    match parse_enhanced_capability_file(&content) {
        Ok(capability_file) => Ok(capability_file),
        Err(_enhanced_error) => {
            // Fall back to legacy format parsing
            serde_yaml::from_str(&content).map_err(|legacy_error| {
                ProxyError::registry(format!(
                    "Failed to parse YAML file {} (tried both enhanced and legacy formats): {}", 
                    path.display(), legacy_error
                ))
            })
        }
    }
}

fn parse_enhanced_capability_file(content: &str) -> Result<CapabilityFile> {
    use serde_yaml::Value;
    use magictunnel::registry::types::{EnhancedToolDefinition, ToolDefinition, FileMetadata};
    
    // Parse as generic YAML first
    let yaml_value: Value = serde_yaml::from_str(content).map_err(|e| {
        ProxyError::registry(format!("Failed to parse YAML: {}", e))
    })?;
    
    // Check if this looks like enhanced format by looking for core.description in tools
    if let Some(Value::Sequence(tools)) = yaml_value.get("tools") {
        if let Some(first_tool) = tools.first() {
            if let Some(Value::Mapping(core)) = first_tool.get("core") {
                if core.get("description").is_some() {
                    // This looks like enhanced format, try to parse and convert
                    return parse_and_convert_enhanced_format(&yaml_value);
                }
            }
        }
    }
    
    Err(ProxyError::registry("Not enhanced format".to_string()))
}

fn parse_and_convert_enhanced_format(yaml_value: &serde_yaml::Value) -> Result<CapabilityFile> {
    use magictunnel::registry::types::{EnhancedToolDefinition, ToolDefinition, FileMetadata};
    
    // Extract metadata if present
    let metadata = if let Some(metadata_value) = yaml_value.get("metadata") {
        Some(serde_yaml::from_value(metadata_value.clone()).map_err(|e| {
            ProxyError::registry(format!("Failed to parse metadata: {}", e))
        })?)
    } else {
        None
    };
    
    // Extract and convert enhanced tools
    let enhanced_tools: Vec<EnhancedToolDefinition> = if let Some(tools_value) = yaml_value.get("tools") {
        serde_yaml::from_value(tools_value.clone()).map_err(|e| {
            ProxyError::registry(format!("Failed to parse enhanced tools: {}", e))
        })?
    } else {
        Vec::new()
    };
    
    // Convert enhanced tools to legacy format
    let legacy_tools: Vec<ToolDefinition> = enhanced_tools.iter()
        .map(|enhanced| enhanced.into())
        .collect();
    
    // Create capability file with legacy tools for CLI manipulation
    if let Some(metadata) = metadata {
        CapabilityFile::with_metadata(metadata, legacy_tools)
    } else {
        CapabilityFile::new(legacy_tools)
    }
}

async fn modify_file_visibility(file_path: &PathBuf, hidden: bool) -> Result<bool> {
    let mut capability_file = load_capability_file(file_path)?;
    let mut modified = false;

    for tool in &mut capability_file.tools {
        if tool.is_hidden() != hidden {
            tool.set_hidden(hidden);
            modified = true;
        }
    }

    if modified {
        save_capability_file(file_path, &capability_file)?;
    }

    Ok(modified)
}

async fn modify_tool_visibility(file_path: &PathBuf, tool_name: &str, hidden: bool) -> Result<()> {
    let mut capability_file = load_capability_file(file_path)?;

    capability_file.set_tool_hidden(tool_name, hidden)?;
    save_capability_file(file_path, &capability_file)?;

    let action = if hidden { "hidden" } else { "shown" };
    println!("Successfully {} tool '{}' in: {}", action, tool_name, file_path.display());

    Ok(())
}

async fn enable_all_tools(config: &Config, confirm: bool) -> Result<()> {
    if !confirm {
        println!("This will enable ALL tools in ALL capability files.");
        println!("Use --confirm to proceed with this action.");
        return Ok(());
    }

    info!("Enabling all tools globally");
    let capability_files = discover_capability_files(config).await?;
    let mut total_changed = 0;

    for file_path in capability_files {
        let mut capability_file = load_capability_file(&file_path)?;
        let disabled_count = capability_file.disabled_tool_count();

        if disabled_count > 0 {
            capability_file.set_all_tools_enabled(true);
            save_capability_file(&file_path, &capability_file)?;
            total_changed += disabled_count;
            println!("Enabled {} tools in: {}", disabled_count, file_path.display());
        }
    }

    println!("Successfully enabled {} tools across all capability files", total_changed);
    Ok(())
}

async fn disable_all_tools(config: &Config, confirm: bool) -> Result<()> {
    if !confirm {
        println!("This will disable ALL tools in ALL capability files.");
        println!("Use --confirm to proceed with this action.");
        return Ok(());
    }

    info!("Disabling all tools globally");
    let capability_files = discover_capability_files(config).await?;
    let mut total_changed = 0;

    for file_path in capability_files {
        let mut capability_file = load_capability_file(&file_path)?;
        let enabled_count = capability_file.enabled_tool_count();

        if enabled_count > 0 {
            capability_file.set_all_tools_enabled(false);
            save_capability_file(&file_path, &capability_file)?;
            total_changed += enabled_count;
            println!("Disabled {} tools in: {}", enabled_count, file_path.display());
        }
    }

    println!("Successfully disabled {} tools across all capability files", total_changed);
    Ok(())
}

async fn enable_file_tools(file_path: &PathBuf, confirm: bool) -> Result<()> {
    if !confirm {
        println!("This will enable ALL tools in: {}", file_path.display());
        println!("Use --confirm to proceed with this action.");
        return Ok(());
    }

    info!("Enabling all tools in file: {}", file_path.display());
    let mut capability_file = load_capability_file(file_path)?;
    let disabled_count = capability_file.disabled_tool_count();

    if disabled_count == 0 {
        println!("All tools in {} are already enabled", file_path.display());
        return Ok(());
    }

    capability_file.set_all_tools_enabled(true);
    save_capability_file(file_path, &capability_file)?;

    println!("Successfully enabled {} tools in: {}", disabled_count, file_path.display());
    Ok(())
}

async fn disable_file_tools(file_path: &PathBuf, confirm: bool) -> Result<()> {
    if !confirm {
        println!("This will disable ALL tools in: {}", file_path.display());
        println!("Use --confirm to proceed with this action.");
        return Ok(());
    }

    info!("Disabling all tools in file: {}", file_path.display());
    let mut capability_file = load_capability_file(file_path)?;
    let enabled_count = capability_file.enabled_tool_count();

    if enabled_count == 0 {
        println!("All tools in {} are already disabled", file_path.display());
        return Ok(());
    }

    capability_file.set_all_tools_enabled(false);
    save_capability_file(file_path, &capability_file)?;

    println!("Successfully disabled {} tools in: {}", enabled_count, file_path.display());
    Ok(())
}

async fn enable_tool(config: &Config, tool_name: &str, file_path: Option<&PathBuf>) -> Result<()> {
    set_tool_enabled_status(config, tool_name, file_path, true).await
}

async fn disable_tool(config: &Config, tool_name: &str, file_path: Option<&PathBuf>) -> Result<()> {
    set_tool_enabled_status(config, tool_name, file_path, false).await
}

async fn set_tool_enabled_status(config: &Config, tool_name: &str, file_path: Option<&PathBuf>, enabled: bool) -> Result<()> {
    let action = if enabled { "enable" } else { "disable" };
    info!("Attempting to {} tool: {}", action, tool_name);

    let file_path = if let Some(path) = file_path {
        path.clone()
    } else {
        // Search for the tool across all capability files
        let capability_files = discover_capability_files(config).await?;
        let mut found_file = None;

        for file in capability_files {
            let capability_file = load_capability_file(&file)?;
            if capability_file.tools.iter().any(|t| t.name == tool_name) {
                found_file = Some(file);
                break;
            }
        }

        found_file.ok_or_else(|| {
            ProxyError::registry(format!("Tool '{}' not found in any capability file", tool_name))
        })?
    };

    let mut capability_file = load_capability_file(&file_path)?;

    capability_file.set_tool_enabled(tool_name, enabled)?;
    save_capability_file(&file_path, &capability_file)?;

    let action = if enabled { "enabled" } else { "disabled" };
    println!("Successfully {} tool '{}' in: {}", action, tool_name, file_path.display());

    Ok(())
}

fn save_capability_file(file_path: &PathBuf, capability_file: &CapabilityFile) -> Result<()> {
    let yaml_content = serde_yaml::to_string(capability_file).map_err(|e| {
        ProxyError::registry(format!("Failed to serialize capability file: {}", e))
    })?;

    fs::write(file_path, yaml_content).map_err(|e| {
        ProxyError::registry(format!("Failed to write file {}: {}", file_path.display(), e))
    })?;

    Ok(())
}

async fn show_mcp_warnings(config: &Config, detailed: bool) -> Result<()> {
    info!("Analyzing MCP capability override warnings");

    let capability_files = discover_capability_files(config).await?;
    let mut total_warnings = 0;
    let mut external_mcp_tools = 0;
    let mut tools_with_capabilities = 0;

    // Create enhancement config from main config
    let enhancement_config = create_enhancement_config_from_main(config);

    println!("MCP Capability Override Warnings");
    println!("================================");
    
    if !enhancement_config.enable_sampling_enhancement && !enhancement_config.enable_elicitation_enhancement {
        println!("✅ No local enhancement features are enabled, no capability override warnings.");
        return Ok(());
    }

    for file_path in &capability_files {
        let capability_file = load_capability_file(file_path)?;
        let mut file_warnings = Vec::new();
        let mut file_external_tools = 0;
        let mut file_tools_with_caps = 0;

        for tool in &capability_file.tools {
            // Check if tool is from external MCP
            if is_external_mcp_tool(tool) {
                file_external_tools += 1;
                
                // Check if tool has original capabilities
                let (has_sampling, has_elicitation) = has_original_mcp_capabilities(tool);
                
                if has_sampling || has_elicitation {
                    file_tools_with_caps += 1;
                }
                
                // Generate warnings for this tool
                let warnings = would_overwrite_mcp_capabilities(tool, &enhancement_config);
                if !warnings.is_empty() {
                    file_warnings.extend(warnings.into_iter().map(|w| (tool.name.clone(), w)));
                }
            }
        }

        external_mcp_tools += file_external_tools;
        tools_with_capabilities += file_tools_with_caps;
        total_warnings += file_warnings.len();

        if !file_warnings.is_empty() || (detailed && file_external_tools > 0) {
            println!("\nFile: {}", file_path.display());
            println!("  External MCP tools: {}", file_external_tools);
            println!("  Tools with original capabilities: {}", file_tools_with_caps);
            println!("  Capability override warnings: {}", file_warnings.len());
            
            if detailed {
                for tool in &capability_file.tools {
                    if is_external_mcp_tool(tool) {
                        let (has_sampling, has_elicitation) = has_original_mcp_capabilities(tool);
                        let routing_type = tool.routing.r#type.as_str();
                        
                        println!("    🔗 {} ({})", tool.name, routing_type);
                        if has_sampling {
                            println!("        ✅ Has original sampling capabilities");
                        }
                        if has_elicitation {
                            println!("        ✅ Has original elicitation capabilities");
                        }
                        if !has_sampling && !has_elicitation {
                            println!("        ❌ No original MCP 2025-06-18 capabilities detected");
                        }
                    }
                }
            }
            
            for (tool_name, warning) in &file_warnings {
                println!("    ⚠️  {}: {}", tool_name, warning);
            }
        }
    }

    println!("\nOverall Summary");
    println!("===============");
    println!("Total external MCP tools: {}", external_mcp_tools);
    println!("Tools with original capabilities: {}", tools_with_capabilities);
    println!("Capability override warnings: {}", total_warnings);
    
    if total_warnings > 0 {
        println!("\n💡 Recommendations:");
        println!("   • Consider disabling local enhancement for tools with original MCP capabilities");
        println!("   • External MCP servers should provide their own sampling/elicitation enhancements");
        println!("   • Use capability file metadata to track external MCP tool sources");
        
        if enhancement_config.enable_sampling_enhancement {
            println!("   • Disable sampling enhancement: Set smart_discovery.enable_sampling = false in config");
        }
        if enhancement_config.enable_elicitation_enhancement {
            println!("   • Disable elicitation enhancement: Set smart_discovery.enable_elicitation = false in config");
        }
    } else {
        println!("✅ No capability override issues detected");
    }

    Ok(())
}

/// Create enhancement config from main config (mirrors logic from ToolEnhancementService)
fn create_enhancement_config_from_main(config: &Config) -> ToolEnhancementConfig {
    config.smart_discovery
        .as_ref()
        .and_then(|sd| {
            // Check if sampling/elicitation are enabled via smart discovery config
            let sampling_enabled = sd.enable_sampling.unwrap_or(false);
            let elicitation_enabled = sd.enable_elicitation.unwrap_or(false);
            
            if sampling_enabled || elicitation_enabled {
                Some(ToolEnhancementConfig {
                    enable_sampling_enhancement: sampling_enabled,
                    enable_elicitation_enhancement: elicitation_enabled,
                    ..ToolEnhancementConfig::default()
                })
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            // Check main config sections
            let sampling_enabled = config.sampling.as_ref().map(|s| s.enabled).unwrap_or(false);
            let elicitation_enabled = config.elicitation.as_ref().map(|e| e.enabled).unwrap_or(false);
            
            ToolEnhancementConfig {
                enable_sampling_enhancement: sampling_enabled,
                enable_elicitation_enhancement: elicitation_enabled,
                ..ToolEnhancementConfig::default()
            }
        })
}

/// Check if a tool comes from external/remote MCP server (mirrors logic from ToolEnhancementService)
fn is_external_mcp_tool(tool: &ToolDefinition) -> bool {
    matches!(tool.routing.r#type.as_str(), "external_mcp" | "websocket")
}

/// Check if a tool has original sampling/elicitation capabilities from external MCP server
fn has_original_mcp_capabilities(tool: &ToolDefinition) -> (bool, bool) {
    let has_sampling = tool.annotations.as_ref()
        .and_then(|ann| ann.get("has_sampling_capability"))
        .map(|v| v == "true")
        .unwrap_or(false);
        
    let has_elicitation = tool.annotations.as_ref()
        .and_then(|ann| ann.get("has_elicitation_capability"))
        .map(|v| v == "true")
        .unwrap_or(false);
        
    (has_sampling, has_elicitation)
}

/// Check if we would be overwriting original MCP capabilities (mirrors logic from ToolEnhancementService)
fn would_overwrite_mcp_capabilities(tool: &ToolDefinition, config: &ToolEnhancementConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    
    if !is_external_mcp_tool(tool) {
        return warnings;
    }
    
    let (has_sampling, has_elicitation) = has_original_mcp_capabilities(tool);
    
    if has_sampling && config.enable_sampling_enhancement {
        warnings.push(format!(
            "Tool '{}' has original sampling capabilities from external MCP server but local sampling enhancement is enabled", 
            tool.name
        ));
    }
    
    if has_elicitation && config.enable_elicitation_enhancement {
        warnings.push(format!(
            "Tool '{}' has original elicitation capabilities from external MCP server but local elicitation enhancement is enabled", 
            tool.name
        ));
    }
    
    warnings
}
