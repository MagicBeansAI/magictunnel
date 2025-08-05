//! External MCP Manager
//! 
//! This module manages multiple External MCP server processes and provides
//! capability discovery, tool execution, and lifecycle management.

use crate::config::{ExternalMcpConfig, ExternalMcpServersConfig, ContainerConfig, McpClientConfig};
use crate::error::{ProxyError, Result};
use crate::mcp::external_process::ExternalMcpProcess;
use crate::mcp::types::{Tool, McpRequest, McpResponse};
use crate::mcp::metrics::{McpMetricsCollector, McpHealthThresholds, HealthStatus};
use crate::mcp::health_checker::{McpHealthChecker, HealthCheckConfig};
use crate::registry::types::{CapabilityFile, ToolDefinition, RoutingConfig};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use tokio::fs;

/// Server version and capability information
#[derive(Debug, Clone, Default)]
struct ServerVersionInfo {
    /// MCP protocol version (e.g., "2024-11-05", "2025-06-18")
    protocol_version: Option<String>,
    /// Server version from the server itself
    server_version: Option<String>,
    /// Server name/info string
    server_info: Option<String>,
    /// Supports MCP 2025-06-18 sampling capabilities
    supports_sampling: bool,
    /// Supports MCP 2025-06-18 elicitation capabilities
    supports_elicitation: bool,
    /// Supports basic tools
    supports_tools: bool,
    /// Supports resources
    supports_resources: bool,
    /// Supports prompts
    supports_prompts: bool,
    /// Supports roots
    supports_roots: bool,
    /// Raw initialization response for debugging
    raw_init_response: Option<serde_json::Value>,
}

/// Configuration for capability file versioning
#[derive(Debug, Clone)]
struct VersioningConfig {
    /// Maximum number of versions to keep (default: 10)
    max_versions: usize,
    /// Whether to enable versioning (default: true)
    enabled: bool,
    /// Whether to compress old versions (default: false)
    compress_versions: bool,
}

impl Default for VersioningConfig {
    fn default() -> Self {
        Self {
            max_versions: 10,
            enabled: true,
            compress_versions: false,
        }
    }
}

/// Manages multiple External MCP server processes
pub struct ExternalMcpManager {
    /// Configuration for External MCP
    config: ExternalMcpConfig,
    /// MCP client configuration
    client_config: McpClientConfig,
    /// Container configuration
    container_config: Option<ContainerConfig>,
    /// Active MCP server processes
    processes: Arc<RwLock<HashMap<String, ExternalMcpProcess>>>,
    /// Discovered capabilities from all servers
    capabilities: Arc<RwLock<HashMap<String, Vec<Tool>>>>,
    /// Server version and capability information
    version_info: Arc<RwLock<HashMap<String, ServerVersionInfo>>>,
    /// Metrics collector for observability
    metrics_collector: Arc<McpMetricsCollector>,
    /// Health checker for active monitoring
    health_checker: Arc<McpHealthChecker>,
    /// Client capabilities context for safe advertisement
    client_capabilities_context: Arc<RwLock<Option<crate::mcp::types::ClientCapabilities>>>,
}

impl ExternalMcpManager {
    /// Create a new External MCP Manager
    pub fn new(config: ExternalMcpConfig, client_config: McpClientConfig) -> Self {
        let container_config = config.containers.clone();

        // Initialize metrics collector with default thresholds
        let metrics_collector = Arc::new(McpMetricsCollector::new(McpHealthThresholds::default()));
        
        // Initialize health checker with default configuration
        let health_checker = Arc::new(McpHealthChecker::new(HealthCheckConfig::default()));

        Self {
            config,
            client_config,
            container_config,
            processes: Arc::new(RwLock::new(HashMap::new())),
            capabilities: Arc::new(RwLock::new(HashMap::new())),
            version_info: Arc::new(RwLock::new(HashMap::new())),
            metrics_collector,
            health_checker,
            client_capabilities_context: Arc::new(RwLock::new(None)),
        }
    }

    /// Set client capabilities context for safe advertisement to external servers
    pub async fn set_client_capabilities_context(&self, client_capabilities: Option<crate::mcp::types::ClientCapabilities>) {
        let mut context = self.client_capabilities_context.write().await;
        *context = client_capabilities;
        
        if let Some(ref caps) = *context {
            info!("🔧 Client capabilities context set for external MCP manager");
            caps.log_capability_advertisement("external MCP manager context", 
                &caps.get_safe_external_advertisement());
        } else {
            info!("⚠️  Client capabilities context cleared - using default advertisement");
        }
    }

    /// Start the External MCP Manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting External MCP Manager");
        debug!("External MCP config file: {}", self.config.config_file);
        debug!("Capabilities output dir: {}", self.config.capabilities_output_dir);
        debug!("Current working directory: {:?}", std::env::current_dir());

        // Load server configurations
        info!("Loading external MCP server configurations...");
        let servers_config = self.load_servers_config().await?;
        
        // Track successfully started servers
        let mut started_servers = 0;
        let total_servers = servers_config.mcp_servers.as_ref().map(|s| s.len()).unwrap_or(0);
        
        // Start all configured servers
        if let Some(mcp_servers) = servers_config.mcp_servers {
            for (server_name, server_config) in mcp_servers {
                match self.start_server(server_name.clone(), server_config).await {
                    Ok(_) => {
                        info!("Successfully started External MCP server: {}", server_name);
                        started_servers += 1;
                    }
                    Err(e) => {
                        error!("Failed to start External MCP server '{}': {}", server_name, e);
                    }
                }
            }
        }

        // Check if any servers started successfully
        if started_servers == 0 {
            let error_msg = if total_servers == 0 {
                "No External MCP servers configured".to_string()
            } else {
                format!("Failed to start any External MCP servers (0/{} started)", total_servers)
            };
            return Err(ProxyError::connection(error_msg));
        }

        info!("Started {}/{} External MCP servers successfully", started_servers, total_servers);

        // Initialize metrics for all servers
        self.initialize_server_metrics().await;

        // Start periodic capability discovery and health monitoring
        self.start_periodic_monitoring().await;

        // Perform initial capability discovery
        self.discover_all_capabilities().await?;

        info!("External MCP Manager started successfully");
        Ok(())
    }

    /// Load server configurations from file
    async fn load_servers_config(&self) -> Result<ExternalMcpServersConfig> {
        let config_path = &self.config.config_file;
        info!("Looking for external MCP config file: {}", config_path);
        
        // Try multiple locations for the config file
        let possible_paths = self.get_possible_config_paths(config_path);
        
        let mut found_path = None;
        for path in &possible_paths {
            debug!("Checking for config file at: {:?}", path);
            if path.exists() {
                info!("Found external MCP config file at: {:?}", path);
                found_path = Some(path.clone());
                break;
            }
        }
        
        let final_path = match found_path {
            Some(path) => path,
            None => {
                warn!("External MCP servers config file not found in any of the following locations:");
                for path in &possible_paths {
                    warn!("  - {:?}", path);
                }
                info!("Current directory: {:?}", std::env::current_dir());
                
                // Don't try to create example config in read-only filesystem
                if std::env::current_dir().map(|d| d == Path::new("/")).unwrap_or(false) {
                    warn!("Running in root directory with read-only filesystem, skipping example config creation");
                    return Ok(ExternalMcpServersConfig {
                        mcp_servers: Some(HashMap::new()),
                        http_services: None,
                        sse_services: None,
                        websocket_services: None,
                    });
                }
                
                // Try to create example config in a writable location
                let writable_path = possible_paths.into_iter()
                    .find(|p| p.parent().map(|parent| parent.is_dir() && parent != Path::new("/")).unwrap_or(false))
                    .unwrap_or_else(|| PathBuf::from(config_path));
                
                info!("Creating example config file at: {:?}", writable_path);
                self.create_example_config(&writable_path).await?;
                return Ok(ExternalMcpServersConfig {
                    mcp_servers: Some(HashMap::new()),
                    http_services: None,
                    sse_services: None,
                    websocket_services: None,
                });
            }
        };

        info!("Reading external MCP config file from: {:?}", final_path);
        let content = tokio::fs::read_to_string(&final_path).await
            .map_err(|e| ProxyError::config(format!("Failed to read config file '{:?}': {}", final_path, e)))?;

        debug!("Config file content length: {} bytes", content.len());
        let servers_config: ExternalMcpServersConfig = serde_yaml::from_str(&content)
            .map_err(|e| ProxyError::config(format!("Failed to parse config file '{}': {}", config_path, e)))?;

        info!("Loaded {} External MCP servers from config", servers_config.mcp_servers.as_ref().map(|s| s.len()).unwrap_or(0));
        if let Some(ref mcp_servers) = servers_config.mcp_servers {
            for (name, _) in mcp_servers {
                debug!("Found server configuration: {}", name);
            }
        }
        Ok(servers_config)
    }

    /// Get possible paths for the config file
    fn get_possible_config_paths(&self, config_path: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        // If it's an absolute path, only check that path
        if config_path.starts_with('/') {
            paths.push(PathBuf::from(config_path));
            return paths;
        }
        
        // 1. Current working directory (if not root)
        if let Ok(cwd) = std::env::current_dir() {
            if cwd != Path::new("/") {
                paths.push(cwd.join(config_path));
            }
        }
        
        // 2. Next to the executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                paths.push(exe_dir.join(config_path));
            }
        }
        
        // 3. In the user's home directory
        if let Some(home_dir) = dirs::home_dir() {
            paths.push(home_dir.join(".magictunnel").join(config_path));
            paths.push(home_dir.join(config_path));
        }
        
        // 4. In /etc/magictunnel (for system-wide config)
        paths.push(PathBuf::from("/etc/magictunnel").join(config_path));
        
        // 5. Check if MAGICTUNNEL_CONFIG_DIR is set
        if let Ok(config_dir) = std::env::var("MAGICTUNNEL_CONFIG_DIR") {
            paths.push(PathBuf::from(config_dir).join(config_path));
        }
        
        // 6. In the directory containing magictunnel-config.yaml (if found)
        // This helps when the main config and external config are in the same directory
        if let Ok(main_config_path) = std::env::var("MAGICTUNNEL_CONFIG") {
            if let Some(parent) = Path::new(&main_config_path).parent() {
                paths.push(parent.join(config_path));
            }
        }
        
        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        paths.retain(|path| seen.insert(path.clone()));
        
        paths
    }

    /// Create example configuration file
    async fn create_example_config(&self, config_path: &Path) -> Result<()> {
        info!("Creating example External MCP servers configuration: {:?}", config_path);

        // Copy the template file if it exists, otherwise create a basic example
        let template_path = config_path.with_extension("yaml.template");
        if template_path.exists() {
            tokio::fs::copy(&template_path, config_path).await
                .map_err(|e| ProxyError::config(format!("Failed to copy template file: {}", e)))?;
        } else {
            // Create a basic example configuration
            let example_config = r#"# External MCP Servers Configuration
# This file uses Claude Desktop's exact configuration format

mcpServers:
  # Filesystem server - File operations (WORKING EXAMPLE)
  filesystem:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
    env:
      PATH: "${PATH}"
  
  # Git server - Git operations (requires uv and mcp-server-git)
  # git:
  #   command: "uv"
  #   args: ["run", "mcp-server-git", "--repository", "."]
  #   env:
  #     PATH: "${PATH}"
  
  # SQLite server - Database operations (requires npm package)
  # sqlite:
  #   command: "npx"
  #   args: ["-y", "@modelcontextprotocol/server-sqlite", "--db-path", "./data.db"]
  #   env:
  #     PATH: "${PATH}"

# Note: Uncomment and configure servers as needed
# Ensure required dependencies are installed:
# - Node.js and npm for @modelcontextprotocol/server-* packages
# - Python and uv for Python-based MCP servers
# - Docker for containerized servers
"#;

            tokio::fs::write(config_path, example_config).await
                .map_err(|e| ProxyError::config(format!("Failed to write example config: {}", e)))?;
        }

        info!("Created example External MCP servers configuration: {}", config_path.display());
        Ok(())
    }

    /// Start a single MCP server process
    async fn start_server(&self, name: String, config: crate::config::McpServerConfig) -> Result<()> {
        debug!("Starting External MCP server: {}", name);

        // Check if server is already running
        {
            let processes = self.processes.read().await;
            if let Some(process) = processes.get(&name) {
                if process.is_running().await {
                    info!("External MCP server '{}' is already running", name);
                    return Ok(());
                }
            }
        }

        // Create and start new process
        let mut process = ExternalMcpProcess::new(name.clone(), config, self.client_config.clone());
        process.start().await?;

        // Perform MCP handshake
        match self.initialize_server(&process).await {
            Ok(_) => {
                info!("Successfully initialized External MCP server: {}", name);
            }
            Err(e) => {
                error!("Failed to initialize External MCP server '{}': {}", name, e);
                let _ = process.stop().await;
                return Err(e);
            }
        }

        // Store the process
        {
            let mut processes = self.processes.write().await;
            processes.insert(name, process);
        }

        Ok(())
    }

    /// Initialize MCP server with handshake
    async fn initialize_server(&self, process: &ExternalMcpProcess) -> Result<Value> {
        self.initialize_server_with_capabilities(process, None).await
    }
    
    /// Initialize MCP server with handshake using client capabilities for safe advertisement
    async fn initialize_server_with_capabilities(
        &self, 
        process: &ExternalMcpProcess, 
        client_capabilities: Option<&crate::mcp::types::ClientCapabilities>
    ) -> Result<Value> {
        debug!("Initializing External MCP server: {} with protocol version: {}, client: {}@{}",
               process.name,
               self.client_config.protocol_version,
               self.client_config.client_name,
               self.client_config.client_version);

        // Use provided capabilities or fall back to context
        let context_capabilities;
        let effective_capabilities = if let Some(caps) = client_capabilities {
            Some(caps)
        } else {
            // Check if we have client capabilities in our context
            context_capabilities = self.client_capabilities_context.read().await.clone();
            context_capabilities.as_ref()
        };

        // Generate safe capability advertisement based on effective client capabilities
        let capabilities = if let Some(client_caps) = effective_capabilities {
            let safe_caps = client_caps.get_safe_external_advertisement();
            
            // Log the capability advertisement
            client_caps.log_capability_advertisement(
                &format!("external MCP server '{}'", process.name),
                &safe_caps
            );
            
            safe_caps
        } else {
            // Fallback: advertise basic capabilities when client is unknown
            debug!("⚠️  Client capabilities unknown for external server '{}' - using basic capabilities", process.name);
            json!({
                "roots": {
                    "listChanged": true
                },
                "sampling": {}
            })
        };

        let params = json!({
            "protocolVersion": self.client_config.protocol_version,
            "capabilities": capabilities,
            "clientInfo": {
                "name": self.client_config.client_name,
                "version": self.client_config.client_version
            }
        });

        let response = process.send_request("initialize", Some(params)).await?;

        if let Some(error) = response.error {
            return Err(ProxyError::mcp(format!("MCP server '{}' initialization failed: {}", process.name, error.message)));
        }

        let result = response.result.ok_or_else(|| {
            ProxyError::mcp(format!("MCP server '{}' returned no result for initialize", process.name))
        })?;

        // Send initialized notification
        //let _notification_response = process.send_request("notifications/initialized", None).await?;
        // Use a much shorter timeout for notifications
        //let notification_future = process.send_request("notifications/initialized", None);
        //match tokio::time::timeout(Duration::from_millis(300), notification_future).await {
        //    Ok(_) => {}, // Got unexpected response
        //    Err(_) => {}, // Timeout is expected for notifications
        //}
        // Don't await the notification at all
        let _ = tokio::time::timeout(
            Duration::from_millis(0), // Very short timeout
            process.send_request("notifications/initialized", None)
        ).await;

        info!("Successfully initialized External MCP server: {}", process.name);
        Ok(result)
    }

    /// Initialize metrics tracking for all configured servers
    async fn initialize_server_metrics(&self) {
        let processes = self.processes.read().await;
        for server_name in processes.keys() {
            self.metrics_collector.initialize_service(server_name).await;
            info!("📊 [METRICS] Initialized metrics for server: {}", server_name);
        }
    }

    /// Start periodic monitoring including capability discovery and health checking
    async fn start_periodic_monitoring(&self) {
        let processes = Arc::clone(&self.processes);
        let capabilities = Arc::clone(&self.capabilities);
        let version_info = Arc::clone(&self.version_info);
        let metrics_collector = Arc::clone(&self.metrics_collector);
        let health_checker = Arc::clone(&self.health_checker);
        let config = self.config.clone();

        tokio::spawn(async move {
            // Capability discovery interval (longer)
            let mut discovery_interval = interval(Duration::from_secs(config.refresh_interval_minutes * 60));
            
            // Health check interval (shorter)
            let mut health_interval = interval(Duration::from_secs(30)); // Every 30 seconds

            loop {
                tokio::select! {
                    _ = discovery_interval.tick() => {
                        info!("🔍 [MONITOR] Starting periodic capability discovery for External MCP servers");
                        
                        let process_names: Vec<String> = {
                            let processes_guard = processes.read().await;
                            processes_guard.keys().cloned().collect()
                        };

                        for server_name in process_names {
                            if let Err(e) = Self::discover_server_capabilities_static(
                                &processes,
                                &capabilities,
                                &version_info,
                                &server_name,
                                &config,
                            ).await {
                                error!("Failed to discover capabilities for server '{}': {}", server_name, e);
                                
                                // Record discovery failure in metrics
                                metrics_collector.record_request_error(&server_name, "capability_discovery_failed", "tools/list").await;
                            }
                        }
                    }
                    
                    _ = health_interval.tick() => {
                        debug!("🏥 [MONITOR] Starting periodic health checks for External MCP servers");
                        
                        let process_names: Vec<String> = {
                            let processes_guard = processes.read().await;
                            processes_guard.keys().cloned().collect()
                        };

                        for server_name in process_names {
                            // Perform health check in parallel
                            let processes_clone = Arc::clone(&processes);
                            let metrics_clone = Arc::clone(&metrics_collector);
                            let health_clone = Arc::clone(&health_checker);
                            let name_clone = server_name.clone();
                            
                            tokio::spawn(async move {
                                let health_result = {
                                    let processes_guard = processes_clone.read().await;
                                    if let Some(process) = processes_guard.get(&name_clone) {
                                        health_clone.perform_ping_check(process).await
                                    } else {
                                        return;
                                    }
                                };
                                
                                // Update metrics with health check result
                                metrics_clone.update_health_status(
                                    &name_clone,
                                    health_result.status,
                                    health_result.response_time_ms,
                                ).await;
                                
                                if matches!(health_result.status, HealthStatus::Unhealthy | HealthStatus::Down) {
                                    if let Some(error) = health_result.error_details {
                                        warn!("🚨 [MONITOR] Health check failed for '{}': {}", name_clone, error);
                                    }
                                }
                            });
                        }
                    }
                }
            }
        });
    }

    /// Discover capabilities from all servers
    pub async fn discover_all_capabilities(&self) -> Result<()> {
        info!("Discovering capabilities from all External MCP servers");

        let server_names: Vec<String> = {
            let processes = self.processes.read().await;
            processes.keys().cloned().collect()
        };

        for server_name in server_names {
            if let Err(e) = self.discover_server_capabilities(&server_name).await {
                error!("Failed to discover capabilities for server '{}': {}", server_name, e);
            }
        }

        Ok(())
    }

    /// Discover capabilities from a specific server (including 2025-06-18 sampling/elicitation)
    async fn discover_server_capabilities(&self, server_name: &str) -> Result<()> {
        Self::discover_server_capabilities_static(
            &self.processes,
            &self.capabilities,
            &self.version_info,
            server_name,
            &self.config,
        ).await
    }

    /// Static method for capability discovery (used by periodic task)
    async fn discover_server_capabilities_static(
        processes: &Arc<RwLock<HashMap<String, ExternalMcpProcess>>>,
        capabilities: &Arc<RwLock<HashMap<String, Vec<Tool>>>>,
        version_info: &Arc<RwLock<HashMap<String, ServerVersionInfo>>>,
        server_name: &str,
        config: &ExternalMcpConfig,
    ) -> Result<()> {
        debug!("Discovering capabilities for External MCP server: {}", server_name);

        // We need to handle the process differently since ExternalMcpProcess doesn't implement Clone
        let process_exists = {
            let processes_guard = processes.read().await;
            processes_guard.contains_key(server_name)
        };

        if !process_exists {
            warn!("External MCP server '{}' not found", server_name);
            return Ok(());
        }

        // Check if process is running and discover tools
        let tools = {
            let processes_guard = processes.read().await;
            if let Some(process) = processes_guard.get(server_name) {
                if !process.is_running().await {
                    warn!("External MCP server '{}' is not running", server_name);
                    return Ok(());
                }

                // Step 1: Discover basic tools
                info!("Sending tools/list request to server '{}' with params: {}", server_name, json!({}));
                let mut tools = match process.send_request("tools/list", Some(json!({}))).await {
            Ok(response) => {
                info!("Received tools/list response from server '{}': {:?}", server_name, response);
                if let Some(error) = response.error {
                    error!("Failed to list tools from server '{}': {}", server_name, error.message);
                    return Err(ProxyError::mcp(format!("Tools list error: {}", error.message)));
                }

                match response.result {
                    Some(result) => {
                        match serde_json::from_value::<crate::mcp::types::ToolListResponse>(result) {
                            Ok(tool_list) => {
                                info!("Discovered {} tools from External MCP server '{}'", tool_list.tools.len(), server_name);
                                tool_list.tools
                            }
                            Err(e) => {
                                error!("Failed to parse tools from server '{}': {}", server_name, e);
                                return Err(ProxyError::mcp(format!("Failed to parse tools: {}", e)));
                            }
                        }
                    }
                    None => {
                        warn!("No tools result from server '{}'", server_name);
                        Vec::new()
                    }
                }
            }
                    Err(e) => {
                        error!("Failed to list tools from server '{}': {}", server_name, e);
                        return Err(e);
                    }
                };

                // Step 2: Query for MCP 2025-06-18 enhanced capabilities (sampling/elicitation)
                info!("Querying MCP 2025-06-18 enhanced capabilities for server '{}'", server_name);
                Self::enhance_tools_with_2025_capabilities(process, server_name, &mut tools).await;
                
                // Step 3: Fetch sampling and elicitation lists if server supports them
                Self::fetch_2025_capability_lists(process, server_name).await;
                
                // Step 4: Fetch prompts and resources if server supports them
                Self::fetch_prompts_and_resources_lists(process, server_name).await;
                
                // Return the enhanced tools
                tools
            } else {
                warn!("External MCP server '{}' not found", server_name);
                return Ok(());
            }
        };

        // Detect server version and capabilities
        let server_version_info = {
            let processes_guard = processes.read().await;
            if let Some(process) = processes_guard.get(server_name) {
                Self::detect_server_version_info(process, server_name).await
            } else {
                ServerVersionInfo::default()
            }
        };

        // Store discovered capabilities
        {
            let mut capabilities_guard = capabilities.write().await;
            capabilities_guard.insert(server_name.to_string(), tools.clone());
        }

        // Store server version information
        {
            let mut version_info_guard = version_info.write().await;
            version_info_guard.insert(server_name.to_string(), server_version_info.clone());
        }

        // Generate capability file with enhanced metadata
        Self::generate_capability_file(server_name, &tools, &server_version_info, config).await?;

        info!("Successfully discovered and generated capabilities for External MCP server: {}", server_name);
        Ok(())
    }

    /// Enhance tools with MCP 2025-06-18 capabilities (sampling/elicitation)
    async fn enhance_tools_with_2025_capabilities(
        process: &ExternalMcpProcess,
        server_name: &str,
        tools: &mut Vec<Tool>,
    ) {
        debug!("Querying MCP 2025-06-18 enhanced capabilities for {} tools", tools.len());
        
        for tool in tools.iter_mut() {
            // Query for sampling capabilities
            if let Err(e) = Self::query_tool_sampling_capability(process, server_name, tool).await {
                debug!("Failed to query sampling capability for tool '{}' on server '{}': {}", tool.name, server_name, e);
            }
            
            // Query for elicitation capabilities  
            if let Err(e) = Self::query_tool_elicitation_capability(process, server_name, tool).await {
                debug!("Failed to query elicitation capability for tool '{}' on server '{}': {}", tool.name, server_name, e);
            }
        }
        
        info!("Enhanced {} tools with MCP 2025-06-18 capabilities for server '{}'", tools.len(), server_name);
    }
    
    /// Query a specific tool's sampling capability from external MCP server
    /// Note: MCP spec doesn't define individual tool capability queries, this is experimental
    async fn query_tool_sampling_capability(
        process: &ExternalMcpProcess,
        server_name: &str,
        tool: &mut Tool,
    ) -> Result<()> {
        debug!("Testing sampling capability for tool '{}' on server '{}'", tool.name, server_name);
        
        // Since MCP spec doesn't define tool-specific capability queries,
        // we'll mark tools as sampling-capable if the server supports sampling
        match process.send_request("sampling/createMessage", Some(json!({
            "messages": [{
                "role": "user",
                "content": {
                    "type": "text", 
                    "text": format!("Test sampling for tool: {}", tool.name)
                }
            }],
            "maxTokens": 1
        }))).await {
            Ok(response) => {
                if response.error.is_none() {
                    // Server supports sampling, mark tool as sampling-capable
                    if tool.annotations.is_none() {
                        tool.annotations = Some(crate::mcp::types::ToolAnnotations::default());
                    }
                    
                    if let Some(ref mut annotations) = tool.annotations {
                        // Create a basic sampling capability indicator
                        annotations.sampling = Some(crate::mcp::types::SamplingCapability {
                            supports_description_enhancement: true,
                            enhanced_description: Some(format!("Tool '{}' supports sampling via server '{}'", tool.name, server_name)),
                            model_used: Some("external_mcp".to_string()),
                            confidence_score: Some(1.0),
                            generated_at: Some(chrono::Utc::now()),
                        });
                        debug!("✅ Marked tool '{}' as sampling-capable on server '{}'", tool.name, server_name);
                    }
                }
            }
            Err(_) => {
                debug!("Server '{}' doesn't support sampling for tool '{}'", server_name, tool.name);
            }
        }
        
        Ok(())
    }
    
    /// Test MCP 2025-06-18 advanced capabilities (sampling and elicitation)  
    async fn fetch_2025_capability_lists(
        process: &ExternalMcpProcess,
        server_name: &str,
    ) {
        debug!("Testing MCP 2025-06-18 advanced capabilities for server '{}'", server_name);
        
        // Test sampling capability by attempting a simple sampling request
        match process.send_request("sampling/createMessage", Some(json!({
            "messages": [{
                "role": "user", 
                "content": {
                    "type": "text",
                    "text": "test"
                }
            }],
            "maxTokens": 1
        }))).await {
            Ok(response) => {
                if response.error.is_none() {
                    info!("✅ Server '{}' supports MCP sampling capability", server_name);
                } else {
                    debug!("Server '{}' doesn't support sampling: {}", server_name, 
                           response.error.as_ref().map(|e| e.message.as_str()).unwrap_or("unknown error"));
                }
            }
            Err(_) => {
                debug!("Server '{}' doesn't support sampling/createMessage endpoint", server_name);
            }
        }
        
        // Test elicitation capability by attempting a simple elicitation request  
        match process.send_request("elicitation/create", Some(json!({
            "prompt": "Test elicitation capability",
            "inputType": "text",
            "required": false
        }))).await {
            Ok(response) => {
                if response.error.is_none() {
                    info!("✅ Server '{}' supports MCP elicitation capability", server_name);
                } else {
                    debug!("Server '{}' doesn't support elicitation: {}", server_name,
                           response.error.as_ref().map(|e| e.message.as_str()).unwrap_or("unknown error"));
                }
            }
            Err(_) => {
                debug!("Server '{}' doesn't support elicitation/create endpoint", server_name);
            }
        }
    }
    
    /// Fetch prompts and resources lists from external MCP server
    async fn fetch_prompts_and_resources_lists(
        process: &ExternalMcpProcess,
        server_name: &str,
    ) {
        debug!("Fetching prompts and resources lists for server '{}'", server_name);
        
        // Fetch prompts list
        match process.send_request("prompts/list", Some(json!({}))).await {
            Ok(response) => {
                if let Some(error) = response.error {
                    debug!("Server '{}' doesn't support prompts list: {}", server_name, error.message);
                } else if let Some(result) = response.result {
                    if let Ok(prompts_list) = serde_json::from_value::<crate::mcp::types::PromptListResponse>(result) {
                        info!("✅ Fetched {} prompts from server '{}'", prompts_list.prompts.len(), server_name);
                        // Store prompts for later use
                        // TODO: Store in capabilities registry or dedicated storage
                    } else {
                        debug!("Failed to parse prompts list from server '{}'", server_name);
                    }
                }
            }
            Err(_) => {
                debug!("Server '{}' doesn't support prompts list endpoint", server_name);
            }
        }
        
        // Fetch resources list
        match process.send_request("resources/list", Some(json!({}))).await {
            Ok(response) => {
                if let Some(error) = response.error {
                    debug!("Server '{}' doesn't support resources list: {}", server_name, error.message);
                } else if let Some(result) = response.result {
                    if let Ok(resources_list) = serde_json::from_value::<crate::mcp::types::ResourceListResponse>(result) {
                        info!("✅ Fetched {} resources from server '{}'", resources_list.resources.len(), server_name);
                        // Store resources for later use
                        // TODO: Store in capabilities registry or dedicated storage
                    } else {
                        debug!("Failed to parse resources list from server '{}'", server_name);
                    }
                }
            }
            Err(_) => {
                debug!("Server '{}' doesn't support resources list endpoint", server_name);
            }
        }
    }
    
    /// Query a specific tool's elicitation capability from external MCP server
    /// Note: MCP spec doesn't define individual tool capability queries, this is experimental
    async fn query_tool_elicitation_capability(
        process: &ExternalMcpProcess,
        server_name: &str,
        tool: &mut Tool,
    ) -> Result<()> {
        debug!("Testing elicitation capability for tool '{}' on server '{}'", tool.name, server_name);
        
        // Since MCP spec doesn't define tool-specific capability queries,
        // we'll mark tools as elicitation-capable if the server supports elicitation
        match process.send_request("elicitation/create", Some(json!({
            "prompt": format!("Test elicitation for tool: {}", tool.name),
            "inputType": "text",
            "required": false
        }))).await {
            Ok(response) => {
                if response.error.is_none() {
                    // Server supports elicitation, mark tool as elicitation-capable
                    if tool.annotations.is_none() {
                        tool.annotations = Some(crate::mcp::types::ToolAnnotations::default());
                    }
                    
                    if let Some(ref mut annotations) = tool.annotations {
                        // Create a basic elicitation capability indicator
                        annotations.elicitation = Some(crate::mcp::types::ElicitationCapability {
                            supports_parameter_elicitation: true,
                            enhanced_keywords: Some(vec![format!("tool_{}", tool.name), server_name.to_string()]),
                            usage_patterns: Some(vec![format!("Tool '{}' supports elicitation via server '{}'", tool.name, server_name)]),
                            parameter_examples: None,
                            generated_at: Some(chrono::Utc::now()),
                        });
                        debug!("✅ Marked tool '{}' as elicitation-capable on server '{}'", tool.name, server_name);
                    }
                }
            }
            Err(_) => {
                debug!("Server '{}' doesn't support elicitation for tool '{}'", server_name, tool.name);
            }
        }
        
        Ok(())
    }

    /// Generate capability file for a server with version information
    async fn generate_capability_file(
        server_name: &str, 
        tools: &[Tool], 
        version_info: &ServerVersionInfo, 
        config: &ExternalMcpConfig
    ) -> Result<()> {
        debug!("Generating capability file for External MCP server: {}", server_name);

        // Create output directory if it doesn't exist
        let output_dir = Path::new(&config.capabilities_output_dir);
        if !output_dir.exists() {
            tokio::fs::create_dir_all(output_dir).await
                .map_err(|e| ProxyError::config(format!("Failed to create output directory '{}': {}", output_dir.display(), e)))?;
        }

        // Check for existing capability file to preserve user settings
        let file_path = output_dir.join(format!("{}.yaml", server_name));
        let existing_settings = Self::load_existing_tool_settings(&file_path).await;

        // Convert tools to capability format with enhanced 2025-06-18 metadata
        let tool_definitions: Vec<ToolDefinition> = tools.iter().map(|tool| {
            let tool_full_name = format!("{}_{}", tool.name, server_name);
            
            // Get existing settings for this tool, preserving user preferences
            let (enabled, hidden) = existing_settings.get(&tool_full_name)
                .map(|(e, h)| (*e, *h))
                .unwrap_or((true, true)); // Default: enabled=true, hidden=true for new tools
            
            // Create enhanced annotations including MCP 2025-06-18 capabilities
            let mut annotations = std::collections::HashMap::new();
            annotations.insert("source".to_string(), "external_mcp".to_string());
            annotations.insert("server".to_string(), server_name.to_string());
            annotations.insert("original_name".to_string(), tool.name.clone());
            
            // Add sampling/elicitation capability indicators if present
            if let Some(ref tool_annotations) = tool.annotations {
                if tool_annotations.sampling.is_some() {
                    annotations.insert("has_sampling_capability".to_string(), "true".to_string());
                }
                if tool_annotations.elicitation.is_some() {
                    annotations.insert("has_elicitation_capability".to_string(), "true".to_string());
                }
                
                // Store MCP 2025-06-18 capability metadata as JSON
                if let Ok(capability_json) = serde_json::to_string(tool_annotations) {
                    annotations.insert("mcp_2025_06_18_capabilities".to_string(), capability_json);
                }
            }
            
            ToolDefinition {
                name: tool_full_name,
                description: tool.description.clone().unwrap_or_else(|| format!("{} (via {} MCP server)", tool.name, server_name)),
                input_schema: tool.input_schema.clone(),
                routing: RoutingConfig {
                    r#type: "external_mcp".to_string(),
                    config: json!({
                        "server_name": server_name,
                        "tool_name": tool.name,
                        "endpoint": server_name,
                        "method": "tools/call",
                        "timeout": 30,
                        "retry_count": 2
                    }),
                },
                annotations: Some(annotations),
                hidden, // Preserve user setting or use default
                enabled, // Preserve user setting or use default
                prompt_refs: Vec::new(),
                resource_refs: Vec::new(),
                sampling_strategy: None,
                elicitation_strategy: None,
            }
        }).collect();

        // Create capability file
        let capability_file = CapabilityFile {
            metadata: Some(crate::registry::types::FileMetadata {
                name: Some(format!("{}-external-mcp", server_name)),
                version: Some("1.0.0".to_string()),
                description: Some(format!("External MCP server capabilities for {}", server_name)),
                author: Some("Magic Tunnel External MCP Manager".to_string()),
                tags: Some(vec![
                    "external-mcp".to_string(),
                    server_name.to_string(),
                    "auto-generated".to_string(),
                    "mcp-server".to_string()
                ]),
            }),
            tools: tool_definitions,
            enhanced_metadata: None,
            enhanced_tools: None,
        };

        // Write capability file only if content has changed
        let file_path = output_dir.join(format!("{}.yaml", server_name));
        let yaml_content = serde_yaml::to_string(&capability_file)
            .map_err(|e| ProxyError::config(format!("Failed to serialize capability file: {}", e)))?;

        // Check if file exists and content is structurally the same
        let should_write = if file_path.exists() {
            match tokio::fs::read_to_string(&file_path).await {
                Ok(existing_content) => {
                    // Parse both YAML contents and compare structures, not strings
                    match (serde_yaml::from_str::<CapabilityFile>(&existing_content), 
                           serde_yaml::from_str::<CapabilityFile>(&yaml_content)) {
                        (Ok(existing_parsed), Ok(new_parsed)) => {
                            // Compare the actual data structures, ignoring property order
                            let metadata_matches = match (&existing_parsed.metadata, &new_parsed.metadata) {
                                (Some(existing_meta), Some(new_meta)) => {
                                    existing_meta.name == new_meta.name &&
                                    existing_meta.description == new_meta.description &&
                                    existing_meta.version == new_meta.version &&
                                    existing_meta.author == new_meta.author &&
                                    existing_meta.tags == new_meta.tags
                                }
                                (None, None) => true,
                                _ => false, // One has metadata, other doesn't
                            };
                            
                            if metadata_matches &&
                               existing_parsed.tools.len() == new_parsed.tools.len() &&
                               tools_are_equivalent(&existing_parsed.tools, &new_parsed.tools) {
                                debug!("No structural changes for External MCP server '{}', skipping write", server_name);
                                false
                            } else {
                                debug!("Structural changes detected for External MCP server '{}', updating file", server_name);
                                true
                            }
                        }
                        _ => {
                            // If we can't parse one of the files, assume they're different
                            debug!("Could not parse YAML for comparison for External MCP server '{}', will write", server_name);
                            true
                        }
                    }
                }
                Err(_) => {
                    debug!("Could not read existing file for External MCP server '{}', will write", server_name);
                    true
                }
            }
        } else {
            debug!("File does not exist for External MCP server '{}', will create", server_name);
            true
        };

        if should_write {
            // Save previous version if file exists and versioning is enabled
            Self::save_previous_version(&file_path, server_name).await?;
            
            // Write new version
            tokio::fs::write(&file_path, yaml_content).await
                .map_err(|e| ProxyError::config(format!("Failed to write capability file '{}': {}", file_path.display(), e)))?;
            info!("Generated capability file for External MCP server '{}': {}", server_name, file_path.display());
            
            // Clean up old versions if needed
            Self::cleanup_old_versions(&file_path, server_name).await?;
        } else {
            debug!("Skipped writing unchanged capability file for External MCP server '{}'", server_name);
        }
        Ok(())
    }
    
    
    /// Detect server version and capability information
    async fn detect_server_version_info(
        process: &ExternalMcpProcess,
        server_name: &str,
    ) -> ServerVersionInfo {
        debug!("Detecting version information for External MCP server: {}", server_name);
        
        let mut version_info = ServerVersionInfo::default();
        
        // Try to get server information from ping or status endpoint
        match process.send_request("ping", Some(json!({}))).await {
            Ok(response) => {
                if let Some(result) = response.result {
                    if let Ok(server_info_str) = serde_json::from_value::<String>(result) {
                        version_info.server_info = Some(server_info_str);
                        debug!("Got server info from ping: {}", version_info.server_info.as_ref().unwrap());
                    }
                }
            }
            Err(_) => {
                debug!("Server '{}' doesn't support ping endpoint", server_name);
            }
        }
        
        // Try to detect protocol version by testing for specific endpoints
        // Test for sampling support (MCP advanced capability)
        match process.send_request("sampling/createMessage", Some(json!({
            "messages": [{
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "test"
                }
            }],
            "maxTokens": 1
        }))).await {
            Ok(response) => {
                if response.error.is_none() {
                    version_info.supports_sampling = true;
                    version_info.protocol_version = Some("2025-06-18".to_string());
                    debug!("Server '{}' supports MCP sampling capability", server_name);
                }
            }
            Err(_) => {
                debug!("Server '{}' doesn't support sampling capability", server_name);
            }
        }
        
        // Test for elicitation support (MCP advanced capability)
        match process.send_request("elicitation/create", Some(json!({
            "prompt": "Test elicitation capability",
            "inputType": "text",
            "required": false
        }))).await {
            Ok(response) => {
                if response.error.is_none() {
                    version_info.supports_elicitation = true;
                    if version_info.protocol_version.is_none() {
                        version_info.protocol_version = Some("2025-06-18".to_string());
                    }
                    debug!("Server '{}' supports MCP elicitation capability", server_name);
                }
            }
            Err(_) => {
                debug!("Server '{}' doesn't support elicitation capability", server_name);
            }
        }
        
        // Test for basic MCP capabilities
        match process.send_request("tools/list", Some(json!({}))).await {
            Ok(response) => {
                if response.error.is_none() {
                    version_info.supports_tools = true;
                    if version_info.protocol_version.is_none() {
                        version_info.protocol_version = Some("2024-11-05".to_string()); // Base MCP version
                    }
                }
            }
            Err(_) => {}
        }
        
        match process.send_request("resources/list", Some(json!({}))).await {
            Ok(response) => {
                if response.error.is_none() {
                    version_info.supports_resources = true;
                }
            }
            Err(_) => {}
        }
        
        match process.send_request("prompts/list", Some(json!({}))).await {
            Ok(response) => {
                if response.error.is_none() {
                    version_info.supports_prompts = true;
                }
            }
            Err(_) => {}
        }
        
        match process.send_request("roots/list", Some(json!({}))).await {
            Ok(response) => {
                if response.error.is_none() {
                    version_info.supports_roots = true;
                }
            }
            Err(_) => {}
        }
        
        info!("Detected External MCP server '{}' capabilities: Protocol={}, Sampling={}, Elicitation={}, Tools={}, Resources={}, Prompts={}, Roots={}",
              server_name,
              version_info.protocol_version.as_deref().unwrap_or("unknown"),
              version_info.supports_sampling,
              version_info.supports_elicitation,
              version_info.supports_tools,
              version_info.supports_resources,
              version_info.supports_prompts,
              version_info.supports_roots
        );
        
        version_info
    }
    
    /// Save the previous version of a capability file before overwriting
    async fn save_previous_version(file_path: &PathBuf, server_name: &str) -> Result<()> {
        let versioning_config = VersioningConfig::default();
        
        if !versioning_config.enabled {
            debug!("Versioning disabled for server '{}', skipping version save", server_name);
            return Ok(());
        }
        
        // Check if current file exists
        if !file_path.exists() {
            debug!("No existing file to version for server '{}'", server_name);
            return Ok(());
        }
        
        // Create versions directory
        let versions_dir = file_path.parent().unwrap().join("versions").join(server_name);
        if !versions_dir.exists() {
            fs::create_dir_all(&versions_dir).await
                .map_err(|e| ProxyError::config(format!("Failed to create versions directory '{}': {}", versions_dir.display(), e)))?;
        }
        
        // Get current file metadata
        let metadata = fs::metadata(file_path).await
            .map_err(|e| ProxyError::config(format!("Failed to get file metadata for '{}': {}", file_path.display(), e)))?;
        
        let modified_time = metadata.modified()
            .map_err(|e| ProxyError::config(format!("Failed to get modification time for '{}': {}", file_path.display(), e)))?;
        
        // Create version filename with timestamp
        let timestamp = chrono::DateTime::<chrono::Utc>::from(modified_time)
            .format("%Y%m%d_%H%M%S")
            .to_string();
        
        let version_filename = format!("{}.{}.yaml", server_name, timestamp);
        let version_path = versions_dir.join(&version_filename);
        
        // Copy current file to version
        fs::copy(file_path, &version_path).await
            .map_err(|e| ProxyError::config(format!("Failed to copy file to version '{}': {}", version_path.display(), e)))?;
        
        info!("Saved previous version of capability file for server '{}': {}", server_name, version_path.display());
        
        // Add version metadata to the versioned file
        Self::add_version_metadata(&version_path, server_name, &timestamp).await?;
        
        Ok(())
    }
    
    /// Add version metadata to a versioned capability file
    async fn add_version_metadata(version_path: &PathBuf, server_name: &str, timestamp: &str) -> Result<()> {
        // Read the existing file
        let content = fs::read_to_string(version_path).await
            .map_err(|e| ProxyError::config(format!("Failed to read version file '{}': {}", version_path.display(), e)))?;
        
        // Parse as YAML
        let mut capability_file: CapabilityFile = serde_yaml::from_str(&content)
            .map_err(|e| ProxyError::config(format!("Failed to parse version file '{}': {}", version_path.display(), e)))?;
        
        // Add version metadata to file metadata
        if let Some(ref mut file_metadata) = capability_file.metadata {
            // Add versioning tags
            if let Some(ref mut tags) = file_metadata.tags {
                tags.push(format!("archived-{}", timestamp));
                tags.push("version-archive".to_string());
            } else {
                file_metadata.tags = Some(vec![
                    format!("archived-{}", timestamp),
                    "version-archive".to_string(),
                ]);
            }
        }
        
        // Update the file metadata version info
        if let Some(ref mut file_metadata) = capability_file.metadata {
            let current_version = file_metadata.version.clone().unwrap_or_else(|| "1.0.0".to_string());
            file_metadata.description = Some(format!(
                "[ARCHIVED {}] {}", 
                timestamp,
                file_metadata.description.as_deref().unwrap_or("External MCP server capabilities")
            ));
            // Keep the original version but add archive marker
            file_metadata.version = Some(format!("{}-archived-{}", current_version, timestamp));
        }
        
        // Write back the enhanced version file
        let enhanced_content = serde_yaml::to_string(&capability_file)
            .map_err(|e| ProxyError::config(format!("Failed to serialize enhanced version file: {}", e)))?;
        
        fs::write(version_path, enhanced_content).await
            .map_err(|e| ProxyError::config(format!("Failed to write enhanced version file '{}': {}", version_path.display(), e)))?;
        
        debug!("Added version metadata to archived file: {}", version_path.display());
        Ok(())
    }
    
    /// Clean up old versions based on versioning configuration
    async fn cleanup_old_versions(file_path: &PathBuf, server_name: &str) -> Result<()> {
        let versioning_config = VersioningConfig::default();
        
        if !versioning_config.enabled {
            return Ok(());
        }
        
        let versions_dir = file_path.parent().unwrap().join("versions").join(server_name);
        if !versions_dir.exists() {
            return Ok(());
        }
        
        // Get all version files for this server
        let mut version_files = Vec::new();
        let mut entries = fs::read_dir(&versions_dir).await
            .map_err(|e| ProxyError::config(format!("Failed to read versions directory '{}': {}", versions_dir.display(), e)))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ProxyError::config(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml") {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with(&format!("{}." ,server_name)) {
                        if let Ok(metadata) = fs::metadata(&path).await {
                            if let Ok(modified) = metadata.modified() {
                                version_files.push((path, modified));
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by modification time (newest first)
        version_files.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Remove excess versions
        if version_files.len() > versioning_config.max_versions {
            let files_to_remove = &version_files[versioning_config.max_versions..];
            
            for (file_path, _) in files_to_remove {
                if let Err(e) = fs::remove_file(file_path).await {
                    warn!("Failed to remove old version file '{}': {}", file_path.display(), e);
                } else {
                    debug!("Removed old version file: {}", file_path.display());
                }
            }
            
            info!("Cleaned up {} old version files for server '{}'", files_to_remove.len(), server_name);
        }
        
        Ok(())
    }
    
    /// Get available versions for a server
    pub async fn get_server_versions(server_name: &str, capabilities_output_dir: &str) -> Result<Vec<String>> {
        let versions_dir = Path::new(capabilities_output_dir).join("versions").join(server_name);
        
        if !versions_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut versions = Vec::new();
        let mut entries = fs::read_dir(&versions_dir).await
            .map_err(|e| ProxyError::config(format!("Failed to read versions directory '{}': {}", versions_dir.display(), e)))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ProxyError::config(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml") {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with(&format!("{}." ,server_name)) {
                        versions.push(file_name.to_string());
                    }
                }
            }
        }
        
        // Sort versions by timestamp (newest first)
        versions.sort_by(|a, b| {
            // Extract timestamp from filename like "server.20241101_143022.yaml"
            let extract_timestamp = |filename: &str| -> String {
                let prefix = format!("{}.", server_name);
                filename.strip_prefix(&prefix)
                    .and_then(|s| s.strip_suffix(".yaml"))
                    .and_then(|s| s.split('.').next())
                    .unwrap_or("")
                    .to_string()
            };
            
            let time_a = extract_timestamp(a);
            let time_b = extract_timestamp(b);
            time_b.cmp(&time_a)
        });
        
        Ok(versions)
    }
    
    /// Restore a specific version of a capability file
    pub async fn restore_version(server_name: &str, version_timestamp: &str, capabilities_output_dir: &str) -> Result<()> {
        let versions_dir = Path::new(capabilities_output_dir).join("versions").join(server_name);
        let version_filename = format!("{}.{}.yaml", server_name, version_timestamp);
        let version_path = versions_dir.join(&version_filename);
        
        if !version_path.exists() {
            return Err(ProxyError::config(format!("Version file not found: {}", version_path.display())));
        }
        
        let current_file = Path::new(capabilities_output_dir).join(format!("{}.yaml", server_name));
        
        // First, save the current version before restoring
        if current_file.exists() {
            Self::save_previous_version(&current_file.to_path_buf(), server_name).await?;
        }
        
        // Copy the version file to current
        fs::copy(&version_path, &current_file).await
            .map_err(|e| ProxyError::config(format!("Failed to restore version '{}': {}", version_path.display(), e)))?;
        
        info!("Restored version {} for server '{}'", version_timestamp, server_name);
        Ok(())
    }
}

/// Helper function to compare tool arrays for structural equivalence
fn tools_are_equivalent(existing: &[ToolDefinition], new: &[ToolDefinition]) -> bool {
    if existing.len() != new.len() {
        return false;
    }
    
    // Sort both arrays by tool name for consistent comparison
    let mut existing_sorted = existing.to_vec();
    let mut new_sorted = new.to_vec();
    existing_sorted.sort_by(|a, b| a.name.cmp(&b.name));
    new_sorted.sort_by(|a, b| a.name.cmp(&b.name));
    
    // Compare each tool
    for (existing_tool, new_tool) in existing_sorted.iter().zip(new_sorted.iter()) {
        if existing_tool.name != new_tool.name ||
           existing_tool.description != new_tool.description ||
           existing_tool.input_schema != new_tool.input_schema ||
           existing_tool.enabled != new_tool.enabled ||
           existing_tool.hidden != new_tool.hidden {
            return false;
        }
        
        // Compare routing configs
        if existing_tool.routing != new_tool.routing {
            return false;
        }
    }
    
    true
}

impl ExternalMcpManager {
    /// Execute a tool on a specific External MCP server
    pub async fn execute_tool(&self, server_name: &str, tool_name: &str, arguments: Value) -> Result<Value> {
        debug!("🔧 [EXECUTE] Executing tool '{}' on External MCP server '{}'", tool_name, server_name);
        let start_time = Instant::now();

        // Check if process exists and is running, then execute tool
        let processes = self.processes.read().await;

        let process = match processes.get(server_name) {
            Some(p) => p,
            None => {
                let error = format!("External MCP server '{}' not found", server_name);
                self.metrics_collector.record_request_error(server_name, "server_not_found", "tools/call").await;
                return Err(ProxyError::mcp(error));
            }
        };

        if !process.is_running().await {
            let error = format!("External MCP server '{}' is not running", server_name);
            self.metrics_collector.record_request_error(server_name, "server_not_running", "tools/call").await;
            return Err(ProxyError::connection(error));
        }

        let params = json!({
            "name": tool_name,
            "arguments": arguments
        });

        match process.send_request("tools/call", Some(params)).await {
            Ok(response) => {
                let elapsed_ms = start_time.elapsed().as_millis() as f64;
                
                if let Some(error) = response.error {
                    // Record error in metrics
                    self.metrics_collector.record_request_error(server_name, "tool_execution_error", "tools/call").await;
                    
                    error!("❌ [EXECUTE] Tool '{}' execution failed on server '{}': {} ({}ms)", 
                           tool_name, server_name, error.message, elapsed_ms);
                    
                    return Err(ProxyError::tool_execution(
                        tool_name.to_string(), 
                        format!("Tool execution failed: {}", error.message)
                    ));
                }

                match response.result {
                    Some(result) => {
                        // Record successful execution in metrics
                        self.metrics_collector.record_request_success(server_name, elapsed_ms, "tools/call").await;
                        
                        info!("✅ [EXECUTE] Tool '{}' executed successfully on server '{}' ({}ms)", 
                              tool_name, server_name, elapsed_ms);
                        
                        Ok(result)
                    }
                    None => {
                        // Record error - no result returned
                        self.metrics_collector.record_request_error(server_name, "no_result_returned", "tools/call").await;
                        
                        error!("❌ [EXECUTE] Tool '{}' returned no result on server '{}' ({}ms)", 
                               tool_name, server_name, elapsed_ms);
                        
                        Err(ProxyError::tool_execution(
                            tool_name.to_string(), 
                            "No result returned from tool execution".to_string()
                        ))
                    }
                }
            }
            Err(e) => {
                let elapsed_ms = start_time.elapsed().as_millis() as f64;
                
                // Record request failure in metrics
                self.metrics_collector.record_request_error(server_name, "request_failed", "tools/call").await;
                
                error!("❌ [EXECUTE] Tool '{}' request failed on server '{}': {} ({}ms)", 
                       tool_name, server_name, e, elapsed_ms);
                
                Err(e)
            }
        }
    }

    /// Get all available tools from all servers
    pub async fn get_all_tools(&self) -> HashMap<String, Vec<Tool>> {
        let capabilities = self.capabilities.read().await;
        capabilities.clone()
    }

    /// Get tools from a specific server
    pub async fn get_server_tools(&self, server_name: &str) -> Option<Vec<Tool>> {
        let capabilities = self.capabilities.read().await;
        capabilities.get(server_name).cloned()
    }

    /// Get list of active server names
    pub async fn get_active_servers(&self) -> Vec<String> {
        let processes = self.processes.read().await;
        let mut active_servers = Vec::new();

        // We need to check each process individually since we can't iterate with async
        let server_names: Vec<String> = processes.keys().cloned().collect();
        drop(processes); // Release the read lock

        for name in server_names {
            let processes = self.processes.read().await;
            if let Some(process) = processes.get(&name) {
                if process.is_running().await {
                    active_servers.push(name);
                }
            }
        }

        active_servers
    }

    /// Get health status of all external MCP services
    pub async fn get_health_status(&self) -> HashMap<String, crate::mcp::metrics::HealthStatus> {
        let processes = self.processes.read().await;
        let mut health_status = HashMap::new();

        for (server_name, process) in processes.iter() {
            let status = if process.is_running().await {
                // Check if the server has tools loaded (indicates healthy)
                let tools = self.get_server_tools(server_name).await.unwrap_or_default();
                if tools.is_empty() {
                    crate::mcp::metrics::HealthStatus::Degraded
                } else {
                    crate::mcp::metrics::HealthStatus::Healthy
                }
            } else {
                crate::mcp::metrics::HealthStatus::Down
            };
            
            health_status.insert(server_name.clone(), status);
        }

        health_status
    }

    /// Get process information for a specific server
    pub async fn get_server_process_info(&self, server_name: &str) -> Option<(Option<u32>, String)> {
        let processes = self.processes.read().await;
        if let Some(process) = processes.get(server_name) {
            let pid = process.get_pid();
            let uptime = process.get_uptime_formatted();
            Some((pid, uptime))
        } else {
            None
        }
    }

    /// Stop all External MCP servers
    pub async fn stop_all(&self) -> Result<()> {
        info!("Stopping all External MCP servers");

        let mut processes = self.processes.write().await;

        for (name, process) in processes.iter_mut() {
            info!("Stopping External MCP server: {}", name);
            if let Err(e) = process.stop().await {
                error!("Failed to stop External MCP server '{}': {}", name, e);
            }
        }

        processes.clear();

        // Clear capabilities
        {
            let mut capabilities = self.capabilities.write().await;
            capabilities.clear();
        }

        info!("All External MCP servers stopped");
        Ok(())
    }

    /// Stop a specific server
    pub async fn stop_server(&self, server_name: &str) -> Result<()> {
        info!("Stopping External MCP server: {}", server_name);

        // Stop the server and remove it from the processes map
        {
            let mut processes = self.processes.write().await;
            if let Some(mut process) = processes.remove(server_name) {
                process.stop().await?;
                info!("Successfully stopped External MCP server: {}", server_name);
            } else {
                warn!("External MCP server '{}' not found or already stopped", server_name);
                return Err(ProxyError::config(format!("Server '{}' not found", server_name)));
            }
        }

        // Clear capabilities for this server
        {
            let mut capabilities = self.capabilities.write().await;
            capabilities.remove(server_name);
        }

        info!("External MCP server '{}' stopped and removed from active servers", server_name);
        Ok(())
    }

    /// Restart a specific server
    pub async fn restart_server(&self, server_name: &str) -> Result<()> {
        info!("Restarting External MCP server: {}", server_name);

        // Stop the server
        {
            let mut processes = self.processes.write().await;
            if let Some(process) = processes.get_mut(server_name) {
                process.stop().await?;
            }
            processes.remove(server_name);
        }

        // Reload configuration and restart
        let servers_config = self.load_servers_config().await?;
        if let Some(server_config) = servers_config.mcp_servers.as_ref().and_then(|servers| servers.get(server_name)) {
            self.start_server(server_name.to_string(), server_config.clone()).await?;
            self.discover_server_capabilities(server_name).await?;
        } else {
            return Err(ProxyError::config(format!("Server '{}' not found in configuration", server_name)));
        }

        info!("Successfully restarted External MCP server: {}", server_name);
        Ok(())
    }
    
    /// Load existing tool settings from capability file to preserve user preferences
    async fn load_existing_tool_settings(file_path: &PathBuf) -> HashMap<String, (bool, bool)> {
        let mut settings = HashMap::new();
        
        // Check if file exists
        if !file_path.exists() {
            debug!("No existing capability file found at: {}", file_path.display());
            return settings;
        }
        
        // Try to read and parse existing file
        match tokio::fs::read_to_string(file_path).await {
            Ok(content) => {
                match serde_yaml::from_str::<CapabilityFile>(&content) {
                    Ok(existing_file) => {
                        for tool in existing_file.tools {
                            settings.insert(tool.name.clone(), (tool.enabled, tool.hidden));
                        }
                        info!("Loaded {} existing tool settings from: {}", settings.len(), file_path.display());
                    }
                    Err(e) => {
                        warn!("Failed to parse existing capability file '{}': {}", file_path.display(), e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read existing capability file '{}': {}", file_path.display(), e);
            }
        }
        
        settings
    }

    /// Get metrics collector for external access
    pub fn metrics_collector(&self) -> Arc<McpMetricsCollector> {
        Arc::clone(&self.metrics_collector)
    }

    /// Get health checker for external access
    pub fn health_checker(&self) -> Arc<McpHealthChecker> {
        Arc::clone(&self.health_checker)
    }

    /// Send a request to a specific external MCP server
    pub async fn send_request_to_server(&self, server_name: &str, method: &str, params: Option<serde_json::Value>) -> Result<McpResponse> {
        let processes = self.processes.read().await;
        
        if let Some(process) = processes.get(server_name) {
            process.send_request(method, params).await
        } else {
            Err(ProxyError::connection(format!("External MCP server '{}' not found", server_name)))
        }
    }

    /// Get list of external MCP servers that support sampling capability (MCP 2025-06-18)
    pub async fn get_sampling_capable_servers(&self) -> Vec<String> {
        let version_info = self.version_info.read().await;
        
        version_info.iter()
            .filter_map(|(server_name, info)| {
                if info.supports_sampling {
                    Some(server_name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get list of external MCP servers that support elicitation capability (MCP 2025-06-18)
    pub async fn get_elicitation_capable_servers(&self) -> Vec<String> {
        let version_info = self.version_info.read().await;
        
        version_info.iter()
            .filter_map(|(server_name, info)| {
                if info.supports_elicitation {
                    Some(server_name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Forward sampling request to external MCP server (MCP 2025-06-18)
    pub async fn forward_sampling_request(
        &self,
        server_name: &str,
        request: &crate::mcp::types::sampling::SamplingRequest,
    ) -> std::result::Result<crate::mcp::types::sampling::SamplingResponse, crate::mcp::types::sampling::SamplingError> {
        debug!("Forwarding sampling request to external MCP server: {}", server_name);
        
        let processes = self.processes.read().await;
        
        if let Some(process) = processes.get(server_name) {
            // Convert SamplingRequest to JSON for MCP transmission
            let params = serde_json::to_value(request).map_err(|e| {
                crate::mcp::types::sampling::SamplingError {
                    code: crate::mcp::types::sampling::SamplingErrorCode::InvalidRequest,
                    message: format!("Failed to serialize sampling request: {}", e),
                    details: None,
                }
            })?;
            
            // Send the sampling/createMessage request to external server
            match process.send_request("sampling/createMessage", Some(params)).await {
                Ok(response) => {
                    // Convert MCP response back to SamplingResponse
                    if let Some(result) = response.result {
                        serde_json::from_value(result).map_err(|e| {
                            crate::mcp::types::sampling::SamplingError {
                                code: crate::mcp::types::sampling::SamplingErrorCode::InternalError,
                                message: format!("Failed to deserialize sampling response: {}", e),
                                details: None,
                            }
                        })
                    } else if let Some(error) = response.error {
                        Err(crate::mcp::types::sampling::SamplingError {
                            code: crate::mcp::types::sampling::SamplingErrorCode::InternalError,
                            message: format!("External server error: {}", error.message),
                            details: error.data.map(|d| {
                                d.as_object()
                                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                                    .unwrap_or_default()
                            }),
                        })
                    } else {
                        Err(crate::mcp::types::sampling::SamplingError {
                            code: crate::mcp::types::sampling::SamplingErrorCode::InternalError,
                            message: "Invalid response from external server".to_string(),
                            details: None,
                        })
                    }
                }
                Err(e) => {
                    error!("Failed to forward sampling request to server '{}': {}", server_name, e);
                    Err(crate::mcp::types::sampling::SamplingError {
                        code: crate::mcp::types::sampling::SamplingErrorCode::InternalError,
                        message: format!("Failed to communicate with external server: {}", e),
                        details: None,
                    })
                }
            }
        } else {
            Err(crate::mcp::types::sampling::SamplingError {
                code: crate::mcp::types::sampling::SamplingErrorCode::InternalError,
                message: format!("External MCP server '{}' not found", server_name),
                details: None,
            })
        }
    }

    /// Forward elicitation request to external MCP server (MCP 2025-06-18)
    pub async fn forward_elicitation_request(
        &self,
        server_name: &str,
        request: &crate::mcp::types::elicitation::ElicitationRequest,
    ) -> std::result::Result<crate::mcp::types::elicitation::ElicitationResponse, crate::mcp::types::elicitation::ElicitationError> {
        debug!("Forwarding elicitation request to external MCP server: {}", server_name);
        
        let processes = self.processes.read().await;
        
        if let Some(process) = processes.get(server_name) {
            // Convert ElicitationRequest to JSON for MCP transmission
            let params = serde_json::to_value(request).map_err(|e| {
                crate::mcp::types::elicitation::ElicitationError {
                    code: crate::mcp::types::elicitation::ElicitationErrorCode::InvalidRequest,
                    message: format!("Failed to serialize elicitation request: {}", e),
                    details: None,
                }
            })?;
            
            // Send the elicitation/create request to external server
            match process.send_request("elicitation/create", Some(params)).await {
                Ok(response) => {
                    if let Some(error) = response.error {
                        error!("External server '{}' returned elicitation error: {}", server_name, error.message);
                        return Err(crate::mcp::types::elicitation::ElicitationError {
                            code: crate::mcp::types::elicitation::ElicitationErrorCode::InternalError,
                            message: error.message,
                            details: error.data.map(|v| [("error_data".to_string(), v)].into_iter().collect()),
                        });
                    }
                    
                    if let Some(result) = response.result {
                        // Parse the elicitation response
                        let elicitation_response: crate::mcp::types::elicitation::ElicitationResponse = 
                            serde_json::from_value(result).map_err(|e| {
                                error!("Failed to parse elicitation response from server '{}': {}", server_name, e);
                                crate::mcp::types::elicitation::ElicitationError {
                                    code: crate::mcp::types::elicitation::ElicitationErrorCode::InvalidRequest,
                                    message: format!("Failed to parse elicitation response: {}", e),
                                    details: None,
                                }
                            })?;
                        
                        debug!("Successfully received elicitation response from external server '{}'", server_name);
                        Ok(elicitation_response)
                    } else {
                        error!("External server '{}' returned no result for elicitation request", server_name);
                        Err(crate::mcp::types::elicitation::ElicitationError {
                            code: crate::mcp::types::elicitation::ElicitationErrorCode::InvalidRequest,
                            message: "No result in elicitation response".to_string(),
                            details: None,
                        })
                    }
                }
                Err(e) => {
                    error!("Failed to forward elicitation request to server '{}': {}", server_name, e);
                    Err(crate::mcp::types::elicitation::ElicitationError {
                        code: crate::mcp::types::elicitation::ElicitationErrorCode::InternalError,
                        message: format!("Request failed: {}", e),
                        details: None,
                    })
                }
            }
        } else {
            error!("External MCP server '{}' not found for elicitation request", server_name);
            Err(crate::mcp::types::elicitation::ElicitationError {
                code: crate::mcp::types::elicitation::ElicitationErrorCode::InternalError,
                message: format!("External MCP server '{}' not found", server_name),
                details: None,
            })
        }
    }
}
