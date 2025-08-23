//! AgentRouter trait and implementations for routing tool calls to different agent types

use std::sync::Arc;

use crate::error::{Result, ProxyError};
use crate::mcp::ToolCall;
use crate::registry::{RoutingConfig, ToolDefinition};
use crate::routing::types::{AgentResult, AgentType};
use crate::discovery::SmartDiscoveryRequest;
use async_trait::async_trait;
use base64::Engine;
use serde_json::json;
use tracing::{debug, error, info, warn};

/// Trait for routing tool calls to appropriate agents
#[async_trait]
pub trait AgentRouter: Send + Sync {
    /// Parse routing configuration into agent type
    fn parse_routing_config(&self, routing: &RoutingConfig) -> Result<AgentType>;
    
    /// Execute tool call with the specified agent
    async fn execute_with_agent(&self, tool_call: &ToolCall, agent: &AgentType) -> Result<AgentResult>;
    
    /// Route a tool call to the appropriate agent (convenience method)
    async fn route(&self, tool_call: &ToolCall, tool_def: &ToolDefinition) -> Result<AgentResult> {
        debug!("Routing tool call: {}", tool_call.name);
        
        // Parse routing configuration into agent type
        let agent = self.parse_routing_config(&tool_def.routing)?;
        
        // Execute the tool call with the selected agent
        self.execute_with_agent(tool_call, &agent).await
    }

    /// Support for downcasting to concrete types
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Default implementation of AgentRouter
pub struct DefaultAgentRouter {
    // External MCP integration for handling external MCP tools
    external_mcp: Option<Arc<tokio::sync::RwLock<crate::mcp::external_integration::ExternalMcpIntegration>>>,
    // Registry service for smart discovery
    registry: Option<Arc<crate::registry::RegistryService>>,
    // Smart discovery service for intelligent tool selection
    smart_discovery: Option<Arc<crate::discovery::SmartDiscoveryService>>,
}

impl DefaultAgentRouter {
    /// Create a new default agent router
    pub fn new() -> Self {
        Self {
            external_mcp: None,
            registry: None,
            smart_discovery: None,
        }
    }

    /// Set the external MCP integration
    pub fn with_external_mcp(
        mut self,
        external_mcp: Arc<tokio::sync::RwLock<crate::mcp::external_integration::ExternalMcpIntegration>>
    ) -> Self {
        self.external_mcp = Some(external_mcp);
        self
    }

    /// Set the registry service for smart discovery
    pub fn with_registry(mut self, registry: Arc<crate::registry::RegistryService>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Set the smart discovery service
    pub fn with_smart_discovery(mut self, smart_discovery: Arc<crate::discovery::SmartDiscoveryService>) -> Self {
        self.smart_discovery = Some(smart_discovery);
        self
    }
}

impl Default for DefaultAgentRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRouter for DefaultAgentRouter {
    fn parse_routing_config(&self, routing: &RoutingConfig) -> Result<AgentType> {
        use crate::error::ProxyError;
        
        match routing.r#type.as_str() {
            "subprocess" => {
                let config = &routing.config;
                Ok(AgentType::Subprocess {
                    command: config.get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("echo")
                        .to_string(),
                    args: config.get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect())
                        .unwrap_or_else(|| vec!["Not configured".to_string()]),
                    timeout: config.get("timeout")
                        .and_then(|v| v.as_u64()),
                    env: config.get("env")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()),
                })
            }
            "http" => {
                let config = &routing.config;
                Ok(AgentType::Http {
                    method: config.get("method")
                        .and_then(|v| v.as_str())
                        .unwrap_or("GET")
                        .to_string(),
                    url: config.get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("http://localhost")
                        .to_string(),
                    headers: config.get("headers")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()),
                    timeout: config.get("timeout")
                        .and_then(|v| v.as_u64()),
                })
            }
            "llm" => {
                let config = &routing.config;
                Ok(AgentType::Llm {
                    provider: config.get("provider")
                        .and_then(|v| v.as_str())
                        .unwrap_or("openai")
                        .to_string(),
                    model: config.get("model")
                        .and_then(|v| v.as_str())
                        .unwrap_or("default-model") // Should be specified in config
                        .to_string(),
                    api_key: config.get("api_key")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    base_url: config.get("base_url")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    timeout: config.get("timeout")
                        .and_then(|v| v.as_u64()),
                })
            }
            "websocket" => {
                let config = &routing.config;
                Ok(AgentType::WebSocket {
                    url: config.get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ws://localhost")
                        .to_string(),
                    headers: config.get("headers")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()),
                })
            }
            "database" => {
                let config = &routing.config;
                Ok(AgentType::Database {
                    db_type: config.get("db_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("sqlite")
                        .to_string(),
                    connection_string: config.get("connection_string")
                        .and_then(|v| v.as_str())
                        .unwrap_or(":memory:")
                        .to_string(),
                    query: config.get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("SELECT 1")
                        .to_string(),
                    timeout: config.get("timeout")
                        .and_then(|v| v.as_u64()),
                })
            }

            "grpc" => {
                let config = &routing.config;
                Ok(AgentType::Grpc {
                    endpoint: config.get("endpoint")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ProxyError::routing("gRPC agent requires endpoint".to_string()))?
                        .to_string(),
                    service: config.get("service")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ProxyError::routing("gRPC agent requires service".to_string()))?
                        .to_string(),
                    method: config.get("method")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ProxyError::routing("gRPC agent requires method".to_string()))?
                        .to_string(),
                    headers: config.get("headers")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()),
                    timeout: config.get("timeout")
                        .and_then(|v| v.as_u64()),
                    request_body: config.get("request_body")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
            }
            "sse" => {
                let config = &routing.config;
                Ok(AgentType::Sse {
                    url: config.get("url")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ProxyError::routing("SSE agent requires url".to_string()))?
                        .to_string(),
                    headers: config.get("headers")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()),
                    timeout: config.get("timeout")
                        .and_then(|v| v.as_u64()),
                    max_events: config.get("max_events")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32),
                    event_filter: config.get("event_filter")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
            }
            "graphql" => {
                let config = &routing.config;
                Ok(AgentType::GraphQL {
                    endpoint: config.get("endpoint")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ProxyError::routing("GraphQL agent requires endpoint".to_string()))?
                        .to_string(),
                    query: config.get("query")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    variables: config.get("variables").cloned(),
                    headers: config.get("headers")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()),
                    timeout: config.get("timeout")
                        .and_then(|v| v.as_u64()),
                    operation_name: config.get("operation_name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
            }
            "external_mcp" => {
                let config = &routing.config;
                Ok(AgentType::ExternalMcp {
                    server_name: config.get("server_name")
                        .or_else(|| config.get("endpoint"))
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ProxyError::routing("External MCP requires server_name or endpoint".to_string()))?
                        .to_string(),
                    tool_name: config.get("tool_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    timeout: config.get("timeout")
                        .and_then(|v| v.as_u64()),
                    mapping_metadata: config.get("mapping_metadata")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()),
                })
            }
            "smart_discovery" => {
                let config = &routing.config;
                
                Ok(AgentType::SmartDiscovery {
                    enabled: config.get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                })
            }
            _ => Err(ProxyError::routing(format!(
                "Unknown routing type: {}",
                routing.r#type
            ))),
        }
    }

    async fn execute_with_agent(&self, tool_call: &ToolCall, agent: &AgentType) -> Result<AgentResult> {
        // Handle external MCP tools using routing config instead of name parsing
        if let AgentType::ExternalMcp { server_name, tool_name, .. } = agent {
            let server_name = server_name.clone();
            let tool_name = tool_name.clone();

            // Use the external MCP integration to execute the tool
            if let Some(external_mcp) = &self.external_mcp {
                debug!("External MCP integration is available, executing tool: {} on server: {}", tool_name, server_name);
                let integration = external_mcp.read().await;
                match integration.execute_tool(&server_name, &tool_name, tool_call.arguments.clone()).await {
                    Ok(result) => {
                        return Ok(AgentResult {
                            success: true,
                            data: Some(result),
                            error: None,
                            metadata: Some(json!({
                                "routing_type": "external_mcp",
                                "server_name": server_name,
                                "tool_name": tool_name,
                                "executed_via": "external_mcp_integration"
                            })),
                        });
                    }
                    Err(e) => {
                        return Ok(AgentResult {
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                            metadata: Some(json!({
                                "routing_type": "external_mcp",
                                "server_name": server_name,
                                "tool_name": tool_name,
                                "error_category": "external_mcp_execution_failed"
                            })),
                        });
                    }
                }
            } else {
                // Fallback if external MCP integration is not available
                warn!("External MCP integration not available for tool: {} on server: {}", tool_name, server_name);
                debug!("self.external_mcp is None - router was not initialized with external MCP support");
                return Ok(AgentResult {
                    success: false,
                    data: None,
                    error: Some("External MCP integration not available".to_string()),
                    metadata: Some(json!({
                        "routing_type": "external_mcp",
                        "server_name": server_name,
                        "tool_name": tool_name,
                        "error_category": "external_mcp_not_available"
                    })),
                });
            }
        }

        // Regular agent execution for non-external MCP tools
        match agent {
            AgentType::Subprocess { command, args, timeout, env } => {
                self.execute_subprocess_agent(tool_call, command, args, *timeout, env).await
            }
            AgentType::Http { method, url, headers, timeout } => {
                self.execute_http_agent(tool_call, method, url, headers, *timeout).await
            }
            AgentType::Llm { provider, model, api_key, base_url, timeout } => {
                self.execute_llm_agent(tool_call, provider, model, api_key, base_url, *timeout).await
            }
            AgentType::WebSocket { url, headers } => {
                self.execute_websocket_agent(tool_call, url, headers).await
            }
            AgentType::Database { db_type, connection_string, query, timeout } => {
                self.execute_database_agent(tool_call, db_type, connection_string, query, *timeout).await
            }
            AgentType::Grpc { endpoint, service, method, headers, timeout, request_body } => {
                self.execute_grpc_agent(tool_call, endpoint, service, method, headers, *timeout, request_body).await
            }
            AgentType::Sse { url, headers, timeout, max_events, event_filter } => {
                self.execute_sse_agent(tool_call, url, headers, *timeout, *max_events, event_filter).await
            }
            AgentType::GraphQL { endpoint, query, variables, headers, timeout, operation_name } => {
                self.execute_graphql_agent(tool_call, endpoint, query, variables, headers, *timeout, operation_name).await
            }
            // External MCP agent type
            AgentType::ExternalMcp { server_name, tool_name, .. } => {
                Err(crate::error::ProxyError::routing(format!(
                    "External MCP agent (server: {}, tool: {}) should be handled by the external MCP integration at a higher level, not directly by the agent router",
                    server_name, tool_name
                )))
            }
            // Smart Discovery agent type
            AgentType::SmartDiscovery { enabled } => {
                self.execute_smart_discovery_agent(tool_call, *enabled).await
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl DefaultAgentRouter {
    /// Execute subprocess agent
    async fn execute_subprocess_agent(
        &self,
        tool_call: &ToolCall,
        command: &str,
        args: &[String],
        timeout: Option<u64>,
        env: &Option<std::collections::HashMap<String, String>>
    ) -> Result<AgentResult> {
        use crate::routing::substitution::substitute_parameters;
        use tokio::process::Command;
        use tokio::time::{timeout as tokio_timeout, Duration};
        use serde_json::json;

        debug!("Executing subprocess agent: {} {:?}", command, args);

        // Substitute parameters in command and args
        let substituted_args = substitute_parameters(args, &tool_call.arguments)?;

        // Create command
        let mut cmd = Command::new(command);
        cmd.args(&substituted_args);

        // Set environment variables if provided
        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        // Execute with timeout
        let timeout_duration = Duration::from_secs(timeout.unwrap_or(30));
        let result = tokio_timeout(timeout_duration, cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                if output.status.success() {
                    Ok(AgentResult {
                        success: true,
                        data: Some(json!({
                            "stdout": stdout,
                            "stderr": stderr,
                            "exit_code": output.status.code()
                        })),
                        error: None,
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "subprocess",
                            "command": command,
                            "args": substituted_args
                        })),
                    })
                } else {
                    Ok(AgentResult {
                        success: false,
                        data: Some(json!({
                            "stdout": stdout,
                            "stderr": stderr,
                            "exit_code": output.status.code()
                        })),
                        error: Some(format!("Command failed with exit code: {:?}", output.status.code())),
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "subprocess",
                            "command": command,
                            "args": substituted_args
                        })),
                    })
                }
            }
            Ok(Err(e)) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(format!("Failed to execute command: {}", e)),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "subprocess",
                    "command": command,
                    "args": substituted_args
                })),
            }),
            Err(_) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(format!("Command timed out after {} seconds", timeout.unwrap_or(30))),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "subprocess",
                    "command": command,
                    "args": substituted_args
                })),
            }),
        }
    }

    /// Execute HTTP agent
    async fn execute_http_agent(
        &self,
        tool_call: &ToolCall,
        method: &str,
        url: &str,
        headers: &Option<std::collections::HashMap<String, String>>,
        timeout: Option<u64>
    ) -> Result<AgentResult> {
        use crate::routing::substitution::{substitute_parameter_string, substitute_headers};
        use reqwest::Client;
        use serde_json::json;
        use tokio::time::{timeout as tokio_timeout, Duration};

        debug!("Executing HTTP agent: {} {}", method, url);

        // Substitute parameters in URL
        let substituted_url = substitute_parameter_string(url, &tool_call.arguments)?;

        // Substitute parameters in headers
        let substituted_headers = substitute_headers(headers, &tool_call.arguments)?;

        // Create HTTP client with timeout
        let timeout_duration = Duration::from_secs(timeout.unwrap_or(30));
        let client = Client::builder()
            .timeout(timeout_duration)
            .use_rustls_tls()
            .tls_built_in_root_certs(true)
            .build()
            .map_err(|e| crate::error::ProxyError::routing(format!("Failed to create HTTP client: {}", e)))?;

        // Build request
        let mut request_builder = match method.to_uppercase().as_str() {
            "GET" => client.get(&substituted_url),
            "POST" => client.post(&substituted_url),
            "PUT" => client.put(&substituted_url),
            "DELETE" => client.delete(&substituted_url),
            "PATCH" => client.patch(&substituted_url),
            "HEAD" => client.head(&substituted_url),
            _ => return Ok(AgentResult {
                success: false,
                data: None,
                error: Some(format!("Unsupported HTTP method: {}", method)),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "http",
                    "method": method,
                    "url": substituted_url
                })),
            }),
        };

        // Add headers
        if let Some(header_map) = &substituted_headers {
            for (key, value) in header_map {
                request_builder = request_builder.header(key, value);
            }
        }

        // Add JSON body for POST/PUT/PATCH requests
        if matches!(method.to_uppercase().as_str(), "POST" | "PUT" | "PATCH") {
            request_builder = request_builder.json(&tool_call.arguments);
        }

        // Execute request with timeout
        let result = tokio_timeout(timeout_duration, request_builder.send()).await;

        match result {
            Ok(Ok(response)) => {
                let status = response.status();
                let headers_map: std::collections::HashMap<String, String> = response
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();

                match response.text().await {
                    Ok(body) => {
                        let success = status.is_success();
                        Ok(AgentResult {
                            success,
                            data: Some(json!({
                                "status": status.as_u16(),
                                "headers": headers_map,
                                "body": body
                            })),
                            error: if success { None } else { Some(format!("HTTP request failed with status: {}", status)) },
                            metadata: Some(json!({
                                "tool_name": tool_call.name,
                                "execution_type": "http",
                                "method": method,
                                "url": substituted_url,
                                "status_code": status.as_u16()
                            })),
                        })
                    }
                    Err(e) => Ok(AgentResult {
                        success: false,
                        data: Some(json!({
                            "status": status.as_u16(),
                            "headers": headers_map
                        })),
                        error: Some(format!("Failed to read response body: {}", e)),
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "http",
                            "method": method,
                            "url": substituted_url,
                            "status_code": status.as_u16()
                        })),
                    })
                }
            }
            Ok(Err(e)) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(format!("HTTP request failed: {}", e)),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "http",
                    "method": method,
                    "url": substituted_url
                })),
            }),
            Err(_) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(format!("HTTP request timed out after {} seconds", timeout.unwrap_or(30))),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "http",
                    "method": method,
                    "url": substituted_url
                })),
            }),
        }
    }

    /// Execute LLM agent
    async fn execute_llm_agent(
        &self,
        tool_call: &ToolCall,
        provider: &str,
        model: &str,
        api_key: &Option<String>,
        base_url: &Option<String>,
        timeout: Option<u64>
    ) -> Result<AgentResult> {
        use serde_json::json;

        debug!("Executing LLM agent: {} {}", provider, model);

        // For now, implement OpenAI-compatible API
        match provider {
            "openai" | "openai-compatible" => {
                self.execute_openai_llm(tool_call, model, api_key, base_url, timeout).await
            }
            "ollama" => {
                self.execute_ollama_llm(tool_call, model, base_url, timeout).await
            }
            _ => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(format!("Unsupported LLM provider: {}", provider)),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "llm",
                    "provider": provider,
                    "model": model
                })),
            })
        }
    }

    /// Execute OpenAI-compatible LLM
    async fn execute_openai_llm(
        &self,
        tool_call: &ToolCall,
        model: &str,
        api_key: &Option<String>,
        base_url: &Option<String>,
        timeout: Option<u64>
    ) -> Result<AgentResult> {
        use reqwest::Client;
        use serde_json::json;
        use tokio::time::{timeout as tokio_timeout, Duration};

        let api_key = api_key.as_ref().ok_or_else(|| {
            crate::error::ProxyError::routing("API key required for OpenAI LLM".to_string())
        })?;

        let base_url = base_url.as_deref().unwrap_or("https://api.openai.com/v1");
        let url = format!("{}/chat/completions", base_url);

        // Extract prompt from tool call arguments
        let prompt = tool_call.arguments.get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("No prompt provided");

        let timeout_duration = Duration::from_secs(timeout.unwrap_or(60));
        let client = Client::builder()
            .timeout(timeout_duration)
            .use_rustls_tls()
            .tls_built_in_root_certs(true)
            .build()
            .map_err(|e| crate::error::ProxyError::routing(format!("Failed to create HTTP client: {}", e)))?;

        let request_body = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": tool_call.arguments.get("max_tokens").unwrap_or(&json!(1000)),
            "temperature": tool_call.arguments.get("temperature").unwrap_or(&json!(0.7))
        });

        let result = tokio_timeout(
            timeout_duration,
            client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
        ).await;

        match result {
            Ok(Ok(response)) => {
                let status = response.status();
                match response.json::<serde_json::Value>().await {
                    Ok(response_json) => {
                        let success = status.is_success();
                        Ok(AgentResult {
                            success,
                            data: Some(response_json),
                            error: if success { None } else { Some(format!("LLM request failed with status: {}", status)) },
                            metadata: Some(json!({
                                "tool_name": tool_call.name,
                                "execution_type": "llm",
                                "provider": "openai",
                                "model": model,
                                "status_code": status.as_u16()
                            })),
                        })
                    }
                    Err(e) => Ok(AgentResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to parse LLM response: {}", e)),
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "llm",
                            "provider": "openai",
                            "model": model,
                            "status_code": status.as_u16()
                        })),
                    })
                }
            }
            Ok(Err(e)) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(format!("LLM request failed: {}", e)),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "llm",
                    "provider": "openai",
                    "model": model
                })),
            }),
            Err(_) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(format!("LLM request timed out after {} seconds", timeout.unwrap_or(60))),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "llm",
                    "provider": "openai",
                    "model": model
                })),
            }),
        }
    }

    /// Execute Ollama LLM
    async fn execute_ollama_llm(
        &self,
        tool_call: &ToolCall,
        model: &str,
        base_url: &Option<String>,
        timeout: Option<u64>
    ) -> Result<AgentResult> {
        use reqwest::Client;
        use serde_json::json;
        use tokio::time::{timeout as tokio_timeout, Duration};

        let base_url = base_url.as_deref().unwrap_or("http://localhost:11434");
        let url = format!("{}/api/generate", base_url);

        // Extract prompt from tool call arguments
        let prompt = tool_call.arguments.get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("No prompt provided");

        let timeout_duration = Duration::from_secs(timeout.unwrap_or(60));
        let client = Client::builder()
            .timeout(timeout_duration)
            .use_rustls_tls()
            .tls_built_in_root_certs(true)
            .build()
            .map_err(|e| crate::error::ProxyError::routing(format!("Failed to create HTTP client: {}", e)))?;

        let request_body = json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        });

        let result = tokio_timeout(
            timeout_duration,
            client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
        ).await;

        match result {
            Ok(Ok(response)) => {
                let status = response.status();
                match response.json::<serde_json::Value>().await {
                    Ok(response_json) => {
                        let success = status.is_success();
                        Ok(AgentResult {
                            success,
                            data: Some(response_json),
                            error: if success { None } else { Some(format!("Ollama request failed with status: {}", status)) },
                            metadata: Some(json!({
                                "tool_name": tool_call.name,
                                "execution_type": "llm",
                                "provider": "ollama",
                                "model": model,
                                "status_code": status.as_u16()
                            })),
                        })
                    }
                    Err(e) => Ok(AgentResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to parse Ollama response: {}", e)),
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "llm",
                            "provider": "ollama",
                            "model": model,
                            "status_code": status.as_u16()
                        })),
                    })
                }
            }
            Ok(Err(e)) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(format!("Ollama request failed: {}", e)),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "llm",
                    "provider": "ollama",
                    "model": model
                })),
            }),
            Err(_) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(format!("Ollama request timed out after {} seconds", timeout.unwrap_or(60))),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "llm",
                    "provider": "ollama",
                    "model": model
                })),
            }),
        }
    }

    /// Execute WebSocket agent
    async fn execute_websocket_agent(
        &self,
        tool_call: &ToolCall,
        url: &str,
        _headers: &Option<std::collections::HashMap<String, String>>
    ) -> Result<AgentResult> {
        use crate::routing::substitution::substitute_parameter_string;
        use serde_json::json;
        use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
        use futures_util::{SinkExt, StreamExt};
        use tokio::time::{timeout as tokio_timeout, Duration};

        debug!("Executing WebSocket agent: {}", url);

        // Substitute parameters in URL
        let substituted_url = substitute_parameter_string(url, &tool_call.arguments)?;

        // For now, implement a simple WebSocket message send/receive
        let timeout_duration = Duration::from_secs(30);

        let result = tokio_timeout(timeout_duration, async {
            // Connect to WebSocket
            let (ws_stream, _) = connect_async(&substituted_url).await
                .map_err(|e| crate::error::ProxyError::routing(format!("WebSocket connection failed: {}", e)))?;

            let (mut write, mut read) = ws_stream.split();

            // Send the tool call as JSON message
            let message = json!({
                "tool_call": tool_call,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            write.send(Message::Text(message.to_string())).await
                .map_err(|e| crate::error::ProxyError::routing(format!("Failed to send WebSocket message: {}", e)))?;

            // Wait for response (with timeout)
            let response_timeout = Duration::from_secs(10);
            let response = tokio_timeout(response_timeout, read.next()).await;

            match response {
                Ok(Some(Ok(Message::Text(text)))) => {
                    // Try to parse as JSON, fallback to plain text
                    let data = serde_json::from_str::<serde_json::Value>(&text)
                        .unwrap_or_else(|_| json!({"text": text}));

                    Ok(AgentResult {
                        success: true,
                        data: Some(data),
                        error: None,
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "websocket",
                            "url": substituted_url
                        })),
                    })
                }
                Ok(Some(Ok(Message::Binary(data)))) => {
                    Ok(AgentResult {
                        success: true,
                        data: Some(json!({
                            "binary_data": base64::prelude::BASE64_STANDARD.encode(&data),
                            "length": data.len()
                        })),
                        error: None,
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "websocket",
                            "url": substituted_url,
                            "data_type": "binary"
                        })),
                    })
                }
                Ok(Some(Ok(msg))) => {
                    Ok(AgentResult {
                        success: true,
                        data: Some(json!({
                            "message_type": format!("{:?}", msg),
                            "raw_message": msg.to_string()
                        })),
                        error: None,
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "websocket",
                            "url": substituted_url
                        })),
                    })
                }
                Ok(Some(Err(e))) => {
                    Ok(AgentResult {
                        success: false,
                        data: None,
                        error: Some(format!("WebSocket message error: {}", e)),
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "websocket",
                            "url": substituted_url
                        })),
                    })
                }
                Ok(None) => {
                    Ok(AgentResult {
                        success: false,
                        data: None,
                        error: Some("WebSocket connection closed without response".to_string()),
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "websocket",
                            "url": substituted_url
                        })),
                    })
                }
                Err(_) => {
                    Ok(AgentResult {
                        success: false,
                        data: None,
                        error: Some("WebSocket response timed out".to_string()),
                        metadata: Some(json!({
                            "tool_name": tool_call.name,
                            "execution_type": "websocket",
                            "url": substituted_url
                        })),
                    })
                }
            }
        }).await;

        match result {
            Ok(agent_result) => agent_result,
            Err(_) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some("WebSocket operation timed out".to_string()),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "websocket",
                    "url": substituted_url
                })),
            }),
        }
    }

    /// Execute database agent
    async fn execute_database_agent(
        &self,
        tool_call: &ToolCall,
        db_type: &str,
        connection_string: &str,
        query: &str,
        timeout: Option<u64>
    ) -> Result<AgentResult> {
        use crate::routing::substitution::substitute_parameter_string;
        use serde_json::json;
        use tokio::time::{timeout as tokio_timeout, Duration};

        debug!("Executing database agent: {} on {}", db_type, connection_string);

        // Substitute parameters in connection string and query
        let substituted_connection = substitute_parameter_string(connection_string, &tool_call.arguments)?;
        let substituted_query = substitute_parameter_string(query, &tool_call.arguments)?;

        let timeout_duration = Duration::from_secs(timeout.unwrap_or(30));

        let result = tokio_timeout(timeout_duration, async {
            match db_type {
                "postgresql" | "postgres" => {
                    self.execute_postgres_query(&substituted_connection, &substituted_query).await
                }
                "sqlite" => {
                    self.execute_sqlite_query(&substituted_connection, &substituted_query).await
                }
                _ => Err(crate::error::ProxyError::routing(format!(
                    "Unsupported database type: {}",
                    db_type
                )))
            }
        }).await;

        match result {
            Ok(Ok(data)) => Ok(AgentResult {
                success: true,
                data: Some(data),
                error: None,
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "database",
                    "db_type": db_type,
                    "query": substituted_query
                })),
            }),
            Ok(Err(e)) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(e.to_string()),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "database",
                    "db_type": db_type,
                    "error_type": "execution_error"
                })),
            }),
            Err(_) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some("Database query timeout".to_string()),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "database",
                    "db_type": db_type,
                    "error_type": "timeout"
                })),
            }),
        }
    }

    /// Execute PostgreSQL query
    async fn execute_postgres_query(
        &self,
        connection_string: &str,
        query: &str
    ) -> Result<serde_json::Value> {
        use tokio_postgres::NoTls;
        use serde_json::json;

        // Connect to PostgreSQL
        let (client, connection) = tokio_postgres::connect(connection_string, NoTls).await
            .map_err(|e| crate::error::ProxyError::routing(format!("PostgreSQL connection failed: {}", e)))?;

        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        // Execute query
        let rows = client.query(query, &[]).await
            .map_err(|e| crate::error::ProxyError::routing(format!("PostgreSQL query failed: {}", e)))?;

        // Convert rows to JSON
        let mut results = Vec::new();
        for row in rows {
            let mut row_data = serde_json::Map::new();
            for (i, column) in row.columns().iter().enumerate() {
                let column_name = column.name();
                let value: serde_json::Value = match column.type_() {
                    &tokio_postgres::types::Type::INT4 => {
                        json!(row.get::<_, Option<i32>>(i))
                    }
                    &tokio_postgres::types::Type::INT8 => {
                        json!(row.get::<_, Option<i64>>(i))
                    }
                    &tokio_postgres::types::Type::TEXT | &tokio_postgres::types::Type::VARCHAR => {
                        json!(row.get::<_, Option<String>>(i))
                    }
                    &tokio_postgres::types::Type::BOOL => {
                        json!(row.get::<_, Option<bool>>(i))
                    }
                    &tokio_postgres::types::Type::FLOAT4 => {
                        json!(row.get::<_, Option<f32>>(i))
                    }
                    &tokio_postgres::types::Type::FLOAT8 => {
                        json!(row.get::<_, Option<f64>>(i))
                    }
                    _ => {
                        // Fallback to string representation
                        json!(row.get::<_, Option<String>>(i))
                    }
                };
                row_data.insert(column_name.to_string(), value);
            }
            results.push(json!(row_data));
        }

        Ok(json!({
            "rows": results,
            "row_count": results.len()
        }))
    }

    /// Execute SQLite query
    async fn execute_sqlite_query(
        &self,
        connection_string: &str,
        query: &str
    ) -> Result<serde_json::Value> {
        use rusqlite::{Connection, params};
        use serde_json::json;

        // Execute in blocking task since rusqlite is synchronous
        let connection_string = connection_string.to_string();
        let query = query.to_string();

        let result = tokio::task::spawn_blocking(move || {
            // Connect to SQLite
            let conn = Connection::open(&connection_string)
                .map_err(|e| crate::error::ProxyError::routing(format!("SQLite connection failed: {}", e)))?;

            // Prepare and execute query
            let mut stmt = conn.prepare(&query)
                .map_err(|e| crate::error::ProxyError::routing(format!("SQLite query preparation failed: {}", e)))?;

            let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

            let rows = stmt.query_map(params![], |row| {
                let mut row_data = serde_json::Map::new();
                for (i, column_name) in column_names.iter().enumerate() {
                    let value: serde_json::Value = match row.get_ref(i) {
                        Ok(rusqlite::types::ValueRef::Null) => json!(null),
                        Ok(rusqlite::types::ValueRef::Integer(i)) => json!(i),
                        Ok(rusqlite::types::ValueRef::Real(f)) => json!(f),
                        Ok(rusqlite::types::ValueRef::Text(s)) => json!(String::from_utf8_lossy(s)),
                        Ok(rusqlite::types::ValueRef::Blob(b)) => json!(base64::prelude::BASE64_STANDARD.encode(b)),
                        Err(_) => json!(null),
                    };
                    row_data.insert(column_name.clone(), value);
                }
                Ok(json!(row_data))
            }).map_err(|e| crate::error::ProxyError::routing(format!("SQLite query execution failed: {}", e)))?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row.map_err(|e| crate::error::ProxyError::routing(format!("SQLite row processing failed: {}", e)))?);
            }

            Ok(json!({
                "rows": results,
                "row_count": results.len()
            }))
        }).await;

        match result {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(crate::error::ProxyError::routing(format!("SQLite task failed: {}", e))),
        }
    }



    /// Execute gRPC agent
    async fn execute_grpc_agent(
        &self,
        tool_call: &ToolCall,
        endpoint: &str,
        service: &str,
        method: &str,
        headers: &Option<std::collections::HashMap<String, String>>,
        timeout: Option<u64>,
        request_body: &Option<String>,
    ) -> Result<AgentResult> {
        use crate::routing::substitution::{substitute_parameter_string, substitute_headers};
        use serde_json::json;
        use tokio::time::{timeout as tokio_timeout, Duration};

        debug!("Executing gRPC agent: {} {}/{}", endpoint, service, method);

        // Substitute parameters in endpoint
        let substituted_endpoint = substitute_parameter_string(endpoint, &tool_call.arguments)?;

        // Substitute parameters in headers
        let substituted_headers = substitute_headers(headers, &tool_call.arguments)?;

        // Substitute parameters in request body
        let substituted_request_body = if let Some(body) = request_body {
            Some(substitute_parameter_string(body, &tool_call.arguments)?)
        } else {
            None
        };

        let timeout_duration = Duration::from_secs(timeout.unwrap_or(30));

        let result = tokio_timeout(timeout_duration, async {
            // For now, we'll implement a mock gRPC call for testing
            // In a real implementation, you would:
            // 1. Create gRPC channel: Endpoint::from_shared(endpoint)?.connect().await?
            // 2. Use proper protobuf definitions and generated client code
            // 3. Make actual gRPC calls with proper request/response types

            let response_data = self.make_generic_grpc_call(
                service,
                method,
                &substituted_request_body,
                &substituted_headers,
            ).await?;

            Ok::<serde_json::Value, crate::error::ProxyError>(response_data)
        }).await;

        match result {
            Ok(Ok(data)) => Ok(AgentResult {
                success: true,
                data: Some(data),
                error: None,
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "grpc",
                    "endpoint": substituted_endpoint,
                    "service": service,
                    "method": method
                })),
            }),
            Ok(Err(e)) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(e.to_string()),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "grpc",
                    "endpoint": substituted_endpoint,
                    "service": service,
                    "method": method,
                    "error_type": "grpc_error"
                })),
            }),
            Err(_) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some("gRPC request timeout".to_string()),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "grpc",
                    "endpoint": substituted_endpoint,
                    "service": service,
                    "method": method,
                    "error_type": "timeout"
                })),
            }),
        }
    }

    /// Make a generic gRPC call using gRPC reflection or direct HTTP/2
    async fn make_generic_grpc_call(
        &self,
        service: &str,
        method: &str,
        request_body: &Option<String>,
        headers: &Option<std::collections::HashMap<String, String>>,
    ) -> Result<serde_json::Value> {
        use serde_json::json;

        debug!("Making gRPC call to {}/{}", service, method);

        // Create HTTP/2 client for gRPC-over-HTTP
        let client = reqwest::Client::builder()
            .http2_prior_knowledge()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| crate::error::ProxyError::routing(format!("Failed to create gRPC client: {}", e)))?;

        // Construct gRPC endpoint (assume it's in the service name for this generic implementation)
        let grpc_endpoint = if service.starts_with("http") {
            service.to_string()
        } else {
            format!("http://localhost:50051/{}", service)
        };

        // Prepare gRPC request body
        let grpc_request_body = if let Some(body) = request_body {
            body.clone()
        } else {
            "{}".to_string()
        };

        // Create gRPC request
        let mut request_builder = client
            .post(format!("{}/{}", grpc_endpoint, method))
            .header("content-type", "application/grpc+proto")
            .header("grpc-encoding", "identity");

        // Add custom headers if provided
        if let Some(header_map) = headers {
            for (key, value) in header_map {
                request_builder = request_builder.header(key, value);
            }
        }

        // For generic gRPC calls, we'll attempt to send JSON that gets converted to protobuf
        // This is a simplified approach - in production you'd use proper protobuf definitions
        match request_builder.body(grpc_request_body).send().await {
            Ok(response) => {
                let status = response.status();
                let response_headers = response.headers().clone();
                
                match response.text().await {
                    Ok(response_text) => {
                        // Extract gRPC status from trailers or headers
                        let grpc_status = response_headers
                            .get("grpc-status")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("0");

                        let grpc_message = response_headers
                            .get("grpc-message")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");

                        if grpc_status == "0" {
                            // Success
                            info!("✅ gRPC call succeeded: {}/{}", service, method);
                            
                            // Try to parse response as JSON, otherwise return raw text
                            let parsed_response = if response_text.trim().is_empty() {
                                json!({
                                    "success": true,
                                    "message": "gRPC call completed successfully (empty response)"
                                })
                            } else if let Ok(json_response) = serde_json::from_str::<serde_json::Value>(&response_text) {
                                json_response
                            } else {
                                json!({
                                    "success": true,
                                    "response": response_text
                                })
                            };
                            
                            Ok(json!({
                                "status": "success",
                                "service": service,
                                "method": method,
                                "grpc_status": grpc_status,
                                "result": parsed_response,
                                "timestamp": chrono::Utc::now().to_rfc3339()
                            }))
                        } else {
                            // gRPC error
                            error!("gRPC call failed: {}/{}, status: {}, message: {}", service, method, grpc_status, grpc_message);
                            Ok(json!({
                                "status": "error",
                                "service": service,
                                "method": method,
                                "grpc_status": grpc_status,
                                "grpc_message": grpc_message,
                                "error": format!("gRPC error: {} - {}", grpc_status, grpc_message),
                                "timestamp": chrono::Utc::now().to_rfc3339()
                            }))
                        }
                    },
                    Err(e) => {
                        error!("Failed to read gRPC response body: {}", e);
                        Err(crate::error::ProxyError::routing(format!("Failed to read gRPC response: {}", e)))
                    }
                }
            },
            Err(e) => {
                error!("Failed to send gRPC request to {}/{}: {}", service, method, e);
                
                // Return error response instead of failing completely
                Ok(json!({
                    "status": "error",
                    "service": service,
                    "method": method,
                    "error": format!("gRPC request failed: {}", e),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            }
        }
    }

    /// Execute SSE agent
    async fn execute_sse_agent(
        &self,
        tool_call: &ToolCall,
        url: &str,
        headers: &Option<std::collections::HashMap<String, String>>,
        timeout: Option<u64>,
        max_events: Option<u32>,
        event_filter: &Option<String>,
    ) -> Result<AgentResult> {
        use crate::routing::substitution::{substitute_parameter_string, substitute_headers};
        use serde_json::json;
        use tokio::time::{timeout as tokio_timeout, Duration};

        debug!("Executing SSE agent: {}", url);

        // Substitute parameters in URL
        let substituted_url = substitute_parameter_string(url, &tool_call.arguments)?;

        // Substitute parameters in headers
        let substituted_headers = substitute_headers(headers, &tool_call.arguments)?;

        // Substitute parameters in event filter
        let substituted_event_filter = if let Some(filter) = event_filter {
            Some(substitute_parameter_string(filter, &tool_call.arguments)?)
        } else {
            None
        };

        let timeout_duration = Duration::from_secs(timeout.unwrap_or(30));

        let result = tokio_timeout(timeout_duration, async {
            // For now, we'll implement a mock SSE call for testing
            // In a real implementation, you would:
            // 1. Create SSE client with reqwest or eventsource-stream
            // 2. Connect to the SSE endpoint
            // 3. Listen for events and filter/aggregate as needed
            // 4. Return collected events or stream them

            let response_data = self.make_generic_sse_call(
                &substituted_url,
                &substituted_headers,
                max_events,
                &substituted_event_filter,
            ).await?;

            Ok::<serde_json::Value, crate::error::ProxyError>(response_data)
        }).await;

        match result {
            Ok(Ok(data)) => Ok(AgentResult {
                success: true,
                data: Some(data),
                error: None,
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "sse",
                    "url": substituted_url,
                    "max_events": max_events,
                    "event_filter": substituted_event_filter
                })),
            }),
            Ok(Err(e)) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some(e.to_string()),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "sse",
                    "url": substituted_url,
                    "error_type": "sse_error"
                })),
            }),
            Err(_) => Ok(AgentResult {
                success: false,
                data: None,
                error: Some("SSE request timeout".to_string()),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "sse",
                    "url": substituted_url,
                    "error_type": "timeout"
                })),
            }),
        }
    }

    /// Make a generic SSE call (simplified implementation)
    async fn make_generic_sse_call(
        &self,
        url: &str,
        headers: &Option<std::collections::HashMap<String, String>>,
        max_events: Option<u32>,
        event_filter: &Option<String>,
    ) -> Result<serde_json::Value> {
        use serde_json::json;
        use tokio::time::{timeout, Duration};
        use futures_util::StreamExt;

        debug!("Making SSE call to {}", url);

        // Create HTTP client
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| crate::error::ProxyError::routing(format!("Failed to create SSE client: {}", e)))?;

        // Build request
        let mut request_builder = client.get(url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache");

        // Add custom headers if provided
        if let Some(header_map) = headers {
            for (key, value) in header_map {
                request_builder = request_builder.header(key, value);
            }
        }

        // Connect and collect events
        let events_limit = max_events.unwrap_or(10);
        let mut collected_events = Vec::new();
        
        let result = timeout(Duration::from_secs(10), async {
            let response = request_builder.send().await
                .map_err(|e| crate::error::ProxyError::routing(format!("SSE connection failed: {}", e)))?;

            if !response.status().is_success() {
                return Err(crate::error::ProxyError::routing(format!("SSE endpoint returned error: {}", response.status())));
            }

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut event_count = 0;

            while let Some(chunk) = stream.next().await {
                if event_count >= events_limit {
                    break;
                }

                match chunk {
                    Ok(bytes_chunk) => {
                        let chunk_str = String::from_utf8_lossy(&bytes_chunk);
                        buffer.push_str(&chunk_str);

                        // Process complete lines
                        let buffer_content = buffer.clone();
                        let lines: Vec<&str> = buffer_content.split('\n').collect();
                        buffer = lines.last().unwrap_or(&"").to_string();

                        let mut current_event = json!({});
                        let mut has_data = false;

                        for line in &lines[..lines.len()-1] {
                            let line = line.trim();
                            
                            if line.is_empty() {
                                // End of event
                                if has_data {
                                    // Apply event filter if specified
                                    let should_include = if let Some(filter) = event_filter {
                                        let event_str = current_event.to_string().to_lowercase();
                                        event_str.contains(&filter.to_lowercase())
                                    } else {
                                        true
                                    };

                                    if should_include {
                                        current_event["timestamp"] = json!(chrono::Utc::now().to_rfc3339());
                                        collected_events.push(current_event.clone());
                                        event_count += 1;
                                        
                                        if event_count >= events_limit {
                                            break;
                                        }
                                    }

                                    current_event = json!({});
                                    has_data = false;
                                }
                            } else if line.starts_with("data: ") {
                                let data = &line[6..];
                                current_event["data"] = json!(data);
                                has_data = true;
                            } else if line.starts_with("event: ") {
                                let event_type = &line[7..];
                                current_event["event"] = json!(event_type);
                            } else if line.starts_with("id: ") {
                                let id = &line[4..];
                                current_event["id"] = json!(id);
                            } else if line.starts_with("retry: ") {
                                let retry = &line[7..];
                                current_event["retry"] = json!(retry);
                            }
                        }
                    },
                    Err(e) => {
                        error!("Error reading SSE stream: {}", e);
                        break;
                    }
                }
            }

            Ok::<Vec<serde_json::Value>, crate::error::ProxyError>(collected_events)
        }).await;
        
        match result {
            Ok(Ok(events)) => {
                info!("✅ SSE call completed: {} events collected from {}", events.len(), url);
                Ok(json!({
                    "status": "success",
                    "url": url,
                    "events": events,
                    "event_count": events.len(),
                    "max_events_requested": events_limit,
                    "event_filter": event_filter,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            },
            Ok(Err(e)) => {
                error!("SSE call failed for {}: {}", url, e);
                Ok(json!({
                    "status": "error",
                    "url": url,
                    "error": format!("SSE error: {}", e),
                    "event_count": 0,
                    "events": Vec::<serde_json::Value>::new(),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            },
            Err(_) => {
                warn!("SSE call timed out for {}, returning partial results", url);
                Ok(json!({
                    "status": "timeout",
                    "url": url,
                    "event_count": 0,
                    "events": Vec::<serde_json::Value>::new(),
                    "message": "Connection timed out, partial results not available",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            }
        }
    }

    /// Execute GraphQL agent
    async fn execute_graphql_agent(
        &self,
        tool_call: &ToolCall,
        endpoint: &str,
        query: &Option<String>,
        variables: &Option<serde_json::Value>,
        headers: &Option<std::collections::HashMap<String, String>>,
        timeout: Option<u64>,
        operation_name: &Option<String>,
    ) -> Result<AgentResult> {
        use crate::routing::substitution::{substitute_parameter_string, substitute_headers, substitute_json_value};
        use serde_json::json;
        use tokio::time::{timeout as tokio_timeout, Duration};

        debug!("Executing GraphQL agent: {}", endpoint);

        // Substitute parameters in endpoint
        let substituted_endpoint = substitute_parameter_string(endpoint, &tool_call.arguments)?;

        // Substitute parameters in headers
        let substituted_headers = substitute_headers(headers, &tool_call.arguments)?;

        // Substitute parameters in query (if provided)
        let substituted_query = if let Some(q) = query {
            Some(substitute_parameter_string(q, &tool_call.arguments)?)
        } else {
            // If no query in config, try to get it from tool call arguments
            tool_call.arguments.get("query")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        };

        // Substitute parameters in variables
        let substituted_variables = if let Some(vars) = variables {
            Some(substitute_json_value(vars, &tool_call.arguments)?)
        } else {
            // If no variables in config, try to get them from tool call arguments
            tool_call.arguments.get("variables").cloned()
        };

        // Substitute parameters in operation name
        let substituted_operation_name = if let Some(op_name) = operation_name {
            Some(substitute_parameter_string(op_name, &tool_call.arguments)?)
        } else {
            tool_call.arguments.get("operation_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        };

        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(timeout.unwrap_or(30));

        let result = tokio_timeout(timeout_duration, async {
            let response_data = self.make_graphql_request(
                &substituted_endpoint,
                &substituted_query,
                &substituted_variables,
                &substituted_headers,
                &substituted_operation_name,
            ).await?;

            Ok::<serde_json::Value, crate::error::ProxyError>(response_data)
        }).await;

        match result {
            Ok(Ok(response_data)) => {
                debug!("GraphQL request completed successfully");
                Ok(AgentResult {
                    success: true,
                    data: Some(response_data),
                    error: None,
                    metadata: Some(json!({
                        "tool_name": tool_call.name,
                        "execution_type": "graphql",
                        "endpoint": substituted_endpoint,
                        "operation_name": substituted_operation_name,
                        "execution_time_ms": start_time.elapsed().as_millis()
                    })),
                })
            }
            Ok(Err(e)) => {
                error!("GraphQL request failed: {}", e);
                Ok(AgentResult {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    metadata: Some(json!({
                        "tool_name": tool_call.name,
                        "execution_type": "graphql",
                        "endpoint": substituted_endpoint,
                        "error_type": "graphql_error"
                    })),
                })
            }
            Err(_) => {
                error!("GraphQL request timed out after {}s", timeout.unwrap_or(30));
                Ok(AgentResult {
                    success: false,
                    data: None,
                    error: Some(format!("Request timed out after {}s", timeout.unwrap_or(30))),
                    metadata: Some(json!({
                        "tool_name": tool_call.name,
                        "execution_type": "graphql",
                        "endpoint": substituted_endpoint,
                        "error_type": "timeout"
                    })),
                })
            }
        }
    }

    /// Make a GraphQL request using real HTTP client
    async fn make_graphql_request(
        &self,
        endpoint: &str,
        query: &Option<String>,
        variables: &Option<serde_json::Value>,
        headers: &Option<std::collections::HashMap<String, String>>,
        operation_name: &Option<String>,
    ) -> Result<serde_json::Value> {
        use serde_json::json;
        use reqwest::Client;

        debug!("Making GraphQL request to {}", endpoint);

        // Ensure we have a query to execute
        let query_string = match query {
            Some(q) => q.clone(),
            None => {
                return Err(ProxyError::routing("GraphQL query is required".to_string()));
            }
        };

        // Build the GraphQL request body
        let mut request_body = json!({
            "query": query_string
        });
        
        if let Some(vars) = variables {
            request_body["variables"] = vars.clone();
        }
        
        if let Some(op_name) = operation_name {
            request_body["operationName"] = json!(op_name);
        }

        // Create HTTP client
        let client = Client::new();
        let mut request_builder = client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .json(&request_body);

        // Add custom headers if provided
        if let Some(custom_headers) = headers {
            for (key, value) in custom_headers {
                request_builder = request_builder.header(key, value);
            }
        }

        // Execute the request
        let start_time = std::time::Instant::now();
        let response = request_builder
            .send()
            .await
            .map_err(|e| ProxyError::routing(format!("GraphQL HTTP request failed: {}", e)))?;
        
        let duration = start_time.elapsed();
        let status = response.status();
        
        // Check if the HTTP request was successful
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProxyError::routing(format!(
                "GraphQL HTTP request failed with status {}: {}", 
                status, error_text
            )));
        }

        // Parse the response
        let response_text = response.text().await
            .map_err(|e| ProxyError::routing(format!("Failed to read GraphQL response: {}", e)))?;
        
        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| ProxyError::routing(format!("Failed to parse GraphQL response JSON: {}", e)))?;

        // Check for GraphQL errors in the response
        if let Some(errors) = response_json.get("errors") {
            if errors.is_array() && !errors.as_array().unwrap().is_empty() {
                warn!("GraphQL response contains errors: {}", errors);
                // Still return the response as GraphQL can have partial data with errors
            }
        }

        // Add execution metadata to the response
        let mut enhanced_response = response_json;
        if enhanced_response.get("extensions").is_none() {
            enhanced_response["extensions"] = json!({});
        }
        
        enhanced_response["extensions"]["execution"] = json!({
            "duration_ms": duration.as_millis(),
            "endpoint": endpoint,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "status": "success"
        });

        debug!("GraphQL request completed in {}ms", duration.as_millis());
        Ok(enhanced_response)
    }

    /// Execute Smart Discovery agent
    async fn execute_smart_discovery_agent(
        &self,
        tool_call: &ToolCall,
        enabled: bool,
    ) -> Result<AgentResult> {
        use serde_json::json;

        debug!("Executing Smart Discovery agent");

        // Check if smart discovery is enabled
        if !enabled {
            return Ok(AgentResult {
                success: false,
                data: None,
                error: Some("Smart discovery is disabled".to_string()),
                metadata: Some(json!({
                    "tool_name": tool_call.name,
                    "execution_type": "smart_discovery",
                    "enabled": enabled
                })),
            });
        }

        // Use the injected smart discovery service
        let smart_discovery_service = match &self.smart_discovery {
            Some(service) => service,
            None => {
                return Ok(AgentResult {
                    success: false,
                    data: None,
                    error: Some("Smart discovery service not available".to_string()),
                    metadata: Some(json!({
                        "tool_name": tool_call.name,
                        "execution_type": "smart_discovery",
                        "error": "service_not_available"
                    })),
                });
            }
        };

        // Parse the request from tool call arguments
        let request = match self.parse_smart_discovery_request(tool_call) {
            Ok(req) => req,
            Err(e) => {
                return Ok(AgentResult {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to parse smart discovery request: {}", e)),
                    metadata: Some(json!({
                        "tool_name": tool_call.name,
                        "execution_type": "smart_discovery",
                        "error": "parse_error"
                    })),
                });
            }
        };

        // Execute smart discovery using the injected service
        match smart_discovery_service.discover_and_execute(request).await {
            Ok(discovery_response) => {
                // Check if discovery was successful and we have a tool to execute
                if discovery_response.success && discovery_response.metadata.original_tool.is_some() {
                    let discovered_tool_name = discovery_response.metadata.original_tool.as_ref().unwrap();
                    
                    // Get the extracted parameters
                    if let Some(extracted_params) = &discovery_response.metadata.mapped_parameters {
                        // Look up the discovered tool definition
                        if let Some(registry) = &self.registry {
                            if let Some(discovered_tool_def) = registry.get_tool(discovered_tool_name) {
                            // Create a new tool call with the discovered tool and extracted parameters
                            let discovered_tool_call = ToolCall::new(
                                discovered_tool_name.clone(),
                                serde_json::Value::Object(extracted_params.clone().into_iter().collect())
                            );
                            
                            debug!("Executing discovered tool '{}' with extracted parameters", discovered_tool_name);
                            
                            // Route the call to the discovered tool
                            match self.route(&discovered_tool_call, &discovered_tool_def).await {
                                Ok(mut execution_result) => {
                                    // Add smart discovery metadata to the execution result
                                    if let Some(ref mut metadata) = execution_result.metadata {
                                        metadata["routing_type"] = json!("smart_discovery");
                                        metadata["original_tool"] = json!(discovered_tool_name);
                                        metadata["confidence_score"] = json!(discovery_response.metadata.confidence_score);
                                        metadata["discovery_reasoning"] = json!(discovery_response.metadata.reasoning);
                                        
                                        // Include tool candidates for debugging and analysis
                                        if let Some(tool_candidates) = &discovery_response.metadata.tool_candidates {
                                            metadata["tool_candidates"] = json!(tool_candidates);
                                        }
                                        
                                        // Include next step recommendation if present
                                        if let Some(next_step) = &discovery_response.next_step {
                                            metadata["next_step"] = json!(next_step);
                                        }
                                    } else {
                                        let mut new_metadata = json!({
                                            "routing_type": "smart_discovery",
                                            "original_tool": discovered_tool_name,
                                            "confidence_score": discovery_response.metadata.confidence_score,
                                            "discovery_reasoning": discovery_response.metadata.reasoning
                                        });
                                        
                                        // Include tool candidates for debugging and analysis
                                        if let Some(tool_candidates) = &discovery_response.metadata.tool_candidates {
                                            new_metadata["tool_candidates"] = json!(tool_candidates);
                                        }
                                        
                                        // Include next step recommendation if present
                                        if let Some(next_step) = &discovery_response.next_step {
                                            new_metadata["next_step"] = json!(next_step);
                                        }
                                        
                                        execution_result.metadata = Some(new_metadata);
                                    }
                                    
                                    Ok(execution_result)
                                }
                                Err(e) => {
                                    Ok(AgentResult {
                                        success: false,
                                        data: None,
                                        error: Some(format!("Failed to execute discovered tool '{}': {}", discovered_tool_name, e)),
                                        metadata: Some(json!({
                                            "tool_name": tool_call.name,
                                            "execution_type": "smart_discovery",
                                            "discovered_tool": discovered_tool_name,
                                            "error": "tool_execution_failed"
                                        })),
                                    })
                                }
                            }
                            } else {
                                Ok(AgentResult {
                                    success: false,
                                    data: None,
                                    error: Some(format!("Discovered tool '{}' not found in registry", discovered_tool_name)),
                                    metadata: Some(json!({
                                        "tool_name": tool_call.name,
                                        "execution_type": "smart_discovery",
                                        "discovered_tool": discovered_tool_name,
                                        "error": "discovered_tool_not_found"
                                    })),
                                })
                            }
                        } else {
                            Ok(AgentResult {
                                success: false,
                                data: None,
                                error: Some("Registry service not available".to_string()),
                                metadata: Some(json!({
                                    "tool_name": tool_call.name,
                                    "execution_type": "smart_discovery",
                                    "error": "registry_not_available"
                                })),
                            })
                        }
                    } else {
                        Ok(AgentResult {
                            success: false,
                            data: None,
                            error: Some("Parameter extraction failed: no parameters extracted".to_string()),
                            metadata: Some(json!({
                                "tool_name": tool_call.name,
                                "execution_type": "smart_discovery",
                                "error": "parameter_extraction_failed"
                            })),
                        })
                    }
                } else {
                    // Discovery failed, return the discovery response as-is
                    let mut metadata = json!({
                        "tool_name": tool_call.name,
                        "execution_type": "smart_discovery",
                        "discovery_metadata": discovery_response.metadata
                    });
                    
                    // Include next step recommendation if present
                    if let Some(next_step) = &discovery_response.next_step {
                        metadata["next_step"] = json!(next_step);
                    }
                    
                    Ok(AgentResult {
                        success: discovery_response.success,
                        data: discovery_response.data,
                        error: discovery_response.error,
                        metadata: Some(metadata),
                    })
                }
            }
            Err(e) => {
                Ok(AgentResult {
                    success: false,
                    data: None,
                    error: Some(format!("Smart discovery execution failed: {}", e)),
                    metadata: Some(json!({
                        "tool_name": tool_call.name,
                        "execution_type": "smart_discovery",
                        "error": "execution_error"
                    })),
                })
            }
        }
    }

    /// Parse smart discovery request from tool call
    fn parse_smart_discovery_request(&self, tool_call: &ToolCall) -> Result<SmartDiscoveryRequest> {
        let request_str = tool_call.arguments.get("request")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::ProxyError::validation("Missing 'request' parameter".to_string()))?;

        let context = tool_call.arguments.get("context")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let preferred_tools = tool_call.arguments.get("preferred_tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            });

        let confidence_threshold = tool_call.arguments.get("confidence_threshold")
            .and_then(|v| v.as_f64());

        Ok(SmartDiscoveryRequest {
            request: request_str.to_string(),
            context,
            preferred_tools,
            confidence_threshold,
            include_error_details: None,
            sequential_mode: None,
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl DefaultAgentRouter {
    /// Route a tool call with authentication context
    pub async fn route_with_auth(
        &self,
        tool_call: &ToolCall,
        tool_def: &ToolDefinition,
        auth_context: Option<&crate::auth::AuthenticationContext>,
    ) -> Result<AgentResult> {
        debug!("Routing tool call with auth: {} (auth: {})", 
               tool_call.name, 
               auth_context.is_some());
        
        // Parse routing configuration into agent type
        let agent = self.parse_routing_config(&tool_def.routing)?;
        
        // Execute the tool call with the selected agent and authentication context
        self.execute_with_agent_and_auth(tool_call, &agent, auth_context).await
    }

    /// Execute tool call with agent and authentication context
    async fn execute_with_agent_and_auth(
        &self,
        tool_call: &ToolCall,
        agent: &AgentType,
        auth_context: Option<&crate::auth::AuthenticationContext>,
    ) -> Result<AgentResult> {
        // For now, we'll handle authentication context only for external MCP calls
        // Other agent types (subprocess, HTTP, LLM, etc.) will be enhanced later
        match agent {
            AgentType::ExternalMcp { server_name, tool_name, .. } => {
                self.execute_external_mcp_with_auth(tool_call, server_name, tool_name, auth_context).await
            }
            _ => {
                // For other agent types, fall back to standard execution for now
                debug!("Authentication context not yet supported for agent type: {:?}", agent);
                self.execute_with_agent(tool_call, agent).await
            }
        }
    }

    /// Execute external MCP tool call with authentication context
    async fn execute_external_mcp_with_auth(
        &self,
        tool_call: &ToolCall,
        server_name: &str,
        tool_name: &str,
        auth_context: Option<&crate::auth::AuthenticationContext>,
    ) -> Result<AgentResult> {
        if let Some(external_mcp) = &self.external_mcp {
            let external_mcp_guard = external_mcp.read().await;
            
            // Get authentication headers if context is available
            let auth_headers = auth_context.map(|ctx| {
                // For external MCP calls, we don't specify a provider to use the best available token
                ctx.get_auth_headers(None)
            }).unwrap_or_default();

            debug!(
                "Executing external MCP tool '{}' on server '{}' with auth headers: {}",
                tool_name, server_name, auth_headers.len()
            );

            // For now, call the standard external MCP execution
            // The external MCP integration will need to be updated separately to use auth headers
            external_mcp_guard.execute_tool_with_auth(tool_name, &tool_call.arguments, auth_headers).await
        } else {
            Err(crate::error::ProxyError::routing(
                "External MCP integration not configured".to_string()
            ))
        }
    }
}
