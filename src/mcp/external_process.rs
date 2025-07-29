//! External MCP Process Management
//! 
//! This module manages MCP server processes using Claude Desktop's exact configuration format.
//! It can spawn local processes (npx, uv run, python, node) and containerized processes (docker run).

use crate::config::{McpServerConfig, ExternalMcpServersConfig, ContainerConfig, McpClientConfig};
use crate::error::{ProxyError, Result};
use crate::mcp::types::{McpRequest, McpResponse, Tool};
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

        // Spawn stdout reader task
        let pending_requests = Arc::clone(&self.pending_requests);
        let server_name = self.name.clone();
        let is_healthy = Arc::clone(&self.is_healthy);
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                debug!("MCP server '{}' stdout: {}", server_name, line);
                
                // Parse JSON-RPC response
                match serde_json::from_str::<McpResponse>(&line) {
                    Ok(response) => {
                        // Handle response
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
                    }
                    Err(e) => {
                        warn!("Failed to parse JSON-RPC response from MCP server '{}': {} (line: {})", server_name, e, line);
                    }
                }
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
