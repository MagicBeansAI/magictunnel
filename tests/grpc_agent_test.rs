//! Tests for gRPC agent routing functionality

use magictunnel::routing::agent_router::DefaultAgentRouter;
use magictunnel::routing::types::AgentType;
use magictunnel::mcp::ToolCall;
use magictunnel::registry::RoutingConfig;
use magictunnel::routing::agent_router::AgentRouter;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_grpc_agent_parsing() {
    let router = DefaultAgentRouter::new();
    
    let routing_config = RoutingConfig {
        r#type: "grpc".to_string(),
        config: json!({
            "endpoint": "https://api.example.com:443",
            "service": "UserService",
            "method": "GetUser",
            "timeout": 30,
            "headers": {
                "authorization": "Bearer token123"
            },
            "request_body": "{\"user_id\": \"{{user_id}}\"}"
        }),
    };

    let agent = router.parse_routing_config(&routing_config).unwrap();
    
    match agent {
        AgentType::Grpc { endpoint, service, method, headers, timeout, request_body } => {
            assert_eq!(endpoint, "https://api.example.com:443");
            assert_eq!(service, "UserService");
            assert_eq!(method, "GetUser");
            assert_eq!(timeout, Some(30));
            assert!(headers.is_some());
            assert!(request_body.is_some());
            
            let headers_map = headers.unwrap();
            assert_eq!(headers_map.get("authorization"), Some(&"Bearer token123".to_string()));
        }
        _ => panic!("Expected gRPC agent type"),
    }
}

#[tokio::test]
async fn test_grpc_agent_parsing_minimal() {
    let router = DefaultAgentRouter::new();
    
    let routing_config = RoutingConfig {
        r#type: "grpc".to_string(),
        config: json!({
            "endpoint": "https://minimal.example.com:443",
            "service": "MinimalService",
            "method": "SimpleCall"
        }),
    };

    let agent = router.parse_routing_config(&routing_config).unwrap();
    
    match agent {
        AgentType::Grpc { endpoint, service, method, headers, timeout, request_body } => {
            assert_eq!(endpoint, "https://minimal.example.com:443");
            assert_eq!(service, "MinimalService");
            assert_eq!(method, "SimpleCall");
            assert_eq!(timeout, None);
            assert_eq!(headers, None);
            assert_eq!(request_body, None);
        }
        _ => panic!("Expected gRPC agent type"),
    }
}

#[tokio::test]
async fn test_grpc_agent_parsing_missing_required_fields() {
    let router = DefaultAgentRouter::new();
    
    // Missing endpoint
    let routing_config = RoutingConfig {
        r#type: "grpc".to_string(),
        config: json!({
            "service": "UserService",
            "method": "GetUser"
        }),
    };
    
    let result = router.parse_routing_config(&routing_config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("gRPC agent requires endpoint"));
    
    // Missing service
    let routing_config = RoutingConfig {
        r#type: "grpc".to_string(),
        config: json!({
            "endpoint": "https://api.example.com:443",
            "method": "GetUser"
        }),
    };
    
    let result = router.parse_routing_config(&routing_config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("gRPC agent requires service"));
    
    // Missing method
    let routing_config = RoutingConfig {
        r#type: "grpc".to_string(),
        config: json!({
            "endpoint": "https://api.example.com:443",
            "service": "UserService"
        }),
    };
    
    let result = router.parse_routing_config(&routing_config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("gRPC agent requires method"));
}

#[tokio::test]
async fn test_grpc_agent_execution_mock() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = ToolCall {
        name: "test_grpc_call".to_string(),
        arguments: json!({
            "user_id": "12345",
            "include_profile": true
        }),
    };
    
    let agent = AgentType::Grpc {
        endpoint: "https://mock.example.com:443".to_string(),
        service: "UserService".to_string(),
        method: "GetUser".to_string(),
        headers: Some({
            let mut headers = HashMap::new();
            headers.insert("authorization".to_string(), "Bearer {{env.TOKEN}}".to_string());
            headers
        }),
        timeout: Some(30),
        request_body: Some("{\"user_id\": \"{{user_id}}\", \"include_profile\": {{include_profile}}}".to_string()),
    };
    
    let result = router.execute_with_agent(&tool_call, &agent).await.unwrap();
    
    // Since this is a mock implementation, we expect it to return a success with mock data
    assert!(result.success);
    assert!(result.data.is_some());
    assert!(result.error.is_none());
    
    let data = result.data.unwrap();
    assert_eq!(data["status"], "success");
    assert_eq!(data["service"], "UserService");
    assert_eq!(data["method"], "GetUser");
    assert!(data["message"].as_str().unwrap().contains("mock implementation"));
    
    // Check metadata
    let metadata = result.metadata.unwrap();
    assert_eq!(metadata["execution_type"], "grpc");
    assert_eq!(metadata["endpoint"], "https://mock.example.com:443");
    assert_eq!(metadata["service"], "UserService");
    assert_eq!(metadata["method"], "GetUser");
}

#[tokio::test]
async fn test_grpc_agent_parameter_substitution() {
    let router = DefaultAgentRouter::new();
    
    let tool_call = ToolCall {
        name: "test_substitution".to_string(),
        arguments: json!({
            "endpoint": "https://dynamic.example.com:443",
            "user_id": "67890",
            "action": "update"
        }),
    };
    
    let agent = AgentType::Grpc {
        endpoint: "{{endpoint}}".to_string(),
        service: "DynamicService".to_string(),
        method: "{{action}}User".to_string(),
        headers: Some({
            let mut headers = HashMap::new();
            headers.insert("x-user-id".to_string(), "{{user_id}}".to_string());
            headers
        }),
        timeout: Some(45),
        request_body: Some("{\"user_id\": \"{{user_id}}\", \"action\": \"{{action}}\"}".to_string()),
    };
    
    let result = router.execute_with_agent(&tool_call, &agent).await.unwrap();
    
    // Check that parameter substitution worked in the metadata
    let metadata = result.metadata.unwrap();
    assert_eq!(metadata["endpoint"], "https://dynamic.example.com:443");
    
    // The mock implementation should have received the substituted values
    let data = result.data.unwrap();
    assert!(data["request_body"].as_str().unwrap().contains("67890"));
    assert!(data["request_body"].as_str().unwrap().contains("update"));
}
