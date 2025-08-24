//! Router implementation for directing tool calls to agents

use crate::error::Result;
use crate::mcp::ToolCall;
use crate::registry::ToolDefinition;
use crate::routing::{AgentRouter, DefaultAgentRouter, EnhancedRouterBuilder};
use crate::routing::types::{AgentResult, RequestContext};
use std::sync::Arc;
use tracing::debug;

/// Router that directs tool calls to appropriate agents
#[derive(Clone)]
pub struct Router {
    /// The underlying agent router implementation
    agent_router: Arc<dyn AgentRouter>,
}

impl Router {
    /// Create a new router with default agent router
    pub fn new() -> Self {
        Self {
            agent_router: Arc::new(DefaultAgentRouter::new()),
        }
    }

    /// Create a new router with external MCP integration
    pub fn with_external_mcp(
        external_mcp: Arc<tokio::sync::RwLock<crate::mcp::external_integration::ExternalMcpIntegration>>
    ) -> Self {
        Self {
            agent_router: Arc::new(DefaultAgentRouter::new().with_external_mcp(external_mcp)),
        }
    }

    /// Create a new router with registry service
    pub fn with_registry(registry: Arc<crate::registry::RegistryService>) -> Self {
        Self {
            agent_router: Arc::new(DefaultAgentRouter::new().with_registry(registry)),
        }
    }

    /// Create a new router with both external MCP integration and registry
    pub fn with_external_mcp_and_registry(
        external_mcp: Arc<tokio::sync::RwLock<crate::mcp::external_integration::ExternalMcpIntegration>>,
        registry: Arc<crate::registry::RegistryService>
    ) -> Self {
        Self {
            agent_router: Arc::new(DefaultAgentRouter::new()
                .with_external_mcp(external_mcp)
                .with_registry(registry)),
        }
    }

    /// Create a new router with registry and smart discovery
    pub fn with_registry_and_smart_discovery(
        registry: Arc<crate::registry::RegistryService>,
        smart_discovery: Arc<crate::discovery::SmartDiscoveryService>
    ) -> Self {
        Self {
            agent_router: Arc::new(DefaultAgentRouter::new()
                .with_registry(registry)
                .with_smart_discovery(smart_discovery)),
        }
    }

    /// Create a new router with external MCP, registry, and smart discovery
    pub fn with_external_mcp_registry_and_smart_discovery(
        external_mcp: Arc<tokio::sync::RwLock<crate::mcp::external_integration::ExternalMcpIntegration>>,
        registry: Arc<crate::registry::RegistryService>,
        smart_discovery: Arc<crate::discovery::SmartDiscoveryService>
    ) -> Self {
        Self {
            agent_router: Arc::new(DefaultAgentRouter::new()
                .with_external_mcp(external_mcp)
                .with_registry(registry)
                .with_smart_discovery(smart_discovery)),
        }
    }

    /// Create a new router with custom agent router
    pub fn with_agent_router(agent_router: Arc<dyn AgentRouter>) -> Self {
        Self { agent_router }
    }

    /// Route a tool call to the appropriate agent
    pub async fn route(&self, tool_call: &ToolCall, tool_def: &ToolDefinition) -> Result<AgentResult> {
        debug!("Routing tool call: {}", tool_call.name);
        self.agent_router.route(tool_call, tool_def).await
    }

    /// Route a tool call with authentication context
    pub async fn route_with_auth(
        &self,
        tool_call: &ToolCall,
        tool_def: &ToolDefinition,
        auth_context: Option<&crate::auth::AuthenticationContext>,
    ) -> Result<AgentResult> {
        debug!("Routing tool call with auth: {} (auth: {})", 
               tool_call.name, 
               auth_context.is_some());
        
        // Check if the underlying agent router supports authentication
        if let Some(auth_router) = self.agent_router.as_any().downcast_ref::<DefaultAgentRouter>() {
            auth_router.route_with_auth(tool_call, tool_def, auth_context).await
        } else {
            // Fall back to standard routing for routers that don't support auth
            debug!("Router doesn't support auth context, falling back to standard routing");
            self.agent_router.route(tool_call, tool_def).await
        }
    }

    /// Route a tool call with request context
    pub async fn route_with_context(
        &self,
        tool_call: &ToolCall,
        tool_def: &ToolDefinition,
        context: &RequestContext,
    ) -> Result<AgentResult> {
        debug!("Routing tool call with context: {} (session_id: {:?}, client_id: {:?})", 
               tool_call.name, context.session_id, context.client_id);
        
        // Check if the underlying agent router supports context
        self.agent_router.route_with_context(tool_call, tool_def, context).await
    }

    /// Create a new router with enhanced features (logging and metrics middleware)
    pub fn new_enhanced() -> Self {
        let enhanced_router = EnhancedRouterBuilder::new()
            .with_logging()
            .with_metrics()
            .build();

        Self {
            agent_router: Arc::new(enhanced_router),
        }
    }

    /// Create a new router with custom enhanced configuration
    pub fn new_enhanced_with_config(log_data: bool, log_timing: bool, enable_metrics: bool) -> Self {
        let mut builder = EnhancedRouterBuilder::new();

        if log_data || log_timing {
            builder = builder.with_logging_config(log_data, log_timing);
        }

        if enable_metrics {
            builder = builder.with_metrics();
        }

        let enhanced_router = builder.build();

        Self {
            agent_router: Arc::new(enhanced_router),
        }
    }

    /// Get metrics if the router is using an enhanced router with metrics middleware
    /// Note: This is a simplified implementation. For full metrics access, use EnhancedAgentRouter directly.
    pub fn get_metrics(&self) -> Option<serde_json::Value> {
        // For now, return None as we don't have a clean way to access metrics through the trait
        // In a production implementation, we'd add a metrics() method to the AgentRouter trait
        // or use a different architecture for metrics access
        None
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
