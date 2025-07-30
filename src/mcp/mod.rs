//! MCP (Model Context Protocol) implementation
//!
//! This module contains the MCP server implementation that handles
//! protocol communication with MCP clients.

pub mod server;
pub mod client;
// Legacy remote MCP modules removed - replaced by external_* modules
pub mod external_process;
pub mod external_manager;
pub mod external_integration;
pub mod network_service_manager;
// Network clients for external MCP services
pub mod clients;
// Legacy local MCP modules removed - replaced by external_* modules
pub mod types;
pub mod resources;
pub mod prompts;
pub mod logging;
pub mod notifications;
pub mod errors;
pub mod session;
pub mod validation;
pub mod metrics;
pub mod health_checker;

// Test modules


pub use server::McpServer;
// Legacy integrations removed - use ExternalMcpIntegration instead
pub use external_integration::{ExternalMcpIntegration, ExternalMcpAgent};
pub use external_manager::ExternalMcpManager;
pub use external_process::ExternalMcpProcess;
pub use network_service_manager::{NetworkMcpServiceManager, NetworkMcpService};
// Network clients
pub use clients::{HttpMcpClient, HttpClientConfig, HttpAuthConfig, SseMcpClient, SseClientConfig, SseAuthConfig};
pub use types::*;
pub use resources::*;
pub use prompts::*;
pub use logging::*;
pub use notifications::*;
pub use errors::{McpError, McpErrorCode};
pub use session::{McpSessionManager, McpSession, SessionConfig, ClientInfo, SessionStats};
pub use validation::{McpMessageValidator, ValidationConfig};
pub use metrics::{McpMetricsCollector, McpServiceMetrics, HealthStatus, HealthCheckResult, McpMetricsSummary};
pub use health_checker::{McpHealthChecker, HealthCheckConfig};
