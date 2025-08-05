//! Streamable HTTP Transport implementation for MCP 2025-06-18 specification
//!
//! This replaces the deprecated HTTP+SSE transport with the new Streamable HTTP transport
//! as required by the MCP 2025-06-18 specification.

use crate::error::{ProxyError, Result};
use crate::mcp::types::{McpRequest, McpResponse};
use crate::mcp::errors::McpError;
use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Streamable HTTP Transport configuration
#[derive(Debug, Clone)]
pub struct StreamableHttpConfig {
    /// Enable enhanced batching
    pub enable_batching: bool,
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,
    /// Enable compression
    pub enable_compression: bool,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Enable keep-alive
    pub enable_keep_alive: bool,
}

impl Default for StreamableHttpConfig {
    fn default() -> Self {
        Self {
            enable_batching: true,
            max_batch_size: 100,
            batch_timeout_ms: 50,
            enable_compression: true,
            max_message_size: 10 * 1024 * 1024, // 10MB
            connection_timeout_seconds: 30,
            enable_keep_alive: true,
        }
    }
}

/// Streamable HTTP connection session
#[derive(Debug)]
pub struct StreamableHttpSession {
    /// Session ID
    pub id: String,
    /// Request sender
    pub request_tx: mpsc::UnboundedSender<McpRequest>,
    /// Response receiver
    pub response_rx: Option<mpsc::UnboundedReceiver<McpResponse>>,
    /// Connection metadata
    pub metadata: HashMap<String, Value>,
    /// Created timestamp
    pub created_at: std::time::Instant,
}

/// Streamable HTTP Transport manager
pub struct StreamableHttpTransport {
    /// Transport configuration
    config: StreamableHttpConfig,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, StreamableHttpSession>>>,
    /// Batch processing queue
    batch_queue: Arc<RwLock<Vec<(String, McpRequest)>>>,
    /// Statistics
    stats: Arc<RwLock<StreamableHttpStats>>,
    /// MCP Server for request processing
    mcp_server: Option<Arc<crate::mcp::McpServer>>,
}

/// Transport statistics
#[derive(Debug, Default, Clone)]
pub struct StreamableHttpStats {
    /// Total connections
    pub total_connections: u64,
    /// Active connections
    pub active_connections: u64,
    /// Total messages processed
    pub total_messages: u64,
    /// Total batches processed
    pub total_batches: u64,
    /// Average batch size
    pub avg_batch_size: f64,
    /// Total bytes transferred
    pub total_bytes: u64,
    /// Connection errors
    pub connection_errors: u64,
    /// Message errors
    pub message_errors: u64,
}

impl StreamableHttpTransport {
    /// Create a new Streamable HTTP Transport
    pub fn new(config: StreamableHttpConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            batch_queue: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(StreamableHttpStats::default())),
            mcp_server: None,
        }
    }

    /// Create a new Streamable HTTP Transport with MCP server integration
    pub fn with_server(config: StreamableHttpConfig, mcp_server: Arc<crate::mcp::McpServer>) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            batch_queue: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(StreamableHttpStats::default())),
            mcp_server: Some(mcp_server),
        }
    }

    /// Handle incoming streamable HTTP request
    pub async fn handle_streamable_request(
        &self,
        req: HttpRequest,
        body: web::Bytes,
    ) -> Result<HttpResponse> {
        // Update connection stats
        {
            let mut stats = self.stats.write().await;
            stats.total_connections += 1;
            stats.active_connections += 1;
        }

        // Validate content type
        let content_type = req.headers().get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("application/json");

        if !content_type.starts_with("application/json") && !content_type.starts_with("application/x-ndjson") {
            return Err(ProxyError::validation("Unsupported content type for Streamable HTTP"));
        }

        // Handle different types of streamable requests
        match content_type {
            ct if ct.starts_with("application/x-ndjson") => {
                self.handle_ndjson_stream(req, body).await
            }
            ct if ct.starts_with("application/json") => {
                // Check if this is a batch request
                if self.is_batch_request(&body)? {
                    self.handle_batch_request(req, body).await
                } else {
                    self.handle_single_request(req, body).await
                }
            }
            _ => Err(ProxyError::validation("Unsupported streamable HTTP content type"))
        }
    }

    /// Handle newline-delimited JSON stream
    async fn handle_ndjson_stream(
        &self,
        _req: HttpRequest,
        body: web::Bytes,
    ) -> Result<HttpResponse> {
        debug!("Processing NDJSON stream with {} bytes", body.len());

        let body_str = std::str::from_utf8(&body)
            .map_err(|_| ProxyError::validation("Invalid UTF-8 in NDJSON stream"))?;

        let mut responses = Vec::new();
        let mut processed_count = 0;

        // Process each line as a separate JSON-RPC request
        for (line_num, line) in body_str.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<McpRequest>(line) {
                Ok(request) => {
                    // Process individual request
                    match self.process_single_mcp_request(request).await {
                        Ok(response) => {
                            responses.push(response);
                            processed_count += 1;
                        }
                        Err(e) => {
                            warn!("Failed to process NDJSON line {}: {}", line_num + 1, e);
                            // Create error response
                            let error_response = McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: "null".to_string(),
                                result: None,
                                error: Some(crate::mcp::McpError::invalid_request(
                                    format!("Line {}: {}", line_num + 1, e)
                                )),
                            };
                            responses.push(error_response);
                        }
                    }
                }
                Err(e) => {
                    warn!("Invalid JSON on line {}: {}", line_num + 1, e);
                }
            }
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_messages += processed_count;
            stats.total_bytes += body.len() as u64;
        }

        // Return NDJSON response
        let response_body = responses
            .iter()
            .map(|r| serde_json::to_string(r).unwrap_or_else(|_| "{}".to_string()))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(HttpResponse::Ok()
            .content_type("application/x-ndjson")
            .insert_header(("X-MCP-Transport", "streamable-http"))
            .insert_header(("X-MCP-Version", "2025-06-18"))
            .insert_header(("X-Processed-Count", processed_count.to_string()))
            .body(response_body))
    }

    /// Handle batch request
    async fn handle_batch_request(
        &self,
        _req: HttpRequest,
        body: web::Bytes,
    ) -> Result<HttpResponse> {
        debug!("Processing batch request with {} bytes", body.len());

        let requests: Vec<McpRequest> = serde_json::from_slice(&body)
            .map_err(|e| ProxyError::validation(format!("Invalid batch JSON: {}", e)))?;

        if requests.len() > self.config.max_batch_size {
            return Err(ProxyError::validation(format!(
                "Batch size {} exceeds maximum {}", 
                requests.len(), 
                self.config.max_batch_size
            )));
        }

        let mut responses = Vec::new();
        for request in requests {
            match self.process_single_mcp_request(request).await {
                Ok(response) => responses.push(response),
                Err(e) => {
                    warn!("Failed to process batch request: {}", e);
                    let error_response = McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: "null".to_string(),
                        result: None,
                        error: Some(crate::mcp::McpError::invalid_request(e.to_string())),
                    };
                    responses.push(error_response);
                }
            }
        }

        // Update batch stats
        {
            let mut stats = self.stats.write().await;
            stats.total_batches += 1;
            stats.total_messages += responses.len() as u64;
            stats.avg_batch_size = (stats.avg_batch_size * (stats.total_batches - 1) as f64 
                + responses.len() as f64) / stats.total_batches as f64;
            stats.total_bytes += body.len() as u64;
        }

        let response_json = serde_json::to_string(&responses)
            .map_err(|e| ProxyError::mcp(format!("Failed to serialize batch response: {}", e)))?;

        Ok(HttpResponse::Ok()
            .content_type("application/json")
            .insert_header(("X-MCP-Transport", "streamable-http"))
            .insert_header(("X-MCP-Version", "2025-06-18"))
            .insert_header(("X-Batch-Size", responses.len().to_string()))
            .body(response_json))
    }

    /// Handle single request
    async fn handle_single_request(
        &self,
        _req: HttpRequest,
        body: web::Bytes,
    ) -> Result<HttpResponse> {
        debug!("Processing single request with {} bytes", body.len());

        let request: McpRequest = serde_json::from_slice(&body)
            .map_err(|e| ProxyError::validation(format!("Invalid JSON-RPC request: {}", e)))?;

        let response = self.process_single_mcp_request(request).await?;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_messages += 1;
            stats.total_bytes += body.len() as u64;
        }

        let response_json = serde_json::to_string(&response)
            .map_err(|e| ProxyError::mcp(format!("Failed to serialize response: {}", e)))?;

        Ok(HttpResponse::Ok()
            .content_type("application/json")
            .insert_header(("X-MCP-Transport", "streamable-http"))
            .insert_header(("X-MCP-Version", "2025-06-18"))
            .body(response_json))
    }

    /// Check if request body is a batch request
    fn is_batch_request(&self, body: &web::Bytes) -> Result<bool> {
        let parsed: Value = serde_json::from_slice(body)
            .map_err(|e| ProxyError::validation(format!("Invalid JSON: {}", e)))?;
        
        Ok(parsed.is_array())
    }

    /// Process a single MCP request through the unified MCP handler
    async fn process_single_mcp_request(&self, request: McpRequest) -> Result<McpResponse> {
        info!("Processing MCP request via Streamable HTTP: {}", request.method);
        
        // Use the unified MCP handler if available
        if let Some(ref mcp_server) = self.mcp_server {
            match mcp_server.handle_mcp_request(request.clone()).await {
                Ok(Some(response_str)) => {
                    // Parse the JSON response string back to McpResponse
                    match serde_json::from_str::<Value>(&response_str) {
                        Ok(response_json) => {
                            let id = request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
                            
                            if let Some(error) = response_json.get("error") {
                                let mcp_error = serde_json::from_value::<McpError>(error.clone())
                                    .unwrap_or_else(|_| McpError {
                                        code: -32603,
                                        message: "Internal error".to_string(),
                                        data: None,
                                    });
                                Ok(McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id,
                                    result: None,
                                    error: Some(mcp_error),
                                })
                            } else {
                                Ok(McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id,
                                    result: response_json.get("result").cloned(),
                                    error: None,
                                })
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse MCP response JSON: {}", e);
                            Ok(McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                                result: None,
                                error: Some(McpError {
                                    code: -32603,
                                    message: "Internal error parsing response".to_string(),
                                    data: None,
                                }),
                            })
                        }
                    }
                }
                Ok(None) => {
                    // No response needed (notification)
                    Ok(McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                        result: Some(serde_json::json!({"status": "notification_processed"})),
                        error: None,
                    })
                }
                Err(e) => {
                    error!("MCP request processing failed: {}", e);
                    Ok(McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                        result: None,
                        error: Some(McpError {
                            code: -32603,
                            message: format!("Request processing failed: {}", e),
                            data: None,
                        }),
                    })
                }
            }
        } else {
            // Fallback when no MCP server is available
            warn!("No MCP server available for Streamable HTTP transport, using fallback response");
            Ok(McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
                result: Some(serde_json::json!({
                    "transport": "streamable-http",
                    "version": "2025-06-18",
                    "method": request.method,
                    "status": "fallback",
                    "message": "Request processed via fallback handler - MCP server not available"
                })),
                error: None,
            })
        }
    }

    /// Create a new streaming session
    pub async fn create_session(&self) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let (request_tx, _request_rx) = mpsc::unbounded_channel();
        let (_response_tx, response_rx) = mpsc::unbounded_channel();

        let session = StreamableHttpSession {
            id: session_id.clone(),
            request_tx,
            response_rx: Some(response_rx),
            metadata: HashMap::new(),
            created_at: std::time::Instant::now(),
        };

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        info!("Created new streamable HTTP session: {}", session_id);
        Ok(session_id)
    }

    /// Close a streaming session
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(_session) = sessions.remove(session_id) {
            info!("Closed streamable HTTP session: {}", session_id);
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.active_connections = stats.active_connections.saturating_sub(1);
            
            Ok(())
        } else {
            Err(ProxyError::mcp(format!("Session not found: {}", session_id)))
        }
    }

    /// Get transport statistics
    pub async fn get_stats(&self) -> StreamableHttpStats {
        (*self.stats.read().await).clone()
    }

    /// Get configuration
    pub fn config(&self) -> &StreamableHttpConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[tokio::test]
    async fn test_transport_creation() {
        let config = StreamableHttpConfig::default();
        let transport = StreamableHttpTransport::new(config);
        
        let stats = transport.get_stats().await;
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_session_management() {
        let config = StreamableHttpConfig::default();
        let transport = StreamableHttpTransport::new(config);
        
        let session_id = transport.create_session().await.unwrap();
        assert!(!session_id.is_empty());
        
        let result = transport.close_session(&session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_batch_detection() {
        let config = StreamableHttpConfig::default();
        let transport = StreamableHttpTransport::new(config);
        
        let single_request = web::Bytes::from(r#"{"jsonrpc":"2.0","method":"test","id":1}"#);
        let is_batch = transport.is_batch_request(&single_request).unwrap();
        assert!(!is_batch);
        
        let batch_request = web::Bytes::from(r#"[{"jsonrpc":"2.0","method":"test","id":1}]"#);
        let is_batch = transport.is_batch_request(&batch_request).unwrap();
        assert!(is_batch);
    }

    #[tokio::test]
    async fn test_single_request_processing() {
        let config = StreamableHttpConfig::default();
        let transport = StreamableHttpTransport::new(config);
        
        let req = TestRequest::default()
            .insert_header(("content-type", "application/json"))
            .to_http_request();
        
        let body = web::Bytes::from(r#"{"jsonrpc":"2.0","method":"test","id":1}"#);
        
        let response = transport.handle_single_request(req, body).await;
        assert!(response.is_ok());
    }
}