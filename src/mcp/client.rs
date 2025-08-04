//! MCP Client for connecting to external MCP servers

use crate::error::{ProxyError, Result};
use crate::mcp::types::{
    Tool, ToolCall, ToolResult, McpRequest, McpResponse, ToolListResponse
};
use crate::mcp::errors::McpError;
use crate::mcp::types::sampling::{SamplingRequest, SamplingResponse, SamplingMessage, SamplingMessageRole, SamplingContent, SamplingContentPart, SamplingStopReason, SamplingUsage};
use crate::mcp::types::elicitation::{ElicitationRequest, ElicitationResponse, ElicitationAction};
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
use chrono;
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
    /// MCP 2025-06-18: Sampling capabilities
    pub sampling: Option<McpSamplingCapabilities>,
    /// MCP 2025-06-18: Elicitation capabilities  
    pub elicitation: Option<McpElicitationCapabilities>,
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

/// MCP 2025-06-18: Sampling capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSamplingCapabilities {
    /// Supported sampling methods
    pub methods: Option<Vec<String>>,
    /// Maximum number of messages per sampling request
    pub max_messages: Option<usize>,
    /// Supported message types
    pub message_types: Option<Vec<String>>,
    /// Sampling capability metadata
    pub metadata: Option<Value>,
}

/// MCP 2025-06-18: Elicitation capabilities  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpElicitationCapabilities {
    /// Supported elicitation methods
    pub methods: Option<Vec<String>>,
    /// Maximum parameter schema complexity
    pub max_schema_depth: Option<usize>,
    /// Supported validation types
    pub validation_types: Option<Vec<String>>,
    /// Elicitation capability metadata
    pub metadata: Option<Value>,
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
    /// Server configuration for accessing LLM settings for MagicTunnel-handled requests
    server_config: Option<Arc<crate::config::Config>>,
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

/// Hybrid processing configuration for MCP sampling and elicitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridProcessingConfig {
    /// Processing strategy for sampling requests
    #[serde(default)]
    pub sampling_strategy: ProcessingStrategy,
    /// Processing strategy for elicitation requests
    #[serde(default)]
    pub elicitation_strategy: ProcessingStrategy,
    /// Enable local processing fallback when proxy fails
    #[serde(default = "default_true")]
    pub enable_local_fallback: bool,
    /// Enable forwarding to external MCP servers
    #[serde(default = "default_true")]
    pub enable_proxy_forwarding: bool,
    /// Timeout for proxy operations in seconds
    #[serde(default = "default_proxy_timeout")]
    pub proxy_timeout_secs: u64,
    /// Maximum external MCP server attempts
    #[serde(default = "default_max_attempts")]
    pub max_external_attempts: u32,
    /// Enable enhanced metadata collection
    #[serde(default = "default_true")]
    pub enable_enhanced_metadata: bool,
}

/// Processing strategy for hybrid MCP operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingStrategy {
    /// Always use MagicTunnel's configured LLMs
    MagictunnelHandled,
    /// Forward to original client (Claude Desktop, etc.)
    ClientForwarded,
    /// Try MagicTunnel first, then client fallback
    MagictunnelFirst,
    /// Try client first, then MagicTunnel fallback
    ClientFirst,
    /// Run both MagicTunnel and client in parallel, return first successful response
    Parallel,
    /// Run both MagicTunnel and client, combine responses intelligently
    Hybrid,
}

impl Default for ProcessingStrategy {
    fn default() -> Self {
        ProcessingStrategy::ClientForwarded
    }
}

impl Default for HybridProcessingConfig {
    fn default() -> Self {
        Self {
            sampling_strategy: ProcessingStrategy::ClientFirst,
            elicitation_strategy: ProcessingStrategy::ClientFirst,
            enable_local_fallback: true,
            enable_proxy_forwarding: true,
            proxy_timeout_secs: 30,
            max_external_attempts: 5,
            enable_enhanced_metadata: true,
        }
    }
}

// Helper functions for serde defaults
fn default_true() -> bool { true }
fn default_proxy_timeout() -> u64 { 30 }
fn default_max_attempts() -> u32 { 5 }

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
    /// Hybrid processing configuration
    #[serde(default)]
    pub hybrid_processing: HybridProcessingConfig,
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
            hybrid_processing: HybridProcessingConfig::default(),
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
            hybrid_processing: HybridProcessingConfig::default(),
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
            server_config: None,
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
            server_config: None,
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
            server_config: None,
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
            server_config: None,
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
            server_config: None,
            // auth_config removed - External MCP uses local processes
        }
    }
    
    /// Set server configuration for accessing LLM settings
    pub fn set_server_config(&mut self, server_config: Arc<crate::config::Config>) {
        self.server_config = Some(server_config);
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
                let message_sender = Arc::new(tx.clone()); // Clone for incoming request handling before moving
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

                                // First try to parse as incoming request (MCP servers can send requests to clients)
                                if let Ok(request) = serde_json::from_str::<McpRequest>(&text) {
                                    // Handle incoming request from MCP server
                                    debug!("Received incoming request '{}' from MCP server '{}'", request.method, client_name_in);
                                    
                                    // TODO: Process incoming request from MCP server
                                    // For now, just log that we received it since handling requires self
                                    warn!("Received incoming request '{}' from MCP server '{}' - processing not implemented in spawned task", request.method, client_name_in);
                                } else if let Ok(response) = serde_json::from_str::<McpResponse>(&text) {
                                    // Handle response to our previous request
                                    let mut pending = pending_requests.write().await;
                                    if let Some(sender) = pending.remove(&response.id) {
                                        if sender.send(response).is_err() {
                                            warn!("Failed to send response to pending request");
                                        }
                                    } else {
                                        warn!("Received response for unknown request ID: {}", response.id);
                                    }
                                } else {
                                    error!("Failed to parse message from '{}' as either McpRequest or McpResponse: {}", client_name_in, text);
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
                                
                                // First try to parse as incoming request (MCP servers can send requests to clients)
                                if let Ok(request) = serde_json::from_str::<McpRequest>(json_data) {
                                    // Handle incoming request from MCP server
                                    debug!("Received incoming request '{}' via SSE from MCP server '{}'", request.method, client_name);
                                    
                                    // For SSE, we can't send the response back directly through the stream
                                    // We need to make a separate HTTP POST request to send the response
                                    // For now, we'll log this as not yet fully implemented
                                    warn!("Incoming SSE request from '{}' received but SSE response mechanism not yet implemented", client_name);
                                    
                                    // TODO: Implement SSE response mechanism in Phase C/D
                                    // This would require making a separate HTTP POST request to send the response back
                                    
                                } else if let Ok(response) = serde_json::from_str::<McpResponse>(json_data) {
                                    // Handle response to our previous request
                                    let id_str = response.id.clone();
                                    let mut pending = pending_requests.write().await;
                                    if let Some(sender) = pending.remove(&id_str) {
                                        let _ = sender.send(response);
                                    }
                                } else {
                                    debug!("Failed to parse SSE event data as either McpRequest or McpResponse: {}", json_data);
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

    /// Send a sampling request to external MCP server (MCP 2025-06-18)
    /// 
    /// This method sends a `sampling/createMessage` request to an external MCP server
    /// that supports sampling capabilities. This is used when MagicTunnel (as server)
    /// needs LLM assistance from an external client.
    pub async fn send_sampling_request(&self, request: SamplingRequest) -> Result<SamplingResponse> {
        debug!("Sending sampling request to MCP server '{}'", self.name);
        
        let params = serde_json::to_value(&request).map_err(|e| {
            ProxyError::mcp(format!("Failed to serialize sampling request: {}", e))
        })?;

        let response = self.send_request("sampling/createMessage", Some(params)).await?;
        
        if let Some(error) = response.error {
            return Err(ProxyError::mcp(format!("MCP server '{}' returned error for sampling: {}", 
                self.name, error.message)));
        }

        let result = response.result.ok_or_else(|| {
            ProxyError::mcp(format!("MCP server '{}' returned no result for sampling request", self.name))
        })?;

        let sampling_response: SamplingResponse = serde_json::from_value(result).map_err(|e| {
            ProxyError::mcp(format!("Failed to deserialize sampling response from '{}': {}", self.name, e))
        })?;
        
        debug!("Successfully received sampling response from MCP server '{}'", self.name);
        Ok(sampling_response)
    }

    /// Send an elicitation request to external MCP server (MCP 2025-06-18)
    /// 
    /// This method sends an `elicitation/create` request to an external MCP server
    /// that supports elicitation capabilities. This is used when MagicTunnel needs
    /// parameter validation or user input from an external client.
    pub async fn send_elicitation_request(&self, request: ElicitationRequest) -> Result<ElicitationResponse> {
        debug!("Sending elicitation request to MCP server '{}'", self.name);
        
        let params = serde_json::to_value(&request).map_err(|e| {
            ProxyError::mcp(format!("Failed to serialize elicitation request: {}", e))
        })?;

        let response = self.send_request("elicitation/create", Some(params)).await?;
        
        if let Some(error) = response.error {
            return Err(ProxyError::mcp(format!("MCP server '{}' returned error for elicitation: {}", 
                self.name, error.message)));
        }

        let result = response.result.ok_or_else(|| {
            ProxyError::mcp(format!("MCP server '{}' returned no result for elicitation request", self.name))
        })?;

        let elicitation_response: ElicitationResponse = serde_json::from_value(result).map_err(|e| {
            ProxyError::mcp(format!("Failed to deserialize elicitation response from '{}': {}", self.name, e))
        })?;
        
        debug!("Successfully received elicitation response from MCP server '{}'", self.name);
        Ok(elicitation_response)
    }

    /// Handle incoming requests from MCP servers (MCP 2025-06-18)
    /// 
    /// This method processes incoming `sampling/createMessage` and `elicitation/create` 
    /// requests from external MCP servers. This enables bidirectional communication
    /// where external servers can request LLM assistance or parameter validation from MagicTunnel.
    async fn handle_incoming_request(
        &self,
        request: McpRequest,
        server_name: String,
        message_sender: Arc<tokio::sync::mpsc::UnboundedSender<Message>>,
    ) -> Result<()> {
        debug!("Processing incoming '{}' request from MCP server '{}'", request.method, server_name);

        let response = match request.method.as_str() {
            "sampling/createMessage" => {
                self.handle_incoming_sampling_request(&request, &server_name).await
            }
            "elicitation/create" => {
                self.handle_incoming_elicitation_request(&request, &server_name).await
            }
            _ => {
                warn!("Unsupported incoming request method '{}' from MCP server '{}'", request.method, server_name);
                // Return method not found error
                Ok(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    result: None,
                    error: Some(McpError {
                        code: -32601, // Method not found
                        message: format!("Method '{}' not supported", request.method),
                        data: None,
                    }),
                })
            }
        };

        // Send response back to the MCP server
        match response {
            Ok(mcp_response) => {
                let response_text = serde_json::to_string(&mcp_response)?;
                let message = Message::Text(response_text);
                
                if message_sender.send(message).is_err() {
                    error!("Failed to send response back to MCP server '{}'", server_name);
                    return Err(ProxyError::connection(format!("Failed to send response to MCP server '{}'", server_name)));
                }
                
                debug!("Successfully sent response for '{}' request to MCP server '{}'", request.method, server_name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to process incoming '{}' request from MCP server '{}': {}", request.method, server_name, e);
                
                // Send error response
                let error_response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    result: None,
                    error: Some(McpError {
                        code: -32603, // Internal error
                        message: format!("Internal error processing {}: {}", request.method, e),
                        data: None,
                    }),
                };
                
                let response_text = serde_json::to_string(&error_response)?;
                let message = Message::Text(response_text);
                
                if message_sender.send(message).is_err() {
                    error!("Failed to send error response back to MCP server '{}'", server_name);
                }
                
                Err(e)
            }
        }
    }

    /// Handle incoming sampling request from MCP server
    async fn handle_incoming_sampling_request(
        &self,
        request: &McpRequest,
        server_name: &str,
    ) -> Result<McpResponse> {
        debug!("Processing sampling request from MCP server '{}'", server_name);

        // Parse the sampling request parameters
        let params = request.params.as_ref().ok_or_else(|| {
            ProxyError::mcp("Missing parameters for sampling request".to_string())
        })?;

        let sampling_request: SamplingRequest = serde_json::from_value(params.clone()).map_err(|e| {
            ProxyError::mcp(format!("Invalid sampling request parameters: {}", e))
        })?;

        // Phase E: Use hybrid processing system with configurable strategies
        info!("🎯 Processing sampling request with hybrid strategy '{:?}' from '{}'", self.config.hybrid_processing.sampling_strategy, server_name);
        
        match self.process_sampling_with_hybrid_strategy(&sampling_request, server_name).await {
            Ok(sampling_response) => {
                debug!("Successfully processed sampling request with hybrid strategy for '{}'", server_name);
                Ok(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    result: Some(serde_json::to_value(sampling_response)?),
                    error: None,
                })
            }
            Err(e) => {
                error!("Failed to process sampling request locally for '{}': {}", server_name, e);
                
                // Return appropriate error response
                Ok(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    result: None,
                    error: Some(McpError {
                        code: -32603, // Internal error
                        message: format!("Local sampling processing failed: {}", e),
                        data: Some(json!({"server": server_name, "error": e.to_string()})),
                    }),
                })
            }
        }
    }

    /// Handle incoming elicitation request from MCP server
    async fn handle_incoming_elicitation_request(
        &self,
        request: &McpRequest,
        server_name: &str,
    ) -> Result<McpResponse> {
        debug!("Processing elicitation request from MCP server '{}'", server_name);

        // Parse the elicitation request parameters
        let params = request.params.as_ref().ok_or_else(|| {
            ProxyError::mcp("Missing parameters for elicitation request".to_string())
        })?;

        let elicitation_request: ElicitationRequest = serde_json::from_value(params.clone()).map_err(|e| {
            ProxyError::mcp(format!("Invalid elicitation request parameters: {}", e))
        })?;

        // Phase E: Use hybrid processing system with configurable strategies
        info!("🎯 Processing elicitation request with hybrid strategy '{:?}' from '{}'", self.config.hybrid_processing.elicitation_strategy, server_name);
        
        match self.process_elicitation_with_hybrid_strategy(&elicitation_request, server_name).await {
            Ok(elicitation_response) => {
                debug!("Successfully processed elicitation request with hybrid strategy for '{}'", server_name);
                Ok(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    result: Some(serde_json::to_value(elicitation_response)?),
                    error: None,
                })
            }
            Err(e) => {
                error!("Failed to process elicitation request locally for '{}': {}", server_name, e);
                
                // Return appropriate error response
                Ok(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                    result: None,
                    error: Some(McpError {
                        code: -32603, // Internal error
                        message: format!("Local elicitation processing failed: {}", e),
                        data: Some(json!({"server": server_name, "error": e.to_string()})),
                    }),
                })
            }
        }
    }

    /// Process sampling request using MagicTunnel's configured LLM
    async fn handle_sampling_with_magictunnel_llm(&self, request: &SamplingRequest) -> Result<SamplingResponse> {
        debug!("Processing sampling request with MagicTunnel's configured LLM");
        
        // Get LLM configuration from server config
        let llm_config = self.get_llm_config().await?;
        
        // Create LLM client
        let llm_client = crate::mcp::llm_client::LlmClient::new(llm_config)
            .map_err(|e| ProxyError::config(&format!("Failed to create LLM client: {}", e)))?;
        
        // Process the request using the configured LLM
        llm_client.handle_sampling_request(request).await
            .map_err(|e| ProxyError::routing(&format!("LLM processing failed: {}", e)))
    }

    /// Process elicitation request using MagicTunnel's configured LLM
    async fn handle_elicitation_with_magictunnel_llm(&self, request: &ElicitationRequest) -> Result<ElicitationResponse> {
        debug!("Processing elicitation request with MagicTunnel's configured LLM");
        
        // Get LLM configuration from server config
        let llm_config = self.get_llm_config().await?;
        
        // Create LLM client
        let llm_client = crate::mcp::llm_client::LlmClient::new(llm_config)
            .map_err(|e| ProxyError::config(&format!("Failed to create LLM client: {}", e)))?;
        
        // Process the request using the configured LLM
        llm_client.handle_elicitation_request(request).await
            .map_err(|e| ProxyError::routing(&format!("LLM processing failed: {}", e)))
    }

    /// Get LLM configuration from server config
    async fn get_llm_config(&self) -> Result<crate::config::LlmConfig> {
        if let Some(server_config) = &self.server_config {
            // Check if the server has sampling configuration
            if let Some(sampling_config) = &server_config.sampling {
                if let Some(llm_config) = &sampling_config.llm_config {
                    debug!("Using server sampling LLM configuration");
                    return Ok(llm_config.clone());
                }
            }
            
            // Check if server has smart discovery with LLM configuration
            if let Some(smart_discovery) = &server_config.smart_discovery {
                if let Some(llm_provider) = &smart_discovery.llm_tool_selection.api_key {
                    debug!("Using server smart discovery LLM configuration");
                    return Ok(crate::config::LlmConfig {
                        provider: smart_discovery.llm_tool_selection.provider.clone(),
                        model: smart_discovery.llm_tool_selection.model.clone(),
                        api_key_env: smart_discovery.llm_tool_selection.api_key_env.clone(),
                        api_base_url: None, // smart_discovery doesn't have api_base_url
                        max_tokens: None, // smart_discovery doesn't have max_tokens
                        temperature: None, // smart_discovery doesn't have temperature
                        additional_params: None, // smart_discovery doesn't have additional_params
                    });
                }
            }
        }
        
        debug!("No server LLM configuration found, using default configuration");
        Ok(crate::config::LlmConfig::default())
    }

    /// Forward sampling/elicitation request to original client (Claude Desktop, etc.)
    async fn forward_to_original_client(&self, request: &SamplingRequest) -> Result<SamplingResponse> {
        debug!("Forwarding request to original client");
        
        // TODO: Implement client forwarding mechanism
        // This would need to communicate back to the original MCP client that initiated the session
        
        Err(ProxyError::routing("Client forwarding not yet implemented"))
    }

    /// Forward sampling request to external MCP servers that support sampling capability  
    /// This enables request forwarding to external MCP servers with sampling support
    async fn forward_sampling_to_external_servers(&self, request: &SamplingRequest, original_client_id: &str) -> Result<SamplingResponse> {
        debug!("Forwarding sampling request to external MCP servers for client: {}", original_client_id);
        
        // TODO: External MCP manager integration needs to be implemented at a higher level
        // For now, single MCP client cannot manage multiple external servers
        debug!("External MCP forwarding not implemented for single client");
        
        // If no external servers is available, fall back to MagicTunnel LLM processing
        debug!("No suitable external servers found, using MagicTunnel LLM processing for client: {}", original_client_id);
        let mut local_response = self.handle_sampling_with_magictunnel_llm(request).await?;
        
        // Add proxy metadata to indicate fallback processing
        if let Some(ref mut metadata) = local_response.metadata {
            metadata.insert("proxy_mode".to_string(), json!("fallback_local"));
            metadata.insert("original_client_id".to_string(), json!(original_client_id));
            metadata.insert("external_forwarding_attempted".to_string(), json!(true));
        }
        
        Ok(local_response)
    }

    /// Forward elicitation request to external MCP servers that support elicitation capability
    /// This enables request forwarding to external MCP servers with elicitation support  
    async fn forward_elicitation_to_external_servers(&self, request: &ElicitationRequest, original_client_id: &str) -> Result<ElicitationResponse> {
        debug!("Forwarding elicitation request to external MCP servers for client: {}", original_client_id);
        
        // TODO: External MCP manager integration needs to be implemented at a higher level
        // For now, single MCP client cannot manage multiple external servers  
        debug!("External MCP forwarding not implemented for single client");
        
        // If no external servers is available, fall back to MagicTunnel LLM processing
        debug!("No suitable external servers found, using MagicTunnel LLM processing for client: {}", original_client_id);
        let mut local_response = self.handle_elicitation_with_magictunnel_llm(request).await?;
        
        // Add proxy metadata to indicate fallback processing
        if let Some(ref mut metadata) = local_response.metadata {
            metadata.insert("proxy_mode".to_string(), json!("fallback_local"));
            metadata.insert("original_client_id".to_string(), json!(original_client_id));
            metadata.insert("external_forwarding_attempted".to_string(), json!(true));
        }
        
        Ok(local_response)
    }

    /// Forward sampling request to specific external server with original client context
    async fn forward_sampling_to_external_server(&self, external_server: &str, request: &SamplingRequest, original_client_id: &str) -> Result<SamplingResponse> {
        debug!("Forwarding sampling request to external server: {} for original client: {}", external_server, original_client_id);
        
        // Create enhanced request with proxy metadata
        let mut enhanced_request = request.clone();
        
        // Add proxy context to metadata
        let proxy_metadata = json!({
            "forwarding_mode": "external_mcp_server",
            "original_client_id": original_client_id,
            "forwarding_server": self.name,
            "external_server": external_server,
            "proxy_timestamp": chrono::Utc::now().to_rfc3339()
        });
        
        if let Some(ref mut metadata) = enhanced_request.metadata {
            metadata.insert("magictunnel_proxy".to_string(), proxy_metadata);
        } else {
            enhanced_request.metadata = Some([
                ("magictunnel_proxy".to_string(), proxy_metadata)
            ].into_iter().collect());
        }
        
        // Send the enhanced request to the external server
        let response = self.send_sampling_request(enhanced_request).await?;
        
        // Add forwarding metadata to response
        let mut enhanced_response = response;
        if let Some(ref mut metadata) = enhanced_response.metadata {
            metadata.insert("forwarded_through_external_server".to_string(), json!(external_server));
            metadata.insert("original_client_id".to_string(), json!(original_client_id));
        }
        
        Ok(enhanced_response)
    }

    /// Forward elicitation request to specific external server with original client context
    async fn forward_elicitation_to_external_server(&self, external_server: &str, request: &ElicitationRequest, original_client_id: &str) -> Result<ElicitationResponse> {
        debug!("Forwarding elicitation request to external server: {} for original client: {}", external_server, original_client_id);
        
        // Create enhanced request with proxy metadata
        let mut enhanced_request = request.clone();
        
        // Add proxy context to metadata
        let proxy_metadata = json!({
            "forwarding_mode": "external_mcp_server",
            "original_client_id": original_client_id,
            "forwarding_server": self.name,
            "external_server": external_server,
            "proxy_timestamp": chrono::Utc::now().to_rfc3339()
        });
        
        if let Some(ref mut metadata) = enhanced_request.metadata {
            metadata.insert("magictunnel_proxy".to_string(), proxy_metadata);
        } else {
            enhanced_request.metadata = Some([
                ("magictunnel_proxy".to_string(), proxy_metadata)
            ].into_iter().collect());
        }
        
        // Send the enhanced request to the external server
        let response = self.send_elicitation_request(enhanced_request).await?;
        
        // Add forwarding metadata to response
        let mut enhanced_response = response;
        if let Some(ref mut metadata) = enhanced_response.metadata {
            metadata.insert("forwarded_through_external_server".to_string(), json!(external_server));
            metadata.insert("original_client_id".to_string(), json!(original_client_id));
        }
        
        Ok(enhanced_response)
    }

    /// Get available external MCP servers for sampling requests
    pub async fn get_sampling_external_servers(&self) -> Result<Vec<String>> {
        // TODO: External MCP server management should be handled at a higher level
        // Single MCP client doesn't manage multiple external servers
        debug!("External MCP server enumeration not implemented for single client");
        Ok(vec![])
    }

    /// Get available external MCP servers for elicitation requests  
    pub async fn get_elicitation_external_servers(&self) -> Result<Vec<String>> {
        // TODO: External MCP server management should be handled at a higher level
        // Single MCP client doesn't manage multiple external servers
        debug!("External MCP server enumeration not implemented for single client");
        Ok(vec![])
    }

    /// Process sampling request using configured hybrid strategy
    async fn process_sampling_with_hybrid_strategy(&self, request: &SamplingRequest, original_client_id: &str) -> Result<SamplingResponse> {
        let strategy = &self.config.hybrid_processing.sampling_strategy;
        debug!("Processing sampling request with hybrid strategy: {:?}", strategy);

        match strategy {
            ProcessingStrategy::MagictunnelHandled => {
                debug!("Using MagicTunnel LLM processing for sampling request");
                self.handle_sampling_with_magictunnel_llm(request).await
            }
            ProcessingStrategy::ClientForwarded => {
                debug!("Forwarding sampling request to original client");
                self.forward_to_original_client(request).await
            }
            ProcessingStrategy::MagictunnelFirst => {
                debug!("Using MagicTunnel-first processing for sampling request");
                match self.handle_sampling_with_magictunnel_llm(request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        debug!("MagicTunnel processing failed, trying client forwarding: {}", e);
                        let mut client_response = self.forward_to_original_client(request).await?;
                        self.enhance_response_metadata(&mut client_response, "magictunnel_first_fallback", Some(&e.to_string()));
                        Ok(client_response)
                    }
                }
            }
            ProcessingStrategy::ClientFirst => {
                debug!("Using client-first processing for sampling request");
                match self.forward_to_original_client(request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        debug!("Client forwarding failed, trying MagicTunnel processing: {}", e);
                        let mut magictunnel_response = self.handle_sampling_with_magictunnel_llm(request).await?;
                        self.enhance_response_metadata(&mut magictunnel_response, "client_first_fallback", Some(&e.to_string()));
                        Ok(magictunnel_response)
                    }
                }
            }
            ProcessingStrategy::Parallel => {
                debug!("Using parallel processing for sampling request");
                self.process_sampling_parallel(request, original_client_id).await
            }
            ProcessingStrategy::Hybrid => {
                debug!("Using hybrid combined processing for sampling request");
                self.process_sampling_hybrid_combined(request, original_client_id).await
            }
        }
    }

    /// Process elicitation request using configured hybrid strategy
    async fn process_elicitation_with_hybrid_strategy(&self, request: &ElicitationRequest, original_client_id: &str) -> Result<ElicitationResponse> {
        let strategy = &self.config.hybrid_processing.elicitation_strategy;
        debug!("Processing elicitation request with hybrid strategy: {:?}", strategy);

        match strategy {
            ProcessingStrategy::MagictunnelHandled => {
                debug!("Using MagicTunnel LLM processing for elicitation request");
                self.handle_elicitation_with_magictunnel_llm(request).await
            }
            ProcessingStrategy::ClientForwarded => {
                debug!("Forwarding elicitation request to original client");
                // TODO: Implement client forwarding for elicitation
                Err(ProxyError::routing("Elicitation client forwarding not yet implemented"))
            }
            ProcessingStrategy::MagictunnelFirst => {
                debug!("Using MagicTunnel-first processing for elicitation request");
                match self.handle_elicitation_with_magictunnel_llm(request).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        debug!("MagicTunnel processing failed, trying client forwarding: {}", e);
                        // TODO: Implement client forwarding fallback
                        Err(ProxyError::routing("Elicitation client forwarding fallback not yet implemented"))
                    }
                }
            }
            ProcessingStrategy::ClientFirst => {
                debug!("Using client-first processing for elicitation request");
                // TODO: Implement client forwarding with MagicTunnel fallback
                Err(ProxyError::routing("Elicitation client-first processing not yet implemented"))
            }
            ProcessingStrategy::Parallel => {
                debug!("Using parallel processing for elicitation request");
                self.process_elicitation_parallel(request, original_client_id).await
            }
            ProcessingStrategy::Hybrid => {
                debug!("Using hybrid combined processing for elicitation request");
                self.process_elicitation_hybrid_combined(request, original_client_id).await
            }
        }
    }

    /// Process sampling request in parallel (MagicTunnel and client simultaneously)
    async fn process_sampling_parallel(&self, request: &SamplingRequest, original_client_id: &str) -> Result<SamplingResponse> {
        debug!("Starting parallel processing for sampling request");
        
        let magictunnel_future = self.handle_sampling_with_magictunnel_llm(request);
        let client_future = self.forward_to_original_client(request);

        // Run both in parallel, return the first successful result
        tokio::select! {
            magictunnel_result = magictunnel_future => {
                match magictunnel_result {
                    Ok(mut response) => {
                        self.enhance_response_metadata(&mut response, "parallel_magictunnel_first", None);
                        debug!("Parallel processing: MagicTunnel completed first");
                        Ok(response)
                    }
                    Err(e) => {
                        debug!("Parallel processing: MagicTunnel failed, falling back to client: {}", e);
                        // Try client processing as fallback
                        let mut client_response = self.forward_to_original_client(request).await?;
                        self.enhance_response_metadata(&mut client_response, "parallel_client_after_magictunnel_fail", Some(&e.to_string()));
                        Ok(client_response)
                    }
                }
            }
            client_result = client_future => {
                match client_result {
                    Ok(mut response) => {
                        self.enhance_response_metadata(&mut response, "parallel_client_first", None);
                        debug!("Parallel processing: client completed first");
                        Ok(response)
                    }
                    Err(e) => {
                        debug!("Parallel processing: client failed, falling back to MagicTunnel: {}", e);
                        // Try MagicTunnel processing as fallback
                        let mut magictunnel_response = self.handle_sampling_with_magictunnel_llm(request).await?;
                        self.enhance_response_metadata(&mut magictunnel_response, "parallel_magictunnel_after_client_fail", Some(&e.to_string()));
                        Ok(magictunnel_response)
                    }
                }
            }
        }
    }

    /// Process elicitation request in parallel (local and proxy simultaneously)
    async fn process_elicitation_parallel(&self, request: &ElicitationRequest, original_client_id: &str) -> Result<ElicitationResponse> {
        debug!("Starting parallel processing for elicitation request");
        
        let local_future = self.handle_elicitation_with_magictunnel_llm(request);
        let proxy_future = self.forward_elicitation_to_external_servers(request, original_client_id);

        // Run both in parallel, return the first successful result
        tokio::select! {
            local_result = local_future => {
                match local_result {
                    Ok(mut response) => {
                        self.enhance_elicitation_response_metadata(&mut response, "parallel_local_first", None);
                        debug!("Parallel processing: local elicitation completed first");
                        Ok(response)
                    }
                    Err(e) => {
                        debug!("Parallel processing: local elicitation failed, falling back to proxy: {}", e);
                        // Try proxy processing as fallback - call method directly instead of awaiting moved future
                        let mut proxy_response = self.forward_elicitation_to_external_servers(request, original_client_id).await?;
                        self.enhance_elicitation_proxy_response_metadata(&mut proxy_response, "parallel_proxy_after_local_fail", Some(&e.to_string()));
                        Ok(proxy_response)
                    }
                }
            }
            proxy_result = proxy_future => {
                match proxy_result {
                    Ok(mut response) => {
                        self.enhance_elicitation_proxy_response_metadata(&mut response, "parallel_proxy_first", None);
                        debug!("Parallel processing: proxy elicitation completed first");
                        Ok(response)
                    }
                    Err(e) => {
                        debug!("Parallel processing: proxy elicitation failed, falling back to local: {}", e);
                        // Try local processing as fallback - call method directly instead of awaiting moved future
                        let mut local_response = self.handle_elicitation_with_magictunnel_llm(request).await?;
                        self.enhance_elicitation_response_metadata(&mut local_response, "parallel_local_after_proxy_fail", Some(&e.to_string()));
                        Ok(local_response)
                    }
                }
            }
        }
    }

    /// Process sampling request with hybrid combined approach (both local and proxy, combine results)
    async fn process_sampling_hybrid_combined(&self, request: &SamplingRequest, original_client_id: &str) -> Result<SamplingResponse> {
        debug!("Starting hybrid combined processing for sampling request");
        
        let local_future = self.handle_sampling_with_magictunnel_llm(request);
        let proxy_future = if self.config.hybrid_processing.enable_proxy_forwarding {
            Some(self.forward_sampling_to_external_servers(request, original_client_id))
        } else {
            None
        };

        if let Some(proxy_future) = proxy_future {
            // Run both and combine results intelligently
            let (local_result, proxy_result) = tokio::join!(local_future, proxy_future);
            
            match (local_result, proxy_result) {
                (Ok(local_response), Ok(proxy_response)) => {
                    debug!("Hybrid combined: both local and proxy succeeded, combining responses");
                    Ok(self.combine_sampling_responses(local_response, proxy_response, original_client_id))
                }
                (Ok(mut local_response), Err(proxy_error)) => {
                    debug!("Hybrid combined: local succeeded, proxy failed: {}", proxy_error);
                    self.enhance_response_metadata(&mut local_response, "hybrid_local_only", Some(&proxy_error.to_string()));
                    Ok(local_response)
                }
                (Err(local_error), Ok(mut proxy_response)) => {
                    debug!("Hybrid combined: proxy succeeded, local failed: {}", local_error);
                    self.enhance_proxy_response_metadata(&mut proxy_response, "hybrid_proxy_only", Some(&local_error.to_string()));
                    Ok(proxy_response)
                }
                (Err(local_error), Err(proxy_error)) => {
                    error!("Hybrid combined: both local and proxy failed - Local: {}, Proxy: {}", local_error, proxy_error);
                    Err(ProxyError::mcp(format!("Hybrid processing failed - Local: {}, Proxy: {}", local_error, proxy_error)))
                }
            }
        } else {
            // Only local processing available
            debug!("Hybrid combined: only local processing available");
            let mut response = local_future.await?;
            self.enhance_response_metadata(&mut response, "hybrid_local_only_no_proxy", None);
            Ok(response)
        }
    }

    /// Process elicitation request with hybrid combined approach (both local and proxy, combine results)
    async fn process_elicitation_hybrid_combined(&self, request: &ElicitationRequest, original_client_id: &str) -> Result<ElicitationResponse> {
        debug!("Starting hybrid combined processing for elicitation request");
        
        let local_future = self.handle_elicitation_with_magictunnel_llm(request);
        let proxy_future = if self.config.hybrid_processing.enable_proxy_forwarding {
            Some(self.forward_elicitation_to_external_servers(request, original_client_id))
        } else {
            None
        };

        if let Some(proxy_future) = proxy_future {
            // Run both and combine results intelligently
            let (local_result, proxy_result) = tokio::join!(local_future, proxy_future);
            
            match (local_result, proxy_result) {
                (Ok(local_response), Ok(proxy_response)) => {
                    debug!("Hybrid combined: both local and proxy elicitation succeeded, combining responses");
                    Ok(self.combine_elicitation_responses(local_response, proxy_response, original_client_id))
                }
                (Ok(mut local_response), Err(proxy_error)) => {
                    debug!("Hybrid combined: local elicitation succeeded, proxy failed: {}", proxy_error);
                    self.enhance_elicitation_response_metadata(&mut local_response, "hybrid_local_only", Some(&proxy_error.to_string()));
                    Ok(local_response)
                }
                (Err(local_error), Ok(mut proxy_response)) => {
                    debug!("Hybrid combined: proxy elicitation succeeded, local failed: {}", local_error);
                    self.enhance_elicitation_proxy_response_metadata(&mut proxy_response, "hybrid_proxy_only", Some(&local_error.to_string()));
                    Ok(proxy_response)
                }
                (Err(local_error), Err(proxy_error)) => {
                    error!("Hybrid combined: both local and proxy elicitation failed - Local: {}, Proxy: {}", local_error, proxy_error);
                    Err(ProxyError::mcp(format!("Hybrid elicitation processing failed - Local: {}, Proxy: {}", local_error, proxy_error)))
                }
            }
        } else {
            // Only local processing available
            debug!("Hybrid combined: only local elicitation processing available");
            let mut response = local_future.await?;
            self.enhance_elicitation_response_metadata(&mut response, "hybrid_local_only_no_proxy", None);
            Ok(response)
        }
    }

    /// Combine local and proxy sampling responses intelligently
    fn combine_sampling_responses(&self, local_response: SamplingResponse, proxy_response: SamplingResponse, original_client_id: &str) -> SamplingResponse {
        debug!("Combining local and proxy sampling responses");
        
        // Use the response with higher confidence or better metadata
        let use_proxy = proxy_response.metadata.as_ref()
            .and_then(|m| m.get("confidence_score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) > 0.7;
        
        let (primary_response, secondary_response) = if use_proxy {
            (proxy_response, local_response)
        } else {
            (local_response, proxy_response)
        };
        
        // Create combined response with enhanced metadata
        SamplingResponse {
            message: primary_response.message,
            model: format!("hybrid-{}", primary_response.model),
            stop_reason: primary_response.stop_reason,
            usage: primary_response.usage.map(|mut usage| {
                // Combine usage statistics
                if let Some(secondary_usage) = secondary_response.usage {
                    usage.input_tokens += secondary_usage.input_tokens;
                    usage.output_tokens += secondary_usage.output_tokens;
                    usage.total_tokens = usage.input_tokens + usage.output_tokens;
                    if let (Some(primary_cost), Some(secondary_cost)) = (usage.cost_usd, secondary_usage.cost_usd) {
                        usage.cost_usd = Some(primary_cost + secondary_cost);
                    }
                }
                usage
            }),
            metadata: Some([
                ("hybrid_processing".to_string(), json!("combined")),
                ("primary_source".to_string(), json!(if use_proxy { "proxy" } else { "local" })),
                ("secondary_source".to_string(), json!(if use_proxy { "local" } else { "proxy" })),
                ("combined_responses".to_string(), json!(2)),
                ("original_client_id".to_string(), json!(original_client_id)),
                ("processing_timestamp".to_string(), json!(chrono::Utc::now().to_rfc3339())),
                ("local_metadata".to_string(), json!(secondary_response.metadata.unwrap_or_default())),
                ("proxy_metadata".to_string(), json!(primary_response.metadata.unwrap_or_default())),
            ].into_iter().collect()),
        }
    }

    /// Combine local and proxy elicitation responses intelligently
    fn combine_elicitation_responses(&self, local_response: ElicitationResponse, proxy_response: ElicitationResponse, original_client_id: &str) -> ElicitationResponse {
        debug!("Combining local and proxy elicitation responses");
        
        // Extract prompts from the data field
        let local_prompt = local_response.data
            .as_ref()
            .and_then(|d| d.get("generated_prompt"))
            .and_then(|p| p.as_str())
            .unwrap_or("No local prompt");
            
        let proxy_prompt = proxy_response.data
            .as_ref()
            .and_then(|d| d.get("generated_prompt"))
            .and_then(|p| p.as_str())
            .unwrap_or("No proxy prompt");
            
        let proxy_schema = proxy_response.data
            .as_ref()
            .and_then(|d| d.get("schema"))
            .cloned()
            .unwrap_or(json!({}));
        
        // Combine prompts and use the more comprehensive schema
        let combined_prompt = format!(
            "Local Analysis: {}\n\nProxy Analysis: {}\n\nCombined Recommendation: Use the most comprehensive approach from both analyses.",
            local_prompt,
            proxy_prompt
        );
        
        ElicitationResponse {
            action: crate::mcp::types::elicitation::ElicitationAction::Accept,
            data: Some(json!({
                "generated_prompt": combined_prompt,
                "schema": proxy_schema
            })),
            reason: Some("Combined local and proxy analysis".to_string()),
            metadata: Some([
                ("hybrid_processing".to_string(), json!("combined")),
                ("combined_responses".to_string(), json!(2)),
                ("original_client_id".to_string(), json!(original_client_id)),
                ("processing_timestamp".to_string(), json!(chrono::Utc::now().to_rfc3339())),
                ("local_metadata".to_string(), json!(local_response.metadata.unwrap_or_default())),
                ("proxy_metadata".to_string(), json!(proxy_response.metadata.unwrap_or_default())),
            ].into_iter().collect()),
            timestamp: Some(chrono::Utc::now()),
        }
    }

    /// Enhance sampling response metadata with hybrid processing information
    fn enhance_response_metadata(&self, response: &mut SamplingResponse, processing_mode: &str, error_info: Option<&str>) {
        if !self.config.hybrid_processing.enable_enhanced_metadata {
            return;
        }
        
        let metadata = response.metadata.get_or_insert_with(HashMap::new);
        metadata.insert("hybrid_processing_mode".to_string(), json!(processing_mode));
        metadata.insert("hybrid_timestamp".to_string(), json!(chrono::Utc::now().to_rfc3339()));
        
        if let Some(error) = error_info {
            metadata.insert("fallback_reason".to_string(), json!(error));
        }
    }

    /// Enhance proxy sampling response metadata with hybrid processing information
    fn enhance_proxy_response_metadata(&self, response: &mut SamplingResponse, processing_mode: &str, error_info: Option<&str>) {
        if !self.config.hybrid_processing.enable_enhanced_metadata {
            return;
        }
        
        let metadata = response.metadata.get_or_insert_with(HashMap::new);
        metadata.insert("hybrid_processing_mode".to_string(), json!(processing_mode));
        metadata.insert("hybrid_timestamp".to_string(), json!(chrono::Utc::now().to_rfc3339()));
        
        if let Some(error) = error_info {
            metadata.insert("fallback_reason".to_string(), json!(error));
        }
    }

    /// Enhance elicitation response metadata with hybrid processing information
    fn enhance_elicitation_response_metadata(&self, response: &mut ElicitationResponse, processing_mode: &str, error_info: Option<&str>) {
        if !self.config.hybrid_processing.enable_enhanced_metadata {
            return;
        }
        
        let metadata = response.metadata.get_or_insert_with(HashMap::new);
        metadata.insert("hybrid_processing_mode".to_string(), json!(processing_mode));
        metadata.insert("hybrid_timestamp".to_string(), json!(chrono::Utc::now().to_rfc3339()));
        
        if let Some(error) = error_info {
            metadata.insert("fallback_reason".to_string(), json!(error));
        }
    }

    /// Enhance proxy elicitation response metadata with hybrid processing information
    fn enhance_elicitation_proxy_response_metadata(&self, response: &mut ElicitationResponse, processing_mode: &str, error_info: Option<&str>) {
        if !self.config.hybrid_processing.enable_enhanced_metadata {
            return;
        }
        
        let metadata = response.metadata.get_or_insert_with(HashMap::new);
        metadata.insert("hybrid_processing_mode".to_string(), json!(processing_mode));
        metadata.insert("hybrid_timestamp".to_string(), json!(chrono::Utc::now().to_rfc3339()));
        
        if let Some(error) = error_info {
            metadata.insert("fallback_reason".to_string(), json!(error));
        }
    }
}
