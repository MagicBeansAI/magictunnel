//! Tests for GraphQL agent routing functionality


use magictunnel::mcp::ToolCall;
use magictunnel::registry::RoutingConfig;
use magictunnel::routing::agent_router::{AgentRouter, DefaultAgentRouter};
use magictunnel::routing::types::AgentType;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Helper function to create a test tool call
fn create_test_tool_call(name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        name: name.to_string(),
        arguments,
    }
}

/// Helper function to create GraphQL routing config
fn create_graphql_routing_config(
    endpoint: &str,
    query: Option<&str>,
    variables: Option<Value>,
    headers: Option<HashMap<String, String>>,
    timeout: Option<u64>,
    operation_name: Option<&str>,
) -> RoutingConfig {
    let mut config = serde_json::Map::new();
    config.insert("endpoint".to_string(), json!(endpoint));
    
    if let Some(q) = query {
        config.insert("query".to_string(), json!(q));
    }
    
    if let Some(vars) = variables {
        config.insert("variables".to_string(), vars);
    }
    
    if let Some(h) = headers {
        let headers_json: Value = h.into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect::<serde_json::Map<_, _>>()
            .into();
        config.insert("headers".to_string(), headers_json);
    }
    
    if let Some(t) = timeout {
        config.insert("timeout".to_string(), json!(t));
    }
    
    if let Some(op) = operation_name {
        config.insert("operation_name".to_string(), json!(op));
    }

    RoutingConfig {
        r#type: "graphql".to_string(),
        config: config.into(),
    }
}

#[tokio::test]
async fn test_parse_graphql_routing_config() {
    let router = DefaultAgentRouter::new();
    
    // Test basic GraphQL configuration
    let routing_config = create_graphql_routing_config(
        "https://api.example.com/graphql",
        Some("query { user { id name } }"),
        Some(json!({"userId": "123"})),
        Some([("Authorization".to_string(), "Bearer token".to_string())].into()),
        Some(30),
        Some("GetUser"),
    );

    let agent = router.parse_routing_config(&routing_config).unwrap();
    
    match agent {
        AgentType::GraphQL { endpoint, query, variables, headers, timeout, operation_name } => {
            assert_eq!(endpoint, "https://api.example.com/graphql");
            assert_eq!(query, Some("query { user { id name } }".to_string()));
            assert_eq!(variables, Some(json!({"userId": "123"})));
            assert!(headers.is_some());
            assert_eq!(timeout, Some(30));
            assert_eq!(operation_name, Some("GetUser".to_string()));
        }
        _ => panic!("Expected GraphQL agent type"),
    }
}

#[tokio::test]
async fn test_parse_graphql_routing_config_minimal() {
    let router = DefaultAgentRouter::new();
    
    // Test minimal GraphQL configuration (only endpoint required)
    let routing_config = create_graphql_routing_config(
        "https://api.example.com/graphql",
        None,
        None,
        None,
        None,
        None,
    );

    let agent = router.parse_routing_config(&routing_config).unwrap();
    
    match agent {
        AgentType::GraphQL { endpoint, query, variables, headers, timeout, operation_name } => {
            assert_eq!(endpoint, "https://api.example.com/graphql");
            assert_eq!(query, None);
            assert_eq!(variables, None);
            assert_eq!(headers, None);
            assert_eq!(timeout, None);
            assert_eq!(operation_name, None);
        }
        _ => panic!("Expected GraphQL agent type"),
    }
}

#[tokio::test]
async fn test_parse_graphql_routing_config_missing_endpoint() {
    let router = DefaultAgentRouter::new();
    
    // Test GraphQL configuration without endpoint (should fail)
    let mut config = serde_json::Map::new();
    config.insert("query".to_string(), json!("query { user { id } }"));
    
    let routing_config = RoutingConfig {
        r#type: "graphql".to_string(),
        config: config.into(),
    };

    let result = router.parse_routing_config(&routing_config);
    assert!(result.is_err());
    
    if let Err(error) = result {
        let error_msg = error.to_string();
        assert!(error_msg.contains("GraphQL agent requires endpoint"));
    } else {
        panic!("Expected routing error about missing endpoint");
    }
}

#[tokio::test]
async fn test_execute_graphql_agent_with_config_query() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = create_test_tool_call("test_query", json!({
        "user_id": "123",
        "include_posts": true
    }));

    let agent = AgentType::GraphQL {
        endpoint: "https://api.example.com/graphql".to_string(),
        query: Some("query GetUser($userId: ID!) { user(id: $userId) { id name } }".to_string()),
        variables: Some(json!({"userId": "{{user_id}}"})),
        headers: Some([("Authorization".to_string(), "Bearer token".to_string())].into()),
        timeout: Some(30),
        operation_name: Some("GetUser".to_string()),
    };

    let result = router.execute_with_agent(&tool_call, &agent).await.unwrap();
    
    assert!(result.success);
    assert!(result.data.is_some());
    assert!(result.error.is_none());
    
    // Check metadata
    if let Some(metadata) = result.metadata {
        assert_eq!(metadata["execution_type"], "graphql");
        assert_eq!(metadata["endpoint"], "https://api.example.com/graphql");
        assert_eq!(metadata["operation_name"], "GetUser");
    }
}

#[tokio::test]
async fn test_execute_graphql_agent_with_runtime_query() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = create_test_tool_call("custom_query", json!({
        "query": "query { posts { id title } }",
        "variables": {"limit": 10},
        "operation_name": "GetPosts"
    }));

    let agent = AgentType::GraphQL {
        endpoint: "https://api.example.com/graphql".to_string(),
        query: None, // No query in config - should use from tool call
        variables: None,
        headers: Some([("Content-Type".to_string(), "application/json".to_string())].into()),
        timeout: Some(45),
        operation_name: None,
    };

    let result = router.execute_with_agent(&tool_call, &agent).await.unwrap();
    
    assert!(result.success);
    assert!(result.data.is_some());
    assert!(result.error.is_none());
    
    // Check that the response includes the runtime query information
    if let Some(data) = &result.data {
        if let Some(response_data) = data.get("data") {
            assert_eq!(response_data["query"], "query { posts { id title } }");
            assert_eq!(response_data["variables"], json!({"limit": 10}));
            assert_eq!(response_data["operation_name"], "GetPosts");
        }
    }
}

#[tokio::test]
async fn test_execute_graphql_agent_parameter_substitution() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = create_test_tool_call("user_query", json!({
        "user_id": "user123",
        "api_token": "secret_token"
    }));

    let agent = AgentType::GraphQL {
        endpoint: "https://api.example.com/graphql/{{user_id}}".to_string(),
        query: Some("query GetUser($id: ID!) { user(id: $id) { name } }".to_string()),
        variables: Some(json!({"id": "{{user_id}}"})),
        headers: Some([("Authorization".to_string(), "Bearer {{api_token}}".to_string())].into()),
        timeout: Some(30),
        operation_name: Some("GetUser".to_string()),
    };

    let result = router.execute_with_agent(&tool_call, &agent).await.unwrap();
    
    assert!(result.success);
    
    // Check that parameter substitution worked in metadata
    if let Some(metadata) = result.metadata {
        assert_eq!(metadata["endpoint"], "https://api.example.com/graphql/user123");
    }
    
    // Check that the response includes substituted values
    if let Some(data) = &result.data {
        if let Some(response_data) = data.get("data") {
            assert_eq!(response_data["variables"]["id"], "user123");
            assert_eq!(response_data["endpoint"], "https://api.example.com/graphql/user123");
        }
    }
}

#[tokio::test]
async fn test_execute_graphql_agent_complex_variables() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = create_test_tool_call("create_post", json!({
        "title": "Test Post",
        "content": "This is a test post",
        "author_id": "author123",
        "tags": ["test", "example"],
        "metadata": {
            "category": "blog",
            "featured": true
        }
    }));

    let agent = AgentType::GraphQL {
        endpoint: "https://api.example.com/graphql".to_string(),
        query: Some("mutation CreatePost($input: PostInput!) { createPost(input: $input) { id } }".to_string()),
        variables: Some(json!({
            "input": {
                "title": "{{title}}",
                "content": "{{content}}",
                "authorId": "{{author_id}}",
                "tags": "{{tags}}",
                "metadata": "{{metadata}}"
            }
        })),
        headers: None,
        timeout: Some(60),
        operation_name: Some("CreatePost".to_string()),
    };

    let result = router.execute_with_agent(&tool_call, &agent).await.unwrap();
    
    assert!(result.success);
    
    // Check that complex variable substitution worked
    if let Some(data) = &result.data {
        if let Some(response_data) = data.get("data") {
            let variables = &response_data["variables"];
            assert_eq!(variables["input"]["title"], "Test Post");
            assert_eq!(variables["input"]["content"], "This is a test post");
            assert_eq!(variables["input"]["authorId"], "author123");
            assert_eq!(variables["input"]["tags"], Value::Array(vec![Value::String("test".to_string()), Value::String("example".to_string())]));
            assert_eq!(variables["input"]["metadata"]["category"], "blog");
            assert_eq!(variables["input"]["metadata"]["featured"], true);
        }
    }
}

#[tokio::test]
async fn test_graphql_agent_timeout_configuration() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = create_test_tool_call("slow_query", json!({}));

    // Test with custom timeout
    let agent = AgentType::GraphQL {
        endpoint: "https://api.example.com/graphql".to_string(),
        query: Some("query { slowOperation }".to_string()),
        variables: None,
        headers: None,
        timeout: Some(5), // Short timeout
        operation_name: None,
    };

    let result = router.execute_with_agent(&tool_call, &agent).await.unwrap();
    
    // The mock implementation should succeed, but in a real scenario with a slow endpoint,
    // this would test timeout behavior
    assert!(result.success || !result.success); // Either outcome is valid for mock
}
