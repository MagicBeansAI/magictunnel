use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::collections::HashMap;
use tracing::{info, error, warn, debug};
use serde_json::json;

mod auth;
mod config;
mod discovery;
mod error;
mod grpc;
mod mcp;
mod metrics;
mod openai;
mod registry;
mod routing;
mod security;
mod services;
mod startup;
mod supervisor;
mod tls;
mod web;

use config::Config;
use mcp::{McpServer, McpErrorCode, ExternalMcpIntegration};
use grpc::McpGrpcServer;
use services::{ServiceLoader, ServiceContainer};
use registry::types::ToolDefinition;
use std::sync::Arc;
use tonic::transport::Server;

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(about = env!("CARGO_PKG_DESCRIPTION"))]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Server host
    #[arg(long)]
    host: Option<String>,

    /// Server port
    #[arg(long)]
    port: Option<u16>,

    /// Run in stdio mode for MCP clients (Claude Desktop, Cursor)
    #[arg(long)]
    stdio: bool,

    /// Run as single-shot MCP client: read one request from stdin, process, return result, and exit
    #[arg(long)]
    mcp_client: bool,

    /// Discover local MCP capabilities once and exit
    #[arg(long)]
    discover_local: bool,

    /// Pre-generate embeddings for all enabled capabilities and exit
    #[arg(long)]
    pregenerate_embeddings: bool,

    /// Override capabilities directory path
    #[arg(long)]
    capabilities_dir: Option<PathBuf>,

    /// Override data directory path (for GraphQL schemas, OpenAPI specs, etc.)
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// Override working directory
    #[arg(long)]
    work_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(&cli.log_level)?;
    
    // Display startup banner
    startup::display_startup_banner(env!("CARGO_PKG_VERSION"));
    
    // Load configuration with full resolution
    let resolution = Config::load_with_resolution(
        Some(&cli.config),
        cli.host,
        cli.port,
    ).map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;
    
    let config = resolution.config.clone();
    
    // Display comprehensive startup information
    let startup_info = startup::StartupAdditionalInfo::new(
        config.server.host.clone(),
        config.server.port,
    );
    
    startup::StartupLogger::display_startup_info(
        &resolution,
        env!("CARGO_PKG_VERSION"),
        Some(&startup_info),
    );

    if cli.discover_local {
        // Run external MCP discovery once and exit
        info!("Running external MCP discovery");
        let mut external_integration = ExternalMcpIntegration::new(Arc::new(config));
        external_integration.start().await?;
        info!("External MCP discovery completed");
        return Ok(());
    } else if cli.pregenerate_embeddings {
        // Pre-generate embeddings for all enabled capabilities and exit
        info!("Pre-generating embeddings for all enabled capabilities");
        pregenerate_embeddings_and_exit(config).await?;
        return Ok(());
    } else if cli.mcp_client {
        // Run as single-shot MCP client
        info!("Running as single-shot MCP client");
        run_mcp_client_mode(config).await?;
        return Ok(());
    } else if cli.stdio {
        // Run in stdio mode for MCP clients like Claude Desktop and Cursor
        info!("Starting Magictunnel in stdio mode");
        run_stdio_mode(config).await?;
    } else {
        // Run in HTTP server mode with new service loading strategy
        run_http_server_mode(&resolution).await?;
    }
    
    Ok(())
}

/// Run in HTTP server mode with new service loading strategy
async fn run_http_server_mode(resolution: &config::ConfigResolution) -> Result<()> {
    info!("🚀 Starting MagicTunnel in HTTP server mode");
    
    let config = &resolution.config;
    let runtime_mode = resolution.get_runtime_mode();
    
    info!("HTTP server will bind to {}:{}", config.server.host, config.server.port);
    
    // Load services based on runtime mode
    let service_loading_start = std::time::Instant::now();
    let service_container = ServiceLoader::load_services(resolution).await?;
    let loading_time = service_loading_start.elapsed();
    
    // Get service loading summary
    let mut loading_summary = ServiceLoader::get_loading_summary(&service_container);
    loading_summary.loading_time_ms = loading_time.as_millis() as u64;
    
    // Log service loading results
    info!("📊 Service Loading Summary:");
    info!("   Mode: {}", loading_summary.runtime_mode);
    info!("   Total services: {}", loading_summary.total_services);
    info!("   Loading time: {}ms", loading_summary.loading_time_ms);
    info!("   Proxy services: {}", loading_summary.proxy_services.join(", "));
    if let Some(ref advanced) = loading_summary.advanced_services {
        if !advanced.is_empty() {
            info!("   Advanced services: {}", advanced.join(", "));
        }
    }
    
    // Create MCP server for main.rs to own and start
    // This avoids the Arc ownership issue since we get an owned server
    let mcp_server = service_container.create_mcp_server_for_main().await?;
    
    let registry = service_container.get_registry()
        .ok_or_else(|| anyhow::anyhow!("Registry not available from service container"))?;
    
    // Initialize gRPC server with registry (if enabled)
    let grpc_port = config.server.port + 1000;
    info!("gRPC server will bind to {}:{}", config.server.host, grpc_port);
    
    let grpc_server = McpGrpcServer::new(Arc::clone(registry));
    
    info!("Starting MagicTunnel servers...");
    
    // Start gRPC server in background task
    let grpc_addr = format!("{}:{}", config.server.host, grpc_port).parse()?;
    let grpc_host = config.server.host.clone();
    
    let _grpc_task = tokio::spawn(async move {
        info!("Starting gRPC server on {}", grpc_addr);
        
        // Import the generated service
        use grpc::mcp_service_server::McpServiceServer;
        
        let service = McpServiceServer::new(grpc_server);
        
        if let Err(e) = Server::builder()
            .add_service(service)
            .serve(grpc_addr)
            .await
        {
            error!("gRPC server failed: {}", e);
        }
    });
    
    info!("gRPC server started in background");
    
    // Start HTTP server in main thread (this will block until completion)
    info!("Starting HTTP server on {}:{}", config.server.host, config.server.port);
    
    // Create Arc for service container so it can be shared
    let service_container_arc = Arc::new(service_container);
    
    // Start the MCP server with service container reference and config resolution
    if let Err(e) = mcp_server.start_with_config_and_services(
        &config.server.host, 
        config.server.port, 
        config.server.tls.clone(),
        Some(service_container_arc.clone()),
        Some(Arc::new(resolution.clone()))
    ).await {
        error!("HTTP server failed: {}", e);
        
        // Attempt graceful shutdown of services
        if let Some(container) = Arc::try_unwrap(service_container_arc).ok() {
            if let Err(shutdown_err) = container.shutdown().await {
                error!("Failed to shutdown services gracefully: {}", shutdown_err);
            }
        }
        
        return Err(e.into());
    }
    
    info!("✅ MCP Proxy servers completed");
    
    // Graceful shutdown when server completes
    if let Some(service_container) = Arc::try_unwrap(service_container_arc).ok() {
        if let Err(e) = service_container.shutdown().await {
            warn!("Service shutdown had errors: {}", e);
        }
    } else {
        warn!("Could not unwrap ServiceContainer Arc for shutdown - some references still exist");
    }
    
    Ok(())
}

/// Run MCP Proxy in stdio mode for MCP clients
async fn run_stdio_mode(config: Config) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use serde_json::json;
    use tokio::sync::Mutex;
    use std::sync::Arc;

    // Initialize MCP server with full configuration (including external MCP integration)
    let mcp_server = McpServer::with_config(&config).await?;

    // Set up stdin/stdout - wrap stdout in Arc<Mutex> for sharing between tasks
    let stdin = tokio::io::stdin();
    let stdout = Arc::new(Mutex::new(tokio::io::stdout()));
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    // Subscribe to notifications and forward them to stdio
    let mut notification_receiver = mcp_server.notification_manager().subscribe();
    let stdout_clone = stdout.clone();
    tokio::spawn(async move {
        debug!("Started notification forwarding for stdio mode");
        while let Ok(notification) = notification_receiver.recv().await {
            let notification_json = match serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "method": notification.method,
                "params": notification.params.unwrap_or_default()
            })) {
                Ok(json) => json,
                Err(e) => {
                    warn!("Failed to serialize notification: {}", e);
                    continue;
                }
            };
            
            let mut stdout_guard = stdout_clone.lock().await;
            if let Err(e) = stdout_guard.write_all(notification_json.as_bytes()).await {
                debug!("Failed to write notification to stdout (client likely disconnected): {}", e);
                break;
            }
            if let Err(e) = stdout_guard.write_all(b"\n").await {
                debug!("Failed to write notification newline to stdout: {}", e);
                break;
            }
            if let Err(e) = stdout_guard.flush().await {
                debug!("Failed to flush notification to stdout: {}", e);
                break;
            }
            debug!("Sent notification via stdio: {}", notification.method);
        }
        debug!("Notification forwarding ended for stdio mode");
    });

    info!("MCP Proxy stdio mode ready - waiting for JSON-RPC messages");

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF - client disconnected
                info!("stdin closed, shutting down stdio mode");
                break;
            }
            Ok(_) => {
                let trimmed_line = line.trim();
                if trimmed_line.is_empty() {
                    continue;
                }

                match handle_stdio_message(&mcp_server, trimmed_line).await {
                    Ok(Some(response)) => {
                        let mut stdout_guard = stdout.lock().await;
                        if let Err(e) = stdout_guard.write_all(response.as_bytes()).await {
                            error!("Failed to write response to stdout: {}", e);
                            break;
                        }
                        if let Err(e) = stdout_guard.write_all(b"\n").await {
                            error!("Failed to write newline to stdout: {}", e);
                            break;
                        }
                        if let Err(e) = stdout_guard.flush().await {
                            error!("Failed to flush stdout: {}", e);
                            break;
                        }
                    }
                    Ok(None) => {
                        // No response needed (e.g., notification)
                    }
                    Err(e) => {
                        error!("Error handling stdio message: {}", e);
                        // Send error response
                        let error_response = json!({
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": {
                                "code": -32603,
                                "message": format!("Internal error: {}", e)
                            }
                        });
                        let mut stdout_guard = stdout.lock().await;
                        if let Err(write_err) = stdout_guard.write_all(error_response.to_string().as_bytes()).await {
                            error!("Failed to write error response: {}", write_err);
                            break;
                        }
                        if let Err(write_err) = stdout_guard.write_all(b"\n").await {
                            error!("Failed to write error newline: {}", write_err);
                            break;
                        }
                        let _ = stdout_guard.flush().await;
                    }
                }
            }
            Err(e) => {
                error!("Failed to read from stdin: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Handle a single JSON-RPC message from stdin
async fn handle_stdio_message(server: &McpServer, message: &str) -> Result<Option<String>> {
    use mcp::types::McpRequest;
    use mcp::errors::McpErrorCode;

    // Parse JSON-RPC request
    let request: McpRequest = match serde_json::from_str(message) {
        Ok(req) => req,
        Err(e) => {
            return Ok(Some(create_error_response(
                None,
                McpErrorCode::ParseError,
                &format!("Invalid JSON: {}", e)
            )));
        }
    };

    // Use the unified MCP handler from McpServer
    server.handle_mcp_request(request).await.map_err(|e| e.into())
}


/// Create an error JSON-RPC response
fn create_error_response(id: Option<&serde_json::Value>, code: McpErrorCode, message: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code as i32,
            "message": message
        }
    }).to_string()
}

/// Run as a single-shot MCP client: read one request from stdin, process it, return result, and exit
async fn run_mcp_client_mode(config: Config) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    // Initialize MCP server with full configuration
    let mcp_server = McpServer::with_config(&config).await?;

    // Read single request from stdin
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    info!("Single-shot MCP client ready - reading request from stdin");

    match reader.read_line(&mut line).await {
        Ok(0) => {
            error!("No input received from stdin");
            std::process::exit(1);
        }
        Ok(_) => {
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() {
                error!("Empty request received");
                std::process::exit(1);
            }

            // Parse and handle the MCP request
            match handle_single_mcp_request(&mcp_server, trimmed_line).await {
                Ok(response) => {
                    // Print response to stdout
                    println!("{}", response);
                    info!("MCP request processed successfully");
                }
                Err(e) => {
                    error!("Error processing MCP request: {}", e);
                    // Print error response to stdout
                    let error_response = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {
                            "code": -32603,
                            "message": format!("Internal error: {}", e)
                        }
                    });
                    println!("{}", error_response);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            error!("Failed to read from stdin: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Handle a single MCP request and return the response
async fn handle_single_mcp_request(server: &McpServer, message: &str) -> Result<String> {
    use mcp::types::McpRequest;
    use mcp::errors::McpErrorCode;

    // Parse JSON-RPC request
    let request: McpRequest = match serde_json::from_str(message) {
        Ok(req) => req,
        Err(e) => {
            let error_response = create_error_response(
                None,
                McpErrorCode::ParseError,
                &format!("Invalid JSON: {}", e)
            );
            return Ok(error_response);
        }
    };

    // Use the unified MCP handler from McpServer
    match server.handle_mcp_request(request).await {
        Ok(Some(response)) => Ok(response),
        Ok(None) => {
            // No response needed (e.g., notification) - return empty success
            Ok(json!({
                "jsonrpc": "2.0",
                "id": null,
                "result": "notification processed"
            }).to_string())
        }
        Err(e) => {
            let error_response = create_error_response(
                None,
                McpErrorCode::InternalError,
                &format!("Request handling failed: {}", e)
            );
            Ok(error_response)
        }
    }
}

/// Pre-generate embeddings for all enabled capabilities and exit
async fn pregenerate_embeddings_and_exit(config: Config) -> Result<()> {
    info!("Starting enhanced embedding pre-generation process");
    
    // Initialize the registry to discover all capabilities
    let registry = Arc::new(registry::RegistryService::new(config.registry.clone()).await?);
    info!("Registry initialized with {} tools", registry.get_all_tools().len());
    
    // Check if smart discovery is configured
    let smart_discovery_config = match &config.smart_discovery {
        Some(discovery_config) if discovery_config.semantic_search.enabled => discovery_config.clone(),
        Some(_) => {
            info!("⚠️  Semantic search is disabled in configuration - skipping embedding generation");
            return Ok(());
        },
        None => {
            info!("⚠️  Smart discovery not configured - skipping embedding generation");
            return Ok(());
        }
    };
    
    info!("🧠 Using embedding model: {}", smart_discovery_config.semantic_search.model_name);
    
    // Initialize sampling service for MCP-compliant sampling/createMessage functionality
    let sampling_service = if config.sampling.as_ref().map(|s| s.enabled).unwrap_or(false) {
        info!("🎯 Initializing sampling service for MCP-compliant LLM message generation");
        match mcp::sampling::SamplingService::from_config(&config) {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                warn!("Failed to initialize sampling service: {}. Using fallback.", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize tool enhancement service for description/keyword/example generation (renamed from sampling to avoid MCP spec confusion)
    let tool_enhancement_service = if config.tool_enhancement.as_ref().map(|s| s.enabled).unwrap_or(false) {
        info!("🔧 Initializing tool enhancement service for description/keyword/example generation");
        match mcp::tool_enhancement::ToolEnhancementService::from_config(&config) {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                warn!("Failed to initialize tool enhancement service: {}. Using base descriptions.", e);
                None
            }
        }
    } else {
        None
    };
    
    let elicitation_service = if config.elicitation.as_ref().map(|e| e.enabled).unwrap_or(false) {
        info!("🎯 Initializing elicitation service for enhanced metadata");
        match mcp::elicitation::ElicitationService::from_config(&config) {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                warn!("Failed to initialize elicitation service: {}. Using base metadata.", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize external MCP integration if configured
    let external_mcp_manager = if let Some(external_config) = &config.external_mcp {
        info!("🔗 Initializing external MCP manager for embedding generation");
        let client_config = config::McpClientConfig::default(); // Use default client config for embedding generation
        let manager = Arc::new(mcp::ExternalMcpManager::new(external_config.clone(), client_config));
        Some(manager)
    } else {
        None
    };
    
    // Initialize content storage service
    let content_storage = if let Some(content_config) = &config.content_storage {
        info!("📦 Initializing content storage service");
        match mcp::ContentStorageService::new(content_config.clone()) {
            Ok(service) => {
                let service = Arc::new(service);
                if let Err(e) = service.initialize().await {
                    error!("Failed to initialize content storage service: {}", e);
                    return Err(e.into());
                }
                Some(service)
            }
            Err(e) => {
                warn!("Failed to create content storage service: {}. Content storage disabled.", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize prompt generation service (not used in embedding generation)
    let _prompt_generator = if let Some(prompt_config) = &config.prompt_generation {
        info!("📝 Initializing prompt generation service");
        match mcp::PromptGeneratorService::new(
            prompt_config.clone(),
            external_mcp_manager.clone(),
            content_storage.clone(),
        ) {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                warn!("Failed to initialize prompt generation service: {}. Prompt generation disabled.", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize resource generation service (not used in embedding generation)
    let _resource_generator = if let Some(resource_config) = &config.resource_generation {
        info!("📋 Initializing resource generation service");
        match mcp::ResourceGeneratorService::new(
            resource_config.clone(),
            external_mcp_manager.clone(),
        ) {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                warn!("Failed to initialize resource generation service: {}. Resource generation disabled.", e);
                None
            }
        }
    } else {
        None
    };
    
    // Initialize external content manager (not used in embedding generation)
    let _external_content_manager = if let Some(external_config) = &config.external_content {
        info!("🔗 Initializing external content manager");
        let manager = mcp::ExternalContentManager::new(
            external_config.clone(),
            external_mcp_manager.clone(),
            content_storage.clone().unwrap_or_else(|| {
                Arc::new(mcp::ContentStorageService::new(mcp::ContentStorageConfig::default()).unwrap())
            }),
        );
        Some(Arc::new(manager))
    } else {
        None
    };
    
    // Initialize enhancement storage service
    debug!("🔧 Enhancement storage config: {:?}", config.enhancement_storage.is_some());
    let enhancement_storage = if let Some(storage_config) = &config.enhancement_storage {
        info!("💾 Initializing enhancement storage service");
        match discovery::EnhancementStorageService::new(storage_config.clone()) {
            Ok(service) => {
                let service = Arc::new(service);
                if let Err(e) = service.initialize().await {
                    error!("Failed to initialize enhancement storage service: {}", e);
                    return Err(e.into());
                }
                Some(service)
            }
            Err(e) => {
                warn!("Failed to create enhancement storage service: {}. Enhancement storage disabled.", e);
                None
            }
        }
    } else {
        None
    };
    
    // Check if tool enhancement is enabled (separate from MCP sampling/elicitation)
    let tool_enhancement_enabled = config.tool_enhancement
        .as_ref()
        .map(|te| te.enabled)
        .unwrap_or(false);
    
    // Check if tools need enhancement before initializing the service
    // Only run enhancement if tool enhancement is enabled in config
    let tools_need_enhancement = if tool_enhancement_enabled {
        info!("🔍 Tool enhancement enabled in config - checking if tools need enhancement");
        check_if_tools_need_enhancement(&registry, &enhancement_storage).await.unwrap_or(true) // Default to true if check fails
    } else {
        info!("📭 Tool enhancement disabled in config - skipping enhancement pipeline");
        false
    };
    
    // Initialize tool enhancement service only if enabled and tools actually need enhancement
    let enhancement_service = if tools_need_enhancement {
        info!("🚀 Initializing tool enhancement pipeline for embedding generation");
        let enhancement_config = discovery::ToolEnhancementConfig {
            enable_description_enhancement: tool_enhancement_service.is_some(),
            enable_tool_enhancement: tool_enhancement_enabled,
            require_approval: false, // No approval needed for embedding generation
            cache_enhancements: true,
            enhancement_timeout_seconds: 60, // Longer timeout for batch processing
            batch_size: 20, // Larger batch for embedding generation
            graceful_degradation: true,
        };
        
        // Use tool_enhancement_service for the enhancement pipeline (tool descriptions/keywords/examples)
        let effective_tool_enhancement_service = tool_enhancement_service.clone();
        
        let smart_discovery_enabled = config.smart_discovery.as_ref().map(|sd| sd.enabled).unwrap_or(false);
        let enhancement_service = Arc::new(discovery::ToolEnhancementPipeline::new_with_storage(
            enhancement_config,
            Arc::clone(&registry),
            effective_tool_enhancement_service,
            elicitation_service,
            enhancement_storage,
            config.elicitation.clone(),
            smart_discovery_enabled,
        ));
        
        // Register enhancement service with registry for tool change notifications
        registry.set_enhancement_callback(Arc::clone(&enhancement_service) as Arc<dyn registry::service::EnhancementCallback>);
        info!("🔔 Enhancement service registered for tool change notifications");
        
        // Run ALL enhancement initialization in background to avoid blocking server startup
        let enhancement_service_clone = Arc::clone(&enhancement_service);
        tokio::spawn(async move {
            info!("🔄 Starting tool enhancement service in background thread");
            
            // Fast initialization
            if let Err(e) = enhancement_service_clone.initialize().await {
                error!("Failed to do fast initialization of enhancement service: {}", e);
                return;
            }
            
            // Full analysis
            if let Err(e) = enhancement_service_clone.initialize_with_analysis().await {
                error!("Failed to complete enhancement service analysis in background: {}", e);
            } else {
                info!("✅ Tool enhancement background analysis completed");
            }
        });
        
        Some(enhancement_service)
    } else {
        info!("✅ All tools already enhanced - skipping enhancement pipeline initialization completely");
        None
    };
    
    // Initialize semantic search service
    let semantic_service = Arc::new(discovery::SemanticSearchService::new(
        smart_discovery_config.semantic_search.clone()
    ));
    semantic_service.initialize().await?;
    
    // Initialize enhanced embedding manager
    let embedding_manager_config = discovery::EmbeddingManagerConfig::default();
    let embedding_manager = if let Some(enhancement_service) = enhancement_service {
        info!("🌟 Creating enhanced embedding manager with sampling/elicitation pipeline");
        discovery::EmbeddingManager::new_with_enhancement(
            registry,
            semantic_service.clone(),
            embedding_manager_config,
            enhancement_service,
        )
    } else {
        discovery::EmbeddingManager::new(
            registry,
            semantic_service.clone(),
            embedding_manager_config,
        )
    };
    embedding_manager.initialize().await?;
    
    // Force sync to generate all embeddings
    info!("🚀 Generating embeddings for all enabled capabilities...");
    let start_time = std::time::Instant::now();
    
    let summary = embedding_manager.force_sync().await?;
    
    let duration = start_time.elapsed();
    
    // Report results
    info!("✅ Embedding generation completed in {:.2}s", duration.as_secs_f64());
    info!("📊 Summary:");
    info!("   - Total processed: {}", summary.total_processed);
    info!("   - Created: {}", summary.created);
    info!("   - Updated: {}", summary.updated);
    info!("   - Removed: {}", summary.removed);
    info!("   - Failed: {}", summary.failed);
    
    if summary.failed > 0 {
        error!("❌ {} operations failed:", summary.failed);
        for op in &summary.operations {
            if !op.success {
                error!("   - {}: {}", op.tool_name, op.error.as_ref().unwrap_or(&"Unknown error".to_string()));
            }
        }
        return Err(anyhow::anyhow!("Embedding generation had {} failures", summary.failed));
    }
    
    // Save embeddings through the semantic service
    semantic_service.save_embeddings().await?;
    info!("💾 Embeddings saved to disk");
    
    info!("🎉 Pre-generation complete! Server startup will now be faster.");
    Ok(())
}

/// Check if tools need enhancement by analyzing existing enhancement storage
/// Only counts regular tools that should be enhanced (excludes external MCP tools and system tools)
async fn check_if_tools_need_enhancement(
    registry: &Arc<registry::RegistryService>,
    enhancement_storage: &Option<Arc<discovery::EnhancementStorageService>>
) -> Result<bool> {
    // Get all enabled tools from registry  
    let all_tools = registry.get_enabled_tools();
    if all_tools.is_empty() {
        info!("📭 No tools found in registry - skipping enhancement");
        return Ok(false);
    }

    info!("🔍 Analyzing {} tools to determine enhancement needs", all_tools.len());

    // Convert Vec to HashMap for the centralized method
    let tools_map: HashMap<String, ToolDefinition> = all_tools.into_iter().collect();
    
    // Use centralized logic for tool analysis
    let (regular_tools, analysis) = discovery::ToolEnhancementPipeline::analyze_tools_for_enhancement(&tools_map, enhancement_storage).await?;

    info!("📊 Tool categorization:");
    info!("  - Total tools: {}", analysis.total_tools);
    info!("  - Regular tools (can be enhanced): {}", analysis.regular_tools_count);
    info!("  - External MCP tools (skip): {}", analysis.external_tools_count);
    info!("  - Disabled tools (skip): {}", analysis.disabled_tools_count);
    info!("  - Generator-enhanced tools (skip): {}", analysis.generator_enhanced_count);

    if regular_tools.is_empty() {
        info!("✅ No regular tools found that need enhancement");
        return Ok(false);
    }

    let needs_enhancement = analysis.needs_enhancement > 0;
    
    info!("🎯 Enhancement analysis results:");
    info!("  - Already enhanced: {}", analysis.already_enhanced);
    info!("  - Need enhancement: {}", analysis.needs_enhancement);
    
    if needs_enhancement {
        info!("⚡ Will initialize enhancement service for {} tools", analysis.needs_enhancement);
    } else {
        info!("✅ All regular tools already enhanced - skipping enhancement service initialization");
    }

    Ok(needs_enhancement)
}

fn init_logging(level: &str) -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    // Initialize the global log buffer (keep last 1000 log entries)
    let log_buffer = web::initialize_global_log_buffer(1000);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_writer(std::io::stderr) // Send logs to stderr for stdio mode
        )
        .with(web::LogBufferLayer::new(log_buffer)) // Add our custom layer to capture logs
        .with(env_filter)
        .init();

    Ok(())
}
