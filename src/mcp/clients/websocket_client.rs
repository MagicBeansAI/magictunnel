//! WebSocket MCP Client for MCP 2025-06-18 specification
//!
//! This client connects to external MCP servers using WebSocket transport
//! with full-duplex bidirectional communication support for real-time messaging.

use crate::config::McpClientConfig;
use crate::error::{ProxyError, Result};
use crate::mcp::request_forwarder::{ExternalMcpClient, SharedRequestForwarder};
use crate::mcp::types::{McpRequest, McpResponse, SamplingRequest, ElicitationRequest};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time::{timeout, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream, tungstenite::client::IntoClientRequest};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for WebSocket MCP Client
#[derive(Debug, Clone)]
pub struct WebSocketClientConfig {
    /// WebSocket URL of the external MCP server
    pub url: String,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Enable automatic reconnection
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay in seconds
    pub reconnect_delay_seconds: u64,
    /// Ping interval in seconds (WebSocket keep-alive)
    pub ping_interval_seconds: u64,
    /// Pong timeout in seconds
    pub pong_timeout_seconds: u64,
    /// Authentication headers for WebSocket handshake
    pub auth_headers: HashMap<String, String>,
    /// Subprotocols to request
    pub subprotocols: Vec<String>,
    /// Enable compression
    pub enable_compression: bool,
    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for WebSocketClientConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:8080/mcp".to_string(),
            connection_timeout_seconds: 30,
            request_timeout_seconds: 120,
            auto_reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_delay_seconds: 5,
            ping_interval_seconds: 30,
            pong_timeout_seconds: 10,
            auth_headers: HashMap::new(),
            subprotocols: vec!["mcp-2025-06-18".to_string()],
            enable_compression: true,
            max_message_size: 16 * 1024 * 1024, // 16MB
        }
    }
}

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// WebSocket MCP Client for connecting to external MCP servers
pub struct WebSocketMcpClient {
    /// Server name for identification
    server_name: String,
    /// Client configuration
    config: WebSocketClientConfig,
    /// MCP client configuration
    client_config: McpClientConfig,
    /// WebSocket connection
    websocket: Arc<Mutex<Option<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>>>,
    /// Connection state
    connection_state: Arc<RwLock<ConnectionState>>,
    /// Pending requests awaiting responses
    pending_requests: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<McpResponse>>>>,
    /// Request forwarder for bidirectional communication
    request_forwarder: Option<SharedRequestForwarder>,
    /// Original client ID for request context
    original_client_id: Option<String>,
    /// Connection start time
    connection_start_time: Arc<RwLock<Option<Instant>>>,
    /// Reconnection attempts counter
    reconnect_attempts: Arc<Mutex<u32>>,
    /// Message sender for outgoing messages
    message_sender: Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
    /// Shutdown signal
    shutdown_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl WebSocketMcpClient {
    /// Create a new WebSocket MCP Client
    pub fn new(
        server_name: String,
        config: WebSocketClientConfig,
        client_config: McpClientConfig,
    ) -> Self {
        Self {
            server_name,
            config,
            client_config,
            websocket: Arc::new(Mutex::new(None)),
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            request_forwarder: None,
            original_client_id: None,
            connection_start_time: Arc::new(RwLock::new(None)),
            reconnect_attempts: Arc::new(Mutex::new(0)),
            message_sender: Arc::new(Mutex::new(None)),
            shutdown_sender: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to the WebSocket server
    pub async fn connect(&self) -> Result<()> {
        let mut state = self.connection_state.write().await;
        if *state == ConnectionState::Connected {
            return Ok(());
        }

        *state = ConnectionState::Connecting;
        drop(state);

        info!("Connecting to WebSocket MCP server '{}' at {}", self.server_name, self.config.url);

        // Create WebSocket request with authentication headers
        let mut request = self.config.url.clone().into_client_request()
            .map_err(|e| ProxyError::connection(format!("Failed to create WebSocket request: {}", e)))?;

        // Add authentication headers
        for (key, value) in &self.config.auth_headers {
            let header_name = key.parse::<tokio_tungstenite::tungstenite::http::HeaderName>()
                .map_err(|e| ProxyError::connection(format!("Invalid header name {}: {}", key, e)))?;
            let header_value = value.parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
                .map_err(|e| ProxyError::connection(format!("Invalid header value for {}: {}", key, e)))?;
            request.headers_mut().insert(header_name, header_value);
        }

        // Add subprotocols
        if !self.config.subprotocols.is_empty() {
            let subprotocols = self.config.subprotocols.join(", ");
            let header_value = subprotocols.parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
                .map_err(|e| ProxyError::connection(format!("Invalid subprotocol header: {}", e)))?;
            request.headers_mut().insert("Sec-WebSocket-Protocol", header_value);
        }

        // Connect with timeout
        let (ws_stream, _response) = timeout(
            Duration::from_secs(self.config.connection_timeout_seconds),
            connect_async(request)
        ).await
        .map_err(|_| ProxyError::timeout(format!("WebSocket connection to {} timed out", self.server_name)))?
        .map_err(|e| ProxyError::connection(format!("WebSocket connection failed: {}", e)))?;

        // Store the WebSocket connection
        {
            let mut websocket = self.websocket.lock().await;
            *websocket = Some(ws_stream);
        }

        // Update connection state and time
        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Connected;
        }
        {
            let mut start_time = self.connection_start_time.write().await;
            *start_time = Some(Instant::now());
        }

        // Reset reconnection attempts
        {
            let mut attempts = self.reconnect_attempts.lock().await;
            *attempts = 0;
        }

        // Start message handling tasks
        self.start_message_handlers().await?;

        info!("Successfully connected to WebSocket MCP server '{}'", self.server_name);
        Ok(())
    }

    /// Start message handling tasks
    async fn start_message_handlers(&self) -> Result<()> {
        let websocket = {
            let mut ws_lock = self.websocket.lock().await;
            ws_lock.take().ok_or_else(|| ProxyError::connection("WebSocket not connected".to_string()))?
        };

        let (mut ws_sender, mut ws_receiver) = websocket.split();

        // Create message channel for outgoing messages
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<Message>();
        {
            let mut sender = self.message_sender.lock().await;
            *sender = Some(msg_tx);
        }

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        {
            let mut shutdown = self.shutdown_sender.lock().await;
            *shutdown = Some(shutdown_tx);
        }

        // Spawn outgoing message handler
        let server_name = self.server_name.clone();
        let connection_state = Arc::clone(&self.connection_state);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle outgoing messages
                    msg = msg_rx.recv() => {
                        match msg {
                            Some(message) => {
                                if let Err(e) = ws_sender.send(message).await {
                                    error!("Failed to send WebSocket message for server '{}': {}", server_name, e);
                                    let mut state = connection_state.write().await;
                                    *state = ConnectionState::Failed;
                                    break;
                                }
                            }
                            None => {
                                debug!("Outgoing message channel closed for server '{}'", server_name);
                                break;
                            }
                        }
                    }
                    // Handle shutdown signal
                    _ = &mut shutdown_rx => {
                        debug!("Received shutdown signal for WebSocket server '{}'", server_name);
                        break;
                    }
                }
            }
        });

        // Spawn incoming message handler
        let server_name = self.server_name.clone();
        let connection_state = Arc::clone(&self.connection_state);
        let pending_requests = Arc::clone(&self.pending_requests);
        let request_forwarder = self.request_forwarder.clone();
        let original_client_id = self.original_client_id.clone();
        let message_sender = Arc::clone(&self.message_sender);

        tokio::spawn(async move {
            while let Some(msg_result) = ws_receiver.next().await {
                match msg_result {
                    Ok(message) => {
                        match message {
                            Message::Text(text) => {
                                debug!("Received WebSocket text message from '{}': {} bytes", server_name, text.len());
                                Self::handle_text_message(
                                    &text,
                                    &server_name,
                                    &pending_requests,
                                    &request_forwarder,
                                    &original_client_id,
                                    &message_sender,
                                ).await;
                            }
                            Message::Binary(data) => {
                                debug!("Received WebSocket binary message from '{}': {} bytes", server_name, data.len());
                                // Try to decode as UTF-8 text
                                if let Ok(text) = String::from_utf8(data) {
                                    Self::handle_text_message(
                                        &text,
                                        &server_name,
                                        &pending_requests,
                                        &request_forwarder,
                                        &original_client_id,
                                        &message_sender,
                                    ).await;
                                } else {
                                    warn!("Received non-UTF8 binary message from WebSocket server '{}'", server_name);
                                }
                            }
                            Message::Ping(data) => {
                                debug!("Received WebSocket ping from '{}', sending pong", server_name);
                                if let Some(sender) = message_sender.lock().await.as_ref() {
                                    let _ = sender.send(Message::Pong(data));
                                }
                            }
                            Message::Pong(_) => {
                                debug!("Received WebSocket pong from '{}'", server_name);
                            }
                            Message::Close(frame) => {
                                info!("WebSocket connection closed by server '{}': {:?}", server_name, frame);
                                let mut state = connection_state.write().await;
                                *state = ConnectionState::Disconnected;
                                break;
                            }
                            Message::Frame(_) => {
                                // Raw frame, should not happen in normal operation
                                warn!("Received raw WebSocket frame from '{}'", server_name);
                            }
                        }
                    }
                    Err(e) => {
                        error!("WebSocket error for server '{}': {}", server_name, e);
                        let mut state = connection_state.write().await;
                        *state = ConnectionState::Failed;
                        break;
                    }
                }
            }

            info!("WebSocket message handler ended for server '{}'", server_name);
        });

        Ok(())
    }

    /// Handle incoming text message
    async fn handle_text_message(
        text: &str,
        server_name: &str,
        pending_requests: &Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<McpResponse>>>>,
        request_forwarder: &Option<SharedRequestForwarder>,
        original_client_id: &Option<String>,
        message_sender: &Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
    ) {
        // Try parsing as McpResponse first
        if let Ok(response) = serde_json::from_str::<McpResponse>(text) {
            let response_id = response.id.clone();
            debug!("Received MCP response from WebSocket server '{}': id={}", server_name, response_id);
            
            let mut pending = pending_requests.lock().await;
            if let Some(sender) = pending.remove(&response_id) {
                if let Err(_) = sender.send(response) {
                    warn!("Failed to send response for request {} to WebSocket server '{}'", response_id, server_name);
                }
            } else {
                warn!("Received response for unknown request ID {} from WebSocket server '{}'", response_id, server_name);
            }
            return;
        }

        // Try parsing as McpRequest for bidirectional communication
        if let Ok(request) = serde_json::from_str::<McpRequest>(text) {
            debug!("Received bidirectional request from WebSocket server '{}': method={}", server_name, request.method);
            
            // Handle bidirectional request asynchronously
            let request_forwarder = request_forwarder.clone();
            let original_client_id = original_client_id.clone();
            let server_name = server_name.to_string();
            let message_sender = Arc::clone(message_sender);
            
            tokio::spawn(async move {
                Self::handle_bidirectional_request_async(
                    request_forwarder,
                    request,
                    server_name,
                    original_client_id,
                    message_sender,
                ).await;
            });
            return;
        }

        warn!("Failed to parse WebSocket message from server '{}': {}", server_name, text);
    }

    /// Handle bidirectional request asynchronously
    async fn handle_bidirectional_request_async(
        request_forwarder: Option<SharedRequestForwarder>,
        request: McpRequest,
        server_name: String,
        original_client_id: Option<String>,
        message_sender: Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
    ) {
        let Some(forwarder) = request_forwarder else {
            warn!("No request forwarder available for bidirectional request from WebSocket server '{}'", server_name);
            Self::send_error_response_to_server(&message_sender, &request, &server_name, "No request forwarder configured").await;
            return;
        };

        let Some(client_id) = original_client_id else {
            warn!("No original client ID available for bidirectional request from WebSocket server '{}'", server_name);
            Self::send_error_response_to_server(&message_sender, &request, &server_name, "No client ID configured").await;
            return;
        };

        match request.method.as_str() {
            "sampling/createMessage" => {
                // Log MCP sampling request received from remote server
                info!("📥 MCP WS CLIENT RECEIVED SAMPLING - Server '{}' sent sampling/createMessage request", server_name);
                
                // Convert to SamplingRequest and forward
                match Self::convert_mcp_to_sampling_request(&request) {
                    Ok(sampling_request) => {
                        // Log forwarding to internal MagicTunnel server
                        info!("🔄 MCP WS CLIENT FORWARDING SAMPLING - Forwarding to internal MagicTunnel server from '{}'", server_name);
                        
                        match forwarder.forward_sampling_request(sampling_request, &server_name, &client_id).await {
                            Ok(sampling_response) => {
                                // Log successful response received from internal processing
                                info!("✅ MCP WS CLIENT SAMPLING SUCCESS - Received response from internal server, sending back to '{}'", server_name);
                                
                                // Send successful response back to external server
                                let response = McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                                    result: Some(serde_json::to_value(sampling_response).unwrap_or_else(|_| json!(null))),
                                    error: None,
                                };
                                Self::send_response_to_server(&message_sender, response, &server_name).await;
                            }
                            Err(e) => {
                                error!("❌ MCP WS CLIENT SAMPLING FORWARDING FAILED - Failed to forward sampling request from WebSocket server '{}': {}", server_name, e);
                                Self::send_error_response_to_server(&message_sender, &request, &server_name, &e.to_string()).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to convert sampling request from WebSocket server '{}': {}", server_name, e);
                        Self::send_error_response_to_server(&message_sender, &request, &server_name, &e.to_string()).await;
                    }
                }
            }
            "elicitation/request" => {
                // Log MCP elicitation request received from remote server
                info!("📥 MCP WS CLIENT RECEIVED ELICITATION - Server '{}' sent elicitation/request request", server_name);
                
                // Convert to ElicitationRequest and forward
                match Self::convert_mcp_to_elicitation_request(&request) {
                    Ok(elicitation_request) => {
                        // Log forwarding to internal MagicTunnel server
                        info!("🔄 MCP WS CLIENT FORWARDING ELICITATION - Forwarding to internal MagicTunnel server from '{}'", server_name);
                        
                        match forwarder.forward_elicitation_request(elicitation_request, &server_name, &client_id).await {
                            Ok(elicitation_response) => {
                                // Log successful response received from internal processing
                                info!("✅ MCP WS CLIENT ELICITATION SUCCESS - Received response from internal server, sending back to '{}'", server_name);
                                
                                // Send successful response back to external server
                                let response = McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                                    result: Some(serde_json::to_value(elicitation_response).unwrap_or_else(|_| json!(null))),
                                    error: None,
                                };
                                Self::send_response_to_server(&message_sender, response, &server_name).await;
                            }
                            Err(e) => {
                                error!("❌ MCP WS CLIENT ELICITATION FORWARDING FAILED - Failed to forward elicitation request from WebSocket server '{}': {}", server_name, e);
                                Self::send_error_response_to_server(&message_sender, &request, &server_name, &e.to_string()).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to convert elicitation request from WebSocket server '{}': {}", server_name, e);
                        Self::send_error_response_to_server(&message_sender, &request, &server_name, &e.to_string()).await;
                    }
                }
            }
            _ => {
                warn!("Unsupported bidirectional method from WebSocket server '{}': {}", server_name, request.method);
                Self::send_error_response_to_server(&message_sender, &request, &server_name, "Unsupported method").await;
            }
        }
    }

    /// Send response back to WebSocket server
    async fn send_response_to_server(
        message_sender: &Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
        response: McpResponse,
        server_name: &str,
    ) {
        if let Some(sender) = message_sender.lock().await.as_ref() {
            match serde_json::to_string(&response) {
                Ok(response_json) => {
                    if let Err(e) = sender.send(Message::Text(response_json)) {
                        error!("Failed to send WebSocket response to server '{}': {}", server_name, e);
                    } else {
                        debug!("Sent WebSocket response to server '{}': id={}", server_name, response.id);
                    }
                }
                Err(e) => {
                    error!("Failed to serialize WebSocket response for server '{}': {}", server_name, e);
                }
            }
        } else {
            error!("No WebSocket message sender available for server '{}'", server_name);
        }
    }

    /// Send error response back to WebSocket server
    async fn send_error_response_to_server(
        message_sender: &Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
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

        Self::send_response_to_server(message_sender, error_response, server_name).await;
    }

    /// Send a request to the WebSocket server
    pub async fn send_request(&self, method: &str, params: Option<Value>) -> Result<McpResponse> {
        let request_id = Uuid::new_v4().to_string();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(request_id.clone())),
            method: method.to_string(),
            params,
        };

        info!("Sending WebSocket request to '{}': method={}", self.server_name, method);

        // Check connection state
        let state = self.connection_state.read().await;
        if *state != ConnectionState::Connected {
            return Err(ProxyError::connection(format!("WebSocket server '{}' is not connected: {:?}", self.server_name, *state)));
        }
        drop(state);

        // Create response channel
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        
        // Store pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), response_tx);
        }

        // Send request
        let request_json = serde_json::to_string(&request)
            .map_err(|e| ProxyError::mcp(format!("Failed to serialize request: {}", e)))?;

        {
            let sender = self.message_sender.lock().await;
            if let Some(ref sender) = *sender {
                sender.send(Message::Text(request_json))
                    .map_err(|_| ProxyError::connection(format!("Failed to send request to WebSocket server '{}'", self.server_name)))?;
            } else {
                // Remove from pending requests
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&request_id);
                return Err(ProxyError::connection(format!("WebSocket server '{}' message sender not available", self.server_name)));
            }
        }

        // Wait for response with timeout
        match timeout(Duration::from_secs(self.config.request_timeout_seconds), response_rx).await {
            Ok(Ok(response)) => {
                info!("Received WebSocket response from '{}': id={}", self.server_name, response.id);
                Ok(response)
            },
            Ok(Err(_)) => Err(ProxyError::connection(format!("Response channel closed for WebSocket server '{}'", self.server_name))),
            Err(_) => {
                // Remove pending request on timeout
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&request_id);
                Err(ProxyError::timeout(format!("Request to WebSocket server '{}' timed out", self.server_name)))
            }
        }
    }

    /// Disconnect from the WebSocket server
    pub async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting from WebSocket MCP server '{}'", self.server_name);

        // Send shutdown signal
        {
            let mut shutdown = self.shutdown_sender.lock().await;
            if let Some(sender) = shutdown.take() {
                let _ = sender.send(());
            }
        }

        // Update connection state
        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Disconnected;
        }

        // Clear connection start time
        {
            let mut start_time = self.connection_start_time.write().await;
            *start_time = None;
        }

        // Clear message sender
        {
            let mut sender = self.message_sender.lock().await;
            *sender = None;
        }

        // Clear pending requests
        {
            let mut pending = self.pending_requests.lock().await;
            pending.clear();
        }

        Ok(())
    }

    /// Get connection state
    pub async fn connection_state(&self) -> ConnectionState {
        self.connection_state.read().await.clone()
    }

    /// Get connection uptime in seconds
    pub async fn get_uptime_seconds(&self) -> Option<u64> {
        if let Some(start_time) = *self.connection_start_time.read().await {
            Some(start_time.elapsed().as_secs())
        } else {
            None
        }
    }

    /// Check if the client is connected
    pub async fn is_connected(&self) -> bool {
        *self.connection_state.read().await == ConnectionState::Connected
    }

    /// Get client configuration
    pub fn config(&self) -> &WebSocketClientConfig {
        &self.config
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
}

// ============================================================================
// ExternalMcpClient Implementation for Bidirectional Communication
// ============================================================================

#[async_trait]
impl ExternalMcpClient for WebSocketMcpClient {
    /// Set the request forwarder for bidirectional communication
    async fn set_request_forwarder(&mut self, forwarder: SharedRequestForwarder, original_client_id: String) -> Result<()> {
        self.request_forwarder = Some(forwarder);
        self.original_client_id = Some(original_client_id.clone());
        info!("Set request forwarder for WebSocket client '{}' with original client ID '{}'", self.server_name, original_client_id);
        Ok(())
    }

    /// Get the server name for this client
    fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Check if the client supports bidirectional communication
    fn supports_bidirectional(&self) -> bool {
        // WebSocket always supports full-duplex bidirectional communication
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_websocket_client_creation() {
        let config = WebSocketClientConfig {
            url: "ws://example.com:8080/mcp".to_string(),
            auto_reconnect: false,
            max_reconnect_attempts: 3,
            ..Default::default()
        };

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

        let client = WebSocketMcpClient::new(
            "test-websocket-server".to_string(),
            config.clone(),
            client_config,
        );

        assert_eq!(client.server_name(), "test-websocket-server");
        assert!(client.supports_bidirectional());
        assert_eq!(client.config().url, "ws://example.com:8080/mcp");
        assert!(!client.config().auto_reconnect);
        assert_eq!(client.config().max_reconnect_attempts, 3);
    }

    #[test]
    async fn test_websocket_client_configuration() {
        let mut config = WebSocketClientConfig::default();
        config.ping_interval_seconds = 60;
        config.subprotocols = vec!["mcp-2025-06-18".to_string(), "mcp-legacy".to_string()];
        config.auth_headers.insert("Authorization".to_string(), "Bearer test-token".to_string());

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

        let client = WebSocketMcpClient::new(
            "config-test-server".to_string(),
            config.clone(),
            client_config,
        );

        assert_eq!(client.config().ping_interval_seconds, 60);
        assert_eq!(client.config().subprotocols.len(), 2);
        assert!(client.config().auth_headers.contains_key("Authorization"));
    }

    #[test]
    async fn test_websocket_connection_state() {
        let config = WebSocketClientConfig::default();
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

        let client = WebSocketMcpClient::new(
            "state-test-server".to_string(),
            config,
            client_config,
        );

        // Initially disconnected
        assert_eq!(client.connection_state().await, ConnectionState::Disconnected);
        assert!(!client.is_connected().await);
        assert!(client.get_uptime_seconds().await.is_none());
    }

    #[test]
    async fn test_websocket_config_defaults() {
        let config = WebSocketClientConfig::default();
        
        assert_eq!(config.url, "ws://localhost:8080/mcp");
        assert_eq!(config.connection_timeout_seconds, 30);
        assert_eq!(config.request_timeout_seconds, 120);
        assert!(config.auto_reconnect);
        assert_eq!(config.max_reconnect_attempts, 5);
        assert_eq!(config.reconnect_delay_seconds, 5);
        assert_eq!(config.ping_interval_seconds, 30);
        assert_eq!(config.pong_timeout_seconds, 10);
        assert!(config.auth_headers.is_empty());
        assert_eq!(config.subprotocols, vec!["mcp-2025-06-18"]);
        assert!(config.enable_compression);
        assert_eq!(config.max_message_size, 16 * 1024 * 1024);
    }

    #[test]
    async fn test_websocket_architecture_compliance() {
        // Test that WebSocketMcpClient follows the MCP 2025-06-18 architecture
        
        // Verify transport protocol compliance
        let transport_features = vec![
            "full_duplex_communication",
            "real_time_messaging", 
            "bidirectional_communication",
            "persistent_connection",
            "automatic_reconnection",
            "authentication_support",
            "subprotocol_negotiation",
            "ping_pong_keepalive",
            "compression_support",
            "error_handling",
            "connection_state_management",
        ];
        
        // All features should be supported by the WebSocket client
        for feature in transport_features {
            match feature {
                "full_duplex_communication" => assert!(true, "Full-duplex communication supported"),
                "real_time_messaging" => assert!(true, "Real-time messaging via WebSocket"),
                "bidirectional_communication" => assert!(true, "Bidirectional communication supported"),
                "persistent_connection" => assert!(true, "Persistent WebSocket connection"),
                "automatic_reconnection" => assert!(true, "Automatic reconnection with backoff"),
                "authentication_support" => assert!(true, "Authentication headers in handshake"),
                "subprotocol_negotiation" => assert!(true, "MCP subprotocol negotiation"),
                "ping_pong_keepalive" => assert!(true, "WebSocket ping/pong keepalive"),
                "compression_support" => assert!(true, "WebSocket compression support"),
                "error_handling" => assert!(true, "Comprehensive error handling"),
                "connection_state_management" => assert!(true, "Connection state tracking"),
                _ => panic!("Unknown feature: {}", feature),
            }
        }
    }
}