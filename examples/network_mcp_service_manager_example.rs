//! Network MCP Service Manager Example
//!
//! This example demonstrates how to use the NetworkMcpServiceManager to manage
//! network-based MCP services configured via YAML.

use magictunnel::config::{ExternalMcpServersConfig, HttpServiceConfig, SseServiceConfig, HttpAuthType, SseAuthType};
use magictunnel::mcp::NetworkMcpServiceManager;
use std::collections::HashMap;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 Network MCP Service Manager Example");
    println!("======================================");
    println!("This demonstrates how to configure and manage network MCP services.\n");

    // Example 1: Create configuration programmatically
    println!("📋 Example 1: Programmatic configuration");
    
    let mut http_services = HashMap::new();
    let mut sse_services = HashMap::new();

    // Configure HTTP service (disabled for demo)
    http_services.insert("example_api".to_string(), HttpServiceConfig {
        enabled: false, // Disabled since we don't have a real endpoint
        base_url: "https://api.example.com/mcp".to_string(),
        auth: HttpAuthType::Bearer {
            token: "demo-token-123".to_string(),
        },
        timeout: 30,
        retry_attempts: 3,
        retry_delay_ms: 1000,
        max_idle_connections: Some(10),
        idle_timeout: Some(60),
    });

    // Configure SSE service (disabled for demo)
    sse_services.insert("streaming_service".to_string(), SseServiceConfig {
        enabled: false, // Disabled since we don't have a real endpoint
        base_url: "https://stream.example.com/mcp/events".to_string(),
        auth: SseAuthType::ApiKey {
            header: "X-API-Key".to_string(),
            key: "demo-api-key-456".to_string(),
        },
        single_session: true,
        connection_timeout: 30,
        request_timeout: 60,
        max_queue_size: 100,
        heartbeat_interval: 30,
        reconnect: true,
        max_reconnect_attempts: 10,
        reconnect_delay_ms: 1000,
        max_reconnect_delay_ms: 30000,
    });

    let config = ExternalMcpServersConfig {
        mcp_servers: None, // No process-based servers in this example
        http_services: Some(http_services),
        sse_services: Some(sse_services),
        websocket_services: None, // Not implemented yet
    };

    println!("✅ Created configuration with:");
    println!("   - 1 HTTP service (example_api)");
    println!("   - 1 SSE service (streaming_service)");
    println!("   - Both services disabled for demo purposes\n");

    // Example 2: Initialize the NetworkMcpServiceManager
    println!("🚀 Example 2: Initialize NetworkMcpServiceManager");
    
    let capabilities_dir = "./demo-capabilities".to_string();
    let manager = NetworkMcpServiceManager::new(config, capabilities_dir);
    
    println!("✅ Created NetworkMcpServiceManager");
    println!("   - Capabilities output: ./demo-capabilities\n");

    // Example 3: Initialize services (this would connect to real services if enabled)
    println!("🔧 Example 3: Initialize services");
    
    match manager.initialize().await {
        Ok(_) => {
            println!("✅ Service manager initialized successfully");
            
            // Get active services
            let active_services = manager.get_active_services().await;
            println!("   - Active services: {:?}", active_services);
            
            // Get all tools (would be empty since services are disabled)
            let all_tools = manager.get_all_tools().await;
            println!("   - Total services with tools: {}", all_tools.len());
            
            // Get health status
            let health_status = manager.get_health_status().await;
            println!("   - Service health status: {:?}", health_status);
        }
        Err(e) => {
            println!("⚠️  Service manager initialization completed (no services enabled): {}", e);
        }
    }

    println!("\n📚 Example 4: Configuration examples for YAML");
    println!("These configurations would go in external-mcp-servers.yaml:");
    println!();

    println!("# HTTP MCP Services");
    println!("httpServices:");
    println!("  production_api:");
    println!("    enabled: true");
    println!("    base_url: \"https://api.production.com/mcp\"");
    println!("    auth:");
    println!("      type: \"bearer\"");
    println!("      token: \"${{PRODUCTION_MCP_TOKEN}}\"");
    println!("    timeout: 45");
    println!("    retry_attempts: 5");
    println!();

    println!("# SSE MCP Services");
    println!("sseServices:");
    println!("  realtime_analytics:");
    println!("    enabled: true");
    println!("    base_url: \"https://stream.analytics.com/mcp/events\"");
    println!("    auth:");
    println!("      type: \"api_key\"");
    println!("      header: \"X-Analytics-Key\"");
    println!("      key: \"${{ANALYTICS_API_KEY}}\"");
    println!("    single_session: false");
    println!("    heartbeat_interval: 20");
    println!();

    println!("🎯 Example 5: How it integrates with MagicTunnel");
    println!("1. 📝 Configure services in external-mcp-servers.yaml");
    println!("2. 🚀 NetworkMcpServiceManager loads and connects to services");
    println!("3. 🔍 Services are discovered and tools are cataloged");
    println!("4. 📋 Capability files are generated for registry integration");
    println!("5. 🎛️  Smart Tool Discovery can route requests to network services");
    println!("6. ❤️  Health monitoring and metrics collection run continuously");
    println!();

    println!("🎉 Network MCP Service Manager example completed!");
    println!("   In a real deployment:");
    println!("   • Services would connect to actual HTTP/SSE MCP endpoints");
    println!("   • Tools would be discovered and made available via Smart Discovery");
    println!("   • All network services integrate seamlessly with process-based MCP servers");
    println!("   • Unified capability registry provides single interface to all tools");

    Ok(())
}