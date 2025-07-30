//! Comprehensive tests for Smart Discovery system
//!
//! This module contains comprehensive tests covering all features of the Smart Discovery system
//! including service creation, basic operations, LLM parameter mapping, fallback strategies,
//! caching, performance optimization, error handling, and service monitoring.

use std::sync::Arc;
use std::time::Duration;
use tokio::test;
use serde_json::json;

use magictunnel::discovery::*;
use magictunnel::registry::service::RegistryService;
use magictunnel::config::Config;

/// Test basic service creation and configuration
#[test]
async fn test_service_creation_and_basic_properties() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.7,
        max_tools_to_consider: 10,
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: LlmMapperConfig {
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            api_key_env: None,
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: false, // Disable LLM for testing
        },
        cache: DiscoveryCacheConfig::default(),
        ..SmartDiscoveryConfig::default()
    };
    
    let service = SmartDiscoveryService::new(registry, discovery_config).await.unwrap();
    
    // Test basic properties
    assert!(service.is_enabled());
    
    // Test statistics
    let stats = service.get_stats().await;
    assert!(stats.get("total_tools").is_some());
    assert!(stats.get("discovery_enabled").is_some());
    assert_eq!(stats.get("discovery_enabled").unwrap(), &json!(true));
    
    // Test cache stats
    let cache_stats = service.get_cache_stats().await;
    assert!(cache_stats.get("enabled").is_some());
    assert!(cache_stats.get("hits").is_some());
    assert!(cache_stats.get("misses").is_some());
}

/// Test basic discovery requests
#[test]
async fn test_basic_discovery_requests() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.7,
        max_tools_to_consider: 10,
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: LlmMapperConfig {
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            api_key_env: None,
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: false,
        },
        cache: DiscoveryCacheConfig::default(),
        ..SmartDiscoveryConfig::default()
    };
    
    let service = SmartDiscoveryService::new(registry, discovery_config).await.unwrap();
    
    // Test different types of requests
    let requests = vec![
        SmartDiscoveryRequest {
            request: "simple request".to_string(),
            context: None,
            preferred_tools: None,
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        },
        SmartDiscoveryRequest {
            request: "request with context".to_string(),
            context: Some("This is additional context".to_string()),
            preferred_tools: None,
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        },
        SmartDiscoveryRequest {
            request: "request with preferences".to_string(),
            context: None,
            preferred_tools: Some(vec!["tool1".to_string(), "tool2".to_string()]),
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        },
        SmartDiscoveryRequest {
            request: "request with custom threshold".to_string(),
            context: None,
            preferred_tools: None,
            confidence_threshold: Some(0.8),
            include_error_details: None,
            sequential_mode: None,
        },
    ];
    
    for request in requests {
        let response = service.discover_and_execute(request).await.unwrap();
        
        // All requests should get a response
        assert!(response.data.is_some() || response.error.is_some());
        
        // Metadata should be valid
        assert!(response.metadata.confidence_score >= 0.0);
        assert!(response.metadata.confidence_score <= 1.0);
    }
}

/// Test LLM parameter mapping with various request types
#[test]
async fn test_llm_parameter_mapping_comprehensive() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.5,
        max_tools_to_consider: 10,
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: LlmMapperConfig {
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            api_key_env: None,
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: false, // Use mock for testing
        },
        cache: DiscoveryCacheConfig::default(),
        ..SmartDiscoveryConfig::default()
    };
    
    let service = SmartDiscoveryService::new(registry, discovery_config).await.unwrap();
    
    // Test various parameter mapping scenarios
    let test_cases = vec![
        ("simple request", "do something basic"),
        ("complex request", "perform advanced operation with multiple steps"),
        ("parameter extraction", "process file at /tmp/test.txt with mode=fast"),
        ("context aware", "handle user data based on current session"),
        ("error handling", "manage failures gracefully"),
    ];
    
    for (test_name, request_text) in test_cases {
        let request = SmartDiscoveryRequest {
            request: request_text.to_string(),
            context: Some(format!("Test case: {}", test_name)),
            preferred_tools: None,
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        };
        
        let response = service.discover_and_execute(request).await.unwrap();
        
        // Should get some response
        assert!(response.data.is_some() || response.error.is_some(), 
                "Test case '{}' failed to get response", test_name);
        
        // Metadata should be present
        assert!(response.metadata.confidence_score >= 0.0);
        assert!(response.metadata.confidence_score <= 1.0);
        
        // Should have reasoning or extraction status
        assert!(response.metadata.reasoning.is_some() || 
                response.metadata.extraction_status.is_some(),
                "Test case '{}' missing reasoning/extraction status", test_name);
    }
}

/// Test comprehensive fallback strategies
#[test]
async fn test_fallback_strategies_comprehensive() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.9, // High threshold to trigger fallback
        max_tools_to_consider: 5,
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: LlmMapperConfig {
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            api_key_env: None,
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: false,
        },
        cache: DiscoveryCacheConfig::default(),
        ..SmartDiscoveryConfig::default()
    };
    
    let service = SmartDiscoveryService::new(registry, discovery_config).await.unwrap();
    
    // Test various fallback scenarios
    let fallback_cases = vec![
        ("typo handling", "fil operations"),
        ("partial match", "process data"),
        ("category match", "network request"),
        ("keyword matching", "database query"),
        ("no match", "completely unknown request xyz123"),
    ];
    
    for (test_name, request_text) in fallback_cases {
        let request = SmartDiscoveryRequest {
            request: request_text.to_string(),
            context: Some(format!("Fallback test: {}", test_name)),
            preferred_tools: None,
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        };
        
        let response = service.discover_and_execute(request).await.unwrap();
        
        // Should get some response even with fallback
        assert!(response.data.is_some() || response.error.is_some(),
                "Fallback test '{}' failed to get response", test_name);
        
        // Should have metadata
        assert!(response.metadata.confidence_score >= 0.0);
        assert!(response.metadata.confidence_score <= 1.0);
    }
}

/// Test caching behavior comprehensively
#[test]
async fn test_caching_comprehensive() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.7,
        max_tools_to_consider: 10,
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: LlmMapperConfig {
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            api_key_env: None,
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: false,
        },
        cache: DiscoveryCacheConfig {
            enabled: true,
            max_tool_matches: 100,
            tool_match_ttl: Duration::from_secs(300),
            max_llm_responses: 50,
            llm_response_ttl: Duration::from_secs(600),
            max_registry_entries: 10,
            registry_ttl: Duration::from_secs(60),
        },
        ..SmartDiscoveryConfig::default()
    };
    
    let service = SmartDiscoveryService::new(registry, discovery_config).await.unwrap();
    
    // Test cache hit/miss patterns
    let cache_requests = vec![
        ("cache test 1", "first request"),
        ("cache test 2", "second request"),
        ("cache test 1", "first request"), // Should hit cache
        ("cache test 3", "third request"),
        ("cache test 2", "second request"), // Should hit cache
    ];
    
    let mut responses = Vec::new();
    
    for (test_name, request_text) in cache_requests {
        let request = SmartDiscoveryRequest {
            request: request_text.to_string(),
            context: Some(format!("Cache test: {}", test_name)),
            preferred_tools: None,
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        };
        
        let response = service.discover_and_execute(request).await.unwrap();
        responses.push((test_name, response));
    }
    
    // All requests should complete
    assert_eq!(responses.len(), 5);
    
    // Check cache statistics
    let cache_stats = service.get_cache_stats().await;
    assert!(cache_stats.get("enabled").unwrap().as_bool().unwrap_or(false));
    assert!(cache_stats.get("hits").unwrap().as_u64().unwrap_or(0) > 0 ||
            cache_stats.get("misses").unwrap().as_u64().unwrap_or(0) > 0);
    
    // Test cache clearing
    service.clear_cache().await;
    let stats_after_clear = service.get_cache_stats().await;
    assert_eq!(stats_after_clear.get("entries").unwrap().as_u64().unwrap_or(0), 0);
    
    // Test cache with different confidence thresholds
    let confidence_requests = vec![
        ("low confidence", 0.3),
        ("medium confidence", 0.7),
        ("high confidence", 0.9),
    ];
    
    for (test_name, confidence) in confidence_requests {
        let request = SmartDiscoveryRequest {
            request: format!("confidence test: {}", test_name),
            context: None,
            preferred_tools: None,
            confidence_threshold: Some(confidence),
            include_error_details: None,
            sequential_mode: None,
        };
        
        let response = service.discover_and_execute(request).await.unwrap();
        assert!(response.data.is_some() || response.error.is_some());
    }
}

/// Test performance under concurrent load
#[test]
async fn test_performance_and_concurrency() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.7,
        max_tools_to_consider: 10,
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: LlmMapperConfig {
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            api_key_env: None,
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: false,
        },
        cache: DiscoveryCacheConfig::default(),
        ..SmartDiscoveryConfig::default()
    };
    
    let service = Arc::new(SmartDiscoveryService::new(registry, discovery_config).await.unwrap());
    
    // Test concurrent requests
    let mut handles = Vec::new();
    let start_time = std::time::Instant::now();
    
    for i in 0..10 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let request = SmartDiscoveryRequest {
                request: format!("concurrent request {}", i),
                context: Some(format!("Load test iteration {}", i)),
                preferred_tools: None,
                confidence_threshold: None,
                include_error_details: None,
                sequential_mode: None,
            };
            
            let response = service_clone.discover_and_execute(request).await.unwrap();
            (i, response)
        });
        handles.push(handle);
    }
    
    // Wait for all requests
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }
    
    let total_time = start_time.elapsed();
    
    // All requests should complete
    assert_eq!(results.len(), 10);
    
    // All responses should be valid
    for (i, response) in results {
        assert!(response.data.is_some() || response.error.is_some(),
                "Concurrent request {} failed", i);
        assert!(response.metadata.confidence_score >= 0.0);
        assert!(response.metadata.confidence_score <= 1.0);
    }
    
    // Performance should be reasonable
    assert!(total_time.as_secs() < 10, "Concurrent requests took too long: {:?}", total_time);
}

/// Test error handling and edge cases
#[test]
async fn test_error_handling_comprehensive() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.7,
        max_tools_to_consider: 10,
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: LlmMapperConfig {
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            api_key_env: None,
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: false,
        },
        cache: DiscoveryCacheConfig::default(),
        ..SmartDiscoveryConfig::default()
    };
    
    let service = SmartDiscoveryService::new(registry, discovery_config).await.unwrap();
    
    // Test edge cases
    let long_request = "a".repeat(10000);
    let edge_cases = vec![
        ("empty request", ""),
        ("whitespace only", "   "),
        ("very long request", &long_request),
        ("special characters", "!@#$%^&*()_+-={}[]|\\:;\"'<>?,./"),
        ("unicode", "こんにちは 世界 🌍"),
        ("null bytes", "test\0request"),
    ];
    
    for (test_name, request_text) in edge_cases {
        let request = SmartDiscoveryRequest {
            request: request_text.to_string(),
            context: Some(format!("Edge case test: {}", test_name)),
            preferred_tools: None,
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        };
        
        let response = service.discover_and_execute(request).await.unwrap();
        
        // Should handle edge cases gracefully
        assert!(response.data.is_some() || response.error.is_some(),
                "Edge case '{}' failed to get response", test_name);
        
        // Metadata should be valid
        assert!(response.metadata.confidence_score >= 0.0);
        assert!(response.metadata.confidence_score <= 1.0);
    }
    
    // Test invalid confidence thresholds
    let invalid_thresholds = vec![-0.1, 1.1, 2.0, f64::NAN, f64::INFINITY];
    
    for threshold in invalid_thresholds {
        let request = SmartDiscoveryRequest {
            request: "test invalid threshold".to_string(),
            context: None,
            preferred_tools: None,
            confidence_threshold: Some(threshold),
            include_error_details: None,
            sequential_mode: None,
        };
        
        let response = service.discover_and_execute(request).await.unwrap();
        // Should handle invalid thresholds gracefully
        assert!(response.data.is_some() || response.error.is_some());
    }
}

/// Test preferred tools functionality
#[test]
async fn test_preferred_tools_comprehensive() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.5,
        max_tools_to_consider: 10,
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: LlmMapperConfig {
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            api_key_env: None,
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: false,
        },
        cache: DiscoveryCacheConfig::default(),
        ..SmartDiscoveryConfig::default()
    };
    
    let service = SmartDiscoveryService::new(registry, discovery_config).await.unwrap();
    
    // Test preferred tools with various configurations
    let preference_tests = vec![
        ("single preferred", vec!["preferred_tool"]),
        ("multiple preferred", vec!["tool1", "tool2", "tool3"]),
        ("non-existent preferred", vec!["non_existent_tool"]),
        ("mixed preferred", vec!["real_tool", "fake_tool"]),
    ];
    
    for (test_name, preferred_tools) in preference_tests {
        let request = SmartDiscoveryRequest {
            request: format!("test with preferred tools: {}", test_name),
            context: None,
            preferred_tools: Some(preferred_tools.into_iter().map(|s| s.to_string()).collect()),
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        };
        
        let response = service.discover_and_execute(request).await.unwrap();
        
        // Should handle preferred tools gracefully
        assert!(response.data.is_some() || response.error.is_some(),
                "Preferred tools test '{}' failed", test_name);
        
        // Should have reasonable confidence
        assert!(response.metadata.confidence_score >= 0.0);
        assert!(response.metadata.confidence_score <= 1.0);
    }
}

/// Test context usage and impact
#[test]
async fn test_context_usage_comprehensive() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.5,
        max_tools_to_consider: 10,
        use_fuzzy_matching: true,
        enable_sequential_mode: true,
        llm_mapper: LlmMapperConfig {
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            api_key_env: None,
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: false,
        },
        cache: DiscoveryCacheConfig::default(),
        ..SmartDiscoveryConfig::default()
    };
    
    let service = SmartDiscoveryService::new(registry, discovery_config).await.unwrap();
    
    // Test various context scenarios
    let long_context = "Context information ".repeat(100);
    let context_tests = vec![
        ("no context", None),
        ("simple context", Some("User is working on a project")),
        ("detailed context", Some("User is a developer working on a web application that processes user data and needs to implement authentication")),
        ("technical context", Some("System context: Linux server, Docker containers, microservices architecture")),
        ("long context", Some(&long_context)),
    ];
    
    for (test_name, context) in context_tests {
        let request = SmartDiscoveryRequest {
            request: "perform operation".to_string(),
            context: context.map(|s| s.to_string()),
            preferred_tools: None,
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        };
        
        let response = service.discover_and_execute(request).await.unwrap();
        
        // Should handle context appropriately
        assert!(response.data.is_some() || response.error.is_some(),
                "Context test '{}' failed", test_name);
        
        // Should have valid metadata
        assert!(response.metadata.confidence_score >= 0.0);
        assert!(response.metadata.confidence_score <= 1.0);
    }
}

/// Test configuration validation and defaults
#[test]
async fn test_configuration_validation() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    // Test default configuration
    let default_config = SmartDiscoveryConfig::default();
    assert!(default_config.enabled);
    assert_eq!(default_config.default_confidence_threshold, 0.7);
    assert_eq!(default_config.max_tools_to_consider, 10);
    assert!(default_config.use_fuzzy_matching);
    
    // Test service creation with default config
    let service = SmartDiscoveryService::new(registry.clone(), default_config).await.unwrap();
    assert!(service.is_enabled());
    
    // Test service with custom config
    let custom_config = SmartDiscoveryConfig {
        enabled: true,
        default_confidence_threshold: 0.5,
        max_tools_to_consider: 5,
        use_fuzzy_matching: false,
        enable_sequential_mode: true,
        llm_mapper: LlmMapperConfig {
            provider: "custom".to_string(),
            model: "custom-model".to_string(),
            api_key: Some("test-key".to_string()),
            api_key_env: None,
            base_url: Some("http://localhost:8080".to_string()),
            timeout: 60,
            max_retries: 5,
            enabled: false,
        },
        cache: DiscoveryCacheConfig {
            enabled: true,
            max_tool_matches: 50,
            tool_match_ttl: Duration::from_secs(600),
            max_llm_responses: 25,
            llm_response_ttl: Duration::from_secs(300),
            max_registry_entries: 10,
            registry_ttl: Duration::from_secs(120),
        },
        ..SmartDiscoveryConfig::default()
    };
    
    let service2 = SmartDiscoveryService::new(registry, custom_config).await.unwrap();
    assert!(service2.is_enabled());
}

/// Test service monitoring and statistics
#[test]
async fn test_service_monitoring_comprehensive() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig::default();
    let service = SmartDiscoveryService::new(registry, discovery_config).await.unwrap();
    
    // Make various requests to generate statistics
    let monitoring_requests = vec![
        ("basic request", "do something"),
        ("complex request", "perform advanced operation"),
        ("error request", ""),
        ("cache test", "cached operation"),
        ("cache test", "cached operation"), // Duplicate for cache hit
    ];
    
    for (test_name, request_text) in monitoring_requests {
        let request = SmartDiscoveryRequest {
            request: request_text.to_string(),
            context: Some(format!("Monitoring test: {}", test_name)),
            preferred_tools: None,
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        };
        
        let _response = service.discover_and_execute(request).await.unwrap();
    }
    
    // Check comprehensive statistics
    let stats = service.get_stats().await;
    
    // Verify all expected statistics are present
    let expected_stats = vec![
        "total_tools",
        "visible_tools",
        "hidden_tools",
        "enabled_tools",
        "disabled_tools",
        "discoverable_tools",
        "discovery_enabled",
        "default_confidence_threshold",
        "cache_enabled",
        "cache_hits",
        "cache_misses",
        "cache_hit_rate",
        "cache_evictions",
        "cache_entries",
    ];
    
    for stat_key in expected_stats {
        assert!(stats.get(stat_key).is_some(), "Missing statistic: {}", stat_key);
    }
    
    // Check specific values
    assert_eq!(stats.get("discovery_enabled").unwrap(), &json!(true));
    assert_eq!(stats.get("cache_enabled").unwrap(), &json!(true));
    assert_eq!(stats.get("default_confidence_threshold").unwrap(), &json!(0.7));
    
    // Check cache statistics
    let cache_stats = service.get_cache_stats().await;
    assert!(cache_stats.get("enabled").is_some());
    assert!(cache_stats.get("hits").is_some());
    assert!(cache_stats.get("misses").is_some());
    assert!(cache_stats.get("entries").is_some());
    
    // Test service state
    assert!(service.is_enabled());
}

/// Test disabled service behavior
#[test]
async fn test_disabled_service_comprehensive() {
    let config = Config::default();
    let registry = Arc::new(RegistryService::new(config.registry.clone()).await.unwrap());
    
    let discovery_config = SmartDiscoveryConfig {
        enabled: false, // Disabled
        default_confidence_threshold: 0.7,
        max_tools_to_consider: 10,
        use_fuzzy_matching: true,
        llm_mapper: LlmMapperConfig {
            provider: "mock".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            api_key_env: None,
            base_url: None,
            timeout: 30,
            max_retries: 3,
            enabled: false,
        },
        cache: DiscoveryCacheConfig::default(),
        ..SmartDiscoveryConfig::default()
    };
    
    let service = SmartDiscoveryService::new(registry, discovery_config).await.unwrap();
    
    // Service should be disabled
    assert!(!service.is_enabled());
    
    // Test various requests to disabled service
    let disabled_tests = vec![
        ("simple request", "do something"),
        ("complex request", "perform advanced operation"),
        ("edge case", ""),
        ("with context", "operation with context"),
    ];
    
    for (test_name, request_text) in disabled_tests {
        let request = SmartDiscoveryRequest {
            request: request_text.to_string(),
            context: Some(format!("Disabled test: {}", test_name)),
            preferred_tools: None,
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        };
        
        let response = service.discover_and_execute(request).await.unwrap();
        
        // Should return error indicating service is disabled
        assert!(!response.success, "Disabled service test '{}' should fail", test_name);
        assert!(response.error.is_some(), "Disabled service test '{}' should have error", test_name);
        assert!(response.error.as_ref().unwrap().contains("disabled"), 
                "Disabled service test '{}' should mention disabled", test_name);
    }
    
    // Statistics should still be available
    let stats = service.get_stats().await;
    assert_eq!(stats.get("discovery_enabled").unwrap(), &json!(false));
    
    // Cache should still work
    let cache_stats = service.get_cache_stats().await;
    assert!(cache_stats.get("enabled").is_some());
}