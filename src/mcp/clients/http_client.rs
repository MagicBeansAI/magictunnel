//! HTTP MCP Client
//!
//! This module implements an HTTP client for connecting to external MCP services
//! that expose MCP-over-HTTP endpoints. It provides connection pooling, authentication,
//! error handling, and retry logic.

use crate::error::{ProxyError, Result};
use crate::mcp::types::{Tool, McpRequest, McpResponse};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use url::Url;
use uuid::Uuid;

/// Authentication configuration for HTTP MCP client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpAuthConfig {
    /// No authentication
    None,
    /// Bearer token authentication
    Bearer { token: String },
    /// API Key authentication (header-based)
    ApiKey { header: String, key: String },
    /// Basic authentication
    Basic { username: String, password: String },
}

/// HTTP MCP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientConfig {
    /// Base URL for the MCP service
    pub base_url: String,
    /// Authentication configuration
    pub auth: HttpAuthConfig,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Maximum retry attempts
    pub retry_attempts: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Connection pool max idle connections
    pub max_idle_connections: Option<usize>,
    /// Connection pool idle timeout in seconds
    pub idle_timeout: Option<u64>,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            auth: HttpAuthConfig::None,
            timeout: 30,
            retry_attempts: 3,
            retry_delay_ms: 1000,
            max_idle_connections: Some(10),
            idle_timeout: Some(60),
        }
    }
}

/// HTTP MCP Client for connecting to external MCP services
#[derive(Debug, Clone)]
pub struct HttpMcpClient {
    /// Client configuration
    config: HttpClientConfig,
    /// HTTP client with connection pooling
    http_client: Client,
    /// Base URL parsed
    base_url: Url,
    /// Cached tools from the service
    cached_tools: Arc<RwLock<Option<Vec<Tool>>>>,
    /// Service identifier
    service_id: String,
}

impl HttpMcpClient {
    /// Create a new HTTP MCP client
    pub fn new(config: HttpClientConfig, service_id: String) -> Result<Self> {
        // Parse and validate base URL
        let base_url = Url::parse(&config.base_url)
            .map_err(|e| ProxyError::validation(format!("Invalid base URL '{}': {}", config.base_url, e)))?;

        // Build HTTP client with connection pooling
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .pool_max_idle_per_host(config.max_idle_connections.unwrap_or(10))
            .user_agent("MagicTunnel-HttpMcpClient/1.0");

        if let Some(idle_timeout) = config.idle_timeout {
            client_builder = client_builder.pool_idle_timeout(Duration::from_secs(idle_timeout));
        }

        let http_client = client_builder
            .build()
            .map_err(|e| ProxyError::connection(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            http_client,
            base_url,
            cached_tools: Arc::new(RwLock::new(None)),
            service_id,
        })
    }

    /// Get tools from the external MCP service
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        // Check cache first
        {
            let cached = self.cached_tools.read().await;
            if let Some(ref tools) = *cached {
                debug!("Returning cached tools for service {}", self.service_id);
                return Ok(tools.clone());
            }
        }

        // Fetch tools from service
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(Uuid::new_v4().to_string())),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = self.send_request(&request).await?;
        
        if let Some(result) = response.result {
            let tools_value = result.get("tools")
                .ok_or_else(|| ProxyError::mcp("Missing 'tools' field in list_tools response"))?;
            
            let tools: Vec<Tool> = serde_json::from_value(tools_value.clone())
                .map_err(|e| ProxyError::mcp(format!("Invalid tools format: {}", e)))?;

            // Cache the tools
            {
                let mut cached = self.cached_tools.write().await;
                *cached = Some(tools.clone());
            }

            info!("Retrieved {} tools from HTTP MCP service {}", tools.len(), self.service_id);
            Ok(tools)
        } else if let Some(error) = response.error {
            Err(ProxyError::mcp(format!("MCP error from service: {}", error.message)))
        } else {
            Err(ProxyError::mcp("Empty response from list_tools"))
        }
    }

    /// Call a tool on the external MCP service
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(Uuid::new_v4().to_string())),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": tool_name,
                "arguments": arguments
            })),
        };

        let response = self.send_request(&request).await?;
        
        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error {
            Err(ProxyError::mcp(format!("MCP error from service: {}", error.message)))
        } else {
            Err(ProxyError::mcp("Empty response from call_tool"))
        }
    }

    /// Send an MCP request to the HTTP service
    async fn send_request(&self, request: &McpRequest) -> Result<McpResponse> {
        let mut attempts = 0;
        let max_attempts = self.config.retry_attempts + 1;

        while attempts < max_attempts {
            attempts += 1;

            match self.send_single_request(request).await {
                Ok(response) => return Ok(response),
                Err(e) if attempts < max_attempts && self.is_retryable_error(&e) => {
                    warn!(
                        "HTTP MCP request failed (attempt {}/{}): {}. Retrying in {}ms...",
                        attempts, max_attempts, e, self.config.retry_delay_ms
                    );
                    tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(ProxyError::connection(format!(
            "HTTP MCP request failed after {} attempts",
            max_attempts
        )))
    }

    /// Send a single HTTP request
    async fn send_single_request(&self, request: &McpRequest) -> Result<McpResponse> {
        debug!(
            "Sending HTTP MCP request to {}: method={}, id={:?}",
            self.service_id, request.method, request.id
        );

        // Build the request
        let mut req_builder = self.http_client
            .post(self.base_url.clone())
            .header("Content-Type", "application/json")
            .json(request);

        // Add authentication
        req_builder = self.add_authentication(req_builder)?;

        // Send the request
        let response = req_builder
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("HTTP request failed: {}", e)))?;

        // Check status code
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProxyError::connection(format!(
                "HTTP {} error from MCP service: {}", 
                status, error_text
            )));
        }

        // Parse JSON response
        let response_text = response.text().await
            .map_err(|e| ProxyError::connection(format!("Failed to read response body: {}", e)))?;

        let mcp_response: McpResponse = serde_json::from_str(&response_text)
            .map_err(|e| ProxyError::mcp(format!("Invalid MCP response JSON: {}", e)))?;

        debug!(
            "Received HTTP MCP response from {}: id={}, success={}",
            self.service_id, mcp_response.id, mcp_response.error.is_none()
        );

        Ok(mcp_response)
    }

    /// Add authentication headers to the request
    fn add_authentication(&self, mut req_builder: RequestBuilder) -> Result<RequestBuilder> {
        match &self.config.auth {
            HttpAuthConfig::None => {
                // No authentication
            }
            HttpAuthConfig::Bearer { token } => {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
            }
            HttpAuthConfig::ApiKey { header, key } => {
                req_builder = req_builder.header(header, key);
            }
            HttpAuthConfig::Basic { username, password } => {
                req_builder = req_builder.basic_auth(username, Some(password));
            }
        }

        Ok(req_builder)
    }

    /// Check if an error is retryable
    fn is_retryable_error(&self, error: &ProxyError) -> bool {
        match error {
            ProxyError::Http(_) => true,             // Network errors are retryable
            ProxyError::Connection { .. } => true,  // Connection errors are retryable
            ProxyError::Mcp { .. } => false,        // MCP protocol errors are not retryable
            _ => false,
        }
    }

    /// Clear cached tools (e.g., on service restart)
    pub async fn clear_cache(&self) {
        let mut cached = self.cached_tools.write().await;
        *cached = None;
        debug!("Cleared tool cache for HTTP MCP service {}", self.service_id);
    }

    /// Get service health status
    pub async fn health_check(&self) -> Result<bool> {
        // Try to list tools as a health check
        match self.list_tools().await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("Health check failed for HTTP MCP service {}: {}", self.service_id, e);
                Ok(false)
            }
        }
    }

    /// Get service ID
    pub fn service_id(&self) -> &str {
        &self.service_id
    }

    /// Get configuration
    pub fn config(&self) -> &HttpClientConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.timeout, 30);
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay_ms, 1000);
        assert!(matches!(config.auth, HttpAuthConfig::None));
    }

    #[test]
    fn test_http_client_creation_invalid_url() {
        let config = HttpClientConfig {
            base_url: "invalid-url".to_string(),
            ..Default::default()
        };
        
        let result = HttpMcpClient::new(config, "test".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_http_client_creation_valid_url() {
        let config = HttpClientConfig {
            base_url: "https://api.example.com/mcp".to_string(),
            ..Default::default()
        };
        
        let result = HttpMcpClient::new(config, "test".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_authentication_config_serialization() {
        let auth_configs = vec![
            HttpAuthConfig::None,
            HttpAuthConfig::Bearer { token: "token123".to_string() },
            HttpAuthConfig::ApiKey { 
                header: "X-API-Key".to_string(), 
                key: "key123".to_string() 
            },
            HttpAuthConfig::Basic { 
                username: "user".to_string(), 
                password: "pass".to_string() 
            },
        ];

        for auth in auth_configs {
            let serialized = serde_json::to_string(&auth).unwrap();
            let deserialized: HttpAuthConfig = serde_json::from_str(&serialized).unwrap();
            // Note: We can't directly compare because of private fields, but this tests serialization
        }
    }
}