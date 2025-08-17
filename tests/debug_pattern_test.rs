//! Debug test to understand pattern matching behavior

use std::collections::HashMap;
use std::path::PathBuf;

use magictunnel::security::{
    AllowlistService, AllowlistConfig, AllowlistContext, 
    AllowlistAction
};

#[test]
fn debug_pattern_matching_flow() {
    println!("🔍 Debug: Pattern Matching Flow Analysis");
    
    // Get the security directory path
    let mut security_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    security_dir.push("security");
    
    println!("Security directory: {:?}", security_dir);
    
    // Create service with pattern loader
    let config = AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Allow,
        emergency_lockdown: false,
        tools: HashMap::new(),
        servers: HashMap::new(),
        capability_patterns: Vec::new(),
        global_patterns: Vec::new(),
    };
    
    let service = match AllowlistService::with_pattern_loader(config, &security_dir) {
        Ok(s) => s,
        Err(e) => {
            println!("❌ Failed to create service: {}", e);
            panic!("Service creation failed");
        }
    };
    
    // Check what patterns were actually loaded
    let loaded_config = service.get_config();
    println!("✅ Service created successfully");
    println!("   Capability patterns loaded: {}", loaded_config.capability_patterns.len());
    println!("   Global patterns loaded: {}", loaded_config.global_patterns.len());
    
    // Print first few patterns for debugging
    if !loaded_config.capability_patterns.is_empty() {
        println!("   First capability pattern: {:?}", loaded_config.capability_patterns[0]);
    }
    if !loaded_config.global_patterns.is_empty() {
        println!("   First global pattern: {:?}", loaded_config.global_patterns[0]);
    }
    
    let context = AllowlistContext {
        user_id: Some("debug_test".to_string()),
        user_roles: vec!["user".to_string()],
        api_key_name: None,
        permissions: vec!["read".to_string()],
        source: Some("debug_test".to_string()),
        client_ip: Some("127.0.0.1".to_string()),
    };
    
    let empty_params = HashMap::new();
    
    // Test different types of tools
    let test_cases = vec![
        ("unknown_tool_xyz", "should_not_match_any_pattern"),
        ("file_delete_user_data", "should_match_destructive_operations"),
        ("password_reset_user", "should_match_credential_operations"),
        ("get_system_info", "should_match_read_operations"),
    ];
    
    println!("\n🧪 Testing Pattern Matching:");
    
    for (tool_name, expectation) in test_cases {
        println!("\n--- Testing: {} ---", tool_name);
        println!("Expected: {}", expectation);
        
        let result = service.check_tool_access(tool_name, &empty_params, &context);
        
        println!("Result: allowed={}, reason='{}', rule_level={:?}", 
                result.allowed, result.reason, result.rule_level);
        
        if let Some(matched_rule) = &result.matched_rule {
            println!("Matched rule: {}", matched_rule);
        } else {
            println!("No matched rule");
        }
        
        println!("Decision time: {}ns", result.decision_time_ns);
    }
    
    // Check cache performance
    println!("\n📊 Cache Performance:");
    println!("   Cache hit ratio: {:.1}%", service.get_cache_hit_ratio() * 100.0);
    println!("   Average decision time: {}ns", service.get_average_decision_time_ns());
    
    // Test repeated calls for cache behavior
    println!("\n🔄 Testing Cache Behavior:");
    let test_tool = "file_delete_important";
    
    // First call (cache miss)
    let start = std::time::Instant::now();
    let _result1 = service.check_tool_access(test_tool, &empty_params, &context);
    let first_time = start.elapsed();
    
    // Second call (should hit cache)
    let start = std::time::Instant::now();
    let _result2 = service.check_tool_access(test_tool, &empty_params, &context);
    let second_time = start.elapsed();
    
    println!("   First call (cache miss): {:?}", first_time);
    println!("   Second call (cache hit): {:?}", second_time);
    println!("   Cache hit ratio after: {:.1}%", service.get_cache_hit_ratio() * 100.0);
    
    println!("\n✅ Debug test completed");
}