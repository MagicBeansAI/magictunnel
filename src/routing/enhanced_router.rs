//! Enhanced agent router with middleware support

use crate::error::Result;
use crate::mcp::ToolCall;
use crate::registry::ToolDefinition;
use crate::routing::{AgentRouter, DefaultAgentRouter};
use crate::routing::middleware::{MiddlewareChain, MiddlewareContext};
use crate::routing::retry::{RetryExecutor, RetryConfig};
use crate::routing::timeout::TimeoutConfig;
use crate::routing::types::{AgentResult, AgentType};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error};

/// Enhanced agent router that supports middleware, retry logic, and centralized timeout configuration
pub struct EnhancedAgentRouter {
    /// The underlying agent router
    inner: Arc<dyn AgentRouter>,
    /// Middleware chain
    middleware: MiddlewareChain,
    /// Retry executor for failed operations
    retry_executor: RetryExecutor,
    /// Centralized timeout configuration
    timeout_config: TimeoutConfig,
}

impl EnhancedAgentRouter {
    /// Create a new enhanced router with default agent router
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DefaultAgentRouter::new()),
            middleware: MiddlewareChain::new(),
            retry_executor: RetryExecutor::new(),
            timeout_config: TimeoutConfig::default(),
        }
    }

    /// Create a new enhanced router with custom agent router
    pub fn with_agent_router(agent_router: Arc<dyn AgentRouter>) -> Self {
        Self {
            inner: agent_router,
            middleware: MiddlewareChain::new(),
            retry_executor: RetryExecutor::new(),
            timeout_config: TimeoutConfig::default(),
        }
    }

    /// Create a new enhanced router with middleware chain
    pub fn with_middleware(middleware: MiddlewareChain) -> Self {
        Self {
            inner: Arc::new(DefaultAgentRouter::new()),
            middleware,
            retry_executor: RetryExecutor::new(),
            timeout_config: TimeoutConfig::default(),
        }
    }

    /// Create a new enhanced router with custom agent router and middleware
    pub fn with_agent_router_and_middleware(
        agent_router: Arc<dyn AgentRouter>,
        middleware: MiddlewareChain,
    ) -> Self {
        Self {
            inner: agent_router,
            middleware,
            retry_executor: RetryExecutor::new(),
            timeout_config: TimeoutConfig::default(),
        }
    }

    /// Create a new enhanced router with custom retry configuration
    pub fn with_retry_config(
        agent_router: Arc<dyn AgentRouter>,
        middleware: MiddlewareChain,
        retry_config: RetryConfig,
    ) -> Self {
        Self {
            inner: agent_router,
            middleware,
            retry_executor: RetryExecutor::with_config(retry_config),
            timeout_config: TimeoutConfig::default(),
        }
    }

    /// Create a new enhanced router with custom timeout configuration
    pub fn with_timeout_config(
        agent_router: Arc<dyn AgentRouter>,
        middleware: MiddlewareChain,
        timeout_config: TimeoutConfig,
    ) -> Self {
        Self {
            inner: agent_router,
            middleware,
            retry_executor: RetryExecutor::new(),
            timeout_config,
        }
    }

    /// Create a new enhanced router with both retry and timeout configuration
    pub fn with_retry_and_timeout_config(
        agent_router: Arc<dyn AgentRouter>,
        middleware: MiddlewareChain,
        retry_config: RetryConfig,
        timeout_config: TimeoutConfig,
    ) -> Self {
        Self {
            inner: agent_router,
            middleware,
            retry_executor: RetryExecutor::with_config(retry_config),
            timeout_config,
        }
    }

    /// Add middleware to the router
    pub fn add_middleware(mut self, middleware: Arc<dyn crate::routing::middleware::RouterMiddleware>) -> Self {
        self.middleware = self.middleware.add_middleware(middleware);
        self
    }

    /// Get a reference to the middleware chain
    pub fn middleware(&self) -> &MiddlewareChain {
        &self.middleware
    }

    /// Get the timeout configuration
    pub fn timeout_config(&self) -> &TimeoutConfig {
        &self.timeout_config
    }

    /// Apply centralized timeout configuration to an agent type
    /// Priority: existing_timeout (tool override) > centralized_config > default
    fn apply_timeout_config(&self, agent: &AgentType) -> AgentType {

        match agent {
            AgentType::Subprocess { command, args, timeout, env } => {
                let final_timeout = if timeout.is_some() {
                    *timeout // Keep existing timeout (tool override)
                } else {
                    Some(self.timeout_config.get_timeout_secs("subprocess", None))
                };

                AgentType::Subprocess {
                    command: command.clone(),
                    args: args.clone(),
                    timeout: final_timeout,
                    env: env.clone(),
                }
            }

            AgentType::Http { method, url, headers, timeout } => {
                let final_timeout = if timeout.is_some() {
                    *timeout // Keep existing timeout (tool override)
                } else {
                    Some(self.timeout_config.get_timeout_secs("http", None))
                };

                AgentType::Http {
                    method: method.clone(),
                    url: url.clone(),
                    headers: headers.clone(),
                    timeout: final_timeout,
                }
            }

            AgentType::Llm { provider, model, api_key, base_url, timeout } => {
                let final_timeout = if timeout.is_some() {
                    *timeout // Keep existing timeout (tool override)
                } else {
                    Some(self.timeout_config.get_timeout_secs("llm", None))
                };

                AgentType::Llm {
                    provider: provider.clone(),
                    model: model.clone(),
                    api_key: api_key.clone(),
                    base_url: base_url.clone(),
                    timeout: final_timeout,
                }
            }

            AgentType::WebSocket { url, headers } => {
                // WebSocket doesn't have timeout field in the enum, but we could add it
                // For now, keep as-is since WebSocket uses hardcoded timeouts
                AgentType::WebSocket {
                    url: url.clone(),
                    headers: headers.clone(),
                }
            }

            AgentType::Database { db_type, connection_string, query, timeout } => {
                let final_timeout = if timeout.is_some() {
                    *timeout // Keep existing timeout (tool override)
                } else {
                    Some(self.timeout_config.get_timeout_secs("database", None))
                };

                AgentType::Database {
                    db_type: db_type.clone(),
                    connection_string: connection_string.clone(),
                    query: query.clone(),
                    timeout: final_timeout,
                }
            }

            AgentType::Grpc { endpoint, service, method, headers, timeout, request_body } => {
                let final_timeout = if timeout.is_some() {
                    *timeout // Keep existing timeout (tool override)
                } else {
                    Some(self.timeout_config.get_timeout_secs("grpc", None))
                };

                AgentType::Grpc {
                    endpoint: endpoint.clone(),
                    service: service.clone(),
                    method: method.clone(),
                    headers: headers.clone(),
                    timeout: final_timeout,
                    request_body: request_body.clone(),
                }
            }

            AgentType::Sse { url, headers, timeout, max_events, event_filter } => {
                let final_timeout = if timeout.is_some() {
                    *timeout // Keep existing timeout (tool override)
                } else {
                    Some(self.timeout_config.get_timeout_secs("sse", None))
                };

                AgentType::Sse {
                    url: url.clone(),
                    headers: headers.clone(),
                    timeout: final_timeout,
                    max_events: *max_events,
                    event_filter: event_filter.clone(),
                }
            }

            AgentType::GraphQL { endpoint, query, variables, headers, timeout, operation_name } => {
                let final_timeout = if timeout.is_some() {
                    *timeout // Keep existing timeout (tool override)
                } else {
                    Some(self.timeout_config.get_timeout_secs("graphql", None))
                };

                AgentType::GraphQL {
                    endpoint: endpoint.clone(),
                    query: query.clone(),
                    variables: variables.clone(),
                    headers: headers.clone(),
                    timeout: final_timeout,
                    operation_name: operation_name.clone(),
                }
            }

            AgentType::ExternalMcp { server_name, tool_name, timeout, mapping_metadata } => {
                let final_timeout = if timeout.is_some() {
                    *timeout // Keep existing timeout (tool override)
                } else {
                    Some(self.timeout_config.get_timeout_secs("external_mcp", None))
                };

                AgentType::ExternalMcp {
                    server_name: server_name.clone(),
                    tool_name: tool_name.clone(),
                    timeout: final_timeout,
                    mapping_metadata: mapping_metadata.clone(),
                }
            }

            AgentType::SmartDiscovery { enabled } => {
                // SmartDiscovery doesn't have timeout field, so keep as-is
                AgentType::SmartDiscovery {
                    enabled: *enabled,
                }
            }
        }
    }
}

impl Default for EnhancedAgentRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRouter for EnhancedAgentRouter {
    fn parse_routing_config(&self, routing: &crate::registry::RoutingConfig) -> Result<crate::routing::types::AgentType> {
        // Delegate to the inner router
        self.inner.parse_routing_config(routing)
    }

    async fn execute_with_agent(&self, tool_call: &ToolCall, agent: &crate::routing::types::AgentType) -> Result<AgentResult> {
        // Create middleware context
        let context = MiddlewareContext::new(tool_call.clone(), agent.clone());

        debug!(
            execution_id = %context.execution_id,
            tool_name = %tool_call.name,
            agent_type = %context.agent_type_name(),
            middleware_count = self.middleware.len(),
            "Enhanced router: Starting execution with middleware and retry logic"
        );

        // Execute before_execution middleware
        if let Err(e) = self.middleware.before_execution(&context).await {
            error!(
                execution_id = %context.execution_id,
                error = %e,
                "Middleware before_execution failed"
            );
            // Continue execution even if middleware fails
        }

        // Apply centralized timeout configuration to the agent
        let agent_with_timeout = self.apply_timeout_config(agent);

        // Execute the actual agent with retry logic and centralized timeout
        let result = self.retry_executor.execute_with_retry(
            &agent_with_timeout,
            &format!("tool_call_{}", tool_call.name),
            || async {
                self.inner.execute_with_agent(tool_call, &agent_with_timeout).await
            }
        ).await;

        match result {
            Ok(agent_result) => {
                // Execute after_execution middleware
                if let Err(e) = self.middleware.after_execution(&context, &agent_result).await {
                    error!(
                        execution_id = %context.execution_id,
                        error = %e,
                        "Middleware after_execution failed"
                    );
                    // Continue and return the result even if middleware fails
                }

                debug!(
                    execution_id = %context.execution_id,
                    success = agent_result.success,
                    duration_ms = context.elapsed().as_millis(),
                    "Enhanced router: Execution completed successfully"
                );

                Ok(agent_result)
            }
            Err(e) => {
                // Execute on_error middleware
                if let Err(middleware_error) = self.middleware.on_error(&context, &e).await {
                    error!(
                        execution_id = %context.execution_id,
                        middleware_error = %middleware_error,
                        original_error = %e,
                        "Middleware on_error failed"
                    );
                    // Continue and return the original error
                }

                error!(
                    execution_id = %context.execution_id,
                    error = %e,
                    duration_ms = context.elapsed().as_millis(),
                    "Enhanced router: Execution failed after retry attempts"
                );

                Err(e)
            }
        }
    }

    async fn route(&self, tool_call: &ToolCall, tool_def: &ToolDefinition) -> Result<AgentResult> {
        debug!("Enhanced router: Routing tool call: {}", tool_call.name);
        
        // Parse routing configuration into agent type
        let agent = self.parse_routing_config(&tool_def.routing)?;
        
        // Execute the tool call with the selected agent (this will use middleware)
        self.execute_with_agent(tool_call, &agent).await
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Builder for creating enhanced routers with middleware, retry, and timeout configuration
pub struct EnhancedRouterBuilder {
    agent_router: Option<Arc<dyn AgentRouter>>,
    middleware: MiddlewareChain,
    retry_config: Option<RetryConfig>,
    timeout_config: Option<TimeoutConfig>,
}

impl EnhancedRouterBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            agent_router: None,
            middleware: MiddlewareChain::new(),
            retry_config: None,
            timeout_config: None,
        }
    }

    /// Set the agent router
    pub fn with_agent_router(mut self, agent_router: Arc<dyn AgentRouter>) -> Self {
        self.agent_router = Some(agent_router);
        self
    }

    /// Add middleware to the builder
    pub fn add_middleware(mut self, middleware: Arc<dyn crate::routing::middleware::RouterMiddleware>) -> Self {
        self.middleware = self.middleware.add_middleware(middleware);
        self
    }

    /// Add logging middleware with default settings
    pub fn with_logging(self) -> Self {
        self.add_middleware(Arc::new(crate::routing::middleware::LoggingMiddleware::new()))
    }

    /// Add logging middleware with custom settings
    pub fn with_logging_config(self, log_data: bool, log_timing: bool) -> Self {
        self.add_middleware(Arc::new(crate::routing::middleware::LoggingMiddleware::with_config(log_data, log_timing)))
    }

    /// Add metrics middleware
    pub fn with_metrics(self) -> Self {
        self.add_middleware(Arc::new(crate::routing::middleware::MetricsMiddleware::new()))
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = Some(retry_config);
        self
    }

    /// Set timeout configuration
    pub fn with_timeout_config(mut self, timeout_config: TimeoutConfig) -> Self {
        self.timeout_config = Some(timeout_config);
        self
    }

    /// Set timeout configuration using builder pattern
    pub fn with_timeout_builder<F>(mut self, builder_fn: F) -> Self
    where
        F: FnOnce(crate::routing::timeout::TimeoutConfigBuilder) -> std::result::Result<TimeoutConfig, String>,
    {
        match builder_fn(TimeoutConfig::builder()) {
            Ok(timeout_config) => {
                self.timeout_config = Some(timeout_config);
                self
            }
            Err(_) => {
                // Use default timeout config if builder fails
                self.timeout_config = Some(TimeoutConfig::default());
                self
            }
        }
    }

    /// Build the enhanced router
    pub fn build(self) -> EnhancedAgentRouter {
        let agent_router = self.agent_router.unwrap_or_else(|| Arc::new(DefaultAgentRouter::new()));
        let retry_config = self.retry_config.unwrap_or_default();
        let timeout_config = self.timeout_config.unwrap_or_default();

        EnhancedAgentRouter::with_retry_and_timeout_config(
            agent_router,
            self.middleware,
            retry_config,
            timeout_config,
        )
    }
}

impl Default for EnhancedRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::middleware::{LoggingMiddleware, MetricsMiddleware};

    #[test]
    fn test_enhanced_router_builder() {
        let router = EnhancedRouterBuilder::new()
            .with_logging()
            .with_metrics()
            .build();

        assert_eq!(router.middleware().len(), 2);
    }

    #[test]
    fn test_enhanced_router_creation() {
        let router = EnhancedAgentRouter::new();
        assert_eq!(router.middleware().len(), 0);

        let router_with_middleware = EnhancedAgentRouter::new()
            .add_middleware(Arc::new(LoggingMiddleware::new()))
            .add_middleware(Arc::new(MetricsMiddleware::new()));

        assert_eq!(router_with_middleware.middleware().len(), 2);
    }

    #[test]
    fn test_enhanced_router_with_retry_config() {
        use crate::routing::retry::{RetryConfig, RetryPolicy};
        use std::collections::HashMap;

        // Create custom retry config
        let mut per_agent_type = HashMap::new();
        per_agent_type.insert(
            "http".to_string(),
            RetryPolicy {
                max_attempts: 5,
                initial_delay_ms: 100,
                max_delay_ms: 2000,
                backoff_multiplier: 2.0,
                use_jitter: true,
            },
        );

        let retry_config = RetryConfig {
            default: RetryPolicy {
                max_attempts: 2,
                initial_delay_ms: 50,
                max_delay_ms: 1000,
                backoff_multiplier: 1.5,
                use_jitter: false,
            },
            per_agent_type,
        };

        // Build router with retry config
        let router = EnhancedRouterBuilder::new()
            .with_retry_config(retry_config)
            .build();

        // Should have retry executor configured
        assert_eq!(router.middleware().len(), 0); // No middleware added
    }

    #[test]
    fn test_enhanced_router_with_timeout_config() {
        use crate::routing::timeout::TimeoutConfig;
        use std::collections::HashMap;

        // Create custom timeout config
        let mut per_agent_type = HashMap::new();
        per_agent_type.insert("subprocess".to_string(), 45);
        per_agent_type.insert("http".to_string(), 60);
        per_agent_type.insert("llm".to_string(), 120);

        let timeout_config = TimeoutConfig {
            default_timeout_secs: 40,
            per_agent_type,
            max_timeout_secs: 600,
        };

        // Build router with timeout config
        let router = EnhancedRouterBuilder::new()
            .with_timeout_config(timeout_config.clone())
            .build();

        // Should have timeout config configured
        assert_eq!(router.timeout_config().default_timeout_secs, 40);
        assert_eq!(router.timeout_config().max_timeout_secs, 600);
        assert_eq!(router.timeout_config().get_timeout_secs("subprocess", None), 45);
        assert_eq!(router.timeout_config().get_timeout_secs("http", None), 60);
        assert_eq!(router.timeout_config().get_timeout_secs("llm", None), 120);
        assert_eq!(router.timeout_config().get_timeout_secs("websocket", None), 40); // Uses default
    }

    #[test]
    fn test_enhanced_router_with_timeout_builder() {
        // Build router with timeout builder
        let router = EnhancedRouterBuilder::new()
            .with_timeout_builder(|builder| {
                builder
                    .default_timeout(35)
                    .max_timeout(500)
                    .subprocess_timeout(25)
                    .http_timeout(50)
                    .llm_timeout(100)
                    .build()
            })
            .build();

        // Should have timeout config configured
        assert_eq!(router.timeout_config().default_timeout_secs, 35);
        assert_eq!(router.timeout_config().max_timeout_secs, 500);
        assert_eq!(router.timeout_config().get_timeout_secs("subprocess", None), 25);
        assert_eq!(router.timeout_config().get_timeout_secs("http", None), 50);
        assert_eq!(router.timeout_config().get_timeout_secs("llm", None), 100);
    }

    #[test]
    fn test_apply_timeout_config() {
        use crate::routing::timeout::TimeoutConfig;
        use std::collections::HashMap;

        // Create router with custom timeout config
        let mut per_agent_type = HashMap::new();
        per_agent_type.insert("subprocess".to_string(), 45);
        per_agent_type.insert("http".to_string(), 60);

        let timeout_config = TimeoutConfig {
            default_timeout_secs: 40,
            per_agent_type,
            max_timeout_secs: 600,
        };

        let router = EnhancedRouterBuilder::new()
            .with_timeout_config(timeout_config)
            .build();

        // Test subprocess agent without existing timeout
        let subprocess_agent = AgentType::Subprocess {
            command: "echo".to_string(),
            args: vec!["test".to_string()],
            timeout: None,
            env: None,
        };

        let modified_agent = router.apply_timeout_config(&subprocess_agent);
        match modified_agent {
            AgentType::Subprocess { timeout, .. } => {
                assert_eq!(timeout, Some(45)); // Should use configured timeout
            }
            _ => panic!("Expected Subprocess agent"),
        }

        // Test subprocess agent with existing timeout (should preserve it)
        let subprocess_agent_with_timeout = AgentType::Subprocess {
            command: "echo".to_string(),
            args: vec!["test".to_string()],
            timeout: Some(99), // Tool override
            env: None,
        };

        let modified_agent = router.apply_timeout_config(&subprocess_agent_with_timeout);
        match modified_agent {
            AgentType::Subprocess { timeout, .. } => {
                assert_eq!(timeout, Some(99)); // Should preserve existing timeout
            }
            _ => panic!("Expected Subprocess agent"),
        }

        // Test HTTP agent
        let http_agent = AgentType::Http {
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            headers: None,
            timeout: None,
        };

        let modified_agent = router.apply_timeout_config(&http_agent);
        match modified_agent {
            AgentType::Http { timeout, .. } => {
                assert_eq!(timeout, Some(60)); // Should use configured timeout
            }
            _ => panic!("Expected Http agent"),
        }
    }
}
