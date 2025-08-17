use std::collections::HashMap;
use std::path::PathBuf;

use magictunnel::security::{
    AllowlistService, AllowlistConfig, AllowlistContext, 
    AllowlistAction, PatternLoader
};

#[test]
fn test_pattern_loading() {
    println!("🧪 Testing Pattern Loading System");
    
    // Get the security directory path (relative to project root)
    let mut security_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    security_dir.push("security");
    
    // Create pattern loader
    let pattern_loader = PatternLoader::new(&security_dir);
    
    // Test loading capability patterns
    println!("Loading capability patterns...");
    let capability_patterns = pattern_loader.load_capability_patterns().unwrap();
    println!("Loaded {} capability patterns", capability_patterns.len());
    
    assert!(!capability_patterns.is_empty(), "Should load capability patterns");
    
    // Verify pattern priority sorting (lower numbers = higher priority, so ascending order)
    for window in capability_patterns.windows(2) {
        let current_priority = window[0].rule.priority.unwrap_or(999);
        let next_priority = window[1].rule.priority.unwrap_or(999);
        assert!(current_priority <= next_priority, 
               "Patterns should be sorted by priority (lower numbers first)");
    }
    
    // Test loading global patterns  
    println!("Loading global patterns...");
    let global_patterns = pattern_loader.load_global_patterns().unwrap();
    println!("Loaded {} global patterns", global_patterns.len());
    
    assert!(!global_patterns.is_empty(), "Should load global patterns");
    
    // Verify pattern priority sorting for global patterns too
    for window in global_patterns.windows(2) {
        let current_priority = window[0].rule.priority.unwrap_or(999);
        let next_priority = window[1].rule.priority.unwrap_or(999);
        assert!(current_priority <= next_priority, 
               "Global patterns should be sorted by priority (lower numbers first)");
    }
    
    println!("✅ Pattern loading test passed!");
}

#[test]
fn test_pattern_service_integration() {
    println!("🧪 Testing Pattern Service Integration");
    
    // Get the security directory path
    let mut security_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    security_dir.push("security");
    
    // Create basic config
    let config = AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Deny,
        emergency_lockdown: false,
        tools: HashMap::new(),
        servers: HashMap::new(),
        capability_patterns: Vec::new(), // Will be loaded from files
        global_patterns: Vec::new(),     // Will be loaded from files
    };
    
    // Create service with pattern loader
    let service = AllowlistService::with_pattern_loader(config, &security_dir).unwrap();
    
    // Verify patterns were loaded
    let loaded_config = service.get_config();
    assert!(!loaded_config.capability_patterns.is_empty(), "Should have loaded capability patterns");
    assert!(!loaded_config.global_patterns.is_empty(), "Should have loaded global patterns");
    
    println!("Loaded {} capability patterns and {} global patterns", 
             loaded_config.capability_patterns.len(), 
             loaded_config.global_patterns.len());
    
    println!("✅ Service integration test passed!");
}

#[test]
fn test_pattern_matching_functionality() {
    println!("🧪 Testing Pattern Matching Functionality");
    
    let mut security_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    security_dir.push("security");
    
    let config = AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Allow, // Default allow to test pattern blocking
        emergency_lockdown: false,
        tools: HashMap::new(),
        servers: HashMap::new(),
        capability_patterns: Vec::new(),
        global_patterns: Vec::new(),
    };
    
    let service = AllowlistService::with_pattern_loader(config, &security_dir).unwrap();
    
    let context = AllowlistContext {
        user_id: Some("test_user".to_string()),
        user_roles: vec!["user".to_string()],
        api_key_name: None,
        permissions: vec!["read".to_string()],
        source: Some("pattern_test".to_string()),
        client_ip: Some("127.0.0.1".to_string()),
    };
    
    let empty_params = HashMap::new();
    
    // Test destructive operation (should be blocked by capability pattern)
    println!("Testing destructive operation: file_delete_user_data");
    let result = service.check_tool_access("file_delete_user_data", &empty_params, &context);
    assert!(!result.allowed, "Destructive operations should be blocked");
    println!("✅ Destructive operation correctly blocked");
    
    // Test read operation (should be allowed by capability pattern)
    println!("Testing read operation: file_read_config");
    let result = service.check_tool_access("file_read_config", &empty_params, &context);
    assert!(result.allowed, "Read operations should be allowed");
    println!("✅ Read operation correctly allowed");
    
    // Test system admin operation (should be blocked by capability pattern)
    println!("Testing system admin operation: sudo_install_package");
    let result = service.check_tool_access("sudo_install_package", &empty_params, &context);
    assert!(!result.allowed, "System admin operations should be blocked");
    println!("✅ System admin operation correctly blocked");
    
    // Test development tool (should be allowed by capability pattern)
    println!("Testing development tool: git_clone_repo");
    let result = service.check_tool_access("git_clone_repo", &empty_params, &context);
    assert!(result.allowed, "Development tools should be allowed");
    println!("✅ Development tool correctly allowed");
    
    // Test credential operation (should be blocked by global pattern)
    println!("Testing credential operation: reset_user_password");
    let result = service.check_tool_access("reset_user_password", &empty_params, &context);
    assert!(!result.allowed, "Credential operations should be blocked");
    println!("✅ Credential operation correctly blocked");
    
    // Test execution command (should be blocked by global pattern)
    println!("Testing execution command: execute_shell_script");
    let result = service.check_tool_access("execute_shell_script", &empty_params, &context);
    assert!(!result.allowed, "Execution commands should be blocked");
    println!("✅ Execution command correctly blocked");
    
    // Test info gathering (should be allowed by global pattern)
    println!("Testing info tool: get_system_status");
    let result = service.check_tool_access("get_system_status", &empty_params, &context);
    assert!(result.allowed, "Info gathering should be allowed");
    println!("✅ Info gathering correctly allowed");
    
    println!("✅ Pattern matching functionality test passed!");
}

#[test]
fn test_pattern_testing_framework() {
    println!("🧪 Testing Pattern Testing Framework");
    
    let mut security_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    security_dir.push("security");
    
    let config = AllowlistConfig::default();
    let service = AllowlistService::with_pattern_loader(config, &security_dir).unwrap();
    
    // Run pattern tests
    let test_results = service.test_patterns().unwrap();
    
    println!("Pattern test results:");
    println!("  Total tests: {}", test_results.total_tests());
    println!("  Passed tests: {}", test_results.passed_tests());
    println!("  Success rate: {:.1}%", test_results.success_rate() * 100.0);
    
    // Print detailed results
    println!("\nCapability pattern test results:");
    for result in &test_results.capability_results {
        let status = if result.passed { "✅" } else { "❌" };
        println!("  {} {} -> expected: {:?}, actual: {:?}", 
                status, result.tool_name, result.expected_match, result.actual_match);
    }
    
    println!("\nGlobal pattern test results:");
    for result in &test_results.global_results {
        let status = if result.passed { "✅" } else { "❌" };
        println!("  {} {} -> expected: {:?}, actual: {:?}", 
                status, result.tool_name, result.expected_match, result.actual_match);
    }
    
    // Verify reasonable success rate
    assert!(test_results.success_rate() > 0.8, "Pattern tests should have >80% success rate");
    
    println!("✅ Pattern testing framework test passed!");
}

#[test]
fn test_pattern_priority_ordering() {
    println!("🧪 Testing Pattern Priority Ordering");
    
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
        user_id: Some("priority_test".to_string()),
        user_roles: vec!["user".to_string()],
        api_key_name: None,
        permissions: vec!["read".to_string()],
        source: Some("priority_test".to_string()),
        client_ip: Some("127.0.0.1".to_string()),
    };
    
    let empty_params = HashMap::new();
    
    // Test a tool that could match multiple patterns
    // "file_delete_backup" could match both "destructive_operations" (priority 100)
    // and "file_write_operations" but destructive should win due to higher priority
    println!("Testing priority ordering with file_delete_backup");
    let result = service.check_tool_access("file_delete_backup", &empty_params, &context);
    assert!(!result.allowed, "Higher priority destructive pattern should block this");
    println!("✅ Higher priority pattern correctly took precedence");
    
    // Test capability vs global priority
    // A tool matching both capability and global patterns should prefer capability
    println!("Testing capability vs global priority");
    let result = service.check_tool_access("file_read_secrets", &empty_params, &context);
    // This could match both capability "file_read_operations" (allow) and global "credential_operations" (deny)
    // But capability should be evaluated first
    println!("file_read_secrets result: allowed={}, reason={}", result.allowed, result.reason);
    
    println!("✅ Pattern priority ordering test passed!");
}

#[test]
fn test_hot_reload_patterns() {
    println!("🧪 Testing Hot Reload of Patterns");
    
    let mut security_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    security_dir.push("security");
    
    let config = AllowlistConfig::default();
    let service = AllowlistService::with_pattern_loader(config, &security_dir).unwrap();
    
    let original_config = service.get_config();
    let original_capability_count = original_config.capability_patterns.len();
    let original_global_count = original_config.global_patterns.len();
    
    println!("Original pattern counts: {} capability, {} global", 
             original_capability_count, original_global_count);
    
    // Test hot reload
    service.reload_external_patterns().unwrap();
    
    let reloaded_config = service.get_config();
    let reloaded_capability_count = reloaded_config.capability_patterns.len();
    let reloaded_global_count = reloaded_config.global_patterns.len();
    
    println!("Reloaded pattern counts: {} capability, {} global", 
             reloaded_capability_count, reloaded_global_count);
    
    // Counts should be the same after reload (assuming files didn't change)
    assert_eq!(original_capability_count, reloaded_capability_count, "Capability pattern count should be same after reload");
    assert_eq!(original_global_count, reloaded_global_count, "Global pattern count should be same after reload");
    
    println!("✅ Hot reload test passed!");
}