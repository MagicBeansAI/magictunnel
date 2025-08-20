//! Simple allowlist data file testing utility

use std::path::PathBuf;
use magictunnel::security::{AllowlistService, AllowlistConfig, AllowlistAction};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 MagicTunnel Enhanced Allowlist System Test");
    
    // Check for allowlist-data.yaml file
    let mut data_file = PathBuf::from(".");
    data_file.push("security");
    data_file.push("allowlist-data.yaml");
    
    if !data_file.exists() {
        println!("❌ Allowlist data file not found at {:?}", data_file);
        println!("💡 Make sure allowlist-data.yaml exists in the security/ directory");
        return Ok(());
    }
    
    // Test enhanced data file loading
    println!("📁 Testing enhanced data file approach...");
    
    // Create basic config
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
        data_file: "./security/allowlist-data.yaml".to_string(),
    };
    
    match AllowlistService::with_data_file(config, "./security/allowlist-data.yaml".to_string()) {
        Ok(service) => {
            let loaded_config = service.get_config();
            println!("✅ Service created successfully with enhanced data file approach");
            println!("   Data file: {}", loaded_config.data_file);
            println!("   Tool rules: {}", loaded_config.tools.len());
            println!("   Capability rules: {}", loaded_config.capability_patterns.len());
            
            // Test real-time pattern testing
            println!("\n🔧 Testing real-time pattern testing API...");
            let test_request = magictunnel::security::allowlist_data::RealTimePatternTestRequest {
                pattern: magictunnel::security::allowlist_data::TestPattern {
                    name: "test_pattern".to_string(),
                    regex: "test_.*".to_string(),
                    action: magictunnel::security::AllowlistAction::Allow,
                    scope: magictunnel::security::allowlist_data::PatternScope::Tools,
                    priority: 10,
                },
                test_tools: vec!["test_tool".to_string(), "prod_tool".to_string()],
                include_evaluation_chain: true,
            };
            
            match service.test_patterns_batch(vec![test_request]) {
                Ok(responses) => {
                    println!("✅ Pattern testing completed");
                    println!("   Total pattern responses: {}", responses.len());
                    if let Some(first_response) = responses.first() {
                        println!("   Tool test results: {}", first_response.tool_results.len());
                        println!("   Pattern tested: {}", first_response.pattern.name);
                    }
                }
                Err(e) => {
                    println!("❌ Pattern testing failed: {}", e);
                }
            }
            
            println!("✅ Enhanced allowlist system test completed successfully");
        }
        Err(e) => {
            println!("❌ Failed to create service: {}", e);
        }
    }
    
    println!("\n🎉 Enhanced allowlist system test completed!");
    Ok(())
}