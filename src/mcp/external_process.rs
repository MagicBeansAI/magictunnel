//! External MCP Process Management
//! 
//! This module manages MCP server processes using Claude Desktop's exact configuration format.
//! It can spawn local processes (npx, uv run, python, node) and containerized processes (docker run).

use crate::config::{McpServerConfig, ExternalMcpServersConfig, ContainerConfig, McpClientConfig};
use crate::error::{ProxyError, Result};
use crate::mcp::types::{McpRequest, McpResponse, Tool};
use crate::mcp::request_forwarder::{RequestForwarder, SharedRequestForwarder, ExternalMcpClient};
use crate::mcp::errors::McpError;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{timeout, Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use async_trait::async_trait;

/// Represents a single MCP server process spawned from configuration
pub struct ExternalMcpProcess {
    /// Server name from configuration
    pub name: String,
    /// Server configuration from Claude Desktop format
    pub config: McpServerConfig,
    /// MCP client configuration for timeouts and connection settings
    client_config: McpClientConfig,
    /// The spawned child process
    process: Option<Child>,
    /// Channel for sending JSON-RPC messages to the process
    stdin_sender: Option<mpsc::UnboundedSender<String>>,
    /// Pending requests waiting for responses
    pending_requests: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<McpResponse>>>>,
    /// Process restart tracking
    restart_count: u32,
    last_restart: Option<Instant>,
    /// Maximum restart attempts
    max_restart_attempts: u32,
    /// Process health status
    is_healthy: Arc<RwLock<bool>>,
    /// Process start time for uptime calculation
    start_time: Option<Instant>,
    /// Request forwarder for bidirectional communication
    request_forwarder: Option<SharedRequestForwarder>,
    /// Original client ID for request context
    original_client_id: Option<String>,
}

impl ExternalMcpProcess {
    /// Create a new External MCP process manager
    pub fn new(name: String, config: McpServerConfig, client_config: McpClientConfig) -> Self {
        let max_restart_attempts = client_config.max_reconnect_attempts;
        Self {
            name,
            config,
            client_config,
            process: None,
            stdin_sender: None,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            restart_count: 0,
            last_restart: None,
            max_restart_attempts,
            is_healthy: Arc::new(RwLock::new(false)),
            start_time: None,
            request_forwarder: None,
            original_client_id: None,
        }
    }

    /// Start the MCP server process
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting External MCP server: {}", self.name);

        // Build command from configuration
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args);

        // Set environment variables
        if let Some(ref env) = self.config.env {
            for (key, value) in env {
                // Support environment variable expansion
                let expanded_value = expand_env_vars(value);
                cmd.env(key, expanded_value);
            }
        }

        // Set working directory
        if let Some(ref cwd) = self.config.cwd {
            cmd.current_dir(cwd);
        }

        // Configure stdio for MCP communication
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // Spawn the process
        let mut child = cmd.spawn()
            .map_err(|e| ProxyError::connection(format!("Failed to spawn MCP server '{}': {}", self.name, e)))?;

        // Set up communication channels
        let stdin = child.stdin.take()
            .ok_or_else(|| ProxyError::connection(format!("Failed to get stdin for MCP server '{}'", self.name)))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| ProxyError::connection(format!("Failed to get stdout for MCP server '{}'", self.name)))?;

        // Create stdin sender channel
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<String>();
        self.stdin_sender = Some(stdin_tx);

        // Spawn stdin writer task
        let server_name = self.name.clone();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(message) = stdin_rx.recv().await {
                if let Err(e) = stdin.write_all(message.as_bytes()).await {
                    error!("Failed to write to MCP server '{}' stdin: {}", server_name, e);
                    break;
                }
                if let Err(e) = stdin.write_all(b"\n").await {
                    error!("Failed to write newline to MCP server '{}' stdin: {}", server_name, e);
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    error!("Failed to flush MCP server '{}' stdin: {}", server_name, e);
                    break;
                }
            }
        });

        // Spawn stdout reader task with bidirectional support
        let pending_requests = Arc::clone(&self.pending_requests);
        let server_name = self.name.clone();
        let is_healthy = Arc::clone(&self.is_healthy);
        let request_forwarder = self.request_forwarder.clone();
        let original_client_id = self.original_client_id.clone();
        let stdin_sender = self.stdin_sender.clone();
        
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                debug!("MCP server '{}' stdout: {}", server_name, line);
                
                // Try parsing as McpResponse first (existing functionality)
                if let Ok(response) = serde_json::from_str::<McpResponse>(&line) {
                    // Handle response ID
                    let id_str = response.id.clone();
                    let mut pending = pending_requests.lock().await;
                    if let Some(sender) = pending.remove(&id_str) {
                        if let Err(_) = sender.send(response) {
                            warn!("Failed to send response for request {} to MCP server '{}'", id_str, server_name);
                        }
                    }
                    
                    // Update health status on successful communication
                    *is_healthy.write().await = true;
                    continue;
                }
                
                // 🚀 NEW: Try parsing as McpRequest for bidirectional communication
                if let Ok(request) = serde_json::from_str::<McpRequest>(&line) {
                    debug!("Received bidirectional request from MCP server '{}': method={}", server_name, request.method);
                    
                    match request.method.as_str() {
                        "sampling/createMessage" => {
                            Self::handle_sampling_request_from_external(
                                &request_forwarder,
                                &stdin_sender,
                                request,
                                &server_name,
                                &original_client_id,
                            ).await;
                        }
                        "elicitation/request" => {
                            Self::handle_elicitation_request_from_external(
                                &request_forwarder,
                                &stdin_sender,
                                request,
                                &server_name,
                                &original_client_id,
                            ).await;
                        }
                        _ => {
                            warn!("Unsupported bidirectional request method from MCP server '{}': {}", server_name, request.method);
                            Self::send_unsupported_method_error(&stdin_sender, &request, &server_name).await;
                        }
                    }
                    
                    // Update health status on successful communication
                    *is_healthy.write().await = true;
                    continue;
                }
                
                // Neither response nor request - log as warning
                warn!("Failed to parse JSON-RPC message from MCP server '{}': {}", server_name, line);
            }
            
            warn!("MCP server '{}' stdout reader ended", server_name);
            *is_healthy.write().await = false;
        });

        self.process = Some(child);
        self.start_time = Some(Instant::now());
        info!("Successfully started External MCP server: {}", self.name);
        
        Ok(())
    }

    /// Stop the MCP server process
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping External MCP server: {}", self.name);

        if let Some(mut process) = self.process.take() {
            // Try graceful shutdown first
            if let Err(e) = process.kill().await {
                warn!("Failed to kill MCP server '{}': {}", self.name, e);
            }
            
            // Wait for process to exit
            match timeout(Duration::from_secs(5), process.wait()).await {
                Ok(Ok(status)) => {
                    info!("MCP server '{}' exited with status: {}", self.name, status);
                }
                Ok(Err(e)) => {
                    error!("Error waiting for MCP server '{}' to exit: {}", self.name, e);
                }
                Err(_) => {
                    warn!("MCP server '{}' did not exit within timeout", self.name);
                }
            }
        }

        self.stdin_sender = None;
        self.start_time = None;
        *self.is_healthy.write().await = false;
        
        Ok(())
    }

    /// Check if the process is running and healthy
    pub async fn is_running(&self) -> bool {
        if self.process.is_some() {
            // For now, just check if the process exists and is marked as healthy
            // TODO: Implement proper process status checking without unsafe casting
            *self.is_healthy.read().await
        } else {
            false
        }
    }

    /// Send a JSON-RPC request to the MCP server
    pub async fn send_request(&self, method: &str, params: Option<Value>) -> Result<McpResponse> {
        let request_id = Uuid::new_v4().to_string();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::String(request_id.clone())),
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| ProxyError::mcp(format!("Failed to serialize request: {}", e)))?;

        info!("Sending MCP request to '{}': {}", self.name, request_json);

        // Create response channel
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        // Store pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), response_tx);
        }

        // Send request
        if let Some(ref sender) = self.stdin_sender {
            sender.send(request_json)
                .map_err(|_| ProxyError::connection(format!("Failed to send request to MCP server '{}'", self.name)))?;
        } else {
            return Err(ProxyError::connection(format!("MCP server '{}' is not running", self.name)));
        }

        // Wait for response with timeout
        match timeout(Duration::from_secs(self.client_config.request_timeout_secs), response_rx).await {
            Ok(Ok(response)) => {
                info!("Received MCP response from '{}': {:?}", self.name, response);
                Ok(response)
            },
            Ok(Err(_)) => Err(ProxyError::connection(format!("Response channel closed for MCP server '{}'", self.name))),
            Err(_) => {
                // Remove pending request on timeout
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&request_id);
                Err(ProxyError::timeout(format!("Request to MCP server '{}' timed out", self.name)))
            }
        }
    }

    /// Get the process ID if the process is running
    pub fn get_pid(&self) -> Option<u32> {
        self.process.as_ref().and_then(|p| p.id())
    }

    /// Get the uptime in seconds if the process is running
    pub fn get_uptime_seconds(&self) -> Option<u64> {
        self.start_time.map(|start| start.elapsed().as_secs())
    }

    /// Get formatted uptime string (e.g., "2h 30m 45s")
    pub fn get_uptime_formatted(&self) -> String {
        match self.get_uptime_seconds() {
            Some(total_seconds) => {
                let hours = total_seconds / 3600;
                let minutes = (total_seconds % 3600) / 60;
                let seconds = total_seconds % 60;
                
                if hours > 0 {
                    format!("{}h {}m {}s", hours, minutes, seconds)
                } else if minutes > 0 {
                    format!("{}m {}s", minutes, seconds)
                } else {
                    format!("{}s", seconds)
                }
            }
            None => "Not running".to_string()
        }
    }

    /// Get process start time
    pub fn get_start_time(&self) -> Option<Instant> {
        self.start_time
    }

    /// Set request forwarder for bidirectional communication
    pub fn set_request_forwarder(&mut self, forwarder: SharedRequestForwarder, original_client_id: String) {
        self.request_forwarder = Some(forwarder);
        info!("Set request forwarder for MCP server '{}' with client ID '{}'.", self.name, original_client_id);
        self.original_client_id = Some(original_client_id);
    }

    /// Handle sampling request from external MCP server
    async fn handle_sampling_request_from_external(
        request_forwarder: &Option<SharedRequestForwarder>,
        stdin_sender: &Option<mpsc::UnboundedSender<String>>,
        request: McpRequest,
        server_name: &str,
        original_client_id: &Option<String>,
    ) {
        let Some(forwarder) = request_forwarder else {
            warn!("No request forwarder set for MCP server '{}', cannot handle sampling request", server_name);
            Self::send_no_forwarder_error(stdin_sender, &request, server_name).await;
            return;
        };
        
        let Some(client_id) = original_client_id else {
            warn!("No original client ID set for MCP server '{}', cannot handle sampling request", server_name);
            Self::send_no_client_error(stdin_sender, &request, server_name).await;
            return;
        };
        
        // Convert McpRequest to SamplingRequest
        let sampling_request = match Self::convert_mcp_to_sampling_request(&request) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to convert MCP request to sampling request from server '{}': {}", server_name, e);
                Self::send_conversion_error(stdin_sender, &request, server_name, &e.to_string()).await;
                return;
            }
        };
        
        // Forward to MagicTunnel Server
        match forwarder.forward_sampling_request(sampling_request, server_name, client_id).await {
            Ok(response) => {
                // Convert response back to MCP format and send to external server
                let mcp_response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    result: Some(serde_json::to_value(response).unwrap_or_else(|_| json!(null))),
                    error: None,
                };
                Self::send_response_to_external(stdin_sender, mcp_response, server_name).await;
            }
            Err(e) => {
                error!("Failed to forward sampling request from server '{}': {}", server_name, e);
                let error_response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    result: None,
                    error: Some(McpError::internal_error(e.to_string())),
                };
                Self::send_response_to_external(stdin_sender, error_response, server_name).await;
            }
        }
    }

    /// Handle elicitation request from external MCP server
    async fn handle_elicitation_request_from_external(
        request_forwarder: &Option<SharedRequestForwarder>,
        stdin_sender: &Option<mpsc::UnboundedSender<String>>,
        request: McpRequest,
        server_name: &str,
        original_client_id: &Option<String>,
    ) {
        let Some(forwarder) = request_forwarder else {
            warn!("No request forwarder set for MCP server '{}', cannot handle elicitation request", server_name);
            Self::send_no_forwarder_error(stdin_sender, &request, server_name).await;
            return;
        };
        
        let Some(client_id) = original_client_id else {
            warn!("No original client ID set for MCP server '{}', cannot handle elicitation request", server_name);
            Self::send_no_client_error(stdin_sender, &request, server_name).await;
            return;
        };
        
        // Convert McpRequest to ElicitationRequest
        let elicitation_request = match Self::convert_mcp_to_elicitation_request(&request) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to convert MCP request to elicitation request from server '{}': {}", server_name, e);
                Self::send_conversion_error(stdin_sender, &request, server_name, &e.to_string()).await;
                return;
            }
        };
        
        // Forward to MagicTunnel Server
        match forwarder.forward_elicitation_request(elicitation_request, server_name, client_id).await {
            Ok(response) => {
                // Convert response back to MCP format and send to external server
                let mcp_response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    result: Some(serde_json::to_value(response).unwrap_or_else(|_| json!(null))),
                    error: None,
                };
                Self::send_response_to_external(stdin_sender, mcp_response, server_name).await;
            }
            Err(e) => {
                error!("Failed to forward elicitation request from server '{}': {}", server_name, e);
                let error_response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    result: None,
                    error: Some(McpError::internal_error(e.to_string())),
                };
                Self::send_response_to_external(stdin_sender, error_response, server_name).await;
            }
        }
    }

    /// Convert MCP request to sampling request
    fn convert_mcp_to_sampling_request(request: &McpRequest) -> Result<crate::mcp::types::SamplingRequest> {
        let params = request.params.as_ref()
            .ok_or_else(|| ProxyError::validation("Missing params in sampling request"))?;
        
        serde_json::from_value(params.clone())
            .map_err(|e| ProxyError::validation(format!("Invalid sampling request params: {}", e)))
    }

    /// Convert MCP request to elicitation request
    fn convert_mcp_to_elicitation_request(request: &McpRequest) -> Result<crate::mcp::types::ElicitationRequest> {
        let params = request.params.as_ref()
            .ok_or_else(|| ProxyError::validation("Missing params in elicitation request"))?;
        
        serde_json::from_value(params.clone())
            .map_err(|e| ProxyError::validation(format!("Invalid elicitation request params: {}", e)))
    }

    /// Send response back to external MCP server
    async fn send_response_to_external(
        stdin_sender: &Option<mpsc::UnboundedSender<String>>,
        response: McpResponse,
        server_name: &str,
    ) {
        if let Some(sender) = stdin_sender {
            match serde_json::to_string(&response) {
                Ok(response_json) => {
                    if let Err(e) = sender.send(response_json) {
                        error!("Failed to send response to MCP server '{}': {}", server_name, e);
                    } else {
                        debug!("Sent response to MCP server '{}': id={}", server_name, response.id);
                    }
                }
                Err(e) => {
                    error!("Failed to serialize response for MCP server '{}': {}", server_name, e);
                }
            }
        } else {
            error!("No stdin sender available for MCP server '{}'", server_name);
        }
    }

    /// Send error response for unsupported method
    async fn send_unsupported_method_error(
        stdin_sender: &Option<mpsc::UnboundedSender<String>>,
        request: &McpRequest,
        server_name: &str,
    ) {
        let error_response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
            result: None,
            error: Some(McpError::method_not_found(format!(
                "Method '{}' not supported for bidirectional communication",
                request.method
            ))),
        };
        Self::send_response_to_external(stdin_sender, error_response, server_name).await;
    }

    /// Send error response for missing request forwarder
    async fn send_no_forwarder_error(
        stdin_sender: &Option<mpsc::UnboundedSender<String>>,
        request: &McpRequest,
        server_name: &str,
    ) {
        let error_response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
            result: None,
            error: Some(McpError::internal_error(
                "Request forwarder not configured for bidirectional communication".to_string()
            )),
        };
        Self::send_response_to_external(stdin_sender, error_response, server_name).await;
    }

    /// Send error response for missing client ID
    async fn send_no_client_error(
        stdin_sender: &Option<mpsc::UnboundedSender<String>>,
        request: &McpRequest,
        server_name: &str,
    ) {
        let error_response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
            result: None,
            error: Some(McpError::internal_error(
                "Original client ID not configured for request forwarding".to_string()
            )),
        };
        Self::send_response_to_external(stdin_sender, error_response, server_name).await;
    }

    /// Send error response for conversion errors
    async fn send_conversion_error(
        stdin_sender: &Option<mpsc::UnboundedSender<String>>,
        request: &McpRequest,
        server_name: &str,
        error_message: &str,
    ) {
        let error_response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
            result: None,
            error: Some(McpError::invalid_params(format!(
                "Request conversion failed: {}",
                error_message
            ))),
        };
        Self::send_response_to_external(stdin_sender, error_response, server_name).await;
    }
}

/// Expand environment variables in a string (supports ${VAR} syntax)
pub fn expand_env_vars(input: &str) -> String {
    let mut result = input.to_string();
    
    // Simple environment variable expansion for ${VAR} syntax
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 2..start + end];
            let replacement = std::env::var(var_name).unwrap_or_default();
            result.replace_range(start..start + end + 1, &replacement);
        } else {
            break;
        }
    }
    
    result
}

// ============================================================================
// ExternalMcpClient Implementation for Bidirectional Communication
// ============================================================================

#[async_trait]
impl ExternalMcpClient for ExternalMcpProcess {
    /// Set the request forwarder for bidirectional communication
    async fn set_request_forwarder(&mut self, forwarder: SharedRequestForwarder) -> Result<()> {
        // Note: We need the original client ID, but we'll set a placeholder for now
        // The actual client ID should be provided when setting up the forwarder
        let client_id = "placeholder-client-id".to_string(); // TODO: Get from context
        self.set_request_forwarder(forwarder, client_id);
        Ok(())
    }

    /// Get the server name for this external process
    fn server_name(&self) -> &str {
        &self.name
    }

    /// Check if the process supports bidirectional communication
    fn supports_bidirectional(&self) -> bool {
        // Stdio always supports bidirectional communication
        true
    }
}

/// Helper method to set request forwarder with client ID
impl ExternalMcpProcess {
    /// Set request forwarder with explicit client ID
    pub fn set_request_forwarder_with_client(&mut self, forwarder: SharedRequestForwarder, client_id: String) {
        self.set_request_forwarder(forwarder, client_id);
    }
}
