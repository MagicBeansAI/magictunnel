//! Middleware system for agent routing with logging and metrics support

use crate::error::{ProxyError, Result};
use crate::mcp::ToolCall;
use crate::routing::types::{AgentResult, AgentType};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Context information passed to middleware
#[derive(Debug, Clone)]
pub struct MiddlewareContext {
    /// Unique execution ID for this routing operation
    pub execution_id: String,
    /// The tool call being executed
    pub tool_call: ToolCall,
    /// The agent type selected for execution
    pub agent_type: AgentType,
    /// When the execution started
    pub start_time: Instant,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

impl MiddlewareContext {
    /// Create a new middleware context
    pub fn new(tool_call: ToolCall, agent_type: AgentType) -> Self {
        Self {
            execution_id: Uuid::new_v4().to_string(),
            tool_call,
            agent_type,
            start_time: Instant::now(),
            metadata: HashMap::new(),
        }
    }

    /// Get the elapsed time since execution started
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Add metadata to the context
    pub fn add_metadata(&mut self, key: String, value: Value) {
        self.metadata.insert(key, value);
    }

    /// Get the agent type as a string
    pub fn agent_type_name(&self) -> &'static str {
        match &self.agent_type {
            AgentType::Subprocess { .. } => "subprocess",
            AgentType::Http { .. } => "http",
            AgentType::Llm { .. } => "llm",
            AgentType::WebSocket { .. } => "websocket",
            AgentType::Database { .. } => "database",
            AgentType::Grpc { .. } => "grpc",
            AgentType::Sse { .. } => "sse",
            AgentType::GraphQL { .. } => "graphql",
            AgentType::ExternalMcp { .. } => "external_mcp",
            AgentType::SmartDiscovery { .. } => "smart_discovery",
        }
    }
}

/// Trait for routing middleware
#[async_trait]
pub trait RouterMiddleware: Send + Sync {
    /// Called before agent execution
    async fn before_execution(&self, context: &MiddlewareContext) -> Result<()>;
    
    /// Called after successful agent execution
    async fn after_execution(&self, context: &MiddlewareContext, result: &AgentResult) -> Result<()>;
    
    /// Called when agent execution fails
    async fn on_error(&self, context: &MiddlewareContext, error: &ProxyError) -> Result<()>;
}

/// Logging middleware that logs all routing operations
pub struct LoggingMiddleware {
    /// Whether to log request/response data
    pub log_data: bool,
    /// Whether to log timing information
    pub log_timing: bool,
}

impl LoggingMiddleware {
    /// Create a new logging middleware with default settings
    pub fn new() -> Self {
        Self {
            log_data: true,
            log_timing: true,
        }
    }

    /// Create a logging middleware with custom settings
    pub fn with_config(log_data: bool, log_timing: bool) -> Self {
        Self { log_data, log_timing }
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RouterMiddleware for LoggingMiddleware {
    async fn before_execution(&self, context: &MiddlewareContext) -> Result<()> {
        // Log routing decision details
        debug!(
            execution_id = %context.execution_id,
            tool_name = %context.tool_call.name,
            agent_type = %context.agent_type_name(),
            routing_decision = "Agent selected based on tool definition routing config",
            "Routing: Agent selection completed, beginning execution"
        );

        if self.log_data {
            info!(
                execution_id = %context.execution_id,
                tool_name = %context.tool_call.name,
                agent_type = %context.agent_type_name(),
                arguments = ?context.tool_call.arguments,
                "Starting agent execution"
            );
        } else {
            info!(
                execution_id = %context.execution_id,
                tool_name = %context.tool_call.name,
                agent_type = %context.agent_type_name(),
                "Starting agent execution"
            );
        }
        Ok(())
    }

    async fn after_execution(&self, context: &MiddlewareContext, result: &AgentResult) -> Result<()> {
        let elapsed = context.elapsed();
        
        if result.success {
            if self.log_timing && self.log_data {
                info!(
                    execution_id = %context.execution_id,
                    tool_name = %context.tool_call.name,
                    agent_type = %context.agent_type_name(),
                    duration_ms = elapsed.as_millis(),
                    success = result.success,
                    data_size = result.data.as_ref().map(|d| d.to_string().len()).unwrap_or(0),
                    "Agent execution completed successfully"
                );
            } else if self.log_timing {
                info!(
                    execution_id = %context.execution_id,
                    tool_name = %context.tool_call.name,
                    agent_type = %context.agent_type_name(),
                    duration_ms = elapsed.as_millis(),
                    success = result.success,
                    "Agent execution completed successfully"
                );
            } else {
                info!(
                    execution_id = %context.execution_id,
                    tool_name = %context.tool_call.name,
                    agent_type = %context.agent_type_name(),
                    success = result.success,
                    "Agent execution completed successfully"
                );
            }
        } else {
            // Enhanced error logging with routing context
            let is_timeout = result.error.as_ref()
                .map(|e| e.contains("timeout") || e.contains("Timeout"))
                .unwrap_or(false);
            let is_retry_exhausted = result.error.as_ref()
                .map(|e| e.contains("retry") || e.contains("attempts"))
                .unwrap_or(false);

            warn!(
                execution_id = %context.execution_id,
                tool_name = %context.tool_call.name,
                agent_type = %context.agent_type_name(),
                duration_ms = elapsed.as_millis(),
                success = result.success,
                error = ?result.error,
                is_timeout = is_timeout,
                is_retry_exhausted = is_retry_exhausted,
                routing_context = "Agent execution failed - check agent configuration and availability",
                "Agent execution completed with failure"
            );

            // Additional debug logging for routing troubleshooting
            debug!(
                execution_id = %context.execution_id,
                tool_name = %context.tool_call.name,
                agent_type = %context.agent_type_name(),
                failure_analysis = if is_timeout { "Timeout detected - consider increasing timeout configuration" }
                                 else if is_retry_exhausted { "Retry attempts exhausted - check agent availability" }
                                 else { "General execution failure - check agent logs" },
                "Routing: Failure analysis for debugging"
            );
        }
        Ok(())
    }

    async fn on_error(&self, context: &MiddlewareContext, error: &ProxyError) -> Result<()> {
        let elapsed = context.elapsed();

        // Enhanced error categorization for routing debugging
        let error_str = error.to_string();
        let is_timeout_error = error_str.contains("timeout") || error_str.contains("Timeout");
        let is_connection_error = error_str.contains("connection") || error_str.contains("Connection");
        let is_auth_error = error_str.contains("auth") || error_str.contains("Auth") || error_str.contains("permission");
        let is_config_error = error_str.contains("config") || error_str.contains("Config") || error_str.contains("invalid");

        error!(
            execution_id = %context.execution_id,
            tool_name = %context.tool_call.name,
            agent_type = %context.agent_type_name(),
            duration_ms = elapsed.as_millis(),
            error = %error,
            error_category = if is_timeout_error { "timeout" }
                           else if is_connection_error { "connection" }
                           else if is_auth_error { "authentication" }
                           else if is_config_error { "configuration" }
                           else { "general" },
            routing_impact = "Agent execution failed before completion",
            "Agent execution failed with error"
        );

        // Detailed debugging information for routing troubleshooting
        debug!(
            execution_id = %context.execution_id,
            tool_name = %context.tool_call.name,
            agent_type = %context.agent_type_name(),
            error_analysis = if is_timeout_error { "Timeout error - check timeout configuration and agent responsiveness" }
                           else if is_connection_error { "Connection error - verify agent availability and network connectivity" }
                           else if is_auth_error { "Authentication error - check agent credentials and permissions" }
                           else if is_config_error { "Configuration error - verify agent routing configuration" }
                           else { "General error - check agent logs and configuration" },
            troubleshooting_hint = "Check agent-specific logs and configuration for detailed error information",
            "Routing: Error analysis for debugging"
        );

        Ok(())
    }
}

/// Metrics middleware that collects performance metrics
pub struct MetricsMiddleware {
    /// Metrics storage (in a real implementation, this would be a metrics backend)
    metrics: Arc<std::sync::Mutex<MetricsStorage>>,
}

#[derive(Debug, Default)]
struct MetricsStorage {
    /// Request counts by agent type
    request_counts: HashMap<String, u64>,
    /// Success counts by agent type
    success_counts: HashMap<String, u64>,
    /// Error counts by agent type
    error_counts: HashMap<String, u64>,
    /// Response times by agent type (in milliseconds)
    response_times: HashMap<String, Vec<u64>>,
    /// Timeout counts by agent type
    timeout_counts: HashMap<String, u64>,
    /// Retry counts by agent type
    retry_counts: HashMap<String, u64>,
    /// Tool-specific metrics
    tool_metrics: HashMap<String, ToolMetrics>,
    /// Total requests
    total_requests: u64,
    /// Total successes
    total_successes: u64,
    /// Total errors
    total_errors: u64,
    /// Total timeouts
    total_timeouts: u64,
    /// Total retries
    total_retries: u64,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
struct ToolMetrics {
    /// Request count for this specific tool
    requests: u64,
    /// Success count for this specific tool
    successes: u64,
    /// Error count for this specific tool
    errors: u64,
    /// Average response time for this tool
    avg_response_time_ms: f64,
    /// Last execution time
    last_execution: Option<std::time::SystemTime>,
}

impl MetricsMiddleware {
    /// Create a new metrics middleware
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(std::sync::Mutex::new(MetricsStorage::default())),
        }
    }

    /// Get current metrics as JSON
    pub fn get_metrics(&self) -> Value {
        let metrics = self.metrics.lock().unwrap();

        // Calculate average response times by agent type
        let mut avg_response_times = HashMap::new();
        for (agent_type, times) in &metrics.response_times {
            if !times.is_empty() {
                let avg = times.iter().sum::<u64>() / times.len() as u64;
                avg_response_times.insert(agent_type.clone(), avg);
            }
        }

        // Calculate success rates by agent type
        let mut success_rates = HashMap::new();
        for (agent_type, requests) in &metrics.request_counts {
            let successes = metrics.success_counts.get(agent_type).unwrap_or(&0);
            let rate = if *requests > 0 {
                (*successes as f64 / *requests as f64) * 100.0
            } else {
                0.0
            };
            success_rates.insert(agent_type.clone(), rate);
        }

        json!({
            "summary": {
                "total_requests": metrics.total_requests,
                "total_successes": metrics.total_successes,
                "total_errors": metrics.total_errors,
                "total_timeouts": metrics.total_timeouts,
                "total_retries": metrics.total_retries,
                "overall_success_rate": if metrics.total_requests > 0 {
                    (metrics.total_successes as f64 / metrics.total_requests as f64) * 100.0
                } else {
                    0.0
                }
            },
            "by_agent_type": {
                "request_counts": metrics.request_counts,
                "success_counts": metrics.success_counts,
                "error_counts": metrics.error_counts,
                "timeout_counts": metrics.timeout_counts,
                "retry_counts": metrics.retry_counts,
                "success_rates": success_rates,
                "avg_response_times_ms": avg_response_times
            },
            "by_tool": metrics.tool_metrics
        })
    }

    /// Get metrics for a specific agent type
    pub fn get_agent_metrics(&self, agent_type: &str) -> Value {
        let metrics = self.metrics.lock().unwrap();

        let requests = metrics.request_counts.get(agent_type).unwrap_or(&0);
        let successes = metrics.success_counts.get(agent_type).unwrap_or(&0);
        let errors = metrics.error_counts.get(agent_type).unwrap_or(&0);
        let timeouts = metrics.timeout_counts.get(agent_type).unwrap_or(&0);
        let retries = metrics.retry_counts.get(agent_type).unwrap_or(&0);

        let avg_response_time = if let Some(times) = metrics.response_times.get(agent_type) {
            if !times.is_empty() {
                times.iter().sum::<u64>() / times.len() as u64
            } else {
                0
            }
        } else {
            0
        };

        json!({
            "agent_type": agent_type,
            "requests": requests,
            "successes": successes,
            "errors": errors,
            "timeouts": timeouts,
            "retries": retries,
            "success_rate": if *requests > 0 {
                (*successes as f64 / *requests as f64) * 100.0
            } else {
                0.0
            },
            "avg_response_time_ms": avg_response_time
        })
    }

    /// Get metrics for a specific tool
    pub fn get_tool_metrics(&self, tool_name: &str) -> Value {
        let metrics = self.metrics.lock().unwrap();

        if let Some(tool_metrics) = metrics.tool_metrics.get(tool_name) {
            json!({
                "tool_name": tool_name,
                "requests": tool_metrics.requests,
                "successes": tool_metrics.successes,
                "errors": tool_metrics.errors,
                "success_rate": if tool_metrics.requests > 0 {
                    (tool_metrics.successes as f64 / tool_metrics.requests as f64) * 100.0
                } else {
                    0.0
                },
                "avg_response_time_ms": tool_metrics.avg_response_time_ms,
                "last_execution": tool_metrics.last_execution
            })
        } else {
            json!({
                "tool_name": tool_name,
                "requests": 0,
                "successes": 0,
                "errors": 0,
                "success_rate": 0.0,
                "avg_response_time_ms": 0.0,
                "last_execution": null
            })
        }
    }

    /// Reset all metrics
    pub fn reset_metrics(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        *metrics = MetricsStorage::default();
    }

    /// Track a retry attempt (called by retry executor)
    pub fn track_retry(&self, agent_type: &str, tool_name: &str) {
        let mut metrics = self.metrics.lock().unwrap();

        // Increment retry counts
        *metrics.retry_counts.entry(agent_type.to_string()).or_insert(0) += 1;
        metrics.total_retries += 1;

        debug!(
            agent_type = agent_type,
            tool_name = tool_name,
            "Metrics: Retry attempt tracked"
        );
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RouterMiddleware for MetricsMiddleware {
    async fn before_execution(&self, context: &MiddlewareContext) -> Result<()> {
        let mut metrics = self.metrics.lock().unwrap();
        let agent_type = context.agent_type_name().to_string();
        let tool_name = context.tool_call.name.clone();

        // Increment request counts by agent type
        *metrics.request_counts.entry(agent_type).or_insert(0) += 1;
        metrics.total_requests += 1;

        // Update tool-specific metrics
        let tool_metrics = metrics.tool_metrics.entry(tool_name.clone()).or_insert_with(ToolMetrics::default);
        tool_metrics.requests += 1;

        debug!(
            execution_id = %context.execution_id,
            tool_name = %tool_name,
            agent_type = %context.agent_type_name(),
            "Metrics: Request started"
        );
        Ok(())
    }

    async fn after_execution(&self, context: &MiddlewareContext, result: &AgentResult) -> Result<()> {
        let mut metrics = self.metrics.lock().unwrap();
        let agent_type = context.agent_type_name().to_string();
        let tool_name = context.tool_call.name.clone();
        let elapsed_ms = context.elapsed().as_millis() as u64;

        // Record response time by agent type
        metrics.response_times.entry(agent_type.clone()).or_insert_with(Vec::new).push(elapsed_ms);

        // Update tool-specific metrics
        if let Some(tool_metrics) = metrics.tool_metrics.get_mut(&tool_name) {
            // Update average response time
            let current_avg = tool_metrics.avg_response_time_ms;
            let request_count = tool_metrics.requests as f64;
            tool_metrics.avg_response_time_ms = if request_count > 1.0 {
                ((current_avg * (request_count - 1.0)) + elapsed_ms as f64) / request_count
            } else {
                elapsed_ms as f64
            };
            tool_metrics.last_execution = Some(std::time::SystemTime::now());
        }

        if result.success {
            *metrics.success_counts.entry(agent_type.clone()).or_insert(0) += 1;
            metrics.total_successes += 1;

            // Update tool success count
            if let Some(tool_metrics) = metrics.tool_metrics.get_mut(&tool_name) {
                tool_metrics.successes += 1;
            }
        } else {
            *metrics.error_counts.entry(agent_type.clone()).or_insert(0) += 1;
            metrics.total_errors += 1;

            // Update tool error count
            if let Some(tool_metrics) = metrics.tool_metrics.get_mut(&tool_name) {
                tool_metrics.errors += 1;
            }

            // Check if this was a timeout error
            let is_timeout = if let Some(data) = &result.data {
                data.to_string().to_lowercase().contains("timeout")
            } else {
                false
            };

            if is_timeout {
                *metrics.timeout_counts.entry(agent_type.clone()).or_insert(0) += 1;
                metrics.total_timeouts += 1;
            }
        }

        debug!(
            execution_id = %context.execution_id,
            tool_name = %tool_name,
            agent_type = %context.agent_type_name(),
            duration_ms = elapsed_ms,
            success = result.success,
            "Metrics: Request completed"
        );
        Ok(())
    }

    async fn on_error(&self, context: &MiddlewareContext, error: &ProxyError) -> Result<()> {
        let mut metrics = self.metrics.lock().unwrap();
        let agent_type = context.agent_type_name().to_string();
        let tool_name = context.tool_call.name.clone();
        let elapsed_ms = context.elapsed().as_millis() as u64;

        // Record response time even for errors
        metrics.response_times.entry(agent_type.clone()).or_insert_with(Vec::new).push(elapsed_ms);

        // Increment error counts by agent type
        *metrics.error_counts.entry(agent_type.clone()).or_insert(0) += 1;
        metrics.total_errors += 1;

        // Update tool-specific error metrics
        if let Some(tool_metrics) = metrics.tool_metrics.get_mut(&tool_name) {
            tool_metrics.errors += 1;

            // Update average response time
            let current_avg = tool_metrics.avg_response_time_ms;
            let request_count = tool_metrics.requests as f64;
            tool_metrics.avg_response_time_ms = if request_count > 1.0 {
                ((current_avg * (request_count - 1.0)) + elapsed_ms as f64) / request_count
            } else {
                elapsed_ms as f64
            };
            tool_metrics.last_execution = Some(std::time::SystemTime::now());
        }

        // Check for specific error types
        let error_str = error.to_string().to_lowercase();
        if error_str.contains("timeout") {
            *metrics.timeout_counts.entry(agent_type.clone()).or_insert(0) += 1;
            metrics.total_timeouts += 1;
        }

        if error_str.contains("retry") {
            *metrics.retry_counts.entry(agent_type.clone()).or_insert(0) += 1;
            metrics.total_retries += 1;
        }

        debug!(
            execution_id = %context.execution_id,
            tool_name = %tool_name,
            agent_type = %context.agent_type_name(),
            duration_ms = elapsed_ms,
            error_type = %error,
            "Metrics: Request failed with error"
        );
        Ok(())
    }
}

/// Chain of middleware that executes in order
pub struct MiddlewareChain {
    middleware: Vec<Arc<dyn RouterMiddleware>>,
}

impl MiddlewareChain {
    /// Create a new empty middleware chain
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    /// Add middleware to the chain
    pub fn add_middleware(mut self, middleware: Arc<dyn RouterMiddleware>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Execute before_execution for all middleware in the chain
    pub async fn before_execution(&self, context: &MiddlewareContext) -> Result<()> {
        for middleware in &self.middleware {
            middleware.before_execution(context).await?;
        }
        Ok(())
    }

    /// Execute after_execution for all middleware in the chain (in reverse order)
    pub async fn after_execution(&self, context: &MiddlewareContext, result: &AgentResult) -> Result<()> {
        for middleware in self.middleware.iter().rev() {
            middleware.after_execution(context, result).await?;
        }
        Ok(())
    }

    /// Execute on_error for all middleware in the chain (in reverse order)
    pub async fn on_error(&self, context: &MiddlewareContext, error: &ProxyError) -> Result<()> {
        for middleware in self.middleware.iter().rev() {
            middleware.on_error(context, error).await?;
        }
        Ok(())
    }

    /// Get the number of middleware in the chain
    pub fn len(&self) -> usize {
        self.middleware.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.middleware.is_empty()
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}
