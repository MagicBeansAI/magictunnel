# MagicTunnel Architecture

## Overview

MagicTunnel is an intelligent bridge between MCP (Model Context Protocol) clients and diverse agents/endpoints. It provides a single, smart tool discovery interface that can find the right tool for any request, map parameters, and proxy the call automatically.

## High-Level Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │───▶│  MagicTunnel    │───▶│  Agents &       │
│  (Any Client)   │    │  (This Project) │    │  Endpoints      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │                        │
                              ▼                        ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │ Capability      │    │ External MCP    │
                       │ Registry        │    │ Servers         │
                       └─────────────────┘    └─────────────────┘
```

## Core Components

### 1. MCP Server Interface (Multi-Protocol Streaming Support)
- **Tool Discovery**: Lists available capabilities as MCP tools
- **Tool Execution**: Routes tool calls to actual agents/endpoints with streaming support
- **Resource Management**: Handles resource creation and access
- **Protocol Compliance**: Full MCP specification support with multiple streaming protocols:
  - **WebSocket**: Real-time bidirectional communication (`/mcp/ws`)
  - **Server-Sent Events**: Legacy streaming support (`/mcp/stream`)
  - **HTTP Streaming**: Progressive tool execution results (`/mcp/call/stream`)
  - **gRPC Streaming**: High-performance binary streaming with concurrent server architecture

### 2. Capability Registry (Flexible File Organization)
- **Flexible Structure**: Support any number of YAML files organized as teams prefer
- **Simple Tool Definitions**: Just name, description, input schema, and routing configuration
- **Custom Organization**: Teams can organize by domain, team ownership, or any structure that makes sense
- **Dynamic Discovery**: Automatically discover and load capabilities from configured directories

### 3. Agent Router - Advanced Multi-Agent Orchestration
- **Subprocess Agent**: Execute local commands, scripts, and system operations with environment control
- **HTTP Agent**: Call REST APIs, web services, and webhooks with retry logic and authentication
- **gRPC Agent**: Call gRPC services and microservices with protobuf support
- **SSE Agent**: Subscribe to Server-Sent Events streams for real-time data feeds
- **GraphQL Agent**: Execute GraphQL queries and mutations with variable substitution
- **LLM Agent**: Integrate with OpenAI, Anthropic, and other AI services with configurable models
- **WebSocket Agent**: Real-time bidirectional communication for interactive applications
- **Database Agent**: Execute SQL queries (PostgreSQL, SQLite) with connection pooling
- **MCP Proxy Agent**: Route to external MCP servers with intelligent conflict resolution
- **Advanced Parameter Substitution**: Handlebars-style templating with conditionals, loops, and environment variables

### 4. MCP Core Features - Full MCP Specification Compliance
- **MCP Logging System**: RFC 5424 syslog-compliant logging with 8 severity levels and rate limiting
- **MCP Notifications**: Event-driven notification system with resource subscriptions and broadcast channels
- **MCP Resource Management**: Complete file-based resource system with URI validation, provider architecture, and web interface
- **MCP Prompt Templates**: Complete template management with argument substitution, validation, and web interface
- **Web Dashboard Integration**: Full frontend interfaces for resources and prompts management
- **HTTP Endpoints**: Dynamic log level control via `/mcp/logging/setLevel`
- **WebSocket Integration**: Full JSON-RPC 2.0 message handling with capability negotiation
- **Thread Safety**: Concurrent operations with Arc<RwLock<T>> patterns for production use
- **Comprehensive Error Handling**: Timeout management, retry logic, and graceful failure recovery
- **Performance Optimization**: Concurrent execution with resource management and monitoring

### 5. Streaming Protocol Support
- **WebSocket**: Full-duplex real-time communication for interactive tools
- **Server-Sent Events**: One-way streaming for progress updates and notifications
- **HTTP Streaming**: Chunked responses for long-running tool executions
- **gRPC Streaming**: High-performance binary streaming with flow control

### 6. External MCP Integration - Unified External MCP Server Management
- **Claude Desktop Compatible**: Exact same configuration format as Claude Desktop
- **Process Management**: Automatic spawning and lifecycle management of MCP servers
- **Container Support**: Built-in Docker/Podman integration for containerized MCP servers
- **Automatic Discovery**: Tools and capabilities discovered automatically from spawned processes
- **Capability Generation**: Automatic generation of capability files for discovered tools
- **Hot Reload**: Configuration changes applied automatically without restart

### 7. Smart Tool Discovery System - Ultimate Clean Interface
- **Zero Visible Tools**: All tools hidden by default for clean interface
- **Smart Discovery**: Natural language tool discovery and execution
- **Visibility Management**: CLI-based tool visibility control (`magictunnel-visibility`)
- **Flexible Configuration**: Per-tool, per-file, and global visibility controls
- **Backward Compatible**: All tools remain fully functional through discovery

### 8. Custom GPT Integration - Full OpenAI Ecosystem Compatibility
- **OpenAPI 3.1.0 Generation**: Automatic conversion of all MCP tools to OpenAPI 3.1.0 specification with proper schema mapping
- **Dual OpenAPI Endpoints**: Full tools spec (`/dashboard/api/openapi.json`) and smart discovery only (`/dashboard/api/openapi-smart.json`)
- **Custom GPT Actions Ready**: Direct integration with Custom GPT Actions, optimized for 30-operation limit with smart discovery
- **REST API Endpoints**: All tools accessible via `/dashboard/api/tools/{name}/execute` with JSON request/response
- **Real-time Schema Updates**: OpenAPI spec reflects current enabled tools dynamically with proper MCP-to-OpenAPI conversion
- **Production Ready**: Tested with real tool execution, proper error handling, and OpenAPI 3.1.0 compliance

## Architectural Decisions

### MCP Logging System Architecture

The MCP logging system provides RFC 5424 syslog-compliant logging with real-time notifications:

```rust
/// MCP-compliant logger with rate limiting and notifications
pub struct McpLogger {
    name: String,
    level: Arc<RwLock<LogLevel>>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
    sender: broadcast::Sender<JsonRpcNotification>,
}

/// 8 RFC 5424 syslog severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,      // 7 - Lowest severity
    Info,       // 6
    Notice,     // 5
    Warning,    // 4
    Error,      // 3
    Critical,   // 2
    Alert,      // 1
    Emergency,  // 0 - Highest severity
}
```

**Key Features**:
- **Rate Limiting**: 100 messages per minute per logger to prevent DoS attacks
- **Thread Safety**: Arc<RwLock<T>> for concurrent access across async tasks
- **Broadcast Notifications**: Real-time log message delivery via tokio::sync::broadcast
- **Structured Logging**: JSON-formatted messages with timestamps and metadata
- **Dynamic Level Control**: HTTP endpoint `/mcp/logging/setLevel` for runtime configuration

### MCP Notifications System Architecture

The notification system provides event-driven updates for MCP clients:

```rust
/// MCP notification manager with resource subscriptions
pub struct McpNotificationManager {
    sender: broadcast::Sender<JsonRpcNotification>,
    resource_subscriptions: Arc<RwLock<HashSet<String>>>,
    capabilities: NotificationCapabilities,
    stats: Arc<RwLock<NotificationStats>>,
}

/// Notification types supported
pub enum NotificationEvent {
    ResourcesListChanged,    // Resource list updated
    PromptsListChanged,      // Prompt templates updated
    ResourceUpdated(String), // Specific resource changed
    ServerStatus(String),    // Server status change
    Custom(String, Value),   // Custom application notifications
}
```

**Key Features**:
- **Resource Subscriptions**: Clients can subscribe to specific resource URI updates
- **List Change Notifications**: Automatic notifications when resources/prompts lists change
- **Capability Flags**: Feature toggles for optional notification types
- **Statistics Tracking**: Subscription counts and notification delivery metrics
- **Broadcast Channels**: Efficient one-to-many notification delivery

### Agent Router Architecture

The router provides a unified interface for executing tool calls across nine different agent types:

```rust
#[async_trait]
pub trait AgentRouter: Send + Sync {
    async fn route_tool_call(&self, tool_call: &ToolCall, routing_config: &RoutingConfig) -> AgentResult;
}

pub struct DefaultAgentRouter {
    http_client: reqwest::Client,
}

impl DefaultAgentRouter {
    // Nine agent types: subprocess, http, grpc, sse, graphql, llm, websocket, database, mcp_proxy
    async fn execute_subprocess(&self, config: &Value, parameters: &Value) -> AgentResult
    async fn execute_http(&self, config: &Value, parameters: &Value) -> AgentResult
    async fn execute_llm(&self, config: &Value, parameters: &Value) -> AgentResult
    async fn execute_websocket(&self, config: &Value, parameters: &Value) -> AgentResult
}
```

### Advanced Parameter Substitution System

**Handlebars-Style Templating with Conditionals**:
```rust
// Basic substitution: {{parameter_name}}
substitute_parameters("echo {{message}}", &json!({"message": "hello"}))
// Result: "echo hello"

// Conditional logic: {{condition ? 'true_value' : 'false_value'}}
substitute_parameters("{{case_sensitive ? '' : '-i'}}", &json!({"case_sensitive": false}))
// Result: "-i"

// Array iteration: {{#each array}}{{this}}{{/each}}
substitute_parameters("{{#each files}}--include={{this}} {{/each}}",
                     &json!({"files": ["*.rs", "*.toml"]}))
// Result: "--include=*.rs --include=*.toml "

// Environment variables: {{env.VARIABLE_NAME}}
substitute_parameters("{{env.API_KEY}}", &json!({}))
// Result: Value from environment variable API_KEY
```

### High-Performance YAML Discovery & Loading

**Design Goal**: Enterprise-scale capability registry with sub-millisecond lookups and near-instant hot-reloading.

#### Technology Stack
```toml
# High-performance file operations
notify = "6.0"           # Cross-platform file watching for hot-reload
rayon = "1.8"            # Data parallelism for CPU-bound operations
globset = "0.4"          # Compiled glob patterns (10x faster than runtime glob)

# Concurrent data structures
dashmap = "5.5"          # Lock-free concurrent HashMap for caching
arc-swap = "1.6"         # Atomic registry updates without locks
```

#### Architecture Pattern: Background Registry Service
```
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│   File Watcher      │───▶│   Registry Service  │───▶│   Cached Registry   │
│   (notify crate)    │    │   (Background)      │    │   (arc-swap)        │
└─────────────────────┘    └─────────────────────┘    └─────────────────────┘
           │                          │                          │
           ▼                          ▼                          ▼
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│   Glob Discovery    │    │   Parallel Parser   │    │   Lock-Free Reads   │
│   (globset)         │    │   (rayon + serde)   │    │   (dashmap cache)   │
└─────────────────────┘    └─────────────────────┘    └─────────────────────┘
```

#### Key Performance Features
- **Parallel Processing**: CPU-bound YAML parsing across all cores using `rayon`
- **Smart Caching**: Lock-free concurrent cache with `dashmap` for <1μs lookups
- **Hot-Reload**: Background file watching with debouncing and incremental updates
- **Glob Optimization**: Pre-compiled patterns with `globset` for efficient discovery
- **Atomic Updates**: Zero-downtime registry swapping with `arc-swap`

## Project Structure

```
magictunnel/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Library root
│   ├── config/              # Configuration management
│   │   ├── mod.rs           # Module organization and re-exports
│   │   └── config.rs        # Configuration types and loading
│   ├── error/               # Error handling
│   │   ├── mod.rs           # Module organization and re-exports
│   │   └── error.rs         # Error types and utilities
│   ├── mcp/                 # MCP protocol implementation
│   │   ├── mod.rs           # Module organization and re-exports
│   │   ├── server.rs        # MCP server with WebSocket and HTTP endpoints
│   │   ├── client.rs        # MCP client for external servers
│   │   ├── discovery.rs     # Server discovery and registry
│   │   ├── mapping.rs       # Tool mapping system
│   │   ├── proxy.rs         # Proxy manager coordination
│   │   ├── types.rs         # MCP protocol types and data structures
│   │   ├── logging.rs       # RFC 5424 syslog-compliant logging system
│   │   ├── notifications.rs # Event-driven notification system
│   │   ├── resources.rs     # Resource management system
│   │   └── prompts.rs       # Prompt template management
│   ├── registry/            # Capability registry
│   │   ├── mod.rs
│   │   ├── service.rs       # Registry service implementation
│   │   ├── loader.rs        # File discovery and loading
│   │   └── types.rs         # Registry types
│   └── routing/             # Tool call routing
│       ├── mod.rs
│       ├── router.rs        # Routing logic
│       ├── types.rs         # Agent types
│       ├── enhanced_router.rs      # Enhanced routing with middleware
│       ├── middleware.rs           # Routing middleware (logging, metrics)
│       ├── retry.rs               # Retry logic and policies
│       ├── timeout.rs             # Timeout configuration and handling
│       └── substitution.rs        # Parameter substitution system

├── data/                    # Default capability directory
├── tests/                   # Integration tests
├── docs/                    # Documentation
├── config.yaml              # Default configuration
├── Cargo.toml               # Rust project configuration
├── rustfmt.toml             # Code formatting rules
├── clippy.toml              # Linting configuration
└── Makefile                 # Development commands
```

## Streaming Protocols

### Protocol Overview

#### 1. WebSocket (`/mcp/ws`)
- **Connection**: Persistent bidirectional connection
- **Protocol**: JSON-RPC 2.0 over WebSocket
- **Message Types**: Request, Response, Notification
- **Features**: 
  - Real-time tool execution with streaming results
  - Server-initiated notifications (resource updates, logs)
  - Connection keep-alive with heartbeat
  - Automatic reconnection support
- **Use Cases**: Interactive clients, real-time dashboards, Claude Desktop integration

#### 2. Server-Sent Events - SSE (`/mcp/stream`)
- **Connection**: One-way server-to-client streaming
- **Protocol**: SSE with JSON data events
- **Message Types**: Data events, heartbeat, connection status
- **Features**:
  - Long-running tool execution with progress updates
  - Automatic reconnection built into browser SSE API
  - Lower overhead than WebSocket for one-way communication
  - HTTP-compatible (works through proxies)
- **Use Cases**: Progress monitoring, log tailing, status updates

#### 3. HTTP Streaming (`/mcp/call/stream`)
- **Connection**: HTTP request with chunked response
- **Protocol**: HTTP with Transfer-Encoding: chunked
- **Message Types**: JSON chunks with partial results
- **Features**:
  - Progressive results for long-running operations
  - Compatible with standard HTTP clients
  - Graceful fallback for non-WebSocket environments
  - Timeout and cancellation support
- **Use Cases**: Batch processing, file operations, data exports

#### 4. gRPC Streaming
- **Connection**: HTTP/2 with Protocol Buffers
- **Protocol**: gRPC with three streaming modes
- **Message Types**: Unary, Server Streaming, Bidirectional Streaming
- **Features**:
  - High-performance binary protocol
  - Type-safe with protobuf schema validation
  - Flow control and backpressure handling
  - Load balancing and service discovery
- **Use Cases**: Microservice integration, high-throughput scenarios, enterprise systems

### Architecture Benefits

#### Why Actix-web?
- **Performance**: High-throughput async runtime with minimal overhead
- **WebSocket Support**: Native WebSocket support with JSON-RPC message handling
- **Streaming**: Built-in support for chunked responses and SSE
- **Middleware**: Rich ecosystem for authentication, logging, and metrics
- **Concurrent Architecture**: Run HTTP and gRPC servers simultaneously

#### Protocol Selection Strategy
1. **WebSocket**: Primary protocol for interactive clients
2. **HTTP**: Simple integrations and testing
3. **SSE**: One-way streaming without WebSocket complexity
4. **gRPC**: High-performance scenarios and microservice integration

## Key Design Principles

- **Flexible File Organization**: Support any number of capability files in any structure
- **Simple Tool Definitions**: No complex atomic/composite classifications
- **Routing-Focused**: Core purpose is routing tool calls, not orchestration
- **MCP Compliant**: Full compatibility with MCP protocol standards
- **Hot-Reloadable**: Dynamic capability discovery and updates
- **Comprehensive Testing**: All streaming protocols validated with performance benchmarks
- **Enterprise Ready**: Authentication, monitoring, and production deployment support
- **Developer Friendly**: Clear documentation, comprehensive examples, and intuitive configuration