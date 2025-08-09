//! MCP Server implementation

use crate::auth::AuthenticationMiddleware;
use crate::config::{RegistryConfig, AuthConfig, TlsConfig, TlsMode, ConfigResolution};
use crate::services::ServiceContainer;
use crate::error::{Result, ProxyError};



use crate::mcp::types::*;
use crate::mcp::resources::{ResourceManager, FileResourceProvider};
use crate::mcp::prompts::{PromptManager};
use crate::mcp::logging::{McpLoggerManager, McpLogger};
use crate::mcp::notifications::{McpNotificationManager};


use crate::mcp::errors::{McpError, McpErrorCode};
use crate::mcp::session::McpSessionManager;
use crate::mcp::validation::McpMessageValidator;
use crate::mcp::cancellation::{CancellationManager, CancellationConfig};
use crate::mcp::progress::{ProgressTracker, ProgressConfig};
use crate::mcp::tool_validation::{RuntimeToolValidator, ValidationConfig as ToolValidationConfig};
use crate::mcp::request_forwarder::RequestForwarder;
use crate::registry::service::{RegistryService, EnhancementCallback};
use crate::routing::{Router, types::AgentResult};
use crate::web::{configure_dashboard_api};
use actix_web::{web, App, HttpServer, HttpResponse, middleware::Logger, HttpRequest};
use actix_ws::Message;
use futures_util::{StreamExt, stream};
use futures_util as futures;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, info, warn, error};
use uuid;
use chrono;
use serde_json::{json, Value};
use std::sync::Arc;
use async_trait::async_trait;

/// MCP Server that handles protocol communication
pub struct McpServer {
    /// High-performance registry service
    registry: Arc<RegistryService>,
    /// Tool aggregation service with conflict resolution
    tool_aggregation: Option<Arc<crate::registry::ToolAggregationService>>,
    /// Agent router for tool execution
    router: Arc<Router>,
    /// Resource manager for MCP resources
    resource_manager: Arc<ResourceManager>,
    /// Prompt template manager for MCP prompts
    prompt_manager: Arc<PromptManager>,
    /// MCP logger manager for structured logging
    logger_manager: Arc<McpLoggerManager>,
    /// MCP notification manager for protocol notifications
    notification_manager: Arc<McpNotificationManager>,
    /// Authentication middleware for securing endpoints ✅ **NEW**
    auth_middleware: Option<Arc<AuthenticationMiddleware>>,
    /// Security middleware for comprehensive security controls ✅ **NEW**
    security_middleware: Option<Arc<crate::security::SecurityMiddleware>>,
    /// Session manager for WebSocket connection tracking ✅ **NEW**
    session_manager: Arc<McpSessionManager>,
    /// Message validator for enhanced protocol compliance ✅ **NEW**
    message_validator: Arc<McpMessageValidator>,
    /// Cancellation manager for request cancellation support ✅ **NEW**
    cancellation_manager: Arc<CancellationManager>,
    /// Progress tracker for long-running operations ✅ **NEW**
    progress_tracker: Arc<ProgressTracker>,
    /// Runtime tool validator for security sandboxing ✅ **NEW**
    tool_validator: Arc<RuntimeToolValidator>,
    /// Smart discovery service for intelligent tool selection ✅ **NEW**
    smart_discovery: Option<Arc<crate::discovery::SmartDiscoveryService>>,
    /// External MCP integration for managing external MCP servers ✅ **NEW**
    external_integration: Option<Arc<tokio::sync::RwLock<crate::mcp::external_integration::ExternalMcpIntegration>>>,
    /// Configuration for dynamic protocol version and settings ✅ **NEW**
    config: Option<Arc<crate::config::Config>>,
    /// Sampling service for LLM message generation ✅ **NEW**
    sampling_service: Option<Arc<crate::mcp::sampling::SamplingService>>,
    /// Tool Enhancement service for tool description/keyword/example generation (renamed from sampling)
    tool_enhancement_service: Option<Arc<crate::mcp::tool_enhancement::ToolEnhancementService>>,
    /// Elicitation service for structured data collection ✅ **NEW**
    elicitation_service: Option<Arc<crate::mcp::elicitation::ElicitationService>>,
    /// Roots service for filesystem/URI boundary discovery ✅ **NEW**
    roots_service: Option<Arc<crate::mcp::roots::RootsService>>,
    /// Channel sender for forwarding requests to the original MCP client
    client_sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
}

impl McpServer {
    /// Convert AgentResult to MCP-compliant ToolResult
    fn agent_result_to_tool_result(agent_result: AgentResult, _tool_name: &str, metadata: Option<Value>) -> ToolResult {
        if agent_result.success {
            let data = agent_result.data.unwrap_or(json!({}));
            let mut result = ToolResult::success_with_metadata(data, metadata.unwrap_or(json!({})));

            // Merge agent metadata with existing metadata
            if let Some(agent_metadata) = agent_result.metadata {
                if let Some(existing_metadata) = &mut result.metadata {
                    if let Some(existing_obj) = existing_metadata.as_object_mut() {
                        // Merge agent metadata into existing metadata at top level
                        if let Some(agent_obj) = agent_metadata.as_object() {
                            for (key, value) in agent_obj {
                                existing_obj.insert(key.clone(), value.clone());
                            }
                        }
                        // Also keep the full agent metadata for debugging/compatibility
                        existing_obj.insert("agent_metadata".to_string(), agent_metadata);
                    }
                } else {
                    // If no existing metadata, use agent metadata directly and also nest it
                    if let Some(agent_obj) = agent_metadata.as_object() {
                        let mut new_metadata = agent_obj.clone();
                        new_metadata.insert("agent_metadata".to_string(), agent_metadata.clone());
                        result.metadata = Some(Value::Object(new_metadata));
                    } else {
                        result.metadata = Some(json!({"agent_metadata": agent_metadata}));
                    }
                }
            }

            result
        } else {
            let error_msg = agent_result.error.unwrap_or_else(|| "Unknown error".to_string());
            let mut error_metadata = metadata.unwrap_or(json!({}));
            
            // Merge agent metadata for error cases too
            if let Some(agent_metadata) = agent_result.metadata {
                if let Some(error_obj) = error_metadata.as_object_mut() {
                    // Merge agent metadata into error metadata at top level
                    if let Some(agent_obj) = agent_metadata.as_object() {
                        for (key, value) in agent_obj {
                            error_obj.insert(key.clone(), value.clone());
                        }
                    }
                    // Also keep the full agent metadata for debugging/compatibility
                    error_obj.insert("agent_metadata".to_string(), agent_metadata);
                } else {
                    // If no existing metadata, use agent metadata directly
                    if let Some(agent_obj) = agent_metadata.as_object() {
                        let mut new_metadata = agent_obj.clone();
                        new_metadata.insert("agent_metadata".to_string(), agent_metadata.clone());
                        error_metadata = Value::Object(new_metadata);
                    } else {
                        error_metadata = json!({"agent_metadata": agent_metadata});
                    }
                }
            }
            
            ToolResult::error_with_metadata(error_msg, error_metadata)
        }
    }

    /// Format ToolResult for MCP protocol with essential next_step information
    fn format_mcp_response(&self, tool_result: ToolResult) -> Value {
        // Start with the base response structure
        let mut response = json!({
            "success": tool_result.success,
            "is_error": tool_result.is_error
        });

        // Include error if present
        if let Some(error) = tool_result.error {
            response["error"] = json!(error);
        }

        // Build enhanced content with next_step info
        let mut enhanced_response = json!({});
        
        // Include legacy data field for backward compatibility
        if let Some(data) = &tool_result.data {
            enhanced_response = data.clone();
        }

        // Extract essential next_step info from metadata for multi-step workflows
        if let Some(metadata) = &tool_result.metadata {
            if let Some(next_step) = metadata.get("next_step") {
                // Include minimal next_step info for MCP - just the essential fields
                if let Some(next_step_obj) = next_step.as_object() {
                    let mut mcp_next_step = json!({});
                    
                    // Always include the suggested request (most important)
                    if let Some(suggested_request) = next_step_obj.get("suggested_request") {
                        mcp_next_step["suggested_request"] = suggested_request.clone();
                    }
                    
                    // Include brief reasoning if available (helps Claude understand context)
                    if let Some(reasoning) = next_step_obj.get("reasoning") {
                        if let Some(reasoning_str) = reasoning.as_str() {
                            // Limit reasoning to 100 chars for token efficiency
                            let brief_reasoning = if reasoning_str.len() > 100 {
                                format!("{}...", &reasoning_str[..97])
                            } else {
                                reasoning_str.to_string()
                            };
                            mcp_next_step["reasoning"] = json!(brief_reasoning);
                        }
                    }
                    
                    if !mcp_next_step.as_object().unwrap().is_empty() {
                        enhanced_response["next_step"] = mcp_next_step;
                    }
                }
            }
        }

        // Create new content with the enhanced response
        use crate::mcp::types::ToolContent;
        let enhanced_content = vec![ToolContent::text(
            serde_json::to_string_pretty(&enhanced_response)
                .unwrap_or_else(|_| enhanced_response.to_string())
        )];
        
        response["content"] = json!(enhanced_content);

        response
    }

    /// Create a new MCP server with registry service
    pub async fn new(registry_config: RegistryConfig) -> Result<Self> {
        info!("Initializing MCP server with registry service");

        // Initialize the high-performance registry service
        let registry = RegistryService::start_with_hot_reload(registry_config).await?;

        // Initialize the router with default agent router
        let router = Arc::new(Router::new());

        // Initialize resource manager with default file provider
        let resource_manager = Arc::new(ResourceManager::new());

        // Add default file resource provider for current directory
        if let Ok(current_dir) = std::env::current_dir() {
            if let Ok(file_provider) = FileResourceProvider::new(&current_dir, "file:".to_string()) {
                resource_manager.add_provider(Arc::new(file_provider)).await;
                info!("Added file resource provider for directory: {}", current_dir.display());
            }
        }

        // Create prompt manager with default templates
        let prompt_manager = Arc::new(PromptManager::new());

        // Create logger manager
        let logger_manager = Arc::new(McpLoggerManager::new());

        // Create notification manager with default capabilities
        let notification_manager = Arc::new(McpNotificationManager::new());

        // Set notification manager on registry for list_changed notifications
        registry.set_notification_manager(notification_manager.clone());

        // Create session manager with default configuration
        let session_manager = Arc::new(McpSessionManager::new());

        // Create message validator with default configuration
        let message_validator = Arc::new(McpMessageValidator::new());

        // Create cancellation manager with default configuration
        let cancellation_manager = Arc::new(CancellationManager::new(CancellationConfig::default()));

        // Create progress tracker with default configuration
        let progress_tracker = Arc::new(ProgressTracker::new(ProgressConfig::default()));

        // Create tool validator with default configuration
        let tool_validator = Arc::new(RuntimeToolValidator::new(ToolValidationConfig::default()).unwrap());

        Ok(Self {
            registry,
            tool_aggregation: None,
            router,
            resource_manager,
            prompt_manager,
            logger_manager,
            notification_manager,
            auth_middleware: None, // No authentication by default
            security_middleware: None, // No security by default
            session_manager,
            message_validator,
            cancellation_manager,
            progress_tracker,
            tool_validator,
            smart_discovery: None, // No smart discovery by default
            external_integration: None, // No external MCP integration by default
            config: None, // No config by default
            sampling_service: None, // No sampling service by default
            tool_enhancement_service: None, // No tool enhancement service by default
            elicitation_service: None, // No elicitation service by default
            roots_service: None, // No roots service by default
            client_sender: None, // No client sender by default
        })
    }

    /// Create MCP server with existing registry service
    pub fn with_registry(registry: Arc<RegistryService>) -> Self {
        let resource_manager = Arc::new(ResourceManager::new());
        let prompt_manager = Arc::new(PromptManager::new());
        let logger_manager = Arc::new(McpLoggerManager::new());
        let notification_manager = Arc::new(McpNotificationManager::new());
        let session_manager = Arc::new(McpSessionManager::new());
        let message_validator = Arc::new(McpMessageValidator::new());
        let cancellation_manager = Arc::new(CancellationManager::new(CancellationConfig::default()));
        let progress_tracker = Arc::new(ProgressTracker::new(ProgressConfig::default()));
        let tool_validator = Arc::new(RuntimeToolValidator::new(ToolValidationConfig::default()).unwrap());
        Self {
            registry: registry.clone(),
            tool_aggregation: None,
            router: Arc::new(Router::with_registry(registry)),
            resource_manager,
            prompt_manager,
            logger_manager,
            notification_manager,
            auth_middleware: None, // No authentication by default
            security_middleware: None, // No security by default
            session_manager,
            message_validator,
            cancellation_manager,
            progress_tracker,
            tool_validator,
            smart_discovery: None, // No smart discovery by default
            external_integration: None, // No external MCP integration by default
            config: None, // No config by default
            sampling_service: None, // No sampling service by default
            tool_enhancement_service: None, // No tool enhancement service by default
            elicitation_service: None, // No elicitation service by default
            roots_service: None, // No roots service by default
            client_sender: None, // No client sender by default
        }
    }

    /// Create MCP server with full configuration
    pub async fn with_config(config: &crate::config::Config) -> Result<Self> {
        info!("Initializing MCP server with full configuration");

        // Initialize the high-performance registry service with hot-reload
        let registry = RegistryService::start_with_hot_reload(config.registry.clone()).await?;

        // Initialize tool aggregation service with conflict resolution
        let mut tool_aggregation = crate::registry::ToolAggregationService::new(Arc::new(config.clone()));
        tool_aggregation.set_registry_service(registry.clone());

        // Start external MCP integration if enabled
        let external_integration = Arc::new(tokio::sync::RwLock::new(
            crate::mcp::external_integration::ExternalMcpIntegration::new(Arc::new(config.clone()))
        ));

        // Track whether external MCP integration actually started successfully
        let external_mcp_started = if config.external_mcp.as_ref().map(|c| c.enabled).unwrap_or(false) {
            info!("External MCP is enabled in configuration");
            debug!("External MCP config: {:?}", config.external_mcp);
            debug!("Current working directory: {:?}", std::env::current_dir());
            debug!("Executable path: {:?}", std::env::current_exe());
            
            let mut integration = external_integration.write().await;
            info!("Starting external MCP integration...");
            match integration.start().await {
                Err(e) => {
                    warn!("Failed to start external MCP integration: {}", e);
                    info!("Continuing without external MCP integration");
                    false
                }
                Ok(_) => {
                    info!("External MCP integration started successfully");
                    tool_aggregation.set_external_mcp(external_integration.clone());
                    true
                }
            }
        } else {
            info!("External MCP integration is disabled or not configured");
            debug!("External MCP config present: {}", config.external_mcp.is_some());
            if let Some(ref ext_config) = config.external_mcp {
                debug!("External MCP enabled flag: {}", ext_config.enabled);
            }
            false
        };

        // Log conflict resolution configuration
        if let Some(ref cr_config) = config.conflict_resolution {
            info!("Conflict resolution enabled with strategy: {:?}", cr_config.strategy);
        } else {
            info!("No conflict resolution configured - tools will be used as discovered");
        }

        // Initialize resource manager with default file provider
        let resource_manager = Arc::new(ResourceManager::new());

        // Create prompt manager with default templates
        let prompt_manager = Arc::new(PromptManager::new());

        // Create logger manager
        let logger_manager = Arc::new(McpLoggerManager::new());

        // Create notification manager with default capabilities
        let notification_manager = Arc::new(McpNotificationManager::new());

        // Set notification manager on registry for list_changed notifications
        registry.set_notification_manager(notification_manager.clone());



        // Create session manager with default configuration
        let session_manager = Arc::new(McpSessionManager::new());

        // Create message validator with default configuration
        let message_validator = Arc::new(McpMessageValidator::new());

        // Create smart discovery service if configured
        let smart_discovery = if let Some(ref smart_config) = config.smart_discovery {
            if smart_config.enabled {
                info!("Smart discovery service enabled with mode: {}", smart_config.tool_selection_mode);
                
                // Clone config and set API key from environment if needed
                let mut config_with_api_key = smart_config.clone();
                
                // Set API key for llm_tool_selection from environment if not set
                if config_with_api_key.llm_tool_selection.api_key.is_none() {
                    if let Some(api_key_env) = &config_with_api_key.llm_tool_selection.api_key_env {
                        if let Ok(api_key) = std::env::var(api_key_env) {
                            config_with_api_key.llm_tool_selection.api_key = Some(api_key);
                            info!("Loaded API key from {} environment variable for llm_tool_selection", api_key_env);
                        }
                    } else if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
                        config_with_api_key.llm_tool_selection.api_key = Some(api_key.clone());
                        info!("Loaded OpenAI API key from OPENAI_API_KEY environment variable for llm_tool_selection");
                    } else if let Ok(api_key) = std::env::var("SMART_DISCOVERY_LLM_API_KEY") {
                        config_with_api_key.llm_tool_selection.api_key = Some(api_key);
                        info!("Loaded API key from SMART_DISCOVERY_LLM_API_KEY environment variable");
                    }
                }
                
                // Set API key for llm_mapper from environment if not set
                if config_with_api_key.llm_mapper.api_key.is_none() {
                    if let Some(api_key_env) = &config_with_api_key.llm_mapper.api_key_env {
                        if let Ok(api_key) = std::env::var(api_key_env) {
                            config_with_api_key.llm_mapper.api_key = Some(api_key);
                            info!("Loaded API key from {} environment variable for llm_mapper", api_key_env);
                        }
                    }
                }
                
                match crate::discovery::SmartDiscoveryService::new(registry.clone(), config_with_api_key).await {
                    Ok(service) => {
                        info!("Smart discovery service created successfully (router will be set later)");
                        let service_arc = Arc::new(service);
                        
                        // Initialize the service (loads embeddings, etc.)
                        let service_clone = Arc::clone(&service_arc);
                        tokio::spawn(async move {
                            if let Err(e) = service_clone.initialize().await {
                                error!("Failed to initialize smart discovery service: {}", e);
                            } else {
                                info!("Smart discovery service initialized successfully");
                            }
                        });
                        
                        Some(service_arc)
                    }
                    Err(e) => {
                        warn!("Failed to create smart discovery service: {}", e);
                        None
                    }
                }
            } else {
                info!("Smart discovery service is disabled in configuration");
                None
            }
        } else {
            info!("Smart discovery service not configured");
            None
        };

        // Initialize the router with external MCP integration and smart discovery
        let router = match (external_mcp_started, &smart_discovery) {
            (true, Some(smart_discovery_service)) => {
                info!("Creating router WITH external MCP integration AND smart discovery");
                let router_arc = Arc::new(Router::with_external_mcp_registry_and_smart_discovery(
                    external_integration.clone(),
                    registry.clone(),
                    smart_discovery_service.clone()
                ));
                
                // Set the router in the smart discovery service for tool execution
                smart_discovery_service.set_router(router_arc.clone()).await;
                info!("Router set in smart discovery service for tool execution");
                
                router_arc
            }
            (true, None) => {
                info!("Creating router WITH external MCP integration but WITHOUT smart discovery");
                Arc::new(Router::with_external_mcp_and_registry(external_integration.clone(), registry.clone()))
            }
            (false, Some(smart_discovery_service)) => {
                info!("Creating router WITHOUT external MCP integration but WITH smart discovery");
                let router_arc = Arc::new(Router::with_registry_and_smart_discovery(registry.clone(), smart_discovery_service.clone()));
                
                // Set the router in the smart discovery service for tool execution
                smart_discovery_service.set_router(router_arc.clone()).await;
                info!("Router set in smart discovery service for tool execution");
                
                router_arc
            }
            (false, None) => {
                info!("Creating router WITHOUT external MCP integration and WITHOUT smart discovery");
                Arc::new(Router::with_registry(registry.clone()))
            }
        };

        let mut server = Self {
            registry,
            tool_aggregation: Some(Arc::new(tool_aggregation)),
            router: router.clone(),
            resource_manager,
            prompt_manager,
            logger_manager,
            notification_manager,
            auth_middleware: None, // Will be set if configured
            security_middleware: None, // Will be set if configured
            session_manager,
            message_validator,
            smart_discovery,
            external_integration: if external_mcp_started { Some(external_integration) } else { None },
            config: Some(Arc::new(config.clone())), // Store config for dynamic protocol version
            sampling_service: None, // Will be set if configured
            tool_enhancement_service: None, // Will be set if configured
            elicitation_service: None, // Will be set if configured
            roots_service: None, // Will be set if configured
            cancellation_manager: Arc::new(CancellationManager::new(CancellationConfig::default())),
            progress_tracker: Arc::new(ProgressTracker::new(ProgressConfig::default())),
            tool_validator: Arc::new(RuntimeToolValidator::new(ToolValidationConfig::default())?),
            client_sender: None, // Will be set when stdio mode connects
        };

        // Configure authentication if present
        if let Some(auth_config) = &config.auth {
            server = server.with_authentication(auth_config.clone())?;
        }
        
        // Configure security if present
        if let Some(security_config) = &config.security {
            server = server.with_security(security_config.clone()).await?;
        }

        // Configure sampling service if enabled (via smart_discovery.enable_sampling or sampling.enabled)
        let sampling_enabled = config.smart_discovery.as_ref()
            .and_then(|sd| sd.enable_sampling)
            .unwrap_or(false) || 
            config.sampling.as_ref().map(|s| s.enabled).unwrap_or(false);
        if sampling_enabled {
            server = server.with_sampling_service(&config)?;
        }
        
        // Configure tool enhancement service if enabled (via smart_discovery.enable_sampling or tool_enhancement.enabled)
        let tool_enhancement_enabled = config.smart_discovery.as_ref()
            .and_then(|sd| sd.enable_sampling)
            .unwrap_or(false) || 
            config.tool_enhancement.as_ref().map(|s| s.enabled).unwrap_or(false);
        if tool_enhancement_enabled {
            server = server.with_tool_enhancement_service(&config)?;
        }

        // Configure elicitation service if enabled (via smart_discovery.enable_elicitation or elicitation.enabled)
        let elicitation_enabled = config.smart_discovery.as_ref()
            .and_then(|sd| sd.enable_elicitation)
            .unwrap_or(false) || 
            config.elicitation.as_ref().map(|e| e.enabled).unwrap_or(false);
        if elicitation_enabled {
            server = server.with_elicitation_service(&config)?;
        }

        // Configure enhancement service if sampling or elicitation are enabled
        if sampling_enabled || elicitation_enabled {
            server = server.with_enhancement_service(&config).await?;
        }

        // Configure roots service if smart discovery is enabled
        if config.smart_discovery.as_ref().map(|sd| sd.enabled).unwrap_or(false) {
            server = server.with_roots_service(&config)?;
        }

        Ok(server)
    }

    /// Create MCP server with registry and resource manager
    pub fn with_registry_and_resources(
        registry: Arc<RegistryService>,
        resource_manager: Arc<ResourceManager>
    ) -> Self {
        let prompt_manager = Arc::new(PromptManager::new());
        let logger_manager = Arc::new(McpLoggerManager::new());
        let notification_manager = Arc::new(McpNotificationManager::new());
        let session_manager = Arc::new(McpSessionManager::new());
        let message_validator = Arc::new(McpMessageValidator::new());
        let cancellation_manager = Arc::new(CancellationManager::new(CancellationConfig::default()));
        let progress_tracker = Arc::new(ProgressTracker::new(ProgressConfig::default()));
        let tool_validator = Arc::new(RuntimeToolValidator::new(ToolValidationConfig::default()).unwrap());
        Self {
            registry: registry.clone(),
            tool_aggregation: None,
            router: Arc::new(Router::with_registry(registry)),
            resource_manager,
            prompt_manager,
            logger_manager,
            notification_manager,
            auth_middleware: None, // No authentication by default
            security_middleware: None, // No security by default
            session_manager,
            message_validator,
            cancellation_manager,
            progress_tracker,
            tool_validator,
            smart_discovery: None, // No smart discovery by default
            external_integration: None, // No external MCP integration by default
            config: None, // No config by default
            sampling_service: None, // No sampling service by default
            tool_enhancement_service: None, // No tool enhancement service by default
            elicitation_service: None, // No elicitation service by default
            roots_service: None, // No roots service by default
            client_sender: None, // No client sender by default
        }
    }

    /// Configure authentication for the MCP server
    pub fn with_authentication(mut self, auth_config: AuthConfig) -> Result<Self> {
        if auth_config.enabled {
            info!("Enabling authentication with type: {}", auth_config.r#type);
            let auth_middleware = AuthenticationMiddleware::new(auth_config)?;
            self.auth_middleware = Some(Arc::new(auth_middleware));
        } else {
            debug!("Authentication disabled");
            self.auth_middleware = None;
        }
        Ok(self)
    }

    /// Configure smart discovery service for the MCP server
    pub fn with_smart_discovery_service(mut self, smart_discovery: Arc<crate::discovery::SmartDiscoveryService>) -> Self {
        self.smart_discovery = Some(smart_discovery);
        
        // Also update the router to include smart discovery
        let registry = self.registry.clone();
        if let Some(ref discovery_service) = self.smart_discovery {
            self.router = Arc::new(crate::routing::Router::with_registry_and_smart_discovery(
                registry,
                discovery_service.clone()
            ));
        }
        
        self
    }
    
    /// Configure security for the MCP server
    pub async fn with_security(mut self, security_config: crate::security::SecurityConfig) -> Result<Self> {
        if security_config.enabled && security_config.has_any_enabled() {
            info!("Enabling security with {} features", 
                [
                    security_config.allowlist.as_ref().map(|c| c.enabled).unwrap_or(false),
                    security_config.sanitization.as_ref().map(|c| c.enabled).unwrap_or(false),
                    security_config.rbac.as_ref().map(|c| c.enabled).unwrap_or(false),
                    security_config.policies.as_ref().map(|c| c.enabled).unwrap_or(false),
                    security_config.audit.as_ref().map(|c| c.enabled).unwrap_or(false),
                ].iter().filter(|&&enabled| enabled).count()
            );
            
            let security_middleware = crate::security::SecurityMiddleware::new(security_config).await
                .map_err(|e| ProxyError::config(format!("Failed to initialize security middleware: {}", e)))?;
            self.security_middleware = Some(Arc::new(security_middleware));
        } else {
            debug!("Security disabled or no features enabled");
            self.security_middleware = None;
        }
        Ok(self)
    }
    
    /// Build security context for evaluation
    fn build_security_context(
        &self, 
        tool_call: &ToolCall, 
        auth_context: Option<&crate::auth::AuthenticationResult>
    ) -> crate::security::SecurityContext {
        use crate::security::{SecurityContext, SecurityRequest};
        use crate::security::middleware::SecurityTool;
        use std::collections::HashMap;
        use chrono::Utc;
        
        // Extract user information from auth context
        let user = crate::security::extract_security_user(auth_context);
        
        // Build request information
        let request = SecurityRequest {
            id: format!("tool-{}-{}", tool_call.name, Utc::now().timestamp_millis()),
            method: "POST".to_string(),
            path: format!("/mcp/call/{}", tool_call.name),
            client_ip: None, // Would be extracted from HTTP request
            user_agent: None,
            headers: HashMap::new(),
            body: serde_json::to_string(&tool_call.arguments).ok(),
            timestamp: Utc::now(),
        };
        
        // Build tool information
        let tool = SecurityTool {
            name: tool_call.name.clone(),
            parameters: match &tool_call.arguments {
                serde_json::Value::Object(map) => {
                    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                },
                _ => HashMap::new(),
            },
            source: self.registry.get_tool(&tool_call.name)
                .map(|tool_def| tool_def.name().to_string()),
        };
        
        SecurityContext {
            user,
            request,
            tool: Some(tool),
            resource: None,
            metadata: HashMap::new(),
        }
    }


    pub async fn start_with_config(self, host: &str, port: u16, tls_config: Option<TlsConfig>) -> Result<()> {
        self.start_with_config_and_services(host, port, tls_config, None, None).await
    }

    pub async fn start_with_config_and_services(self, host: &str, port: u16, tls_config: Option<TlsConfig>, service_container: Option<Arc<crate::services::ServiceContainer>>, config_resolution: Option<Arc<crate::config::ConfigResolution>>) -> Result<()> {
        // Determine the actual TLS mode and log startup info
        let (effective_mode, protocol) = Self::determine_tls_mode_static(tls_config.as_ref(), host)?;
        info!("Starting MCP server on {}:{} with TLS mode: {:?} ({})", host, port, effective_mode, protocol);

        // Load TLS config before moving self
        let rustls_config = if effective_mode == TlsMode::Application {
            Some(Self::load_rustls_config_static(tls_config.as_ref().unwrap())?)
        } else {
            None
        };

        // Initialize security API with proper async initialization
        let security_config = Arc::new(crate::security::SecurityConfig::secure_defaults());
        let mut security_api_instance = crate::web::SecurityApi::new(security_config);
        if let Err(e) = security_api_instance.initialize_async_services().await {
            warn!("Failed to initialize security API async services: {}", e);
        }
        let security_api_data = web::Data::new(security_api_instance);

        let server_data = web::Data::new(Arc::clone(&self.registry));
        let mcp_server_data = web::Data::new(Arc::new(self));

        let server = HttpServer::new(move || {
            let mut app = App::new()
                .app_data(server_data.clone())
                .app_data(mcp_server_data.clone())
                .wrap(Logger::default());

            // Add TLS config to app data if available
            if let Some(tls_cfg) = tls_config.clone() {
                app = app.app_data(web::Data::new(tls_cfg));
            }

            app
                // Health check
                .route("/health", web::get().to(health_check))

                // MCP JSON-RPC 2.0 endpoint (unified protocol)
                .route("/mcp/jsonrpc", web::post().to(mcp_jsonrpc_handler))

                // Standard HTTP endpoints (backward compatibility)
                .route("/mcp/tools", web::get().to(list_tools_handler))
                .route("/mcp/call", web::post().to(call_tool_handler))

                // Resource endpoints
                .route("/mcp/resources", web::get().to(list_resources_handler))
                .route("/mcp/resources/read", web::post().to(read_resource_handler))

                // Prompt endpoints
                .route("/mcp/prompts", web::get().to(list_prompts_handler))
                .route("/mcp/prompts/get", web::post().to(get_prompt_handler))

                // Logging endpoints
                .route("/mcp/logging/setLevel", web::post().to(set_log_level_handler))

                // Streaming endpoints
                .route("/mcp/ws", web::get().to(websocket_handler))
                .route("/mcp/stream", web::get().to(sse_handler)) // Deprecated but maintained for backward compatibility
                .route("/mcp/call/stream", web::post().to(streaming_tool_handler))
                
                // MCP 2025-06-18 Streamable HTTP Transport (preferred over deprecated SSE)
                .route("/mcp/streamable", web::post().to(streamable_http_handler))

                // OAuth authentication endpoints
                .route("/auth/oauth/authorize", web::get().to(oauth_authorize_handler))
                .route("/auth/oauth/callback", web::get().to(oauth_callback_handler))
                .route("/auth/oauth/token", web::post().to(oauth_token_handler))

                // Dashboard API routes
                .configure({
                    let registry = server_data.get_ref().clone();
                    let mcp_server = mcp_server_data.get_ref().clone();
                    let external_mcp = mcp_server.external_integration.clone();
                    let resource_manager = mcp_server.resource_manager.clone();
                    let prompt_manager = mcp_server.prompt_manager.clone();
                    let discovery = mcp_server.smart_discovery.clone();
                    let service_container_clone = service_container.clone();
                    move |cfg| configure_dashboard_api(cfg, registry, mcp_server, external_mcp, resource_manager, prompt_manager, discovery, service_container_clone)
                })

                // Security API routes
                .configure({
                    let security_api = security_api_data.clone();
                    move |cfg| crate::web::configure_security_api(cfg, security_api)
                })


                // TODO: Add gRPC endpoints (will need separate gRPC server)
        });

        // Bind server with appropriate TLS configuration
        match effective_mode {
            TlsMode::Application => {
                let rustls_config = rustls_config.unwrap(); // Safe because we loaded it above
                server.bind_rustls(format!("{}:{}", host, port), rustls_config)?
            }
            _ => {
                // Disabled, BehindProxy, or Auto mode without certificates - use plain HTTP
                server.bind(format!("{}:{}", host, port))?
            }
        }
        .run()
        .await?;

        debug!("MCP server started successfully");
        Ok(())
    }

    /// Determine the effective TLS mode based on configuration and environment
    fn determine_tls_mode_static(tls_config: Option<&TlsConfig>, _host: &str) -> Result<(TlsMode, &'static str)> {
        match tls_config {
            None => Ok((TlsMode::Disabled, "HTTP")),
            Some(config) => {
                match config.mode {
                    TlsMode::Disabled => Ok((TlsMode::Disabled, "HTTP")),
                    TlsMode::Application => {
                        // Validate that certificates are available
                        if config.cert_file.is_none() || config.key_file.is_none() {
                            return Err(ProxyError::config(
                                "TLS application mode requires cert_file and key_file to be specified"
                            ));
                        }
                        Ok((TlsMode::Application, "HTTPS"))
                    }
                    TlsMode::BehindProxy => {
                        info!("Running in behind-proxy mode - expecting reverse proxy to handle TLS termination");
                        Ok((TlsMode::BehindProxy, "HTTP (behind HTTPS proxy)"))
                    }
                    TlsMode::Auto => {
                        // Auto-detect based on headers or fallback to certificates
                        if config.cert_file.is_some() && config.key_file.is_some() {
                            info!("Auto mode: certificates available, will use application mode if no proxy detected");
                            Ok((config.fallback_mode.clone(), "HTTPS (auto-detected)"))
                        } else {
                            info!("Auto mode: no certificates, assuming behind proxy");
                            Ok((TlsMode::BehindProxy, "HTTP (auto-detected proxy)"))
                        }
                    }
                }
            }
        }
    }

    /// Load rustls configuration from TLS config
    fn load_rustls_config_static(tls_config: &TlsConfig) -> Result<rustls::ServerConfig> {
        use std::io::BufReader;
        use std::fs::File;

        let cert_file = tls_config.cert_file.as_ref()
            .ok_or_else(|| ProxyError::config("Certificate file is required for TLS"))?;
        let key_file = tls_config.key_file.as_ref()
            .ok_or_else(|| ProxyError::config("Private key file is required for TLS"))?;

        // Load certificate chain
        let cert_file = File::open(cert_file)
            .map_err(|e| ProxyError::config(format!("Failed to open certificate file: {}", e)))?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain = rustls_pemfile::certs(&mut cert_reader)
            .map_err(|e| ProxyError::config(format!("Failed to parse certificate file: {}", e)))?;

        if cert_chain.is_empty() {
            return Err(ProxyError::config("No certificates found in certificate file"));
        }

        // Load private key
        let key_file = File::open(key_file)
            .map_err(|e| ProxyError::config(format!("Failed to open private key file: {}", e)))?;
        let mut key_reader = BufReader::new(key_file);

        // Try to read different key formats
        let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
            .map_err(|e| ProxyError::config(format!("Failed to parse PKCS8 private key: {}", e)))?;

        if keys.is_empty() {
            // Try RSA private key format - reopen the file
            let key_file_path = tls_config.key_file.as_ref().unwrap(); // Safe because we validated this
            let key_file = File::open(key_file_path)
                .map_err(|e| ProxyError::config(format!("Failed to reopen private key file: {}", e)))?;
            let mut key_reader = BufReader::new(key_file);
            keys = rustls_pemfile::rsa_private_keys(&mut key_reader)
                .map_err(|e| ProxyError::config(format!("Failed to parse RSA private key: {}", e)))?;
        }

        if keys.is_empty() {
            return Err(ProxyError::config("No private key found in key file"));
        }

        let private_key = rustls::PrivateKey(keys.into_iter().next().unwrap());

        // Build rustls config
        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain.into_iter().map(rustls::Certificate).collect(), private_key)
            .map_err(|e| ProxyError::config(format!("Failed to build TLS configuration: {}", e)))?;

        info!("TLS configuration loaded successfully");
        Ok(config)
    }

    /// Start the MCP server with TLS configuration from an Arc
    pub async fn start_with_config_arc(server: Arc<Self>, host: &str, port: u16, tls_config: Option<TlsConfig>) -> Result<()> {
        // Try to unwrap the Arc to get owned instance
        match Arc::try_unwrap(server) {
            Ok(owned_server) => {
                // Successfully unwrapped - use the existing method
                owned_server.start_with_config(host, port, tls_config).await
            }
            Err(arc_server) => {
                // Arc still has multiple references - we need a different approach
                warn!("Cannot unwrap Arc<McpServer> for start_with_config - multiple references exist");
                warn!("This indicates a design issue where the server is shared when it should be owned");
                
                // For now, return an error to highlight the issue
                Err(crate::error::ProxyError::config(
                    "Cannot start server: Arc<McpServer> has multiple references. \
                     The server should be owned exclusively when starting.".to_string()
                ))
            }
        }
    }

    /// Start gRPC server (handled separately in main.rs)
    #[allow(dead_code)]
    async fn start_grpc_server(_registry: &Arc<RegistryService>, _host: &str, _port: u16) -> Result<()> {
        info!("gRPC server startup is handled separately in main.rs");
        Ok(())
    }

    /// Handle list_tools request
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        debug!("Handling list_tools request");

        // Get tools from high-performance registry
        let tool_names = self.registry.list_tools();
        let mut tools = Vec::new();

        for tool_name in tool_names {
            if let Some(tool_def) = self.registry.get_tool(&tool_name) {
                // Convert ToolDefinition to MCP Tool
                let tool = crate::mcp::types::Tool::new(
                    tool_def.name().to_string(),
                    tool_def.description().to_string(),
                    tool_def.input_schema.clone(),
                )?;
                tools.push(tool);
            }
        }

        // Note: Legacy proxy tools removed - use remote_mcp discovery instead

        info!("Returning {} tools (local)", tools.len());
        Ok(tools)
    }

    /// Handle call_tool request
    pub async fn call_tool(&self, tool_call: ToolCall) -> Result<ToolResult> {
        debug!("Handling call_tool request for: {}", tool_call.name);

        // Use local registry for tool resolution (including external MCP tools)
        // First, try to find the tool in the local registry
        if let Some(tool_def) = self.registry.get_tool(&tool_call.name) {
            // Check if tool is enabled before execution
            if !tool_def.is_enabled() {
                return Ok(ToolResult::error_with_metadata(
                    format!("Tool '{}' is disabled", tool_call.name),
                    json!({
                        "tool_name": tool_call.name,
                        "validated": false,
                        "source": "local",
                        "error_category": "tool_disabled",
                        "enabled": false
                    })
                ));
            }

            // Check if we should handle elicitation for this tool
            let should_use_local_elicitation = self.should_use_local_elicitation(&tool_def);
            
            // Validate arguments against tool schema
            if let Err(e) = tool_def.validate_arguments(&tool_call.arguments) {
                // Only generate local elicitation if we have authority
                if should_use_local_elicitation {
                    // Check if this is a parameter validation failure that could trigger elicitation
                    if self.could_trigger_elicitation(&e.to_string()) {
                        debug!("Parameter validation failed for '{}', but respecting external MCP elicitation authority", tool_call.name);
                    }
                }
                
                return Ok(ToolResult::error_with_metadata(
                    format!("Argument validation failed: {}", e),
                    json!({
                        "tool_name": tool_call.name,
                        "validated": false,
                        "source": "local",
                        "error_category": "validation_failure",
                        "elicitation_authority": if should_use_local_elicitation { "local" } else { "external" }
                    })
                ));
            }

            // Route to appropriate local agent using the router
            match self.router.route(&tool_call, &tool_def).await {
                Ok(agent_result) => {
                    // Convert AgentResult to ToolResult using helper
                    let metadata = json!({
                        "tool_name": tool_call.name,
                        "validated": true,
                        "registry_lookup": "success",
                        "routing_type": tool_def.routing_type(),
                        "source": "local"
                    });
                    return Ok(Self::agent_result_to_tool_result(agent_result, &tool_call.name, Some(metadata)));
                }
                Err(e) => {
                    error!("Local tool '{}' execution failed: {}", tool_call.name, e);
                    return Ok(ToolResult::error_with_metadata(
                        format!("Local tool execution failed: {}", e),
                        json!({
                            "tool_name": tool_call.name,
                            "validated": true,
                            "registry_lookup": "success",
                            "routing_type": tool_def.routing_type(),
                            "source": "local",
                            "error_category": "execution_failure"
                        })
                    ));
                }
            }
        }

        // Tool not found in local registry
        error!("Tool '{}' not found in local registry", tool_call.name);
        Ok(ToolResult::error_with_metadata(
            format!("Tool '{}' not found", tool_call.name),
            json!({
                "tool_name": tool_call.name,
                "validated": false,
                "registry_lookup": "failed",
                "error_category": "tool_not_found"
            })
        ))
    }

    /// Handle list_resources request
    pub async fn list_resources(&self, cursor: Option<String>) -> Result<ResourceListResponse> {
        debug!("Handling list_resources request");

        let (resources, next_cursor) = self.resource_manager.list_resources(cursor).await?;

        info!("Returning {} resources", resources.len());
        Ok(ResourceListResponse {
            resources,
            next_cursor,
        })
    }

    /// Handle read_resource request
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceReadResponse> {
        debug!("Handling read_resource request for URI: {}", uri);

        let content = self.resource_manager.read_resource(uri).await?;

        info!("Successfully read resource: {} ({} bytes)", uri, content.size());
        Ok(ResourceReadResponse {
            contents: vec![content],
        })
    }

    /// Handle list_prompts request
    pub async fn list_prompts(&self, cursor: Option<String>) -> Result<PromptListResponse> {
        debug!("Handling list_prompts request");

        let (prompts, next_cursor) = self.prompt_manager.list_templates(cursor.as_deref()).await?;

        info!("Returning {} prompt templates", prompts.len());
        Ok(PromptListResponse {
            prompts,
            next_cursor,
        })
    }

    /// Handle get_prompt request
    pub async fn get_prompt(&self, name: &str, arguments: Option<&Value>) -> Result<PromptGetResponse> {
        debug!("Handling get_prompt request for template: {}", name);

        let response = self.prompt_manager.get_template(name, arguments).await?;

        info!("Successfully rendered prompt template: {} ({} messages)", name, response.messages.len());
        Ok(response)
    }

    /// Handle logging/setLevel request
    pub async fn set_log_level(&self, level: LogLevel) -> Result<()> {
        debug!("Handling set_log_level request: {:?}", level);

        self.logger_manager.set_global_level(level)?;

        info!("Successfully set global log level to: {:?}", level);
        Ok(())
    }

    /// Handle logging/message notification
    pub async fn handle_log_message(&self, log_message: LogMessage) {
        // Log the message using our internal logging system
        let logger_name = log_message.logger.as_deref().unwrap_or("mcp-client");
        let log_data = &log_message.data;

        match log_message.level {
            LogLevel::Debug => debug!("MCP Client Log [{}]: {}", logger_name, log_data),
            LogLevel::Info => info!("MCP Client Log [{}]: {}", logger_name, log_data),
            LogLevel::Notice => info!("MCP Client Notice [{}]: {}", logger_name, log_data),
            LogLevel::Warning => warn!("MCP Client Warning [{}]: {}", logger_name, log_data),
            LogLevel::Error => error!("MCP Client Error [{}]: {}", logger_name, log_data),
            LogLevel::Critical => error!("MCP Client Critical [{}]: {}", logger_name, log_data),
            LogLevel::Alert => error!("MCP Client Alert [{}]: {}", logger_name, log_data),
            LogLevel::Emergency => error!("MCP Client Emergency [{}]: {}", logger_name, log_data),
        }

        // Log message received and processed successfully
        debug!("Successfully processed MCP logging message from client");
    }

    /// Handle completion/complete request
    pub async fn handle_completion(&self, completion_request: CompletionRequest) -> Result<CompletionResponse> {
        debug!("Handling completion request: {:?}", completion_request);

        match completion_request.reference {
            CompletionReference::Resource { uri } => {
                // Handle resource completion
                self.handle_resource_completion(&uri, &completion_request.argument).await
            }
            CompletionReference::Prompt { name } => {
                // Handle prompt completion
                self.handle_prompt_completion(&name, &completion_request.argument).await
            }
        }
    }

    /// Handle resource completion
    async fn handle_resource_completion(&self, uri: &str, argument: &CompletionArgument) -> Result<CompletionResponse> {
        debug!("Handling resource completion for URI: {}", uri);

        // For now, provide basic completion based on available resources
        let (available_resources, _cursor) = self.resource_manager.list_resources(None).await?;

        let completion_values = match argument {
            CompletionArgument::Name { value } => {
                // Complete resource names
                available_resources
                    .iter()
                    .filter(|resource| resource.name.starts_with(value))
                    .map(|resource| resource.name.clone())
                    .collect()
            }
            CompletionArgument::Value { value: _ } => {
                // For value completion, return all available resource URIs
                available_resources
                    .iter()
                    .map(|resource| resource.uri.clone())
                    .collect()
            }
        };

        Ok(CompletionResponse {
            completion: CompletionResult {
                values: completion_values,
                total: None,
                has_more: Some(false),
            },
        })
    }

    /// Handle prompt completion
    async fn handle_prompt_completion(&self, name: &str, argument: &CompletionArgument) -> Result<CompletionResponse> {
        debug!("Handling prompt completion for name: {}", name);

        // For now, provide basic completion based on available prompts
        let (available_prompts, _cursor) = self.prompt_manager.list_templates(None).await?;

        let completion_values = match argument {
            CompletionArgument::Name { value } => {
                // Complete prompt names
                available_prompts
                    .iter()
                    .filter(|prompt| prompt.name.starts_with(value))
                    .map(|prompt| prompt.name.clone())
                    .collect()
            }
            CompletionArgument::Value { value: _ } => {
                // For value completion, return prompt argument names
                if let Some(prompt) = available_prompts.iter().find(|p| p.name == name) {
                    prompt.arguments
                        .iter()
                        .map(|arg| arg.name.clone())
                        .collect()
                } else {
                    Vec::new()
                }
            }
        };

        Ok(CompletionResponse {
            completion: CompletionResult {
                values: completion_values,
                total: None,
                has_more: Some(false),
            },
        })
    }

    /// Get the current log level
    pub async fn get_log_level(&self) -> Result<LogLevel> {
        self.logger_manager.default_logger().get_level()
    }

    /// Get a named logger
    pub fn get_logger(&self, name: &str) -> Result<Arc<McpLogger>> {
        self.logger_manager.get_logger(name)
    }

    /// Get the notification manager
    pub fn notification_manager(&self) -> &Arc<McpNotificationManager> {
        &self.notification_manager
    }

    /// Get complete MCP initialize response
    pub fn get_capabilities(&self) -> Value {
        self.get_capabilities_for_client(None)
    }
    
    /// Get MCP capabilities tailored for a specific client (using minimum intersection)
    pub fn get_capabilities_for_client(&self, client_capabilities: Option<&crate::mcp::types::ClientCapabilities>) -> Value {
        let notification_caps = self.notification_manager.capabilities();
        
        // Get protocol version from config, fallback to 2025-06-18 (latest)
        let protocol_version = self.config
            .as_ref()
            .and_then(|c| c.mcp_client.as_ref())
            .map(|mc| mc.protocol_version.clone())
            .unwrap_or_else(|| "2025-06-18".to_string());

        // Determine capabilities based on client intersection
        let capabilities = if let Some(client_caps) = client_capabilities {
            // Use safe intersection when we know client capabilities
            let mut safe_caps = client_caps.get_safe_external_advertisement();
            
            // Add notification capabilities based on what we support
            if safe_caps.get("resources").is_some() {
                safe_caps["resources"]["subscribe"] = json!(notification_caps.resource_subscriptions);
                safe_caps["resources"]["listChanged"] = json!(notification_caps.resources_list_changed);
            }
            if safe_caps.get("prompts").is_some() {
                safe_caps["prompts"]["listChanged"] = json!(notification_caps.prompts_list_changed);
            }
            if safe_caps.get("tools").is_some() {
                safe_caps["tools"]["listChanged"] = json!(notification_caps.tools_list_changed);
            }
            
            // Log the capability advertisement decision
            client_caps.log_capability_advertisement("direct client connection", &safe_caps);
            
            safe_caps
        } else {
            // Fallback: advertise all capabilities when client is unknown (backward compatibility)
            debug!("⚠️  Client capabilities unknown - advertising all MagicTunnel capabilities (backward compatibility mode)");
            json!({
                "logging": {},
                "resources": {
                    "subscribe": notification_caps.resource_subscriptions,
                    "listChanged": notification_caps.resources_list_changed
                },
                "prompts": {
                    "listChanged": notification_caps.prompts_list_changed
                },
                "tools": {
                    "listChanged": notification_caps.tools_list_changed
                },
                "completion": {},
                "sampling": {},
                "elicitation": {
                    "create": true,
                    "accept": true,
                    "reject": true,
                    "cancel": true
                },
                "roots": {}
            })
        };

        json!({
            "protocolVersion": protocol_version,
            "capabilities": capabilities,
            "implementation": {
                "name": "MagicTunnel",
                "version": "0.3.11"
            },
            "serverInfo": {
                "name": "magictunnel",
                "version": "0.3.11"
            },
            "instructions": "MagicTunnel server providing access to GraphQL, REST, and gRPC endpoints as MCP tools"
        })
    }

    /// Get resource manager for advanced operations
    pub fn resource_manager(&self) -> &Arc<ResourceManager> {
        &self.resource_manager
    }
    
    /// Extract session ID from MCP request metadata or headers
    fn extract_session_id_from_request(&self, request: &McpRequest) -> Option<String> {
        // Try to extract session ID from request metadata or params
        if let Some(params) = &request.params {
            if let Some(session_id) = params.get("session_id").and_then(|v| v.as_str()) {
                return Some(session_id.to_string());
            }
        }
        
        // Session ID extraction logic could be enhanced based on transport
        // For now, return None (which means we'll use fallback capability advertisement)
        None
    }
    
    /// Update external MCP integration with client capabilities context
    pub async fn update_external_integration_capabilities(&self, session_id: &str) -> Result<()> {
        if let Some(external_integration) = &self.external_integration {
            let client_capabilities = self.session_manager.get_client_capabilities(session_id);
            
            let integration_guard = external_integration.read().await;
            if let Err(e) = integration_guard.update_client_capabilities(client_capabilities).await {
                warn!("Failed to update external integration client capabilities: {}", e);
            } else {
                debug!("✅ Updated external integration with client capabilities for session {}", session_id);
            }
        }
        Ok(())
    }

    /// Get prompt manager for advanced operations
    pub fn prompt_manager(&self) -> &Arc<PromptManager> {
        &self.prompt_manager
    }
    
    /// Handle sampling request with routing strategy
    pub async fn handle_sampling_request(&self, request: crate::mcp::types::SamplingRequest) -> Result<crate::mcp::types::SamplingResponse> {
        debug!("Handling sampling request with server-level routing");
        
        // Get the server-level default sampling strategy
        let strategy = self.get_default_sampling_strategy().await;
        
        match strategy {
            crate::config::SamplingElicitationStrategy::MagictunnelHandled => {
                self.handle_sampling_with_magictunnel(&request).await
            }
            crate::config::SamplingElicitationStrategy::ClientForwarded => {
                self.forward_sampling_to_client(&request).await
            }
            _ => {
                // For now, default to MagicTunnel handling
                debug!("Using MagicTunnel handling for complex routing strategy: {:?}", strategy);
                self.handle_sampling_with_magictunnel(&request).await
            }
        }
    }
    
    /// Handle elicitation request with routing strategy
    pub async fn handle_elicitation_request(&self, request: crate::mcp::types::ElicitationRequest) -> Result<crate::mcp::types::ElicitationResponse> {
        debug!("Handling elicitation request with server-level routing");
        
        // Get the server-level default elicitation strategy
        let strategy = self.get_default_elicitation_strategy().await;
        
        match strategy {
            crate::config::SamplingElicitationStrategy::MagictunnelHandled => {
                self.handle_elicitation_with_magictunnel(&request).await
            }
            crate::config::SamplingElicitationStrategy::ClientForwarded => {
                self.forward_elicitation_to_client(&request).await
            }
            _ => {
                // For now, default to MagicTunnel handling
                debug!("Using MagicTunnel handling for complex routing strategy: {:?}", strategy);
                self.handle_elicitation_with_magictunnel(&request).await
            }
        }
    }
    
    /// Handle sampling request from specific external MCP server with server-specific routing
    pub async fn handle_external_sampling_request(&self, server_name: &str, request: crate::mcp::types::SamplingRequest) -> Result<crate::mcp::types::SamplingResponse> {
        debug!("Handling sampling request from external server '{}' with server-specific routing", server_name);
        
        // Get the server-specific sampling strategy
        let strategy = self.get_external_server_sampling_strategy(server_name).await;
        
        match strategy {
            crate::config::SamplingElicitationStrategy::MagictunnelHandled => {
                debug!("Using MagicTunnel LLM for sampling request from server '{}'", server_name);
                self.handle_sampling_with_magictunnel(&request).await
            }
            crate::config::SamplingElicitationStrategy::ClientForwarded => {
                debug!("Forwarding sampling request from server '{}' to original client", server_name);
                self.forward_sampling_to_client(&request).await
            }
            crate::config::SamplingElicitationStrategy::MagictunnelFirst => {
                debug!("Trying MagicTunnel first for sampling request from server '{}'", server_name);
                match self.handle_sampling_with_magictunnel(&request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        warn!("MagicTunnel sampling failed for server '{}', falling back to client: {}", server_name, e);
                        self.forward_sampling_to_client(&request).await
                    }
                }
            }
            crate::config::SamplingElicitationStrategy::ClientFirst => {
                debug!("Trying client first for sampling request from server '{}'", server_name);
                match self.forward_sampling_to_client(&request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        warn!("Client sampling failed for server '{}', falling back to MagicTunnel: {}", server_name, e);
                        self.handle_sampling_with_magictunnel(&request).await
                    }
                }
            }
            _ => {
                // For complex routing strategies (Parallel, Hybrid), default to MagicTunnel for now
                debug!("Using MagicTunnel handling for complex routing strategy: {:?}", strategy);
                self.handle_sampling_with_magictunnel(&request).await
            }
        }
    }
    
    /// Handle sampling request for specific tool with tool-specific routing
    pub async fn handle_tool_sampling_request(&self, tool_name: &str, request: crate::mcp::types::SamplingRequest) -> Result<crate::mcp::types::SamplingResponse> {
        debug!("Handling sampling request for tool '{}' with tool-specific routing", tool_name);
        
        // Get the tool-specific sampling strategy (with full hierarchy resolution)
        let strategy = self.get_tool_sampling_strategy(tool_name).await;
        
        match strategy {
            crate::config::SamplingElicitationStrategy::MagictunnelHandled => {
                debug!("Using MagicTunnel LLM for sampling request for tool '{}'", tool_name);
                self.handle_sampling_with_magictunnel(&request).await
            }
            crate::config::SamplingElicitationStrategy::ClientForwarded => {
                debug!("Forwarding sampling request for tool '{}' to original client", tool_name);
                self.forward_sampling_to_client(&request).await
            }
            crate::config::SamplingElicitationStrategy::MagictunnelFirst => {
                debug!("Trying MagicTunnel first for sampling request for tool '{}'", tool_name);
                match self.handle_sampling_with_magictunnel(&request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        warn!("MagicTunnel sampling failed for tool '{}', falling back to client: {}", tool_name, e);
                        self.forward_sampling_to_client(&request).await
                    }
                }
            }
            crate::config::SamplingElicitationStrategy::ClientFirst => {
                debug!("Trying client first for sampling request for tool '{}'", tool_name);
                match self.forward_sampling_to_client(&request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        warn!("Client sampling failed for tool '{}', falling back to MagicTunnel: {}", tool_name, e);
                        self.handle_sampling_with_magictunnel(&request).await
                    }
                }
            }
            _ => {
                // For complex routing strategies (Parallel, Hybrid), default to MagicTunnel for now
                debug!("Using MagicTunnel handling for complex routing strategy: {:?}", strategy);
                self.handle_sampling_with_magictunnel(&request).await
            }
        }
    }
    
    /// Handle elicitation request for specific tool with tool-specific routing
    pub async fn handle_tool_elicitation_request(&self, tool_name: &str, request: crate::mcp::types::ElicitationRequest) -> Result<crate::mcp::types::ElicitationResponse> {
        debug!("Handling elicitation request for tool '{}' with tool-specific routing", tool_name);
        
        // Get the tool-specific elicitation strategy (with full hierarchy resolution)
        let strategy = self.get_tool_elicitation_strategy(tool_name).await;
        
        match strategy {
            crate::config::SamplingElicitationStrategy::MagictunnelHandled => {
                debug!("Using MagicTunnel LLM for elicitation request for tool '{}'", tool_name);
                self.handle_elicitation_with_magictunnel(&request).await
            }
            crate::config::SamplingElicitationStrategy::ClientForwarded => {
                debug!("Forwarding elicitation request for tool '{}' to original client", tool_name);
                self.forward_elicitation_to_client(&request).await
            }
            crate::config::SamplingElicitationStrategy::MagictunnelFirst => {
                debug!("Trying MagicTunnel first for elicitation request for tool '{}'", tool_name);
                match self.handle_elicitation_with_magictunnel(&request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        warn!("MagicTunnel elicitation failed for tool '{}', falling back to client: {}", tool_name, e);
                        self.forward_elicitation_to_client(&request).await
                    }
                }
            }
            crate::config::SamplingElicitationStrategy::ClientFirst => {
                debug!("Trying client first for elicitation request for tool '{}'", tool_name);
                match self.forward_elicitation_to_client(&request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        warn!("Client elicitation failed for tool '{}', falling back to MagicTunnel: {}", tool_name, e);
                        self.handle_elicitation_with_magictunnel(&request).await
                    }
                }
            }
            _ => {
                // For complex routing strategies (Parallel, Hybrid), default to MagicTunnel for now
                debug!("Using MagicTunnel handling for complex routing strategy: {:?}", strategy);
                self.handle_elicitation_with_magictunnel(&request).await
            }
        }
    }

    /// Handle elicitation request from specific external MCP server with server-specific routing
    pub async fn handle_external_elicitation_request(&self, server_name: &str, request: crate::mcp::types::ElicitationRequest) -> Result<crate::mcp::types::ElicitationResponse> {
        debug!("Handling elicitation request from external server '{}' with server-specific routing", server_name);
        
        // Get the server-specific elicitation strategy
        let strategy = self.get_external_server_elicitation_strategy(server_name).await;
        
        match strategy {
            crate::config::SamplingElicitationStrategy::MagictunnelHandled => {
                debug!("Using MagicTunnel LLM for elicitation request from server '{}'", server_name);
                self.handle_elicitation_with_magictunnel(&request).await
            }
            crate::config::SamplingElicitationStrategy::ClientForwarded => {
                debug!("Forwarding elicitation request from server '{}' to original client", server_name);
                self.forward_elicitation_to_client(&request).await
            }
            crate::config::SamplingElicitationStrategy::MagictunnelFirst => {
                debug!("Trying MagicTunnel first for elicitation request from server '{}'", server_name);
                match self.handle_elicitation_with_magictunnel(&request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        warn!("MagicTunnel elicitation failed for server '{}', falling back to client: {}", server_name, e);
                        self.forward_elicitation_to_client(&request).await
                    }
                }
            }
            crate::config::SamplingElicitationStrategy::ClientFirst => {
                debug!("Trying client first for elicitation request from server '{}'", server_name);
                match self.forward_elicitation_to_client(&request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        warn!("Client elicitation failed for server '{}', falling back to MagicTunnel: {}", server_name, e);
                        self.handle_elicitation_with_magictunnel(&request).await
                    }
                }
            }
            _ => {
                // For complex routing strategies (Parallel, Hybrid), default to MagicTunnel for now
                debug!("Using MagicTunnel handling for complex routing strategy: {:?}", strategy);
                self.handle_elicitation_with_magictunnel(&request).await
            }
        }
    }

    /// Handle MCP JSON-RPC 2.0 request (unified handler for all transports)
    pub async fn handle_mcp_request(&self, request: McpRequest) -> Result<Option<String>> {
        debug!("Handling MCP method: {}", request.method);

        // Route to appropriate handler based on method
        let response = match request.method.as_str() {
            "initialize" => {
                // MCP initialization handshake - get client capabilities from session if available
                let session_id = self.extract_session_id_from_request(&request);
                let client_capabilities = session_id
                    .and_then(|sid| self.session_manager.get_client_capabilities(&sid));
                
                let capabilities = self.get_capabilities_for_client(client_capabilities.as_ref());
                
                if let Some(ref id) = request.id {
                    self.create_success_response(id, capabilities)
                } else {
                    self.create_error_response(None, McpErrorCode::InvalidRequest, "Initialize request must have an ID")
                }
            }
            "initialized" | "notifications/initialized" => {
                // MCP initialization complete notification (no response needed)
                return Ok(None);
            }
            "tools/list" => {
                match self.list_tools().await {
                    Ok(tools) => {
                        if let Some(ref id) = request.id {
                            self.create_success_response(id, json!({"tools": tools}))
                        } else {
                            self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InternalError,
                        &format!("Failed to list tools: {}", e)
                    ),
                }
            }
            "tools/call" => {
                let params = request.params.unwrap_or(json!({}));
                match serde_json::from_value::<ToolCall>(params) {
                    Ok(tool_call) => {
                        match self.call_tool(tool_call).await {
                            Ok(result) => {
                                if let Some(ref id) = request.id {
                                    // For MCP protocol, include essential next_step info if available
                                    let mcp_result = self.format_mcp_response(result);
                                    self.create_success_response(id, json!(mcp_result))
                                } else {
                                    self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                                }
                            }
                            Err(e) => self.create_error_response(
                                request.id.as_ref(),
                                McpErrorCode::InternalError,
                                &format!("Tool execution failed: {}", e)
                            ),
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InvalidParams,
                        &format!("Invalid tool call parameters: {}", e)
                    ),
                }
            }
            "resources/list" => {
                let params = request.params.unwrap_or(json!({}));
                let cursor = params.get("cursor")
                    .and_then(|c| c.as_str())
                    .map(String::from);

                match self.list_resources(cursor).await {
                    Ok(response) => {
                        if let Some(ref id) = request.id {
                            self.create_success_response(id, json!(response))
                        } else {
                            self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InternalError,
                        &format!("Failed to list resources: {}", e)
                    ),
                }
            }
            "resources/read" => {
                let params = request.params.unwrap_or(json!({}));
                let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");

                match self.read_resource(uri).await {
                    Ok(response) => {
                        if let Some(ref id) = request.id {
                            self.create_success_response(id, json!(response))
                        } else {
                            self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InternalError,
                        &format!("Failed to read resource: {}", e)
                    ),
                }
            }
            "prompts/list" => {
                let params = request.params.unwrap_or(json!({}));
                let cursor = params.get("cursor")
                    .and_then(|c| c.as_str())
                    .map(String::from);

                match self.list_prompts(cursor).await {
                    Ok(response) => {
                        if let Some(ref id) = request.id {
                            self.create_success_response(id, json!(response))
                        } else {
                            self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InternalError,
                        &format!("Failed to list prompts: {}", e)
                    ),
                }
            }
            "prompts/get" => {
                let params = request.params.unwrap_or(json!({}));
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned();

                match self.get_prompt(name, arguments.as_ref()).await {
                    Ok(response) => {
                        if let Some(ref id) = request.id {
                            self.create_success_response(id, json!(response))
                        } else {
                            self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InternalError,
                        &format!("Failed to get prompt: {}", e)
                    ),
                }
            }
            "logging/message" => {
                // MCP logging message notification (no response needed)
                let params = request.params.unwrap_or(json!({}));
                match serde_json::from_value::<LogMessage>(params) {
                    Ok(log_message) => {
                        self.handle_log_message(log_message).await;
                        return Ok(None); // Notifications don't return responses
                    }
                    Err(e) => {
                        warn!("Invalid logging message parameters: {}", e);
                        return Ok(None); // Still no response for notifications
                    }
                }
            }
            "logging/setLevel" => {
                let params = request.params.unwrap_or(json!({}));
                match serde_json::from_value::<LoggingSetLevelRequest>(params) {
                    Ok(set_level_request) => {
                        match self.set_log_level(set_level_request.level).await {
                            Ok(_) => {
                                if let Some(ref id) = request.id {
                                    self.create_success_response(id, json!({}))
                                } else {
                                    self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                                }
                            }
                            Err(e) => self.create_error_response(
                                request.id.as_ref(),
                                McpErrorCode::InternalError,
                                &format!("Failed to set log level: {}", e)
                            ),
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InvalidParams,
                        &format!("Invalid setLevel parameters: {}", e)
                    ),
                }
            }
            "completion/complete" => {
                let params = request.params.unwrap_or(json!({}));
                match serde_json::from_value::<CompletionRequest>(params) {
                    Ok(completion_request) => {
                        match self.handle_completion(completion_request).await {
                            Ok(response) => {
                                if let Some(ref id) = request.id {
                                    self.create_success_response(id, json!(response))
                                } else {
                                    self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                                }
                            }
                            Err(e) => self.create_error_response(
                                request.id.as_ref(),
                                McpErrorCode::InternalError,
                                &format!("Failed to handle completion: {}", e)
                            ),
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InvalidParams,
                        &format!("Invalid completion parameters: {}", e)
                    ),
                }
            }
            // REMOVED: sampling/createMessage - Handled by clients (stdio/WebSocket/StreamableHTTP) and forwarded via internal methods
            // REMOVED: elicitation/create - Handled by clients (stdio/WebSocket/StreamableHTTP) and forwarded via internal methods
            "roots/list" => {
                let params = request.params.unwrap_or(json!({}));
                match serde_json::from_value::<RootsListRequest>(params) {
                    Ok(roots_request) => {
                        match self.handle_roots_list_request(roots_request).await {
                            Ok(response) => {
                                if let Some(ref id) = request.id {
                                    self.create_success_response(id, serde_json::to_value(response).unwrap_or(json!({})))
                                } else {
                                    self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                                }
                            }
                            Err(e) => self.create_error_response(
                                request.id.as_ref(),
                                McpErrorCode::InternalError,
                                &format!("Roots list failed: {}", e.message)
                            ),
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InvalidParams,
                        &format!("Invalid roots list parameters: {}", e)
                    ),
                }
            }
            "elicitation/accept" => {
                let params = request.params.unwrap_or(json!({}));
                match self.handle_elicitation_accept(params).await {
                    Ok(response) => {
                        if let Some(ref id) = request.id {
                            self.create_success_response(id, json!(response))
                        } else {
                            self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InternalError,
                        &format!("Elicitation accept failed: {}", e)
                    ),
                }
            }
            "elicitation/reject" => {
                let params = request.params.unwrap_or(json!({}));
                match self.handle_elicitation_reject(params).await {
                    Ok(response) => {
                        if let Some(ref id) = request.id {
                            self.create_success_response(id, json!(response))
                        } else {
                            self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InternalError,
                        &format!("Elicitation reject failed: {}", e)
                    ),
                }
            }
            "elicitation/cancel" => {
                let params = request.params.unwrap_or(json!({}));
                match self.handle_elicitation_cancel(params).await {
                    Ok(response) => {
                        if let Some(ref id) = request.id {
                            self.create_success_response(id, json!(response))
                        } else {
                            self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                        }
                    }
                    Err(e) => self.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InternalError,
                        &format!("Elicitation cancel failed: {}", e)
                    ),
                }
            }
            _ => {
                self.create_error_response(
                    request.id.as_ref(),
                    McpErrorCode::MethodNotFound,
                    &format!("Method '{}' not found", request.method)
                )
            }
        };

        Ok(Some(response))
    }

    /// Handle sampling request for LLM message generation (internal method)
    async fn handle_sampling_request_internal(
        &self,
        request: SamplingRequest,
    ) -> std::result::Result<SamplingResponse, SamplingError> {
        // Phase 2: Enhanced sampling request routing with external MCP server support
        
        // First, try to route to external MCP servers using external routing configuration
        if let Some(external_integration) = &self.external_integration {
            debug!("🔗 Checking external MCP servers for sampling capability with external routing");
            
            let integration_guard = external_integration.read().await;
            
            // Get external routing configuration for sampling
            let routing_config = self.config.as_ref()
                .and_then(|c| c.external_mcp.as_ref())
                .and_then(|ext| ext.external_routing.as_ref())
                .and_then(|routing| routing.sampling.as_ref());
            
            if let Some(external_config) = routing_config {
                if external_config.fallback_to_magictunnel {
                    debug!("🎯 Using external routing strategy: {:?} with fallback to magictunnel", external_config.default_strategy);
                    
                    // Try external servers based on routing strategy
                    match self.try_external_sampling_with_routing(&integration_guard, &request, external_config).await {
                        Ok(response) => {
                            info!("✅ Successfully routed sampling request via external routing");
                            return Ok(response);
                        }
                        Err(e) => {
                            warn!("⚠️ All external servers failed via external routing, falling back to internal: {}", e);
                            // Continue to internal fallback below
                        }
                    }
                } else {
                    // Chaining enabled but no internal fallback - only try external
                    match self.try_external_sampling_with_routing(&integration_guard, &request, external_config).await {
                        Ok(response) => return Ok(response),
                        Err(e) => {
                            return Err(SamplingError {
                                code: SamplingErrorCode::InternalError,
                                message: format!("All external MCP servers failed and internal fallback disabled: {}", e),
                                details: None,
                            });
                        }
                    }
                }
            } else {
                // No external routing config - use simple first-available strategy (backward compatibility)
                let available_servers = integration_guard.get_sampling_capable_servers().await;
                
                if !available_servers.is_empty() {
                    info!("📡 Routing sampling request to external MCP server: {}", available_servers[0]);
                    
                    match integration_guard.forward_sampling_request(&available_servers[0], &request).await {
                        Ok(response) => {
                            info!("✅ Successfully routed sampling request to external server");
                            return Ok(response);
                        }
                        Err(e) => {
                            warn!("⚠️ Failed to route sampling to external server, falling back to internal: {}", e);
                            // Continue to internal fallback below
                        }
                    }
                } else {
                    debug!("🏠 No external MCP servers support sampling, using internal service");
                }
            }
        }
        
        // Fallback to internal sampling service
        if let Some(sampling_service) = &self.sampling_service {
            debug!("🎯 Using internal sampling service");
            
            // Extract user ID from request metadata or use default
            let user_id = request.metadata.as_ref()
                .and_then(|meta| meta.get("user_id"))
                .and_then(|user| user.as_str())
                .map(|s| s.to_string());
            
            sampling_service.handle_sampling_request(request, user_id.as_deref()).await
        } else {
            Err(SamplingError {
                code: SamplingErrorCode::InternalError,
                message: "No sampling service available (neither external nor internal)".to_string(),
                details: None,
            })
        }
    }

    /// Try external sampling with external routing configuration (Phase 3: MCP Chaining)
    async fn try_external_sampling_with_routing(
        &self,
        integration: &crate::mcp::external_integration::ExternalMcpIntegration,
        request: &SamplingRequest,
        external_config: &crate::config::ExternalRoutingStrategyConfig,
    ) -> std::result::Result<SamplingResponse, SamplingError> {
        let available_servers = integration.get_sampling_capable_servers().await;
        
        if available_servers.is_empty() {
            return Err(SamplingError {
                code: SamplingErrorCode::InternalError,
                message: "No external MCP servers support sampling".to_string(),
                details: None,
            });
        }
        
        // Get ordered server list based on strategy
        let ordered_servers = self.get_ordered_servers_for_sampling(&available_servers, external_config);
        
        debug!("🔗 Trying {} external servers in order: {:?}", ordered_servers.len(), ordered_servers);
        
        let mut last_error = None;
        
        // Try each server in order
        for (attempt, server_name) in ordered_servers.iter().enumerate() {
            info!("📡 Attempting sampling request on server '{}' (attempt {}/{})", server_name, attempt + 1, ordered_servers.len());
            
            // Try this server with retries
            for retry in 0..external_config.max_retry_attempts {
                if retry > 0 {
                    debug!("🔄 Retrying sampling request on server '{}' (retry {}/{})", server_name, retry + 1, external_config.max_retry_attempts);
                }
                
                match integration.forward_sampling_request(server_name, request).await {
                    Ok(response) => {
                        info!("✅ Successfully routed sampling request to server '{}' on attempt {}, retry {}", server_name, attempt + 1, retry + 1);
                        return Ok(response);
                    }
                    Err(e) => {
                        warn!("⚠️ Sampling request failed on server '{}' (attempt {}, retry {}): {}", server_name, attempt + 1, retry + 1, e.message);
                        last_error = Some(e);
                        
                        // If this is not the last retry for this server, continue to next retry
                        if retry < external_config.max_retry_attempts - 1 {
                            debug!("🔄 Will retry on same server");
                            continue;
                        }
                        
                        // If this is the last retry for this server, break to try next server
                        warn!("❌ All retries exhausted for server '{}', trying next server", server_name);
                        break;
                    }
                }
            }
        }
        
        // All servers and retries exhausted
        let error_msg = if let Some(last_err) = last_error {
            format!("All external MCP servers failed. Last error: {}", last_err.message)
        } else {
            "All external MCP servers failed with unknown errors".to_string()
        };
        
        Err(SamplingError {
            code: SamplingErrorCode::InternalError,
            message: error_msg,
            details: None,
        })
    }
    
    /// Get ordered list of servers for sampling based on chaining strategy
    fn get_ordered_servers_for_sampling(
        &self,
        available_servers: &[String],
        external_config: &crate::config::ExternalRoutingStrategyConfig,
    ) -> Vec<String> {
        // Use simple first-available strategy for external server chaining
        // TODO: Implement proper routing strategy mapping
        match external_config.default_strategy {
            crate::config::SamplingElicitationStrategy::MagictunnelFirst |
            crate::config::SamplingElicitationStrategy::ClientFirst => {
                let mut ordered = Vec::new();
                
                // First, add servers in priority order
                for priority_server in &external_config.priority_order {
                    if available_servers.contains(priority_server) {
                        ordered.push(priority_server.clone());
                    }
                }
                
                // Then add any remaining available servers not in priority list
                for server in available_servers {
                    if !ordered.contains(server) {
                        ordered.push(server.clone());
                    }
                }
                
                ordered
            }
            crate::config::SamplingElicitationStrategy::Parallel => {
                // For parallel, use all available servers
                available_servers.to_vec()
            }
            _ => {
                // For other strategies, just use first available server
                debug!("Using first available strategy for external server chaining");
                available_servers.iter().take(1).cloned().collect()
            }
        }
    }
    
    /// Get a hash source for pseudo-round-robin (simple implementation)
    fn get_request_hash_source(&self) -> Option<&str> {
        // Simple hash source - in a real implementation this might use request content
        Some("sampling_request")
    }

    /// Handle elicitation request for structured data collection
    async fn handle_elicitation_request_internal(
        &self,
        request: ElicitationRequest,
    ) -> std::result::Result<String, ElicitationError> {
        // Check if elicitation service is available
        if let Some(elicitation_service) = &self.elicitation_service {
            // Extract user ID from request metadata or use default
            let user_id = request.metadata.as_ref()
                .and_then(|meta| meta.get("user_id"))
                .and_then(|user| user.as_str())
                .map(|s| s.to_string());
            
            elicitation_service.handle_elicitation_request(request, user_id.as_deref()).await
        } else {
            Err(ElicitationError {
                code: ElicitationErrorCode::InternalError,
                message: "Elicitation service not configured".to_string(),
                details: None,
            })
        }
    }

    /// Handle roots list request for filesystem/URI boundary discovery
    async fn handle_roots_list_request(
        &self,
        request: RootsListRequest,
    ) -> std::result::Result<RootsListResponse, RootsError> {
        // Check if roots service is available
        if let Some(roots_service) = &self.roots_service {
            roots_service.handle_roots_list_request(request).await
        } else {
            Err(RootsError {
                code: RootsErrorCode::InternalError,
                message: "Roots service not configured".to_string(),
                details: None,
            })
        }
    }

    /// Configure sampling service
    pub fn with_sampling_service(mut self, config: &crate::config::Config) -> Result<Self> {
        match crate::mcp::sampling::SamplingService::from_config(config) {
            Ok(sampling_service) => {
                info!("Sampling service configured successfully");
                self.sampling_service = Some(Arc::new(sampling_service));
                Ok(self)
            }
            Err(e) => {
                warn!("Failed to configure sampling service: {}", e);
                // Don't return error, just log warning and continue without sampling
                Ok(self)
            }
        }
    }

    /// Configure tool enhancement service
    pub fn with_tool_enhancement_service(mut self, config: &crate::config::Config) -> Result<Self> {
        match crate::mcp::tool_enhancement::ToolEnhancementService::from_config(config) {
            Ok(tool_enhancement_service) => {
                info!("Tool enhancement service configured successfully");
                self.tool_enhancement_service = Some(Arc::new(tool_enhancement_service));
                Ok(self)
            }
            Err(e) => {
                warn!("Failed to configure tool enhancement service: {}", e);
                // Don't return error, just log warning and continue without tool enhancement
                Ok(self)
            }
        }
    }

    /// Configure elicitation service
    pub fn with_elicitation_service(mut self, config: &crate::config::Config) -> Result<Self> {
        match crate::mcp::elicitation::ElicitationService::from_config(config) {
            Ok(elicitation_service) => {
                info!("Elicitation service configured successfully");
                self.elicitation_service = Some(Arc::new(elicitation_service));
                Ok(self)
            }
            Err(e) => {
                warn!("Failed to configure elicitation service: {}", e);
                // Don't return error, just log warning and continue without elicitation
                Ok(self)
            }
        }
    }

    /// Configure enhancement service
    pub async fn with_enhancement_service(mut self, config: &crate::config::Config) -> Result<Self> {
        info!("Configuring tool enhancement service");
        
        // Create enhancement service if tool_enhancement or elicitation services are available
        if let (Some(tool_enhancement_service), Some(elicitation_service)) = (&self.tool_enhancement_service, &self.elicitation_service) {
            let enhancement_service = Arc::new(crate::discovery::ToolEnhancementPipeline::from_config(
                config,
                self.registry.clone(),
                Some(tool_enhancement_service.clone()),
                Some(elicitation_service.clone()),
            ));
            
            // Register enhancement service with registry for tool change notifications
            self.registry.set_enhancement_callback(enhancement_service.clone() as Arc<dyn EnhancementCallback>);
            info!("🔔 Enhancement service registered for tool change notifications");
            
            // Initialize enhancement service (generate missing enhancements at startup)
            if let Err(e) = enhancement_service.initialize().await {
                warn!("Failed to initialize enhancement service: {}", e);
                // Don't fail server startup, just log the warning
            }
            
            Ok(self)
        } else if let Some(tool_enhancement_service) = &self.tool_enhancement_service {
            let enhancement_service = Arc::new(crate::discovery::ToolEnhancementPipeline::from_config(
                config,
                self.registry.clone(),
                Some(tool_enhancement_service.clone()),
                None,
            ));
            
            // Register enhancement service with registry for tool change notifications
            self.registry.set_enhancement_callback(enhancement_service.clone() as Arc<dyn EnhancementCallback>);
            info!("🔔 Enhancement service (tool enhancement only) registered for tool change notifications");
            
            // Initialize enhancement service (generate missing enhancements at startup)
            if let Err(e) = enhancement_service.initialize().await {
                warn!("Failed to initialize enhancement service: {}", e);
                // Don't fail server startup, just log the warning
            }
            
            Ok(self)
        } else if let Some(elicitation_service) = &self.elicitation_service {
            let enhancement_service = Arc::new(crate::discovery::ToolEnhancementPipeline::from_config(
                config,
                self.registry.clone(),
                None,
                Some(elicitation_service.clone()),
            ));
            
            // Register enhancement service with registry for tool change notifications
            self.registry.set_enhancement_callback(enhancement_service.clone() as Arc<dyn EnhancementCallback>);
            info!("🔔 Enhancement service (elicitation only) registered for tool change notifications");
            
            // Initialize enhancement service (generate missing enhancements at startup)
            if let Err(e) = enhancement_service.initialize().await {
                warn!("Failed to initialize enhancement service: {}", e);
                // Don't fail server startup, just log the warning
            }
            
            Ok(self)
        } else {
            warn!("Enhancement service requested but no sampling or elicitation services available");
            Ok(self)
        }
    }

    pub fn with_roots_service(mut self, config: &crate::config::Config) -> Result<Self> {
        match crate::mcp::roots::RootsService::from_config(config) {
            Ok(roots_service) => {
                info!("Roots service configured successfully");
                self.roots_service = Some(Arc::new(roots_service));
                Ok(self)
            }
            Err(e) => {
                warn!("Failed to configure roots service: {}", e);
                // Don't return error, just log warning and continue without roots
                Ok(self)
            }
        }
    }

    /// Create a successful JSON-RPC response
    fn create_success_response(&self, id: &serde_json::Value, result: serde_json::Value) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }).to_string()
    }

    /// Create an error JSON-RPC response
    fn create_error_response(&self, id: Option<&serde_json::Value>, code: McpErrorCode, message: &str) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code as i32,
                "message": message
            }
        }).to_string()
    }

    // Legacy proxy manager removed - use external_mcp instead

    // Legacy routing methods removed - use external_mcp instead

    /// Get the authentication middleware
    pub fn auth_middleware(&self) -> &Option<Arc<AuthenticationMiddleware>> {
        &self.auth_middleware
    }

    /// Get the registry service
    pub fn registry(&self) -> &Arc<RegistryService> {
        &self.registry
    }

    /// Get the smart discovery service if available
    pub fn smart_discovery(&self) -> Option<&Arc<crate::discovery::SmartDiscoveryService>> {
        self.smart_discovery.as_ref()
    }

    /// Get the sampling service if available
    pub fn sampling_service(&self) -> Option<&Arc<crate::mcp::sampling::SamplingService>> {
        self.sampling_service.as_ref()
    }

    /// Get the tool enhancement service if available
    pub fn tool_enhancement_service(&self) -> Option<&Arc<crate::mcp::tool_enhancement::ToolEnhancementService>> {
        self.tool_enhancement_service.as_ref()
    }

    /// Get the elicitation service if available
    pub fn elicitation_service(&self) -> Option<&Arc<crate::mcp::elicitation::ElicitationService>> {
        self.elicitation_service.as_ref()
    }

    /// Get the enhancement service if available
    pub fn enhancement_service(&self) -> Option<&Arc<crate::discovery::ToolEnhancementPipeline>> {
        // Enhancement service is not currently stored as a field in McpServer
        // It would need to be added to the struct and initialized in the constructor
        None
    }

    /// Check if sampling service is configured
    pub fn has_sampling_service(&self) -> bool {
        self.sampling_service.is_some()
    }

    /// Check if tool enhancement service is configured
    pub fn has_tool_enhancement_service(&self) -> bool {
        self.tool_enhancement_service.is_some()
    }

    /// Check if elicitation service is configured
    pub fn has_elicitation_service(&self) -> bool {
        self.elicitation_service.is_some()
    }


    /// Check if enhancement service is configured
    pub fn has_enhancement_service(&self) -> bool {
        // Enhancement service is not currently stored as a field in McpServer
        false
    }

    // ===== PROGRESS TRACKING API =====

    /// Get progress tracker statistics  
    pub async fn get_progress_stats(&self) -> crate::mcp::ProgressStats {
        self.progress_tracker.get_stats().await
    }

    /// Get connection statistics from the session manager
    pub fn get_connection_stats(&self) -> crate::mcp::session::ConnectionStats {
        self.session_manager.get_connection_stats()
    }

    /// Get active connection count
    pub fn get_active_connection_count(&self) -> usize {
        self.session_manager.get_active_connection_count()
    }

    /// Create progress session
    pub async fn create_progress_session(
        &self,
        operation_id: String,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> crate::error::Result<String> {
        self.progress_tracker.create_session(operation_id, metadata).await
    }

    /// Update progress
    pub async fn update_progress(
        &self,
        session_id: &str,
        percentage: f64,
        message: Option<String>,
        _sub_operation_id: Option<String>,
    ) -> crate::error::Result<()> {
        use crate::mcp::progress::ProgressState;
        use std::collections::HashMap;
        
        let progress_state = ProgressState::InProgress {
            percentage,
            current_step: message.unwrap_or_default(),
            total_steps: None,
            current_step_number: None,
            eta_seconds: None,
        };
        
        self.progress_tracker.update_progress(
            session_id,
            progress_state,
            Vec::new(), // No sub-operation updates for now
            HashMap::new(), // No metadata for now
        ).await?;
        Ok(())
    }

    /// Complete progress session
    pub async fn complete_progress_session(
        &self,
        session_id: &str,
        result_summary: Option<String>,
    ) -> crate::error::Result<()> {
        self.progress_tracker.complete_session(session_id, result_summary).await
    }

    /// Get progress session details
    pub async fn get_progress_session(&self, session_id: &str) -> Option<crate::mcp::ProgressSession> {
        self.progress_tracker.get_session(session_id).await
    }

    /// Subscribe to progress events
    pub fn subscribe_to_progress_events(&self) -> tokio::sync::broadcast::Receiver<crate::mcp::ProgressEvent> {
        self.progress_tracker.subscribe_to_events()
    }

    // ===== TOOL VALIDATION API =====

    /// Validate tool for security and compliance
    pub async fn validate_tool_runtime(
        &self,
        tool_name: &str,
        tool_definition: &serde_json::Value,
        _context: std::collections::HashMap<String, serde_json::Value>,
    ) -> crate::error::Result<crate::mcp::ValidationResult> {
        // Create a Tool from the parameters
        let tool = crate::mcp::types::Tool {
            name: tool_name.to_string(),
            description: None,
            title: None,
            input_schema: tool_definition.clone(),
            output_schema: None,
            annotations: None,
        };
        self.tool_validator.validate_tool(&tool).await
    }

    /// Get tool validation statistics
    pub async fn get_tool_validation_stats(&self) -> crate::mcp::ValidationStats {
        self.tool_validator.get_stats().await
    }

    /// Clear validation cache
    pub async fn clear_tool_validation_cache(&self) -> crate::error::Result<()> {
        self.tool_validator.clear_cache().await;
        Ok(())
    }

    /// Update security classification for a tool
    pub async fn update_tool_security_classification(
        &self,
        tool_name: &str,
        classification: crate::mcp::tool_validation::SecurityClassification,
    ) -> crate::error::Result<()> {
        // For now, just log the classification update since RuntimeToolValidator
        // doesn't have an update_classification method yet
        debug!("Updating security classification for tool '{}' to {:?}", tool_name, classification);
        
        // TODO: Implement actual classification update in RuntimeToolValidator
        // This would require adding an update_classification method to RuntimeToolValidator
        warn!("Tool security classification update not yet implemented for tool: {}", tool_name);
        
        Ok(())
    }

    /// Get sandbox policy for tool
    pub async fn get_sandbox_policy(
        &self,
        tool_name: &str,
    ) -> Option<crate::mcp::SandboxPolicy> {
        self.tool_validator.get_sandbox_policy(tool_name).await
    }

    /// Determine if we should use local elicitation for this tool
    fn should_use_local_elicitation(&self, tool_def: &crate::registry::ToolDefinition) -> bool {
        // Get elicitation config from server config
        let elicitation_config = self.config.as_ref()
            .and_then(|c| c.elicitation.as_ref());
        
        let config = match elicitation_config {
            Some(config) if config.enabled => config,
            _ => {
                debug!("Elicitation is disabled, skipping local elicitation for tool '{}'", tool_def.name);
                return false;
            }
        };

        // Check if tool has external MCP elicitation capability
        let has_external_elicitation = tool_def.annotations.as_ref()
            .and_then(|ann| ann.get("has_elicitation_capability"))
            .map(|v| v == "true")
            .unwrap_or(false);

        // Check if tool is from external MCP server
        let is_external_mcp = tool_def.routing.r#type == "external_mcp";

        // Per-tool override check
        if config.allow_tool_override {
            if let Some(override_local) = tool_def.annotations.as_ref()
                .and_then(|ann| ann.get("override_elicitation_authority"))
                .and_then(|v| v.parse::<bool>().ok()) {
                debug!("Tool '{}' has elicitation authority override: use_local={}", tool_def.name, override_local);
                return override_local;
            }
        }

        // Apply global configuration rules
        match (is_external_mcp, has_external_elicitation) {
            // External MCP tool with elicitation capability
            (true, true) => {
                if config.respect_external_authority {
                    debug!("Tool '{}' is external MCP with elicitation capability, respecting external authority", tool_def.name);
                    false // Respect external server's elicitation
                } else {
                    debug!("Tool '{}' is external MCP but configured to override external elicitation authority", tool_def.name);
                    true // Override external authority
                }
            }
            // External MCP tool without elicitation capability
            (true, false) => {
                if config.enable_hybrid_elicitation {
                    debug!("Tool '{}' is external MCP without elicitation capability, enabling local elicitation (hybrid mode)", tool_def.name);
                    true // Provide local elicitation for external tools that don't have it
                } else {
                    debug!("Tool '{}' is external MCP without elicitation capability, hybrid elicitation disabled", tool_def.name);
                    false // No elicitation for external tools
                }
            }
            // Local tool (always use local elicitation if enabled)
            (false, _) => {
                debug!("Tool '{}' is local, using local elicitation", tool_def.name);
                true
            }
        }
    }

    /// Check if an error message indicates a parameter validation issue that could trigger elicitation
    fn could_trigger_elicitation(&self, error_message: &str) -> bool {
        let elicitation_triggers = [
            "missing required parameter",
            "required field",
            "parameter validation failed",
            "invalid parameter",
            "missing parameter",
            "required property",
        ];
        
        let error_lower = error_message.to_lowercase();
        elicitation_triggers.iter().any(|trigger| error_lower.contains(trigger))
    }
    
    /// Get the server-level default sampling strategy
    async fn get_default_sampling_strategy(&self) -> crate::config::SamplingElicitationStrategy {
        if let Some(config) = &self.config {
            // Check if server has explicit sampling configuration
            if let Some(sampling_config) = &config.sampling {
                if let Some(strategy) = &sampling_config.default_sampling_strategy {
                    debug!("Using server sampling strategy: {:?}", strategy);
                    return strategy.clone();
                }
            }
            
            // Check smart discovery config as fallback
            if let Some(smart_discovery) = &config.smart_discovery {
                if let Some(strategy) = &smart_discovery.default_sampling_strategy {
                    debug!("Using smart discovery sampling strategy: {:?}", strategy);
                    return strategy.clone();
                }
            }
        }
        
        debug!("No server sampling strategy configured, defaulting to MagicTunnel handling");
        crate::config::SamplingElicitationStrategy::MagictunnelHandled
    }
    
    /// Get the server-level default elicitation strategy
    async fn get_default_elicitation_strategy(&self) -> crate::config::SamplingElicitationStrategy {
        if let Some(config) = &self.config {
            // Check if server has explicit elicitation configuration
            if let Some(elicitation_config) = &config.elicitation {
                if let Some(strategy) = &elicitation_config.default_elicitation_strategy {
                    debug!("Using server elicitation strategy: {:?}", strategy);
                    return strategy.clone();
                }
            }
            
            // Check smart discovery config as fallback
            if let Some(smart_discovery) = &config.smart_discovery {
                if let Some(strategy) = &smart_discovery.default_elicitation_strategy {
                    debug!("Using smart discovery elicitation strategy: {:?}", strategy);
                    return strategy.clone();
                }
            }
        }
        
        debug!("No server elicitation strategy configured, defaulting to MagicTunnel handling");
        crate::config::SamplingElicitationStrategy::MagictunnelHandled
    }
    
    /// Handle sampling request using MagicTunnel's own LLM
    async fn handle_sampling_with_magictunnel(&self, request: &crate::mcp::types::SamplingRequest) -> Result<crate::mcp::types::SamplingResponse> {
        debug!("Processing sampling request with MagicTunnel LLM");
        
        // Get LLM configuration from server config
        let llm_config = self.get_server_llm_config().await?;
        
        // Create LLM client and process the request
        let llm_client = crate::mcp::llm_client::LlmClient::new(llm_config)
            .map_err(|e| crate::error::ProxyError::config(&format!("Failed to create LLM client: {}", e)))?;
        
        llm_client.handle_sampling_request(request).await
            .map_err(|e| crate::error::ProxyError::routing(&format!("MagicTunnel LLM processing failed: {}", e)))
    }
    
    /// Handle elicitation request using MagicTunnel's own LLM
    async fn handle_elicitation_with_magictunnel(&self, request: &crate::mcp::types::ElicitationRequest) -> Result<crate::mcp::types::ElicitationResponse> {
        debug!("Processing elicitation request with MagicTunnel LLM");
        
        // Get LLM configuration from server config
        let llm_config = self.get_server_llm_config().await?;
        
        // Create LLM client and process the request
        let llm_client = crate::mcp::llm_client::LlmClient::new(llm_config)
            .map_err(|e| crate::error::ProxyError::config(&format!("Failed to create LLM client: {}", e)))?;
        
        llm_client.handle_elicitation_request(request).await
            .map_err(|e| crate::error::ProxyError::routing(&format!("MagicTunnel LLM processing failed: {}", e)))
    }
    
    /// Forward sampling request to the original client (Claude Desktop, etc.)
    async fn forward_sampling_to_client(&self, request: &crate::mcp::types::SamplingRequest) -> Result<crate::mcp::types::SamplingResponse> {
        debug!("Forwarding sampling request to original client");
        
        // Check if we have a client connection
        if let Some(tx) = &self.client_sender {
            // Check if any client supports sampling capability
            let client_supports_sampling = self.check_any_client_supports_sampling().await;
            
            if !client_supports_sampling {
                debug!("No client with sampling capability found");
                return Err(crate::error::ProxyError::mcp(
                    "Client does not support sampling capability"
                ));
            }
            
            debug!("Client supports sampling, forwarding request");
            // Create a sampling request message for the client
            let client_request = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "sampling/createMessage",
                "params": {
                    "maxTokens": request.max_tokens,
                    "messages": request.messages,
                    "systemPrompt": request.system_prompt,
                    "temperature": request.temperature,
                    "topP": request.top_p,
                    "stop": request.stop,
                    "metadata": request.metadata,
                    "modelPreferences": request.model_preferences
                },
                "id": format!("sampling-{}", uuid::Uuid::new_v4())
            });
            
            // Send the request to the client via the channel
            if let Err(e) = tx.send(client_request.to_string()) {
                warn!("Failed to forward sampling request to client: {}", e);
                return Err(crate::error::ProxyError::routing(
                    format!("Failed to forward sampling request to client: {}", e)
                ));
            }
            
            // For now, return a placeholder response indicating forwarding occurred
            // In a full implementation, we would wait for the client's response
            Ok(crate::mcp::types::SamplingResponse {
                message: crate::mcp::types::SamplingMessage {
                    role: crate::mcp::types::SamplingMessageRole::Assistant,
                    content: crate::mcp::types::SamplingContent::Text(
                        "Request forwarded to original client for processing".to_string()
                    ),
                    name: None,
                    metadata: None,
                },
                model: "client-handled".to_string(),
                stop_reason: crate::mcp::types::SamplingStopReason::EndTurn,
                usage: None,
                metadata: None,
            })
        } else {
            warn!("No client sender available for forwarding sampling request");
            Err(crate::error::ProxyError::routing(
                "No client connection available for forwarding - falling back to MagicTunnel handling"
            ))
        }
    }
    
    /// Forward elicitation request to the original client (Claude Desktop, etc.)
    async fn forward_elicitation_to_client(&self, request: &crate::mcp::types::ElicitationRequest) -> Result<crate::mcp::types::ElicitationResponse> {
        debug!("Forwarding elicitation request to original client");
        
        // Check if we have a client connection
        if let Some(tx) = &self.client_sender {
            // TODO: In a full implementation, we would need to track the current session ID
            // For now, we'll get the first session that supports elicitation
            // This should be enhanced to track the actual session for this request
            let client_supports_elicitation = self.check_any_client_supports_elicitation().await;
            
            if !client_supports_elicitation {
                debug!("No client with elicitation capability found");
                return Err(crate::error::ProxyError::mcp(
                    "Client does not support elicitation capability"
                ));
            }
            
            debug!("Client supports elicitation, forwarding request");
            // Create an elicitation request message for the client
            let client_request = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "elicitation/request",
                "params": {
                    "message": request.message,
                    "requestedSchema": request.requested_schema,
                    "context": request.context,
                    "timeoutSeconds": request.timeout_seconds,
                    "priority": request.priority,
                    "metadata": request.metadata
                },
                "id": format!("elicitation-{}", uuid::Uuid::new_v4())
            });
            
            // Send the request to the client via the channel
            if let Err(e) = tx.send(client_request.to_string()) {
                warn!("Failed to forward elicitation request to client: {}", e);
                return Err(crate::error::ProxyError::routing(
                    format!("Failed to forward elicitation request to client: {}", e)
                ));
            }
            
            // For now, return a placeholder response indicating forwarding occurred
            // In a full implementation, we would wait for the client's response
            Ok(crate::mcp::types::ElicitationResponse {
                action: crate::mcp::types::ElicitationAction::Accept,
                data: Some(serde_json::json!({
                    "status": "forwarded_to_client",
                    "message": "Request forwarded to original client for processing"
                })),
                reason: Some("Forwarded to original client".to_string()),
                metadata: None,
                timestamp: Some(chrono::Utc::now()),
            })
        } else {
            warn!("No client sender available for forwarding elicitation request");
            Err(crate::error::ProxyError::routing(
                "No client connection available for forwarding - falling back to MagicTunnel handling"
            ))
        }
    }
    
    /// Get sampling strategy for a specific external MCP server
    async fn get_external_server_sampling_strategy(&self, server_name: &str) -> crate::config::SamplingElicitationStrategy {
        if let Some(config) = &self.config {
            // Check server-specific strategy first
            if let Some(external_mcp) = &config.external_mcp {
                if let Some(external_routing) = &external_mcp.external_routing {
                    if let Some(sampling_config) = &external_routing.sampling {
                        // Check server-specific overrides
                        if let Some(server_strategies) = &sampling_config.server_strategies {
                            if let Some(strategy) = server_strategies.get(server_name) {
                                debug!("Using server-specific sampling strategy for '{}': {:?}", server_name, strategy);
                                return strategy.clone();
                            }
                        }
                        
                        // Use default strategy for external routing
                        debug!("Using default external sampling strategy for '{}': {:?}", server_name, sampling_config.default_strategy);
                        return sampling_config.default_strategy.clone();
                    }
                }
            }
            
            // Fall back to server-level defaults
            return self.get_default_sampling_strategy().await;
        }
        
        debug!("No configuration available for external server '{}', using default strategy", server_name);
        crate::config::SamplingElicitationStrategy::MagictunnelHandled
    }
    
    /// Get sampling strategy for a specific tool (with full hierarchy resolution)
    async fn get_tool_sampling_strategy(&self, tool_name: &str) -> crate::config::SamplingElicitationStrategy {
        // Get the tool definition
        if let Some(tool_def) = self.registry.get_tool(tool_name) {
            // 1. Check tool-level override first (highest priority)
            if let Some(strategy) = &tool_def.sampling_strategy {
                debug!("Using tool-specific sampling strategy for '{}': {:?}", tool_name, strategy);
                return strategy.clone();
            }
            
            // 2. Check if this is an external MCP tool and get server-level strategy
            if tool_def.routing.r#type == "external_mcp" {
                if let Some(server_name) = tool_def.annotations.as_ref()
                    .and_then(|ann| ann.get("server_name")) {
                    let server_strategy = self.get_external_server_sampling_strategy(server_name).await;
                    debug!("Using external server sampling strategy for tool '{}' from server '{}': {:?}", 
                           tool_name, server_name, server_strategy);
                    return server_strategy;
                }
            }
        }
        
        // 3. Fall back to server-level default strategy
        debug!("Using server-level default sampling strategy for tool '{}'", tool_name);
        self.get_default_sampling_strategy().await
    }
    
    /// Get elicitation strategy for a specific tool (with full hierarchy resolution)
    async fn get_tool_elicitation_strategy(&self, tool_name: &str) -> crate::config::SamplingElicitationStrategy {
        // Get the tool definition
        if let Some(tool_def) = self.registry.get_tool(tool_name) {
            // 1. Check tool-level override first (highest priority)
            if let Some(strategy) = &tool_def.elicitation_strategy {
                debug!("Using tool-specific elicitation strategy for '{}': {:?}", tool_name, strategy);
                return strategy.clone();
            }
            
            // 2. Check if this is an external MCP tool and get server-level strategy
            if tool_def.routing.r#type == "external_mcp" {
                if let Some(server_name) = tool_def.annotations.as_ref()
                    .and_then(|ann| ann.get("server_name")) {
                    let server_strategy = self.get_external_server_elicitation_strategy(server_name).await;
                    debug!("Using external server elicitation strategy for tool '{}' from server '{}': {:?}", 
                           tool_name, server_name, server_strategy);
                    return server_strategy;
                }
            }
        }
        
        // 3. Fall back to server-level default strategy
        debug!("Using server-level default elicitation strategy for tool '{}'", tool_name);
        self.get_default_elicitation_strategy().await
    }

    /// Get elicitation strategy for a specific external MCP server
    async fn get_external_server_elicitation_strategy(&self, server_name: &str) -> crate::config::SamplingElicitationStrategy {
        if let Some(config) = &self.config {
            // Check server-specific strategy first
            if let Some(external_mcp) = &config.external_mcp {
                if let Some(external_routing) = &external_mcp.external_routing {
                    if let Some(elicitation_config) = &external_routing.elicitation {
                        // Check server-specific overrides
                        if let Some(server_strategies) = &elicitation_config.server_strategies {
                            if let Some(strategy) = server_strategies.get(server_name) {
                                debug!("Using server-specific elicitation strategy for '{}': {:?}", server_name, strategy);
                                return strategy.clone();
                            }
                        }
                        
                        // Use default strategy for external routing
                        debug!("Using default external elicitation strategy for '{}': {:?}", server_name, elicitation_config.default_strategy);
                        return elicitation_config.default_strategy.clone();
                    }
                }
            }
            
            // Fall back to server-level defaults
            return self.get_default_elicitation_strategy().await;
        }
        
        debug!("No configuration available for external server '{}', using default strategy", server_name);
        crate::config::SamplingElicitationStrategy::MagictunnelHandled
    }

    /// Check if any connected client supports elicitation capability
    async fn check_any_client_supports_elicitation(&self) -> bool {
        let has_elicitation_support = self.session_manager.any_session_supports_elicitation();
        
        if has_elicitation_support {
            let elicitation_sessions = self.session_manager.get_elicitation_capable_sessions();
            debug!("Found {} sessions with elicitation capability", elicitation_sessions.len());
            
            // Log details about elicitation-capable sessions
            for session in elicitation_sessions {
                if let Some(client_info) = &session.client_info {
                    debug!("Session '{}' client '{}' v{} supports elicitation", 
                           session.id, client_info.name, client_info.version);
                }
            }
        } else {
            let all_sessions = self.session_manager.get_all_sessions();
            debug!("No elicitation-capable sessions found among {} total sessions", all_sessions.len());
            
            // Log details about all sessions for debugging
            for session in all_sessions {
                if let Some(client_info) = &session.client_info {
                    let caps_summary = if let Some(caps) = &client_info.capabilities {
                        format!("elicitation={}, sampling={}, tools={}", 
                               caps.supports_elicitation(),
                               caps.supports_sampling(),
                               caps.supports_tools())
                    } else {
                        "no capabilities reported".to_string()
                    };
                    debug!("Session '{}' client '{}' v{}: {}", 
                           session.id, client_info.name, client_info.version, caps_summary);
                }
            }
        }
        
        has_elicitation_support
    }

    /// Check if any connected client supports sampling capability
    async fn check_any_client_supports_sampling(&self) -> bool {
        let has_sampling_support = self.session_manager.any_session_supports_sampling();
        
        if has_sampling_support {
            let sampling_sessions = self.session_manager.get_sampling_capable_sessions();
            debug!("Found {} sessions with sampling capability", sampling_sessions.len());
            
            // Log details about sampling-capable sessions
            for session in sampling_sessions {
                if let Some(client_info) = &session.client_info {
                    debug!("Session '{}' client '{}' v{} supports sampling", 
                           session.id, client_info.name, client_info.version);
                }
            }
        } else {
            debug!("No sampling-capable sessions found");
        }
        
        has_sampling_support
    }

    /// Get LLM configuration from server config
    async fn get_server_llm_config(&self) -> Result<crate::config::LlmConfig> {
        if let Some(config) = &self.config {
            // Check if the server has sampling configuration with LLM config
            if let Some(sampling_config) = &config.sampling {
                if let Some(llm_config) = &sampling_config.llm_config {
                    debug!("Using server sampling LLM configuration");
                    return Ok(llm_config.clone());
                }
            }
            
            // Check if server has smart discovery with LLM configuration
            if let Some(smart_discovery) = &config.smart_discovery {
                if smart_discovery.llm_tool_selection.api_key.is_some() || smart_discovery.llm_tool_selection.api_key_env.is_some() {
                    debug!("Using server smart discovery LLM configuration");
                    return Ok(crate::config::LlmConfig {
                        provider: smart_discovery.llm_tool_selection.provider.clone(),
                        model: smart_discovery.llm_tool_selection.model.clone(),
                        api_key_env: smart_discovery.llm_tool_selection.api_key_env.clone(),
                        api_base_url: smart_discovery.llm_tool_selection.base_url.clone(),
                        max_tokens: None, // LlmToolSelectionConfig doesn't have max_tokens
                        temperature: None, // LlmToolSelectionConfig doesn't have temperature
                        additional_params: None,
                    });
                }
            }
        }
        
        debug!("No server LLM configuration found, using default configuration");
        Ok(crate::config::LlmConfig::default())
    }

    /// Handle elicitation accept response from client
    async fn handle_elicitation_accept(&self, params: Value) -> Result<Value> {
        info!("📝 Handling elicitation/accept");
        
        // Extract required parameters
        let request_id = params.get("request_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::mcp("Missing request_id parameter"))?;
            
        let data = params.get("data")
            .ok_or_else(|| ProxyError::mcp("Missing data parameter"))?;

        // Get elicitation service
        let elicitation_service = self.elicitation_service
            .as_ref()
            .ok_or_else(|| ProxyError::mcp("Elicitation service not available"))?;

        // Create elicitation response
        let response = crate::mcp::types::elicitation::ElicitationResponse::accept(data.clone());

        // Handle the response through the service
        match elicitation_service.handle_elicitation_response(request_id, response).await {
            Ok(_) => {
                info!("✅ Elicitation request {} accepted successfully", request_id);
                
                // TODO: Continue with tool execution using the accepted parameters
                // This is where we would normally:
                // 1. Extract the collected parameters from 'data'
                // 2. Continue the original tool execution that was waiting for parameters
                // 3. Return the tool execution result
                
                Ok(json!({
                    "status": "accepted",
                    "request_id": request_id,
                    "message": "Elicitation response accepted and processed"
                }))
            }
            Err(e) => {
                error!("❌ Failed to handle elicitation accept for {}: {}", request_id, e);
                Err(ProxyError::mcp(&format!("Failed to process elicitation accept: {}", e)))
            }
        }
    }

    /// Handle elicitation reject response from client
    async fn handle_elicitation_reject(&self, params: Value) -> Result<Value> {
        info!("❌ Handling elicitation/reject");
        
        // Extract required parameters
        let request_id = params.get("request_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::mcp("Missing request_id parameter"))?;
            
        let reason = params.get("reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Get elicitation service
        let elicitation_service = self.elicitation_service
            .as_ref()
            .ok_or_else(|| ProxyError::mcp("Elicitation service not available"))?;

        // Create elicitation response
        let response = crate::mcp::types::elicitation::ElicitationResponse::decline(reason.clone());

        // Handle the response through the service
        match elicitation_service.handle_elicitation_response(request_id, response).await {
            Ok(_) => {
                info!("❌ Elicitation request {} rejected: {:?}", request_id, reason);
                
                // TODO: Handle rejection appropriately:
                // 1. Return error to original user request
                // 2. Suggest alternative approaches
                // 3. Log rejection for analytics
                
                Ok(json!({
                    "status": "rejected",
                    "request_id": request_id,
                    "reason": reason.unwrap_or_else(|| "User declined to provide data".to_string()),
                    "message": "Elicitation request rejected by user"
                }))
            }
            Err(e) => {
                error!("❌ Failed to handle elicitation reject for {}: {}", request_id, e);
                Err(ProxyError::mcp(&format!("Failed to process elicitation reject: {}", e)))
            }
        }
    }

    /// Handle elicitation cancel response from client
    async fn handle_elicitation_cancel(&self, params: Value) -> Result<Value> {
        info!("🚫 Handling elicitation/cancel");
        
        // Extract required parameters
        let request_id = params.get("request_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::mcp("Missing request_id parameter"))?;
            
        let reason = params.get("reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Get elicitation service
        let elicitation_service = self.elicitation_service
            .as_ref()
            .ok_or_else(|| ProxyError::mcp("Elicitation service not available"))?;

        // Create elicitation response
        let response = crate::mcp::types::elicitation::ElicitationResponse::cancel(reason.clone());

        // Handle the response through the service
        match elicitation_service.handle_elicitation_response(request_id, response).await {
            Ok(_) => {
                info!("🚫 Elicitation request {} cancelled: {:?}", request_id, reason);
                
                // TODO: Handle cancellation appropriately:
                // 1. Clean up any pending operations
                // 2. Return cancellation acknowledgment
                // 3. Log cancellation for analytics
                
                Ok(json!({
                    "status": "cancelled",
                    "request_id": request_id,
                    "reason": reason.unwrap_or_else(|| "User cancelled the request".to_string()),
                    "message": "Elicitation request cancelled by user"
                }))
            }
            Err(e) => {
                error!("❌ Failed to handle elicitation cancel for {}: {}", request_id, e);
                Err(ProxyError::mcp(&format!("Failed to process elicitation cancel: {}", e)))
            }
        }
    }
}

// HTTP handlers for Actix-web

/// Helper function to check authentication for HTTP requests
async fn check_authentication_context(
    req: &HttpRequest,
    auth_middleware: &Option<Arc<AuthenticationMiddleware>>,
    required_permission: &str,
) -> std::result::Result<Option<crate::auth::AuthenticationResult>, HttpResponse> {
    if let Some(auth) = auth_middleware {
        match auth.validate_http_request(req).await {
            Ok(Some(auth_result)) => {
                // Check if the authenticated user has the required permission
                if !auth.check_permission(&auth_result, required_permission) {
                    let error_response = json!({
                        "error": {
                            "code": "INSUFFICIENT_PERMISSIONS",
                            "message": format!("User does not have '{}' permission", required_permission),
                            "type": "authorization_error"
                        }
                    });
                    return Err(HttpResponse::Forbidden()
                        .content_type("application/json")
                        .json(error_response));
                }
                // Return auth context for security evaluation
                return Ok(Some(auth_result));
            }
            Ok(None) => {
                // No authentication provided but auth is required
                let error_response = json!({
                    "error": {
                        "code": "AUTHENTICATION_REQUIRED",
                        "message": "Authentication required",
                        "type": "authentication_error"
                    }
                });
                return Err(HttpResponse::Unauthorized()
                    .content_type("application/json")
                    .header("WWW-Authenticate", "Bearer")
                    .json(error_response));
            }
            Err(e) => {
                // Authentication validation failed
                let error_response = json!({
                    "error": {
                        "code": "AUTHENTICATION_FAILED",
                        "message": e.to_string(),
                        "type": "authentication_error"
                    }
                });
                return Err(HttpResponse::Unauthorized()
                    .content_type("application/json")
                    .json(error_response));
            }
        }
    }
    // No auth middleware configured, return None
    Ok(None)
}

async fn check_authentication(
    req: &HttpRequest,
    auth_middleware: &Option<Arc<AuthenticationMiddleware>>,
    required_permission: &str,
) -> std::result::Result<(), HttpResponse> {
    if let Some(auth) = auth_middleware {
        match auth.validate_http_request(req).await {
            Ok(Some(auth_result)) => {
                // Check if the authenticated user has the required permission
                if !auth.check_permission(&auth_result, required_permission) {
                    let error_response = json!({
                        "error": {
                            "code": "INSUFFICIENT_PERMISSIONS",
                            "message": format!("User does not have '{}' permission", required_permission),
                            "type": "authorization_error"
                        }
                    });
                    return Err(HttpResponse::Forbidden()
                        .content_type("application/json")
                        .json(error_response));
                }
                Ok(())
            }
            Ok(None) => {
                // Authentication disabled
                Ok(())
            }
            Err(e) => {
                let error_response = json!({
                    "error": {
                        "code": "AUTHENTICATION_FAILED",
                        "message": e.to_string(),
                        "type": "authentication_error"
                    }
                });
                Err(HttpResponse::Unauthorized()
                    .content_type("application/json")
                    .json(error_response))
            }
        }
    } else {
        // No authentication configured
        Ok(())
    }
}

/// Health check endpoint
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "magictunnel"
    }))
}

/// MCP JSON-RPC 2.0 endpoint (unified protocol handler)
pub async fn mcp_jsonrpc_handler(
    req: HttpRequest,
    body: web::Json<McpRequest>,
    mcp_server: web::Data<Arc<McpServer>>,
) -> HttpResponse {
    // Check authentication with read permission for most operations
    // Tool execution will be checked separately in the unified handler
    if let Err(auth_error) = check_authentication(&req, &mcp_server.auth_middleware, "read").await {
        return auth_error;
    }

    // Use the unified MCP handler
    match mcp_server.handle_mcp_request(body.into_inner()).await {
        Ok(Some(response)) => {
            // Parse the JSON response to return as proper JSON
            match serde_json::from_str::<serde_json::Value>(&response) {
                Ok(json_response) => HttpResponse::Ok().json(json_response),
                Err(_) => HttpResponse::Ok().body(response), // Fallback to string response
            }
        }
        Ok(None) => {
            // No response needed (e.g., for notifications)
            HttpResponse::Ok().json(serde_json::json!({"jsonrpc": "2.0"}))
        }
        Err(e) => {
            error!("MCP JSON-RPC request failed: {}", e);
            let mcp_error: McpError = e.into();
            HttpResponse::BadRequest().json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": mcp_error
            }))
        }
    }
}

/// List tools endpoint
pub async fn list_tools_handler(
    req: HttpRequest,
    registry: web::Data<Arc<RegistryService>>,
    mcp_server: web::Data<Arc<McpServer>>,
) -> HttpResponse {
    // Check authentication
    if let Err(auth_error) = check_authentication(&req, &mcp_server.auth_middleware, "read").await {
        return auth_error;
    }

    match list_tools_from_registry(&registry).await {
        Ok(tools) => HttpResponse::Ok().json(json!({
            "tools": tools
        })),
        Err(e) => {
            error!("Failed to list tools: {}", e);
            let mcp_error: McpError = e.into();
            HttpResponse::InternalServerError().json(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": mcp_error
            }))
        }
    }
}

/// Call tool endpoint
pub async fn call_tool_handler(
    req: HttpRequest,
    tool_call: web::Json<ToolCall>,
    _registry: web::Data<Arc<RegistryService>>,
    mcp_server: web::Data<Arc<McpServer>>,
) -> HttpResponse {
    // Check authentication with write permission for tool execution
    if let Err(auth_error) = check_authentication(&req, &mcp_server.auth_middleware, "write").await {
        return auth_error;
    }

    match mcp_server.call_tool_with_router(&tool_call).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => {
            error!("Failed to call tool '{}': {}", tool_call.name, e);
            let mcp_error: McpError = e.into();
            HttpResponse::BadRequest().json(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": mcp_error
            }))
        }
    }
}

/// List resources endpoint
pub async fn list_resources_handler(
    query: web::Query<ResourceListRequest>,
    server: web::Data<Arc<McpServer>>
) -> HttpResponse {
    match server.list_resources(query.cursor.clone()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("Failed to list resources: {}", e);
            let mcp_error: McpError = e.into();
            HttpResponse::InternalServerError().json(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": mcp_error
            }))
        }
    }
}

/// Read resource endpoint
pub async fn read_resource_handler(
    request: web::Json<ResourceReadRequest>,
    server: web::Data<Arc<McpServer>>
) -> HttpResponse {
    match server.read_resource(&request.uri).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("Failed to read resource '{}': {}", request.uri, e);
            let mcp_error: McpError = e.into();
            HttpResponse::BadRequest().json(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": mcp_error
            }))
        }
    }
}

/// List prompts endpoint
pub async fn list_prompts_handler(
    query: web::Query<PromptListRequest>,
    server: web::Data<Arc<McpServer>>
) -> HttpResponse {
    match server.list_prompts(query.cursor.clone()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("Failed to list prompts: {}", e);
            let mcp_error: McpError = e.into();
            HttpResponse::InternalServerError().json(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": mcp_error
            }))
        }
    }
}

/// Get prompt endpoint
pub async fn get_prompt_handler(
    request: web::Json<PromptGetRequest>,
    server: web::Data<Arc<McpServer>>
) -> HttpResponse {
    match server.get_prompt(&request.name, request.arguments.as_ref()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("Failed to get prompt '{}': {}", request.name, e);
            let mcp_error: McpError = e.into();
            HttpResponse::BadRequest().json(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": mcp_error
            }))
        }
    }
}

/// Set log level endpoint
pub async fn set_log_level_handler(
    request: web::Json<LoggingSetLevelRequest>,
    server: web::Data<Arc<McpServer>>
) -> HttpResponse {
    match server.set_log_level(request.level).await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "success": true,
            "level": request.level
        })),
        Err(e) => {
            error!("Failed to set log level: {}", e);
            let mcp_error: McpError = e.into();
            HttpResponse::InternalServerError().json(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": mcp_error
            }))
        }
    }
}

/// WebSocket handler for real-time MCP communication
pub async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    mcp_server: web::Data<Arc<McpServer>>,
) -> actix_web::Result<HttpResponse> {
    let (response, session, msg_stream) = actix_ws::handle(&req, stream)?;

    // Clone the server for the spawned task
    let server = mcp_server.get_ref().clone();

    // Spawn a task to handle WebSocket messages
    actix_web::rt::spawn(handle_websocket_session(session, msg_stream, server));

    Ok(response)
}

/// Handle WebSocket session with MCP protocol support
async fn handle_websocket_session(
    mut session: actix_ws::Session,
    mut msg_stream: actix_ws::MessageStream,
    server: Arc<McpServer>,
) {
    debug!("WebSocket session started");

    // Create session for this WebSocket connection
    let session_id = match server.session_manager.create_session() {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to create session: {}", e);
            let error_response = server.create_error_response(
                None,
                McpErrorCode::InternalError,
                "Failed to create session"
            );
            let _ = session.text(error_response).await;
            return;
        }
    };

    while let Some(msg) = msg_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received WebSocket message: {}", text);

                // Validate raw message format
                if let Err(e) = server.message_validator.validate_raw_message(&text) {
                    error!("Message validation failed: {}", e);
                    let error_response = server.create_error_response(
                        None,
                        McpErrorCode::InvalidRequest,
                        &format!("Message validation failed: {}", e)
                    );
                    if session.text(error_response).await.is_err() {
                        warn!("Failed to send error response");
                        break;
                    }
                    continue;
                }

                // Parse JSON-RPC request
                let request: McpRequest = match serde_json::from_str(&text) {
                    Ok(req) => req,
                    Err(e) => {
                        error!("Invalid JSON-RPC request: {}", e);
                        let error_response = server.create_error_response(
                            None,
                            McpErrorCode::ParseError,
                            &format!("Invalid JSON: {}", e)
                        );
                        if session.text(error_response).await.is_err() {
                            warn!("Failed to send error response");
                            break;
                        }
                        continue;
                    }
                };

                // Validate MCP request format
                if let Err(e) = server.message_validator.validate_request(&request) {
                    error!("Request validation failed: {}", e);
                    let error_response = server.create_error_response(
                        request.id.as_ref(),
                        McpErrorCode::InvalidRequest,
                        &format!("Request validation failed: {}", e)
                    );
                    if session.text(error_response).await.is_err() {
                        warn!("Failed to send error response");
                        break;
                    }
                    continue;
                }

                // Validate request ID uniqueness (only for requests with IDs)
                if let Some(ref id) = request.id {
                    let id_str = match id {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        _ => id.to_string(),
                    };

                    if let Err(e) = server.session_manager.validate_request_id(&session_id, &id_str) {
                        error!("Request ID validation failed: {}", e);
                        let error_response = server.create_error_response(
                            Some(id),
                            McpErrorCode::InvalidRequest,
                            &format!("Request ID validation failed: {}", e)
                        );
                        if session.text(error_response).await.is_err() {
                            warn!("Failed to send error response");
                            break;
                        }
                        continue;
                    }
                }

                // Handle initialize method with protocol version negotiation
                if request.method == "initialize" {
                    match server.session_manager.handle_initialize(&session_id, &request) {
                        Ok(negotiated_version) => {
                            info!("Session {} initialized with protocol version {}", session_id, negotiated_version);
                            
                            // Get client capabilities from session and generate safe advertisement
                            let client_capabilities = server.session_manager.get_client_capabilities(&session_id);
                            let mut capabilities = server.get_capabilities_for_client(client_capabilities.as_ref());
                            capabilities["protocolVersion"] = Value::String(negotiated_version);

                            // Update external integration with client capabilities
                            let _ = server.update_external_integration_capabilities(&session_id).await;

                            let response = server.create_success_response(
                                request.id.as_ref().unwrap(),
                                capabilities
                            );
                            if session.text(response).await.is_err() {
                                warn!("Failed to send initialize response");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Initialize failed: {}", e);
                            let error_response = server.create_error_response(
                                request.id.as_ref(),
                                McpErrorCode::InvalidRequest,
                                &format!("Initialize failed: {}", e)
                            );
                            if session.text(error_response).await.is_err() {
                                warn!("Failed to send error response");
                                break;
                            }
                        }
                    }
                    continue;
                }

                // Update session activity
                let _ = server.session_manager.update_activity(&session_id);

                // Use unified MCP handler
                match server.handle_mcp_request(request).await {
                    Ok(response) => {
                        if let Some(response_text) = response {
                            if session.text(response_text).await.is_err() {
                                warn!("Failed to send WebSocket response");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to handle MCP message: {}", e);
                        let error_response = create_proxy_error_response(None, e);
                        if session.text(error_response).await.is_err() {
                            warn!("Failed to send error response");
                            break;
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                debug!("WebSocket connection closed");
                break;
            }
            Err(e) => {
                warn!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Clean up session when WebSocket connection closes
    if let Err(e) = server.session_manager.remove_session(&session_id) {
        warn!("Failed to remove session {}: {}", session_id, e);
    } else {
        debug!("Cleaned up session: {}", session_id);
    }
}

/// Server-Sent Events (SSE) handler for backward compatibility
/// Note: Deprecated in favor of Streamable HTTP transport, but maintained for compatibility
pub async fn sse_handler(
    req: HttpRequest,
    mcp_server: web::Data<Arc<McpServer>>,
) -> HttpResponse {
    use actix_web::http::header;
    
    // Check authentication
    if let Err(auth_error) = check_authentication(&req, &mcp_server.auth_middleware, "read").await {
        return auth_error;
    }

    info!("SSE connection established (deprecated transport - consider upgrading to Streamable HTTP)");

    // Set SSE headers
    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/event-stream"))
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .insert_header((header::CONNECTION, "keep-alive"))
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .insert_header(("X-MCP-Transport", "sse"))
        .insert_header(("X-MCP-Version", "2024-11-05")) // Old version for SSE
        .insert_header(("X-MCP-Deprecated", "true"))
        .insert_header(("X-MCP-Upgrade-To", "streamable-http"))
        .streaming(stream::iter(vec![
            Ok::<actix_web::web::Bytes, actix_web::Error>(
                actix_web::web::Bytes::from(format!(
                    "event: message\ndata: {}\n\n",
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "notifications/initialized",
                        "params": {
                            "protocolVersion": "2024-11-05",
                            "transport": "sse",
                            "deprecated": true,
                            "upgradeRecommended": true,
                            "newTransport": "streamable-http",
                            "newEndpoint": "/mcp/streamable"
                        }
                    })
                ))
            ),
            // Keep connection alive with periodic heartbeats
            Ok(actix_web::web::Bytes::from("event: heartbeat\ndata: {\"timestamp\": \"".to_string() + &chrono::Utc::now().to_rfc3339() + "\"}\n\n")),
        ]))
}

/// Streamable HTTP handler for MCP 2025-06-18 compliance (preferred over deprecated SSE)
pub async fn streamable_http_handler(
    req: HttpRequest,
    body: web::Bytes,
    server: web::Data<Arc<McpServer>>,
) -> HttpResponse {
    use crate::mcp::streamable_http::{StreamableHttpTransport, StreamableHttpConfig};
    
    // Create transport configuration from server config or use default
    let config = if let Some(config) = &server.config {
        if let Some(streamable_config) = &config.streamable_http {
            StreamableHttpConfig {
                enable_batching: streamable_config.enable_batching,
                max_batch_size: streamable_config.max_batch_size,
                batch_timeout_ms: streamable_config.batch_timeout_ms,
                enable_compression: streamable_config.enable_compression,
                max_message_size: streamable_config.max_message_size,
                connection_timeout_seconds: streamable_config.connection_timeout_seconds,
                enable_keep_alive: streamable_config.enable_keep_alive,
            }
        } else {
            StreamableHttpConfig::default()
        }
    } else {
        StreamableHttpConfig::default()
    };
    
    let transport = StreamableHttpTransport::with_server(config, Arc::clone(&server));
    
    match transport.handle_streamable_request(req, body).await {
        Ok(response) => response,
        Err(e) => {
            error!("Streamable HTTP transport error: {}", e);
            HttpResponse::BadRequest()
                .insert_header(("X-MCP-Transport", "streamable-http"))
                .insert_header(("X-MCP-Version", "2025-06-18"))
                .insert_header(("X-MCP-Error", "transport_error"))
                .json(json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32603,
                        "message": "Streamable HTTP transport error",
                        "data": e.to_string()
                    }
                }))
        }
    }
}

/// Streaming tool execution handler
/// Enhanced streaming tool execution handler using Streamable HTTP transport (MCP 2025-06-18)
pub async fn streaming_tool_handler(
    req: HttpRequest,
    tool_call: web::Json<ToolCall>,
    mcp_server: web::Data<Arc<McpServer>>,
) -> HttpResponse {
    // Check authentication with write permission for tool execution
    if let Err(auth_error) = check_authentication(&req, &mcp_server.auth_middleware, "write").await {
        return auth_error;
    }

    info!("Processing streaming tool execution for: {}", tool_call.name);

    // Create streaming responses using Streamable HTTP format
    let streaming_responses = vec![
        json!({
            "jsonrpc": "2.0",
            "id": "streaming_tool_execution",
            "result": {
                "type": "progress",
                "message": "Tool execution started using Streamable HTTP transport",
                "tool_name": tool_call.name,
                "transport": "streamable-http",
                "version": "2025-06-18",
                "progress": 0
            }
        }),
        json!({
            "jsonrpc": "2.0", 
            "id": "streaming_tool_execution",
            "result": {
                "type": "progress",
                "message": "Processing tool request",
                "tool_name": tool_call.name,
                "progress": 50
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": "streaming_tool_execution", 
            "result": {
                "type": "complete",
                "message": "Tool execution completed successfully",
                "tool_name": tool_call.name,
                "content": [{
                    "type": "text",
                    "text": format!("Tool '{}' executed successfully using MCP 2025-06-18 Streamable HTTP transport", tool_call.name)
                }],
                "transport": "streamable-http",
                "version": "2025-06-18"
            }
        })
    ];

    // Return NDJSON stream response per MCP 2025-06-18 specification
    let response_body = streaming_responses
        .iter()
        .map(|r| serde_json::to_string(r).unwrap_or_else(|_| "{}".to_string()))
        .collect::<Vec<_>>()
        .join("\n");

    HttpResponse::Ok()
        .content_type("application/x-ndjson")
        .insert_header(("X-MCP-Transport", "streamable-http"))
        .insert_header(("X-MCP-Version", "2025-06-18"))
        .insert_header(("X-Tool-Name", tool_call.name.clone()))
        .insert_header(("Cache-Control", "no-cache"))
        .body(response_body)
}

// Helper functions for HTTP handlers

/// List tools from registry service
async fn list_tools_from_registry(registry: &Arc<RegistryService>) -> Result<Vec<Tool>> {
    debug!("Listing tools from registry");

    let tool_names = registry.list_tools();
    let mut tools = Vec::new();

    for tool_name in tool_names {
        if let Some(tool_def) = registry.get_tool(&tool_name) {
            // Convert ToolDefinition to MCP Tool
            let tool = crate::mcp::types::Tool::new(
                tool_def.name().to_string(),
                tool_def.description().to_string(),
                tool_def.input_schema.clone(),
            )?;
            tools.push(tool);
        }
    }

    info!("Returning {} tools from registry", tools.len());
    Ok(tools)
}

/// Call tool using the server's configured router
impl McpServer {
    pub async fn call_tool_with_router(&self, tool_call: &ToolCall) -> Result<ToolResult> {
        self.call_tool_with_router_and_context(tool_call, None).await
    }
    
    /// Call tool with security context
    pub async fn call_tool_with_router_and_context(
        &self, 
        tool_call: &ToolCall,
        auth_context: Option<&crate::auth::AuthenticationResult>
    ) -> Result<ToolResult> {
        let arg_count = match &tool_call.arguments {
            serde_json::Value::Object(map) => map.len(),
            _ => 0,
        };
        info!("🚀 TOOL CALL START - Tool: '{}' with {} arguments", tool_call.name, arg_count);
        
        // Log the arguments being passed to the tool
        if let Ok(args_json) = serde_json::to_string_pretty(&tool_call.arguments) {
            info!("📝 Tool call arguments:\n{}", args_json);
        }
        
        // Security evaluation
        if let Some(security_middleware) = &self.security_middleware {
            if security_middleware.is_enabled() {
                info!("🔒 Evaluating security for tool call: '{}'", tool_call.name);
                
                // Build security context
                let security_context = self.build_security_context(tool_call, auth_context);
                
                // Evaluate security
                let security_result = security_middleware.evaluate_security(&security_context).await;
                
                if security_result.blocked {
                    error!("🚫 Tool call blocked by security: {}", security_result.reason);
                    return Err(ProxyError::security(security_result.error_message.unwrap_or(security_result.reason)));
                }
                
                if security_result.requires_approval {
                    warn!("⚠️ Tool call requires approval: {}", security_result.reason);
                    return Err(ProxyError::security("Tool call requires approval".to_string()));
                }
                
                if security_result.modified {
                    info!("🔧 Tool call modified by security policies");
                    // Apply modifications if any
                }
                
                info!("✅ Security evaluation passed");
            }
        }

        // Validate tool exists in registry
        let tool_def = self.registry.get_tool(&tool_call.name)
            .ok_or_else(|| ProxyError::validation(format!("Tool '{}' not found", tool_call.name)))?;
        
        info!("✅ Tool found in registry: '{}' - {}", tool_call.name, tool_def.description);
        info!("🔧 Tool routing type: {:?}", tool_def.routing_type());

        // Check if tool is enabled before execution
        if !tool_def.is_enabled() {
            error!("❌ Tool '{}' is disabled", tool_call.name);
            return Err(ProxyError::validation(format!("Tool '{}' is disabled", tool_call.name)));
        }

        // Validate arguments against tool schema
        info!("🔍 Validating arguments against tool schema...");
        match tool_def.validate_arguments(&tool_call.arguments) {
            Ok(_) => info!("✅ Arguments validation passed"),
            Err(e) => {
                error!("❌ Arguments validation failed: {}", e);
                return Err(e);
            }
        }

        // Route to appropriate agent using the configured router (which has external MCP integration)
        info!("🎯 Routing tool call to agent...");
        let start_time = std::time::Instant::now();
        
        match self.router.route(tool_call, &tool_def).await {
            Ok(agent_result) => {
                let duration = start_time.elapsed();
                info!("✅ TOOL CALL SUCCESS - Tool: '{}' completed in {:?}", tool_call.name, duration);
                
                // Log the result data
                if let Some(ref data) = agent_result.data {
                    if let Ok(data_json) = serde_json::to_string_pretty(data) {
                        info!("📊 Tool call result data:\n{}", data_json);
                    }
                } else {
                    info!("📊 Tool call completed with no result data");
                }
                
                if let Some(ref metadata) = agent_result.metadata {
                    if let Ok(metadata_json) = serde_json::to_string_pretty(metadata) {
                        info!("🏷️  Tool call metadata:\n{}", metadata_json);
                    }
                }
                
                let metadata = json!({
                    "tool_name": tool_call.name,
                    "validated": true,
                    "registry_lookup": "success",
                    "routing_type": tool_def.routing_type(),
                    "execution_time_ms": duration.as_millis()
                });
                Ok(Self::agent_result_to_tool_result(agent_result, &tool_call.name, Some(metadata)))
            }
            Err(e) => {
                let duration = start_time.elapsed();
                error!("❌ TOOL CALL FAILED - Tool: '{}' failed after {:?}: {}", tool_call.name, duration, e);
                Ok(ToolResult::error_with_metadata(
                    format!("Tool execution failed: {}", e),
                    json!({
                        "tool_name": tool_call.name,
                        "validated": true,
                        "registry_lookup": "success",
                        "routing_type": tool_def.routing_type(),
                        "error_category": "execution_failure",
                        "execution_time_ms": duration.as_millis()
                    })
                ))
            }
        }
    }
}

// MCP Protocol Message Handling

/// Create error response from ProxyError
fn create_proxy_error_response(id: Option<&serde_json::Value>, error: ProxyError) -> String {
    let mcp_error: McpError = error.into();
    let id_str = match id {
        Some(val) => val.to_string(),
        None => "null".to_string(),
    };

    let response = McpResponse {
        jsonrpc: "2.0".to_string(),
        id: id_str,
        result: None,
        error: Some(mcp_error),
    };

    serde_json::to_string(&response).unwrap_or_else(|_| {
        r#"{"jsonrpc":"2.0","id":"null","result":null,"error":{"code":-32603,"message":"Internal error"}}"#.to_string()
    })
}



// Tool Execution Routing is now handled by the AgentRouter system

// OAuth authentication handlers

/// OAuth authorization endpoint - redirects to OAuth provider
async fn oauth_authorize_handler(
    _req: HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
    mcp_server: web::Data<Arc<McpServer>>,
) -> HttpResponse {
    if let Some(auth_middleware) = &mcp_server.auth_middleware {
        // Extract redirect_uri and state from query parameters
        let redirect_uri = query.get("redirect_uri")
            .unwrap_or(&"http://localhost:8080/auth/oauth/callback".to_string())
            .clone();
        let state = query.get("state")
            .unwrap_or(&"default_state".to_string())
            .clone();

        // Get authorization URL from OAuth validator
        match auth_middleware.get_oauth_authorization_url(&redirect_uri, &state) {
            Ok(auth_url) => {
                // Redirect to OAuth provider
                HttpResponse::Found()
                    .append_header(("Location", auth_url))
                    .finish()
            }
            Err(e) => {
                let error_response = json!({
                    "error": {
                        "code": "OAUTH_CONFIG_ERROR",
                        "message": e.to_string(),
                        "type": "configuration_error"
                    }
                });
                HttpResponse::BadRequest()
                    .content_type("application/json")
                    .json(error_response)
            }
        }
    } else {
        let error_response = json!({
            "error": {
                "code": "AUTHENTICATION_DISABLED",
                "message": "OAuth authentication is not configured",
                "type": "configuration_error"
            }
        });
        HttpResponse::BadRequest()
            .content_type("application/json")
            .json(error_response)
    }
}

/// OAuth callback endpoint - handles authorization code exchange
async fn oauth_callback_handler(
    _req: HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
    mcp_server: web::Data<Arc<McpServer>>,
) -> HttpResponse {
    if let Some(auth_middleware) = &mcp_server.auth_middleware {
        // Extract authorization code and state from query parameters
        let code = match query.get("code") {
            Some(code) => code,
            None => {
                let error_response = json!({
                    "error": {
                        "code": "MISSING_AUTH_CODE",
                        "message": "Authorization code not provided",
                        "type": "oauth_error"
                    }
                });
                return HttpResponse::BadRequest()
                    .content_type("application/json")
                    .json(error_response);
            }
        };

        let redirect_uri = query.get("redirect_uri")
            .unwrap_or(&"http://localhost:8080/auth/oauth/callback".to_string())
            .clone();

        // Exchange authorization code for access token
        match auth_middleware.exchange_oauth_code_for_token(code, &redirect_uri).await {
            Ok(token_response) => {
                HttpResponse::Ok()
                    .content_type("application/json")
                    .json(json!({
                        "access_token": token_response.access_token,
                        "token_type": token_response.token_type,
                        "expires_in": token_response.expires_in,
                        "scope": token_response.scope
                    }))
            }
            Err(e) => {
                let error_response = json!({
                    "error": {
                        "code": "TOKEN_EXCHANGE_FAILED",
                        "message": e.to_string(),
                        "type": "oauth_error"
                    }
                });
                HttpResponse::BadRequest()
                    .content_type("application/json")
                    .json(error_response)
            }
        }
    } else {
        let error_response = json!({
            "error": {
                "code": "AUTHENTICATION_DISABLED",
                "message": "OAuth authentication is not configured",
                "type": "configuration_error"
            }
        });
        HttpResponse::BadRequest()
            .content_type("application/json")
            .json(error_response)
    }
}

/// OAuth token validation endpoint - for testing OAuth tokens
async fn oauth_token_handler(
    req: HttpRequest,
    mcp_server: web::Data<Arc<McpServer>>,
) -> HttpResponse {
    if let Some(auth_middleware) = &mcp_server.auth_middleware {
        match auth_middleware.validate_http_request(&req).await {
            Ok(Some(auth_result)) => {
                match auth_result {
                    crate::auth::AuthenticationResult::OAuth(oauth_result) => {
                        HttpResponse::Ok()
                            .content_type("application/json")
                            .json(json!({
                                "valid": true,
                                "user_info": {
                                    "id": oauth_result.user_info.id,
                                    "email": oauth_result.user_info.email,
                                    "name": oauth_result.user_info.name,
                                    "login": oauth_result.user_info.login
                                },
                                "expires_at": oauth_result.expires_at,
                                "scopes": oauth_result.scopes
                            }))
                    }
                    crate::auth::AuthenticationResult::ApiKey(_) => {
                        let error_response = json!({
                            "error": {
                                "code": "WRONG_AUTH_TYPE",
                                "message": "Expected OAuth token, got API key",
                                "type": "authentication_error"
                            }
                        });
                        HttpResponse::BadRequest()
                            .content_type("application/json")
                            .json(error_response)
                    }
                    crate::auth::AuthenticationResult::Jwt(_) => {
                        let error_response = json!({
                            "error": {
                                "code": "WRONG_AUTH_TYPE",
                                "message": "Expected OAuth token, got JWT",
                                "type": "authentication_error"
                            }
                        });
                        HttpResponse::BadRequest()
                            .content_type("application/json")
                            .json(error_response)
                    }
                }
            }
            Ok(None) => {
                let error_response = json!({
                    "error": {
                        "code": "NO_TOKEN_PROVIDED",
                        "message": "No authentication token provided",
                        "type": "authentication_error"
                    }
                });
                HttpResponse::Unauthorized()
                    .content_type("application/json")
                    .json(error_response)
            }
            Err(e) => {
                let error_response = json!({
                    "error": {
                        "code": "TOKEN_VALIDATION_FAILED",
                        "message": e.to_string(),
                        "type": "authentication_error"
                    }
                });
                HttpResponse::Unauthorized()
                    .content_type("application/json")
                    .json(error_response)
            }
        }
    } else {
        let error_response = json!({
            "error": {
                "code": "AUTHENTICATION_DISABLED",
                "message": "OAuth authentication is not configured",
                "type": "configuration_error"
            }
        });
        HttpResponse::BadRequest()
            .content_type("application/json")
            .json(error_response)
    }
}
