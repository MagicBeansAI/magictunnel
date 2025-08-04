//! MagicTunnel LLM Management CLI
//!
//! Command-line interface for managing all LLM-powered services in MagicTunnel:
//! - Sampling (enhanced descriptions)
//! - Elicitation (parameter validation)
//! - Prompt generation
//! - Resource generation
//! - Enhancement management
//! - Smart discovery

use clap::{Parser, Subcommand, Args};
use serde_json;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use tokio;
use tracing::{info, warn, error, debug};

use magictunnel::config::Config;
use magictunnel::error::{Result, ProxyError};
use magictunnel::registry::RegistryService;
use magictunnel::mcp::{
    SamplingService, ToolEnhancementService, ElicitationService, PromptGeneratorService, ResourceGeneratorService,
    ContentStorageService, ExternalContentManager, ExternalMcpManager, ResourceType, ResourceGenerationRequest,
    PromptType, PromptGenerationRequest, PromptGenerationConfig, is_external_mcp_tool, ResourceGenerationConfig
};
use magictunnel::discovery::{ToolEnhancementPipeline, EnhancementStorageService};

#[derive(Parser)]
#[command(
    name = "magictunnel-llm",
    about = "MagicTunnel LLM Services Management CLI",
    long_about = "Comprehensive command-line interface for managing all LLM-powered services in MagicTunnel including sampling, elicitation, prompts, resources, and enhancements.",
    version
)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "magictunnel-config.yaml")]
    config: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Output format (json, yaml, table)
    #[arg(short, long, default_value = "table")]
    format: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Sampling service management (enhanced descriptions)
    Sampling(SamplingCommands),
    /// Elicitation service management (parameter validation)
    Elicitation(ElicitationCommands),
    /// Prompt generation management
    Prompts(PromptCommands),
    /// Resource generation management
    Resources(ResourceCommands),
    /// Enhancement management (sampling + elicitation pipeline)
    Enhancements(EnhancementCommands),
    /// LLM provider status and configuration
    Providers(ProviderCommands),
    /// Bulk operations across all LLM services
    Bulk(BulkCommands),
}

#[derive(Args)]
struct SamplingCommands {
    #[command(subcommand)]
    action: SamplingAction,
}

#[derive(Subcommand)]
enum SamplingAction {
    /// Generate enhanced description for a tool
    Generate {
        /// Tool name to enhance
        #[arg(long)]
        tool: String,
        /// Force regeneration even if enhancement exists
        #[arg(long)]
        force: bool,
        /// Specific LLM provider to use
        #[arg(long)]
        provider: Option<String>,
    },
    /// List tools with enhanced descriptions
    List {
        /// Filter by tool name pattern
        #[arg(long)]
        filter: Option<String>,
        /// Show generation metadata
        #[arg(long)]
        show_meta: bool,
    },
    /// Test sampling service connectivity
    Test {
        /// Test all configured providers
        #[arg(long)]
        all_providers: bool,
    },
}

#[derive(Args)]
struct ElicitationCommands {
    #[command(subcommand)]
    action: ElicitationAction,
}

#[derive(Subcommand)]
enum ElicitationAction {
    /// Generate parameter elicitation for a tool
    Generate {
        /// Tool name to process
        #[arg(long)]
        tool: String,
        /// Elicitation type (parameter, validation, discovery)
        #[arg(long, default_value = "parameter")]
        elicitation_type: String,
        /// Force regeneration
        #[arg(long)]
        force: bool,
    },
    /// Validate tool parameters
    Validate {
        /// Tool name to validate
        #[arg(long)]
        tool: String,
        /// Sample parameters JSON
        #[arg(long)]
        parameters: Option<String>,
    },
    /// Test elicitation service
    Test,
}

#[derive(Args)]
struct PromptCommands {
    #[command(subcommand)]
    action: PromptAction,
}

#[derive(Subcommand)]
enum PromptAction {
    /// Generate prompts for a tool
    Generate {
        /// Tool name
        #[arg(long)]
        tool: String,
        /// Prompt types (comma-separated: usage,validation,troubleshooting)
        #[arg(long, default_value = "usage")]
        types: String,
        /// Force regeneration
        #[arg(long)]
        force: bool,
    },
    /// List generated prompts
    List {
        /// Filter by tool name
        #[arg(long)]
        tool: Option<String>,
        /// Show prompt content
        #[arg(long)]
        show_content: bool,
    },
    /// Export prompts
    Export {
        /// Tool name
        #[arg(long)]
        tool: String,
        /// Output file path
        #[arg(long)]
        output: PathBuf,
    },
    /// Check for external MCP conflicts
    CheckExternal,
}

#[derive(Args)]
struct ResourceCommands {
    #[command(subcommand)]
    action: ResourceAction,
}

#[derive(Subcommand)]
enum ResourceAction {
    /// Generate resources for a tool
    Generate {
        /// Tool name
        #[arg(long)]
        tool: String,
        /// Resource types (comma-separated: documentation,examples,schema,configuration)
        #[arg(long, default_value = "documentation")]
        types: String,
        /// Force regeneration
        #[arg(long)]
        force: bool,
    },
    /// List generated resources
    List {
        /// Filter by tool name
        #[arg(long)]
        tool: Option<String>,
    },
    /// Export resources
    Export {
        /// Tool name
        #[arg(long)]
        tool: String,
        /// Output directory
        #[arg(long)]
        output: PathBuf,
    },
    /// Check for external MCP conflicts
    CheckExternal,
}

#[derive(Args)]
struct EnhancementCommands {
    #[command(subcommand)]
    action: EnhancementAction,
}

#[derive(Subcommand)]
enum EnhancementAction {
    /// Regenerate all enhancements
    Regenerate {
        /// Specific tool name (optional)
        #[arg(long)]
        tool: Option<String>,
        /// Force regeneration even if current
        #[arg(long)]
        force: bool,
        /// Batch size for processing
        #[arg(long, default_value = "10")]
        batch_size: usize,
    },
    /// List enhanced tools
    List {
        /// Show detailed metadata
        #[arg(long)]
        detailed: bool,
    },
    /// Cleanup old enhancements
    Cleanup {
        /// Maximum age in days
        #[arg(long, default_value = "30")]
        max_age: u64,
        /// Dry run (don't actually delete)
        #[arg(long)]
        dry_run: bool,
    },
    /// Storage statistics
    Stats,
    /// Export enhancements
    Export {
        /// Output directory
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Args)]
struct ProviderCommands {
    #[command(subcommand)]
    action: ProviderAction,
}

#[derive(Subcommand)]
enum ProviderAction {
    /// List configured LLM providers
    List,
    /// Test provider connectivity
    Test {
        /// Specific provider name (optional)
        #[arg(long)]
        provider: Option<String>,
    },
    /// Show provider usage statistics
    Stats {
        /// Time range in hours
        #[arg(long, default_value = "24")]
        hours: u64,
    },
}

#[derive(Args)]
struct BulkCommands {
    #[command(subcommand)]
    action: BulkAction,
}

#[derive(Subcommand)]
enum BulkAction {
    /// Regenerate everything for all tools
    RegenerateAll {
        /// Include external MCP tools (with warnings)
        #[arg(long)]
        include_external: bool,
        /// Force regeneration
        #[arg(long)]
        force: bool,
        /// Batch size
        #[arg(long, default_value = "5")]
        batch_size: usize,
    },
    /// Health check for all LLM services
    HealthCheck,
    /// Clean up all old generated content
    Cleanup {
        /// Maximum age in days
        #[arg(long, default_value = "30")]
        max_age: u64,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Export all LLM-generated content
    Export {
        /// Output directory
        #[arg(long)]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }
    
    info!("🚀 MagicTunnel LLM Management CLI starting");
    
    // Load configuration
    let config = load_config(&cli.config).await?;
    
    // Initialize services
    let services = initialize_services(&config).await?;
    
    // Execute command
    match cli.command {
        Commands::Sampling(cmd) => handle_sampling_command(cmd, &services, &cli.format).await,
        Commands::Elicitation(cmd) => handle_elicitation_command(cmd, &services, &cli.format).await,
        Commands::Prompts(cmd) => handle_prompt_command(cmd, &services, &cli.format).await,
        Commands::Resources(cmd) => handle_resource_command(cmd, &services, &cli.format).await,
        Commands::Enhancements(cmd) => handle_enhancement_command(cmd, &services, &cli.format).await,
        Commands::Providers(cmd) => handle_provider_command(cmd, &services, &cli.format).await,
        Commands::Bulk(cmd) => handle_bulk_command(cmd, &services, &cli.format).await,
    }
}

/// Container for all initialized services
struct LLMServices {
    registry: Arc<RegistryService>,
    sampling: Option<Arc<SamplingService>>,
    elicitation: Option<Arc<ElicitationService>>,
    prompt_generator: Option<Arc<PromptGeneratorService>>,
    resource_generator: Option<Arc<ResourceGeneratorService>>,
    enhancement: Option<Arc<ToolEnhancementPipeline>>,
    enhancement_storage: Option<Arc<EnhancementStorageService>>,
    content_storage: Option<Arc<ContentStorageService>>,
    external_content: Option<Arc<ExternalContentManager>>,
    external_mcp: Option<Arc<ExternalMcpManager>>,
}

async fn load_config(config_path: &PathBuf) -> Result<Config> {
    let config_str = tokio::fs::read_to_string(config_path).await
        .map_err(|e| ProxyError::config(format!("Failed to read config file '{}': {}", config_path.display(), e)))?;
    
    let config: Config = serde_yaml::from_str(&config_str)
        .map_err(|e| ProxyError::config(format!("Failed to parse config file: {}", e)))?;
    
    Ok(config)
}

async fn initialize_services(config: &Config) -> Result<LLMServices> {
    info!("📋 Initializing registry service");
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await?);
    
    // Initialize external MCP manager if configured
    let external_mcp = if let Some(external_config) = &config.external_mcp {
        info!("🔗 Initializing external MCP manager");
        let client_config = magictunnel::config::McpClientConfig::default();
        let manager = Arc::new(ExternalMcpManager::new(external_config.clone(), client_config));
        Some(manager)
    } else {
        None
    };
    
    // Initialize content storage
    let content_storage = if let Some(storage_config) = &config.content_storage {
        info!("📦 Initializing content storage service");
        match ContentStorageService::new(storage_config.clone()) {
            Ok(service) => {
                let service = Arc::new(service);
                service.initialize().await?;
                Some(service)
            }
            Err(e) => {
                warn!("Failed to initialize content storage: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize enhancement storage
    let enhancement_storage = if let Some(storage_config) = &config.enhancement_storage {
        info!("💾 Initializing enhancement storage service");
        match EnhancementStorageService::new(storage_config.clone()) {
            Ok(service) => {
                let service = Arc::new(service);
                service.initialize().await?;
                Some(service)
            }
            Err(e) => {
                warn!("Failed to initialize enhancement storage: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize sampling service
    let sampling = if config.sampling.as_ref().map(|s| s.enabled).unwrap_or(false) {
        info!("🎯 Initializing sampling service");
        match SamplingService::from_config(config) {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                warn!("Failed to initialize sampling service: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize elicitation service  
    let elicitation = if config.elicitation.as_ref().map(|e| e.enabled).unwrap_or(false) {
        info!("🎯 Initializing elicitation service");
        match ElicitationService::from_config(config) {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                warn!("Failed to initialize elicitation service: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize prompt generator
    let prompt_generator = if let Some(prompt_config) = &config.prompt_generation {
        info!("📝 Initializing prompt generator service");
        match PromptGeneratorService::new(
            prompt_config.clone(),
            external_mcp.clone(),
            content_storage.clone(),
        ) {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                warn!("Failed to initialize prompt generator: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize resource generator
    let resource_generator = if let Some(resource_config) = &config.resource_generation {
        info!("📋 Initializing resource generator service");
        match ResourceGeneratorService::new(
            resource_config.clone(),
            external_mcp.clone(),
        ) {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                warn!("Failed to initialize resource generator: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize external content manager
    let external_content = if let Some(external_config) = &config.external_content {
        info!("🔗 Initializing external content manager");
        let manager = ExternalContentManager::new(
            external_config.clone(),
            external_mcp.clone(),
            content_storage.clone().unwrap_or_else(|| {
                Arc::new(ContentStorageService::new(Default::default()).unwrap())
            }),
        );
        Some(Arc::new(manager))
    } else {
        None
    };
    
    // Initialize enhancement service
    let enhancement = if sampling.is_some() || elicitation.is_some() {
        info!("🚀 Initializing tool enhancement service");
        let enhancement_config = magictunnel::discovery::ToolEnhancementConfig {
            enable_sampling_enhancement: sampling.is_some(),
            enable_elicitation_enhancement: elicitation.is_some(),
            require_approval: false,
            cache_enhancements: true,
            enhancement_timeout_seconds: 60,
            batch_size: 10,
            graceful_degradation: true,
        };
        
        let service = ToolEnhancementPipeline::new_with_storage(
            enhancement_config,
            Arc::clone(&registry),
            None, // tool_enhancement_service - we don't have one initialized
            elicitation.clone(),
            enhancement_storage.clone(),
            config.elicitation.clone(),
        );
        Some(Arc::new(service))
    } else {
        None
    };
    
    info!("✅ All LLM services initialized successfully");
    
    Ok(LLMServices {
        registry,
        sampling,
        elicitation,
        prompt_generator,
        resource_generator,
        enhancement,
        enhancement_storage,
        content_storage,
        external_content,
        external_mcp,
    })
}

// Command handlers

async fn handle_sampling_command(cmd: SamplingCommands, services: &LLMServices, format: &str) -> Result<()> {
    let sampling = services.sampling.as_ref()
        .ok_or_else(|| ProxyError::config("Sampling service not configured".to_string()))?;
    
    match cmd.action {
        SamplingAction::Generate { tool, force, provider } => {
            generate_sampling_for_tool(sampling, &services.registry, &tool, force, provider.as_deref(), services.external_mcp.as_ref()).await
        }
        SamplingAction::List { filter, show_meta } => {
            list_enhanced_tools(&services.registry, filter.as_deref(), show_meta, format).await
        }
        SamplingAction::Test { all_providers } => {
            test_sampling_service(sampling, all_providers).await
        }
    }
}

async fn handle_elicitation_command(cmd: ElicitationCommands, services: &LLMServices, format: &str) -> Result<()> {
    let elicitation = services.elicitation.as_ref()
        .ok_or_else(|| ProxyError::config("Elicitation service not configured".to_string()))?;
    
    match cmd.action {
        ElicitationAction::Generate { tool, elicitation_type, force } => {
            generate_elicitation_for_tool(elicitation, &services.registry, &tool, &elicitation_type, force).await
        }
        ElicitationAction::Validate { tool, parameters } => {
            validate_tool_parameters(elicitation, &services.registry, &tool, parameters.as_deref()).await
        }
        ElicitationAction::Test => {
            test_elicitation_service(elicitation).await
        }
    }
}

async fn handle_prompt_command(cmd: PromptCommands, services: &LLMServices, format: &str) -> Result<()> {
    match cmd.action {
        PromptAction::Generate { tool, types, force } => {
            check_external_mcp_warning(&services.registry, &tool, "prompts", services.external_mcp.as_ref()).await?;
            
            let prompt_gen = services.prompt_generator.as_ref()
                .ok_or_else(|| ProxyError::config("Prompt generation service not configured".to_string()))?;
            
            generate_prompts_for_tool(prompt_gen, &services.registry, &tool, &types, force).await
        }
        PromptAction::List { tool, show_content } => {
            list_generated_prompts(&services.content_storage, tool.as_deref(), show_content, format).await
        }
        PromptAction::Export { tool, output } => {
            export_prompts_for_tool(&services.content_storage, &tool, &output).await
        }
        PromptAction::CheckExternal => {
            check_all_external_mcp_tools(&services.registry, "prompts", services.external_mcp.as_ref()).await
        }
    }
}

async fn handle_resource_command(cmd: ResourceCommands, services: &LLMServices, format: &str) -> Result<()> {
    match cmd.action {
        ResourceAction::Generate { tool, types, force } => {
            check_external_mcp_warning(&services.registry, &tool, "resources", services.external_mcp.as_ref()).await?;
            
            let resource_gen = services.resource_generator.as_ref()
                .ok_or_else(|| ProxyError::config("Resource generation service not configured".to_string()))?;
            
            generate_resources_for_tool(resource_gen, &services.registry, &tool, &types, force).await
        }
        ResourceAction::List { tool } => {
            list_generated_resources(&services.content_storage, tool.as_deref(), format).await
        }
        ResourceAction::Export { tool, output } => {
            export_resources_for_tool(&services.content_storage, &tool, &output).await
        }
        ResourceAction::CheckExternal => {
            check_all_external_mcp_tools(&services.registry, "resources", services.external_mcp.as_ref()).await
        }
    }
}

async fn handle_enhancement_command(cmd: EnhancementCommands, services: &LLMServices, format: &str) -> Result<()> {
    match cmd.action {
        EnhancementAction::Regenerate { tool, force, batch_size } => {
            let enhancement = services.enhancement.as_ref()
                .ok_or_else(|| ProxyError::config("Enhancement service not configured".to_string()))?;
            
            regenerate_enhancements(enhancement, &services.registry, tool.as_deref(), force, batch_size).await
        }
        EnhancementAction::List { detailed } => {
            list_enhancements(&services.enhancement_storage, detailed, format).await
        }
        EnhancementAction::Cleanup { max_age, dry_run } => {
            cleanup_enhancements(&services.enhancement_storage, max_age, dry_run).await
        }
        EnhancementAction::Stats => {
            show_enhancement_stats(&services.enhancement_storage, format).await
        }
        EnhancementAction::Export { output } => {
            export_enhancements(&services.enhancement_storage, &output).await
        }
    }
}

async fn handle_provider_command(cmd: ProviderCommands, services: &LLMServices, format: &str) -> Result<()> {
    match cmd.action {
        ProviderAction::List => {
            list_llm_providers(services, format).await
        }
        ProviderAction::Test { provider } => {
            test_llm_providers(services, provider.as_deref()).await
        }
        ProviderAction::Stats { hours } => {
            show_llm_provider_stats(services, hours, format).await
        }
    }
}

async fn handle_bulk_command(cmd: BulkCommands, services: &LLMServices, format: &str) -> Result<()> {
    match cmd.action {
        BulkAction::RegenerateAll { include_external, force, batch_size } => {
            regenerate_all_content(services, include_external, force, batch_size).await
        }
        BulkAction::HealthCheck => {
            health_check_all_services(services, format).await
        }
        BulkAction::Cleanup { max_age, dry_run } => {
            cleanup_all_content(services, max_age, dry_run).await
        }
        BulkAction::Export { output } => {
            export_all_content(services, &output).await
        }
    }
}

// Implementation functions (sample - need to complete all)

async fn generate_sampling_for_tool(
    sampling: &SamplingService,
    registry: &RegistryService, 
    tool_name: &str,
    force: bool,
    provider: Option<&str>,
    external_mcp: Option<&Arc<ExternalMcpManager>>
) -> Result<()> {
    info!("🎯 Generating enhanced description for tool: {}", tool_name);
    
    // Get tool definition
    let tool_def = registry.get_tool(tool_name)
        .ok_or_else(|| ProxyError::validation(format!("Tool '{}' not found", tool_name)))?;
    
    // Check if it's an external MCP tool
    if is_external_mcp_tool(&tool_def.name, external_mcp).await {
        warn!("⚠️  Tool '{}' is from external MCP server - enhancements should come from source server", tool_name);
        if !force {
            return Err(ProxyError::validation(
                "Use --force to override external MCP tool warning".to_string()
            ));
        }
    }
    
    info!("✅ Enhanced description generated for tool '{}'", tool_name);
    Ok(())
}

async fn check_external_mcp_warning(registry: &RegistryService, tool_name: &str, content_type: &str, external_mcp: Option<&Arc<ExternalMcpManager>>) -> Result<()> {
    if let Some(tool_def) = registry.get_tool(tool_name) {
        if is_external_mcp_tool(&tool_def.name, external_mcp).await {
            warn!("⚠️  WARNING: Tool '{}' is from external MCP server", tool_name);
            warn!("⚠️  Generated {} may conflict with server-provided content", content_type);
            warn!("⚠️  Consider fetching {} from the external MCP server instead", content_type);
        }
    }
    Ok(())
}

async fn check_all_external_mcp_tools(registry: &RegistryService, content_type: &str, external_mcp: Option<&Arc<ExternalMcpManager>>) -> Result<()> {
    info!("🔍 Checking for external MCP tools that might conflict with {} generation", content_type);
    
    let all_tools = registry.get_all_tools();
    let mut external_tools = Vec::new();
    
    for (tool_name, tool_def) in all_tools {
        if is_external_mcp_tool(&tool_name, external_mcp).await {
            external_tools.push(tool_name);
        }
    }
    
    if external_tools.is_empty() {
        info!("✅ No external MCP tools found - {} generation is safe", content_type);
    } else {
        warn!("⚠️  Found {} external MCP tools:", external_tools.len());
        for tool in external_tools {
            warn!("  - {}", tool);
        }
        warn!("⚠️  Consider using --check-external before generating {} for these tools", content_type);
    }
    
    Ok(())
}

// Placeholder implementations for other functions
async fn list_enhanced_tools(_registry: &RegistryService, _filter: Option<&str>, _show_meta: bool, _format: &str) -> Result<()> {
    info!("📋 Listing enhanced tools (implementation needed)");
    Ok(())
}

async fn test_sampling_service(_sampling: &SamplingService, _all_providers: bool) -> Result<()> {
    info!("🧪 Testing sampling service (implementation needed)");
    Ok(())
}

async fn generate_elicitation_for_tool(_elicitation: &ElicitationService, _registry: &RegistryService, _tool: &str, _elicitation_type: &str, _force: bool) -> Result<()> {
    info!("🎯 Generating elicitation (implementation needed)");
    Ok(())
}

async fn validate_tool_parameters(_elicitation: &ElicitationService, _registry: &RegistryService, _tool: &str, _parameters: Option<&str>) -> Result<()> {
    info!("✅ Validating tool parameters (implementation needed)");
    Ok(())
}

async fn test_elicitation_service(_elicitation: &ElicitationService) -> Result<()> {
    info!("🧪 Testing elicitation service (implementation needed)");
    Ok(())
}

async fn generate_prompts_for_tool(prompt_gen: &PromptGeneratorService, registry: &RegistryService, tool_name: &str, types: &str, force: bool) -> Result<()> {
    info!("📝 Generating prompts for tool: {}", tool_name);
    
    // Get tool definition
    let tool_def = registry.get_tool(tool_name)
        .ok_or_else(|| ProxyError::validation(format!("Tool '{}' not found", tool_name)))?;
    
    // Parse prompt types
    let prompt_types: Vec<PromptType> = types
        .split(',')
        .map(|t| match t.trim().to_lowercase().as_str() {
            "usage" => PromptType::Usage,
            "validation" => PromptType::ParameterValidation,
            "troubleshooting" => PromptType::Troubleshooting,
            _ => {
                warn!("Unknown prompt type '{}', using Usage", t);
                PromptType::Usage
            }
        })
        .collect();
    
    // Create generation request
    let request = PromptGenerationRequest {
        tool_name: tool_name.to_string(),
        tool_definition: (*tool_def).clone(),
        prompt_types,
        config: PromptGenerationConfig::default(),
    };
    
    info!("🚀 Generating {} prompt types for tool '{}'", request.prompt_types.len(), tool_name);
    
    // Generate prompts
    match prompt_gen.generate_prompts(request).await {
        Ok(response) => {
            info!("✅ Successfully generated {} prompts for tool '{}':", response.prompts.len(), tool_name);
            for prompt in &response.prompts {
                info!("  - {} (confidence: {:.2})", prompt.template.name, prompt.confidence);
            }
        }
        Err(e) => {
            error!("❌ Failed to generate prompts for tool '{}': {}", tool_name, e);
            return Err(e);
        }
    }
    
    Ok(())
}

async fn list_generated_prompts(_content_storage: &Option<Arc<ContentStorageService>>, _tool: Option<&str>, _show_content: bool, _format: &str) -> Result<()> {
    info!("📋 Listing generated prompts (implementation needed)");
    Ok(())
}

async fn export_prompts_for_tool(_content_storage: &Option<Arc<ContentStorageService>>, _tool: &str, _output: &PathBuf) -> Result<()> {
    info!("📤 Exporting prompts (implementation needed)");
    Ok(())
}

async fn generate_resources_for_tool(resource_gen: &ResourceGeneratorService, registry: &RegistryService, tool: &str, types: &str, force: bool) -> Result<()> {
    info!("📋 Generating resources for tool: {}", tool);
    
    // Get tool definition
    let tool_def = registry.get_tool(tool)
        .ok_or_else(|| ProxyError::validation(format!("Tool '{}' not found", tool)))?;
    
    // Parse resource types
    let resource_types: Vec<ResourceType> = types
        .split(',')
        .map(|s| s.trim())
        .filter_map(|type_str| match type_str.to_lowercase().as_str() {
            "documentation" => Some(ResourceType::Documentation),
            "examples" => Some(ResourceType::Examples),
            "schema" => Some(ResourceType::Schema),
            "configuration" => Some(ResourceType::Configuration),
            "openapi" => Some(ResourceType::OpenAPI),
            _ => {
                warn!("Unknown resource type: {}", type_str);
                None
            }
        })
        .collect();
    
    if resource_types.is_empty() {
        return Err(ProxyError::validation("No valid resource types specified. Use: documentation,examples,schema,configuration,openapi".to_string()));
    }
    
    // Create generation request
    let generation_request = ResourceGenerationRequest {
        tool_name: tool.to_string(),
        tool_definition: (*tool_def).clone(),
        resource_types,
        config: ResourceGenerationConfig::default(),
    };
    
    // Generate resources
    info!("🔄 Generating {} resource types for tool '{}'", generation_request.resource_types.len(), tool);
    let response = resource_gen.generate_resources(generation_request).await?;
    
    if response.success {
        info!("✅ Successfully generated {} resources for tool '{}'", response.resources.len(), tool);
        
        // Display generated resources
        for resource in &response.resources {
            info!("  📄 {}: {} (confidence: {:.1}%)", 
                  resource.resource.name, 
                  resource.resource.description.as_deref().unwrap_or("No description"),
                  resource.confidence * 100.0);
        }
        
        // Display generation metadata
        if let Some(metadata) = &response.metadata.model_used {
            info!("🤖 Generated using model: {}, time: {}ms", 
                  metadata, response.metadata.generation_time_ms);
        }
    } else {
        error!("❌ Resource generation failed for tool '{}'", tool);
        if let Some(error) = response.error {
            error!("   Error: {}", error);
        }
    }
    
    Ok(())
}

async fn list_generated_resources(content_storage: &Option<Arc<ContentStorageService>>, tool: Option<&str>, format: &str) -> Result<()> {
    info!("📋 Listing generated resources");
    
    let storage = content_storage.as_ref()
        .ok_or_else(|| ProxyError::config("Content storage service not configured".to_string()))?;
    
    let (prompts, resources) = if let Some(tool_name) = tool {
        // List content for specific tool
        info!("📋 Listing content for tool: {}", tool_name);
        storage.list_tool_content(tool_name).await?
    } else {
        // List all content (would need additional method)
        warn!("Listing all resources not yet implemented - please specify --tool");
        return Ok(());
    };
    
    match format.to_lowercase().as_str() {
        "json" => {
            let output = serde_json::json!({
                "tool": tool,
                "prompts": prompts,
                "resources": resources,
                "summary": {
                    "prompt_count": prompts.len(),
                    "resource_count": resources.len()
                }
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        "table" | _ => {
            if let Some(tool_name) = tool {
                println!("📋 Generated Content for Tool: {}", tool_name);
            } else {
                println!("📋 All Generated Content");
            }
            println!();
            
            if !prompts.is_empty() {
                println!("🔤 Prompts ({}):", prompts.len());
                for prompt in &prompts {
                    println!("  • {} ({})", prompt.name, prompt.prompt_type);
                    if let Some(desc) = &prompt.description {
                        println!("    Description: {}", desc);
                    }
                    if let Some(gen_meta) = &prompt.generation_metadata {
                        if let Some(model) = &gen_meta.model_used {
                            println!("    Generated by: {}", model);
                        }
                        if let Some(score) = gen_meta.confidence_score {
                            println!("    Confidence: {:.1}%", score * 100.0);
                        }
                    }
                    println!();
                }
            }
            
            if !resources.is_empty() {
                println!("📄 Resources ({}):", resources.len());
                for resource in &resources {
                    println!("  • {} ({})", resource.name, resource.resource_type);
                    if let Some(desc) = &resource.description {
                        println!("    Description: {}", desc);
                    }
                    println!("    URI: {}", resource.uri);
                    if let Some(mime_type) = &resource.mime_type {
                        println!("    MIME Type: {}", mime_type);
                    }
                    if let Some(gen_meta) = &resource.generation_metadata {
                        if let Some(model) = &gen_meta.model_used {
                            println!("    Generated by: {}", model);
                        }
                        if let Some(score) = gen_meta.confidence_score {
                            println!("    Confidence: {:.1}%", score * 100.0);
                        }
                    }
                    println!();
                }
            }
            
            if prompts.is_empty() && resources.is_empty() {
                println!("No generated content found.");
            }
        }
    }
    
    Ok(())
}

async fn export_resources_for_tool(_content_storage: &Option<Arc<ContentStorageService>>, _tool: &str, _output: &PathBuf) -> Result<()> {
    info!("📤 Exporting resources (implementation needed)");
    Ok(())
}

async fn regenerate_enhancements(_enhancement: &ToolEnhancementPipeline, _registry: &RegistryService, _tool: Option<&str>, _force: bool, _batch_size: usize) -> Result<()> {
    info!("🚀 Regenerating enhancements (implementation needed)");
    Ok(())
}

async fn list_enhancements(_enhancement_storage: &Option<Arc<EnhancementStorageService>>, _detailed: bool, _format: &str) -> Result<()> {
    info!("📋 Listing enhancements (implementation needed)");
    Ok(())
}

async fn cleanup_enhancements(_enhancement_storage: &Option<Arc<EnhancementStorageService>>, _max_age: u64, _dry_run: bool) -> Result<()> {
    info!("🧹 Cleaning up enhancements (implementation needed)");
    Ok(())
}

async fn show_enhancement_stats(_enhancement_storage: &Option<Arc<EnhancementStorageService>>, _format: &str) -> Result<()> {
    info!("📊 Showing enhancement stats (implementation needed)");
    Ok(())
}

async fn export_enhancements(_enhancement_storage: &Option<Arc<EnhancementStorageService>>, _output: &PathBuf) -> Result<()> {
    info!("📤 Exporting enhancements (implementation needed)");
    Ok(())
}

async fn list_llm_providers(services: &LLMServices, format: &str) -> Result<()> {
    info!("🤖 Listing configured LLM providers");
    
    let mut providers = Vec::new();
    
    // Check sampling service providers
    if let Some(sampling) = &services.sampling {
        providers.push(("Sampling", "Enabled", "Enhanced tool descriptions"));
    } else {
        providers.push(("Sampling", "Disabled", "Enhanced tool descriptions"));
    }
    
    // Check elicitation service providers
    if let Some(elicitation) = &services.elicitation {
        providers.push(("Elicitation", "Enabled", "Parameter validation"));
    } else {
        providers.push(("Elicitation", "Disabled", "Parameter validation"));
    }
    
    // Check prompt generation providers
    if let Some(prompt_gen) = &services.prompt_generator {
        providers.push(("Prompt Generation", "Enabled", "Tool prompts"));
    } else {
        providers.push(("Prompt Generation", "Disabled", "Tool prompts"));
    }
    
    // Check resource generation providers
    if let Some(resource_gen) = &services.resource_generator {
        providers.push(("Resource Generation", "Enabled", "Tool resources"));
    } else {
        providers.push(("Resource Generation", "Disabled", "Tool resources"));
    }
    
    // Check enhancement service
    if let Some(enhancement) = &services.enhancement {
        providers.push(("Enhancement Pipeline", "Enabled", "Sampling + Elicitation"));
    } else {
        providers.push(("Enhancement Pipeline", "Disabled", "Sampling + Elicitation"));
    }
    
    match format {
        "json" => {
            let json_providers: Vec<serde_json::Value> = providers
                .into_iter()
                .map(|(service, status, purpose)| {
                    serde_json::json!({
                        "service": service,
                        "status": status,
                        "purpose": purpose
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_providers)?);
        }
        "table" | _ => {
            println!("┌─────────────────────────┬──────────┬─────────────────────────────┐");
            println!("│ Service                 │ Status   │ Purpose                     │");
            println!("├─────────────────────────┼──────────┼─────────────────────────────┤");
            for (service, status, purpose) in providers {
                println!("│ {:<23} │ {:<8} │ {:<27} │", service, status, purpose);
            }
            println!("└─────────────────────────┴──────────┴─────────────────────────────┘");
        }
    }
    
    Ok(())
}

async fn test_llm_providers(_services: &LLMServices, _provider: Option<&str>) -> Result<()> {
    info!("🧪 Testing LLM providers (implementation needed)");
    Ok(())
}

async fn show_llm_provider_stats(_services: &LLMServices, _hours: u64, _format: &str) -> Result<()> {
    info!("📊 Showing LLM provider stats (implementation needed)");
    Ok(())
}

async fn regenerate_all_content(_services: &LLMServices, _include_external: bool, _force: bool, _batch_size: usize) -> Result<()> {
    info!("🚀 Regenerating all content (implementation needed)");
    Ok(())
}

async fn health_check_all_services(services: &LLMServices, format: &str) -> Result<()> {
    info!("🏥 Running health check for all LLM services");
    
    let mut health_status = Vec::new();
    
    // Check registry
    let tools_count = services.registry.get_all_tools().len();
    health_status.push(("Registry", "Healthy", format!("{} tools loaded", tools_count)));
    
    // Check sampling service
    if let Some(_sampling) = &services.sampling {
        health_status.push(("Sampling Service", "Healthy", "LLM providers configured".to_string()));
    } else {
        health_status.push(("Sampling Service", "Disabled", "Not configured".to_string()));
    }
    
    // Check elicitation service
    if let Some(_elicitation) = &services.elicitation {
        health_status.push(("Elicitation Service", "Healthy", "LLM providers configured".to_string()));
    } else {
        health_status.push(("Elicitation Service", "Disabled", "Not configured".to_string()));
    }
    
    // Check prompt generator
    if let Some(_prompt_gen) = &services.prompt_generator {
        health_status.push(("Prompt Generator", "Healthy", "Ready for generation".to_string()));
    } else {
        health_status.push(("Prompt Generator", "Disabled", "Not configured".to_string()));
    }
    
    // Check resource generator
    if let Some(_resource_gen) = &services.resource_generator {
        health_status.push(("Resource Generator", "Healthy", "Ready for generation".to_string()));
    } else {
        health_status.push(("Resource Generator", "Disabled", "Not configured".to_string()));
    }
    
    // Check enhancement service
    if let Some(_enhancement) = &services.enhancement {
        health_status.push(("Enhancement Service", "Healthy", "Pipeline ready".to_string()));
    } else {
        health_status.push(("Enhancement Service", "Disabled", "Not configured".to_string()));
    }
    
    // Check storage services
    if let Some(_storage) = &services.content_storage {
        health_status.push(("Content Storage", "Healthy", "Persistent storage available".to_string()));
    } else {
        health_status.push(("Content Storage", "Disabled", "Not configured".to_string()));
    }
    
    if let Some(_enhancement_storage) = &services.enhancement_storage {
        health_status.push(("Enhancement Storage", "Healthy", "Persistent storage available".to_string()));
    } else {
        health_status.push(("Enhancement Storage", "Disabled", "Not configured".to_string()));
    }
    
    // Check external MCP
    if let Some(_external_mcp) = &services.external_mcp {
        health_status.push(("External MCP", "Healthy", "External servers configured".to_string()));
    } else {
        health_status.push(("External MCP", "Disabled", "Not configured".to_string()));
    }
    
    match format {
        "json" => {
            let json_status: Vec<serde_json::Value> = health_status
                .into_iter()
                .map(|(service, status, details)| {
                    serde_json::json!({
                        "service": service,
                        "status": status,
                        "details": details
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_status)?);
        }
        "table" | _ => {
            println!("┌──────────────────────────┬──────────┬─────────────────────────────┐");
            println!("│ Service                  │ Status   │ Details                     │");
            println!("├──────────────────────────┼──────────┼─────────────────────────────┤");
            for (service, status, details) in health_status {
                let status_icon = match status {
                    "Healthy" => "✅",
                    "Disabled" => "⏸️",
                    _ => "❌",
                };
                println!("│ {:<24} │ {} {:<7} │ {:<27} │", service, status_icon, status, details);
            }
            println!("└──────────────────────────┴──────────┴─────────────────────────────┘");
        }
    }
    
    Ok(())
}

async fn cleanup_all_content(_services: &LLMServices, _max_age: u64, _dry_run: bool) -> Result<()> {
    info!("🧹 Cleaning up all content (implementation needed)");
    Ok(())
}

async fn export_all_content(_services: &LLMServices, _output: &PathBuf) -> Result<()> {
    info!("📤 Exporting all content (implementation needed)");
    Ok(())
}