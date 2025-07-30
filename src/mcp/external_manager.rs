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
    /// Metrics collector for observability
    metrics_collector: Arc<McpMetricsCollector>,
    /// Health checker for active monitoring
    health_checker: Arc<McpHealthChecker>,
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
            metrics_collector,
            health_checker,
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
        debug!("Initializing External MCP server: {} with protocol version: {}, client: {}@{}",
               process.name,
               self.client_config.protocol_version,
               self.client_config.client_name,
               self.client_config.client_version);

        let params = json!({
            "protocolVersion": self.client_config.protocol_version,
            "capabilities": {
                "roots": {
                    "listChanged": true
                },
                "sampling": {}
            },
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

    /// Discover capabilities from a specific server
    async fn discover_server_capabilities(&self, server_name: &str) -> Result<()> {
        Self::discover_server_capabilities_static(
            &self.processes,
            &self.capabilities,
            server_name,
            &self.config,
        ).await
    }

    /// Static method for capability discovery (used by periodic task)
    async fn discover_server_capabilities_static(
        processes: &Arc<RwLock<HashMap<String, ExternalMcpProcess>>>,
        capabilities: &Arc<RwLock<HashMap<String, Vec<Tool>>>>,
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

                // Discover tools
                info!("Sending tools/list request to server '{}' with params: {}", server_name, json!({}));
                match process.send_request("tools/list", Some(json!({}))).await {
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
                }
            } else {
                warn!("External MCP server '{}' not found", server_name);
                return Ok(());
            }
        };

        // Store discovered capabilities
        {
            let mut capabilities_guard = capabilities.write().await;
            capabilities_guard.insert(server_name.to_string(), tools.clone());
        }

        // Generate capability file
        Self::generate_capability_file(server_name, &tools, config).await?;

        info!("Successfully discovered and generated capabilities for External MCP server: {}", server_name);
        Ok(())
    }

    /// Generate capability file for a server
    async fn generate_capability_file(server_name: &str, tools: &[Tool], config: &ExternalMcpConfig) -> Result<()> {
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

        // Convert tools to capability format
        let tool_definitions: Vec<ToolDefinition> = tools.iter().map(|tool| {
            let tool_full_name = format!("{}_{}", tool.name, server_name);
            
            // Get existing settings for this tool, preserving user preferences
            let (enabled, hidden) = existing_settings.get(&tool_full_name)
                .map(|(e, h)| (*e, *h))
                .unwrap_or((true, true)); // Default: enabled=true, hidden=true for new tools
            
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
                annotations: Some({
                    let mut annotations = std::collections::HashMap::new();
                    annotations.insert("source".to_string(), "external_mcp".to_string());
                    annotations.insert("server".to_string(), server_name.to_string());
                    annotations.insert("original_name".to_string(), tool.name.clone());
                    annotations
                }),
                hidden, // Preserve user setting or use default
                enabled, // Preserve user setting or use default
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
            tokio::fs::write(&file_path, yaml_content).await
                .map_err(|e| ProxyError::config(format!("Failed to write capability file '{}': {}", file_path.display(), e)))?;
            info!("Generated capability file for External MCP server '{}': {}", server_name, file_path.display());
        } else {
            debug!("Skipped writing unchanged capability file for External MCP server '{}'", server_name);
        }
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
}
