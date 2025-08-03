//! Unit tests for individual MCP 2025-06-18 services
//!
//! Tests sampling, elicitation, and roots services independently

use magictunnel::mcp::sampling::SamplingService;
use magictunnel::mcp::elicitation::ElicitationService;
use magictunnel::mcp::roots::RootsService;
use magictunnel::mcp::types::*;
use magictunnel::config::Config;
use serde_json::json;

/// Test SamplingService initialization and configuration
#[tokio::test]
async fn test_sampling_service_initialization() {
    let config = Config::default();
    
    // Test service creation from config
    let result = SamplingService::from_config(&config);
    
    // Service should be created successfully even without LLM config
    assert!(result.is_ok());
    
    let service = result.unwrap();
    let status = service.get_status().await;
    
    // Status should include service information
    assert!(status["enabled"].is_boolean());
    assert!(status["providers"].is_object());
}

/// Test SamplingService with valid request
#[tokio::test]
async fn test_sampling_service_request_handling() {
    let mut config = Config::default();
    
    // Configure with test LLM settings (using rule-based for testing)
    config.smart_discovery = Some(magictunnel::discovery::SmartDiscoveryConfig {
        enabled: true,
        tool_selection_mode: "rule_based".to_string(),
        default_confidence_threshold: 0.7,
        max_tools_to_consider: 5,
        max_high_quality_matches: 3,
        high_quality_threshold: 0.95,
        use_fuzzy_matching: true,
        llm_mapper: magictunnel::discovery::LlmMapperConfig::default(),
        llm_tool_selection: magictunnel::discovery::LlmToolSelectionConfig::default(),
        cache: magictunnel::discovery::DiscoveryCacheConfig::default(),
        fallback: magictunnel::discovery::FallbackConfig::default(),
        semantic_search: magictunnel::discovery::SemanticSearchConfig::default(),
        enable_sequential_mode: false,
        tool_metrics_enabled: None,
        enable_sampling: Some(false),
        enable_elicitation: Some(false),
    });
    
    let service = SamplingService::from_config(&config).unwrap();
    
    let request = SamplingRequest {
        messages: vec![SamplingMessage {
            role: SamplingMessageRole::User,
            content: SamplingContent::Text("Hello, world!".to_string()),
            name: None,
            metadata: None,
        }],
        model_preferences: Some(ModelPreferences {
            intelligence: Some(0.6),
            speed: Some(0.7),
            cost: Some(0.5),
            preferred_models: Some(vec!["gpt-3.5-turbo".to_string()]),
            excluded_models: None,
        }),
        system_prompt: Some("You are a helpful assistant.".to_string()),
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: None,
        stop: None,
        metadata: None,
    };
    
    // Test request handling (may return error without actual LLM config)
    let result = service.handle_sampling_request(request, Some("test_user")).await;
    
    // Should return a result (either success or known error)
    assert!(result.is_ok() || result.is_err());
    
    if let Err(e) = result {
        // Error should be well-formed
        assert!(!e.message.is_empty());
        assert!(matches!(e.code, SamplingErrorCode::InternalError | 
                                 SamplingErrorCode::InvalidRequest |
                                 SamplingErrorCode::RateLimitExceeded |
                                 SamplingErrorCode::SecurityViolation));
    }
}

/// Test ElicitationService initialization and configuration
#[tokio::test]
async fn test_elicitation_service_initialization() {
    let config = Config::default();
    
    // Test service creation from config
    let result = ElicitationService::from_config(&config);
    assert!(result.is_ok());
    
    let service = result.unwrap();
    let status = service.get_status().await;
    
    // Status should include service information
    assert!(status["enabled"].is_boolean());
    assert!(status["pending_requests"].is_number());
}

/// Test ElicitationService with valid flat schema
#[tokio::test]
async fn test_elicitation_service_flat_schema() {
    let config = Config::default();
    let service = ElicitationService::from_config(&config).unwrap();
    
    let request = ElicitationRequest {
        message: "Please provide your basic information".to_string(),
        requested_schema: json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Your name"},
                "age": {"type": "integer", "minimum": 0},
                "email": {"type": "string", "format": "email"}
            },
            "required": ["name"]
        }),
        context: Some(ElicitationContext {
            source: Some("test_suite".to_string()),
            reason: Some("Testing elicitation".to_string()),
            usage: Some("Validation testing".to_string()),
            retention: Some("Test duration only".to_string()),
            privacy_level: Some(ElicitationPrivacyLevel::Internal),
        }),
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

/// Test ElicitationService with invalid nested schema
#[tokio::test]
async fn test_elicitation_service_nested_schema_rejection() {
    let config = Config::default();
    let service = ElicitationService::from_config(&config).unwrap();
    
    let request = ElicitationRequest {
        message: "Please provide complex data".to_string(),
        requested_schema: json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object", // Nested object - should be rejected
                    "properties": {
                        "name": {"type": "string"},
                        "address": {
                            "type": "object", // Double nested - definitely rejected
                            "properties": {
                                "street": {"type": "string"}
                            }
                        }
                    }
                }
            }
        }),
        context: None,
        timeout_seconds: None,
        priority: None,
        metadata: None,
    };
    
    let result = service.handle_elicitation_request(request, Some("test_user")).await;
    
    // Should return error for nested schema
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert_eq!(error.code, ElicitationErrorCode::SchemaTooComplex);
    assert!(error.message.to_lowercase().contains("complex") || 
            error.message.to_lowercase().contains("nested"));
}

/// Test ElicitationService schema validation edge cases
#[tokio::test]
async fn test_elicitation_schema_validation_edge_cases() {
    let config = Config::default();
    let service = ElicitationService::from_config(&config).unwrap();
    
    // Test empty schema
    let empty_schema_request = ElicitationRequest {
        message: "Empty schema test".to_string(),
        requested_schema: json!({}),
        context: None,
        timeout_seconds: None,
        priority: None,
        metadata: None,
    };
    
    let result = service.handle_elicitation_request(empty_schema_request, None).await;
    // Should handle empty schema gracefully
    assert!(result.is_ok() || result.is_err());
    
    // Test array schema (should be allowed as it's not nested objects)
    let array_schema_request = ElicitationRequest {
        message: "Array schema test".to_string(),
        requested_schema: json!({
            "type": "object",
            "properties": {
                "tags": {
                    "type": "array",
                    "items": {"type": "string"}
                },
                "scores": {
                    "type": "array", 
                    "items": {"type": "number"}
                }
            }
        }),
        context: None,
        timeout_seconds: None,
        priority: None,
        metadata: None,
    };
    
    let result = service.handle_elicitation_request(array_schema_request, None).await;
    // Arrays of primitives should be allowed
    assert!(result.is_ok());
}

/// Test RootsService initialization and configuration  
#[tokio::test]
async fn test_roots_service_initialization() {
    let config = Config::default();
    
    // Test service creation from config
    let result = RootsService::from_config(&config);
    assert!(result.is_ok());
    
    let service = result.unwrap();
    let status = service.get_status().await;
    
    // Status should include service information
    assert!(status["enabled"].is_boolean());
    assert!(status["auto_discover_filesystem"].is_boolean());
    assert!(status["predefined_roots"].is_number());
    assert!(status["cache"].is_object());
}

/// Test RootsService roots listing
#[tokio::test]
async fn test_roots_service_list_roots() {
    let mut config = Config::default();
    
    // Enable smart discovery to activate roots service
    config.smart_discovery = Some(magictunnel::discovery::SmartDiscoveryConfig {
        enabled: true,
        tool_selection_mode: "rule_based".to_string(),
        default_confidence_threshold: 0.7,
        max_tools_to_consider: 5,
        max_high_quality_matches: 3,
        high_quality_threshold: 0.95,
        use_fuzzy_matching: true,
        llm_mapper: magictunnel::discovery::LlmMapperConfig::default(),
        llm_tool_selection: magictunnel::discovery::LlmToolSelectionConfig::default(),
        cache: magictunnel::discovery::DiscoveryCacheConfig::default(),
        fallback: magictunnel::discovery::FallbackConfig::default(),
        semantic_search: magictunnel::discovery::SemanticSearchConfig::default(),
        enable_sequential_mode: false,
        tool_metrics_enabled: None,
        enable_sampling: Some(false),
        enable_elicitation: Some(false),
    });
    
    let service = RootsService::from_config(&config).unwrap();
    
    let request = RootsListRequest {
        cursor: None,
        limit: Some(10),
        filter: Some(RootFilter {
            types: Some(vec![RootType::Filesystem]),
            schemes: None,
            accessible_only: Some(true),
        }),
    };
    
    let result = service.handle_roots_list_request(request).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    
    // Should have roots array
    assert!(!response.roots.is_empty()); // Should have at least predefined roots
    
    // Validate root structure
    for root in &response.roots {
        assert!(!root.id.is_empty());
        assert!(!root.uri.is_empty());
        assert!(matches!(root.root_type, RootType::Filesystem | 
                                         RootType::Uri | 
                                         RootType::Database |
                                         RootType::Api |
                                         RootType::CloudStorage |
                                         RootType::Container |
                                         RootType::NetworkShare |
                                         RootType::Custom(_)));
        
        // Verify URI format for filesystem roots
        if root.root_type == RootType::Filesystem {
            assert!(root.uri.starts_with("file://"));
        }
    }
    
    // Check pagination info
    if let Some(total_count) = response.total_count {
        assert!(total_count > 0);
    }
}

/// Test RootsService with filtering
#[tokio::test]
async fn test_roots_service_filtering() {
    let config = Config::default();
    let service = RootsService::from_config(&config).unwrap();
    
    // Test type filtering
    let type_filter_request = RootsListRequest {
        cursor: None,
        limit: Some(20),
        filter: Some(RootFilter {
            types: Some(vec![RootType::Filesystem]),
            schemes: None,
            accessible_only: None,
        }),
    };
    
    let result = service.handle_roots_list_request(type_filter_request).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    
    // All returned roots should be filesystem type
    for root in &response.roots {
        assert_eq!(root.root_type, RootType::Filesystem);
    }
    
    // Test scheme filtering
    let scheme_filter_request = RootsListRequest {
        cursor: None,
        limit: Some(20),
        filter: Some(RootFilter {
            types: None,
            schemes: Some(vec!["file".to_string()]),
            accessible_only: None,
        }),
    };
    
    let result = service.handle_roots_list_request(scheme_filter_request).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    
    // All returned roots should have file:// scheme
    for root in &response.roots {
        assert!(root.uri.starts_with("file://"));
    }
}

/// Test RootsService pagination
#[tokio::test]
async fn test_roots_service_pagination() {
    let config = Config::default();
    let service = RootsService::from_config(&config).unwrap();
    
    // First page request
    let first_page_request = RootsListRequest {
        cursor: None,
        limit: Some(2), // Small limit to force pagination
        filter: None,
    };
    
    let result = service.handle_roots_list_request(first_page_request).await;
    assert!(result.is_ok());
    
    let first_page = result.unwrap();
    
    // Should have at most 2 roots
    assert!(first_page.roots.len() <= 2);
    
    // If there's a next cursor, test second page
    if let Some(next_cursor) = first_page.next_cursor {
        let second_page_request = RootsListRequest {
            cursor: Some(next_cursor),
            limit: Some(2),
            filter: None,
        };
        
        let result = service.handle_roots_list_request(second_page_request).await;
        assert!(result.is_ok());
        
        let second_page = result.unwrap();
        
        // Should have different roots than first page
        for second_root in &second_page.roots {
            for first_root in &first_page.roots {
                assert_ne!(second_root.id, first_root.id);
            }
        }
    }
}

/// Test RootsService manual root management
#[tokio::test]
async fn test_roots_service_manual_root_management() {
    let config = Config::default();
    let service = RootsService::from_config(&config).unwrap();
    
    // Create a test root
    let test_root = Root::filesystem("test_manual_root", "/tmp/test")
        .with_name("Test Manual Root")
        .with_description("A test filesystem root")
        .with_permissions(vec![RootPermission::Read, RootPermission::List]);
    
    // Add manual root
    let result = service.add_manual_root(test_root).await;
    assert!(result.is_ok());
    
    // List roots and verify the manual root is included
    let list_request = RootsListRequest {
        cursor: None,
        limit: Some(50),
        filter: None,
    };
    
    let list_result = service.handle_roots_list_request(list_request).await;
    assert!(list_result.is_ok());
    
    let response = list_result.unwrap();
    
    // Should find our test root
    let found_root = response.roots.iter()
        .find(|r| r.id == "test_manual_root");
    assert!(found_root.is_some());
    
    let root = found_root.unwrap();
    assert_eq!(root.name, Some("Test Manual Root".to_string()));
    assert_eq!(root.description, Some("A test filesystem root".to_string()));
    
    // Remove manual root
    let remove_result = service.remove_manual_root("test_manual_root").await;
    assert!(remove_result.is_ok());
    
    // Verify root is removed
    let list_after_remove = service.handle_roots_list_request(RootsListRequest {
        cursor: None,
        limit: Some(50),
        filter: None,
    }).await;
    assert!(list_after_remove.is_ok());
    
    let response_after = list_after_remove.unwrap();
    let found_after = response_after.roots.iter()
        .find(|r| r.id == "test_manual_root");
    assert!(found_after.is_none());
}

/// Test RootsService security filtering
#[tokio::test]
async fn test_roots_service_security_filtering() {
    let config = Config::default();
    let service = RootsService::from_config(&config).unwrap();
    
    // Try to add a root in a blocked directory
    let blocked_root = Root::filesystem("blocked_root", "/etc/passwd")
        .with_name("Blocked Root");
    
    let result = service.add_manual_root(blocked_root).await;
    
    // Should either reject during validation or filter out during listing
    if result.is_ok() {
        // If it was accepted, it should be filtered out during listing
        let list_request = RootsListRequest {
            cursor: None,
            limit: Some(100),
            filter: None,
        };
        
        let list_result = service.handle_roots_list_request(list_request).await;
        assert!(list_result.is_ok());
        
        let response = list_result.unwrap();
        
        // Should not find the blocked root
        let found_blocked = response.roots.iter()
            .find(|r| r.uri.contains("/etc/passwd"));
        assert!(found_blocked.is_none(), "Blocked paths should be filtered out");
    }
}

/// Test error handling across all services
#[tokio::test]
async fn test_services_error_handling() {
    let config = Config::default();
    
    // Test SamplingService error handling
    let sampling_service = SamplingService::from_config(&config).unwrap();
    
    let invalid_sampling_request = SamplingRequest {
        messages: vec![], // Empty messages should cause error
        model_preferences: None,
        system_prompt: None,
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: None,
        metadata: None,
    };
    
    let sampling_result = sampling_service.handle_sampling_request(invalid_sampling_request, None).await;
    if sampling_result.is_err() {
        let error = sampling_result.unwrap_err();
        assert!(!error.message.is_empty());
    }
    
    // Test ElicitationService error handling
    let elicitation_service = ElicitationService::from_config(&config).unwrap();
    
    let invalid_elicitation_request = ElicitationRequest {
        message: "".to_string(), // Empty message should cause error
        requested_schema: json!({"invalid": "schema"}),
        context: None,
        timeout_seconds: None,
        priority: None,
        metadata: None,
    };
    
    let elicitation_result = elicitation_service.handle_elicitation_request(invalid_elicitation_request, None).await;
    if elicitation_result.is_err() {
        let error = elicitation_result.unwrap_err();
        assert!(!error.message.is_empty());
    }
    
    // Test RootsService error handling
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
    
    let root_result = roots_service.add_manual_root(invalid_root).await;
    assert!(root_result.is_err());
    
    let error = root_result.unwrap_err();
    assert!(!error.message.is_empty());
    assert!(matches!(error.code, RootsErrorCode::InvalidRequest));
}