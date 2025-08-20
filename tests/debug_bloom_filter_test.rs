//! Debug test to understand bloom filter behavior

use std::collections::HashMap;
use std::path::PathBuf;

use magictunnel::security::{
    AllowlistService, AllowlistConfig, AllowlistContext, 
    AllowlistAction
};

#[test]
fn debug_bloom_filter_behavior() {
    println!("🔍 Debug: Bloom Filter Behavior Analysis");
    
    // Get the security directory path
    let mut security_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    security_dir.push("security");
    
    // Create service with enhanced data file approach
    let mut data_file = security_dir.clone();
    data_file.push("allowlist-data.yaml");
    
    let config = AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Allow,
        emergency_lockdown: false,
        tools: HashMap::new(),
        tool_patterns: Vec::new(),
        capabilities: HashMap::new(),
        capability_patterns: Vec::new(),
        global_patterns: Vec::new(),
        mt_level_rules: HashMap::new(),
        data_file: data_file.to_string_lossy().to_string(),
    };
    
    let service = match AllowlistService::with_data_file(config.clone(), data_file.to_string_lossy().to_string()) {
        Ok(s) => s,
        Err(_) => AllowlistService::new(config).unwrap()
    };
    
    let context = AllowlistContext {
        user_id: Some("debug_bloom".to_string()),
        user_roles: vec!["user".to_string()],
        api_key_name: None,
        permissions: vec!["read".to_string()],
        source: Some("debug_bloom".to_string()),
        client_ip: Some("127.0.0.1".to_string()),
    };
    
    let empty_params = HashMap::new();
    
    // Test the exact pattern from our YAML file
    println!("\\n🧪 Testing exact pattern matching:");
    
    // The destructive_operations pattern is:
    // ".*(?:delete|destroy|remove|rm|kill|terminate|drop|truncate|purge).*"
    // So "file_delete_user_data" should definitely match "delete"
    
    let test_cases = vec![
        ("file_delete_user_data", "Contains 'delete' - should match destructive_operations"),
        ("delete", "Exact 'delete' - should match destructive_operations"),
        ("password_reset", "Contains 'password' - should match credential_operations"),
        ("password", "Exact 'password' - should match credential_operations"),
        ("execute_command", "Contains 'execute' - should match execution_commands"),
        ("read_file", "Contains 'read' - should match read_only_operations"),
    ];
    
    for (tool_name, description) in test_cases {
        println!("\\n--- Testing: {} ---", tool_name);
        println!("Expected: {}", description);
        
        let result = service.check_tool_access(tool_name, &empty_params, &context);
        
        println!("Result: allowed={}, reason='{}', rule_level={:?}", 
                result.allowed, result.reason, result.rule_level);
        
        if let Some(matched_rule) = &result.matched_rule {
            println!("✅ Matched rule: {}", matched_rule);
        } else {
            println!("❌ No matched rule - fell through to default");
        }
    }
    
    println!("\\n🔧 Debugging: Manual regex testing");
    
    // Let's manually test the patterns
    use regex::Regex;
    
    let destructive_pattern = r".*(?:delete|destroy|remove|rm|kill|terminate|drop|truncate|purge).*";
    let regex = Regex::new(destructive_pattern).unwrap();
    
    let test_tool = "file_delete_user_data";
    println!("Testing '{}' against pattern '{}'", test_tool, destructive_pattern);
    println!("Manual regex match: {}", regex.is_match(test_tool));
    
    let credential_pattern = r".*(?:password|key|token|secret|credential|auth|login|oauth).*";
    let regex2 = Regex::new(credential_pattern).unwrap();
    
    let test_tool2 = "password_reset_user";
    println!("Testing '{}' against pattern '{}'", test_tool2, credential_pattern);
    println!("Manual regex match: {}", regex2.is_match(test_tool2));
    
    println!("\\n✅ Debug bloom filter test completed");
}