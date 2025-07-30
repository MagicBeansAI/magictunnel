//! HTTP MCP Client Usage Example
//!
//! This example demonstrates how to use the HttpMcpClient to connect to
//! external MCP services over HTTP.

use magictunnel::mcp::{HttpMcpClient, HttpClientConfig, HttpAuthConfig};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (basic setup)
    println!("Note: For full logging, add tracing-subscriber dependency and use tracing_subscriber::init()");

    println!("🚀 HTTP MCP Client Example");
    println!("==========================");

    // Example 1: Basic HTTP client (no authentication)
    println!("\n📡 Example 1: Basic HTTP client");
    let basic_config = HttpClientConfig {
        base_url: "https://api.example.com/mcp".to_string(),
        auth: HttpAuthConfig::None,
        timeout: 30,
        retry_attempts: 3,
        retry_delay_ms: 1000,
        max_idle_connections: Some(10),
        idle_timeout: Some(60),
    };

    let basic_client = HttpMcpClient::new(basic_config, "example-basic".to_string())
        .expect("Failed to create basic HTTP client");

    println!("✅ Created basic HTTP MCP client for: {}", basic_client.service_id());

    // Example 2: HTTP client with Bearer token authentication
    println!("\n🔐 Example 2: HTTP client with Bearer authentication");
    let bearer_config = HttpClientConfig {
        base_url: "https://api.secure.com/mcp".to_string(),
        auth: HttpAuthConfig::Bearer {
            token: "your-bearer-token-here".to_string(),
        },
        timeout: 45,
        retry_attempts: 5,
        retry_delay_ms: 2000,
        max_idle_connections: Some(20),
        idle_timeout: Some(90),
    };

    let bearer_client = HttpMcpClient::new(bearer_config, "secure-service".to_string())
        .expect("Failed to create Bearer auth HTTP client");

    println!("✅ Created Bearer auth HTTP MCP client for: {}", bearer_client.service_id());

    // Example 3: HTTP client with API Key authentication
    println!("\n🔑 Example 3: HTTP client with API Key authentication");
    let api_key_config = HttpClientConfig {
        base_url: "https://api.apikey.com/mcp".to_string(),
        auth: HttpAuthConfig::ApiKey {
            header: "X-API-Key".to_string(),
            key: "your-api-key-here".to_string(),
        },
        timeout: 60,
        retry_attempts: 2,
        retry_delay_ms: 500,
        max_idle_connections: Some(5),
        idle_timeout: Some(30),
    };

    let api_key_client = HttpMcpClient::new(api_key_config, "api-key-service".to_string())
        .expect("Failed to create API Key HTTP client");

    println!("✅ Created API Key HTTP MCP client for: {}", api_key_client.service_id());

    // Example 4: HTTP client with Basic authentication
    println!("\n🔒 Example 4: HTTP client with Basic authentication");
    let basic_auth_config = HttpClientConfig {
        base_url: "https://api.basic.com/mcp".to_string(),
        auth: HttpAuthConfig::Basic {
            username: "your-username".to_string(),
            password: "your-password".to_string(),
        },
        ..Default::default()
    };

    let basic_auth_client = HttpMcpClient::new(basic_auth_config, "basic-auth-service".to_string())
        .expect("Failed to create Basic auth HTTP client");

    println!("✅ Created Basic auth HTTP MCP client for: {}", basic_auth_client.service_id());

    // Example 5: Demonstrating client operations (commented out since we don't have a real server)
    println!("\n🛠️  Example 5: Client operations (simulated)");
    println!("Note: The following operations would work with a real MCP HTTP service:");
    
    println!("  📋 Listing tools:");
    println!("    let tools = client.list_tools().await?;");
    println!("    println!(\"Found {{}} tools\", tools.len());");

    println!("  🔧 Calling a tool:");
    println!("    let result = client.call_tool(\"example_tool\", json!({{\"param\": \"value\"}})).await?;");
    println!("    println!(\"Tool result: {{:?}}\", result);");

    println!("  🔄 Clearing cache:");
    println!("    client.clear_cache().await;");

    println!("  ❤️  Health check:");
    println!("    let is_healthy = client.health_check().await?;");
    println!("    println!(\"Service is healthy: {{}}\", is_healthy);");

    // Example 6: Configuration examples for different use cases
    println!("\n⚙️  Example 6: Configuration examples for different scenarios");
    
    // High-performance configuration
    let high_perf_config = HttpClientConfig {
        base_url: "https://api.highperf.com/mcp".to_string(),
        auth: HttpAuthConfig::None,
        timeout: 10,                    // Fast timeout
        retry_attempts: 1,              // Minimal retries
        retry_delay_ms: 100,            // Quick retry
        max_idle_connections: Some(50), // Large connection pool
        idle_timeout: Some(300),        // Long-lived connections
    };
    println!("🏎️  High-performance config: 10s timeout, 50 connections, minimal retries");

    // Resilient configuration
    let resilient_config = HttpClientConfig {
        base_url: "https://api.unreliable.com/mcp".to_string(),
        auth: HttpAuthConfig::None,
        timeout: 120,                   // Long timeout
        retry_attempts: 10,             // Many retries
        retry_delay_ms: 5000,           // Long retry delay
        max_idle_connections: Some(3),  // Small connection pool
        idle_timeout: Some(30),         // Short-lived connections
    };
    println!("🛡️  Resilient config: 120s timeout, 10 retries, long delays");

    // Real-world production configuration
    let production_config = HttpClientConfig {
        base_url: "https://api.production.com/mcp".to_string(),
        auth: HttpAuthConfig::Bearer {
            token: std::env::var("MCP_TOKEN").unwrap_or_default(),
        },
        timeout: 30,
        retry_attempts: 3,
        retry_delay_ms: 1000,
        max_idle_connections: Some(10),
        idle_timeout: Some(60),
    };
    println!("🏭 Production config: Environment token, balanced timeouts and retries");

    println!("\n✨ HTTP MCP Client examples completed!");
    println!("   To use with a real service, replace the URLs with actual MCP HTTP endpoints.");
    println!("   The client supports all standard MCP operations: list_tools, call_tool, etc.");

    Ok(())
}