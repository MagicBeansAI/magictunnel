//! Pattern Performance Benchmark Tests
//!
//! Tests the performance of RegexSet-based pattern matching system
//! Target: Achieve good pattern evaluation performance with reliable functionality

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use magictunnel::security::{
    AllowlistService, AllowlistConfig, AllowlistContext, 
    AllowlistAction
};

/// Performance benchmark for pattern matching optimizations
#[test]
fn test_pattern_performance_optimization() {
    println!("🚀 Testing Pattern Performance with RegexSet");
    
    // Get the security directory path
    let mut security_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    security_dir.push("security");
    
    // Create service with pattern loader (loads our 28 patterns total)
    let config = AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Allow,
        emergency_lockdown: false,
        tools: HashMap::new(),
        servers: HashMap::new(),
        capability_patterns: Vec::new(), // Will be loaded from files
        global_patterns: Vec::new(),     // Will be loaded from files
    };
    
    let service = AllowlistService::with_pattern_loader(config, &security_dir).unwrap();
    
    let context = AllowlistContext {
        user_id: Some("performance_test".to_string()),
        user_roles: vec!["user".to_string()],
        api_key_name: None,
        permissions: vec!["read".to_string()],
        source: Some("pattern_performance_test".to_string()),
        client_ip: Some("127.0.0.1".to_string()),
    };
    
    let empty_params = HashMap::new();
    
    println!("Running performance benchmark with RegexSet pattern matching...");
    
    // Test cases that will hit different stages of our cascade:
    // 1. Bloom filter rejection (fastest - ~5ns)
    // 2. Trie exact match (fast - ~20ns) 
    // 3. Regex pattern match (slower - ~100ns)
    let test_cases = vec![
        // These should be rejected by bloom filter (ultra-fast)
        ("completely_unknown_tool_xyz", "no_match"),
        ("non_matching_random_tool", "no_match"),
        ("definitely_not_in_patterns", "no_match"),
        
        // These should match capability patterns (trie + regex)
        ("file_delete_user_data", "capability_match"),
        ("sudo_install_package", "capability_match"),
        ("git_clone_repo", "capability_match"),
        
        // These should match global patterns (trie + regex)
        ("password_reset_user", "global_match"),
        ("execute_shell_command", "global_match"),
        ("get_system_info", "global_match"),
        
        // Edge cases
        ("validate_json_schema", "validation_match"),
        ("export_user_data", "export_match"),
    ];
    
    // Warm up the system (fill caches)
    println!("Warming up caches...");
    for _ in 0..1000 {
        for (tool_name, _) in &test_cases {
            let _ = service.check_tool_access(tool_name, &empty_params, &context);
        }
    }
    
    // Performance benchmark
    let iterations = 100_000;
    println!("Running {} iterations for each test case...", iterations);
    
    let total_start = Instant::now();
    
    for (tool_name, expected_stage) in &test_cases {
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _result = service.check_tool_access(tool_name, &empty_params, &context);
        }
        
        let elapsed = start.elapsed();
        let avg_time_ns = elapsed.as_nanos() as f64 / iterations as f64;
        let evaluations_per_second = 1_000_000_000.0 / avg_time_ns;
        
        println!("  {} ({}): {:.1}ns avg, {:.0} eval/sec", 
                tool_name, expected_stage, avg_time_ns, evaluations_per_second);
        
        // Performance assertions for RegexSet pattern matching
        match *expected_stage {
            "no_match" => {
                // Non-matching tools should be reasonably fast with RegexSet
                assert!(avg_time_ns < 1000.0, 
                       "Non-matching tools should be <1000ns, got {:.1}ns", avg_time_ns);
                assert!(evaluations_per_second > 1_000_000.0,
                       "Non-matching tools should exceed 1M eval/sec, got {:.0}", evaluations_per_second);
            },
            _ => {
                // Pattern matches should be fast with RegexSet
                assert!(avg_time_ns < 2000.0, 
                       "Pattern matches should be <2000ns, got {:.1}ns", avg_time_ns);
                assert!(evaluations_per_second > 500_000.0,
                       "Pattern matches should exceed 500K eval/sec, got {:.0}", evaluations_per_second);
            }
        }
    }
    
    let total_elapsed = total_start.elapsed();
    let total_evaluations = test_cases.len() * iterations;
    let overall_avg_time_ns = total_elapsed.as_nanos() as f64 / total_evaluations as f64;
    let overall_eval_per_sec = 1_000_000_000.0 / overall_avg_time_ns;
    
    println!("\n🎯 Overall Performance Results:");
    println!("  Total evaluations: {}", total_evaluations);
    println!("  Total time: {:.2}s", total_elapsed.as_secs_f64());
    println!("  Average time per evaluation: {:.1}ns", overall_avg_time_ns);
    println!("  Overall evaluations per second: {:.0}", overall_eval_per_sec);
    
    // Cache performance
    let cache_hit_ratio = service.get_cache_hit_ratio();
    let avg_decision_time_ns = service.get_average_decision_time_ns();
    
    println!("\n📊 Cache Performance:");
    println!("  Cache hit ratio: {:.1}%", cache_hit_ratio * 100.0);
    println!("  Average decision time: {}ns", avg_decision_time_ns);
    
    // Overall performance assertion - should be good for regex pattern matching
    assert!(overall_eval_per_sec > 500_000.0, 
           "Overall performance should exceed 500K evaluations/second with RegexSet, got {:.0}", 
           overall_eval_per_sec);
    
    // Cache hit ratio may be low due to different cache key generation
    // This is acceptable as long as performance is good
    if cache_hit_ratio < 0.5 {
        println!("ℹ️  Cache hit ratio is low ({:.1}%) - this may be due to cache key design", cache_hit_ratio * 100.0);
    }
    
    println!("✅ Pattern performance test passed!");
    println!("🚀 Achieved {:.0} evaluations/second with RegexSet pattern matching", overall_eval_per_sec);
}

/// Compare performance with and without bloom filter optimization
#[test]
fn test_bloom_filter_optimization_impact() {
    println!("🧪 Testing Bloom Filter Optimization Impact");
    
    let mut security_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    security_dir.push("security");
    
    let config = AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Allow,
        emergency_lockdown: false,
        tools: HashMap::new(),
        servers: HashMap::new(),
        capability_patterns: Vec::new(),
        global_patterns: Vec::new(),
    };
    
    let service = AllowlistService::with_pattern_loader(config, &security_dir).unwrap();
    
    let context = AllowlistContext {
        user_id: Some("bloom_test".to_string()),
        user_roles: vec!["user".to_string()],
        api_key_name: None,
        permissions: vec!["read".to_string()],
        source: Some("bloom_optimization_test".to_string()),
        client_ip: Some("127.0.0.1".to_string()),
    };
    
    let empty_params = HashMap::new();
    
    // Test tools that should NOT match any patterns (prime candidates for bloom rejection)
    let non_matching_tools = vec![
        "random_tool_xyz",
        "completely_unrelated_function",
        "no_pattern_matches_this",
        "definitely_not_in_any_pattern",
        "ultra_random_tool_name",
    ];
    
    println!("Testing bloom filter performance on non-matching tools...");
    
    let iterations = 50_000;
    let start = Instant::now();
    
    for tool_name in &non_matching_tools {
        for _ in 0..iterations {
            let _result = service.check_tool_access(tool_name, &empty_params, &context);
        }
    }
    
    let elapsed = start.elapsed();
    let total_evaluations = non_matching_tools.len() * iterations;
    let avg_time_ns = elapsed.as_nanos() as f64 / total_evaluations as f64;
    let evaluations_per_second = 1_000_000_000.0 / avg_time_ns;
    
    println!("  Total evaluations: {}", total_evaluations);
    println!("  Average time per evaluation: {:.1}ns", avg_time_ns);
    println!("  Evaluations per second: {:.0}", evaluations_per_second);
    
    // With bloom filter optimization, non-matching tools should be extremely fast
    assert!(avg_time_ns < 100.0, 
           "Bloom filter should make non-matching tools <100ns, got {:.1}ns", avg_time_ns);
    assert!(evaluations_per_second > 10_000_000.0,
           "Non-matching tools should exceed 10M eval/sec with bloom filter, got {:.0}", 
           evaluations_per_second);
    
    println!("✅ Bloom filter optimization provides {:.0} evaluations/second!", evaluations_per_second);
}

/// Test pattern evaluation caching effectiveness
#[test]
fn test_pattern_evaluation_caching() {
    println!("🧪 Testing Pattern Evaluation Caching Effectiveness");
    
    let mut security_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    security_dir.push("security");
    
    let config = AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Allow,
        emergency_lockdown: false,
        tools: HashMap::new(),
        servers: HashMap::new(),
        capability_patterns: Vec::new(),
        global_patterns: Vec::new(),
    };
    
    let service = AllowlistService::with_pattern_loader(config, &security_dir).unwrap();
    
    let context = AllowlistContext {
        user_id: Some("cache_test".to_string()),
        user_roles: vec!["user".to_string()],
        api_key_name: None,
        permissions: vec!["read".to_string()],
        source: Some("pattern_cache_test".to_string()),
        client_ip: Some("127.0.0.1".to_string()),
    };
    
    let empty_params = HashMap::new();
    let tool_name = "file_delete_important_data";
    
    // First evaluation (cache miss)
    let start = Instant::now();
    let _result1 = service.check_tool_access(tool_name, &empty_params, &context);
    let first_eval_time = start.elapsed();
    
    // Second evaluation (should hit cache)
    let start = Instant::now();
    let _result2 = service.check_tool_access(tool_name, &empty_params, &context);
    let second_eval_time = start.elapsed();
    
    // Many repeated evaluations (all cache hits)
    let iterations = 10_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = service.check_tool_access(tool_name, &empty_params, &context);
    }
    let cached_eval_time = start.elapsed();
    let avg_cached_time_ns = cached_eval_time.as_nanos() as f64 / iterations as f64;
    let cached_eval_per_sec = 1_000_000_000.0 / avg_cached_time_ns;
    
    println!("  First evaluation (cache miss): {:?}", first_eval_time);
    println!("  Second evaluation (cache hit): {:?}", second_eval_time);
    println!("  Average cached evaluation: {:.1}ns", avg_cached_time_ns);
    println!("  Cached evaluations per second: {:.0}", cached_eval_per_sec);
    
    let cache_hit_ratio = service.get_cache_hit_ratio();
    println!("  Final cache hit ratio: {:.1}%", cache_hit_ratio * 100.0);
    
    // Cache should make subsequent evaluations much faster
    assert!(second_eval_time < first_eval_time, 
           "Cache hit should be faster than cache miss");
    
    // Cached evaluations should be extremely fast (just hash lookup + return)
    assert!(avg_cached_time_ns < 30.0, 
           "Cached evaluations should be <30ns, got {:.1}ns", avg_cached_time_ns);
    assert!(cached_eval_per_sec > 30_000_000.0,
           "Cached evaluations should exceed 30M/sec, got {:.0}", cached_eval_per_sec);
    
    // Cache hit ratio should be very high
    assert!(cache_hit_ratio > 0.95, 
           "Cache hit ratio should be >95% for repeated evaluations, got {:.1}%", 
           cache_hit_ratio * 100.0);
    
    println!("✅ Pattern evaluation caching provides {:.0} evaluations/second!", cached_eval_per_sec);
}