//! Request Forwarding Infrastructure for Bidirectional MCP Communication
//!
//! This module provides the unified interface for forwarding requests from external MCP clients
//! back to the MagicTunnel Server. This enables true bidirectional communication where external
//! MCP servers can send sampling/elicitation requests during tool execution.

use crate::error::Result;
use crate::mcp::types::sampling::{SamplingRequest, SamplingResponse};
use crate::mcp::types::elicitation::{ElicitationRequest, ElicitationResponse};
use async_trait::async_trait;
use std::sync::Arc;

/// Unified trait for forwarding requests from external MCP clients to MagicTunnel Server
/// 
/// This trait is implemented by the MCP Server to handle incoming requests from external
/// MCP servers during tool execution. It enables the bidirectional communication flow:
/// 
/// External MCP Server → ExternalMcpProcess/HttpMcpClient → RequestForwarder → MagicTunnel Server
#[async_trait]
pub trait RequestForwarder: Send + Sync {
    /// Forward a sampling request from an external MCP server to MagicTunnel Server
    /// 
    /// # Arguments
    /// * `request` - The sampling request from the external server
    /// * `source_server` - Name/ID of the external server that sent the request
    /// * `original_client_id` - ID of the original client (e.g., "claude-desktop-abc123")
    /// 
    /// # Returns
    /// The sampling response to send back to the external server
    async fn forward_sampling_request(
        &self,
        request: SamplingRequest,
        source_server: &str,
        original_client_id: &str,
    ) -> Result<SamplingResponse>;

    /// Forward an elicitation request from an external MCP server to MagicTunnel Server
    /// 
    /// # Arguments
    /// * `request` - The elicitation request from the external server
    /// * `source_server` - Name/ID of the external server that sent the request
    /// * `original_client_id` - ID of the original client (e.g., "claude-desktop-abc123")
    /// 
    /// # Returns
    /// The elicitation response to send back to the external server
    async fn forward_elicitation_request(
        &self,
        request: ElicitationRequest,
        source_server: &str,
        original_client_id: &str,
    ) -> Result<ElicitationResponse>;

    /// Forward a notification from an external MCP server to MagicTunnel Server
    /// 
    /// # Arguments
    /// * `notification_method` - The notification method (e.g., "notifications/tools/list_changed")
    /// * `source_server` - Name/ID of the external server that sent the notification
    /// * `original_client_id` - ID of the original client (e.g., "claude-desktop-abc123")
    /// 
    /// # Returns
    /// Result indicating success or failure of forwarding
    async fn forward_notification(
        &self,
        notification_method: &str,
        source_server: &str,
        original_client_id: &str,
    ) -> Result<()>;

    /// Get the name/ID of this request forwarder for logging purposes
    fn forwarder_id(&self) -> &str {
        "request_forwarder"
    }
}

/// Arc-wrapped RequestForwarder for shared ownership across async tasks
pub type SharedRequestForwarder = Arc<dyn RequestForwarder>;

/// Helper trait for external MCP clients to set up request forwarding
#[async_trait]
pub trait ExternalMcpClient: Send + Sync {
    /// Set the request forwarder for bidirectional communication
    /// 
    /// This should be called after creating the external MCP client to enable
    /// bidirectional communication. The forwarder will be used to send incoming
    /// requests from the external server back to MagicTunnel.
    async fn set_request_forwarder(&mut self, forwarder: SharedRequestForwarder) -> Result<()>;

    /// Get the server name/ID for this external client
    fn server_name(&self) -> &str;

    /// Check if the client supports bidirectional communication
    fn supports_bidirectional(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::types::sampling::*;
    use crate::mcp::types::elicitation::*;
    use serde_json::json;

    struct MockRequestForwarder {
        id: String,
    }

    #[async_trait]
    impl RequestForwarder for MockRequestForwarder {
        async fn forward_sampling_request(
            &self,
            request: SamplingRequest,
            source_server: &str,
            original_client_id: &str,
        ) -> Result<SamplingResponse> {
            // Mock implementation for testing
            Ok(SamplingResponse {
                message: SamplingMessage {
                    role: SamplingMessageRole::Assistant,
                    content: SamplingContent::Text(format!(
                        "Mock response from {} for client {} via server {}",
                        self.id, original_client_id, source_server
                    )),
                    name: Some("MockForwarder".to_string()),
                    metadata: None,
                },
                model: "mock-model".to_string(),
                stop_reason: SamplingStopReason::EndTurn,
                usage: None,
                metadata: Some([
                    ("mock_forwarder".to_string(), json!(true)),
                    ("source_server".to_string(), json!(source_server)),
                    ("original_client".to_string(), json!(original_client_id)),
                ].into_iter().collect()),
            })
        }

        async fn forward_elicitation_request(
            &self,
            _request: ElicitationRequest,
            source_server: &str,
            original_client_id: &str,
        ) -> Result<ElicitationResponse> {
            // Mock implementation for testing
            Ok(ElicitationResponse {
                action: crate::mcp::types::elicitation::ElicitationAction::Accept,
                data: Some(json!({
                    "mock_response": true,
                    "source_server": source_server,
                    "original_client": original_client_id
                })),
                reason: Some(format!("Mock elicitation from {}", self.id)),
                metadata: Some([
                    ("mock_forwarder".to_string(), json!(true)),
                ].into_iter().collect()),
                timestamp: Some(chrono::Utc::now()),
            })
        }

        async fn forward_notification(
            &self,
            notification_method: &str,
            source_server: &str,
            original_client_id: &str,
        ) -> Result<()> {
            // Mock implementation for testing - just log
            println!("Mock forwarding notification {} from {} for client {}", 
                     notification_method, source_server, original_client_id);
            Ok(())
        }

        fn forwarder_id(&self) -> &str {
            &self.id
        }
    }

    #[tokio::test]
    async fn test_request_forwarder_sampling() {
        let forwarder = MockRequestForwarder {
            id: "test-forwarder".to_string(),
        };

        let request = SamplingRequest {
            system_prompt: Some("Test system prompt".to_string()),
            messages: vec![SamplingMessage {
                role: SamplingMessageRole::User,
                content: SamplingContent::Text("Test message".to_string()),
                name: None,
                metadata: None,
            }],
            model_preferences: None,
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: None,
            stop: None,
            metadata: None,
        };

        let response = forwarder
            .forward_sampling_request(request, "test-server", "claude-desktop-123")
            .await
            .unwrap();

        assert_eq!(response.model, "mock-model");
        assert!(response.metadata.is_some());
        
        let metadata = response.metadata.unwrap();
        assert_eq!(metadata.get("source_server").unwrap(), "test-server");
        assert_eq!(metadata.get("original_client").unwrap(), "claude-desktop-123");
    }

    #[tokio::test]
    async fn test_request_forwarder_elicitation() {
        let forwarder = MockRequestForwarder {
            id: "test-forwarder".to_string(),
        };

        let request = ElicitationRequest {
            message: "Test elicitation request".to_string(),
            requested_schema: json!({
                "type": "object",
                "properties": {
                    "test": {"type": "string"}
                }
            }),
            context: Some(crate::mcp::types::elicitation::ElicitationContext {
                source: Some("test-tool".to_string()),
                reason: Some("Testing elicitation".to_string()),
                usage: Some("test-usage".to_string()),
                retention: Some("temporary".to_string()),
                privacy_level: Some(crate::mcp::types::elicitation::ElicitationPrivacyLevel::Public),
            }),
            timeout_seconds: Some(30),
            priority: Some(crate::mcp::types::elicitation::ElicitationPriority::Normal),
            metadata: None,
        };

        let response = forwarder
            .forward_elicitation_request(request, "test-server", "claude-desktop-123")
            .await
            .unwrap();

        assert!(matches!(response.action, crate::mcp::types::elicitation::ElicitationAction::Accept));
        assert!(response.data.is_some());
        assert!(response.reason.is_some());
        assert!(response.metadata.is_some());
    }

    #[tokio::test]
    async fn test_forwarder_id() {
        let forwarder = MockRequestForwarder {
            id: "custom-id".to_string(),
        };

        assert_eq!(forwarder.forwarder_id(), "custom-id");
    }
}