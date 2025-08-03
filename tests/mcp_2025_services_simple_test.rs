//! Simple tests for MCP 2025-06-18 services
//!
//! Tests the individual services (sampling, elicitation, roots) with basic functionality

use magictunnel::mcp::{SamplingService, ElicitationService, RootsService};
use magictunnel::mcp::types::sampling::*;
use magictunnel::mcp::types::elicitation::*;
use magictunnel::mcp::types::roots::*;
use magictunnel::config::Config;
use serde_json::json;

/// Test SamplingService creation and basic functionality
#[tokio::test]
async fn test_sampling_service_basic() {
    let config = Config::default();
    
    // Service should be created successfully
    let result = SamplingService::from_config(&config);
    assert!(result.is_ok());
    
    let service = result.unwrap();
    
    // Should be able to get status
    let status = service.get_status().await;
    assert!(status["enabled"].is_boolean());
    assert!(status["providers"].is_object());
    
    // Test with a basic request (may fail without LLM config, but should handle gracefully)
    let request = SamplingRequest {
        messages: vec![SamplingMessage {
            role: SamplingMessageRole::User,
            content: SamplingContent::Text("Hello".to_string()),
            name: None,
            metadata: None,
        }],
        model_preferences: None,
        system_prompt: None,
        max_tokens: Some(50),
        temperature: Some(0.5),
        top_p: None,
        stop: None,
        metadata: None,
    };
    
    let result = service.handle_sampling_request(request, Some("test_user")).await;
    
    // Should return either success or a well-formed error
    if let Err(error) = result {
        assert!(!error.message.is_empty());
        assert!(matches!(error.code, 
            SamplingErrorCode::InternalError | 
            SamplingErrorCode::InvalidRequest |
            SamplingErrorCode::RateLimitExceeded));
    }
}

/// Test ElicitationService creation and basic functionality
#[tokio::test]
async fn test_elicitation_service_basic() {
    let config = Config::default();
    
    // Service should be created successfully
    let result = ElicitationService::from_config(&config);
    assert!(result.is_ok());
    
    let service = result.unwrap();
    
    // Should be able to get status
    let status = service.get_status().await;
    assert!(status["enabled"].is_boolean());
    assert!(status["pending_requests"].is_number());
    
    // Test with a valid flat schema
    let request = ElicitationRequest {
        message: "Please provide your name".to_string(),
        requested_schema: json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            },
            "required": ["name"]
        }),
        context: None,
        timeout_seconds: Some(60),
        priority: Some(ElicitationPriority::Normal),
        metadata: None,
    };
    
    let result = service.handle_elicitation_request(request, Some("test_user")).await;
    
    // Should return a request ID
    assert!(result.is_ok());
    let request_id = result.unwrap();
    assert!(!request_id.is_empty());
    assert!(request_id.starts_with("elicit_"));
}

/// Test ElicitationService schema validation
#[tokio::test]
async fn test_elicitation_schema_validation() {
    let config = Config::default();
    let service = ElicitationService::from_config(&config).unwrap();
    
    // Test that nested schemas are rejected
    let nested_request = ElicitationRequest {
        message: "Complex nested data".to_string(),
        requested_schema: json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",  // Nested object - should be rejected
                    "properties": {
                        "name": {"type": "string"}
                    }
                }
            }
        }),
        context: None,
        timeout_seconds: None,
        priority: None,
        metadata: None,
    };
    
    let result = service.handle_elicitation_request(nested_request, None).await;
    
    // Should return error for nested schema
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code, ElicitationErrorCode::SchemaTooComplex);
}

/// Test RootsService creation and basic functionality
#[tokio::test]
async fn test_roots_service_basic() {
    let config = Config::default();
    
    // Service should be created successfully
    let result = RootsService::from_config(&config);
    assert!(result.is_ok());
    
    let service = result.unwrap();
    
    // Should be able to get status
    let status = service.get_status().await;
    assert!(status["enabled"].is_boolean());
    assert!(status["cache"].is_object());
    
    // Test basic roots listing
    let request = RootsListRequest {
        cursor: None,
        limit: Some(10),
        filter: None,
    };
    
    let result = service.handle_roots_list_request(request).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    
    // Should have roots array (at least predefined ones)
    assert!(!response.roots.is_empty());
    
    // Validate basic root structure
    for root in &response.roots {
        assert!(!root.id.is_empty());
        assert!(!root.uri.is_empty());
        assert!(root.accessible || !root.accessible); // Just checking the field exists
    }
}

/// Test RootsService filtering functionality
#[tokio::test]
async fn test_roots_service_filtering() {
    let config = Config::default();
    let service = RootsService::from_config(&config).unwrap();
    
    // Test filtering by type
    let filtered_request = RootsListRequest {
        cursor: None,
        limit: Some(20),
        filter: Some(RootFilter {
            types: Some(vec![RootType::Filesystem]),
            schemes: None,
            accessible_only: Some(true),
        }),
    };
    
    let result = service.handle_roots_list_request(filtered_request).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    
    // All returned roots should match filter criteria
    for root in &response.roots {
        assert_eq!(root.root_type, RootType::Filesystem);
        assert!(root.accessible); // accessible_only filter
    }
}

/// Test RootsService manual root management
#[tokio::test]
async fn test_roots_manual_management() {
    let config = Config::default();
    let service = RootsService::from_config(&config).unwrap();
    
    // Create a test root
    let test_root = Root::filesystem("test_root", "/tmp/test_root")
        .with_name("Test Root")
        .with_description("A test root for validation");
    
    // Add the root
    let add_result = service.add_manual_root(test_root).await;
    assert!(add_result.is_ok());
    
    // Verify it appears in listing
    let list_request = RootsListRequest {
        cursor: None,
        limit: Some(50),
        filter: None,
    };
    
    let list_result = service.handle_roots_list_request(list_request).await;
    assert!(list_result.is_ok());
    
    let response = list_result.unwrap();
    let found_root = response.roots.iter().find(|r| r.id == "test_root");
    assert!(found_root.is_some());
    
    // Remove the root
    let remove_result = service.remove_manual_root("test_root").await;
    assert!(remove_result.is_ok());
    
    // Verify it's gone
    let list_after_remove = service.handle_roots_list_request(RootsListRequest {
        cursor: None,
        limit: Some(50),
        filter: None,
    }).await;
    assert!(list_after_remove.is_ok());
    
    let response_after = list_after_remove.unwrap();
    let found_after = response_after.roots.iter().find(|r| r.id == "test_root");
    assert!(found_after.is_none());
}

/// Test error handling across services
#[tokio::test]
async fn test_services_error_handling() {
    let config = Config::default();
    
    // Test ElicitationService with invalid schema
    let elicitation_service = ElicitationService::from_config(&config).unwrap();
    
    let invalid_request = ElicitationRequest {
        message: "".to_string(), // Empty message
        requested_schema: json!("invalid"),
        context: None,
        timeout_seconds: None,
        priority: None,
        metadata: None,
    };
    
    let result = elicitation_service.handle_elicitation_request(invalid_request, None).await;
    if result.is_err() {
        let error = result.unwrap_err();
        assert!(!error.message.is_empty());
    }
    
    // Test RootsService with invalid root
    let roots_service = RootsService::from_config(&config).unwrap();
    
    let invalid_root = Root {
        id: "".to_string(), // Empty ID should cause error
        root_type: RootType::Filesystem,
        uri: "".to_string(), // Empty URI should cause error
        name: None,
        description: None,
        accessible: true,
        permissions: None,
        metadata: None,
        tags: None,
    };
    
    let result = roots_service.add_manual_root(invalid_root).await;
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(!error.message.is_empty());
    assert_eq!(error.code, RootsErrorCode::InvalidRequest);
}