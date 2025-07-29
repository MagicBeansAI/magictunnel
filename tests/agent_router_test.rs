//! Tests for the AgentRouter trait and implementations

use magictunnel::error::Result;
use magictunnel::mcp::{Tool, ToolCall};
use magictunnel::registry::{RoutingConfig, ToolDefinition};
use magictunnel::routing::{AgentRouter, DefaultAgentRouter, Router, AgentType};
use serde_json::json;
use std::sync::Arc;

/// Create a test tool call
fn create_test_tool_call(name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        name: name.to_string(),
        arguments,
    }
}

/// Create a test tool definition
fn create_test_tool_definition(routing_type: &str, config: serde_json::Value) -> ToolDefinition {
    let tool = Tool::new(
        "test_tool".to_string(),
        "Test tool".to_string(),
        json!({"type": "object"}),
    ).expect("Failed to create test tool");

    let routing = RoutingConfig::new(routing_type.to_string(), config);

    ToolDefinition::new(tool, routing).unwrap()
}

#[tokio::test]
async fn test_default_agent_router_creation() {
    let router = DefaultAgentRouter::new();
    // Should create without error
    assert!(true);
}

#[tokio::test]
async fn test_router_creation() {
    let router = Router::new();
    // Should create without error
    assert!(true);
    
    // Test with custom agent router
    let agent_router = Arc::new(DefaultAgentRouter::new());
    let custom_router = Router::with_agent_router(agent_router);
    // Should create without error
    assert!(true);
}

#[tokio::test]
async fn test_subprocess_routing_config_parsing() {
    let router = DefaultAgentRouter::new();
    
    let config = json!({
        "command": "echo",
        "args": ["hello", "world"],
        "timeout": 30
    });
    
    let routing = RoutingConfig::new("subprocess".to_string(), config);
    let result = router.parse_routing_config(&routing);
    
    assert!(result.is_ok());
    let agent_type = result.unwrap();
    
    match agent_type {
        AgentType::Subprocess { command, args, timeout, .. } => {
            assert_eq!(command, "echo");
            assert_eq!(args, vec!["hello", "world"]);
            assert_eq!(timeout, Some(30));
        }
        _ => panic!("Expected Subprocess agent type"),
    }
}

#[tokio::test]
async fn test_http_routing_config_parsing() {
    let router = DefaultAgentRouter::new();
    
    let config = json!({
        "method": "POST",
        "url": "https://api.example.com/test",
        "timeout": 60
    });
    
    let routing = RoutingConfig::new("http".to_string(), config);
    let result = router.parse_routing_config(&routing);
    
    assert!(result.is_ok());
    let agent_type = result.unwrap();
    
    match agent_type {
        AgentType::Http { method, url, timeout, .. } => {
            assert_eq!(method, "POST");
            assert_eq!(url, "https://api.example.com/test");
            assert_eq!(timeout, Some(60));
        }
        _ => panic!("Expected Http agent type"),
    }
}

#[tokio::test]
async fn test_llm_routing_config_parsing() {
    let router = DefaultAgentRouter::new();
    
    let config = json!({
        "provider": "openai",
        "model": "gpt-4",
        "api_key": "test-key",
        "timeout": 120
    });
    
    let routing = RoutingConfig::new("llm".to_string(), config);
    let result = router.parse_routing_config(&routing);
    
    assert!(result.is_ok());
    let agent_type = result.unwrap();
    
    match agent_type {
        AgentType::Llm { provider, model, api_key, timeout, .. } => {
            assert_eq!(provider, "openai");
            assert_eq!(model, "gpt-4");
            assert_eq!(api_key, Some("test-key".to_string()));
            assert_eq!(timeout, Some(120));
        }
        _ => panic!("Expected Llm agent type"),
    }
}

#[tokio::test]
async fn test_websocket_routing_config_parsing() {
    let router = DefaultAgentRouter::new();
    
    let config = json!({
        "url": "ws://localhost:8080/test"
    });
    
    let routing = RoutingConfig::new("websocket".to_string(), config);
    let result = router.parse_routing_config(&routing);
    
    assert!(result.is_ok());
    let agent_type = result.unwrap();
    
    match agent_type {
        AgentType::WebSocket { url, .. } => {
            assert_eq!(url, "ws://localhost:8080/test");
        }
        _ => panic!("Expected WebSocket agent type"),
    }
}

#[tokio::test]
async fn test_unknown_routing_type() {
    let router = DefaultAgentRouter::new();
    
    let config = json!({});
    let routing = RoutingConfig::new("unknown".to_string(), config);
    let result = router.parse_routing_config(&routing);
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Unknown routing type"));
}

#[tokio::test]
async fn test_subprocess_execution() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = create_test_tool_call("test_echo", json!({
        "message": "hello world"
    }));
    
    let tool_def = create_test_tool_definition("subprocess", json!({
        "command": "echo",
        "args": ["{message}"],
        "timeout": 10
    }));
    
    let result = router.route(&tool_call, &tool_def).await;
    assert!(result.is_ok());
    
    let agent_result = result.unwrap();
    assert!(agent_result.success);
    assert!(agent_result.data.is_some());
    assert!(agent_result.error.is_none());
    
    // Check that the output contains our message
    let data = agent_result.data.unwrap();
    let stdout = data.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
    assert!(stdout.contains("hello world"));
}

#[tokio::test]
async fn test_subprocess_execution_with_failure() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = create_test_tool_call("test_false", json!({}));
    
    let tool_def = create_test_tool_definition("subprocess", json!({
        "command": "false",  // Command that always fails
        "args": [],
        "timeout": 10
    }));
    
    let result = router.route(&tool_call, &tool_def).await;
    assert!(result.is_ok());
    
    let agent_result = result.unwrap();
    assert!(!agent_result.success);  // Should fail
    assert!(agent_result.error.is_some());
}

#[tokio::test]
async fn test_subprocess_execution_with_timeout() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = create_test_tool_call("test_sleep", json!({}));
    
    let tool_def = create_test_tool_definition("subprocess", json!({
        "command": "sleep",
        "args": ["5"],  // Sleep for 5 seconds
        "timeout": 1    // But timeout after 1 second
    }));
    
    let result = router.route(&tool_call, &tool_def).await;
    assert!(result.is_ok());
    
    let agent_result = result.unwrap();
    assert!(!agent_result.success);  // Should timeout
    assert!(agent_result.error.is_some());
    
    let error = agent_result.error.unwrap();
    assert!(error.contains("timed out"));
}

#[tokio::test]
async fn test_router_integration() {
    let router = Router::new();
    
    let tool_call = create_test_tool_call("test_integration", json!({
        "command": "echo integration test"
    }));
    
    let tool_def = create_test_tool_definition("subprocess", json!({
        "command": "bash",
        "args": ["-c", "{command}"],
        "timeout": 10
    }));
    
    let result = router.route(&tool_call, &tool_def).await;
    assert!(result.is_ok());
    
    let agent_result = result.unwrap();
    assert!(agent_result.success);
    
    // Verify metadata
    let metadata = agent_result.metadata.unwrap();
    assert_eq!(metadata.get("tool_name").unwrap(), "test_integration");
    assert_eq!(metadata.get("execution_type").unwrap(), "subprocess");
}

#[tokio::test]
async fn test_parameter_substitution_in_routing() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = create_test_tool_call("test_params", json!({
        "name": "Alice",
        "age": 30,
        "active": true
    }));
    
    let tool_def = create_test_tool_definition("subprocess", json!({
        "command": "echo",
        "args": ["Hello {name}, you are {age} years old, active: {active}"],
        "timeout": 10
    }));
    
    let result = router.route(&tool_call, &tool_def).await;
    assert!(result.is_ok());
    
    let agent_result = result.unwrap();
    assert!(agent_result.success);
    
    let data = agent_result.data.unwrap();
    let stdout = data.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
    assert!(stdout.contains("Hello Alice"));
    assert!(stdout.contains("30 years old"));
    assert!(stdout.contains("active: true"));
}

#[tokio::test]
async fn test_database_routing_config_parsing() {
    let router = DefaultAgentRouter::new();

    let routing = RoutingConfig {
        r#type: "database".to_string(),
        config: json!({
            "db_type": "sqlite",
            "connection_string": ":memory:",
            "query": "SELECT 1 as test_value",
            "timeout": 30
        }),
    };

    let _tool_call = ToolCall {
        name: "test_db_tool".to_string(),
        arguments: json!({}),
    };

    // Test parsing the routing config
    let agent_type = router.parse_routing_config(&routing);
    assert!(agent_type.is_ok());

    match agent_type.unwrap() {
        AgentType::Database { db_type, connection_string, query, timeout } => {
            assert_eq!(db_type, "sqlite");
            assert_eq!(connection_string, ":memory:");
            assert_eq!(query, "SELECT 1 as test_value");
            assert_eq!(timeout, Some(30));
        }
        _ => panic!("Expected Database agent type"),
    }
}

#[tokio::test]
async fn test_database_sqlite_execution() {
    let router = DefaultAgentRouter::new();

    let routing = RoutingConfig {
        r#type: "database".to_string(),
        config: json!({
            "db_type": "sqlite",
            "connection_string": ":memory:",
            "query": "SELECT {{value}} as result",
            "timeout": 10
        }),
    };

    let tool_call = ToolCall {
        name: "test_sqlite_tool".to_string(),
        arguments: json!({
            "value": 42
        }),
    };

    // Parse the routing config to get the agent type
    let agent_type = router.parse_routing_config(&routing).unwrap();

    let result = router.execute_with_agent(&tool_call, &agent_type).await;
    assert!(result.is_ok());

    let agent_result = result.unwrap();
    assert!(agent_result.success);
    assert!(agent_result.data.is_some());

    // Check the result structure
    let data = agent_result.data.unwrap();
    assert!(data.get("rows").is_some());
    assert!(data.get("row_count").is_some());

    let rows = data.get("rows").unwrap().as_array().unwrap();
    assert_eq!(rows.len(), 1);

    let first_row = &rows[0];
    let result_value = first_row.get("result").unwrap().as_i64().unwrap();
    assert_eq!(result_value, 42);
}
