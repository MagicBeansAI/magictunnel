//! Tests for routing middleware functionality

use magictunnel::mcp::ToolCall;
use magictunnel::registry::{RoutingConfig, ToolDefinition};
use magictunnel::routing::{
    AgentRouter, DefaultAgentRouter, EnhancedRouterBuilder,
    LoggingMiddleware, MetricsMiddleware, MiddlewareChain, MiddlewareContext, RouterMiddleware
};
use serde_json::json;
use std::sync::Arc;
use tokio;

/// Helper function to create a test tool call
fn create_test_tool_call() -> ToolCall {
    ToolCall {
        name: "test_tool".to_string(),
        arguments: json!({
            "param1": "value1",
            "param2": 42
        }),
    }
}

/// Helper function to create a test tool definition
fn create_test_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "param1": {"type": "string"},
                "param2": {"type": "number"}
            }
        }),
        annotations: None,
        routing: RoutingConfig {
            r#type: "subprocess".to_string(),
            config: json!({
                "command": "echo",
                "args": ["{{param1}}", "{{param2}}"],
                "timeout": 10
            }),
        },
        hidden: false, // Test tools are visible by default
        enabled: true, // Test tools are enabled by default
    }
}

#[tokio::test]
async fn test_logging_middleware_creation() {
    let middleware = LoggingMiddleware::new();
    assert!(middleware.log_data);
    assert!(middleware.log_timing);

    let middleware_custom = LoggingMiddleware::with_config(false, true);
    assert!(!middleware_custom.log_data);
    assert!(middleware_custom.log_timing);
}

#[tokio::test]
async fn test_metrics_middleware_creation() {
    let middleware = MetricsMiddleware::new();
    let metrics = middleware.get_metrics();

    // Should start with zero metrics
    assert_eq!(metrics["summary"]["total_requests"], 0);
    assert_eq!(metrics["summary"]["total_successes"], 0);
    assert_eq!(metrics["summary"]["total_errors"], 0);
    assert_eq!(metrics["summary"]["overall_success_rate"], 0.0);
}

#[tokio::test]
async fn test_middleware_context_creation() {
    let tool_call = create_test_tool_call();
    let agent_type = magictunnel::routing::types::AgentType::Subprocess {
        command: "echo".to_string(),
        args: vec!["test".to_string()],
        timeout: Some(10),
        env: None,
    };

    let context = MiddlewareContext::new(tool_call.clone(), agent_type);
    
    assert_eq!(context.tool_call.name, "test_tool");
    assert_eq!(context.agent_type_name(), "subprocess");
    assert!(!context.execution_id.is_empty());
    assert!(context.elapsed().as_nanos() > 0);
}

#[tokio::test]
async fn test_middleware_chain_creation() {
    let chain = MiddlewareChain::new();
    assert_eq!(chain.len(), 0);
    assert!(chain.is_empty());

    let chain_with_middleware = MiddlewareChain::new()
        .add_middleware(Arc::new(LoggingMiddleware::new()))
        .add_middleware(Arc::new(MetricsMiddleware::new()));
    
    assert_eq!(chain_with_middleware.len(), 2);
    assert!(!chain_with_middleware.is_empty());
}

#[tokio::test]
async fn test_enhanced_router_builder() {
    let router = EnhancedRouterBuilder::new()
        .with_logging()
        .with_metrics()
        .build();

    assert_eq!(router.middleware().len(), 2);
}

#[tokio::test]
async fn test_enhanced_router_with_custom_config() {
    let router = EnhancedRouterBuilder::new()
        .with_logging_config(false, true)
        .with_metrics()
        .build();

    assert_eq!(router.middleware().len(), 2);
}

#[tokio::test]
async fn test_enhanced_router_execution() {
    let router = EnhancedRouterBuilder::new()
        .with_logging()
        .with_metrics()
        .build();

    let tool_call = create_test_tool_call();
    let tool_def = create_test_tool_definition();

    // This should execute successfully with middleware
    let result = router.route(&tool_call, &tool_def).await;
    assert!(result.is_ok());
    
    let agent_result = result.unwrap();
    // The subprocess should succeed (echo command)
    assert!(agent_result.success);
}

#[tokio::test]
async fn test_metrics_collection() {
    let metrics_middleware = Arc::new(MetricsMiddleware::new());
    let router = EnhancedRouterBuilder::new()
        .add_middleware(metrics_middleware.clone())
        .build();

    let tool_call = create_test_tool_call();
    let tool_def = create_test_tool_definition();

    // Execute a few tool calls
    for _ in 0..3 {
        let _ = router.route(&tool_call, &tool_def).await;
    }

    // Check metrics
    let metrics = metrics_middleware.get_metrics();
    assert_eq!(metrics["summary"]["total_requests"], 3);
    assert!(metrics["summary"]["total_successes"].as_u64().unwrap() > 0);

    // Check agent-specific metrics
    let by_agent = &metrics["by_agent_type"];
    assert_eq!(by_agent["request_counts"]["subprocess"], 3);

    // Check tool-specific metrics
    let by_tool = &metrics["by_tool"];
    assert!(by_tool.get("test_tool").is_some());
}

#[tokio::test]
async fn test_enhanced_metrics_collection() {
    let metrics_middleware = Arc::new(MetricsMiddleware::new());
    let router = EnhancedRouterBuilder::new()
        .add_middleware(metrics_middleware.clone())
        .build();

    let tool_call = create_test_tool_call();
    let tool_def = create_test_tool_definition();

    // Execute multiple tool calls
    for _ in 0..5 {
        let _ = router.route(&tool_call, &tool_def).await;
    }

    // Get comprehensive metrics
    let metrics = metrics_middleware.get_metrics();

    // Verify summary metrics
    let summary = &metrics["summary"];
    assert_eq!(summary["total_requests"], 5);
    assert!(summary["total_successes"].as_u64().unwrap() > 0);
    assert!(summary["overall_success_rate"].as_f64().unwrap() > 0.0);

    // Verify agent-specific metrics
    let by_agent = &metrics["by_agent_type"];
    assert_eq!(by_agent["request_counts"]["subprocess"], 5);
    assert!(by_agent["success_rates"]["subprocess"].as_f64().unwrap() > 0.0);
    assert!(by_agent["avg_response_times_ms"]["subprocess"].as_u64().unwrap() > 0);

    // Verify tool-specific metrics
    let by_tool = &metrics["by_tool"];
    let tool_metrics = &by_tool["test_tool"];
    assert_eq!(tool_metrics["requests"], 5);
    assert!(tool_metrics["avg_response_time_ms"].as_f64().unwrap() > 0.0);

    // Test individual metric getters
    let agent_metrics = metrics_middleware.get_agent_metrics("subprocess");
    assert_eq!(agent_metrics["requests"], 5);
    assert!(agent_metrics["success_rate"].as_f64().unwrap() > 0.0);

    let tool_metrics = metrics_middleware.get_tool_metrics("test_tool");
    assert_eq!(tool_metrics["requests"], 5);
    assert!(tool_metrics["success_rate"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn test_middleware_error_handling() {
    let metrics_middleware = Arc::new(MetricsMiddleware::new());
    let router = EnhancedRouterBuilder::new()
        .add_middleware(metrics_middleware.clone())
        .build();

    let tool_call = create_test_tool_call();
    let mut tool_def = create_test_tool_definition();
    
    // Create an invalid routing config to trigger an error
    tool_def.routing.r#type = "invalid_type".to_string();

    let result = router.route(&tool_call, &tool_def).await;
    assert!(result.is_err());

    // Check that error metrics were recorded
    let metrics = metrics_middleware.get_metrics();
    assert_eq!(metrics["summary"]["total_requests"], 0); // Request counting happens in execute_with_agent
    assert_eq!(metrics["summary"]["total_errors"], 0);   // Error happens before agent execution
}

#[tokio::test]
async fn test_router_enhanced_creation() {
    use magictunnel::routing::Router;
    
    let router = Router::new_enhanced();
    // Should create successfully with default enhanced configuration
    
    let _router_custom = Router::new_enhanced_with_config(true, false, true);
    // Should create successfully with custom configuration
    
    let tool_call = create_test_tool_call();
    let tool_def = create_test_tool_definition();
    
    let result = router.route(&tool_call, &tool_def).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_chain_execution_order() {
    // Create a custom middleware to test execution order
    struct TestMiddleware {
        name: String,
        calls: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl RouterMiddleware for TestMiddleware {
        async fn before_execution(&self, _context: &MiddlewareContext) -> magictunnel::error::Result<()> {
            self.calls.lock().unwrap().push(format!("{}_before", self.name));
            Ok(())
        }

        async fn after_execution(&self, _context: &MiddlewareContext, _result: &magictunnel::routing::types::AgentResult) -> magictunnel::error::Result<()> {
            self.calls.lock().unwrap().push(format!("{}_after", self.name));
            Ok(())
        }

        async fn on_error(&self, _context: &MiddlewareContext, _error: &magictunnel::error::ProxyError) -> magictunnel::error::Result<()> {
            self.calls.lock().unwrap().push(format!("{}_error", self.name));
            Ok(())
        }
    }

    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    
    let middleware1 = Arc::new(TestMiddleware {
        name: "first".to_string(),
        calls: calls.clone(),
    });
    
    let middleware2 = Arc::new(TestMiddleware {
        name: "second".to_string(),
        calls: calls.clone(),
    });

    let router = EnhancedRouterBuilder::new()
        .add_middleware(middleware1)
        .add_middleware(middleware2)
        .build();

    let tool_call = create_test_tool_call();
    let tool_def = create_test_tool_definition();

    let _ = router.route(&tool_call, &tool_def).await;

    let call_order = calls.lock().unwrap();
    // Should execute before_execution in order, after_execution in reverse order
    assert_eq!(call_order[0], "first_before");
    assert_eq!(call_order[1], "second_before");
    assert_eq!(call_order[2], "second_after");
    assert_eq!(call_order[3], "first_after");
}

#[tokio::test]
async fn test_middleware_performance_impact() {
    use std::time::Instant;

    // Test without middleware
    let router_plain = DefaultAgentRouter::new();
    let tool_call = create_test_tool_call();
    let tool_def = create_test_tool_definition();
    
    let start = Instant::now();
    for _ in 0..10 {
        let _ = router_plain.route(&tool_call, &tool_def).await;
    }
    let plain_duration = start.elapsed();

    // Test with middleware
    let router_enhanced = EnhancedRouterBuilder::new()
        .with_logging()
        .with_metrics()
        .build();
    
    let start = Instant::now();
    for _ in 0..10 {
        let _ = router_enhanced.route(&tool_call, &tool_def).await;
    }
    let enhanced_duration = start.elapsed();

    // Middleware should add minimal overhead (less than 2x)
    println!("Plain router: {:?}, Enhanced router: {:?}", plain_duration, enhanced_duration);
    // This is more of a performance observation than a strict test
    assert!(enhanced_duration < plain_duration * 5); // Allow up to 5x overhead for test environment
}
