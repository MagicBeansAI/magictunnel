# Testing Guide

## Overview

MagicTunnel includes comprehensive test coverage to ensure reliability and protocol compliance with 500+ tests across multiple test suites, including complete LLM Backend APIs test coverage.

## Test Architecture

### Test Organization
- **Unit Tests**: Individual component testing (in `src/` files)
- **Integration Tests**: Cross-component testing (in `tests/` directory)
- **Performance Tests**: Load testing and benchmarking
- **Protocol Tests**: MCP specification compliance verification
- **Capability Tests**: Real-world integration scenarios

## Test Suites

### 1. Agent Router Tests (`agent_router_test.rs`)
- **Coverage**: Complete agent routing system validation (24 tests)
- **Scope**:
  - Subprocess agent execution and parameter substitution
  - HTTP agent with retry logic and authentication
  - gRPC agent with service calls and protobuf handling
  - SSE agent with event stream handling and filtering
  - GraphQL agent with query execution and variable substitution
  - LLM agent integration with multiple providers
  - WebSocket agent real-time communication
  - Database agent with SQL execution and connection management
  - Advanced parameter substitution with conditionals
  - Error handling and timeout scenarios

### 2. Data Structures Tests (`data_structures_test.rs`)
- **Coverage**: Comprehensive validation of all MCP types and structures (26 tests)
- **Scope**:
  - JSON-RPC 2.0 message format validation
  - MCP tool definition structure verification
  - Parameter schema validation and type checking
  - Error response format compliance
  - Serialization/deserialization consistency

### 3. Streaming Protocols Tests (`streaming_protocols_test.rs`)
- **Coverage**: All four streaming protocols (WebSocket, SSE, HTTP, gRPC)
- **Scope**:
  - WebSocket connection handling and message format
  - Server-Sent Events streaming and reconnection
  - HTTP chunked response streaming
  - gRPC unary, server streaming, and bidirectional streaming

### 4. Performance Tests (`performance_test.rs`)
- **Coverage**: Load testing and performance benchmarking
- **Scope**:
  - High-concurrency tool execution
  - Memory usage and leak detection
  - Response time measurement across protocols
  - Registry lookup performance validation

### 5. MCP Core Features Tests (39 tests)
- **MCP Logging Tests** (13 tests) - RFC 5424 syslog compliance and rate limiting
- **MCP Notifications Tests** (17 tests) - Event-driven notifications and subscriptions
- **MCP Integration Tests** (9 tests) - End-to-end server functionality

### 6. Authentication System Tests (26 tests)
- API key authentication validation
- OAuth 2.0 flow testing
- JWT token validation
- Permission-based access control
- Security middleware testing

### 7. GraphQL Capability Generator Tests (45 tests)
- SDL schema parsing with 100% GraphQL specification compliance
- Introspection JSON parsing with multiple format support
- Operation extraction (queries, mutations, subscriptions)
- Type system support (scalars, objects, enums, interfaces, unions, input objects)
- Schema extensions and directive processing
- Schema validation and safety analysis
- Authentication integration and real-world schema testing

### 8. Configuration Validation Tests (7 tests)
- Server configuration validation (host, port, timeout)
- Registry configuration validation (paths, security)
- Authentication configuration validation (API keys, OAuth)
- Logging configuration validation (levels, formats)
- Environment variable override testing
- Cross-dependency validation (port conflicts)
- File structure and security validation

### 9. External MCP Integration Tests (14 tests)
- MCP client connection handling
- Server discovery and registry
- Tool mapping and conflict resolution
- Connection management and health monitoring
- Tool aggregation functionality

### 10. Security Validation Tests (9 tests)
- Input sanitization testing
- Attack prevention validation
- Rate limiting verification
- Authentication bypass prevention
- CORS validation

### 11. LLM Backend APIs Test Coverage (60+ tests) - NEW in v0.3.4 ✅
- **Elicitation Service API Tests** (10 tests) - Metadata extraction, parameter validation, and batch processing
- **Sampling Service API Tests** (12 tests) - Tool description enhancement and content generation  
- **Enhanced Resource Management API Tests** (12 tests) - Filtering, pagination, content reading, and provider management
- **Enhanced Prompt Management API Tests** (14 tests) - CRUD operations, template management, and content generation
- **Enhanced Ranking and Discovery Tests** (12 tests) - Updated ranking algorithms with LLM integration and hybrid AI intelligence
- **LLM Backend APIs Integration Tests** (5 tests) - Cross-service workflows and data consistency validation

## Capability Testing Files

Comprehensive testing capabilities demonstrating all agent routing features:

- **`capabilities/testing/agent_routing_showcase.yaml`** - All nine agent types with advanced examples
- **`capabilities/grpc/grpc_services.yaml`** - gRPC service integration examples
- **`capabilities/sse/sse_streams.yaml`** - SSE stream subscription examples
- **`capabilities/graphql/graphql_services.yaml`** - GraphQL query and mutation examples
- **`capabilities/testing/error_handling_showcase.yaml`** - Error scenarios and edge cases
- **`capabilities/testing/performance_showcase.yaml`** - Performance benchmarking tools
- **`capabilities/testing/real_world_integrations.yaml`** - Production-ready integration examples

## Running Tests

### All Tests
```bash
cargo test
```

### Specific Test Suite
```bash
cargo test --test agent_router_test
cargo test --test data_structures_test
cargo test --test mcp_server_test
cargo test --test grpc_integration_test
```

### Specific Test Category
```bash
# Run MCP core features tests (39 tests total)
cargo test mcp::logging_tests --lib         # 13 logging tests
cargo test mcp::notifications_tests --lib   # 17 notification tests
cargo test mcp::integration_tests --lib     # 9 integration tests
cargo test mcp --lib                        # All 39 MCP tests

# Run capability generator tests
cargo test registry::graphql_generator::tests --lib  # 45 GraphQL generator tests
cargo test registry::openapi_generator::tests --lib  # 13 OpenAPI generator tests

# Test capability file parsing
cargo test yaml_parsing

# Authentication tests
cargo test auth --lib

# Smart discovery tests
cargo test discovery --lib

# LLM Backend APIs tests (v0.3.4)
cargo test --test elicitation_service_api_test     # 10 elicitation service tests
cargo test --test tool_enhancement_service_api_test        # 12 tool enhancement service tests
cargo test --test enhanced_resource_management_api_test  # 12 resource management tests
cargo test --test enhanced_prompt_management_api_test    # 14 prompt management tests  
cargo test --test enhanced_ranking_discovery_test        # 12 ranking and discovery tests
cargo test --test llm_backend_apis_integration_test      # 5 integration tests
```

### Specific Test
```bash
cargo test test_subprocess_agent
```

### With Output
```bash
cargo test -- --nocapture
```

### Performance Tests Only
```bash
cargo test --test performance_test --release
```

### Run Tests with Specific Features
```bash
# Test with authentication enabled
cargo test --features "auth"

# Test with TLS enabled
cargo test --features "tls"

# Test with all features
cargo test --all-features
```

## Test Coverage Metrics

- **📊 Data Structures Tests** (26 tests) - Comprehensive validation of all MCP types and structures
- **🔧 Integration Tests** (5 tests) - Configuration and CLI validation
- **⚙️ Configuration Validation Tests** (7 tests) - Comprehensive configuration validation
- **📡 MCP Server Tests** (14 tests) - JSON-RPC compliance and message format validation
- **🚀 gRPC Integration Tests** (6 tests) - Complete gRPC streaming protocol validation
- **🤖 Agent Router Tests** (24 tests) - Complete agent routing system validation
- **🔧 GraphQL Capability Generator Tests** (45 tests) - Complete GraphQL schema processing
- **🔍 MCP Core Features Tests** (39 tests) - Complete MCP specification compliance
  - **MCP Logging Tests** (13 tests) - RFC 5424 syslog compliance and rate limiting
  - **MCP Notifications Tests** (17 tests) - Event-driven notifications and subscriptions
  - **MCP Integration Tests** (9 tests) - End-to-end server functionality
- **🛡️ Authentication Tests** (26 tests) - Complete security validation
- **🔒 Security Tests** (9 tests) - Input validation and attack prevention
- **🌐 External MCP Tests** (14 tests) - External server integration
- **📈 Performance Tests** (Variable) - Load testing and benchmarking

**Total**: 500+ tests with comprehensive coverage across all system components including complete LLM Backend APIs coverage

## Writing Tests

### Unit Test Example
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tool_execution() {
        let router = DefaultAgentRouter::new();
        let tool_call = ToolCall {
            name: "test_tool".to_string(),
            arguments: json!({"input": "test"}),
        };
        
        let config = json!({
            "type": "subprocess",
            "command": "echo",
            "args": ["{{input}}"]
        });
        
        let result = router.route_tool_call(&tool_call, &config).await;
        assert!(result.is_ok());
    }
}
```

### Integration Test Example
```rust
// tests/integration_test.rs
use magictunnel::*;

#[tokio::test]
async fn test_mcp_server_integration() {
    let config = Config::default();
    let server = McpServer::new(config).await.unwrap();
    
    // Test server startup
    let addr = server.start().await.unwrap();
    assert!(addr.port() > 0);
    
    // Test tool listing
    let tools = server.list_tools().await.unwrap();
    assert!(!tools.is_empty());
}
```

### Performance Test Example
```rust
#[tokio::test]
async fn test_concurrent_tool_execution() {
    let router = DefaultAgentRouter::new();
    let mut handles = vec![];
    
    // Launch 100 concurrent tool executions
    for i in 0..100 {
        let router = router.clone();
        let handle = tokio::spawn(async move {
            let tool_call = ToolCall {
                name: format!("tool_{}", i),
                arguments: json!({"id": i}),
            };
            router.route_tool_call(&tool_call, &default_config()).await
        });
        handles.push(handle);
    }
    
    // Wait for all to complete
    let results = futures::future::join_all(handles).await;
    assert_eq!(results.len(), 100);
    
    // Verify all succeeded
    for result in results {
        assert!(result.unwrap().is_ok());
    }
}
```

## Test Configuration

### Environment Variables for Testing
```bash
# Enable test logging
export RUST_LOG=debug

# Test database URL
export TEST_DATABASE_URL=sqlite::memory:

# Mock API endpoints
export TEST_API_BASE_URL=http://localhost:8080

# Disable external dependencies in tests
export MAGICTUNNEL_TEST_MODE=true
```

### Test-Specific Configuration
```yaml
# test-config.yaml
server:
  host: "127.0.0.1"
  port: 0  # Random port for testing
  
registry:
  paths: ["./test_capabilities"]
  
logging:
  level: "debug"
  format: "text"

# Disable external integrations during testing
external_mcp:
  enabled: false
  
smart_discovery:
  enabled: false  # Disable LLM calls in tests
```

## Mock and Stub Objects

### HTTP Client Mock
```rust
use mockito::Server;

#[tokio::test]
async fn test_http_agent_with_mock() {
    let mut server = Server::new_async().await;
    
    let mock = server.mock("POST", "/api/test")
        .with_status(200)
        .with_body(r#"{"result": "success"}"#)
        .create_async()
        .await;
    
    let config = json!({
        "type": "http",
        "method": "POST",
        "url": format!("{}/api/test", server.url()),
        "body": r#"{"test": true}"#
    });
    
    let router = DefaultAgentRouter::new();
    let result = router.execute_http(&config, &json!({})).await;
    
    mock.assert_async().await;
    assert!(result.is_ok());
}
```

### Database Mock
```rust
use sqlx::sqlite::SqlitePool;

#[tokio::test]
async fn test_database_agent() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    
    // Setup test schema
    sqlx::query("CREATE TABLE test (id INTEGER, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    
    let config = json!({
        "type": "database",
        "query": "SELECT * FROM test WHERE id = {{id}}",
        "parameters": {"id": 1}
    });
    
    let router = DefaultAgentRouter::new();
    let result = router.execute_database(&config, &json!({"id": 1})).await;
    
    assert!(result.is_ok());
}
```

## Continuous Integration

### GitHub Actions Workflow
```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        
    - name: Run tests
      run: cargo test --all-features
      
    - name: Run integration tests
      run: cargo test --test integration_test
      
    - name: Run performance tests
      run: cargo test --test performance_test --release
      
    - name: Generate coverage report
      run: |
        cargo install cargo-tarpaulin
        cargo tarpaulin --out Xml
        
    - name: Upload coverage
      uses: codecov/codecov-action@v3
```

## Test Data Management

### Test Fixtures
```rust
// tests/fixtures/mod.rs
pub fn sample_tool_definition() -> serde_json::Value {
    json!({
        "name": "test_tool",
        "description": "A test tool",
        "inputSchema": {
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        },
        "routing": {
            "type": "subprocess",
            "command": "echo",
            "args": ["{{input}}"]
        }
    })
}

pub fn sample_config() -> Config {
    Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            ..Default::default()
        },
        ..Default::default()
    }
}
```

### Temporary Test Files
```rust
use tempfile::TempDir;

#[tokio::test]
async fn test_capability_loading() {
    let temp_dir = TempDir::new().unwrap();
    let capability_file = temp_dir.path().join("test.yaml");
    
    std::fs::write(&capability_file, r#"
tools:
  - name: "test_tool"
    description: "Test tool"
    inputSchema:
      type: object
"#).unwrap();
    
    let registry = CapabilityRegistry::new();
    let tools = registry.load_from_path(temp_dir.path()).await.unwrap();
    
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "test_tool");
}
```

## Debugging Tests

### Enable Detailed Logging
```bash
# Run specific test with debug logging
RUST_LOG=magictunnel=debug cargo test test_name -- --nocapture

# Run tests with trace logging
RUST_LOG=trace cargo test -- --nocapture
```

### Debug Specific Components
```bash
# Debug only agent router
RUST_LOG=magictunnel::routing=debug cargo test

# Debug only MCP server
RUST_LOG=magictunnel::mcp=debug cargo test

# Debug only registry
RUST_LOG=magictunnel::registry=debug cargo test
```

### Test with Different Configurations
```bash
# Test with authentication enabled
AUTH_ENABLED=true cargo test

# Test with external MCP disabled
EXTERNAL_MCP_ENABLED=false cargo test

# Test with smart discovery disabled
SMART_DISCOVERY_ENABLED=false cargo test
```

## Test Best Practices

### 1. Isolation
- Each test should be independent
- Use temporary directories for file operations
- Clean up resources after tests
- Use random ports for server tests

### 2. Deterministic
- Use fixed seeds for random operations
- Mock external dependencies
- Use consistent test data
- Avoid time-dependent assertions

### 3. Fast Execution
- Use in-memory databases for database tests
- Mock slow external services
- Parallelize independent tests
- Use appropriate timeouts

### 4. Comprehensive Coverage
- Test both success and error paths
- Test edge cases and boundary conditions
- Test with different configurations
- Include integration tests

### 5. Clear Documentation
- Use descriptive test names
- Add comments for complex test logic
- Document test setup requirements
- Explain expected behavior

## Troubleshooting Tests

### Common Issues

1. **Port conflicts**: Use port 0 for random port assignment
2. **File permissions**: Use temp directories with proper permissions
3. **Async issues**: Use `tokio::test` for async tests
4. **Resource cleanup**: Use `Drop` or explicit cleanup
5. **Timing issues**: Use appropriate timeouts and retries

### Test Environment Setup
```bash
# Install required tools
cargo install cargo-tarpaulin  # Coverage
cargo install cargo-nextest    # Faster test runner

# Run tests with nextest
cargo nextest run

# Generate coverage report
cargo tarpaulin --out Html
```

This comprehensive testing framework ensures MagicTunnel maintains high quality and reliability across all its components and features.