//! Simple pattern testing utility

use std::path::PathBuf;
use magictunnel::security::{PatternLoader, AllowlistService, AllowlistConfig, AllowlistAction};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 MagicTunnel Pattern System Test");
    
    // Get security directory
    let mut security_dir = PathBuf::from(".");
    security_dir.push("security");
    
    if !security_dir.exists() {
        println!("❌ Security directory not found at {:?}", security_dir);
        return Ok(());
    }
    
    // Test pattern loader
    println!("📁 Testing pattern loader...");
    let pattern_loader = PatternLoader::new(&security_dir);
    
    // Load capability patterns
    match pattern_loader.load_capability_patterns() {
        Ok(patterns) => {
            println!("✅ Loaded {} capability patterns", patterns.len());
            for (i, pattern) in patterns.iter().take(3).enumerate() {
                println!("  {}. {} (priority: {})", 
                    i + 1, 
                    pattern.rule.name.as_ref().unwrap_or(&"unnamed".to_string()),
                    pattern.rule.priority.unwrap_or(0)
                );
            }
        }
        Err(e) => {
            println!("❌ Failed to load capability patterns: {}", e);
        }
    }
    
    // Load global patterns
    match pattern_loader.load_global_patterns() {
        Ok(patterns) => {
            println!("✅ Loaded {} global patterns", patterns.len());
            for (i, pattern) in patterns.iter().take(3).enumerate() {
                println!("  {}. {} (priority: {})", 
                    i + 1, 
                    pattern.rule.name.as_ref().unwrap_or(&"unnamed".to_string()),
                    pattern.rule.priority.unwrap_or(0)
                );
            }
        }
        Err(e) => {
            println!("❌ Failed to load global patterns: {}", e);
        }
    }
    
    // Test service integration
    println!("\n🔧 Testing service integration...");
    let config = AllowlistConfig {
        enabled: true,
        default_action: AllowlistAction::Allow,
        emergency_lockdown: false,
        tools: HashMap::new(),
        servers: HashMap::new(),
        capability_patterns: Vec::new(),
        global_patterns: Vec::new(),
    };
    
    match AllowlistService::with_pattern_loader(config, &security_dir) {
        Ok(service) => {
            let loaded_config = service.get_config();
            println!("✅ Service created successfully");
            println!("   Capability patterns: {}", loaded_config.capability_patterns.len());
            println!("   Global patterns: {}", loaded_config.global_patterns.len());
            
            // Test pattern testing framework
            match service.test_patterns() {
                Ok(results) => {
                    println!("✅ Pattern tests completed");
                    println!("   Total tests: {}", results.total_tests());
                    println!("   Passed: {}", results.passed_tests());
                    println!("   Success rate: {:.1}%", results.success_rate() * 100.0);
                }
                Err(e) => {
                    println!("❌ Pattern tests failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create service: {}", e);
        }
    }
    
    println!("\n🎉 Pattern system test completed!");
    Ok(())
}