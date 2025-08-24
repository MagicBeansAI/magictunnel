//! gRPC server implementation for MCP protocol

use std::sync::Arc;
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error};
use async_stream;

use crate::registry::RegistryService;
use crate::mcp::types::{ToolCall, ToolResult as McpToolResult, Tool as McpTool};
use crate::routing::{Router, types::RequestContext};

// Include the generated protobuf code
tonic::include_proto!("mcp");

/// gRPC server implementation for MCP protocol
pub struct McpGrpcServer {
    registry: Arc<RegistryService>,
    router: Arc<Router>,
}

impl McpGrpcServer {
    /// Create a new gRPC server with registry
    pub fn new(registry: Arc<RegistryService>) -> Self {
        let router = Arc::new(Router::new_enhanced());
        Self { registry, router }
    }
    
    /// Create a new gRPC server with registry and router
    pub fn with_router(registry: Arc<RegistryService>, router: Arc<Router>) -> Self {
        Self { registry, router }
    }
}

/// Conversion functions between MCP types and protobuf types
impl McpGrpcServer {
    /// Convert MCP Tool to protobuf Tool
    fn mcp_tool_to_proto(mcp_tool: &McpTool) -> Tool {
        // Convert annotations from MCP Tool if available
        let annotations = mcp_tool.annotations.as_ref().map(|tool_annotations| {
            ToolAnnotations {
                title: tool_annotations.title.clone(),
                read_only: tool_annotations.read_only_hint,
                destructive: tool_annotations.destructive_hint,
                idempotent: tool_annotations.idempotent_hint,
                open_world: tool_annotations.open_world_hint,
            }
        });
        
        Tool {
            name: mcp_tool.name.clone(),
            description: mcp_tool.description.clone().unwrap_or_else(|| format!("Tool: {}", mcp_tool.name)),
            input_schema: serde_json::to_string(&mcp_tool.input_schema).unwrap_or_else(|_| "{}".to_string()),
            annotations,
        }
    }

    /// Convert MCP ToolResult to protobuf ToolResult
    fn mcp_result_to_proto(mcp_result: &McpToolResult) -> ToolResult {
        ToolResult {
            success: mcp_result.success,
            data: mcp_result.data.as_ref().map(|d| serde_json::to_string(d).unwrap_or_else(|_| "null".to_string())),
            error: mcp_result.error.clone(),
            metadata: mcp_result.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string())),
        }
    }
}

/// Implement the gRPC service trait
#[tonic::async_trait]
impl mcp_service_server::McpService for McpGrpcServer {
    /// Stream type for CallTool responses
    type CallToolStream = Pin<Box<dyn Stream<Item = std::result::Result<CallToolResponse, Status>> + Send + 'static>>;

    /// Stream type for StreamMcp responses
    type StreamMcpStream = Pin<Box<dyn Stream<Item = std::result::Result<McpMessage, Status>> + Send + 'static>>;

    /// List available tools
    async fn list_tools(
        &self,
        _request: Request<ListToolsRequest>,
    ) -> std::result::Result<Response<ListToolsResponse>, Status> {
        debug!("gRPC list_tools called");

        let tool_names = self.registry.list_tools();
        let mut tools = Vec::new();

        for tool_name in tool_names {
            if let Some(tool_def) = self.registry.get_tool(&tool_name) {
                let mcp_tool = match crate::mcp::types::Tool::new(
                    tool_def.name().to_string(),
                    tool_def.description().to_string(),
                    tool_def.input_schema.clone(),
                ) {
                    Ok(tool) => tool,
                    Err(e) => {
                        error!("Failed to create MCP tool: {}", e);
                        continue;
                    }
                };

                let proto_tool = Self::mcp_tool_to_proto(&mcp_tool);
                tools.push(proto_tool);
            }
        }

        let response = ListToolsResponse { tools };
        Ok(Response::new(response))
    }

    /// Call a tool with streaming response
    async fn call_tool(
        &self,
        request: Request<CallToolRequest>,
    ) -> std::result::Result<Response<Self::CallToolStream>, Status> {
        debug!("gRPC call_tool called");

        let req = request.into_inner();
        let tool_call = ToolCall {
            name: req.name,
            arguments: serde_json::from_str(&req.arguments).unwrap_or_else(|_| serde_json::json!({})),
        };

        let registry = self.registry.clone();
        let router = self.router.clone();

        let stream = async_stream::stream! {
            // Get tool definition from registry
            if let Some(tool_def) = registry.get_tool(&tool_call.name) {
                // Create MCP tool for validation
                let mcp_tool = match crate::mcp::types::Tool::new(
                    tool_def.name().to_string(),
                    tool_def.description().to_string(),
                    tool_def.input_schema.clone(),
                ) {
                    Ok(tool) => tool,
                    Err(e) => {
                        let error_response = CallToolResponse {
                            response_type: Some(call_tool_response::ResponseType::Error(ToolError {
                                code: "VALIDATION_ERROR".to_string(),
                                message: format!("Tool validation failed: {}", e),
                                details: None,
                            })),
                        };
                        yield Ok(error_response);
                        return;
                    }
                };

                // Execute tool call using the router
                let context = RequestContext::with_session(
                    format!("grpc-session-{}", chrono::Utc::now().timestamp_millis()),
                    Some(format!("grpc-client-{}", chrono::Utc::now().timestamp_millis()))
                );

                let result = match router.route_with_context(&tool_call, &tool_def, &context).await {
                    Ok(agent_result) => {
                        debug!("gRPC tool execution successful for '{}': success={}", tool_call.name, agent_result.success);
                        
                        if agent_result.success {
                            let data = agent_result.data.unwrap_or(serde_json::json!({}));
                            let metadata = serde_json::json!({
                                "tool_name": tool_call.name,
                                "method": "grpc",
                                "routing_info": agent_result.metadata
                            });
                            McpToolResult::success_with_metadata(data, metadata)
                        } else {
                            let error_msg = agent_result.error.unwrap_or_else(|| "Tool execution failed".to_string());
                            let metadata = serde_json::json!({
                                "tool_name": tool_call.name,
                                "method": "grpc",
                                "routing_info": agent_result.metadata
                            });
                            McpToolResult::error_with_metadata(error_msg, metadata)
                        }
                    }
                    Err(e) => {
                        error!("gRPC tool execution failed for '{}': {}", tool_call.name, e);
                        McpToolResult::error_with_metadata(
                            format!("Tool execution failed: {}", e),
                            serde_json::json!({
                                "tool_name": tool_call.name,
                                "method": "grpc",
                                "error_type": "routing_error"
                            })
                        )
                    }
                };

                let proto_result = Self::mcp_result_to_proto(&result);
                let response = CallToolResponse {
                    response_type: Some(call_tool_response::ResponseType::Result(proto_result)),
                };

                yield Ok(response);
            } else {
                let error_response = CallToolResponse {
                    response_type: Some(call_tool_response::ResponseType::Error(ToolError {
                        code: "NOT_FOUND".to_string(),
                        message: format!("Tool '{}' not found", tool_call.name),
                        details: None,
                    })),
                };
                yield Ok(error_response);
            }
        };

        Ok(Response::new(Box::pin(stream) as Self::CallToolStream))
    }

    /// Bidirectional streaming for real-time communication
    async fn stream_mcp(
        &self,
        request: Request<Streaming<McpMessage>>,
    ) -> std::result::Result<Response<Self::StreamMcpStream>, Status> {
        debug!("gRPC stream_mcp called");

        let mut in_stream = request.into_inner();
        let _registry = self.registry.clone();

        // Create MCP server instance for handling requests
        let mcp_server = crate::mcp::McpServer::with_registry(_registry.clone());

        let stream = async_stream::stream! {
            while let Some(message) = in_stream.next().await {
                match message {
                    Ok(msg) => {
                        debug!("Received gRPC message: {:?}", msg);

                        // Check if this is a JSON-RPC message in the heartbeat content
                        if let Some(mcp_message::MessageType::Heartbeat(heartbeat)) = &msg.message_type {
                            // For now, treat heartbeat messages as potential JSON-RPC containers
                            // In a real implementation, you'd have a proper message field for JSON-RPC content
                            let json_content = format!("{{\"jsonrpc\":\"2.0\",\"id\":\"{}\",\"method\":\"tools/list\",\"params\":{{}}}}", msg.id);

                            match serde_json::from_str::<crate::mcp::types::McpRequest>(&json_content) {
                                Ok(mcp_request) => {
                                    // Use unified MCP handler
                                    match mcp_server.handle_mcp_request(mcp_request).await {
                                        Ok(Some(response)) => {
                                            let response_msg = McpMessage {
                                                id: msg.id,
                                                message_type: Some(mcp_message::MessageType::Heartbeat(HeartbeatMessage {
                                                    timestamp: chrono::Utc::now().timestamp(),
                                                    count: heartbeat.count + 1,
                                                })),
                                            };
                                            yield Ok(response_msg);
                                        }
                                        Ok(None) => {
                                            // No response needed for notifications
                                        }
                                        Err(e) => {
                                            error!("MCP request failed: {}", e);
                                            let error_response = McpMessage {
                                                id: msg.id,
                                                message_type: Some(mcp_message::MessageType::ToolResponse(CallToolResponse {
                                                    response_type: Some(call_tool_response::ResponseType::Error(ToolError {
                                                        code: "MCP_ERROR".to_string(),
                                                        message: format!("MCP request failed: {}", e),
                                                        details: None,
                                                    })),
                                                })),
                                            };
                                            yield Ok(error_response);
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Fallback to echo for non-JSON-RPC messages
                                    let response = McpMessage {
                                        id: msg.id,
                                        message_type: Some(mcp_message::MessageType::Heartbeat(HeartbeatMessage {
                                            timestamp: chrono::Utc::now().timestamp(),
                                            count: heartbeat.count + 1,
                                        })),
                                    };
                                    yield Ok(response);
                                }
                            }
                        } else {
                            // Echo other message types
                            yield Ok(msg);
                        }
                    }
                    Err(e) => {
                        error!("Error receiving gRPC message: {}", e);
                        let error_response = McpMessage {
                            id: "error".to_string(),
                            message_type: Some(mcp_message::MessageType::ToolResponse(CallToolResponse {
                                response_type: Some(call_tool_response::ResponseType::Error(ToolError {
                                    code: "STREAM_ERROR".to_string(),
                                    message: format!("Stream error: {}", e),
                                    details: None,
                                })),
                            })),
                        };
                        yield Ok(error_response);
                        break;
                    }
                }
            }
        };

        Ok(Response::new(Box::pin(stream) as Self::StreamMcpStream))
    }
}


