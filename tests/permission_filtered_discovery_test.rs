//! Comprehensive Integration Tests for Permission-Based Pre-Filtering System
//!
//! This module contains end-to-end tests for the permission-based pre-filtering
//! system that ensures users only see tools they can actually access.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tokio::test;

use ahash::{AHashMap, AHashSet};
use magictunnel::discovery::cache_invalidation::{
    CacheInvalidationConfig, CacheInvalidationEvent, CacheInvalidationManager,
};
use magictunnel::discovery::fast_evaluator::{
    CompiledRule, CompiledRuleType, FastPermissionEvaluator, FastUserContext, RuleAction,
};
use magictunnel::discovery::filtered_service::{
    FilteredDiscoveryConfig, FilteredSmartDiscoveryService,
};
use magictunnel::discovery::filtered_tool_listing::{
    FilteredListingConfig, FilteredToolListingService,
};
use magictunnel::discovery::permission_cache::{
    PermissionCacheConfig, PermissionCacheManager,
};
use magictunnel::discovery::service::SmartDiscoveryService;
use magictunnel::discovery::*;
use magictunnel::registry::service::RegistryService;
use magictunnel::registry::types::ToolDefinition;
use magictunnel::security::{SecurityContext, SecurityRequest, SecurityUser, RbacService, RbacConfig};

/// Helper to create a test RBAC service
fn create_test_rbac_service() -> Arc<RbacService> {
    let config = RbacConfig::default();
    Arc::new(RbacService::new(config).expect("Failed to create test RBAC service"))
}

/// Helper function to create test security context
fn create_security_context(user_id: &str, roles: Vec<String>) -> SecurityContext {
    SecurityContext {
        user: Some(SecurityUser {
            id: Some(user_id.to_string()),
            roles,
            api_key_name: None,
            permissions: vec![],
            auth_method: "test".to_string(),
        }),
        request: SecurityRequest {
            id: "test-req-123".to_string(),
            method: "POST".to_string(),
            path: "/mcp/call".to_string(),
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            headers: HashMap::new(),
            body: None,
            timestamp: chrono::Utc::now(),
        },
        tool: None,
        resource: None,
        metadata: HashMap::new(),
    }
}

/// Helper function to create test tool definition
fn create_tool_definition(
    name: &str,
    description: &str,
    _categories: Vec<String>,
) -> ToolDefinition {
    use magictunnel::registry::types::RoutingConfig;
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            },
            "required": ["input"]
        }),
        routing: RoutingConfig {
            r#type: "test".to_string(),
            config: Value::Object(serde_json::Map::new()),
        },
        annotations: None,
        hidden: false,
        enabled: true,
        prompt_refs: Vec::new(),
        resource_refs: Vec::new(),
        sampling_strategy: None,
        elicitation_strategy: None,
    }
}

/// Helper function to create test tools collection
fn create_test_tools() -> AHashMap<String, ToolDefinition> {
    let mut tools = AHashMap::new();

    // Admin-only tools
    tools.insert(
        "admin_delete_user".to_string(),
        create_tool_definition(
            "admin_delete_user",
            "Delete a user account",
            vec!["admin".to_string()],
        ),
    );
    tools.insert(
        "admin_backup_database".to_string(),
        create_tool_definition(
            "admin_backup_database",
            "Backup the database",
            vec!["admin".to_string()],
        ),
    );

    // Developer tools
    tools.insert(
        "deploy_application".to_string(),
        create_tool_definition(
            "deploy_application",
            "Deploy application to production",
            vec!["development".to_string()],
        ),
    );
    tools.insert(
        "view_logs".to_string(),
        create_tool_definition(
            "view_logs",
            "View application logs",
            vec!["development".to_string()],
        ),
    );

    // User tools
    tools.insert(
        "read_file".to_string(),
        create_tool_definition(
            "read_file",
            "Read a file from the filesystem",
            vec!["files".to_string()],
        ),
    );
    tools.insert(
        "list_files".to_string(),
        create_tool_definition(
            "list_files",
            "List files in a directory",
            vec!["files".to_string()],
        ),
    );

    // Public tools (available to everyone)
    tools.insert(
        "get_weather".to_string(),
        create_tool_definition(
            "get_weather",
            "Get weather information",
            vec!["weather".to_string()],
        ),
    );
    tools.insert(
        "calculate".to_string(),
        create_tool_definition(
            "calculate",
            "Perform calculations",
            vec!["math".to_string()],
        ),
    );

    tools
}

/// Test basic permission cache functionality
#[test]
async fn test_permission_cache_basic_functionality() {
    let config = PermissionCacheConfig::default();
    let rbac_service = create_test_rbac_service();
    let cache_manager = Arc::new(PermissionCacheManager::new(config, rbac_service));

    // Test admin user context
    let admin_context = create_security_context("admin_user", vec!["admin".to_string()]);
    let admin_cache = cache_manager.get_user_cache(&admin_context).await;

    assert!(admin_cache.is_some());
    let admin_cache = admin_cache.unwrap();
    assert_eq!(admin_cache.user_id, "admin_user");
    assert!(admin_cache.user_roles.contains(&"admin".to_string()));

    // Test regular user context
    let user_context = create_security_context("regular_user", vec!["user".to_string()]);
    let user_cache = cache_manager.get_user_cache(&user_context).await;

    assert!(user_cache.is_some());
    let user_cache = user_cache.unwrap();
    assert_eq!(user_cache.user_id, "regular_user");
    assert!(user_cache.user_roles.contains(&"user".to_string()));
}

/// Test fast permission evaluator with different user types
#[test]
async fn test_fast_permission_evaluator() {
    let mut evaluator = FastPermissionEvaluator::new(RuleAction::Deny);

    // Test admin user (should have high permissions)
    let admin_context = create_security_context("admin_user", vec!["admin".to_string()]);
    let admin_fast_context = FastUserContext::from_security_context(&admin_context).unwrap();

    // Admin should have admin permissions
    assert!(admin_fast_context.has_role("admin"));
    assert!(admin_fast_context.permissions_bitmap != 0); // Should have permissions

    // Test tool access
    let admin_result =
        evaluator.is_tool_allowed(&admin_fast_context, &"admin_delete_user".to_string());
    println!("Admin tool access result: {:?}", admin_result);

    // Test regular user (should have limited permissions)
    let user_context = create_security_context("regular_user", vec!["user".to_string()]);
    let user_fast_context = FastUserContext::from_security_context(&user_context).unwrap();

    assert!(user_fast_context.has_role("user"));
    assert!(!user_fast_context.has_role("admin"));

    let user_result =
        evaluator.is_tool_allowed(&user_fast_context, &"admin_delete_user".to_string());
    println!("User tool access result: {:?}", user_result);

    // Test batch evaluation
    let tools = vec![
        "admin_delete_user".to_string(),
        "read_file".to_string(),
        "get_weather".to_string(),
    ];
    let batch_results = evaluator.batch_evaluate(&user_fast_context, &tools);

    assert_eq!(batch_results.len(), 3);
    println!("Batch evaluation results: {:?}", batch_results);
}

/// Test filtered tool listing service
#[test]
async fn test_filtered_tool_listing_service() {
    let cache_config = PermissionCacheConfig::default();
    let rbac_service = create_test_rbac_service();
    let permission_cache = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));

    let listing_config = FilteredListingConfig {
        enable_permission_filtering: true,
        show_excluded_count: true,
        enable_audit_trail: true,
        max_filtering_time_ms: 100,
        sort_by_relevance: false,
    };

    let listing_service = FilteredToolListingService::new(permission_cache, listing_config);

    let test_tools = create_test_tools();

    // Test admin user - should see all tools
    let admin_context = create_security_context("admin_user", vec!["admin".to_string()]);
    let admin_result = listing_service
        .filter_tools_for_listing(&test_tools, &admin_context)
        .await;

    assert!(admin_result.is_ok());
    let admin_filtered = admin_result.unwrap();

    println!(
        "Admin user sees {} out of {} tools",
        admin_filtered.allowed_tools.len(),
        admin_filtered.total_tools_available
    );
    println!("Tools excluded: {}", admin_filtered.tools_excluded);

    // Test regular user - should see fewer tools
    let user_context = create_security_context("regular_user", vec!["user".to_string()]);
    let user_result = listing_service
        .filter_tools_for_listing(&test_tools, &user_context)
        .await;

    assert!(user_result.is_ok());
    let user_filtered = user_result.unwrap();

    println!(
        "Regular user sees {} out of {} tools",
        user_filtered.allowed_tools.len(),
        user_filtered.total_tools_available
    );
    println!("Tools excluded: {}", user_filtered.tools_excluded);

    // Regular user should see fewer tools than admin
    assert!(user_filtered.allowed_tools.len() <= admin_filtered.allowed_tools.len());

    // Check audit trail
    if let Some(audit_trail) = &user_filtered.audit_trail {
        println!(
            "Audit trail - Total tools: {}",
            audit_trail.total_tools_available
        );
        println!(
            "Allowlist exclusions: {}",
            audit_trail.tools_excluded_by_allowlist.len()
        );
        println!(
            "RBAC exclusions: {}",
            audit_trail.tools_excluded_by_rbac.len()
        );
    }
}

/// Test cache invalidation system
#[test]
async fn test_cache_invalidation_system() {
    let cache_config = PermissionCacheConfig::default();
    let rbac_service = create_test_rbac_service();
    let cache_manager = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));

    let invalidation_config = CacheInvalidationConfig {
        default_user_cache_ttl: Duration::from_secs(5), // Short TTL for testing
        admin_cache_ttl: Duration::from_secs(2),
        cleanup_interval: Duration::from_secs(1),
        event_history_retention: Duration::from_secs(60),
        max_concurrent_invalidations: 5,
        enable_cache_warming: true,
        enable_predictive_invalidation: false,
    };

    let invalidation_manager =
        CacheInvalidationManager::new(Arc::clone(&cache_manager), invalidation_config);

    // Create initial user cache
    let user_context = create_security_context("test_user", vec!["user".to_string()]);
    let initial_cache = cache_manager.get_user_cache(&user_context).await;
    assert!(initial_cache.is_some());

    // Test user permission change invalidation
    let user_change_event = CacheInvalidationEvent::UserPermissionsChanged {
        user_id: "test_user".to_string(),
        old_roles: vec!["user".to_string()],
        new_roles: vec!["admin".to_string()],
    };

    let invalidation_result = invalidation_manager.invalidate(user_change_event).await;
    assert!(invalidation_result.is_ok());

    // Wait a bit for background processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check stats
    let stats = invalidation_manager.get_stats().await;
    assert!(stats.total_invalidations > 0);
    println!("Invalidation stats: {:?}", stats);

    // Test emergency cache clear
    let emergency_event = CacheInvalidationEvent::EmergencyCacheClear {
        reason: "Security test".to_string(),
        affected_users: None,
    };

    let emergency_result = invalidation_manager.invalidate(emergency_event).await;
    assert!(emergency_result.is_ok());

    // Check updated stats
    let updated_stats = invalidation_manager.get_stats().await;
    assert!(updated_stats.emergency_clears > 0);

    // Get recent events for audit
    let recent_events = invalidation_manager.get_recent_events(10).await;
    assert!(recent_events.len() > 0);
    println!("Recent invalidation events: {}", recent_events.len());
}

/// Test performance with large number of tools
#[test]
async fn test_performance_with_large_tool_set() {
    let cache_config = PermissionCacheConfig::default();
    let rbac_service = create_test_rbac_service();
    let permission_cache = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));

    let listing_config = FilteredListingConfig {
        enable_permission_filtering: true,
        show_excluded_count: false,
        enable_audit_trail: false, // Disable for performance testing
        max_filtering_time_ms: 50, // Tight timeout
        sort_by_relevance: false,
    };

    let listing_service = FilteredToolListingService::new(permission_cache, listing_config);

    // Create large tool set (1000 tools)
    let mut large_tool_set = AHashMap::new();
    for i in 0..1000 {
        let tool_name = format!("tool_{}", i);
        let category = if i % 10 == 0 {
            "admin"
        } else if i % 5 == 0 {
            "developer"
        } else {
            "user"
        };
        large_tool_set.insert(
            tool_name.clone(),
            create_tool_definition(
                &tool_name,
                &format!("Test tool {}", i),
                vec![category.to_string()],
            ),
        );
    }

    println!("Created {} test tools", large_tool_set.len());

    // Test filtering performance
    let user_context = create_security_context("perf_user", vec!["user".to_string()]);

    let start_time = Instant::now();
    let result = listing_service
        .filter_tools_for_listing(&large_tool_set, &user_context)
        .await;
    let elapsed = start_time.elapsed();

    assert!(result.is_ok());
    let filtered = result.unwrap();

    println!("Performance test results:");
    println!("  Total tools: {}", filtered.total_tools_available);
    println!("  Allowed tools: {}", filtered.allowed_tools.len());
    println!("  Excluded tools: {}", filtered.tools_excluded);
    println!("  Filtering time: {:.2}ms", elapsed.as_millis());
    println!(
        "  Tools per second: {:.0}",
        filtered.filtering_metrics.tools_per_second
    );
    println!(
        "  Completed within timeout: {}",
        filtered.filtering_metrics.completed_within_timeout
    );

    // Assert performance targets
    assert!(
        elapsed.as_millis() < 100,
        "Filtering took too long: {}ms",
        elapsed.as_millis()
    );
    assert!(
        filtered.filtering_metrics.tools_per_second > 10_000.0,
        "Too slow: {} tools/sec",
        filtered.filtering_metrics.tools_per_second
    );
}

/// Test edge cases and error handling
#[test]
async fn test_edge_cases_and_error_handling() {
    let cache_config = PermissionCacheConfig::default();
    let rbac_service = create_test_rbac_service();
    let permission_cache = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));

    let listing_config = FilteredListingConfig::default();
    let listing_service = FilteredToolListingService::new(permission_cache, listing_config);

    // Test with empty tool set
    let empty_tools = AHashMap::new();
    let user_context = create_security_context("test_user", vec!["user".to_string()]);

    let empty_result = listing_service
        .filter_tools_for_listing(&empty_tools, &user_context)
        .await;
    assert!(empty_result.is_ok());
    let empty_filtered = empty_result.unwrap();
    assert_eq!(empty_filtered.allowed_tools.len(), 0);
    assert_eq!(empty_filtered.total_tools_available, 0);

    // Test with user having no roles
    let no_roles_context = create_security_context("no_roles_user", vec![]);
    let test_tools = create_test_tools();

    let no_roles_result = listing_service
        .filter_tools_for_listing(&test_tools, &no_roles_context)
        .await;
    assert!(no_roles_result.is_ok());
    let no_roles_filtered = no_roles_result.unwrap();
    println!(
        "User with no roles sees {} tools",
        no_roles_filtered.allowed_tools.len()
    );

    // Test with invalid security context (no user)
    let invalid_context = SecurityContext {
        user: None,
        request: SecurityRequest {
            id: "invalid-req-123".to_string(),
            method: "POST".to_string(),
            path: "/test".to_string(),
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test".to_string()),
            headers: HashMap::new(),
            body: None,
            timestamp: chrono::Utc::now(),
        },
        tool: None,
        resource: None,
        metadata: HashMap::new(),
    };

    let invalid_result = listing_service
        .filter_tools_for_listing(&test_tools, &invalid_context)
        .await;
    assert!(invalid_result.is_ok());
    let invalid_filtered = invalid_result.unwrap();
    // Should still return results but with low confidence
    println!(
        "Invalid context result: {} tools allowed",
        invalid_filtered.allowed_tools.len()
    );
}

/// Test concurrent access and thread safety
#[test]
async fn test_concurrent_access() {
    let cache_config = PermissionCacheConfig::default();
    let rbac_service = create_test_rbac_service();
    let permission_cache = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));

    let listing_config = FilteredListingConfig {
        enable_permission_filtering: true,
        show_excluded_count: false,
        enable_audit_trail: false, // Disable for concurrency testing
        max_filtering_time_ms: 100,
        sort_by_relevance: false,
    };

    let listing_service = Arc::new(FilteredToolListingService::new(
        permission_cache,
        listing_config,
    ));
    let test_tools = Arc::new(create_test_tools());

    // Create multiple concurrent tasks
    let mut tasks = Vec::new();

    for i in 0..10 {
        let service = Arc::clone(&listing_service);
        let tools = Arc::clone(&test_tools);

        let task = tokio::spawn(async move {
            let user_id = format!("user_{}", i);
            let roles = if i % 3 == 0 {
                vec!["admin".to_string()]
            } else if i % 2 == 0 {
                vec!["developer".to_string()]
            } else {
                vec!["user".to_string()]
            };

            let context = create_security_context(&user_id, roles);

            // Perform multiple operations per task
            for _ in 0..5 {
                let result = service.filter_tools_for_listing(&tools, &context).await;
                assert!(result.is_ok());
            }

            user_id
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    let results = futures_util::future::join_all(tasks).await;

    // Verify all tasks completed successfully
    for result in results {
        assert!(result.is_ok());
        println!("Completed concurrent test for user: {}", result.unwrap());
    }

    println!("Concurrent access test completed successfully");
}

/// Test filtered smart discovery integration
#[test]
async fn test_filtered_smart_discovery_integration() {
    // This test would require a full SmartDiscoveryService setup
    // For now, we'll test the configuration and basic initialization

    let cache_config = PermissionCacheConfig::default();
    let rbac_service = create_test_rbac_service();
    let permission_cache = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));

    let discovery_config = FilteredDiscoveryConfig {
        enable_permission_filtering: true,
        enable_audit_trail: true,
        max_permission_filtering_time_ms: 50,
        log_excluded_tools: true,
        cache_config: PermissionCacheConfig::default(),
    };

    // Create a mock SmartDiscoveryService (this would normally be the real service)
    // For testing purposes, we'll just verify the configuration is valid
    assert!(discovery_config.enable_permission_filtering);
    assert!(discovery_config.enable_audit_trail);
    assert_eq!(discovery_config.max_permission_filtering_time_ms, 50);

    println!("Filtered smart discovery configuration validated");
}

/// Test audit trail generation and analysis
#[test]
async fn test_audit_trail_generation() {
    let cache_config = PermissionCacheConfig::default();
    let rbac_service = create_test_rbac_service();
    let permission_cache = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));

    let listing_config = FilteredListingConfig {
        enable_permission_filtering: true,
        show_excluded_count: true,
        enable_audit_trail: true, // Enable audit trail
        max_filtering_time_ms: 100,
        sort_by_relevance: false,
    };

    let listing_service = FilteredToolListingService::new(permission_cache, listing_config);
    let test_tools = create_test_tools();

    // Test with user having limited permissions
    let user_context = create_security_context("audit_user", vec!["user".to_string()]);
    let result = listing_service
        .filter_tools_for_listing(&test_tools, &user_context)
        .await;

    assert!(result.is_ok());
    let filtered = result.unwrap();

    // Verify audit trail was generated
    assert!(filtered.audit_trail.is_some());
    let audit_trail = filtered.audit_trail.unwrap();

    println!("Audit trail analysis:");
    println!("  Request ID: {}", audit_trail.request_id);
    println!("  User context: {:?}", audit_trail.user_context);
    println!(
        "  Total tools available: {}",
        audit_trail.total_tools_available
    );
    println!(
        "  Allowlist exclusions: {}",
        audit_trail.tools_excluded_by_allowlist.len()
    );
    println!(
        "  RBAC exclusions: {}",
        audit_trail.tools_excluded_by_rbac.len()
    );
    println!(
        "  Allowed tools considered: {}",
        audit_trail.allowed_tools_considered.len()
    );

    // Verify audit trail contains expected data
    assert_eq!(audit_trail.total_tools_available, test_tools.len());
    assert!(audit_trail.timestamp.timestamp() > 0);

    // Test exclusion summary
    let exclusion_summary = audit_trail.get_exclusion_summary();
    println!("  Exclusion summary: {:?}", exclusion_summary);

    // Test audit summary
    let summary = audit_trail.get_summary();
    println!("  Summary: {:?}", summary);
    assert_eq!(summary.total_tools, test_tools.len());
}

/// Integration test combining all components
#[test]
async fn test_full_integration() {
    println!("Starting full integration test of permission-based pre-filtering system...");

    // Setup all components
    let cache_config = PermissionCacheConfig {
        max_user_caches: 100,
        default_user_cache_ttl: Duration::from_secs(300),
        cleanup_interval: Duration::from_secs(60),
        enable_stats: true,
    };

    let rbac_service = create_test_rbac_service();
    let permission_cache = Arc::new(PermissionCacheManager::new(cache_config.clone(), rbac_service));

    let invalidation_config = CacheInvalidationConfig::default();
    let invalidation_manager =
        CacheInvalidationManager::new(Arc::clone(&permission_cache), invalidation_config);

    let listing_config = FilteredListingConfig {
        enable_permission_filtering: true,
        show_excluded_count: true,
        enable_audit_trail: true,
        max_filtering_time_ms: 100,
        sort_by_relevance: true,
    };

    let listing_service =
        FilteredToolListingService::new(Arc::clone(&permission_cache), listing_config);

    let test_tools = create_test_tools();

    // Test scenario 1: Admin user workflow
    println!("\n=== Testing Admin User Workflow ===");
    let admin_context = create_security_context("admin_user", vec!["admin".to_string()]);
    let admin_result = listing_service
        .filter_tools_for_listing(&test_tools, &admin_context)
        .await;

    assert!(admin_result.is_ok());
    let admin_filtered = admin_result.unwrap();

    println!(
        "Admin sees {} out of {} tools",
        admin_filtered.allowed_tools.len(),
        admin_filtered.total_tools_available
    );

    // Test scenario 2: Developer user workflow
    println!("\n=== Testing Developer User Workflow ===");
    let dev_context = create_security_context("dev_user", vec!["developer".to_string()]);
    let dev_result = listing_service
        .filter_tools_for_listing(&test_tools, &dev_context)
        .await;

    assert!(dev_result.is_ok());
    let dev_filtered = dev_result.unwrap();

    println!(
        "Developer sees {} out of {} tools",
        dev_filtered.allowed_tools.len(),
        dev_filtered.total_tools_available
    );

    // Test scenario 3: Regular user workflow
    println!("\n=== Testing Regular User Workflow ===");
    let user_context = create_security_context("regular_user", vec!["user".to_string()]);
    let user_result = listing_service
        .filter_tools_for_listing(&test_tools, &user_context)
        .await;

    assert!(user_result.is_ok());
    let user_filtered = user_result.unwrap();

    println!(
        "Regular user sees {} out of {} tools",
        user_filtered.allowed_tools.len(),
        user_filtered.total_tools_available
    );

    // Verify hierarchy: admin >= developer >= user
    assert!(admin_filtered.allowed_tools.len() >= dev_filtered.allowed_tools.len());
    assert!(dev_filtered.allowed_tools.len() >= user_filtered.allowed_tools.len());

    // Test scenario 4: Permission change and cache invalidation
    println!("\n=== Testing Permission Change and Cache Invalidation ===");

    // Trigger user permission change
    let permission_change = CacheInvalidationEvent::UserPermissionsChanged {
        user_id: "regular_user".to_string(),
        old_roles: vec!["user".to_string()],
        new_roles: vec!["developer".to_string()],
    };

    let invalidation_result = invalidation_manager.invalidate(permission_change).await;
    assert!(invalidation_result.is_ok());

    // Wait for cache invalidation to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Get updated stats
    let final_stats = invalidation_manager.get_stats().await;
    let cache_stats = permission_cache.get_stats();

    println!("\n=== Final System Statistics ===");
    println!("Cache stats: {:?}", cache_stats);
    println!("Invalidation stats: {:?}", final_stats);

    // Verify system is working correctly
    assert!(final_stats.total_invalidations > 0);
    assert!(cache_stats.cached_users >= 0);

    println!("\n✅ Full integration test completed successfully!");
}

/// Performance benchmark test
#[test]
async fn test_performance_benchmark() {
    println!("Starting performance benchmark...");

    let cache_config = PermissionCacheConfig::default();
    let rbac_service = create_test_rbac_service();
    let permission_cache = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));

    let listing_config = FilteredListingConfig {
        enable_permission_filtering: true,
        show_excluded_count: false,
        enable_audit_trail: false,   // Disable for pure performance testing
        max_filtering_time_ms: 1000, // Generous timeout
        sort_by_relevance: false,
    };

    let listing_service = FilteredToolListingService::new(permission_cache, listing_config);

    // Create different sized tool sets
    let test_sizes = vec![100, 1000, 10000];

    for size in test_sizes {
        println!("\n--- Testing with {} tools ---", size);

        let mut tools = AHashMap::new();
        for i in 0..size {
            let tool_name = format!("benchmark_tool_{}", i);
            tools.insert(
                tool_name.clone(),
                create_tool_definition(
                    &tool_name,
                    &format!("Benchmark tool {}", i),
                    vec!["user".to_string()],
                ),
            );
        }

        let user_context = create_security_context("benchmark_user", vec!["user".to_string()]);

        // Warm up
        let _ = listing_service
            .filter_tools_for_listing(&tools, &user_context)
            .await;

        // Benchmark multiple runs
        let mut timings = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            let result = listing_service
                .filter_tools_for_listing(&tools, &user_context)
                .await;
            let elapsed = start.elapsed();

            assert!(result.is_ok());
            timings.push(elapsed);
        }

        // Calculate statistics
        let avg_time = timings.iter().sum::<Duration>() / timings.len() as u32;
        let min_time = timings.iter().min().unwrap();
        let max_time = timings.iter().max().unwrap();

        let tools_per_second = size as f64 / avg_time.as_secs_f64();

        println!("  Average time: {:.2}ms", avg_time.as_millis());
        println!("  Min time: {:.2}ms", min_time.as_millis());
        println!("  Max time: {:.2}ms", max_time.as_millis());
        println!("  Tools per second: {:.0}", tools_per_second);

        // Performance assertions
        assert!(
            avg_time.as_millis() < 100,
            "Average time too slow for {} tools: {}ms",
            size,
            avg_time.as_millis()
        );
        assert!(
            tools_per_second > 1000.0,
            "Too slow: {} tools/sec for {} tools",
            tools_per_second,
            size
        );
    }

    println!("\n✅ Performance benchmark completed successfully!");
}
