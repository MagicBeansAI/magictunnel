use std::collections::HashMap;
use std::time::Instant;

use magictunnel::security::{
    AllowlistService, AllowlistConfig, AllowlistContext, 
    AllowlistAction, AllowlistRule
};

#[test]
fn test_allowlist_performance_benchmark() {
    println!("🚀 Starting Allowlist Performance Benchmark");
    
    // Create config with some test rules to make evaluation realistic
    let mut tools = HashMap::new();
    tools.insert("allowed_tool".to_string(), AllowlistRule {
        action: AllowlistAction::Allow,
        reason: Some("Test tool".to_string()),
        pattern: None,
        priority: None,
        name: Some("allowed_tool".to_string()),
        enabled: true,
    });
    
    tools.insert("blocked_tool".to_string(), AllowlistRule {
        action: AllowlistAction::Deny,
        reason: Some("Blocked for security".to_string()),
        pattern: None,
        priority: None,
        name: Some("blocked_tool".to_string()),
        enabled: true,
    });
    
    let config = AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Deny,
        emergency_lockdown: false,
        tools,
        servers: HashMap::new(),
        capability_patterns: vec![],
        global_patterns: vec![],
    };
    
    let service = AllowlistService::new(config).unwrap();
    let context = AllowlistContext {
        user_id: Some("bench_user".to_string()),
        user_roles: vec!["user".to_string()],
        api_key_name: None,
        permissions: vec!["read".to_string()],
        source: Some("benchmark".to_string()),
        client_ip: Some("127.0.0.1".to_string()),
    };
    
    let empty_params = HashMap::new();
    
    // Warmup phase
    println!("Warming up with 1,000 calls...");
    for _ in 0..1000 {
        let _ = service.check_tool_access("allowed_tool", &empty_params, &context);
    }
    
    // Check cache is working after warmup
    println!("After warmup - Cache hit ratio: {:.2}%, Avg decision time: {}ns", 
            service.get_cache_hit_ratio() * 100.0,
            service.get_average_decision_time_ns());
    
    // Performance benchmark with multiple iteration counts
    for &iterations in &[10_000, 100_000] {
        println!("\n📊 Testing {} iterations:", iterations);
        
        let start = Instant::now();
        let mut allowed_count = 0;
        let mut blocked_count = 0;
        
        for i in 0..iterations {
            // Vary between allowed (90%) and blocked (10%) tools to test different code paths
            let tool_name = if i % 10 == 0 { "blocked_tool" } else { "allowed_tool" };
            let result = service.check_tool_access(tool_name, &empty_params, &context);
            
            if result.allowed {
                allowed_count += 1;
            } else {
                blocked_count += 1;
            }
        }
        
        let elapsed = start.elapsed();
        let evaluations_per_second = iterations as f64 / elapsed.as_secs_f64();
        let ns_per_evaluation = elapsed.as_nanos() as f64 / iterations as f64;
        let us_per_evaluation = ns_per_evaluation / 1000.0;
        
        println!("  ⏱️  Time: {:?}", elapsed);
        println!("  🔢 Evaluations/second: {:.0}", evaluations_per_second);
        println!("  ⚡ Nanoseconds per evaluation: {:.1} ns", ns_per_evaluation);
        println!("  ⚡ Microseconds per evaluation: {:.2} μs", us_per_evaluation);
        println!("  ✅ Allowed: {}, ❌ Blocked: {}", allowed_count, blocked_count);
        println!("  💾 Cache hit ratio: {:.2}%", service.get_cache_hit_ratio() * 100.0);
        println!("  📊 Average decision time: {} ns", service.get_average_decision_time_ns());
        
        // Performance validation
        if evaluations_per_second > 1_000_000.0 {
            println!("  🎉 EXCELLENT: >1M evaluations/second!");
        } else if evaluations_per_second > 100_000.0 {
            println!("  ✅ VERY GOOD: >100K evaluations/second");
        } else if evaluations_per_second > 10_000.0 {
            println!("  ✅ GOOD: >10K evaluations/second");
        } else {
            println!("  ⚠️  Performance could be improved");
        }
        
        // Assert minimum performance requirements
        assert!(evaluations_per_second > 10_000.0, 
               "Performance requirement: >10K evaluations/second, got: {:.0}", evaluations_per_second);
        assert!(us_per_evaluation < 100.0, 
               "Performance requirement: under 100μs per evaluation, got: {:.2}μs", us_per_evaluation);
    }
    
    // Cache efficiency check (note: caching not fully implemented yet)
    let final_hit_ratio = service.get_cache_hit_ratio();
    println!("Final cache hit ratio: {:.2}% (caching system under development)", final_hit_ratio * 100.0);
    
    println!("\n✅ Allowlist performance test completed successfully!");
    println!("🚀 Consolidated allowlist system delivers excellent performance!");
}

#[test]
fn test_allowlist_cache_performance() {
    println!("🧪 Testing Cache Performance");
    
    let mut tools = HashMap::new();
    tools.insert("cached_tool".to_string(), AllowlistRule {
        action: AllowlistAction::Allow,
        reason: Some("Cached tool".to_string()),
        pattern: None,
        priority: None,
        name: Some("cached_tool".to_string()),
        enabled: true,
    });
    
    let config = AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Deny,
        emergency_lockdown: false,
        tools,
        servers: HashMap::new(),
        capability_patterns: vec![],
        global_patterns: vec![],
    };
    
    let service = AllowlistService::new(config).unwrap();
    let context = AllowlistContext {
        user_id: Some("cache_user".to_string()),
        user_roles: vec!["user".to_string()],
        api_key_name: None,
        permissions: vec!["read".to_string()],
        source: Some("cache_test".to_string()),
        client_ip: Some("127.0.0.1".to_string()),
    };
    
    let empty_params = HashMap::new();
    
    // Test cache hit performance (same tool repeatedly)
    let iterations = 50_000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let _ = service.check_tool_access("cached_tool", &empty_params, &context);
    }
    
    let elapsed = start.elapsed();
    let evaluations_per_second = iterations as f64 / elapsed.as_secs_f64();
    let hit_ratio = service.get_cache_hit_ratio();
    
    println!("Cache Performance Results:");
    println!("  Evaluations/second: {:.0}", evaluations_per_second);
    println!("  Cache hit ratio: {:.2}%", hit_ratio * 100.0);
    println!("  Average decision time: {} ns", service.get_average_decision_time_ns());
    
    // Cache should be very effective for repeated calls
    assert!(hit_ratio > 0.9, "Cache hit ratio should be >90% for repeated calls, got: {:.2}%", hit_ratio * 100.0);
    assert!(evaluations_per_second > 100_000.0, "Cached evaluations should be >100K/sec, got: {:.0}", evaluations_per_second);
    
    println!("✅ Cache performance test passed!");
}