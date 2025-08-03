use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, error, warn};
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
mod supervisor;
mod tls;
mod web;

use config::Config;
use mcp::{McpServer, McpErrorCode, ExternalMcpIntegration};
use grpc::McpGrpcServer;
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
    
    info!("Starting Magictunnel v{}", env!("CARGO_PKG_VERSION"));
    
    // Load configuration
    let config = Config::load(&cli.config, cli.host, cli.port)
        .map_err(|e| {
            error!("Failed to load configuration: {}", e);
            e
        })?;
    
    info!("Configuration loaded successfully");

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
        // Run in HTTP server mode (existing implementation)
        info!("HTTP server will bind to {}:{}", config.server.host, config.server.port);

        // Calculate gRPC port (HTTP port + 1000)
        let grpc_port = config.server.port + 1000;
        info!("gRPC server will bind to {}:{}", config.server.host, grpc_port);

        // Initialize MCP HTTP server with full configuration
        let http_server = McpServer::with_config(&config).await?;

        // Get registry from the server for gRPC server
        let registry = http_server.registry().clone();

        // Initialize gRPC server with registry
        let grpc_server = McpGrpcServer::new(registry.clone());

        info!("Starting Magictunnel servers...");

        // Start gRPC server in background task
        let grpc_addr = format!("{}:{}", config.server.host, grpc_port).parse()?;

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
        if let Err(e) = http_server.start_with_config(&config.server.host, config.server.port, config.server.tls.clone()).await {
            error!("HTTP server failed: {}", e);
            return Err(e.into());
        }

        info!("MCP Proxy servers completed");
    }
    
    Ok(())
}

/// Run MCP Proxy in stdio mode for MCP clients
async fn run_stdio_mode(config: Config) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use serde_json::json;

    // Initialize MCP server with full configuration (including external MCP integration)
    let mcp_server = McpServer::with_config(&config).await?;

    // Set up stdin/stdout
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

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
                        if let Err(e) = stdout.write_all(response.as_bytes()).await {
                            error!("Failed to write response to stdout: {}", e);
                            break;
                        }
                        if let Err(e) = stdout.write_all(b"\n").await {
                            error!("Failed to write newline to stdout: {}", e);
                            break;
                        }
                        if let Err(e) = stdout.flush().await {
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
                        if let Err(write_err) = stdout.write_all(error_response.to_string().as_bytes()).await {
                            error!("Failed to write error response: {}", write_err);
                            break;
                        }
                        if let Err(write_err) = stdout.write_all(b"\n").await {
                            error!("Failed to write error newline: {}", write_err);
                            break;
                        }
                        let _ = stdout.flush().await;
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

/// Create a successful JSON-RPC response
fn create_success_response(id: &serde_json::Value, result: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    }).to_string()
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
    
    // Initialize sampling and elicitation services if enabled
    let sampling_service = if config.sampling.as_ref().map(|s| s.enabled).unwrap_or(false) ||
                             smart_discovery_config.enable_sampling.unwrap_or(false) {
        info!("🎯 Initializing sampling service for enhanced descriptions");
        match mcp::sampling::SamplingService::from_config(&config) {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                warn!("Failed to initialize sampling service: {}. Using base descriptions.", e);
                None
            }
        }
    } else {
        None
    };
    
    let elicitation_service = if config.elicitation.as_ref().map(|e| e.enabled).unwrap_or(false) ||
                                smart_discovery_config.enable_elicitation.unwrap_or(false) {
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
    
    // Initialize prompt generation service
    let prompt_generator = if let Some(prompt_config) = &config.prompt_generation {
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
    
    // Initialize resource generation service
    let resource_generator = if let Some(resource_config) = &config.resource_generation {
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
    
    // Initialize external content manager
    let external_content_manager = if let Some(external_config) = &config.external_content {
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
    
    // Initialize tool enhancement service if sampling/elicitation enabled
    let enhancement_service = if sampling_service.is_some() || elicitation_service.is_some() {
        info!("🚀 Initializing tool enhancement pipeline for embedding generation");
        let enhancement_config = discovery::ToolEnhancementConfig {
            enable_sampling_enhancement: sampling_service.is_some(),
            enable_elicitation_enhancement: elicitation_service.is_some(),
            require_approval: false, // No approval needed for embedding generation
            cache_enhancements: true,
            enhancement_timeout_seconds: 60, // Longer timeout for batch processing
            batch_size: 20, // Larger batch for embedding generation
            graceful_degradation: true,
        };
        
        let enhancement_service = Arc::new(discovery::ToolEnhancementService::new_with_storage(
            enhancement_config,
            Arc::clone(&registry),
            sampling_service,
            elicitation_service,
            enhancement_storage,
        ));
        
        // Register enhancement service with registry for tool change notifications
        registry.set_enhancement_callback(Arc::clone(&enhancement_service) as Arc<dyn registry::service::EnhancementCallback>);
        info!("🔔 Enhancement service registered for tool change notifications");
        
        // Initialize enhancement service (generate missing enhancements at startup)
        if let Err(e) = enhancement_service.initialize().await {
            error!("Failed to initialize enhancement service: {}", e);
            return Err(e.into());
        }
        
        Some(enhancement_service)
    } else {
        info!("🔧 No enhancement services available - using base descriptions");
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

fn init_logging(level: &str) -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_writer(std::io::stderr) // Send logs to stderr for stdio mode
        )
        .with(env_filter)
        .init();

    Ok(())
}
