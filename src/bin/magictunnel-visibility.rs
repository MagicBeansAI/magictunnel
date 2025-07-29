//! Visibility Management CLI
//!
//! This CLI tool provides commands for managing tool visibility in capability files.
//! It allows you to hide/show tools globally, per-file, or individually.

use clap::{Parser, Subcommand};
use magictunnel::config::Config;
use magictunnel::registry::types::CapabilityFile;
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
            if capability_file.get_tool(tool_name).is_some() {
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
            if capability_file.get_tool(tool_name).is_some() {
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

    serde_yaml::from_str(&content).map_err(|e| {
        ProxyError::registry(format!("Failed to parse YAML file {}: {}", path.display(), e))
    })
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
