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
pub mod sampling;
pub mod elicitation;
pub mod roots;
pub mod request_generator;
pub mod prompt_generator;
pub mod resource_generator;
pub mod content_storage;
pub mod external_content_manager;
pub mod streamable_http;
pub mod cancellation;
pub mod progress;
pub mod tool_validation;

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
pub use sampling::{SamplingService, SamplingConfig};
pub use elicitation::{ElicitationService, ElicitationConfig};
pub use roots::{RootsService};
pub use request_generator::{RequestGeneratorService, RequestGeneratorConfig};
pub use prompt_generator::{PromptGeneratorService, PromptGenerationConfig, PromptGenerationRequest, PromptGenerationResponse, GeneratedPrompt, PromptType, is_external_mcp_tool, get_external_mcp_server_for_tool};
pub use resource_generator::{ResourceGeneratorService, ResourceGenerationConfig, ResourceGenerationRequest, ResourceGenerationResponse, GeneratedResource, ResourceType};
pub use content_storage::{ContentStorageService, ContentStorageConfig, StoredPrompt, StoredResource, StorageMetadata};
pub use external_content_manager::{ExternalContentManager, ExternalContentConfig};
pub use streamable_http::{StreamableHttpTransport, StreamableHttpConfig, StreamableHttpStats};
pub use cancellation::{CancellationManager, CancellationConfig, CancellationToken, CancellationReason, CancellationEvent, CancellationStats};
pub use progress::{ProgressTracker, ProgressConfig, ProgressSession, ProgressState, ProgressEvent, ProgressEventType, ProgressUpdate, SubOperation, SubOperationState, ProgressStats, ProgressGranularity};
pub use tool_validation::{RuntimeToolValidator, ValidationConfig as ToolValidationConfig, ValidationResult, SecurityClassification, SandboxPolicy, ValidationStats};
