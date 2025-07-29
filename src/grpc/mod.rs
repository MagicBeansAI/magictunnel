pub mod server;

pub use server::McpGrpcServer;

// Re-export the generated protobuf types
pub use server::mcp_service_server;
