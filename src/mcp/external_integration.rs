//! External MCP Integration Module
//! 
//! This module integrates the External MCP Manager with the main application,
//! providing a unified interface for managing External MCP servers.

use crate::config::Config;
use crate::error::{ProxyError, Result};
use crate::mcp::external_manager::ExternalMcpManager;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// External MCP Integration Manager
pub struct ExternalMcpIntegration {
    config: Arc<Config>,
    manager: Option<Arc<ExternalMcpManager>>,
    manager_handle: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for ExternalMcpIntegration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalMcpIntegration")
            .field("config", &"<Config>")
            .field("manager_initialized", &self.manager.is_some())
            .field("has_background_task", &self.manager_handle.is_some())
            .finish()
    }
}

impl ExternalMcpIntegration {
    /// Create a new External MCP integration manager
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            manager: None,
            manager_handle: None,
        }
    }

    /// Start the External MCP integration with client capabilities for safe advertisement
    pub async fn start_with_capabilities(&mut self, client_capabilities: Option<&crate::mcp::types::ClientCapabilities>) -> Result<()> {
        info!("Starting External MCP integration with client capability context");
        debug!("Config state: external_mcp present: {}", self.config.external_mcp.is_some());

        // Check if External MCP is enabled
        let external_mcp_config = match &self.config.external_mcp {
            Some(config) if config.enabled => {
                info!("External MCP configuration found and enabled");
                debug!("External MCP config details: {:?}", config);
                config.clone()
            },
            Some(config) => {
                info!("External MCP is disabled in configuration (enabled={})", config.enabled);
                return Ok(());
            }
            None => {
                info!("External MCP configuration not found");
                return Ok(());
            }
        };

        // Get client configuration with capability context
        let client_config = self.config.mcp_client.clone().unwrap_or_else(|| {
            let mut config = crate::config::McpClientConfig::default();
            config.protocol_version = "2025-06-18".to_string();
            config.client_name = env!("CARGO_PKG_NAME").to_string();
            config.client_version = env!("CARGO_PKG_VERSION").to_string();
            config
        });

        // Create manager
        let manager = Arc::new(ExternalMcpManager::new(external_mcp_config, client_config));
        
        // Store manager reference
        self.manager = Some(manager.clone());

        // Set client capabilities context in the manager and start servers
        if let Some(client_caps) = client_capabilities {
            client_caps.log_capability_advertisement("external MCP integration startup", 
                &client_caps.get_safe_external_advertisement());
            manager.set_client_capabilities_context(Some(client_caps.clone())).await;
        } else {
            info!("⚠️  No client capabilities provided - using default capability advertisement");
        }
        
        let start_result = manager.start().await;

        match start_result {
            Ok(()) => {
                info!("✅ External MCP integration started successfully");
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to start External MCP integration: {}", e);
                Err(e)
            }
        }
    }

    /// Start the External MCP integration (backward compatibility)
    pub async fn start(&mut self) -> Result<()> {
        self.start_with_capabilities(None).await
    }
    
    /// Update client capabilities context in the manager (for runtime updates)
    pub async fn update_client_capabilities(&self, client_capabilities: Option<crate::mcp::types::ClientCapabilities>) -> Result<()> {
        if let Some(manager) = &self.manager {
            manager.set_client_capabilities_context(client_capabilities).await;
            Ok(())
        } else {
            Err(ProxyError::Internal(anyhow::anyhow!("External MCP manager not initialized")))
        }
    }


    /// Start background monitoring task
    async fn start_monitoring_task(&mut self, manager: Arc<ExternalMcpManager>) {
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Check server health and restart if needed
                let active_servers = manager.get_active_servers().await;
                if active_servers.is_empty() {
                    warn!("No active External MCP servers found");
                } else {
                    debug!("Active External MCP servers: {:?}", active_servers);
                }
                
                // TODO: Add health checks and automatic restart logic
            }
        });
        
        self.manager_handle = Some(handle);
    }

    /// Stop the External MCP integration
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping External MCP integration");

        // Stop monitoring task
        if let Some(handle) = self.manager_handle.take() {
            handle.abort();
        }

        // Stop the manager
        if let Some(manager) = &self.manager {
            manager.stop_all().await?;
        }

        self.manager = None;
        info!("External MCP integration stopped");
        Ok(())
    }

    /// Execute a tool on an External MCP server
    pub async fn execute_tool(&self, server_name: &str, tool_name: &str, arguments: Value) -> Result<Value> {
        match &self.manager {
            Some(manager) => {
                manager.execute_tool(server_name, tool_name, arguments).await
            }
            None => {
                Err(ProxyError::connection("External MCP Manager is not running".to_string()))
            }
        }
    }

    /// Execute a tool on an External MCP server with authentication headers
    pub async fn execute_tool_with_auth(
        &self, 
        tool_name: &str, 
        arguments: &Value, 
        auth_headers: std::collections::HashMap<String, String>
    ) -> Result<crate::routing::types::AgentResult> {
        match &self.manager {
            Some(manager) => {
                // For now, we need to determine the server name from the tool name
                // This is a temporary implementation - in the future, we should pass server_name explicitly
                let server_name = self.determine_server_for_tool(tool_name).await
                    .unwrap_or_else(|| "default".to_string());

                debug!(
                    "Executing external MCP tool '{}' on server '{}' with {} auth headers",
                    tool_name, server_name, auth_headers.len()
                );

                // Execute the tool and convert result to AgentResult
                match manager.execute_tool(&server_name, tool_name, arguments.clone()).await {
                    Ok(result) => {
                        Ok(crate::routing::types::AgentResult {
                            success: true,
                            data: Some(result),
                            error: None,
                            metadata: Some(serde_json::json!({
                                "tool_name": tool_name,
                                "server_name": server_name,
                                "execution_type": "external_mcp",
                                "auth_headers_count": auth_headers.len(),
                                "authenticated": !auth_headers.is_empty()
                            })),
                        })
                    }
                    Err(e) => {
                        Ok(crate::routing::types::AgentResult {
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                            metadata: Some(serde_json::json!({
                                "tool_name": tool_name,
                                "server_name": server_name,
                                "execution_type": "external_mcp",
                                "error_category": "execution_failure",
                                "auth_headers_count": auth_headers.len(),
                                "authenticated": !auth_headers.is_empty()
                            })),
                        })
                    }
                }
            }
            None => {
                Ok(crate::routing::types::AgentResult {
                    success: false,
                    data: None,
                    error: Some("External MCP Manager is not running".to_string()),
                    metadata: Some(serde_json::json!({
                        "tool_name": tool_name,
                        "execution_type": "external_mcp",
                        "error_category": "manager_not_running",
                        "auth_headers_count": auth_headers.len(),
                        "authenticated": !auth_headers.is_empty()
                    })),
                })
            }
        }
    }

    /// Determine which server hosts a given tool
    /// This is a temporary helper method - in the future, routing should be more explicit
    async fn determine_server_for_tool(&self, tool_name: &str) -> Option<String> {
        if let Some(manager) = &self.manager {
            // Check all servers to find which one has this tool
            for server_name in manager.get_active_servers().await {
                if let Ok(Some(tools)) = self.get_server_tools(&server_name).await {
                    for tool in tools {
                        if tool.name == tool_name {
                            debug!("Found tool '{}' on server '{}'", tool_name, server_name);
                            return Some(server_name.to_string());
                        }
                    }
                }
            }
        }
        warn!("Could not determine server for tool '{}'", tool_name);
        None
    }

    /// Get all available tools from all External MCP servers
    pub async fn get_all_tools(&self) -> Result<HashMap<String, Vec<crate::mcp::types::Tool>>> {
        match &self.manager {
            Some(manager) => {
                Ok(manager.get_all_tools().await)
            }
            None => {
                Ok(HashMap::new())
            }
        }
    }

    /// Get tools from a specific External MCP server
    pub async fn get_server_tools(&self, server_name: &str) -> Result<Option<Vec<crate::mcp::types::Tool>>> {
        match &self.manager {
            Some(manager) => {
                Ok(manager.get_server_tools(server_name).await)
            }
            None => {
                Ok(None)
            }
        }
    }

    /// Get the external manager reference for monitoring purposes
    pub fn get_manager(&self) -> Option<&Arc<ExternalMcpManager>> {
        self.manager.as_ref()
    }

    /// Get the network service manager reference (placeholder - not implemented yet)
    pub fn get_network_manager(&self) -> Option<&Arc<crate::mcp::network_service_manager::NetworkMcpServiceManager>> {
        // This would be implemented when network service manager is integrated
        None
    }

    /// Check if the integration is running
    pub fn is_running(&self) -> bool {
        self.manager.is_some()
    }

    /// Get status information for the integration
    pub async fn get_status(&self) -> HashMap<String, serde_json::Value> {
        let mut status = HashMap::new();
        
        status.insert("running".to_string(), serde_json::json!(self.is_running()));
        
        if let Some(manager) = &self.manager {
            let active_servers = manager.get_active_servers().await;
            let health_status = manager.get_health_status().await;
            
            status.insert("active_servers".to_string(), serde_json::json!(active_servers));
            status.insert("health_status".to_string(), serde_json::json!(health_status));
        }
        
        status
    }

    /// Get the manager (temporary method to fix compilation)
    pub async fn get_server_tools_fallback(&self, server_name: &str) -> Result<Option<Vec<crate::mcp::types::Tool>>> {
        match &self.manager {
            Some(manager) => {
                Ok(manager.get_server_tools(server_name).await)
            }
            None => {
                Ok(None)
            }
        }
    }

    /// Get list of active External MCP servers
    pub async fn get_active_servers(&self) -> Result<Vec<String>> {
        match &self.manager {
            Some(manager) => {
                Ok(manager.get_active_servers().await)
            }
            None => {
                Ok(Vec::new())
            }
        }
    }

    /// Stop a specific External MCP server
    pub async fn stop_server(&self, server_name: &str) -> Result<()> {
        match &self.manager {
            Some(manager) => {
                manager.stop_server(server_name).await
            }
            None => {
                Err(ProxyError::connection("External MCP Manager is not running".to_string()))
            }
        }
    }

    /// Restart a specific External MCP server
    pub async fn restart_server(&self, server_name: &str) -> Result<()> {
        match &self.manager {
            Some(manager) => {
                manager.restart_server(server_name).await
            }
            None => {
                Err(ProxyError::connection("External MCP Manager is not running".to_string()))
            }
        }
    }

    /// Discover capabilities from all servers
    pub async fn discover_all_capabilities(&self) -> Result<()> {
        match &self.manager {
            Some(manager) => {
                manager.discover_all_capabilities().await
            }
            None => {
                Err(ProxyError::connection("External MCP Manager is not running".to_string()))
            }
        }
    }

    /// Check if External MCP is enabled and running
    pub fn is_enabled(&self) -> bool {
        self.manager.is_some()
    }

    /// Get External MCP configuration
    pub fn get_config(&self) -> Option<&crate::config::ExternalMcpConfig> {
        self.config.external_mcp.as_ref()
    }

    /// Get process information for a specific server
    pub async fn get_server_process_info(&self, server_name: &str) -> Option<(Option<u32>, String)> {
        match &self.manager {
            Some(manager) => {
                manager.get_server_process_info(server_name).await
            }
            None => None
        }
    }

    /// Get metrics collector for accessing MCP service metrics
    pub fn metrics_collector(&self) -> Option<std::sync::Arc<crate::mcp::metrics::McpMetricsCollector>> {
        self.manager.as_ref().map(|manager| manager.metrics_collector())
    }

    /// Get list of external MCP servers that support sampling capability (MCP 2025-06-18)
    pub async fn get_sampling_capable_servers(&self) -> Vec<String> {
        if let Some(manager) = &self.manager {
            manager.get_sampling_capable_servers().await
        } else {
            Vec::new()
        }
    }

    /// Forward sampling request to external MCP server (MCP 2025-06-18)
    pub async fn forward_sampling_request(
        &self,
        server_name: &str,
        request: &crate::mcp::types::sampling::SamplingRequest,
    ) -> std::result::Result<crate::mcp::types::sampling::SamplingResponse, crate::mcp::types::sampling::SamplingError> {
        if let Some(manager) = &self.manager {
            manager.forward_sampling_request(server_name, request).await
        } else {
            Err(crate::mcp::types::sampling::SamplingError {
                code: crate::mcp::types::sampling::SamplingErrorCode::InternalError,
                message: "External MCP manager not initialized".to_string(),
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
        if let Some(manager) = &self.manager {
            manager.forward_elicitation_request(server_name, request).await
        } else {
            Err(crate::mcp::types::elicitation::ElicitationError {
                code: crate::mcp::types::elicitation::ElicitationErrorCode::InternalError,
                message: "External MCP manager not initialized".to_string(),
                details: None,
            })
        }
    }

    /// Get list of external MCP servers that support elicitation capability (MCP 2025-06-18)
    pub async fn get_elicitation_capable_servers(&self) -> Vec<String> {
        if let Some(manager) = &self.manager {
            manager.get_elicitation_capable_servers().await
        } else {
            vec![]
        }
    }
}

impl Drop for ExternalMcpIntegration {
    fn drop(&mut self) {
        if let Some(handle) = self.manager_handle.take() {
            handle.abort();
        }
    }
}

/// External MCP Agent Router
/// 
/// This provides routing functionality for tools that need to be executed
/// on External MCP servers through the agent routing system.
pub struct ExternalMcpAgent {
    integration: Arc<ExternalMcpIntegration>,
}

impl ExternalMcpAgent {
    /// Create a new External MCP Agent
    pub fn new(integration: Arc<ExternalMcpIntegration>) -> Self {
        Self { integration }
    }

    /// Execute a tool through the External MCP system
    pub async fn execute(&self, server_name: &str, tool_name: &str, arguments: Value) -> Result<Value> {
        self.integration.execute_tool(server_name, tool_name, arguments).await
    }

    /// Check if the agent can handle a specific tool
    pub async fn can_handle(&self, server_name: &str, tool_name: &str) -> bool {
        if let Ok(Some(tools)) = self.integration.get_server_tools(server_name).await {
            tools.iter().any(|tool| tool.name == tool_name)
        } else {
            false
        }
    }

    /// Get available tools for routing decisions
    pub async fn get_available_tools(&self) -> HashMap<String, Vec<String>> {
        let mut available_tools = HashMap::new();
        
        if let Ok(all_tools) = self.integration.get_all_tools().await {
            for (server_name, tools) in all_tools {
                let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
                available_tools.insert(server_name, tool_names);
            }
        }
        
        available_tools
    }
}

/// Utility functions for External MCP integration
pub mod utils {
    use super::*;

    /// Check if a command is available in the system PATH
    pub async fn is_command_available(command: &str) -> bool {
        match tokio::process::Command::new("which")
            .arg(command)
            .output()
            .await
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// Validate External MCP server configuration
    pub fn validate_server_config(config: &crate::config::McpServerConfig) -> Result<()> {
        if config.command.is_empty() {
            return Err(ProxyError::config("Server command cannot be empty".to_string()));
        }

        if config.args.is_empty() {
            return Err(ProxyError::config("Server args cannot be empty".to_string()));
        }

        // Validate working directory if specified
        if let Some(ref cwd) = config.cwd {
            if !std::path::Path::new(cwd).exists() {
                return Err(ProxyError::config(format!("Working directory does not exist: {}", cwd)));
            }
        }

        Ok(())
    }

    /// Expand environment variables in server configuration
    pub fn expand_config_env_vars(config: &mut crate::config::McpServerConfig) {
        // Expand environment variables in args
        for arg in &mut config.args {
            *arg = expand_env_vars(arg);
        }

        // Expand environment variables in env values
        if let Some(ref mut env) = config.env {
            for (_, value) in env.iter_mut() {
                *value = expand_env_vars(value);
            }
        }

        // Expand environment variables in cwd
        if let Some(ref mut cwd) = config.cwd {
            *cwd = expand_env_vars(cwd);
        }
    }

    /// Simple environment variable expansion
    fn expand_env_vars(input: &str) -> String {
        let mut result = input.to_string();
        
        // Handle ${VAR} syntax
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
}
