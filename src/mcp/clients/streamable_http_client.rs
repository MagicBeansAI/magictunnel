//! Streamable HTTP MCP Client for MCP 2025-06-18 specification
//!
//! This client connects to external MCP servers using the Streamable HTTP transport
//! with full bidirectional communication support for sampling and elicitation requests.

use crate::config::McpClientConfig;
use crate::error::{ProxyError, Result};
use crate::mcp::request_forwarder::{ExternalMcpClient, SharedRequestForwarder};
use crate::mcp::types::{McpRequest, McpResponse, SamplingRequest, ElicitationRequest};
use async_trait::async_trait;
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for Streamable HTTP MCP Client
#[derive(Debug, Clone)]
pub struct StreamableHttpClientConfig {
    /// Base URL of the external MCP server
    pub base_url: String,
    /// Enable NDJSON streaming
    pub enable_ndjson: bool,
    /// Enable batch requests
    pub enable_batching: bool,
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Authentication headers
    pub auth_headers: HashMap<String, String>,
    /// Enable keep-alive connections
    pub enable_keep_alive: bool,
    /// Maximum concurrent bidirectional requests
    pub max_concurrent_requests: usize,
}

impl Default for StreamableHttpClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3001".to_string(),
            enable_ndjson: true,
            enable_batching: true,
            max_batch_size: 100,
            batch_timeout_ms: 50,
            connection_timeout_seconds: 30,
            request_timeout_seconds: 120,
            auth_headers: HashMap::new(),
            enable_keep_alive: true,
            max_concurrent_requests: 10,
        }
    }
}

/// Streamable HTTP MCP Client for connecting to external MCP servers
pub struct StreamableHttpMcpClient {
    /// Server name for identification
    server_name: String,
    /// Client configuration
    config: StreamableHttpClientConfig,
    /// MCP client configuration
    client_config: McpClientConfig,
    /// HTTP client
    http_client: Client,
    /// Pending requests awaiting responses
    pending_requests: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<McpResponse>>>>,
    /// Request forwarder for bidirectional communication
    request_forwarder: Option<SharedRequestForwarder>,
    /// Original client ID for request context
    original_client_id: Option<String>,
    /// Active bidirectional request handlers
    active_handlers: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// Connection health status
    is_healthy: Arc<RwLock<bool>>,
}

impl StreamableHttpMcpClient {
    /// Create a new Streamable HTTP MCP Client
    pub fn new(
        server_name: String,
        config: StreamableHttpClientConfig,
        client_config: McpClientConfig,
    ) -> Result<Self> {
        // Build HTTP client with configuration
        let mut http_client_builder = Client::builder()
            .timeout(Duration::from_secs(config.connection_timeout_seconds))
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(5);

        if config.enable_keep_alive {
            http_client_builder = http_client_builder.tcp_keepalive(Duration::from_secs(60));
        }

        let http_client = http_client_builder
            .build()
            .map_err(|e| ProxyError::connection(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            server_name,
            config,
            client_config,
            http_client,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            request_forwarder: None,
            original_client_id: None,
            active_handlers: Arc::new(RwLock::new(HashMap::new())),
            is_healthy: Arc::new(RwLock::new(false)),
        })
    }

    /// Send a request to the external MCP server
    pub async fn send_request(&self, method: &str, params: Option<Value>) -> Result<McpResponse> {
        let request_id = Uuid::new_v4().to_string();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(request_id.clone())),
            method: method.to_string(),
            params,
        };

        info!("Sending Streamable HTTP request to '{}': method={}", self.server_name, method);

        // Create response channel
        let (response_tx, _response_rx) = tokio::sync::oneshot::channel();
        
        // Store pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), response_tx);
        }

        // Send request via appropriate method
        let response = if self.config.enable_ndjson {
            self.send_ndjson_request(request).await
        } else {
            self.send_json_request(request).await
        };

        match response {
            Ok(mcp_response) => {
                // Remove from pending requests
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&request_id);

                info!("Received Streamable HTTP response from '{}': id={}", self.server_name, mcp_response.id);
                
                // Update health status
                *self.is_healthy.write().await = true;
                
                Ok(mcp_response)
            }
            Err(e) => {
                // Remove from pending requests on error
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&request_id);
                
                error!("Streamable HTTP request failed for server '{}': {}", self.server_name, e);
                *self.is_healthy.write().await = false;
                Err(e)
            }
        }
    }

    /// Send request using NDJSON streaming
    async fn send_ndjson_request(&self, request: McpRequest) -> Result<McpResponse> {
        let endpoint = format!("{}/mcp/streamable", self.config.base_url);
        let request_json = serde_json::to_string(&request)
            .map_err(|e| ProxyError::mcp(format!("Failed to serialize request: {}", e)))?;

        let mut request_builder = self.http_client
            .post(&endpoint)
            .header("Content-Type", "application/x-ndjson")
            .header("Accept", "application/x-ndjson")
            .header("X-MCP-Transport", "streamable-http")
            .header("X-MCP-Version", "2025-06-18");

        // Add authentication headers
        for (key, value) in &self.config.auth_headers {
            request_builder = request_builder.header(key, value);
        }

        let response = timeout(
            Duration::from_secs(self.config.request_timeout_seconds),
            request_builder.body(request_json).send()
        ).await
        .map_err(|_| ProxyError::timeout(format!("Request to {} timed out", self.server_name)))?
        .map_err(|e| ProxyError::connection(format!("HTTP request failed: {}", e)))?;

        self.parse_ndjson_response(response).await
    }

    /// Send request using regular JSON
    async fn send_json_request(&self, request: McpRequest) -> Result<McpResponse> {
        let endpoint = format!("{}/mcp/streamable", self.config.base_url);
        
        let mut request_builder = self.http_client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("X-MCP-Transport", "streamable-http")
            .header("X-MCP-Version", "2025-06-18")
            .json(&request);

        // Add authentication headers
        for (key, value) in &self.config.auth_headers {
            request_builder = request_builder.header(key, value);
        }

        let response = timeout(
            Duration::from_secs(self.config.request_timeout_seconds),
            request_builder.send()
        ).await
        .map_err(|_| ProxyError::timeout(format!("Request to {} timed out", self.server_name)))?
        .map_err(|e| ProxyError::connection(format!("HTTP request failed: {}", e)))?;

        self.parse_json_response(response).await
    }

    /// Parse NDJSON response and handle bidirectional requests
    async fn parse_ndjson_response(&self, response: Response) -> Result<McpResponse> {
        let status = response.status();
        if !status.is_success() {
            return Err(ProxyError::mcp(format!("HTTP error {}: {}", status, status.canonical_reason().unwrap_or("Unknown"))));
        }

        let response_text = response.text().await
            .map_err(|e| ProxyError::connection(format!("Failed to read response body: {}", e)))?;

        debug!("Received NDJSON response from '{}': {} bytes", self.server_name, response_text.len());

        let mut main_response = None;
        
        // Process each line in the NDJSON response
        for (line_num, line) in response_text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            // Try parsing as McpResponse first
            if let Ok(mcp_response) = serde_json::from_str::<McpResponse>(line) {
                if main_response.is_none() {
                    main_response = Some(mcp_response);
                } else {
                    warn!("Multiple responses in NDJSON stream from server '{}', using first", self.server_name);
                }
                continue;
            }

            // Try parsing as McpRequest for bidirectional communication
            if let Ok(mcp_request) = serde_json::from_str::<McpRequest>(line) {
                debug!("Received bidirectional request in NDJSON from '{}': method={}", self.server_name, mcp_request.method);
                
                // Handle bidirectional request asynchronously
                let request_forwarder = self.request_forwarder.clone();
                let original_client_id = self.original_client_id.clone();
                let server_name = self.server_name.clone();
                let http_client = self.http_client.clone();
                let base_url = self.config.base_url.clone();
                let auth_headers = self.config.auth_headers.clone();
                
                let handler_id = Uuid::new_v4().to_string();
                let active_handlers = Arc::clone(&self.active_handlers);
                let handler_id_clone = handler_id.clone();
                
                let handler = tokio::spawn(async move {
                    Self::handle_bidirectional_request_async(
                        request_forwarder,
                        mcp_request,
                        server_name,
                        original_client_id,
                        http_client,
                        base_url,
                        auth_headers,
                    ).await;
                    
                    // Remove handler from active list
                    let mut handlers = active_handlers.write().await;
                    handlers.remove(&handler_id_clone);
                });
                
                // Store handler
                {
                    let mut handlers = self.active_handlers.write().await;
                    handlers.insert(handler_id, handler);
                }
                
                continue;
            }

            warn!("Failed to parse NDJSON line {} from server '{}': {}", line_num + 1, self.server_name, line);
        }

        if let Some(response) = main_response {
            Ok(response)
        } else {
            Err(ProxyError::mcp(format!("No valid response found in NDJSON stream from server '{}'", self.server_name)))
        }
    }

    /// Parse JSON response 
    async fn parse_json_response(&self, response: Response) -> Result<McpResponse> {
        let status = response.status();
        if !status.is_success() {
            return Err(ProxyError::mcp(format!("HTTP error {}: {}", status, status.canonical_reason().unwrap_or("Unknown"))));
        }

        let mcp_response: McpResponse = response.json().await
            .map_err(|e| ProxyError::mcp(format!("Failed to parse JSON response: {}", e)))?;

        Ok(mcp_response)
    }

    /// Handle bidirectional request asynchronously
    async fn handle_bidirectional_request_async(
        request_forwarder: Option<SharedRequestForwarder>,
        request: McpRequest,
        server_name: String,
        original_client_id: Option<String>,
        http_client: Client,
        base_url: String,
        auth_headers: HashMap<String, String>,
    ) {
        let Some(forwarder) = request_forwarder.as_ref() else {
            warn!("No request forwarder available for bidirectional request from server '{}'", server_name);
            Self::send_error_response_to_server(&http_client, &base_url, &auth_headers, &request, &server_name, "No request forwarder configured").await;
            return;
        };

        let Some(client_id) = original_client_id else {
            warn!("No original client ID available for bidirectional request from server '{}'", server_name);
            Self::send_error_response_to_server(&http_client, &base_url, &auth_headers, &request, &server_name, "No client ID configured").await;
            return;
        };

        match request.method.as_str() {
            "notifications/tools/list_changed" => {
                // Handle tools list_changed notification from external server
                info!("📥 MCP CLIENT RECEIVED NOTIFICATION - Server '{}' sent tools/list_changed notification", server_name);
                
                // Forward the notification to internal MagicTunnel server
                info!("🔔 MCP CLIENT FORWARDING NOTIFICATION - Forwarding tools/list_changed to internal MagicTunnel server from '{}'", server_name);
                
                // Forward the notification using the request forwarder
                if let Err(e) = forwarder.forward_notification("notifications/tools/list_changed", &server_name, &client_id).await {
                    error!("Failed to forward tools/list_changed notification from {}: {}", server_name, e);
                } else {
                    info!("✅ Successfully forwarded tools/list_changed notification from {}", server_name);
                }
                
                // Notifications don't require responses, so we don't send anything back
                return;
            }
            "notifications/resources/list_changed" => {
                // Handle resources list_changed notification from external server  
                info!("📥 MCP CLIENT RECEIVED NOTIFICATION - Server '{}' sent resources/list_changed notification", server_name);
                debug!("Resources list changed notification received from external server: {}", server_name);
                // Notifications don't require responses
                return;
            }
            "notifications/prompts/list_changed" => {
                // Handle prompts list_changed notification from external server
                info!("📥 MCP CLIENT RECEIVED NOTIFICATION - Server '{}' sent prompts/list_changed notification", server_name);
                debug!("Prompts list changed notification received from external server: {}", server_name);
                // Notifications don't require responses
                return;
            }
            "sampling/createMessage" => {
                // Log MCP sampling request received from remote server
                info!("📥 MCP CLIENT RECEIVED SAMPLING - Server '{}' sent sampling/createMessage request", server_name);
                
                // Convert to SamplingRequest and forward
                match Self::convert_mcp_to_sampling_request(&request) {
                    Ok(sampling_request) => {
                        // Log forwarding to internal MagicTunnel server
                        info!("🔄 MCP CLIENT FORWARDING SAMPLING - Forwarding to internal MagicTunnel server from '{}'", server_name);
                        
                        match forwarder.forward_sampling_request(sampling_request, &server_name, &client_id).await {
                            Ok(sampling_response) => {
                                // Log successful response received from internal processing
                                info!("✅ MCP CLIENT SAMPLING SUCCESS - Received response from internal server, sending back to '{}'", server_name);
                                
                                // Send successful response back to external server
                                let response = McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                                    result: Some(serde_json::to_value(sampling_response).unwrap_or_else(|_| json!(null))),
                                    error: None,
                                };
                                Self::send_response_to_server(&http_client, &base_url, &auth_headers, response, &server_name).await;
                            }
                            Err(e) => {
                                error!("❌ MCP CLIENT SAMPLING FORWARDING FAILED - Failed to forward sampling request from server '{}': {}", server_name, e);
                                Self::send_error_response_to_server(&http_client, &base_url, &auth_headers, &request, &server_name, &e.to_string()).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to convert sampling request from server '{}': {}", server_name, e);
                        Self::send_error_response_to_server(&http_client, &base_url, &auth_headers, &request, &server_name, &e.to_string()).await;
                    }
                }
            }
            "elicitation/request" => {
                // Log MCP elicitation request received from remote server
                info!("📥 MCP CLIENT RECEIVED ELICITATION - Server '{}' sent elicitation/request request", server_name);
                
                // Convert to ElicitationRequest and forward
                match Self::convert_mcp_to_elicitation_request(&request) {
                    Ok(elicitation_request) => {
                        // Log forwarding to internal MagicTunnel server
                        info!("🔄 MCP CLIENT FORWARDING ELICITATION - Forwarding to internal MagicTunnel server from '{}'", server_name);
                        
                        match forwarder.forward_elicitation_request(elicitation_request, &server_name, &client_id).await {
                            Ok(elicitation_response) => {
                                // Log successful response received from internal processing
                                info!("✅ MCP CLIENT ELICITATION SUCCESS - Received response from internal server, sending back to '{}'", server_name);
                                
                                // Send successful response back to external server
                                let response = McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                                    result: Some(serde_json::to_value(elicitation_response).unwrap_or_else(|_| json!(null))),
                                    error: None,
                                };
                                Self::send_response_to_server(&http_client, &base_url, &auth_headers, response, &server_name).await;
                            }
                            Err(e) => {
                                error!("❌ MCP CLIENT ELICITATION FORWARDING FAILED - Failed to forward elicitation request from server '{}': {}", server_name, e);
                                Self::send_error_response_to_server(&http_client, &base_url, &auth_headers, &request, &server_name, &e.to_string()).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to convert elicitation request from server '{}': {}", server_name, e);
                        Self::send_error_response_to_server(&http_client, &base_url, &auth_headers, &request, &server_name, &e.to_string()).await;
                    }
                }
            }
            _ => {
                warn!("Unsupported bidirectional method from server '{}': {}", server_name, request.method);
                Self::send_error_response_to_server(&http_client, &base_url, &auth_headers, &request, &server_name, "Unsupported method").await;
            }
        }
    }

    /// Send response back to external server
    async fn send_response_to_server(
        http_client: &Client,
        base_url: &str,
        auth_headers: &HashMap<String, String>,
        response: McpResponse,
        server_name: &str,
    ) {
        let endpoint = format!("{}/mcp/streamable/response", base_url);
        
        let mut request_builder = http_client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .header("X-MCP-Transport", "streamable-http")
            .header("X-MCP-Version", "2025-06-18")
            .json(&response);

        // Add authentication headers
        for (key, value) in auth_headers {
            request_builder = request_builder.header(key, value);
        }

        match request_builder.send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    debug!("Successfully sent bidirectional response to server '{}': id={}", server_name, response.id);
                } else {
                    warn!("Failed to send bidirectional response to server '{}': HTTP {}", server_name, resp.status());
                }
            }
            Err(e) => {
                error!("Failed to send bidirectional response to server '{}': {}", server_name, e);
            }
        }
    }

    /// Send error response back to external server
    async fn send_error_response_to_server(
        http_client: &Client,
        base_url: &str,
        auth_headers: &HashMap<String, String>,
        request: &McpRequest,
        server_name: &str,
        error_message: &str,
    ) {
        let error_response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
            result: None,
            error: Some(crate::mcp::McpError::internal_error(error_message.to_string())),
        };

        Self::send_response_to_server(http_client, base_url, auth_headers, error_response, server_name).await;
    }

    /// Convert MCP request to sampling request
    fn convert_mcp_to_sampling_request(request: &McpRequest) -> Result<SamplingRequest> {
        let params = request.params.as_ref()
            .ok_or_else(|| ProxyError::validation("Missing params in sampling request"))?;
        
        serde_json::from_value(params.clone())
            .map_err(|e| ProxyError::validation(format!("Invalid sampling request params: {}", e)))
    }

    /// Convert MCP request to elicitation request
    fn convert_mcp_to_elicitation_request(request: &McpRequest) -> Result<ElicitationRequest> {
        let params = request.params.as_ref()
            .ok_or_else(|| ProxyError::validation("Missing params in elicitation request"))?;
        
        serde_json::from_value(params.clone())
            .map_err(|e| ProxyError::validation(format!("Invalid elicitation request params: {}", e)))
    }

    /// Check if the client is healthy
    pub async fn is_healthy(&self) -> bool {
        *self.is_healthy.read().await
    }

    /// Get client configuration
    pub fn config(&self) -> &StreamableHttpClientConfig {
        &self.config
    }

    /// Close the client and cleanup resources
    pub async fn close(&self) -> Result<()> {
        info!("Closing Streamable HTTP client for server '{}'", self.server_name);

        // Cancel all active bidirectional request handlers
        let mut handlers = self.active_handlers.write().await;
        for (handler_id, handler) in handlers.drain() {
            debug!("Cancelling bidirectional handler: {}", handler_id);
            handler.abort();
        }

        // Clear pending requests
        let mut pending = self.pending_requests.lock().await;
        pending.clear();

        *self.is_healthy.write().await = false;
        
        Ok(())
    }
}

// ============================================================================
// ExternalMcpClient Implementation for Bidirectional Communication
// ============================================================================

#[async_trait]
impl ExternalMcpClient for StreamableHttpMcpClient {
    /// Set the request forwarder for bidirectional communication
    async fn set_request_forwarder(&mut self, forwarder: SharedRequestForwarder) -> Result<()> {
        let client_id = "streamable-http-client".to_string(); // TODO: Get from context
        self.request_forwarder = Some(forwarder);
        self.original_client_id = Some(client_id);
        info!("Set request forwarder for Streamable HTTP client '{}'", self.server_name);
        Ok(())
    }

    /// Get the server name for this client
    fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Check if the client supports bidirectional communication
    fn supports_bidirectional(&self) -> bool {
        // Streamable HTTP supports bidirectional communication via NDJSON streaming
        self.config.enable_ndjson
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_client_creation() {
        let config = StreamableHttpClientConfig::default();
        let client_config = McpClientConfig {
            connect_timeout_secs: 30,
            request_timeout_secs: 120,
            max_reconnect_attempts: 3,
            reconnect_delay_secs: 5,
            auto_reconnect: true,
            protocol_version: "2025-06-18".to_string(),
            client_name: "test-client".to_string(),
            client_version: "0.3.4".to_string(),
        };

        let client = StreamableHttpMcpClient::new(
            "test-server".to_string(),
            config,
            client_config,
        );

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.server_name(), "test-server");
        assert!(client.supports_bidirectional());
    }

    #[test]
    async fn test_configuration() {
        let mut config = StreamableHttpClientConfig::default();
        config.base_url = "https://example.com".to_string();
        config.enable_ndjson = false;

        let client_config = McpClientConfig {
            connect_timeout_secs: 30,
            request_timeout_secs: 120,
            max_reconnect_attempts: 3,
            reconnect_delay_secs: 5,
            auto_reconnect: true,
            protocol_version: "2025-06-18".to_string(),
            client_name: "test-client".to_string(),
            client_version: "0.3.4".to_string(),
        };

        let client = StreamableHttpMcpClient::new(
            "test-server".to_string(),
            config.clone(),
            client_config,
        ).unwrap();

        assert_eq!(client.config().base_url, "https://example.com");
        assert!(!client.config().enable_ndjson);
        assert!(!client.supports_bidirectional()); // NDJSON disabled = no bidirectional
    }

    #[test]
    async fn test_request_conversion() {
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!("test-123")),
            method: "sampling/createMessage".to_string(),
            params: Some(json!({
                "system_prompt": "Test system prompt",
                "messages": [
                    {
                        "role": "user",
                        "content": "Test message"
                    }
                ],
                "max_tokens": 100
            })),
        };

        let result = StreamableHttpMcpClient::convert_mcp_to_sampling_request(&request);
        assert!(result.is_ok());
        
        let sampling_request = result.unwrap();
        assert_eq!(sampling_request.system_prompt, Some("Test system prompt".to_string()));
        assert_eq!(sampling_request.max_tokens, Some(100));
        assert_eq!(sampling_request.messages.len(), 1);
    }
}