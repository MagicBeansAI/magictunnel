//! SSE MCP Client Usage Example
//!
//! This example demonstrates how to use the SseMcpClient to connect to
//! external MCP services over Server-Sent Events (SSE).
//!
//! Note: This is a documentation example. To use these clients in practice,
//! import them from magictunnel::mcp and ensure the eventsource-client
//! dependency is properly configured.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📡 SSE MCP Client Example");
    println!("========================");
    println!("This example shows how to configure and use SSE MCP clients.");
    println!();

    // Example 1: Basic SSE client configuration
    println!("🔗 Example 1: Basic SSE client configuration");
    println!("```rust");
    println!("use magictunnel::mcp::{{SseMcpClient, SseClientConfig, SseAuthConfig}};");
    println!();
    println!("let basic_config = SseClientConfig {{");
    println!("    base_url: \"https://api.example.com/mcp/events\".to_string(),");
    println!("    auth: SseAuthConfig::None,");
    println!("    single_session: true,");
    println!("    connection_timeout: 30,");
    println!("    request_timeout: 60,");
    println!("    max_queue_size: 100,");
    println!("    heartbeat_interval: 30,");
    println!("    reconnect: true,");
    println!("    max_reconnect_attempts: 5,");
    println!("    reconnect_delay_ms: 1000,");
    println!("    max_reconnect_delay_ms: 30000,");
    println!("}};");
    println!();
    println!("let client = SseMcpClient::new(basic_config, \"example-sse\".to_string())?;");
    println!("client.connect().await?;");
    println!("let tools = client.list_tools().await?;");
    println!("```");
    println!();

    // Example 2: SSE client with Bearer authentication
    println!("🔐 Example 2: SSE client with Bearer authentication");
    println!("```rust");
    println!("let bearer_config = SseClientConfig {{");
    println!("    base_url: \"https://stream.secure.com/mcp/events\".to_string(),");
    println!("    auth: SseAuthConfig::Bearer {{");
    println!("        token: \"your-bearer-token-here\".to_string(),");
    println!("    }},");
    println!("    single_session: false, // Multi-session capable");
    println!("    connection_timeout: 45,");
    println!("    request_timeout: 120,");
    println!("    // ... other config options");
    println!("    ..Default::default()");
    println!("}};");
    println!("```");
    println!();

    // Example 3: API Key authentication
    println!("🔑 Example 3: API Key authentication");
    println!("```rust");
    println!("let api_key_config = SseClientConfig {{");
    println!("    base_url: \"https://stream.apikey.com/mcp/events\".to_string(),");
    println!("    auth: SseAuthConfig::ApiKey {{");
    println!("        header: \"X-API-Key\".to_string(),");
    println!("        key: \"your-api-key-here\".to_string(),");
    println!("    }},");
    println!("    ..Default::default()");
    println!("}};");
    println!("```");
    println!();

    // Example 4: Query Parameter authentication
    println!("🔗 Example 4: Query Parameter authentication");
    println!("```rust");
    println!("let query_param_config = SseClientConfig {{");
    println!("    base_url: \"https://stream.query.com/mcp/events\".to_string(),");
    println!("    auth: SseAuthConfig::QueryParam {{");
    println!("        param: \"token\".to_string(),");
    println!("        value: \"your-query-token-here\".to_string(),");
    println!("    }},");
    println!("    ..Default::default()");
    println!("}};");
    println!("```");
    println!();

    // Example 5: Client operations
    println!("🛠️  Example 5: Client operations");
    println!("```rust");
    println!("// Connect to SSE stream");
    println!("client.connect().await?;");
    println!();
    println!("// List available tools");
    println!("let tools = client.list_tools().await?;");
    println!("println!(\"Found {{}} tools\", tools.len());");
    println!();
    println!("// Call a tool");
    println!("let result = client.call_tool(\"example_tool\", json!({{\"param\": \"value\"}})).await?;");
    println!("println!(\"Tool result: {{:?}}\", result);");
    println!();
    println!("// Check connection state");
    println!("let state = client.connection_state().await;");
    println!("println!(\"Connection state: {{:?}}\", state);");
    println!();
    println!("// Health check");
    println!("let is_healthy = client.health_check().await?;");
    println!("println!(\"Service is healthy: {{}}\", is_healthy);");
    println!();
    println!("// Disconnect");
    println!("client.disconnect().await?;");
    println!("```");
    println!();

    // Configuration examples
    println!("⚙️  Configuration Examples");
    println!("========================");
    println!();
    
    println!("🚀 High-throughput configuration:");
    println!("```rust");
    println!("SseClientConfig {{");
    println!("    max_queue_size: 1000,      // Large queue for high volume");
    println!("    heartbeat_interval: 10,     // Frequent heartbeats");
    println!("    connection_timeout: 10,     // Fast connection");
    println!("    request_timeout: 30,        // Quick timeouts");
    println!("    max_reconnect_attempts: 20, // Many reconnection attempts");
    println!("    // ...other settings");
    println!("}}");
    println!("```");
    println!();

    println!("🛡️  Resilient configuration:");
    println!("```rust");
    println!("SseClientConfig {{");
    println!("    connection_timeout: 120,     // Long connection timeout");
    println!("    request_timeout: 300,        // Very long request timeout");
    println!("    max_reconnect_attempts: 0,   // Unlimited reconnection attempts");
    println!("    reconnect_delay_ms: 5000,    // Long delays between reconnects");
    println!("    max_reconnect_delay_ms: 300000, // Up to 5 minutes between attempts");
    println!("    // ...other settings");
    println!("}}");
    println!("```");
    println!();

    println!("⚡ Multi-session configuration:");
    println!("```rust");
    println!("SseClientConfig {{");
    println!("    single_session: false,      // Multi-session support");
    println!("    max_queue_size: 10,         // Small queue (not needed for multi-session)");
    println!("    // ...other settings");
    println!("}}");
    println!("```");
    println!();

    println!("✨ SSE MCP Client features:");
    println!("• 🔗 Real-time streaming communication via Server-Sent Events");
    println!("• 🔐 Multiple authentication methods (Bearer, API Key, Query Parameters)");
    println!("• 📦 Single-session request queuing for reliable processing");
    println!("• 🔄 Automatic reconnection with exponential backoff");
    println!("• ❤️  Heartbeat monitoring for connection health");
    println!("• ⚙️  Flexible configuration for various use cases");
    println!();
    println!("To use with a real service, replace URLs with actual SSE MCP endpoints.");
    println!("The client supports all standard MCP operations: list_tools, call_tool, etc.");

    Ok(())
}