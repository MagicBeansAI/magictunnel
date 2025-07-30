//! MCP Network Clients
//!
//! This module contains client implementations for connecting to external MCP services
//! over various network protocols (HTTP, SSE, WebSocket, etc.).

pub mod http_client;
pub mod sse_client;

// Re-export main types
pub use http_client::{HttpMcpClient, HttpClientConfig, HttpAuthConfig};
pub use sse_client::{SseMcpClient, SseClientConfig, SseAuthConfig};

// Future client modules will be added here:
// pub mod websocket_client;
// pub mod grpc_client;