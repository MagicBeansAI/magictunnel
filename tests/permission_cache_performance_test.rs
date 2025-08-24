//! Performance Tests for Permission-Based Caching System
//!
//! This module contains specialized performance tests that validate the
//! sub-millisecond performance requirements of the permission system.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::test;
// use criterion::black_box; // Not using criterion for now
use std::collections::HashMap;

use magictunnel::discovery::permission_cache::{
    PermissionCacheManager, PermissionCacheConfig
};
use magictunnel::discovery::fast_evaluator::{
    FastPermissionEvaluator, FastUserContext, RuleAction
};
use magictunnel::security::{SecurityContext, SecurityUser, SecurityRequest, RbacService, RbacConfig};
use ahash::{AHashMap, AHashSet};

/// Helper to create a test RBAC service
fn create_test_rbac_service() -> Arc<RbacService> {
    let config = RbacConfig::default();
    Arc::new(RbacService::new(config).expect("Failed to create test RBAC service"))
}

/// Create large-scale test data for performance testing
fn create_large_test_data(num_tools: usize, num_roles: usize) -> (AHashMap<String, Vec<String>>, Vec<String>) {
    let mut tools_to_roles = AHashMap::new();
    let mut all_roles = Vec::new();
    
    // Create roles
    for i in 0..num_roles {
        all_roles.push(format!("role_{}", i));
    }
    
    // Create tools with random role assignments
    for i in 0..num_tools {
        let tool_name = format!("tool_{}", i);
        let mut tool_roles = Vec::new();
        
        // Assign 1-3 random roles to each tool
        let role_count = (i % 3) + 1;
        for j in 0..role_count {
            let role_index = (i + j) % num_roles;
            tool_roles.push(all_roles[role_index].clone());
        }
        
        tools_to_roles.insert(tool_name, tool_roles);
    }
    
    (tools_to_roles, all_roles)
}

/// Helper to create security context for performance testing
fn create_perf_security_context(user_id: &str, roles: Vec<String>) -> SecurityContext {
    SecurityContext {
        user: Some(SecurityUser {
            id: Some(user_id.to_string()),
            roles,
            api_key_name: None,
            permissions: vec![],
            auth_method: "test".to_string(),
        }),
        request: SecurityRequest {
            id: "perf-req-123".to_string(),
            method: "POST".to_string(),
            path: "/mcp/tools/list".to_string(),
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("perf-test".to_string()),
            headers: HashMap::new(),
            body: None,
            timestamp: chrono::Utc::now(),
        },
        tool: None,
        resource: None,
        metadata: HashMap::new(),
    }
}

/// Benchmark individual tool permission checks
#[test]
async fn bench_individual_tool_permission_checks() {
    println!("🚀 Benchmarking individual tool permission checks...");
    
    let mut evaluator = FastPermissionEvaluator::new(RuleAction::Allow);
    
    // Create test contexts for different user types
    let admin_context = create_perf_security_context("admin", vec!["admin".to_string()]);
    let admin_fast = FastUserContext::from_security_context(&admin_context).unwrap();
    
    let user_context = create_perf_security_context("user", vec!["user".to_string()]);
    let user_fast = FastUserContext::from_security_context(&user_context).unwrap();
    
    // Test tools
    let test_tools: Vec<String> = (0..1000).map(|i| format!("tool_{}", i)).collect();
    
    // Benchmark admin user (high permissions)
    let start = Instant::now();
    let mut admin_results = 0;
    for tool in &test_tools {
        let result = evaluator.is_tool_allowed(&admin_fast, tool);
        if result.allowed { admin_results += 1; }
    }
    let admin_duration = start.elapsed();
    
    // Benchmark regular user (limited permissions)
    let start = Instant::now();
    let mut user_results = 0;
    for tool in &test_tools {
        let result = evaluator.is_tool_allowed(&user_fast, tool);
        if result.allowed { user_results += 1; }
    }
    let user_duration = start.elapsed();
    
    println!("Individual Permission Check Results:");
    println!("  Tools tested: {}", test_tools.len());
    println!("  Admin user:");
    println!("    Time: {:.2}ms ({:.0}ns per check)", admin_duration.as_millis(), admin_duration.as_nanos() as f64 / test_tools.len() as f64);
    println!("    Allowed tools: {}", admin_results);
    println!("    Checks per second: {:.0}", test_tools.len() as f64 / admin_duration.as_secs_f64());
    
    println!("  Regular user:");
    println!("    Time: {:.2}ms ({:.0}ns per check)", user_duration.as_millis(), user_duration.as_nanos() as f64 / test_tools.len() as f64);
    println!("    Allowed tools: {}", user_results);
    println!("    Checks per second: {:.0}", test_tools.len() as f64 / user_duration.as_secs_f64());
    
    // Performance assertions - target <100μs per check
    assert!(admin_duration.as_nanos() / (test_tools.len() as u128) < 100_000, 
            "Admin checks too slow: {}ns per check", admin_duration.as_nanos() / (test_tools.len() as u128));
    assert!(user_duration.as_nanos() / (test_tools.len() as u128) < 100_000, 
            "User checks too slow: {}ns per check", user_duration.as_nanos() / (test_tools.len() as u128));
}

/// Benchmark batch permission evaluation
#[test]
async fn bench_batch_permission_evaluation() {
    println!("🚀 Benchmarking batch permission evaluation...");
    
    let mut evaluator = FastPermissionEvaluator::new(RuleAction::Deny);
    
    let user_context = create_perf_security_context("batch_user", vec!["user".to_string(), "developer".to_string()]);
    let user_fast = FastUserContext::from_security_context(&user_context).unwrap();
    
    // Test different batch sizes
    let batch_sizes = vec![10, 100, 1000, 10000];
    
    for batch_size in batch_sizes {
        let tools: Vec<String> = (0..batch_size).map(|i| format!("batch_tool_{}", i)).collect();
        
        // Warm up
        let _ = evaluator.batch_evaluate(&user_fast, &tools);
        
        // Benchmark
        let start = Instant::now();
        let results = evaluator.batch_evaluate(&user_fast, &tools);
        let duration = start.elapsed();
        
        let allowed_count = results.iter().filter(|(_, allowed)| *allowed).count();
        let tools_per_second = batch_size as f64 / duration.as_secs_f64();
        
        println!("Batch size {}: {:.2}ms, {} allowed, {:.0} tools/sec", 
                 batch_size, duration.as_millis(), allowed_count, tools_per_second);
        
        // Performance assertion - should process at least 100k tools/second
        assert!(tools_per_second > 100_000.0, 
                "Batch evaluation too slow: {:.0} tools/sec for batch size {}", tools_per_second, batch_size);
    }
}

/// Benchmark permission cache performance
#[test]
async fn bench_permission_cache_performance() {
    println!("🚀 Benchmarking permission cache performance...");
    
    let cache_config = PermissionCacheConfig {
        max_user_caches: 10_000,
        default_user_cache_ttl: Duration::from_secs(300),
        cleanup_interval: Duration::from_secs(60),
        enable_stats: true,
    };
    
    let rbac_service = create_test_rbac_service();
    let cache_manager = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));
    
    // Test cache performance with different numbers of users and tools
    let test_scenarios = vec![
        (100, 1000),   // 100 users, 1k tools
        (1000, 10000), // 1k users, 10k tools
        (5000, 50000), // 5k users, 50k tools
    ];
    
    for (num_users, num_tools) in test_scenarios {
        println!("\n--- Testing {} users with {} tools ---", num_users, num_tools);
        
        let (tools_to_roles, all_roles) = create_large_test_data(num_tools, 10);
        
        // Populate caches for all users
        let start = Instant::now();
        for i in 0..num_users {
            let user_id = format!("user_{}", i);
            let user_roles = vec![all_roles[i % all_roles.len()].clone()];
            let context = create_perf_security_context(&user_id, user_roles);
            
            let _ = cache_manager.get_user_cache(&context).await;
        }
        let population_time = start.elapsed();
        
        // Test cache hit performance
        let start = Instant::now();
        let mut total_allowed = 0;
        for i in 0..num_users {
            let user_id = format!("user_{}", i);
            let user_roles = vec![all_roles[i % all_roles.len()].clone()];
            let context = create_perf_security_context(&user_id, user_roles);
            
            if let Some(cache) = cache_manager.get_user_cache(&context).await {
                total_allowed += cache.allowed_tools.len();
            }
        }
        let lookup_time = start.elapsed();
        
        let cache_stats = cache_manager.get_stats();
        
        println!("  Cache population: {:.2}ms ({:.0}μs per user)", 
                 population_time.as_millis(), population_time.as_micros() as f64 / num_users as f64);
        println!("  Cache lookups: {:.2}ms ({:.0}μs per lookup)", 
                 lookup_time.as_millis(), lookup_time.as_micros() as f64 / num_users as f64);
        println!("  Total tools allowed: {}", total_allowed);
        println!("  Cache stats: {} cached users, {} total hits, {} total misses", 
                 cache_stats.cached_users, cache_stats.total_hits, cache_stats.total_misses);
        
        // Performance assertions
        assert!(population_time.as_micros() / (num_users as u128) < 1000, 
                "Cache population too slow: {}μs per user", population_time.as_micros() / (num_users as u128));
        assert!(lookup_time.as_micros() / (num_users as u128) < 100, 
                "Cache lookup too slow: {}μs per lookup", lookup_time.as_micros() / (num_users as u128));
    }
}

/// Benchmark bitmap operations performance
#[test]
async fn bench_bitmap_operations() {
    println!("🚀 Benchmarking bitmap operations performance...");
    
    let user_context = create_perf_security_context("bitmap_user", vec!["admin".to_string(), "developer".to_string(), "user".to_string()]);
    let fast_context = FastUserContext::from_security_context(&user_context).unwrap();
    
    println!("User permissions bitmap: 0b{:064b}", fast_context.permissions_bitmap);
    
    // Test individual permission checks
    let start = Instant::now();
    let mut permission_results = 0;
    for bit in 0..64 {
        if fast_context.has_permission(bit) {
            permission_results += 1;
        }
    }
    let individual_time = start.elapsed();
    
    // Test batch permission mask checks
    let test_masks = vec![
        0x00000000000000FF, // First 8 bits
        0x000000000000FF00, // Bits 8-15
        0x0000000000FF0000, // Bits 16-23
        0xFFFFFFFFFFFFFFFF, // All bits
        0x5555555555555555, // Every other bit
        0xAAAAAAAAAAAAAAAA, // Opposite pattern
    ];
    
    let start = Instant::now();
    let mut mask_results = 0;
    for _ in 0..10000 {
        for mask in &test_masks {
            if fast_context.has_any_permission(*mask) {
                mask_results += 1;
            }
        }
    }
    let mask_time = start.elapsed();
    
    println!("Bitmap Performance Results:");
    println!("  Individual permission checks (64 bits): {:.0}ns total ({:.0}ns per check)", 
             individual_time.as_nanos(), individual_time.as_nanos() as f64 / 64.0);
    println!("  Permissions found: {}", permission_results);
    
    println!("  Batch mask operations (60k operations): {:.2}ms ({:.0}ns per operation)", 
             mask_time.as_millis(), mask_time.as_nanos() as f64 / 60000.0);
    println!("  Mask hits: {}", mask_results);
    
    // Performance assertions - bitmap operations should be extremely fast
    assert!(individual_time.as_nanos() / 64 < 50, 
            "Individual permission check too slow: {}ns", individual_time.as_nanos() / 64);
    assert!(mask_time.as_nanos() / 60000 < 100, 
            "Mask operation too slow: {}ns", mask_time.as_nanos() / 60000);
}

/// Stress test with high concurrency
#[test]
async fn stress_test_high_concurrency() {
    println!("🚀 Running high concurrency stress test...");
    
    let cache_config = PermissionCacheConfig {
        max_user_caches: 10_000,
        default_user_cache_ttl: Duration::from_secs(60), // Shorter TTL for stress testing
        cleanup_interval: Duration::from_secs(10),
        enable_stats: true,
    };
    
    let rbac_service = create_test_rbac_service();
    let cache_manager = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));
    let mut evaluator = FastPermissionEvaluator::new(RuleAction::Allow);
    
    let test_tools: Vec<String> = (0..1000).map(|i| format!("stress_tool_{}", i)).collect();
    
    // Create many concurrent tasks
    let num_tasks = 100;
    let operations_per_task = 100;
    
    let start = Instant::now();
    let mut tasks = Vec::new();
    
    for task_id in 0..num_tasks {
        let cache_manager = Arc::clone(&cache_manager);
        let tools = test_tools.clone();
        
        let task = tokio::spawn(async move {
            let mut task_results = 0;
            
            for op in 0..operations_per_task {
                let user_id = format!("stress_user_{}_{}", task_id, op);
                let roles = vec![format!("role_{}", op % 5)];
                let context = create_perf_security_context(&user_id, roles);
                
                // Get user cache
                if let Some(_cache) = cache_manager.get_user_cache(&context).await {
                    task_results += 1;
                }
                
                // Simulate some work
                tokio::task::yield_now().await;
            }
            
            task_results
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let results = futures_util::future::join_all(tasks).await;
    let total_time = start.elapsed();
    
    let total_operations = results.iter().map(|r| r.as_ref().unwrap()).sum::<i32>();
    let operations_per_second = total_operations as f64 / total_time.as_secs_f64();
    
    println!("Concurrency Stress Test Results:");
    println!("  Tasks: {}", num_tasks);
    println!("  Operations per task: {}", operations_per_task);
    println!("  Total operations: {}", total_operations);
    println!("  Total time: {:.2}ms", total_time.as_millis());
    println!("  Operations per second: {:.0}", operations_per_second);
    
    let cache_stats = cache_manager.get_stats();
    println!("  Final cache stats: {} users, {} hits, {} misses", 
             cache_stats.cached_users, cache_stats.total_hits, cache_stats.total_misses);
    
    // Verify all tasks completed successfully
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap() > 0, "Task should have completed some operations");
    }
    
    // Performance assertion - should handle high concurrency
    assert!(operations_per_second > 1000.0, 
            "Concurrency performance too low: {:.0} ops/sec", operations_per_second);
}

/// Memory usage and scalability test
#[test]
async fn test_memory_usage_scalability() {
    println!("🚀 Testing memory usage and scalability...");
    
    let cache_config = PermissionCacheConfig {
        max_user_caches: 100_000, // Large cache for memory testing
        default_user_cache_ttl: Duration::from_secs(3600),
        cleanup_interval: Duration::from_secs(300),
        enable_stats: true,
    };
    
    let rbac_service = create_test_rbac_service();
    let cache_manager = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));
    
    // Test different user counts to observe memory scaling
    let user_counts = vec![100, 1000, 10_000];
    
    for user_count in user_counts {
        println!("\n--- Testing with {} users ---", user_count);
        
        let start = Instant::now();
        
        // Create caches for many users
        for i in 0..user_count {
            let user_id = format!("mem_user_{}", i);
            let roles = vec![
                format!("role_{}", i % 10),
                format!("secondary_role_{}", i % 20),
            ];
            let context = create_perf_security_context(&user_id, roles);
            
            let _ = cache_manager.get_user_cache(&context).await;
            
            // Periodically yield to avoid blocking
            if i % 100 == 0 {
                tokio::task::yield_now().await;
            }
        }
        
        let creation_time = start.elapsed();
        let cache_stats = cache_manager.get_stats();
        
        println!("  Cache creation time: {:.2}ms", creation_time.as_millis());
        println!("  Cached users: {}", cache_stats.cached_users);
        println!("  Estimated memory usage: {:.2} MB", cache_stats.estimated_memory_bytes as f64 / 1_000_000.0);
        
        // Test lookup performance with full cache
        let start = Instant::now();
        let sample_size = std::cmp::min(1000, user_count);
        
        for i in 0..sample_size {
            let user_id = format!("mem_user_{}", i * user_count / sample_size);
            let roles = vec![format!("role_{}", (i * user_count / sample_size) % 10)];
            let context = create_perf_security_context(&user_id, roles);
            
            let _ = cache_manager.get_user_cache(&context).await;
        }
        
        let lookup_time = start.elapsed();
        
        println!("  Lookup time for {} samples: {:.2}ms ({:.0}μs per lookup)", 
                 sample_size, lookup_time.as_millis(), lookup_time.as_micros() as f64 / sample_size as f64);
        
        // Performance assertions
        assert!(creation_time.as_millis() / (user_count as u128) < 1, 
                "Cache creation too slow: {}ms per user", creation_time.as_millis() / (user_count as u128));
        assert!(lookup_time.as_micros() / (sample_size as u128) < 1000, 
                "Cache lookup too slow with large cache: {}μs per lookup", lookup_time.as_micros() / (sample_size as u128));
    }
}

/// Test real-world scenario simulation
#[test]
async fn test_real_world_scenario_simulation() {
    println!("🚀 Running real-world scenario simulation...");
    
    // Simulate a real deployment scenario:
    // - 5000 users with different role distributions
    // - 50000 tools with realistic permission requirements
    // - Mixed read/write operations
    // - Cache invalidations
    
    let cache_config = PermissionCacheConfig {
        max_user_caches: 10_000,
        default_user_cache_ttl: Duration::from_secs(300),
        cleanup_interval: Duration::from_secs(30),
        enable_stats: true,
    };
    
    let rbac_service = create_test_rbac_service();
    let cache_manager = Arc::new(PermissionCacheManager::new(cache_config, rbac_service));
    let mut evaluator = FastPermissionEvaluator::new(RuleAction::Deny);
    
    // Create realistic user distribution
    let total_users = 5000;
    let admin_users = total_users / 50;      // 2% admins
    let developer_users = total_users / 10;   // 10% developers
    let user_users = total_users - admin_users - developer_users; // 88% regular users
    
    println!("Simulating real-world deployment:");
    println!("  Total users: {}", total_users);
    println!("  Admins: {} ({}%)", admin_users, admin_users * 100 / total_users);
    println!("  Developers: {} ({}%)", developer_users, developer_users * 100 / total_users);
    println!("  Regular users: {} ({}%)", user_users, user_users * 100 / total_users);
    
    // Simulate realistic tool access patterns
    let tool_categories = vec![
        ("admin_tools", 100),      // 100 admin tools
        ("dev_tools", 500),        // 500 developer tools
        ("user_tools", 2000),      // 2000 user tools
        ("public_tools", 1000),    // 1000 public tools
    ];
    
    let mut all_tools = Vec::new();
    for (category, count) in tool_categories {
        for i in 0..count {
            all_tools.push(format!("{}_tool_{}", category, i));
        }
    }
    
    println!("  Total tools: {}", all_tools.len());
    
    // Simulate typical access patterns
    let start = Instant::now();
    let mut total_checks = 0;
    let mut total_allowed = 0;
    
    // Simulate different user types accessing tools
    for user_type in &["admin", "developer", "user"] {
        let user_count = match *user_type {
            "admin" => admin_users,
            "developer" => developer_users,
            "user" => user_users,
            _ => 0,
        };
        
        for user_id in 0..user_count {
            let full_user_id = format!("{}_user_{}", user_type, user_id);
            let roles = vec![user_type.to_string()];
            let context = create_perf_security_context(&full_user_id, roles);
            
            if let Some(fast_context) = FastUserContext::from_security_context(&context) {
                // Each user checks a subset of tools (realistic usage)
                let tools_to_check = match *user_type {
                    "admin" => &all_tools[..],           // Admins check all tools
                    "developer" => &all_tools[..3000],   // Developers check most tools
                    "user" => &all_tools[2000..],        // Users check user+public tools
                    _ => &[],
                };
                
                for tool in tools_to_check.iter().take(100) { // Limit per user for simulation
                    let result = evaluator.is_tool_allowed(&fast_context, tool);
                    total_checks += 1;
                    if result.allowed {
                        total_allowed += 1;
                    }
                }
            }
        }
    }
    
    let simulation_time = start.elapsed();
    let checks_per_second = total_checks as f64 / simulation_time.as_secs_f64();
    
    println!("\nReal-world simulation results:");
    println!("  Total permission checks: {}", total_checks);
    println!("  Total allowed: {} ({:.1}%)", total_allowed, total_allowed as f64 * 100.0 / total_checks as f64);
    println!("  Total time: {:.2}ms", simulation_time.as_millis());
    println!("  Checks per second: {:.0}", checks_per_second);
    println!("  Average time per check: {:.0}ns", simulation_time.as_nanos() as f64 / total_checks as f64);
    
    let final_stats = cache_manager.get_stats();
    println!("  Final cache stats: {} users, {:.2} MB memory", 
             final_stats.cached_users, final_stats.estimated_memory_bytes as f64 / 1_000_000.0);
    
    // Real-world performance assertions
    assert!(checks_per_second > 50_000.0, 
            "Real-world performance too low: {:.0} checks/sec", checks_per_second);
    assert!(simulation_time.as_nanos() / (total_checks as u128) < 1_000_000, 
            "Average check time too slow: {}ns", simulation_time.as_nanos() / (total_checks as u128));
    
    println!("✅ Real-world scenario simulation completed successfully!");
}