use magictunnel::routing::agent_router::{DefaultAgentRouter, AgentRouter};
use magictunnel::routing::types::AgentType;
use magictunnel::registry::RoutingConfig;
use magictunnel::mcp::types::{ToolCall};
use serde_json::json;

#[tokio::test]
async fn test_sse_agent_parsing() {
    let routing_config = RoutingConfig {
        r#type: "sse".to_string(),
        config: json!({
            "url": "https://api.example.com/events",
            "headers": {
                "Authorization": "Bearer token123",
                "Accept": "text/event-stream"
            },
            "timeout": 30,
            "max_events": 10,
            "event_filter": "message"
        }),
    };

    let router = DefaultAgentRouter::new();
    let agent_type = router.parse_routing_config(&routing_config).unwrap();

    match agent_type {
        AgentType::Sse { url, headers, timeout, max_events, event_filter } => {
            assert_eq!(url, "https://api.example.com/events");
            assert!(headers.is_some());
            let headers = headers.unwrap();
            assert_eq!(headers.get("Authorization"), Some(&"Bearer token123".to_string()));
            assert_eq!(headers.get("Accept"), Some(&"text/event-stream".to_string()));
            assert_eq!(timeout, Some(30));
            assert_eq!(max_events, Some(10));
            assert_eq!(event_filter, Some("message".to_string()));
        }
        _ => panic!("Expected SSE agent type"),
    }
}

#[tokio::test]
async fn test_sse_agent_parsing_minimal() {
    let routing_config = RoutingConfig {
        r#type: "sse".to_string(),
        config: json!({
            "url": "https://api.example.com/events"
        }),
    };

    let router = DefaultAgentRouter::new();
    let agent_type = router.parse_routing_config(&routing_config).unwrap();

    match agent_type {
        AgentType::Sse { url, headers, timeout, max_events, event_filter } => {
            assert_eq!(url, "https://api.example.com/events");
            assert!(headers.is_none());
            assert!(timeout.is_none());
            assert!(max_events.is_none());
            assert!(event_filter.is_none());
        }
        _ => panic!("Expected SSE agent type"),
    }
}

#[tokio::test]
async fn test_sse_agent_parsing_missing_required_fields() {
    let routing_config = RoutingConfig {
        r#type: "sse".to_string(),
        config: json!({
            "headers": {
                "Authorization": "Bearer token123"
            }
        }),
    };

    let router = DefaultAgentRouter::new();
    let result = router.parse_routing_config(&routing_config);
    
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("SSE agent requires url"));
}

#[tokio::test]
async fn test_sse_agent_parameter_substitution() {
    let routing_config = RoutingConfig {
        r#type: "sse".to_string(),
        config: json!({
            "url": "https://api.example.com/events/{{channel}}",
            "headers": {
                "Authorization": "Bearer {{token}}",
                "X-User-ID": "{{user_id}}"
            },
            "timeout": 45,
            "max_events": 20,
            "event_filter": "{{event_type}}"
        }),
    };

    let tool_call = ToolCall {
        name: "sse_subscribe".to_string(),
        arguments: json!({
            "channel": "notifications",
            "token": "secret123",
            "user_id": "user456",
            "event_type": "update"
        }),
    };

    let router = DefaultAgentRouter::new();
    let agent_type = router.parse_routing_config(&routing_config).unwrap();
    let result = router.execute_with_agent(&tool_call, &agent_type).await.unwrap();

    assert!(result.success);
    assert!(result.data.is_some());
    
    let data = result.data.unwrap();
    assert_eq!(data["status"], "success");
    assert_eq!(data["url"], "https://api.example.com/events/notifications");
    assert_eq!(data["event_filter"], "update");
    
    // Check metadata
    let metadata = result.metadata.unwrap();
    assert_eq!(metadata["execution_type"], "sse");
    assert_eq!(metadata["url"], "https://api.example.com/events/notifications");
    assert_eq!(metadata["max_events"], 20);
    assert_eq!(metadata["event_filter"], "update");
}

#[tokio::test]
async fn test_sse_agent_execution_mock() {
    let routing_config = RoutingConfig {
        r#type: "sse".to_string(),
        config: json!({
            "url": "https://api.example.com/events",
            "headers": {
                "Authorization": "Bearer token123"
            },
            "timeout": 30,
            "max_events": 5,
            "event_filter": "message"
        }),
    };

    let tool_call = ToolCall {
        name: "sse_subscribe".to_string(),
        arguments: json!({}),
    };

    let router = DefaultAgentRouter::new();
    let agent_type = router.parse_routing_config(&routing_config).unwrap();
    let result = router.execute_with_agent(&tool_call, &agent_type).await.unwrap();

    assert!(result.success);
    assert!(result.data.is_some());
    
    let data = result.data.unwrap();
    assert_eq!(data["status"], "success");
    assert_eq!(data["url"], "https://api.example.com/events");
    assert!(data["events"].is_array());
    assert_eq!(data["event_count"], 2); // Mock returns 2 events
    assert_eq!(data["max_events"], 5);
    assert_eq!(data["event_filter"], "message");
    
    // Verify mock events structure
    let events = data["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert!(events[0]["id"].is_string());
    assert!(events[0]["event"].is_string());
    assert!(events[0]["data"].is_string());
    assert!(events[0]["timestamp"].is_string());
}
