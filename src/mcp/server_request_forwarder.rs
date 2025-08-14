//! RequestForwarder Implementation for McpServer
//! 
//! This file contains the implementation of the RequestForwarder trait for McpServer
//! to enable bidirectional communication from external MCP servers.

use crate::mcp::server::McpServer;
use crate::mcp::request_forwarder::RequestForwarder;
use crate::error::Result;
use async_trait::async_trait;
use serde_json::json;
use tracing::debug;

#[async_trait]
impl RequestForwarder for McpServer {
    /// Forward a sampling request from an external MCP server to MagicTunnel Server
    /// 
    /// This enables the bidirectional communication flow where external MCP servers
    /// can request LLM assistance during tool execution.
    async fn forward_sampling_request(
        &self,
        mut request: crate::mcp::types::SamplingRequest,
        source_server: &str,
        original_client_id: &str,
    ) -> Result<crate::mcp::types::SamplingResponse> {
        debug!(
            "Forwarding sampling request from external server '{}' for original client '{}'",
            source_server, original_client_id
        );

        // Enrich the request with bidirectional metadata
        let mut metadata = request.metadata.unwrap_or_default();
        metadata.insert("source_external_server".to_string(), json!(source_server));
        metadata.insert("original_client_id".to_string(), json!(original_client_id));
        metadata.insert("bidirectional_request".to_string(), json!(true));
        metadata.insert("forwarded_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));
        request.metadata = Some(metadata);

        // Use the existing sampling request handler with the enriched request
        // This will apply the configured routing strategy (MagictunnelHandled, ClientForwarded, etc.)
        self.handle_sampling_request(request).await
    }

    /// Forward an elicitation request from an external MCP server to MagicTunnel Server
    async fn forward_elicitation_request(
        &self,
        mut request: crate::mcp::types::ElicitationRequest,
        source_server: &str,
        original_client_id: &str,
    ) -> Result<crate::mcp::types::ElicitationResponse> {
        debug!(
            "Forwarding elicitation request from external server '{}' for original client '{}'",
            source_server, original_client_id
        );

        // Enrich the request with bidirectional metadata
        let mut metadata = request.metadata.unwrap_or_default();
        metadata.insert("source_external_server".to_string(), json!(source_server));
        metadata.insert("original_client_id".to_string(), json!(original_client_id));
        metadata.insert("bidirectional_request".to_string(), json!(true));
        metadata.insert("forwarded_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));
        request.metadata = Some(metadata);

        // Use the existing elicitation request handler with the enriched request
        self.handle_elicitation_request(request).await
    }

    /// Forward a notification from an external MCP server to MagicTunnel Server
    /// 
    /// This enables external MCP servers to notify the MagicTunnel server when their
    /// capabilities change, which will then be forwarded to all connected clients.
    async fn forward_notification(
        &self,
        notification_method: &str,
        source_server: &str,
        original_client_id: &str,
    ) -> Result<()> {
        debug!(
            "Forwarding notification '{}' from external server '{}' for original client '{}'",
            notification_method, source_server, original_client_id
        );

        match notification_method {
            "notifications/tools/list_changed" => {
                // Trigger a tools list changed notification to all connected clients
                self.notification_manager().notify_tools_list_changed()?;
                debug!("Successfully forwarded tools/list_changed notification from external server: {}", source_server);
                Ok(())
            }
            "notifications/resources/list_changed" => {
                // Trigger a resources list changed notification to all connected clients
                self.notification_manager().notify_resources_list_changed()?;
                debug!("Successfully forwarded resources/list_changed notification from external server: {}", source_server);
                Ok(())
            }
            "notifications/prompts/list_changed" => {
                // Trigger a prompts list changed notification to all connected clients
                self.notification_manager().notify_prompts_list_changed()?;
                debug!("Successfully forwarded prompts/list_changed notification from external server: {}", source_server);
                Ok(())
            }
            _ => {
                debug!("Unknown notification method '{}' from external server '{}'", notification_method, source_server);
                Ok(()) // Don't fail for unknown notifications
            }
        }
    }

    /// Get the forwarder ID for this MCP Server
    fn forwarder_id(&self) -> &str {
        "magictunnel-server"
    }
}