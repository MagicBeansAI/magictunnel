//! MCP Client for connecting to external MCP servers

use crate::error::{ProxyError, Result};
use crate::mcp::types::{
    Tool, ToolCall, ToolResult, McpRequest, McpResponse, ToolListResponse
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message, tungstenite::client::IntoClientRequest, tungstenite::http::HeaderName};
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use base64::{Engine as _, engine::general_purpose};
use reqwest;

#[derive(Debug, Clone)]
pub enum Protocol {
    WebSocket,
    SSE,
}

impl Protocol {
    pub fn from_config_and_url(config_protocol: Option<&str>, endpoint: &str) -> Self {
        match config_protocol {
            Some("websocket") | Some("ws") => Protocol::WebSocket,
            Some("sse") | Some("server-sent-events") => Protocol::SSE,
            Some("auto") | None => {
                // Auto-detect based on URL
                if endpoint.contains("/sse") || endpoint.starts_with("https://") {
                    Protocol::SSE
                } else {
                    Protocol::WebSocket
                }
            }
            _ => Protocol::WebSocket, // Default fallback
        }
    }
}

// Default functions for serde
fn default_protocol_version() -> String {
    "2025-06-18".to_string()
}

fn default_client_name() -> String {
    env!("CARGO_PKG_NAME").to_string()
}

fn default_client_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// MCP Prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<McpPromptArgument>>,
}

/// MCP Prompt argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

/// MCP Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerCapabilities {
    pub protocol_version: String,
    pub server_info: McpServerInfo,
    pub capabilities: McpCapabilities,
}

/// MCP Server info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
}

/// MCP Capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapabilities {
    pub tools: Option<McpToolCapabilities>,
    pub resources: Option<McpResourceCapabilities>,
    pub prompts: Option<McpPromptCapabilities>,
    pub logging: Option<Value>,
}

/// Tool capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCapabilities {
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// Resource capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceCapabilities {
    pub subscribe: Option<bool>,
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// Prompt capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptCapabilities {
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// MCP Client for connecting to external MCP servers
#[derive(Debug)]
pub struct McpClient {
    /// Server endpoint URL
    pub endpoint: String,
    /// Server name/identifier
    pub name: String,
    /// Connection protocol (WebSocket or SSE)
    protocol: Protocol,
    /// Connection state
    connection_state: Arc<RwLock<ConnectionState>>,
    /// Pending requests waiting for responses
    pending_requests: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<McpResponse>>>>,
    /// Message sender for outgoing messages (WebSocket only)
    message_sender: Option<mpsc::UnboundedSender<Message>>,
    /// HTTP client for SSE requests
    http_client: Option<reqwest::Client>,
    /// SSE event stream receiver (SSE only)
    sse_receiver: Option<mpsc::UnboundedReceiver<String>>,
    /// Connection configuration
    config: ClientConfig,
    // Authentication removed - External MCP uses local processes, no auth needed
    // Authentication removed - External MCP uses local processes, no auth needed
}

/// Connection state for MCP client
#[derive(Debug, Clone)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected { connected_at: Instant },
    Reconnecting { attempt: u32, last_attempt: Instant },
    Failed { error: String, failed_at: Instant },
}

/// Configuration for MCP client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay in seconds
    pub reconnect_delay_secs: u64,
    /// Enable automatic reconnection
    pub auto_reconnect: bool,
    /// MCP protocol version to use
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    /// Client name for MCP handshake
    #[serde(default = "default_client_name")]
    pub client_name: String,
    /// Client version for MCP handshake
    #[serde(default = "default_client_version")]
    pub client_version: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 30,
            request_timeout_secs: 60,
            max_reconnect_attempts: 5,
            reconnect_delay_secs: 5,
            auto_reconnect: true,
            protocol_version: "2025-06-18".to_string(),
            client_name: env!("CARGO_PKG_NAME").to_string(),
            client_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl From<crate::config::McpClientConfig> for ClientConfig {
    fn from(config: crate::config::McpClientConfig) -> Self {
        Self {
            connect_timeout_secs: config.connect_timeout_secs,
            request_timeout_secs: config.request_timeout_secs,
            max_reconnect_attempts: config.max_reconnect_attempts,
            reconnect_delay_secs: config.reconnect_delay_secs,
            auto_reconnect: config.auto_reconnect,
            protocol_version: config.protocol_version,
            client_name: config.client_name,
            client_version: config.client_version,
        }
    }
}

impl McpClient {
    /// Create a new MCP client
    pub fn new(name: String, endpoint: String) -> Self {
        let protocol = Protocol::from_config_and_url(None, &endpoint);
        Self {
            protocol: protocol.clone(),
            endpoint,
            name,
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            message_sender: None,
            http_client: match protocol {
                Protocol::SSE => Some(reqwest::Client::new()),
                Protocol::WebSocket => None,
            },
            sse_receiver: None,
            config: ClientConfig::default(),
            // auth_config removed - External MCP uses local processes
        }
    }

    /// Create a new MCP client with custom configuration
    pub fn with_config(name: String, endpoint: String, config: ClientConfig) -> Self {
        let protocol = Protocol::from_config_and_url(None, &endpoint);
        Self {
            protocol: protocol.clone(),
            endpoint,
            name,
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            message_sender: None,
            http_client: match protocol {
                Protocol::SSE => Some(reqwest::Client::new()),
                Protocol::WebSocket => None,
            },
            sse_receiver: None,
            config,
            // auth_config removed - External MCP uses local processes
        }
    }

    /// Create a new MCP client with authentication configuration
    // Legacy method - authentication removed for External MCP
    #[deprecated(since = "0.2.14", note = "Use new() instead - External MCP uses local processes")]
    pub fn with_auth(name: String, endpoint: String, config: ClientConfig, _auth_config: crate::config::AuthConfig) -> Self {
        let protocol = Protocol::from_config_and_url(None, &endpoint);
        Self {
            protocol: protocol.clone(),
            endpoint,
            name,
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            message_sender: None,
            http_client: match protocol {
                Protocol::SSE => Some(reqwest::Client::new()),
                Protocol::WebSocket => None,
            },
            sse_receiver: None,
            config,
            // auth_config removed - External MCP uses local processes
        }
    }

    /// Create a new MCP client with protocol, configuration, and authentication
    pub fn with_protocol_and_auth(
        name: String,
        endpoint: String,
        protocol_config: Option<String>,
        config: ClientConfig,
        _auth_config: crate::config::AuthConfig  // Deprecated - not used
    ) -> Self {
        let protocol = Protocol::from_config_and_url(protocol_config.as_deref(), &endpoint);
        Self {
            protocol: protocol.clone(),
            endpoint,
            name,
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            message_sender: None,
            http_client: match protocol {
                Protocol::SSE => Some(reqwest::Client::new()),
                Protocol::WebSocket => None,
            },
            sse_receiver: None,
            config,
            // auth_config removed - External MCP uses local processes
        }
    }

    /// Create a new MCP client with protocol and configuration
    pub fn with_protocol_and_config(
        name: String,
        endpoint: String,
        protocol_config: Option<String>,
        config: ClientConfig
    ) -> Self {
        let protocol = Protocol::from_config_and_url(protocol_config.as_deref(), &endpoint);
        Self {
            protocol: protocol.clone(),
            endpoint,
            name,
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            message_sender: None,
            http_client: match protocol {
                Protocol::SSE => Some(reqwest::Client::new()),
                Protocol::WebSocket => None,
            },
            sse_receiver: None,
            config,
            // auth_config removed - External MCP uses local processes
        }
    }

    /// Connect to the MCP server
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to MCP server '{}' at {} using {:?} protocol", self.name, self.endpoint, self.protocol);

        // Update connection state
        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Connecting;
        }

        match self.protocol {
            Protocol::WebSocket => self.connect_websocket().await,
            Protocol::SSE => self.connect_sse().await,
        }
    }

    /// Connect using WebSocket protocol
    async fn connect_websocket(&mut self) -> Result<()> {

        // Prepare WebSocket request with authentication headers
        let url = url::Url::parse(&self.endpoint)
            .map_err(|e| ProxyError::connection(format!("Invalid WebSocket URL '{}': {}", self.endpoint, e)))?;

        let mut request = url.into_client_request()
            .map_err(|e| ProxyError::connection(format!("Failed to create WebSocket request: {}", e)))?;

        // Authentication removed - External MCP uses local processes, no auth needed
        /*
        if let Some(ref auth_config) = self.auth_config {
            let headers = request.headers_mut();
            info!("Processing authentication for MCP server '{}' with type: '{}'", self.name, auth_config.auth_type);

            match auth_config.auth_type.as_str() {
                "Bearer" => {
                    if let Some(ref token) = auth_config.token {
                        headers.insert("Authorization", format!("Bearer {}", token).parse()
                            .map_err(|e| ProxyError::connection(format!("Invalid Bearer token: {}", e)))?);
                        info!("Added Bearer authentication header for MCP server '{}'", self.name);
                    } else {
                        warn!("Bearer authentication configured but no token provided for MCP server '{}'", self.name);
                    }
                }
                "Basic" => {
                    if let (Some(ref username), Some(ref password)) = (&auth_config.username, &auth_config.password) {
                        let credentials = general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                        headers.insert("Authorization", format!("Basic {}", credentials).parse()
                            .map_err(|e| ProxyError::connection(format!("Invalid Basic credentials: {}", e)))?);
                        debug!("Added Basic authentication header for MCP server '{}'", self.name);
                    }
                }
                "ApiKey" => {
                    if let Some(ref token) = auth_config.token {
                        headers.insert("X-API-Key", token.parse()
                            .map_err(|e| ProxyError::connection(format!("Invalid API key: {}", e)))?);
                        debug!("Added API Key authentication header for MCP server '{}'", self.name);
                    }
                }
                "none" | _ => {
                    info!("No authentication configured for MCP server '{}' (type: '{}')", self.name, auth_config.auth_type);
                }
            }

            // Add any custom headers
            if let Some(ref custom_headers) = auth_config.custom_headers {
                for (header_name, header_value) in custom_headers.iter() {
                    // Convert header name to HeaderName
                    let header_name_parsed = HeaderName::from_bytes(header_name.as_bytes())
                        .map_err(|e| ProxyError::connection(format!("Invalid header name '{}': {}", header_name, e)))?;
                    headers.insert(header_name_parsed, header_value.parse()
                        .map_err(|e| ProxyError::connection(format!("Invalid custom header value for '{}': {}", header_name, e)))?);
                    info!("Added custom header '{}' for MCP server '{}'", header_name, self.name);
                }
            }
        } else {
            info!("No authentication configuration found for MCP server '{}'", self.name);
        }
        */

        // Attempt WebSocket connection with authentication
        let connect_result = tokio::time::timeout(
            Duration::from_secs(self.config.connect_timeout_secs),
            connect_async(request)
        ).await;

        match connect_result {
            Ok(Ok((ws_stream, _))) => {
                info!("Successfully connected to MCP server '{}'", self.name);
                
                // Update connection state
                {
                    let mut state = self.connection_state.write().await;
                    *state = ConnectionState::Connected { connected_at: Instant::now() };
                }

                // Split the WebSocket stream
                let (mut ws_sender, mut ws_receiver) = ws_stream.split();
                
                // Create message channel
                let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
                self.message_sender = Some(tx);

                // Clone necessary data for the tasks
                let pending_requests: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<McpResponse>>>> = Arc::clone(&self.pending_requests);
                let connection_state: Arc<RwLock<ConnectionState>> = Arc::clone(&self.connection_state);
                let client_name_out = self.name.clone();
                let client_name_in = self.name.clone();

                // Spawn task to handle outgoing messages
                tokio::spawn(async move {
                    while let Some(message) = rx.recv().await {
                        if let Err(e) = ws_sender.send(message).await {
                            error!("Failed to send WebSocket message for client '{}': {}", client_name_out, e);
                            break;
                        }
                    }
                });

                // Spawn task to handle incoming messages
                tokio::spawn(async move {
                    while let Some(msg) = ws_receiver.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                debug!("Received message from MCP server '{}': {}", client_name_in, text);

                                // Parse MCP response
                                match serde_json::from_str::<McpResponse>(&text) {
                                    Ok(response) => {
                                        // Find and notify pending request
                                        let mut pending = pending_requests.write().await;
                                        if let Some(sender) = pending.remove(&response.id) {
                                            if sender.send(response).is_err() {
                                                warn!("Failed to send response to pending request");
                                            }
                                        } else {
                                            warn!("Received response for unknown request ID: {}", response.id);
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to parse MCP response from '{}': {}", client_name_in, e);
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => {
                                info!("WebSocket connection closed for client '{}'", client_name_in);
                                break;
                            }
                            Err(e) => {
                                error!("WebSocket error for client '{}': {}", client_name_in, e);
                                break;
                            }
                            _ => {}
                        }
                    }

                    // Update connection state when connection is lost
                    let mut state = connection_state.write().await;
                    *state = ConnectionState::Disconnected;
                });

                Ok(())
            }
            Ok(Err(e)) => {
                error!("Failed to connect to MCP server '{}': {}", self.name, e);
                let mut state = self.connection_state.write().await;
                *state = ConnectionState::Failed { 
                    error: e.to_string(), 
                    failed_at: Instant::now() 
                };
                Err(ProxyError::connection(format!("Failed to connect to MCP server '{}': {}", self.name, e)))
            }
            Err(_) => {
                error!("Connection timeout for MCP server '{}'", self.name);
                let mut state = self.connection_state.write().await;
                *state = ConnectionState::Failed { 
                    error: "Connection timeout".to_string(), 
                    failed_at: Instant::now() 
                };
                Err(ProxyError::connection(format!("Connection timeout for MCP server '{}'", self.name)))
            }
        }
    }

    /// Connect using SSE protocol (following mcp-remote pattern)
    async fn connect_sse(&mut self) -> Result<()> {
        // Convert WebSocket URL to HTTP URL for SSE
        let mut sse_url = self.endpoint.clone();
        if sse_url.starts_with("wss://") {
            sse_url = sse_url.replace("wss://", "https://");
        } else if sse_url.starts_with("ws://") {
            sse_url = sse_url.replace("ws://", "http://");
        }

        info!("Connecting to SSE endpoint: {}", sse_url);

        // Build HTTP client with authentication headers
        let mut client_builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.connect_timeout_secs));

        // Create default headers for authentication
        let mut headers = reqwest::header::HeaderMap::new();

        // Add essential SSE headers (matching mcp-remote)
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("text/event-stream")
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("mcp-remote/0.0.1")
        );

        // Authentication removed - External MCP uses local processes, no auth needed
        /*
        if let Some(ref auth_config) = self.auth_config {
            info!("Processing authentication for SSE MCP server '{}' with type: '{}'", self.name, auth_config.auth_type);

            match auth_config.auth_type.as_str() {
                "Bearer" => {
                    if let Some(ref token) = auth_config.token {
                        headers.insert(
                            reqwest::header::AUTHORIZATION,
                            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
                                .map_err(|e| ProxyError::connection(format!("Invalid Bearer token: {}", e)))?
                        );
                        info!("Added Bearer authentication header for SSE MCP server '{}'", self.name);
                    } else {
                        warn!("Bearer authentication configured but no token provided for SSE MCP server '{}'", self.name);
                    }
                }
                "Basic" => {
                    if let (Some(ref username), Some(ref password)) = (&auth_config.username, &auth_config.password) {
                        let credentials = general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                        headers.insert(
                            reqwest::header::AUTHORIZATION,
                            reqwest::header::HeaderValue::from_str(&format!("Basic {}", credentials))
                                .map_err(|e| ProxyError::connection(format!("Invalid Basic credentials: {}", e)))?
                        );
                        debug!("Added Basic authentication header for SSE MCP server '{}'", self.name);
                    }
                }
                "ApiKey" => {
                    if let Some(ref token) = auth_config.token {
                        headers.insert(
                            reqwest::header::HeaderName::from_static("x-api-key"),
                            reqwest::header::HeaderValue::from_str(token)
                                .map_err(|e| ProxyError::connection(format!("Invalid API key: {}", e)))?
                        );
                        debug!("Added API Key authentication header for SSE MCP server '{}'", self.name);
                    }
                }
                "none" | _ => {
                    info!("No authentication configured for SSE MCP server '{}' (type: '{}')", self.name, auth_config.auth_type);
                }
            }

            // Add any custom headers
            if let Some(ref custom_headers) = auth_config.custom_headers {
                for (header_name, header_value) in custom_headers.iter() {
                    let header_name_parsed = reqwest::header::HeaderName::from_bytes(header_name.as_bytes())
                        .map_err(|e| ProxyError::connection(format!("Invalid header name '{}': {}", header_name, e)))?;
                    let header_value_parsed = reqwest::header::HeaderValue::from_str(header_value)
                        .map_err(|e| ProxyError::connection(format!("Invalid custom header value for '{}': {}", header_name, e)))?;
                    headers.insert(header_name_parsed, header_value_parsed);
                    info!("Added custom header '{}' for SSE MCP server '{}'", header_name, self.name);
                }
            }
        } else {
            info!("No authentication configuration found for SSE MCP server '{}'", self.name);
        }
        */

        // Build client with default headers
        let client = client_builder
            .build()
            .map_err(|e| ProxyError::connection(format!("Failed to build HTTP client: {}", e)))?;

        // Store the HTTP client
        self.http_client = Some(client.clone());

        // Start SSE stream connection (GET request)
        info!("Opening SSE stream connection to {}", sse_url);
        let response = client
            .get(&sse_url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("Failed to connect to SSE stream: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            return Err(ProxyError::connection(format!("SSE stream connection failed with status {}: {}", status, error_body)));
        }

        info!("SSE stream connected successfully");

        // Create channel for SSE events
        let (sse_tx, sse_rx) = mpsc::unbounded_channel();
        self.sse_receiver = Some(sse_rx);

        // Spawn task to handle SSE stream
        let client_name = self.name.clone();
        let pending_requests = self.pending_requests.clone();
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&chunk_str);

                        // Process complete SSE events
                        while let Some(event_end) = buffer.find("\n\n") {
                            let event_data = buffer[..event_end].to_string();
                            buffer = buffer[event_end + 2..].to_string();

                            // Parse SSE event
                            if let Some(data_line) = event_data.lines().find(|line| line.starts_with("data: ")) {
                                let json_data = &data_line[6..]; // Remove "data: " prefix
                                if let Ok(response) = serde_json::from_str::<McpResponse>(json_data) {
                                    // Match response to pending request
                                    let id_str = response.id.clone();
                                    let mut pending = pending_requests.write().await;
                                    if let Some(sender) = pending.remove(&id_str) {
                                        let _ = sender.send(response);
                                    }
                                } else {
                                    debug!("Failed to parse SSE event data: {}", json_data);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("SSE stream error for '{}': {}", client_name, e);
                        break;
                    }
                }
            }

            info!("SSE stream ended for '{}'", client_name);
        });

        // Update connection state
        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Connected { connected_at: Instant::now() };
        }

        info!("Successfully connected to SSE MCP server '{}'", self.name);
        Ok(())
    }

    /// Disconnect from the MCP server
    pub async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from MCP server '{}'", self.name);
        
        // Clear message sender to close the connection
        self.message_sender = None;
        
        // Update connection state
        let mut state = self.connection_state.write().await;
        *state = ConnectionState::Disconnected;
        
        Ok(())
    }

    /// Check if client is connected
    pub async fn is_connected(&self) -> bool {
        matches!(*self.connection_state.read().await, ConnectionState::Connected { .. })
    }

    /// Get connection state
    pub async fn get_connection_state(&self) -> ConnectionState {
        self.connection_state.read().await.clone()
    }

    /// Send a request to the MCP server and wait for response
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<McpResponse> {
        // Check if connected
        if !self.is_connected().await {
            return Err(ProxyError::connection(format!("Not connected to MCP server '{}'", self.name)));
        }

        match self.protocol {
            Protocol::WebSocket => self.send_websocket_request(method, params).await,
            Protocol::SSE => self.send_sse_request(method, params).await,
        }
    }

    /// Send a request via WebSocket
    async fn send_websocket_request(&self, method: &str, params: Option<Value>) -> Result<McpResponse> {
        // Generate request ID
        let request_id = Uuid::new_v4().to_string();

        // Create request
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::String(request_id.clone())),
            method: method.to_string(),
            params,
        };

        // Create response channel
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.clone(), tx);
        }

        // Send request
        let message = Message::Text(serde_json::to_string(&request)?);
        if let Some(sender) = &self.message_sender {
            sender.send(message).map_err(|_| {
                ProxyError::connection(format!("Failed to send message to MCP server '{}'", self.name))
            })?;
        } else {
            return Err(ProxyError::connection(format!("No message sender for MCP server '{}'", self.name)));
        }

        // Wait for response with timeout
        let response = tokio::time::timeout(
            Duration::from_secs(self.config.request_timeout_secs),
            rx
        ).await;

        match response {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                // Remove from pending requests
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err(ProxyError::connection(format!("Response channel closed for MCP server '{}'", self.name)))
            }
            Err(_) => {
                // Remove from pending requests
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err(ProxyError::connection(format!("Request timeout for MCP server '{}'", self.name)))
            }
        }
    }

    /// Send a request via SSE (HTTP POST + wait for SSE response)
    async fn send_sse_request(&self, method: &str, params: Option<Value>) -> Result<McpResponse> {
        // Generate request ID
        let request_id = Uuid::new_v4().to_string();

        // Create request
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::String(request_id.clone())),
            method: method.to_string(),
            params,
        };

        // Create response channel
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.clone(), tx);
        }

        // Get HTTP client
        let client = self.http_client.as_ref()
            .ok_or_else(|| ProxyError::connection(format!("No HTTP client for SSE MCP server '{}'", self.name)))?;

        // Convert WebSocket URL to HTTP URL for POST requests
        let mut post_url = self.endpoint.clone();
        if post_url.starts_with("wss://") {
            post_url = post_url.replace("wss://", "https://");
        } else if post_url.starts_with("ws://") {
            post_url = post_url.replace("ws://", "http://");
        }

        // Build headers for POST request (matching mcp-remote)
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json")
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("mcp-remote/0.0.1")
        );

        // Authentication removed - External MCP uses local processes, no auth needed
        /*
        if let Some(ref auth_config) = self.auth_config {
            if let Some(ref custom_headers) = auth_config.custom_headers {
                for (header_name, header_value) in custom_headers.iter() {
                    if let (Ok(name), Ok(value)) = (
                        reqwest::header::HeaderName::from_bytes(header_name.as_bytes()),
                        reqwest::header::HeaderValue::from_str(header_value)
                    ) {
                        headers.insert(name, value);
                    }
                }
            }
        }
        */

        // Send HTTP POST request (don't expect direct response)
        debug!("Sending SSE request to {}: {}", post_url, serde_json::to_string(&request).unwrap_or_default());
        let post_result = client
            .post(&post_url)
            .headers(headers)
            .json(&request)
            .send()
            .await;

        match post_result {
            Ok(http_response) => {
                let status = http_response.status();
                if !status.is_success() {
                    let error_body = http_response.text().await
                        .unwrap_or_else(|_| "Unable to read error response".to_string());
                    error!("SSE POST request failed with status {}: {}", status, error_body);

                    // Remove from pending requests
                    let mut pending = self.pending_requests.write().await;
                    pending.remove(&request_id);

                    return Err(ProxyError::connection(format!("SSE POST failed with status: {} - {}", status, error_body)));
                }
                debug!("SSE POST request sent successfully");
            }
            Err(e) => {
                // Remove from pending requests
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);

                return Err(ProxyError::connection(format!("SSE POST request failed: {}", e)));
            }
        }

        // Wait for response from SSE stream with timeout
        let response = tokio::time::timeout(
            Duration::from_secs(self.config.request_timeout_secs),
            rx
        ).await;

        match response {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                // Remove from pending requests
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err(ProxyError::connection(format!("SSE response channel closed for MCP server '{}'", self.name)))
            }
            Err(_) => {
                // Remove from pending requests
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err(ProxyError::connection(format!("SSE request timeout for MCP server '{}'", self.name)))
            }
        }
    }

    /// Send a notification to the MCP server (no response expected)
    async fn send_notification(&self, method: &str, params: Option<Value>) -> Result<()> {
        // Check if connected
        if !self.is_connected().await {
            return Err(ProxyError::connection(format!("Not connected to MCP server '{}'", self.name)));
        }

        // Create notification (no ID for notifications)
        let notification = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: method.to_string(),
            params,
        };

        // Send notification
        let message = Message::Text(serde_json::to_string(&notification)?);
        if let Some(sender) = &self.message_sender {
            sender.send(message).map_err(|_| {
                ProxyError::connection(format!("Failed to send notification to MCP server '{}'", self.name))
            })?;
        } else {
            return Err(ProxyError::connection(format!("No message sender for MCP server '{}'", self.name)));
        }

        Ok(())
    }

    /// Initialize connection with MCP server (handshake)
    pub async fn initialize(&self) -> Result<McpServerCapabilities> {
        debug!("Initializing MCP server '{}'", self.name);

        let params = json!({
            "protocolVersion": self.config.protocol_version,
            "capabilities": {
                "roots": {
                    "listChanged": true
                },
                "sampling": {}
            },
            "clientInfo": {
                "name": self.config.client_name,
                "version": self.config.client_version
            }
        });

        let response = self.send_request("initialize", Some(params)).await?;

        if let Some(error) = response.error {
            return Err(ProxyError::mcp(format!("MCP server '{}' returned error during initialize: {}", self.name, error.message)));
        }

        let result = response.result.ok_or_else(|| {
            ProxyError::mcp(format!("MCP server '{}' returned no result for initialize", self.name))
        })?;

        let capabilities: McpServerCapabilities = serde_json::from_value(result)?;

        // Send initialized notification as required by MCP protocol
        self.send_notification("notifications/initialized", None).await?;

        Ok(capabilities)
    }

    /// List available tools from the MCP server
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        debug!("Listing tools from MCP server '{}'", self.name);

        let response = self.send_request("tools/list", Some(json!({}))).await?;

        if let Some(error) = response.error {
            return Err(ProxyError::mcp(format!("MCP server '{}' returned error: {}", self.name, error.message)));
        }

        let result = response.result.ok_or_else(|| {
            ProxyError::mcp(format!("MCP server '{}' returned no result for tools/list", self.name))
        })?;

        let tool_list: ToolListResponse = serde_json::from_value(result)?;
        Ok(tool_list.tools)
    }

    /// List available resources from the MCP server
    pub async fn list_resources(&self) -> Result<Vec<McpResource>> {
        debug!("Listing resources from MCP server '{}'", self.name);

        let response = self.send_request("resources/list", Some(json!({}))).await?;

        if let Some(error) = response.error {
            return Err(ProxyError::mcp(format!("MCP server '{}' returned error: {}", self.name, error.message)));
        }

        let result = response.result.ok_or_else(|| {
            ProxyError::mcp(format!("MCP server '{}' returned no result for resources/list", self.name))
        })?;

        let resources: Vec<McpResource> = serde_json::from_value(result.get("resources").unwrap_or(&json!([])).clone())?;
        Ok(resources)
    }

    /// List available prompts from the MCP server
    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>> {
        debug!("Listing prompts from MCP server '{}'", self.name);

        let response = self.send_request("prompts/list", None).await?;

        if let Some(error) = response.error {
            return Err(ProxyError::mcp(format!("MCP server '{}' returned error: {}", self.name, error.message)));
        }

        let result = response.result.ok_or_else(|| {
            ProxyError::mcp(format!("MCP server '{}' returned no result for prompts/list", self.name))
        })?;

        let prompts: Vec<McpPrompt> = serde_json::from_value(result.get("prompts").unwrap_or(&json!([])).clone())?;
        Ok(prompts)
    }

    /// Call a tool on the MCP server
    pub async fn call_tool(&self, tool_call: &ToolCall) -> Result<ToolResult> {
        debug!("Calling tool '{}' on MCP server '{}'", tool_call.name, self.name);
        
        let params = json!({
            "name": tool_call.name,
            "arguments": tool_call.arguments
        });

        let response = self.send_request("tools/call", Some(params)).await?;
        
        if let Some(error) = response.error {
            return Err(ProxyError::mcp(format!("MCP server '{}' returned error for tool '{}': {}", 
                self.name, tool_call.name, error.message)));
        }

        let result = response.result.ok_or_else(|| {
            ProxyError::mcp(format!("MCP server '{}' returned no result for tool '{}'", self.name, tool_call.name))
        })?;

        let tool_result: ToolResult = serde_json::from_value(result)?;
        Ok(tool_result)
    }
}
