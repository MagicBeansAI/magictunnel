# MagicTunnel - Intelligent Agent Orchestration Bridge

## 🎯 Project Overview

**MagicTunnel** is an intelligent bridge that connects Model Context Protocol (MCP) clients to internal systems and external APIs through diverse agents and endpoints, enabling rapid capability discovery and execution without requiring full MCP server implementations for each capability.

**Current Version**: 0.2.48 - **OpenAPI 3.1 Custom GPT Integration Complete** ✅

## 🚀 Vision & Goals

### Core Vision
Create a **unified gateway** that allows any MCP client (Claude, GPT-4, custom agents) to discover and intelligently select from a rich ecosystem of capabilities, while maintaining the flexibility to integrate both custom agents and existing MCP servers.

### Key Goals
- **🔗 Universal Compatibility**: Support any MCP client through standard protocol compliance
- **⚡ Rapid Development**: Enable capability addition without per-tool MCP server development
- **🧠 Intelligent Orchestration**: Provide dependency-aware capability selection and composition
- **🌐 Ecosystem Integration**: Bridge custom agents with existing MCP servers
- **🏢 Enterprise Ready**: Support multi-tenant deployments with authentication and monitoring

## 🏗️ Architecture Overview

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

### Core Components

#### 1. **MCP Server Interface** (Multi-Protocol Streaming Support)
- **Tool Discovery**: Lists available capabilities as MCP tools
- **Tool Execution**: Routes tool calls to actual agents/endpoints with streaming support
- **Resource Management**: Handles resource creation and access
- **Protocol Compliance**: Full MCP specification support with multiple streaming protocols:
  - **WebSocket**: Real-time bidirectional communication (`/mcp/ws`)
  - **Server-Sent Events**: Legacy streaming support (`/mcp/stream`)
  - **HTTP Streaming**: Progressive tool execution results (`/mcp/call/stream`)
  - **gRPC Streaming**: ✅ **COMPLETE** - High-performance binary streaming with concurrent server architecture

#### 2. **Capability Registry** (Flexible File Organization)
- **Flexible Structure**: Support any number of YAML files organized as teams prefer
- **Simple Tool Definitions**: Just name, description, input schema, and routing configuration
- **Custom Organization**: Teams can organize by domain, team ownership, or any structure that makes sense
- **Dynamic Discovery**: Automatically discover and load capabilities from configured directories

#### 3. **Agent Router** ✅ **COMPLETE** - Advanced Multi-Agent Orchestration
- **Subprocess Agent**: Execute local commands, scripts, and system operations with environment control
- **HTTP Agent**: Call REST APIs, web services, and webhooks with retry logic and authentication
- **gRPC Agent**: Call gRPC services and microservices with protobuf support ✅ **NEW**
- **SSE Agent**: Subscribe to Server-Sent Events streams for real-time data feeds ✅ **NEW**
- **GraphQL Agent**: Execute GraphQL queries and mutations with variable substitution ✅ **NEW**
- **LLM Agent**: Integrate with OpenAI, Anthropic, and other AI services with configurable models
- **WebSocket Agent**: Real-time bidirectional communication for interactive applications
- **Database Agent**: Execute SQL queries (PostgreSQL, SQLite) with connection pooling ✅ **NEW**
- **MCP Proxy Agent**: Route to external MCP servers with intelligent conflict resolution ✅ **NEW**
- **Advanced Parameter Substitution**: Handlebars-style templating with conditionals, loops, and environment variables

#### 4. **MCP Core Features** ✅ **COMPLETE** - Full MCP Specification Compliance
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

#### 4. **Streaming Protocol Support**
- **WebSocket**: Full-duplex real-time communication for interactive tools
- **Server-Sent Events**: One-way streaming for progress updates and notifications
- **HTTP Streaming**: Chunked responses for long-running tool executions
- **gRPC Streaming**: High-performance binary streaming with flow control

#### 5. **Automatic Capability Generation** ✅ **NEW IN PHASE 3.5** - GraphQL Schema to MCP Tools
- **GraphQL Schema Parser**: Parse GraphQL SDL and introspection JSON with 100% specification compliance
- **Operation Extraction**: Automatically generate MCP tools from GraphQL queries, mutations, and subscriptions
- **Type System Support**: Complete support for all GraphQL types (scalars, objects, enums, interfaces, unions, input objects)
- **Advanced Features**: Schema extensions, directives, default values, multi-line arguments, and circular references
- **Schema Validation**: Comprehensive validation with type checking, completeness verification, and safety analysis
- **Authentication Integration**: Support for Bearer tokens, API keys, and custom headers
- **Real-World Ready**: Tested with complex schemas (9,951 lines, 484 operations) and production GraphQL APIs

#### 6. **External MCP Integration** ✅ **COMPLETE IN v0.2.48** - Unified External MCP Server Management
- **Claude Desktop Compatible**: Exact same configuration format as Claude Desktop
- **Process Management**: Automatic spawning and lifecycle management of MCP servers
- **Container Support**: Built-in Docker/Podman integration for containerized MCP servers
- **Automatic Discovery**: Tools and capabilities discovered automatically from spawned processes
- **Capability Generation**: Automatic generation of capability files for discovered tools
- **Hot Reload**: Configuration changes applied automatically without restart

#### 7. **External MCP System** ✅ **COMPLETE** - Unified External MCP Server Management
- **Claude Desktop Compatible**: Exact same configuration format as Claude Desktop
- **Process Management**: Automatic spawning and lifecycle management of MCP servers
- **Container Support**: Built-in Docker/Podman integration for containerized MCP servers
- **Automatic Discovery**: Tools and capabilities discovered automatically from spawned processes
- **Conflict Resolution**: Built-in strategies (`local_first`, `remote_first`, `prefix`) for tool name conflicts
- **Hot Reload**: Configuration changes applied automatically without restart
- **Modern Architecture**: Single integration point for all external MCP server management

#### 8. **Smart Tool Discovery System** ✅ **COMPLETE** - Ultimate Clean Interface
- **Zero Visible Tools**: All 83 tools hidden by default for clean interface
- **Smart Discovery**: Natural language tool discovery and execution
- **Visibility Management**: CLI-based tool visibility control (`magictunnel-visibility`)
- **Flexible Configuration**: Per-tool, per-file, and global visibility controls
- **Backward Compatible**: All tools remain fully functional through discovery

#### 9. **Custom GPT Integration** ✅ **COMPLETE** - Full OpenAI Ecosystem Compatibility
- **OpenAPI 3.1.0 Generation**: Automatic conversion of all MCP tools to OpenAPI 3.1.0 specification with proper schema mapping
- **Dual OpenAPI Endpoints**: Full tools spec (`/dashboard/api/openapi.json`) and smart discovery only (`/dashboard/api/openapi-smart.json`)
- **Custom GPT Actions Ready**: Direct integration with Custom GPT Actions, optimized for 30-operation limit with smart discovery
- **REST API Endpoints**: All tools accessible via `/dashboard/api/tools/{name}/execute` with JSON request/response
- **Real-time Schema Updates**: OpenAPI spec reflects current enabled tools dynamically with proper MCP-to-OpenAPI conversion
- **Production Ready**: Tested with real tool execution, proper error handling, and OpenAPI 3.1.0 compliance

#### 10. **Unified Architecture**
- **Direct Integration**: Route to custom agents and endpoints
- **External MCP Integration**: Unified management of external MCP servers ✅ **IMPLEMENTED**
- **Intelligent Selection**: Choose optimal routing based on capability metadata
- **Conflict-Free Operation**: Seamlessly handle tool name conflicts across sources

## 🎯 Value Propositions

### For Developers
- **⚡ Rapid Prototyping**: Add new capabilities via simple YAML configuration
- **🔧 No MCP Implementation**: Leverage existing agents without MCP server development
- **📊 Rich Metadata**: Access performance, cost, and reliability information
- **🔗 Dependency Management**: Automatic resolution of capability dependencies

### For Organizations
- **🏢 Enterprise Deployment**: Multi-tenant architecture with authentication
- **📈 Scalable Architecture**: Handle multiple clients and capability sources
- **🔒 Security**: API key management and access control
- **📊 Monitoring**: Comprehensive logging and performance tracking

### For the Ecosystem
- **🌐 Bridge Builder**: Connect MCP clients with non-MCP agents
- **🤝 Interoperability**: Enable collaboration between different agent frameworks
- **📚 Capability Marketplace**: Facilitate sharing and discovery of capabilities
- **🧠 Intelligent Layer**: Add orchestration intelligence to the MCP ecosystem

## 🌐 MCP Ecosystem Integration

### Overview

MagicTunnel positions itself as a **strategic gateway** in the Model Context Protocol (MCP) ecosystem, functioning as both an MCP server and client to create a unified, intelligent interface for diverse capabilities.

### MCP Ecosystem Architecture

The MCP ecosystem consists of several key components:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │◄──►│   MCP Server    │◄──►│   Resources     │
│  (Claude, GPT)  │    │  (Tool Provider)│    │  (Files, Data)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │                       ▼                       │
         │              ┌─────────────────┐              │
         │              │   MCP Server    │              │
         │              │  (Another Tool) │              │
         │              └─────────────────┘              │
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │    │   MCP Server    │    │   MCP Server    │
│  (Your App)     │    │  (Your Tool)    │    │  (Your Tool)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Dual Role Architecture

#### 1. As an MCP Server (Primary Role)
MagicTunnel acts as a **unified MCP server** that aggregates capabilities from multiple sources:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │───▶│  MagicTunnel    │───▶│  Non-MCP Agents │
│  (Claude, GPT)  │    │  (MCP Server)   │    │  (Bash, APIs)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌─────────────────┐
                       │ Capability      │
                       │ Registry        │
                       └─────────────────┘
```

#### 2. As an MCP Client (Proxy Role)
MagicTunnel can also act as an **MCP client** to proxy other MCP servers:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │───▶│  MagicTunnel    │───▶│  Other MCP      │
│  (Claude, GPT)  │    │  (MCP Proxy)    │    │  Servers        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │                       │
                              ▼                       ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │ Capability      │    │  MCP Server     │
                       │ Registry        │    │  (GitHub, etc.) │
                       └─────────────────┘    └─────────────────┘
```

### Hierarchical MCP Architecture

MagicTunnel creates a **hierarchical MCP architecture** by proxying multiple MCP servers:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │───▶│  MagicTunnel    │───▶│  MCP Server 1   │
│  (Claude, GPT)  │    │  (MCP Proxy)    │    │  (GitHub Tools) │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │ MCP Server 2    │    │ MCP Server 3    │
                       │ (File System)   │    │ (Database)      │
                       └─────────────────┘    └─────────────────┘
```

### Key Integration Benefits

#### 1. **Unified Interface**
- **Single MCP server** exposes all capabilities from multiple sources
- **Consistent tool discovery** across different providers
- **Unified authentication** and configuration management

#### 2. **Capability Aggregation**
- **Combine custom tools** with existing MCP servers
- **No need to reimplement** popular MCP server functionality
- **Leverage ecosystem** tools (GitHub, file system, database, etc.)

#### 3. **Intelligent Routing**
- **Smart tool selection** between multiple sources based on metadata
- **Fallback mechanisms** if one server becomes unavailable
- **Load balancing** across multiple server instances

#### 4. **Enhanced Metadata**
- **Enrich external tools** with cost, performance, and reliability information
- **Consistent categorization** across all capabilities
- **Unified search** and discovery interface

### Real-World Integration Examples

#### Example 1: GitHub + Custom Tools
```yaml
tools:
  # GitHub tools (proxied from external MCP server)
  - name: github_search_repos
    description: Search GitHub repositories
    source: mcp_proxy
    mcp_server: github-server
    routing:
      type: "mcp_proxy"
      server: "github-server"
    
  # Custom tools (direct routing)
  - name: execute_bash_command
    description: Execute bash commands
    routing:
      type: "subprocess"
      command: "bash"
      args: ["-c", "{command}"]
```

#### Example 2: Multi-Server Ecosystem
```yaml
tools:
  # File system tools (proxied)
  - name: read_file
    description: Read file contents
    source: mcp_proxy
    mcp_server: filesystem-server
    
  # Database tools (proxied)
  - name: query_database
    description: Execute SQL query
    source: mcp_proxy
    mcp_server: database-server
    
  # Custom analysis tools (direct)
  - name: weather_analysis
    description: Analyze weather data
    routing:
      type: "http"
      config:
        url: "https://api.weather.com/analyze"
        method: "POST"
```

### Implementation Status

#### ✅ **Phase 1: Direct Integration** - **COMPLETE**
- **MCP server implementation** with WebSocket, SSE, HTTP, and gRPC streaming
- **Direct agent routing** supporting subprocess, HTTP, LLM, and WebSocket agents
- **Capability registry** with flexible YAML-based configuration
- **Advanced tool discovery** with hot-reload and pattern matching

#### ✅ **Phase 2: MCP Core Features** - **COMPLETE**
- **MCP Resource Management** with read-only resource system
- **MCP Prompt Templates** with argument substitution and validation
- **MCP Logging System** with RFC 5424 syslog compliance
- **MCP Notifications** with event-driven notification system

#### ✅ **Phase 3.1: External MCP Integration** - **COMPLETE**
- **External MCP server management** with unified configuration
- **Process lifecycle management** for external MCP servers
- **Tool conflict resolution** with intelligent routing strategies
- **Automatic capability discovery** from external servers

### Strategic Ecosystem Positioning

#### Your Role in the Ecosystem
1. **Capability Aggregator**: Unify diverse tools under one intelligent MCP server
2. **Intelligent Router**: Make smart decisions about tool selection and execution
3. **Performance Optimizer**: Optimize for cost, performance, and reliability metrics
4. **Ecosystem Bridge**: Connect MCP clients with both MCP and non-MCP capabilities

#### Ecosystem Benefits
1. **For MCP Clients**: Single server providing comprehensive capabilities from multiple sources
2. **For MCP Servers**: Additional routing, caching, and optimization layer
3. **For Developers**: Easy capability addition and management without protocol complexity
4. **For Users**: Intelligent tool selection with transparent access to entire ecosystem

### Ecosystem Integration Value

This architecture positions MagicTunnel as a **strategic gateway** in the MCP ecosystem:

- **✅ Ecosystem Integration**: Seamlessly work with existing MCP servers
- **✅ Rapid Development**: Leverage existing tools without reimplementation
- **✅ Intelligent Selection**: Prove intelligent capability selection and routing
- **✅ Future Growth**: Easy addition of new capabilities and servers
- **✅ Market Position**: Strategic layer providing value across the entire MCP ecosystem

MagicTunnel becomes the **intelligent gateway** to the entire MCP ecosystem, providing unified access, smart routing, and enhanced capabilities while maintaining full compatibility with MCP standards.

## 🔧 GraphQL Capability Generator

MagicTunnel includes a comprehensive GraphQL capability generator that automatically converts GraphQL schemas into MCP tools, enabling seamless integration with GraphQL APIs.

### ✨ Key Features

#### **100% GraphQL Specification Compliance**
- **SDL Schema Parsing**: Parse GraphQL Schema Definition Language with complete syntax support
- **Introspection JSON**: Support for GraphQL introspection query results in multiple formats
- **Type System**: Full support for all GraphQL types (scalars, objects, enums, interfaces, unions, input objects)
- **Schema Extensions**: Support for `extend` keyword to merge type definitions
- **Directives**: Parse and process GraphQL directives for operation customization

#### **Advanced Schema Processing**
- **Multi-line Arguments**: Handle complex real-world schemas with multi-line field definitions
- **Schema Validation**: Comprehensive validation with type checking and safety analysis
- **Circular References**: Intelligent handling of circular type dependencies
- **Default Values**: Support for argument default values with type validation
- **Custom Scalars**: Recognition and handling of custom scalar types

#### **Intelligent Tool Generation**
- **Operation Extraction**: Automatically generate MCP tools from queries, mutations, and subscriptions
- **Parameter Mapping**: Convert GraphQL arguments to JSON Schema for MCP tool input validation
- **Authentication Integration**: Support for Bearer tokens, API keys, and custom headers
- **Naming Conventions**: Configurable tool naming with prefix support

#### **Production Ready**
- **Real-World Testing**: Tested with complex schemas (9,951 lines, 484 operations)
- **Comprehensive Testing**: 45 test cases covering all GraphQL features
- **Error Handling**: Robust error handling with detailed validation messages
- **Performance**: Optimized for large schemas and high-throughput scenarios

### 🚀 Usage Example

```bash
# Generate MCP tools from GraphQL schema
cargo run --bin graphql_generator -- \
  --endpoint "https://api.github.com/graphql" \
  --auth-header "Authorization: Bearer YOUR_TOKEN" \
  --prefix "github" \
  --output "capabilities/github_tools.yaml"
```

This automatically creates MCP tools for all GraphQL operations, making them available to any MCP client.

## 🧠 Smart Tool Discovery System

MagicTunnel features an advanced **Smart Tool Discovery System** that provides a clean, uncluttered interface while maintaining access to all capabilities through intelligent discovery. This system reduces N tools to 1 intelligent proxy tool, solving context overflow issues in MCP clients. ⭐ **Now with enhanced hybrid AI intelligence and comprehensive tool coverage.**

### ✨ Key Innovation

**The Problem:** Traditional MCP systems expose 50+ individual tools, causing context overflow in AI systems due to message limits.

**The Solution:** Smart Discovery reduces N tools to 1 intelligent proxy tool (`smart_tool_discovery`) that:
1. Analyzes natural language requests
2. Finds the best matching tool using hybrid search strategies  
3. Maps parameters from natural language to tool schema
4. Proxies the call to the actual tool
5. Returns results with discovery metadata

### 🏗️ Architecture Components

#### 1. **Smart Discovery Service** (SmartDiscoveryService)
- Main orchestration layer
- Handles tool selection and parameter mapping
- Manages caching and performance optimization

#### 2. **Multi-Strategy Search Engine**
- **Rule-based Search**: Keyword/fuzzy matching
- **Semantic Search**: Vector similarity using embeddings
- **LLM-based Search**: AI-powered tool selection
- **Hybrid Search**: Intelligent combination of all three

#### 3. **LLM Parameter Mapper** (LlmParameterMapper)
- Extracts parameters from natural language
- Maps to tool schema requirements
- Provides parameter validation and suggestions

#### 4. **Discovery Cache** (DiscoveryCache)
- Caches tool matches and LLM responses
- Optimizes performance for repeated queries
- Reduces API costs

#### 5. **Fallback Manager** (FallbackManager)
- Handles failures gracefully
- Provides alternative suggestions
- Learns from past interactions

### 🔍 Search Strategies

#### 1. Rule-based Search (Fast)
**Best for:** Exact tool names, common keywords, quick matches

**How it works:**
- Exact tool name matching with fuzzy fallback
- Keyword matching in tool descriptions
- Category-based classification (network, file, database)
- Typo tolerance and partial matching

#### 2. Semantic Search (Intelligent)
**Best for:** Natural language queries, conceptual matches, synonyms

**How it works:**
- Converts requests to embedding vectors
- Compares with pre-computed tool embeddings
- Uses cosine similarity for matching
- Supports multiple embedding models

**Supported Models:**
- **OpenAI**: `text-embedding-3-small` (1536 dims), `text-embedding-3-large` (3072 dims)
- **Ollama**: `nomic-embed-text` (768 dims) - Recommended for local development
- **External API**: Custom embedding services
- **Fallback Models**: `all-MiniLM-L6-v2` (384 dims), `all-mpnet-base-v2` (768 dims)

#### 3. LLM-based Search (Advanced)
**Best for:** Complex queries, disambiguation, multi-step reasoning

**How it works:**
- Uses AI models (OpenAI, Anthropic, Ollama) for tool selection
- Provides reasoning and confidence scores
- Handles ambiguous or complex requests
- Supports context-aware selection

#### 4. Hybrid Search (Recommended) ⭐ **Enhanced**
**Best for:** Production environments, optimal accuracy, robust fallback

**Three-Layer Hybrid Matching:**
1. **Semantic Search (30% weight)** - Uses embeddings for natural language understanding
2. **Rule-Based Matching (15% weight)** - Fast keyword and pattern matching  
3. **LLM Intelligence (55% weight)** - Advanced AI reasoning and context understanding

**Sequential Processing:**
```
User Request → Semantic Search → Rule-Based Search → LLM Evaluation → Combined Scoring → Tool Selection
```

**Key Improvements:**
- ✅ **Complete Tool Coverage**: All 94 tools are evaluated by all enabled methods
- ✅ **Optimized Weight Distribution**: LLM-First Strategy prioritizes AI intelligence (55%)
- ✅ **Cost-Effective LLM Usage**: Multi-criteria selection limits LLM to 30 tools maximum
- ✅ **Enhanced Observability**: Detailed reasoning shows contribution from each method

### 📊 Example Output

When a user requests "ping google.com", the system provides:

```json
{
  "discovery_reasoning": "Hybrid(Semantic: 0.732, Rule: 0.550, LLM: 0.900) = 0.797",
  "confidence_score": 0.7972411894798279,
  "tool_name": "check_network_connectivity",
  "parameters": {
    "host": "google.com"
  },
  "execution_result": {
    "success": true,
    "output": "PING google.com: 4 packets transmitted, 4 received"
  }
}
```

This shows:
- **Semantic**: Found good similarity (0.732)
- **Rule-based**: Matched keywords (0.550) 
- **LLM**: High confidence selection (0.900)
- **Final**: Combined weighted score (0.797)

### 🚀 Natural Language Interface

#### **Basic Usage**
Use the single `smart_tool_discovery` tool with natural language requests:

```json
{
  "name": "smart_tool_discovery",
  "arguments": {
    "request": "read the config.yaml file"
  }
}
```

**Web Dashboard Integration**: Access smart discovery through the web dashboard at `http://localhost:5173` with toggle between HTTP API and MCP protocol execution modes.

#### **Advanced Usage**
```json
{
  "name": "smart_tool_discovery", 
  "arguments": {
    "request": "make HTTP POST request with authentication",
    "context": "API endpoint is https://api.example.com/data, use Bearer token",
    "preferred_tools": ["http_client"],
    "confidence_threshold": 0.8,
    "include_error_details": true
  }
}
```

#### **Multiple Tool Suggestions**
```json
{
  "name": "smart_tool_discovery",
  "arguments": {
    "request": "backup files",
    "max_suggestions": 3,
    "include_reasoning": true
  }
}
```

**Response:**
```json
{
  "success": true,
  "result": {
    "suggestions": [
      {
        "tool_name": "file_backup",
        "confidence_score": 0.92,
        "reasoning": "Primary backup tool for file operations"
      },
      {
        "tool_name": "archive_files", 
        "confidence_score": 0.87,
        "reasoning": "Creates compressed archives of files"
      },
      {
        "tool_name": "sync_directory",
        "confidence_score": 0.75, 
        "reasoning": "Synchronizes files to backup location"
      }
    ]
  }
}
```

#### **Common Request Patterns**
- **File Operations**: "read the package.json file", "write data to output.txt"
- **HTTP Requests**: "make GET request to health endpoint", "send POST with JSON data"
- **Database Operations**: "query users table for active accounts"
- **System Tasks**: "check system health", "execute build script"
- **Network Testing**: "ping google.com to test connectivity"

### 🛠️ Parameter Extraction

#### **Automatic Parameter Mapping**
The system automatically extracts parameters from natural language:

**Input:** `"send a GET request to https://api.example.com/users"`

**Extracted Parameters:**
```json
{
  "method": "GET",
  "url": "https://api.example.com/users"
}
```

#### **Parameter Validation**
The system validates extracted parameters against tool schemas:

```json
{
  "parameter_extraction": {
    "status": "success",
    "extracted_parameters": {
      "url": "https://api.example.com/users",
      "method": "GET"
    },
    "validation_results": {
      "url": "valid",
      "method": "valid"
    },
    "missing_required": [],
    "suggestions": []
  }
}
```

#### **Parameter Suggestions**
When parameters are missing or invalid:

```json
{
  "parameter_extraction": {
    "status": "partial_success", 
    "extracted_parameters": {
      "url": "api.example.com"
    },
    "validation_results": {
      "url": "invalid - missing protocol"
    },
    "missing_required": ["method"],
    "suggestions": [
      "Add protocol to URL (http:// or https://)",
      "Specify HTTP method (GET, POST, PUT, DELETE)"
    ]
  }
}
```

### ⚙️ Configuration Modes

#### **Mode Selection**
```yaml
smart_discovery:
  tool_selection_mode: "hybrid"  # Choose your strategy
```

**Available modes:**
- `"rule_based"` - Fast keyword matching only
- `"semantic_based"` - Vector similarity only
- `"llm_based"` - AI-powered selection only
- `"hybrid"` - Intelligent combination (recommended)

#### **Complete Configuration**
```yaml
smart_discovery:
  enabled: true
  tool_selection_mode: "hybrid"
  default_confidence_threshold: 0.7
  max_tools_to_consider: 10
  max_high_quality_matches: 5
  high_quality_threshold: 0.95
  use_fuzzy_matching: true
  
  # LLM Tool Selection
  llm_tool_selection:
    enabled: true
    provider: "openai"
    model: "gpt-4o-mini"
    api_key_env: "OPENAI_API_KEY"
    timeout: 30
    max_retries: 3
    batch_size: 15
    max_context_tokens: 4000
  
  # LLM Parameter Mapping
  llm_mapper:
    enabled: true
    provider: "openai"
    model: "gpt-4o-mini"
    api_key_env: "OPENAI_API_KEY"
    timeout: 30
    max_retries: 3
  
  # Performance Caching
  cache:
    enabled: true
    max_tool_matches: 1000
    tool_match_ttl: 3600
    max_llm_responses: 500
    llm_response_ttl: 1800
    max_registry_entries: 100
    registry_ttl: 300
  
  # Fallback Strategy
  fallback:
    enabled: true
    max_suggestions: 3
    enable_learning: true
    enable_keyword_fallback: true
    enable_category_fallback: true
    enable_partial_match_fallback: true
  
  # Semantic Search
  semantic_search:
    enabled: true
    model_name: "openai:text-embedding-3-small"
    similarity_threshold: 0.55
    max_results: 10
    
    # Storage Configuration
    storage:
      embeddings_file: "./data/embeddings/tool_embeddings.bin"
      metadata_file: "./data/embeddings/tool_metadata.json"
      hash_file: "./data/embeddings/content_hashes.json"
      backup_count: 3
      auto_backup: true
      compression: true
    
    # Model Configuration
    model:
      cache_dir: "./data/models"
      device: "cpu"
      max_sequence_length: 512
      batch_size: 32
      normalize_embeddings: true
      
    # Performance Configuration
    performance:
      lazy_loading: true
      embedding_cache_size: 1000
      parallel_processing: true
      worker_threads: 4

# Embedding Management
embedding_manager:
  batch_size: 10
  check_interval_seconds: 300
  preserve_user_settings: true
  background_monitoring: true
  auto_save: true

# Visibility Configuration
visibility:
  hide_individual_tools: true      # Hide individual tools when smart discovery is enabled
  expose_smart_discovery_only: true # Only expose smart_tool_discovery
  allow_override: true             # Allow individual tools to override hidden setting
  default_hidden: false            # Default hidden state for new tools
```

#### **Environment Variables**
Configure LLM providers and semantic search settings:

```bash
# LLM API Keys
export OPENAI_API_KEY="your-openai-key"
export ANTHROPIC_API_KEY="your-anthropic-key"
export OLLAMA_BASE_URL="http://localhost:11434"

# Semantic Search Configuration
export MAGICTUNNEL_SEMANTIC_MODEL="openai:text-embedding-3-small"  # Override embedding model
export MAGICTUNNEL_EMBEDDING_FILE="./custom/path/embeddings.bin"    # Custom embedding file path
export MAGICTUNNEL_DISABLE_SEMANTIC="false"                         # Enable/disable semantic search

# For Ollama models
export OLLAMA_BASE_URL="http://localhost:11434"

# For custom embedding API
export EMBEDDING_API_URL="http://your-server:8080"
```

### 🏃‍♂️ Performance Optimization

#### **Caching Strategy**
The system uses multi-level caching:

1. **Tool Match Cache** - Caches search results
2. **LLM Response Cache** - Caches AI responses
3. **Registry Cache** - Caches tool registry data
4. **Embedding Cache** - Caches embedding vectors

#### **Cost Optimization**
- **Limited LLM Scope**: Maximum 30 tools evaluated by LLM
- **Strategic Selection**: Multi-criteria approach balances cost vs coverage
- **Caching**: LLM responses cached for repeated queries

#### **Speed**
- **Sequential Processing**: Optimized for accuracy over pure speed
- **Embedding Cache**: Fast semantic search with pre-computed embeddings
- **Rule-Based Speed**: Instant exact matching for common patterns

### 🛠️ Visibility Management CLI
MagicTunnel includes a powerful CLI tool (`magictunnel-visibility`) for managing tool visibility:

```bash
# Check current visibility status
cargo run --bin magictunnel-visibility -- -c config.yaml status

# Show detailed status with per-file breakdown
cargo run --bin magictunnel-visibility -- -c config.yaml status --detailed

# Hide specific tools
cargo run --bin magictunnel-visibility -- -c config.yaml hide-tool tool_name

# Show specific tools
cargo run --bin magictunnel-visibility -- -c config.yaml show-tool tool_name

# Hide all tools in a capability file
cargo run --bin magictunnel-visibility -- -c config.yaml hide-file capabilities/web/http_client.yaml

# Show all tools in a capability file
cargo run --bin magictunnel-visibility -- -c config.yaml show-file capabilities/web/http_client.yaml

# Hide all tools globally
cargo run --bin magictunnel-visibility -- -c config.yaml hide-all

# Show all tools globally
cargo run --bin magictunnel-visibility -- -c config.yaml show-all
```

#### **Per-Tool Visibility Control**
Individual tools can be marked as hidden in capability files:

```yaml
tools:
  - name: "internal_debug_tool"
    description: "Internal debugging tool"
    hidden: true  # Hidden from main tool list, available through discovery
    inputSchema:
      type: "object"
      properties:
        debug_level:
          type: "string"
          description: "Debug level (info, debug, trace)"
```

### 🚨 Error Handling and Fallbacks

#### **Graceful Degradation**
The system provides intelligent fallbacks:

1. **LLM Failure** → Falls back to semantic + rule-based search
2. **Semantic Search Failure** → Falls back to rule-based search
3. **No High-Confidence Match** → Provides suggestions with reasoning
4. **Parameter Extraction Failure** → Requests clarification with examples

#### **Error Response Format**
```json
{
  "success": false,
  "error": {
    "code": "TOOL_NOT_FOUND",
    "message": "No suitable tool found for request",
    "details": {
      "search_method": "hybrid",
      "searched_tools": 45,
      "highest_confidence": 0.45,
      "threshold": 0.7
    },
    "suggestions": [
      "Try being more specific about the action you want to perform",
      "Check if the tool exists: list available tools",
      "Lower confidence threshold for more permissive matching"
    ],
    "fallback_options": [
      {
        "tool_name": "execute_command",
        "confidence": 0.45,
        "reasoning": "Generic command execution tool"
      }
    ]
  }
}
```

### 🔧 Troubleshooting

#### **Common Issues**

##### **Semantic Search Returns 0 Results**
✅ **FIXED**: Service initialization now properly loads embeddings
- Verify embeddings exist in `./data/embeddings/`
- Check Ollama is running for `ollama:nomic-embed-text` model
- Confirm similarity threshold is appropriate (0.55 recommended)

##### **Missing Tool Scores**
✅ **FIXED**: All methods now evaluate all tools
- All enabled tools get evaluated by all enabled methods
- Enhanced logging shows which methods contributed
- No tools are missed in evaluation process

##### **High LLM Costs**
✅ **OPTIMIZED**: Multi-criteria selection limits LLM usage
- LLM evaluates maximum 30 tools (vs all 94 previously)
- Strategic sampling ensures good coverage with lower cost
- Caching reduces repeated LLM calls

##### **Low Confidence Scores**
```yaml
# Lower the threshold for testing
smart_discovery:
  default_confidence_threshold: 0.5
```

##### **LLM API Failures**
```bash
# Check API key configuration
echo $OPENAI_API_KEY

# Test API connectivity
curl -H "Authorization: Bearer $OPENAI_API_KEY" \
     https://api.openai.com/v1/models
```

##### **Slow Performance**
```yaml
# Enable caching and reduce batch sizes
smart_discovery:
  cache:
    enabled: true
  llm_tool_selection:
    batch_size: 5
```

#### **Debug Mode**
Enable detailed logging:

```bash
export RUST_LOG=magictunnel::discovery=debug
./target/release/magictunnel
```

#### **Health Checks**
```bash
# Check semantic search status
curl http://localhost:8080/health/semantic

# Get detailed statistics
curl http://localhost:8080/v1/discovery/stats

# Check external MCP status (if configured)
curl http://localhost:8080/mcp/external/status
```

### 🔗 API Reference

#### **Smart Discovery Endpoints**
- `POST /v1/mcp/call` - Execute smart tool discovery
- `GET /v1/discovery/stats` - Get discovery statistics
- `POST /v1/embeddings/sync` - Force embedding synchronization
- `GET /health/semantic` - Semantic search health check
- `POST /dashboard/api/mcp/execute` - Web dashboard MCP execution endpoint
- `GET /dashboard` - Web dashboard interface with MCP mode toggle

#### **Configuration Endpoints**
- `GET /v1/config/semantic` - Get semantic search configuration
- `PUT /v1/config/semantic` - Update semantic search configuration

#### **WebSocket Integration**
Smart Discovery works seamlessly with WebSocket connections:

```javascript
// WebSocket client example
const ws = new WebSocket('ws://localhost:8080/mcp/ws');

ws.send(JSON.stringify({
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "smart_tool_discovery",
    "arguments": {
      "request": "check server health"
    }
  }
}));
```

### 🚀 Migration and Best Practices

#### **From Traditional MCP to Smart Discovery**

1. **Enable Smart Discovery**:
   ```yaml
   smart_discovery:
     enabled: true
     tool_selection_mode: "rule_based"  # Start conservative
   ```

2. **Test with Existing Tools**:
   ```bash
   # Test basic functionality
   curl -X POST http://localhost:8080/v1/mcp/call \
     -H "Content-Type: application/json" \
     -d '{"name": "smart_tool_discovery", "arguments": {"request": "list files"}}'
   ```

3. **Gradually Enable Advanced Features**:
   ```yaml
   # Add semantic search
   semantic_search:
     enabled: true
     
   # Then hybrid mode
   tool_selection_mode: "hybrid"
   ```

#### **Production Deployment Best Practices**

1. **Use Hybrid Mode** for best accuracy and robustness
2. **Enable Caching** to reduce API costs and improve performance
3. **Set Appropriate Thresholds** (0.7 is a good starting point)
4. **Monitor API Usage** for LLM providers
5. **Configure Fallbacks** for graceful error handling

#### **Development and Testing**

1. **Start with Rule-based Mode** for faster iteration
2. **Use Lower Thresholds** (0.5) for more permissive matching  
3. **Enable Debug Logging** for understanding behavior
4. **Test Different Query Styles** to find optimal patterns
5. **Monitor Cache Hit Rates** for performance optimization

#### **Security Considerations**

1. **Protect API Keys** - Use environment variables, never commit to code
2. **Secure Embedding Files** - Restrict file permissions for embedding storage
3. **Validate User Input** - Sanitize search queries and parameters
4. **Monitor API Usage** - Track embedding generation costs and usage
5. **Rate Limiting** - Implement rate limits for discovery endpoints
6. **Access Control** - Secure discovery configuration endpoints

#### **⚡ Embedding Pre-Generation for Faster Startup**

For production deployments, pre-generate embeddings to eliminate startup delays. Multiple embedding models are supported:

##### **🔥 Ollama (Recommended for Local Development)**
```bash
# First time setup - install Ollama and pull embedding model
ollama pull nomic-embed-text

# Pre-generate embeddings with real semantic understanding
make pregenerate-embeddings-ollama    # Real embeddings (768 dimensions)

# Run server with real semantic search
make run-release-ollama               # RECOMMENDED FOR LOCAL DEVELOPMENT
```

##### **⚠️ Fallback Models (Hash-Based, Limited Functionality)**
```bash
# WARNING: These use deterministic hash fallbacks, NOT real semantic embeddings
# They produce consistent but meaningless vectors for development/testing only

make pregenerate-embeddings-local    # all-MiniLM-L6-v2 (hash fallback)
make pregenerate-embeddings-hq       # all-mpnet-base-v2 (hash fallback)

# Run server (limited semantic understanding)
make run-release-local               # Hash fallback - poor semantic matching
make run-release-hq                  # Hash fallback - poor semantic matching

# RECOMMENDATION: Use Ollama or OpenAI instead for real semantic search
```

##### **🌐 Cloud Models (API Key Required)**
```bash
# Pre-generate embeddings
make pregenerate-embeddings-openai OPENAI_API_KEY=your-key     # OpenAI (1536 dimensions)
make pregenerate-embeddings-external EMBEDDING_API_URL=http://your-server:8080  # Custom API

# Run server  
make run-release-semantic OPENAI_API_KEY=your-key             # RECOMMENDED FOR PRODUCTION
make run-release-external EMBEDDING_API_URL=http://your-server:8080  # Custom API
```

##### **🖥️ Local LLM Server**
```bash
# First time setup
ollama pull nomic-embed-text

# Pre-generate embeddings
make pregenerate-embeddings-ollama                            # Ollama server
EMBEDDING_API_URL=http://localhost:8080 make pregenerate-embeddings-external  # Custom server

# Run server
make run-release-ollama                                       # Ollama server  
EMBEDDING_API_URL=http://localhost:8080 make run-release-external  # Custom server
```

**CLI Usage Examples:**
```bash
# Ollama models (recommended for local - requires Ollama server running)
OLLAMA_BASE_URL=http://localhost:11434 MAGICTUNNEL_SEMANTIC_MODEL="ollama:nomic-embed-text" \
./target/release/magictunnel --config magictunnel-config.yaml --pregenerate-embeddings

# Local transformer models (WARNING: these use deterministic fallbacks, not real embeddings)
MAGICTUNNEL_SEMANTIC_MODEL="all-MiniLM-L6-v2" \
./target/release/magictunnel --config magictunnel-config.yaml --pregenerate-embeddings

MAGICTUNNEL_SEMANTIC_MODEL="all-mpnet-base-v2" \
./target/release/magictunnel --config magictunnel-config.yaml --pregenerate-embeddings

# OpenAI models (requires API key)
OPENAI_API_KEY=your-key MAGICTUNNEL_SEMANTIC_MODEL="openai:text-embedding-3-small" \
./target/release/magictunnel --config magictunnel-config.yaml --pregenerate-embeddings

OPENAI_API_KEY=your-key MAGICTUNNEL_SEMANTIC_MODEL="openai:text-embedding-3-large" \
./target/release/magictunnel --config magictunnel-config.yaml --pregenerate-embeddings

# Ollama models (requires Ollama server running)
OLLAMA_BASE_URL=http://localhost:11434 MAGICTUNNEL_SEMANTIC_MODEL="ollama:nomic-embed-text" \
./target/release/magictunnel --config magictunnel-config.yaml --pregenerate-embeddings

# Custom local model file
MAGICTUNNEL_SEMANTIC_MODEL="local:/path/to/your/model" \
./target/release/magictunnel --config magictunnel-config.yaml --pregenerate-embeddings

# External embedding API
EMBEDDING_API_URL=http://your-server:8080 MAGICTUNNEL_SEMANTIC_MODEL="external:api" \
./target/release/magictunnel --config magictunnel-config.yaml --pregenerate-embeddings
```

**Model Comparison:**

| Model | Dimensions | Speed | Quality | API Key | Status | Best For |
|-------|------------|-------|---------|---------|--------|----------|
| `ollama:nomic-embed-text` | 768 | ⚡⚡ | ⭐⭐⭐⭐ | ❌ | ✅ **Real embeddings** | **🏆 Local development (recommended)** |
| `openai:text-embedding-3-small` | 1536 | ⚡⚡⚡ | ⭐⭐⭐⭐⭐ | ✅ | ✅ **Real embeddings** | **🏆 Production (recommended)** |
| `openai:text-embedding-3-large` | 3072 | ⚡⚡ | ⭐⭐⭐⭐⭐ | ✅ | ✅ **Real embeddings** | Premium production |
| `external:api` | Variable | ⚡⚡ | ⭐⭐⭐⭐ | ❌ | ✅ **Real embeddings** | Custom embedding services |
| `all-MiniLM-L6-v2` | 384 | ⚡⚡⚡ | 🚫 | ❌ | ⚠️ **Hash fallback** | Development/testing only |
| `all-mpnet-base-v2` | 768 | ⚡⚡ | 🚫 | ❌ | ⚠️ **Hash fallback** | Development/testing only |

**Benefits:**
- ⚡ **Faster Startup**: Embeddings pre-computed, no runtime generation delays
- 🚀 **Production Ready**: Perfect for containerized deployments and CI/CD
- 📊 **Detailed Reports**: Shows created/updated/failed embedding counts
- 🔧 **Flexible Models**: Use Ollama (local), OpenAI (cloud), or custom APIs
- 💰 **Cost Control**: Ollama provides local embeddings with no API costs
- 🧠 **Real Semantic Understanding**: Ollama and OpenAI provide genuine semantic embeddings

**⚠️ About Fallback Models:**
The `all-MiniLM-L6-v2` and `all-mpnet-base-v2` options use **deterministic hash fallbacks**, not real semantic embeddings. They:
- Generate consistent vectors for the same input text
- **Don't understand semantic meaning** ("ping google" ≠ "network test")
- Are only suitable for development/testing, not production
- **Should be replaced with Ollama or OpenAI** for real semantic search

**What gets pre-generated:**
- Embeddings for all enabled tools (excludes smart_tool_discovery to prevent recursion)
- Saved to `./data/embeddings/` directory
- Automatic detection of tool changes and updates
- Comprehensive error reporting for failed operations

### 🎯 Benefits

1. **Clean User Experience**: Users see a clean interface without tool clutter
2. **Natural Language**: Express requests in plain English instead of learning tool names
3. **Intelligent Discovery**: Automatically finds the best tool for any request
4. **Parameter Mapping**: Automatically maps natural language to tool parameters
5. **Scalable**: Works with any number of tools (tested with 83+ tools)
6. **High Performance**: Fast local discovery with LLM-powered parameter extraction
7. **Flexible Management**: Easy CLI-based visibility control
8. **Developer Friendly**: Simple configuration and backward compatibility

### 🔧 Troubleshooting

**Common Issues:**

1. **"No suitable tool found"**
   - Try lowering confidence threshold: `"confidence_threshold": 0.5`
   - Be more specific: "read the config.yaml file" vs "read file"
   - Add context: Provide more details about your goal

2. **"Missing required parameter"**
   - Include specific details: "read the config.yaml file from /app/config/"
   - Use context field: `"context": "Application startup configuration"`

3. **"Ambiguous request"**
   - Be more specific: "search for error messages in log files" vs "search"
   - Use preferred tools: `"preferred_tools": ["grep_tool", "search_files"]`

### 📊 Current Status

- **Total Tools**: 83 across 15 capability files
- **Visible Tools**: 0 (complete Smart Tool Discovery mode)
- **Hidden Tools**: 83 (all available through discovery)
- **Management**: Full CLI control with real-time status reporting
- **LLM Integration**: OpenAI, Anthropic, and Ollama support for parameter mapping

## 🔧 Configuration & Setup

### **Version 0.2.48 External MCP Migration** ✅ **COMPLETE**

**🎉 MAJOR MILESTONE**: Complete migration from hybrid routing to unified External MCP system!

#### **Hybrid Routing Removal**
- **Legacy Code Elimination**: Completely removed all hybrid routing code and dependencies
- **Clean Architecture**: Unified External MCP system for all external MCP server integration
- **Test Suite Migration**: Updated all tests to reflect new architecture (457 tests passing)
- **Configuration Migration**: Migrated all configurations to External MCP format
- **Zero Compilation Errors**: Clean build with no legacy code remnants

#### **External MCP System**
- **Claude Desktop Compatible**: Exact same configuration format as Claude Desktop
- **Process Management**: Automatic spawning and lifecycle management of MCP servers
- **Container Support**: Built-in Docker/Podman integration for containerized MCP servers
- **Automatic Discovery**: Tools and capabilities discovered automatically from spawned processes
- **Conflict Resolution**: Built-in strategies (`local_first`, `remote_first`, `prefix`) for tool name conflicts

#### **Modern Architecture Benefits**
- **Simplified Configuration**: Familiar Claude Desktop format reduces learning curve
- **Reduced Complexity**: Single system instead of multiple routing mechanisms
- **Better Performance**: Optimized for modern MCP server patterns
- **Enhanced Reliability**: Robust process management and error handling
- **Future Proof**: Extensible architecture for additional MCP server types

### **Quick Configuration Setup**

```yaml
# config.yaml - Basic setup
server:
  host: "0.0.0.0"
  port: 3000
  websocket: true
  timeout: 30

registry:
  type: "file"
  paths:
    - "./capabilities"
  hot_reload: true

external_mcp:
  enabled: true
  config_file: "external-mcp-servers.yaml"
  capabilities_output_dir: "./capabilities/external-mcp"
  refresh_interval_minutes: 5
  containers:
    runtime: "docker"
    remote_prefix_format: "{server_name}"

logging:
  level: "info"
  format: "text"
```

### Environment Configuration

MagicTunnel supports multiple environment configurations for different deployment scenarios (development, staging, production). This provides better API key management and environment-specific settings.

#### Environment File Structure

| File | Purpose | Git Tracked |
|------|---------|-------------|
| `.env.example` | Template/documentation | ✅ Yes |
| `.env.development` | Development settings | ✅ Yes |
| `.env.production` | Production settings | ✅ Yes |
| `.env.staging` | Staging settings | ✅ Yes |
| `.env` | Your base config | ❌ No |
| `.env.local` | Your local overrides | ❌ No |

#### File Loading Order (Precedence)

The system loads environment files in this order, with later files overriding earlier ones:

1. `.env` - Base environment file (lowest precedence)
2. `.env.{environment}` - Environment-specific file (e.g., `.env.production`)
3. `.env.local` - Local overrides (highest precedence, git-ignored)
4. System environment variables
5. CLI arguments

#### Environment Detection

The system determines the environment using:
- `MAGICTUNNEL_ENV` environment variable (highest precedence)
- `ENV` environment variable
- `NODE_ENV` environment variable
- Defaults to `"development"`

#### Environment-Specific Examples

**Development (.env.development)**:
```bash
# Optimized for development
MCP_LOG_LEVEL=debug
MCP_LOG_FORMAT=text
MCP_HOST=127.0.0.1
MCP_PORT=3001
MCP_HOT_RELOAD=true
DEV_MODE=true
```

**Production (.env.production)**:
```bash
# Optimized for production
MCP_LOG_LEVEL=info
MCP_LOG_FORMAT=json
MCP_HOST=0.0.0.0
MCP_PORT=3000
MCP_HOT_RELOAD=false
MCP_TLS_MODE=application
```

**Usage**:
```bash
# Development mode
MAGICTUNNEL_ENV=development cargo run --release

# Production mode  
MAGICTUNNEL_ENV=production cargo run --release

# Copy and customize for your needs
cp .env.development .env.local
# Edit .env.local with your API keys
```

## 🛠️ Technical Implementation

### MCP Format Compliance
All capabilities follow official MCP tool definitions with custom extensions:

```yaml
tools:
  - name: "execute_command"
    description: "Execute bash commands and system operations"
    inputSchema:
      type: "object"
      properties:
        command: { type: "string", description: "The bash command to execute" }
        timeout: { type: "number", default: 30 }
      required: ["command"]
    annotations:
      title: "Execute Command"
      destructiveHint: true
    # Routing configuration - this is all we need!
    routing:
      type: "subprocess"
      command: "bash"
      args: ["-c", "{command}"]
      timeout: "{timeout}"
```

### Agent Routing Configuration ✅ **COMPLETE**

MagicTunnel supports nine powerful agent types with advanced parameter substitution:

#### 1. **Subprocess Agent** - Local Command Execution
```yaml
routing:
  type: "subprocess"
  config:
    command: "find"
    args: ["{{directory}}", "-name", "{{pattern}}"]
    timeout: 60
    env:
      SEARCH_PATH: "{{directory}}"
      DEBUG: "{{debug_mode ? '1' : '0'}}"
```

#### 2. **HTTP Agent** - REST API Integration
```yaml
routing:
  type: "http"
  config:
    method: "POST"
    url: "{{api_endpoint}}/search"
    headers:
      Authorization: "Bearer {{env.API_TOKEN}}"
      Content-Type: "application/json"
    body: |
      {
        "query": "{{query}}",
        "filters": {{filters}},
        "limit": {{limit}}
      }
    timeout: 30
    retry_attempts: 3
```

#### 3. **LLM Agent** - AI Model Integration
```yaml
routing:
  type: "llm"
  config:
    provider: "openai"
    model: "{{model_name}}"
    api_key: "{{env.OPENAI_API_KEY}}"
    system_prompt: "You are a {{role}} assistant. {{context}}"
    user_prompt: "{{user_input}}"
    timeout: 120
    max_tokens: 2000
```

#### 4. **WebSocket Agent** - Real-time Communication
```yaml
routing:
  type: "websocket"
  config:
    url: "{{websocket_url}}"
    headers:
      Authorization: "Bearer {{auth_token}}"
    message: |
      {
        "action": "{{action}}",
        "data": {{payload}},
        "session_id": "{{session_id}}"
      }
    timeout: 60
```

### Advanced Parameter Substitution Features

- **Basic Substitution**: `{{parameter_name}}`
- **Conditional Logic**: `{{case_sensitive ? '' : '-i'}}`
- **Array Iteration**: `{{#each items}}--include={{this}}{{/each}}`
- **Environment Variables**: `{{env.API_KEY}}`
- **Complex JSON**: Dynamic object construction with nested substitution
- **Default Values**: `{{timeout || 30}}`
- **String Operations**: `{{name | upper}}` (coming soon)

## 🎯 Use Cases

### 1. **Rapid AI Agent Development**
- Connect Claude/GPT-4 to bash commands, APIs, and databases
- No need to implement MCP servers for each capability
- Intelligent capability selection based on task requirements

### 2. **Enterprise Integration**
- Provide unified access to internal tools and APIs
- Multi-tenant deployment for different teams/projects
- Authentication and access control for sensitive capabilities

### 3. **Ecosystem Bridge**
- Connect existing MCP servers with custom agents
- Aggregate capabilities from multiple sources
- Provide intelligent routing and optimization

### 4. **Research & Development**
- Validate intelligent agent orchestration concepts
- Experiment with capability composition and dependency resolution
- Prototype new agent interaction patterns

## 🚀 Getting Started

### Prerequisites
- Rust 1.70+
- MCP client (Claude Desktop, custom implementation)
- Optional: Docker for containerized deployment

### Development Architecture
**Port Configuration**: Frontend (5173) → MagicTunnel API (3001) → Supervisor (8081)
- **Web Dashboard**: Svelte frontend with real-time monitoring and tool management
- **MCP Mode Toggle**: Switch between HTTP API and MCP protocol execution seamlessly in the dashboard
- **Smart Discovery Integration**: Visual tool discovery interface with dual-mode (HTTP/MCP) support
- **Custom Restart System**: Supervisor integration for flexible restart workflows with Makefile command support
- **Service Management**: Process lifecycle management for external MCP services
- See `frontend.md` and `frontend_todo.md` for detailed architecture documentation

## 🛠️ CLI Tools

MagicTunnel includes several powerful CLI tools for different aspects of system management:

### 1. **Main Server** (`magictunnel`)
```bash
# Start the main MCP server
cargo run --bin magictunnel -- --config config.yaml

# Start with stdio mode for Claude Desktop
cargo run --bin magictunnel -- --stdio --config config.yaml
```

### 2. **Visibility Management** (`magictunnel-visibility`)
```bash
# Check tool visibility status
cargo run --bin magictunnel-visibility -- -c config.yaml status

# Manage individual tools
cargo run --bin magictunnel-visibility -- -c config.yaml hide-tool tool_name
cargo run --bin magictunnel-visibility -- -c config.yaml show-tool tool_name

# Manage entire capability files
cargo run --bin magictunnel-visibility -- -c config.yaml hide-file capabilities/web/http_client.yaml
cargo run --bin magictunnel-visibility -- -c config.yaml show-file capabilities/web/http_client.yaml

# Global visibility control
cargo run --bin magictunnel-visibility -- -c config.yaml hide-all
cargo run --bin magictunnel-visibility -- -c config.yaml show-all
```

### 3. **Capability Generators**
```bash
# Unified generator CLI
cargo run --bin mcp-generator -- init --output mcp-generator.toml
cargo run --bin mcp-generator -- openapi --spec "https://api.example.com/openapi.json"

# Individual generators
cargo run --bin openapi_generator -- --spec "https://petstore.swagger.io/v2/swagger.json"
cargo run --bin graphql_generator -- --schema "schema.graphql"
```

### 4. **Management CLI** (`magictunnel-cli`) ✅ **NEW**

Complete management interface for MCP resources, prompts, and system administration:

#### **MCP Resources Management**
```bash
# List all available MCP resources
magictunnel-cli resources list --server http://localhost:3001

# Read resource content by URI
magictunnel-cli resources read --uri "file://path/to/resource"

# Export resource content to file (handles text and binary)
magictunnel-cli resources export --uri "file://path/to/resource" --output exported_file.txt
```

#### **MCP Prompts Management**
```bash
# List all prompt templates with arguments
magictunnel-cli prompts list

# Execute a prompt template with arguments
magictunnel-cli prompts execute "summarize_code" --args '{"language": "rust", "file": "src/main.rs"}'

# Export prompt execution results to JSON
magictunnel-cli prompts export "summarize_code" --args '{"language": "rust"}' --output result.json
```

#### **Tools Management**
```bash
# List all available tools
magictunnel-cli tools list

# Execute a tool with arguments
magictunnel-cli tools execute "smart_tool_discovery" --args '{"request": "ping google.com"}'

# Get detailed tool information and schema
magictunnel-cli tools info "smart_tool_discovery"
```

#### **Services Management**
```bash
# List all MCP services with status indicators
magictunnel-cli services list

# Manage service lifecycle
magictunnel-cli services restart "service_name"
magictunnel-cli services start "service_name"
magictunnel-cli services stop "service_name"
```

#### **Server Management**
```bash
# Check server status and configuration
magictunnel-cli server status

# Restart the MagicTunnel server
magictunnel-cli server restart

# Check server health endpoints
magictunnel-cli server health
```

**Key Features:**
- **Unified Interface**: Single CLI for all management tasks
- **Rich Output**: Emojis and color-coded status indicators
- **JSON Support**: Complex arguments via JSON for tools and prompts
- **Error Handling**: Comprehensive error reporting with suggestions
- **Flexible Configuration**: Configurable server URL (defaults to `http://localhost:3001`)

### Quick Start

#### Option 1: Basic Setup (Local Tools Only)
```bash
# Clone the repository
git clone https://github.com/gouravd/magictunnel
cd magictunnel

# Install dependencies
cargo build --release

# Run with default configuration
./target/release/magictunnel --config config.yaml

# Connect your MCP client to ws://localhost:3000
```

#### Option 1b: Smart Discovery with Pre-Generated Embeddings (Recommended)
```bash
# Clone and build
git clone https://github.com/gouravd/magictunnel
cd magictunnel
make build-release-semantic

# Pre-generate embeddings for faster startup (requires OpenAI API key)
make pregenerate-embeddings-openai OPENAI_API_KEY=your-openai-key

# Run with smart discovery and fast startup
make run-release-semantic OPENAI_API_KEY=your-openai-key

# Connect your MCP client to ws://localhost:3000
# Use single tool: smart_tool_discovery with natural language requests
```

#### Option 2: Quick Start with Popular MCP Servers
```bash
# 1. Set up MagicTunnel
git clone https://github.com/gouravd/magictunnel
cd magictunnel
cargo build --release

# 2. Install popular MCP servers (in separate terminal)
npm install -g @modelcontextprotocol/server-filesystem
npm install -g @modelcontextprotocol/server-git
npm install -g @modelcontextprotocol/server-sqlite

# 3. Start MCP servers (each in separate terminal)
npx @modelcontextprotocol/server-filesystem /home/user/projects  # Terminal 1
npx @modelcontextprotocol/server-git --repository /home/user/repo  # Terminal 2
npx @modelcontextprotocol/server-sqlite /home/user/data.db  # Terminal 3

# 4. Create config with MCP servers
cat > config-with-servers.yaml << EOF
mcp_proxy:
  enabled: true
  auto_connect: true
  auto_discover_tools: true

mcp_servers:
  - name: "filesystem"
    endpoint: "ws://localhost:3001"
    tool_prefix: "fs"
    enabled: true

  - name: "git"
    endpoint: "ws://localhost:3002"
    tool_prefix: "git"
    enabled: true

  - name: "sqlite"
    endpoint: "ws://localhost:3003"
    tool_prefix: "db"
    enabled: true

server:
  host: "127.0.0.1"
  port: 3000
  grpc_port: 4000

registry:
  paths:
    - "./capabilities/**/*.yaml"
EOF

# 5. Start MagicTunnel with server connections
./target/release/magictunnel --config config-with-servers.yaml

# 6. Connect your MCP client to ws://localhost:3000
# You now have access to filesystem, git, and database tools!
```

#### Verify Setup
```bash
# Check available tools (includes both local and proxied tools)
curl http://localhost:3000/tools

# Check MCP server connections
curl http://localhost:3000/mcp/proxy/status
```

## 🔗 MagicTunnel Quick Start Guide

### Overview

MagicTunnel allows you to connect to external MCP servers and aggregate their tools into a unified interface. This enables building powerful distributed AI systems that can access tools from multiple specialized MCP servers.

### Phase 3.1 Features ✅ COMPLETE

- **MCP Client**: Connect to external MCP servers via WebSocket
- **Server Discovery**: Register and manage multiple MCP server connections
- **Tool Mapping**: Intelligent name mapping and conflict resolution
- **Connection Management**: Robust connection handling with health monitoring
- **Tool Aggregation**: Unified interface combining local and remote tools

### Quick Setup with Popular MCP Servers

#### 1. Install Common MCP Servers
```bash
# Install popular MCP servers
npm install -g @modelcontextprotocol/server-filesystem
npm install -g @modelcontextprotocol/server-git
npm install -g @modelcontextprotocol/server-sqlite
npm install -g @modelcontextprotocol/server-brave-search
npm install -g @modelcontextprotocol/server-github

# Set up environment variables (optional)
export BRAVE_API_KEY="your-brave-api-key"
export GITHUB_PERSONAL_ACCESS_TOKEN="your-github-token"
```

#### 2. Start MCP Servers (in separate terminals)
```bash
# Terminal 1: Filesystem server
npx @modelcontextprotocol/server-filesystem /home/user/projects

# Terminal 2: Git server
npx @modelcontextprotocol/server-git --repository /home/user/my-repo

# Terminal 3: SQLite server
npx @modelcontextprotocol/server-sqlite /home/user/data.db

# Terminal 4: Search server (requires API key)
npx @modelcontextprotocol/server-brave-search

# Terminal 5: GitHub server (requires token)
npx @modelcontextprotocol/server-github
```

### MagicTunnel Configuration

#### 1. Basic MagicTunnel Setup

Create a configuration file with MagicTunnel settings:

```yaml
# config.yaml
mcp_proxy:
  enabled: true
  auto_connect: true
  auto_discover_tools: true

mcp_servers:
  - name: "database-server"
    endpoint: "ws://localhost:3001"
    tool_prefix: "db"
    enabled: true
```

#### 2. Start with MagicTunnel

```bash
# Start with MagicTunnel enabled
cargo run -- --config config.yaml

# Or use environment variables
MCP_PROXY_ENABLED=true cargo run
```

#### 3. Multi-Service Setup

```yaml
mcp_servers:
  - name: "weather-api"
    endpoint: "ws://weather-service:3001"
    tool_prefix: "weather"
    enabled: true

  - name: "email-service"
    endpoint: "ws://email-service:3002"
    tool_prefix: "email"
    enabled: true

  - name: "file-system"
    endpoint: "ws://fs-service:3003"
    tool_prefix: "fs"
    enabled: true
```

#### 4. Custom Tool Mappings

```yaml
tool_mappings:
  - local_name: "query_database"
    server_name: "postgres-server"
    remote_name: "execute_sql"
    description: "Execute SQL queries"

  - local_name: "get_forecast"
    server_name: "weather-api"
    remote_name: "forecast"
    description: "Get weather forecast"
```

### Common MCP Server Examples

Connect to popular MCP servers in the ecosystem:

#### 1. **Filesystem MCP Server** - File Operations
```yaml
mcp_servers:
  - name: "filesystem"
    endpoint: "ws://localhost:3001"
    tool_prefix: "fs"
    enabled: true
    description: "File system operations (read, write, list directories)"
    # Install: npm install @modelcontextprotocol/server-filesystem
    # Run: npx @modelcontextprotocol/server-filesystem /path/to/allowed/directory
```

#### 2. **Git MCP Server** - Version Control
```yaml
mcp_servers:
  - name: "git"
    endpoint: "ws://localhost:3002"
    tool_prefix: "git"
    enabled: true
    description: "Git operations (commit, branch, status, diff)"
    # Install: npm install @modelcontextprotocol/server-git
    # Run: npx @modelcontextprotocol/server-git --repository /path/to/repo
```

#### 3. **SQLite MCP Server** - Database Operations
```yaml
mcp_servers:
  - name: "sqlite"
    endpoint: "ws://localhost:3003"
    tool_prefix: "db"
    enabled: true
    description: "SQLite database operations"
    # Install: npm install @modelcontextprotocol/server-sqlite
    # Run: npx @modelcontextprotocol/server-sqlite /path/to/database.db
```

#### 4. **Brave Search MCP Server** - Web Search
```yaml
mcp_servers:
  - name: "brave-search"
    endpoint: "ws://localhost:3004"
    tool_prefix: "search"
    enabled: true
    description: "Web search using Brave Search API"
    # Install: npm install @modelcontextprotocol/server-brave-search
    # Run: npx @modelcontextprotocol/server-brave-search
    # Requires: BRAVE_API_KEY environment variable
```

#### 5. **GitHub MCP Server** - GitHub Integration
```yaml
mcp_servers:
  - name: "github"
    endpoint: "ws://localhost:3005"
    tool_prefix: "gh"
    enabled: true
    description: "GitHub operations (repos, issues, PRs)"
    # Install: npm install @modelcontextprotocol/server-github
    # Run: npx @modelcontextprotocol/server-github
    # Requires: GITHUB_PERSONAL_ACCESS_TOKEN environment variable
```

#### 6. **Slack MCP Server** - Team Communication
```yaml
mcp_servers:
  - name: "slack"
    endpoint: "ws://localhost:3006"
    tool_prefix: "slack"
    enabled: true
    description: "Slack messaging and channel operations"
    # Install: npm install @modelcontextprotocol/server-slack
    # Run: npx @modelcontextprotocol/server-slack
    # Requires: SLACK_BOT_TOKEN environment variable
```

### External MCP Servers

Connect to external MCP servers using Claude Desktop's configuration format:

#### 1. **Make.com Cloud MCP Server** - Automation Platform
```yaml
mcp_servers:
  - name: "make-cloud"
    endpoint: "https://eu1.make.com/mcp/api/v1/u/YOUR_MCP_TOKEN/sse"
    tool_prefix: "make"
    enabled: true
    description: "Make.com automation and workflow tools"
    # Get token from: https://www.make.com/en/integrations/mcp
    # Supports: Scenario execution, webhook management, data processing
```

#### 2. **Firecrawl Cloud MCP Server** - Web Scraping
```yaml
mcp_servers:
  - name: "firecrawl-cloud"
    endpoint: "https://api.firecrawl.dev/mcp/sse"
    tool_prefix: "crawl"
    enabled: true
    description: "Cloud-based web scraping and data extraction"
    # Get API key from: https://firecrawl.dev
    # Supports: Web scraping, PDF extraction, structured data
```

#### 3. **IBM watsonx MCP Server** - AI Platform
```yaml
mcp_servers:
  - name: "watsonx"
    endpoint: "https://api.watsonx.ibm.com/mcp/v1/sse"
    tool_prefix: "wx"
    enabled: true
    description: "IBM watsonx AI platform tools"
    # Requires: IBM Cloud API key and watsonx instance
    # Supports: Model inference, data analysis, governance
```

#### 4. **Alibaba Cloud RDS MCP Server** - Database Management
```yaml
mcp_servers:
  - name: "alibaba-rds"
    endpoint: "https://rds.ap-southeast-1.aliyuncs.com/mcp/sse"
    tool_prefix: "rds"
    enabled: true
    description: "Alibaba Cloud RDS database management"
    # Requires: Alibaba Cloud access key and secret
    # Supports: Database creation, backup, monitoring
```

#### 5. **Custom Enterprise MCP Server** - Your Organization
```yaml
mcp_servers:
  - name: "enterprise-tools"
    endpoint: "https://mcp.yourcompany.com/api/v1/sse"
    tool_prefix: "corp"
    enabled: true
    description: "Internal enterprise tools and APIs"
    client_config:
      connect_timeout_secs: 60
      request_timeout_secs: 120
    # Custom headers for authentication
    # Supports: Internal APIs, databases, workflows
```

#### 6. **Hybrid Setup: Local + Remote Servers**
```yaml
# config.yaml - Production-ready hybrid setup
mcp_proxy:
  enabled: true
  auto_connect: true
  auto_discover_tools: true
  health_check_interval_secs: 300
  registry:
    max_servers: 50
    default_client_config:
      connect_timeout_secs: 30
      request_timeout_secs: 60

mcp_servers:
  # Local development tools
  - name: "filesystem"
    endpoint: "ws://localhost:3001"
    tool_prefix: "fs"
    enabled: true
    description: "Local file operations"

  - name: "git"
    endpoint: "ws://localhost:3002"
    tool_prefix: "git"
    enabled: true
    description: "Local git operations"

  # Cloud-hosted services
  - name: "make-cloud"
    endpoint: "https://eu1.make.com/mcp/api/v1/u/YOUR_MCP_TOKEN/sse"
    tool_prefix: "make"
    enabled: true
    description: "Make.com automation platform"
    client_config:
      connect_timeout_secs: 60
      request_timeout_secs: 120

  - name: "firecrawl-cloud"
    endpoint: "https://api.firecrawl.dev/mcp/sse"
    tool_prefix: "crawl"
    enabled: true
    description: "Cloud web scraping service"

  - name: "watsonx"
    endpoint: "https://api.watsonx.ibm.com/mcp/v1/sse"
    tool_prefix: "wx"
    enabled: true
    description: "IBM watsonx AI platform"
    client_config:
      connect_timeout_secs: 90
      request_timeout_secs: 180

  # Enterprise internal server
  - name: "enterprise-tools"
    endpoint: "https://mcp.yourcompany.com/api/v1/sse"
    tool_prefix: "corp"
    enabled: true
    description: "Internal enterprise APIs"
    client_config:
      connect_timeout_secs: 45
      request_timeout_secs: 90

# External MCP configuration
external_mcp:
  enabled: true
  config_file: "external-mcp-servers.yaml"
  conflict_resolution: "prefix"
  discovery:
    enabled: true
    refresh_interval_minutes: 5
  capabilities:
    output_dir: "./capabilities/external-mcp"
    format: "yaml"
    include_local_tools: true
```

### Advanced MCP Proxy Configuration

#### Connection Management

```yaml
mcp_proxy:
  registry:
    max_servers: 50
    default_client_config:
      connect_timeout_secs: 30
      request_timeout_secs: 60
      max_reconnect_attempts: 5
      auto_reconnect: true
```

#### Tool Mapping Options

```yaml
mcp_proxy:
  mapping:
    default_prefix_format: "{server}_{tool}"
    auto_generate_mappings: true
    allow_conflicts: false
    max_tool_name_length: 64
```

### Using Aggregated Tools

Tools from external servers are available with prefixed names:

```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "method": "tools/list"
}
```

Response includes both local and remote tools:
```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "result": {
    "tools": [
      {
        "name": "local_tool",
        "description": "Local tool"
      },
      {
        "name": "db_query",
        "description": "Database query tool (via proxy from 'database-server')"
      }
    ]
  }
}
```

### Programmatic MCP Proxy Usage

```rust
use mcp_proxy::mcp::proxy::McpProxyManager;
use mcp_proxy::mcp::discovery::ServerConfig;

// Create proxy manager
let proxy_manager = McpProxyManager::new();

// Register a server
let server_config = ServerConfig {
    name: "my-server".to_string(),
    endpoint: "ws://localhost:3001".to_string(),
    tool_prefix: Some("srv".to_string()),
    enabled: true,
    // ... other config
};

proxy_manager.register_server(server_config).await?;
proxy_manager.connect_server("my-server").await?;

// List aggregated tools
let tools = proxy_manager.list_available_tools().await?;

// Call a tool
let tool_call = ToolCall {
    name: "srv_some_tool".to_string(),
    arguments: json!({"param": "value"}),
};
let result = proxy_manager.call_tool(&tool_call).await?;
```

### Monitoring MCP Proxy Status

Check proxy status:
```bash
curl http://localhost:3000/mcp/proxy/status
```

Response:
```json
{
  "total_servers": 3,
  "connected_servers": 2,
  "total_tool_mappings": 15,
  "servers": [
    {
      "name": "database-server",
      "endpoint": "ws://localhost:3001",
      "status": "Connected",
      "enabled": true
    }
  ]
}
```

### MCP Proxy Troubleshooting

#### Connection Issues

1. **Check server endpoints**: Ensure external MCP servers are running and accessible
2. **Verify WebSocket connectivity**: Test WebSocket connections manually
3. **Check firewall settings**: Ensure ports are open for WebSocket connections
4. **Review logs**: Check proxy logs for connection errors

#### Tool Mapping Issues

1. **Name conflicts**: Use `allow_conflicts: true` or custom mappings
2. **Missing tools**: Verify server connection and tool discovery
3. **Prefix issues**: Check `tool_prefix` configuration

#### Performance Issues

1. **Timeout settings**: Adjust `request_timeout_secs` for slow operations
2. **Connection limits**: Check `max_servers` setting
3. **Health check frequency**: Adjust `health_check_interval_secs`

### MCP Proxy Best Practices

1. **Use descriptive prefixes**: Choose clear, short prefixes for tool organization
2. **Monitor connections**: Set up monitoring for server health and connectivity
3. **Handle failures gracefully**: Configure appropriate timeouts and retry settings
4. **Organize by domain**: Group related servers with consistent naming
5. **Test connectivity**: Verify all external servers before production deployment

### Configuration

MagicTunnel includes comprehensive configuration validation to ensure secure and reliable operation:

```yaml
# config.yaml
server:
  host: "0.0.0.0"              # Validated: non-empty host
  port: 3000                   # Validated: 1-65535 range
  websocket: true              # Enable WebSocket support
  timeout: 30                  # Validated: 1-3600 seconds

registry:
  type: "file"                 # Validated: supported types only
  paths:
    - "./capabilities"         # Validated: secure paths only
    - "./team-a/tools"        # No path traversal allowed
    - "./integrations/*.yaml"  # Glob patterns supported
    - "./custom/endpoints"     # Multiple directories
  hot_reload: true            # Enable file watching
  validation:
    strict: true              # Strict validation mode
    allow_unknown_fields: false

authentication:                # Optional but validated when present
  type: "api_key"             # Validated: supported auth types
  api_keys:
    - "your-secret-key-here"  # Validated: minimum 16 characters

logging:                      # Optional logging configuration
  level: "info"              # Validated: debug|info|notice|warning|error|critical|alert|emergency
  format: "json"             # Validated: json|text
  file: "/var/log/magictunnel.log"  # Optional file output
```

#### Environment Variable Support ✅ **NEW**

Override any configuration setting using environment variables:

```bash
# Server settings
export MCP_HOST="0.0.0.0"
export MCP_PORT="8080"
export MCP_WEBSOCKET="true"
export MCP_TIMEOUT="60"

# Registry settings
export MCP_REGISTRY_TYPE="file"
export MCP_REGISTRY_PATHS="./capabilities,./tools"
export MCP_HOT_RELOAD="true"

# Logging settings
export MCP_LOG_LEVEL="debug"
export MCP_LOG_FORMAT="json"
```

## 🧪 Testing & Validation

MagicTunnel includes comprehensive test coverage to ensure reliability and protocol compliance:

### Test Suites (457+ tests total)

- **📊 Data Structures Tests** (26 tests) - Comprehensive validation of all MCP types and structures
- **🔧 Integration Tests** (5 tests) - Configuration and CLI validation
- **⚙️ Configuration Validation Tests** (7 tests) - ✅ **NEW** - Comprehensive configuration validation
  - Server configuration validation (host, port, timeout)
  - Registry configuration validation (paths, security)
  - Authentication configuration validation (API keys, OAuth)
  - Logging configuration validation (levels, formats)
  - Environment variable override testing
  - Cross-dependency validation (port conflicts)
  - File structure and security validation
- **📡 MCP Server Tests** (14 tests) - JSON-RPC compliance and message format validation
- **🚀 gRPC Integration Tests** (6 tests) - Complete gRPC streaming protocol validation
- **🤖 Agent Router Tests** (24 tests) - Complete agent routing system validation
  - Subprocess agent execution and parameter substitution
  - HTTP agent with retry logic and authentication
  - gRPC agent with service calls and protobuf handling ✅ **NEW**
  - SSE agent with event stream handling and filtering ✅ **NEW**
  - GraphQL agent with query execution and variable substitution ✅ **NEW**
  - LLM agent integration with multiple providers
  - WebSocket agent real-time communication
  - Database agent with SQL execution and connection management ✅ **NEW**
  - Advanced parameter substitution with conditionals
  - Error handling and timeout scenarios
- **🔧 GraphQL Capability Generator Tests** (45 tests) - Complete GraphQL schema processing ✅ **NEW**
  - SDL schema parsing with 100% GraphQL specification compliance
  - Introspection JSON parsing with multiple format support
  - Operation extraction (queries, mutations, subscriptions)
  - Type system support (scalars, objects, enums, interfaces, unions, input objects)
  - Schema extensions and directive processing
  - Schema validation and safety analysis
  - Authentication integration and real-world schema testing
- **🔍 MCP Core Features Tests** (39 tests) - ✅ **NEW** - Complete MCP specification compliance
  - **MCP Logging Tests** (13 tests) - RFC 5424 syslog compliance and rate limiting
  - **MCP Notifications Tests** (17 tests) - Event-driven notifications and subscriptions
  - **MCP Integration Tests** (9 tests) - End-to-end server functionality

### Capability Testing Files ✅ **NEW**

Comprehensive testing capabilities demonstrating all agent routing features:

- **`capabilities/testing/agent_routing_showcase.yaml`** - All nine agent types with advanced examples
- **`capabilities/grpc/grpc_services.yaml`** - gRPC service integration examples ✅ **NEW**
- **`capabilities/sse/sse_streams.yaml`** - SSE stream subscription examples ✅ **NEW**
- **`capabilities/graphql/graphql_services.yaml`** - GraphQL query and mutation examples ✅ **NEW**
- **`capabilities/testing/error_handling_showcase.yaml`** - Error scenarios and edge cases
- **`capabilities/testing/performance_showcase.yaml`** - Performance benchmarking tools
- **`capabilities/testing/real_world_integrations.yaml`** - Production-ready integration examples

### Running Tests

```bash
# Run all tests (457+ tests total)
cargo test

# Run specific test suites
cargo test --test data_structures_test      # 26 tests
cargo test --test integration_test          # 5 tests
cargo test --test mcp_server_test          # 14 tests
cargo test --test grpc_integration_test    # 6 tests
cargo test --test agent_router_test        # 14 tests
cargo test --test sse_agent_test           # 5 tests
cargo test --test graphql_agent_tests      # 8 tests

# Run capability generator tests
cargo test registry::graphql_generator::tests --lib  # 45 GraphQL generator tests
cargo test registry::openapi_generator::tests --lib  # 13 OpenAPI generator tests

# Run MCP core features tests (39 tests total)
cargo test mcp::logging_tests --lib         # 13 logging tests
cargo test mcp::notifications_tests --lib   # 17 notification tests
cargo test mcp::integration_tests --lib     # 9 integration tests
cargo test mcp --lib                        # All 39 MCP tests

# Test capability file parsing
cargo test yaml_parsing

# Performance validation
cargo test --release
```

**All tests passing** ✅ - Complete protocol validation with performance benchmarks

See [Testing Guide](DEVELOPMENT.md#testing-guide) and [Capability Testing Guide](capabilities/testing/README.md) for detailed information.

## 🏗️ Detailed Architecture

### Architectural Decisions

#### MCP Logging System Architecture

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

#### MCP Notifications System Architecture

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

### Agent Routing System ✅ **COMPLETE**

#### Agent Router Architecture

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

#### Advanced Parameter Substitution System

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

### OpenAPI Capability Generator Architecture ✅ **COMPLETE**

**Design Goal**: Comprehensive OpenAPI 3.0 specification parsing and automatic MCP tool generation for REST API integration.

#### OpenAPI Generator Core Architecture

The OpenAPI capability generator provides complete OpenAPI 3.0 specification processing:

```rust
/// OpenAPI capability generator with comprehensive configuration
pub struct OpenAPICapabilityGenerator {
    base_url: String,
    auth_config: Option<AuthConfig>,
    prefix: Option<String>,
    operation_filter: Vec<String>,
    path_filter: Vec<String>,
    method_filter: Vec<String>,
    naming_convention: NamingConvention,
    include_deprecated: bool,
}

/// Naming conventions for generated tools
#[derive(Debug, Clone)]
pub enum NamingConvention {
    OperationId,        // Use OpenAPI operationId
    MethodPath,         // Combine HTTP method + path
    Custom(String),     // Custom format string
}
```

**Key Features**:
- **OpenAPI 3.0 Parsing**: Complete JSON/YAML specification parsing with auto-detection
- **HTTP Method Support**: All standard HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, TRACE)
- **Parameter Mapping**: Path, query, header, and cookie parameters with JSON Schema conversion
- **Authentication Integration**: API Key, Bearer Token, Basic Auth, and OAuth 2.0 support
- **Advanced Filtering**: Filter operations by methods, operation IDs, tags, or path patterns
- **Flexible Naming**: Multiple naming conventions with prefix support for organization

### Project Structure

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
│   │   ├── client.rs        # MCP client for external servers ✅ **NEW PHASE 3.1**
│   │   ├── discovery.rs     # Server discovery and registry ✅ **NEW PHASE 3.1**
│   │   ├── mapping.rs       # Tool mapping system ✅ **NEW PHASE 3.1**
│   │   ├── proxy.rs         # Proxy manager coordination ✅ **NEW PHASE 3.1**
│   │   ├── types.rs         # MCP protocol types and data structures
│   │   ├── logging.rs       # RFC 5424 syslog-compliant logging system ✅ NEW
│   │   ├── notifications.rs # Event-driven notification system ✅ NEW
│   │   ├── resources.rs     # Resource management system ✅ NEW
│   │   ├── prompts.rs       # Prompt template management ✅ NEW
│   │   ├── logging_tests.rs # Comprehensive logging tests (13 tests) ✅ NEW
│   │   ├── notifications_tests.rs # Notification system tests (17 tests) ✅ NEW
│   │   └── integration_tests.rs   # End-to-end integration tests (9 tests) ✅ NEW
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

### Development Commands

```bash
# Build the project
make build

# Run with default configuration
make run

# Run in development mode (debug logging)
make dev

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Kill all magictunnel processes
pkill -f magictunnel

# Check linting (if available)
cargo clippy

# Format code
cargo fmt
```

## 📊 Project Status

- **📋 Design Phase**: Complete (5 comprehensive design documents)
- **🏗️ Implementation**: ✅ **Phase 2.4 Complete** (Authentication System)
  - ✅ Phase 1.1: Foundation & Streaming Protocols
  - ✅ Phase 1.2: Enhanced data structures with comprehensive validation
  - ✅ Phase 2.3: MCP Core Features System
  - ✅ Phase 2.4: Authentication & Security System
  - ✅ Phase 1.3: Enterprise-scale capability registry with sub-millisecond lookups
  - ✅ Phase 1.4: Complete MCP Server with gRPC streaming
  - ✅ **Phase 2.0: MAJOR MILESTONE** - Complete Agent Routing System
    - ✅ **Multi-Agent Support**: Subprocess, HTTP, LLM, and WebSocket agents
    - ✅ **Advanced Parameter Substitution**: Handlebars-style templating with conditionals
    - ✅ **Comprehensive Error Handling**: Timeout management and retry logic
    - ✅ **Production Integration**: Router integrated into main MCP server
    - ✅ **Testing Capabilities**: Four comprehensive capability testing files
  - ✅ **Phase 2.3: MCP CORE FEATURES** - Complete MCP Specification Compliance
    - ✅ **MCP Resource Management**: Read-only resource system with URI validation
    - ✅ **MCP Prompt Templates**: Template management with argument substitution
    - ✅ **MCP Logging System**: RFC 5424 syslog-compliant logging with 8 severity levels
    - ✅ **MCP Notifications**: Event-driven notification system with resource subscriptions
    - ✅ **HTTP Endpoints**: Dynamic log level control via `/mcp/logging/setLevel`
    - ✅ **WebSocket Integration**: Full JSON-RPC 2.0 message handling
    - ✅ **Capability Declaration**: MCP server capability advertisement
  - ✅ **Phase 3.1: MCP PROXY INTEGRATION** - External MCP Server Connectivity
    - ✅ **MCP Client**: WebSocket-based client for connecting to external MCP servers
    - ✅ **Server Discovery**: Registry system for managing multiple MCP server connections
    - ✅ **Tool Mapping**: Intelligent name mapping and conflict resolution system
    - ✅ **Connection Management**: Robust connection handling with health monitoring
    - ✅ **Tool Aggregation**: Unified interface combining local and remote tools
  - ✅ **Phase 3.2: AUTHENTICATION & SECURITY** - Complete Authentication System
    - ✅ **API Key Authentication**: Fully implemented with Bearer token support and validation
    - ✅ **OAuth 2.0 Authentication**: Complete implementation with GitHub, Google, Microsoft provider support
    - ✅ **JWT Authentication**: Complete implementation with multi-algorithm support (HS256/384/512, RS256/384/512, ES256/384)
    - ✅ **Permission-Based Access Control**: Granular read/write/admin permissions per endpoint
    - ✅ **API Key Management**: Expiration, active/inactive status, and validation
    - ✅ **Security Middleware**: HTTP request validation and error handling with fallback authentication
    - ✅ **Backward Compatibility**: Authentication disabled by default for easy adoption
    - ✅ **Comprehensive Testing**: 34 authentication integration tests covering all scenarios (API Key, OAuth 2.0, JWT)
  - ✅ **Phase 3.4.1: TLS/SSL SECURITY** - Complete Hybrid TLS Architecture
    - ✅ **Application-Level TLS**: Direct HTTPS with rustls for simple deployments
    - ✅ **Reverse Proxy Support**: HTTP mode for nginx/traefik TLS termination with full request inspection
    - ✅ **Auto-Detection Mode**: Smart detection of proxy headers for hybrid environments
    - ✅ **Advanced Security Features**: HSTS, security headers, rate limiting, DDoS protection
    - ✅ **Certificate Monitoring**: Automated certificate health monitoring and expiration alerts
    - ✅ **Comprehensive Testing**: 47 TLS and security tests with 78.7% success rate
  - ✅ **Phase 3.5: GRAPHQL CAPABILITY GENERATOR** - Complete GraphQL Schema Processing
    - ✅ **100% GraphQL Specification Compliance**: SDL and introspection JSON parsing
    - ✅ **Advanced Schema Features**: Extensions, directives, multi-line arguments, circular references
    - ✅ **Comprehensive Testing**: 45 tests covering all GraphQL features
    - ✅ **Real-World Validation**: Complex schemas (9,951 lines, 484 operations)
  - ✅ **Phase 3.6: OPENAPI CAPABILITY GENERATOR** - Complete OpenAPI 3.0 & Swagger 2.0 Processing ← **FULLY COMPLETED**
    - ✅ **OpenAPI 3.0 & Swagger 2.0 Support**: JSON/YAML parsing with auto-detection and format conversion
    - ✅ **Complete HTTP Methods**: All standard HTTP methods with parameter mapping
    - ✅ **Comprehensive Authentication**: API Key, Bearer, Basic, OAuth support
    - ✅ **Advanced Schema Processing**: Complex nested objects, inheritance patterns, and validation
    - ✅ **Component & Reference Resolution**: Full $ref resolution for OpenAPI components and definitions
    - ✅ **Advanced Configuration**: Filtering, naming conventions, tool prefixes
    - ✅ **CLI Tool**: Production-ready command-line interface
    - ✅ **Comprehensive Testing**: 13 tests with real-world validation and Swagger 2.0 support
- **🧪 Testing**: ✅ **Complete** (457+ library tests + comprehensive integration test suites)
  - ✅ **MCP Core Features**: Complete MCP specification compliance (39 tests)
    - ✅ **MCP Logging**: RFC 5424 syslog compliance and rate limiting (13 tests)
    - ✅ **MCP Notifications**: Event-driven notifications and subscriptions (17 tests)
    - ✅ **MCP Integration**: End-to-end server functionality (9 tests)
  - ✅ **Authentication System**: Complete security validation (26 tests) ✅ **UPDATED**
  - ✅ **Agent Router**: Complete multi-agent system validation (19 tests)
  - ✅ **External MCP Integration**: Unified external MCP server management (14 tests)
  - ✅ **gRPC Integration**: Complete streaming protocol validation (6 tests)
  - ✅ **Data Structures**: Comprehensive type validation (26 tests)
  - ✅ **MCP Protocol**: Full JSON-RPC compliance verified (14 tests)
  - ✅ **Integration**: Configuration and CLI testing complete (5 tests)
  - ✅ **Capability Files**: YAML parsing and validation (2 tests)
  - ✅ **MCP Proxy Integration**: External server connectivity validation (8 tests) ✅ **NEW**
  - ✅ **Security Validation**: Input sanitization and attack prevention (9 tests) ✅ **NEW**
  - ✅ **Configuration Validation**: Comprehensive config validation (7 tests) ✅ **NEW**
  - ✅ **Test Coverage Analysis**: Coverage reporting and metrics (4 tests) ✅ **NEW**
  - ✅ **TLS & Security Tests**: Comprehensive TLS and security validation (47 tests) ✅ **NEW**
  - ✅ **Concurrent Architecture**: HTTP and gRPC servers running simultaneously
  - ✅ **Type Safety**: Complete compile-time validation with protobuf integration
- **📚 Documentation**: ✅ **Complete** (Comprehensive documentation suite)
  - ✅ **Agent Routing Documentation**: Complete configuration examples and usage guides
  - ✅ **Capability Testing Guide**: Comprehensive testing documentation with real-world examples
  - ✅ **API Documentation**: Complete API reference for all protocols (WebSocket, HTTP, gRPC)
  - ✅ **Deployment Guide**: Production-ready deployment for Docker and Kubernetes
  - ✅ **Development Guide**: Updated with Phase 2.0 agent routing architecture

## 📊 API Documentation

### Overview

MagicTunnel provides multiple API interfaces for different client needs and performance requirements.

### Supported Protocols

#### 1. WebSocket (Primary)
- **Endpoint**: `ws://localhost:3000/mcp/ws`
- **Protocol**: JSON-RPC 2.0
- **Features**: Real-time bidirectional communication, tool streaming
- **Best For**: Interactive clients, real-time applications

#### 2. HTTP REST API
- **Base URL**: `http://localhost:3000`
- **Protocol**: Standard HTTP with JSON
- **Features**: Simple request/response, easy integration
- **Best For**: Simple integrations, testing, curl-based workflows

#### 3. Server-Sent Events (SSE)
- **Endpoint**: `/mcp/stream`
- **Protocol**: Server-Sent Events with JSON
- **Features**: One-way streaming, progress updates
- **Best For**: Long-running operations, progress monitoring

#### 4. gRPC Streaming
- **Port**: HTTP port + 1000 (default: 4000)
- **Protocol**: gRPC with Protocol Buffers
- **Features**: High-performance binary streaming, type safety
- **Best For**: High-throughput applications, microservice integration

### WebSocket API (JSON-RPC 2.0)

#### Connection
```javascript
const ws = new WebSocket('ws://localhost:3000/mcp/ws');
ws.onopen = () => console.log('Connected to MagicTunnel');
```

#### List Tools
```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "method": "tools/list"
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "result": {
    "tools": [
      {
        "name": "execute_command",
        "description": "Execute bash commands",
        "inputSchema": {
          "type": "object",
          "properties": {
            "command": {"type": "string"}
          }
        }
      }
    ]
  }
}
```

#### Call Tool
```json
{
  "jsonrpc": "2.0",
  "id": "2",
  "method": "tools/call",
  "params": {
    "name": "execute_command",
    "arguments": {
      "command": "echo 'Hello World'"
    }
  }
}
```

### HTTP REST API

#### Health Check
```bash
curl http://localhost:3000/health
```

Response:
```json
{"status": "healthy", "timestamp": "2024-01-15T10:30:00Z"}
```

#### List Tools
```bash
curl http://localhost:3000/tools
```

#### Call Tool
```bash
curl -X POST http://localhost:3000/tools/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "execute_command",
    "arguments": {"command": "echo test"}
  }'
```

### Agent Configuration Examples

#### Subprocess Agent
```yaml
routing:
  type: "subprocess"
  config:
    command: "python3"
    args: ["-c", "{{script}}"]
    timeout: 30
    env:
      PYTHONPATH: "{{env.PYTHONPATH}}"
```

#### HTTP Agent
```yaml
routing:
  type: "http"
  config:
    method: "POST"
    url: "{{api_endpoint}}/process"
    headers:
      Authorization: "Bearer {{env.API_TOKEN}}"
      Content-Type: "application/json"
    body: '{"data": {{input_data}}}'
    timeout: 60
```

### Custom GPT Integration ✅ **COMPLETE**

MagicTunnel now provides **complete OpenAPI 3.1.0 compatibility** for seamless integration with ChatGPT Custom GPTs and other OpenAI-compatible systems.

#### Dual OpenAPI Specification Endpoints

**Full Tools Specification** (100+ tools):
```bash
# Get complete OpenAPI 3.1.0 specification for all enabled tools
curl http://localhost:3001/dashboard/api/openapi.json
```

**Smart Discovery Only** (Perfect for Custom GPT):
```bash
# Get OpenAPI 3.1.0 specification with only smart tool discovery (1 endpoint)
curl http://localhost:3001/dashboard/api/openapi-smart.json
```

#### Features
- **🔧 OpenAPI 3.1.0 Generation**: Latest OpenAPI standard with enhanced JSON Schema support
- **📊 Dual Endpoints**: Choose between full tools access or smart discovery only
- **🎯 Custom GPT Optimized**: Smart discovery endpoint stays under 30-operation limit
- **🎯 Tool Execution Endpoints**: Each tool available at `/dashboard/api/tools/{name}/execute`
- **📋 Complete Documentation**: Full OpenAPI spec with descriptions, parameters, and response schemas
- **🔗 Custom GPT Ready**: Direct integration with ChatGPT Custom GPT Actions
- **⚡ Real-time Updates**: OpenAPI specs reflect current enabled tools dynamically

#### Custom GPT Setup (Recommended)

**Option 1: Smart Discovery Only (Recommended)**
1. **Get Smart OpenAPI Spec**: `curl http://localhost:3001/dashboard/api/openapi-smart.json > smart-spec.json`
2. **Create Custom GPT**: Upload the smart discovery OpenAPI specification to ChatGPT Custom GPT Actions
3. **Configure Instructions**: Add instructions for using natural language with smart_tool_discovery
4. **Test Integration**: Access all MagicTunnel tools through intelligent discovery

**Option 2: Full Tools Access (For Advanced Users)**
1. **Get Full OpenAPI Spec**: `curl http://localhost:3001/dashboard/api/openapi.json > full-spec.json`
2. **Note**: May exceed Custom GPT's 30-operation limit depending on enabled tools
3. **Create Custom GPT**: Upload if under operation limit, otherwise use smart discovery

#### Custom GPT Instructions Template
```
You have access to MagicTunnel's comprehensive toolkit through smart discovery. Use the smartToolDiscovery action with natural language requests like:

- "check system status and disk usage"
- "read the contents of package.json"
- "ping google.com to test connectivity"
- "make GET request to https://api.github.com/user"

Always explain which tool was discovered and executed for transparency.
```

#### Example Tool Execution
```bash
# Execute any MagicTunnel tool via REST API (used by Custom GPT)
curl -X POST http://localhost:3001/dashboard/api/tools/smart_tool_discovery/execute \
  -H "Content-Type: application/json" \
  -d '{
    "request": "check system status",
    "confidence_threshold": 0.7
  }'
```

This integration makes **all MagicTunnel capabilities accessible to ChatGPT users** without requiring MCP client setup, with smart discovery providing natural language access to the entire tool ecosystem while staying under Custom GPT operation limits.

#### LLM Agent
```yaml
routing:
  type: "llm"
  config:
    provider: "openai"
    model: "gpt-4"
    api_key: "{{env.OPENAI_API_KEY}}"
    system_prompt: "You are a helpful assistant."
    user_prompt: "{{user_input}}"
    max_tokens: 2000
```

#### WebSocket Agent
```yaml
routing:
  type: "websocket"
  config:
    url: "wss://api.example.com/ws"
    headers:
      Authorization: "Bearer {{auth_token}}"
    message: '{"action": "{{action}}", "data": {{payload}}}'
    timeout: 45
```

### MCP Logging and Notifications API

#### MCP Logging System

The MCP logging system provides RFC 5424 syslog-compliant logging with 8 severity levels:

**Severity Levels (RFC 5424)**:
- `emergency` (0) - System is unusable
- `alert` (1) - Action must be taken immediately
- `critical` (2) - Critical conditions
- `error` (3) - Error conditions
- `warning` (4) - Warning conditions
- `notice` (5) - Normal but significant condition
- `info` (6) - Informational messages
- `debug` (7) - Debug-level messages

**Log Message via JSON-RPC**:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/message",
  "params": {
    "level": "info",
    "logger": "mcp-server",
    "message": "Tool executed successfully",
    "data": {"tool_name": "execute_command", "duration_ms": 150}
  }
}
```

**Dynamic Log Level Control via HTTP**:
```bash
# Set log level to debug
curl -X POST http://localhost:3000/mcp/logging/setLevel \
  -H "Content-Type: application/json" \
  -d '{"level": "debug"}'

# Set log level for specific logger
curl -X POST http://localhost:3000/mcp/logging/setLevel \
  -H "Content-Type: application/json" \
  -d '{"level": "info", "logger": "agent-router"}'
```

**Rate Limiting**: The logging system implements rate limiting (100 messages per minute per logger) to prevent DoS attacks and log flooding.

#### MCP Notifications System

The notification system provides real-time updates for resource changes, server status, and custom events:

**Subscribe to Resource Updates**:
```json
{
  "jsonrpc": "2.0",
  "id": "3",
  "method": "notifications/subscribe",
  "params": {
    "resource_uri": "file:///project/config.yaml"
  }
}
```

**Resource Update Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/resource_updated",
  "params": {
    "resource_uri": "file:///project/config.yaml",
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

**Server Status Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/message",
  "params": {
    "level": "info",
    "message": "MCP proxy connected to external server",
    "data": {"server_name": "filesystem-server", "endpoint": "ws://localhost:3001"}
  }
}
```

## 📊 Streaming Protocols

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

### Usage Examples

#### WebSocket Connection
```javascript
const ws = new WebSocket('ws://localhost:3000/mcp/ws');
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('Received:', message);
};
```

#### Server-Sent Events
```javascript
const eventSource = new EventSource('/mcp/stream');
eventSource.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Stream update:', data);
};
```

#### HTTP Streaming
```bash
curl -N http://localhost:3000/mcp/call/stream \
  -H "Content-Type: application/json" \
  -d '{"name": "long_running_task", "arguments": {}}'
```

#### gRPC Client Example
```rust
let mut client = McpServiceClient::connect("http://localhost:4000").await?;
let request = tonic::Request::new(ListToolsRequest {});
let response = client.list_tools(request).await?;
```

## 🧪 Testing Guide

### Overview

MagicTunnel includes comprehensive test coverage to ensure reliability and protocol compliance with 457+ tests across multiple test suites.

### Test Architecture

#### Test Organization
- **Unit Tests**: Individual component testing (in `src/` files)
- **Integration Tests**: Cross-component testing (in `tests/` directory)
- **Performance Tests**: Load testing and benchmarking
- **Protocol Tests**: MCP specification compliance verification
- **Capability Tests**: Real-world integration scenarios

### Test Suites

#### 1. Agent Router Tests (`agent_router_test.rs`)
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

#### 2. Data Structures Tests (`data_structures_test.rs`)
- **Coverage**: Comprehensive validation of all MCP types and structures (26 tests)
- **Scope**:
  - JSON-RPC 2.0 message format validation
  - MCP tool definition structure verification
  - Parameter schema validation and type checking
  - Error response format compliance
  - Serialization/deserialization consistency

#### 3. Streaming Protocols Tests (`streaming_protocols_test.rs`)
- **Coverage**: All four streaming protocols (WebSocket, SSE, HTTP, gRPC)
- **Scope**:
  - WebSocket connection handling and message format
  - Server-Sent Events streaming and reconnection
  - HTTP chunked response streaming
  - gRPC unary, server streaming, and bidirectional streaming

#### 4. Performance Tests (`performance_test.rs`)
- **Coverage**: Load testing and performance benchmarking
- **Scope**:
  - High-concurrency tool execution
  - Memory usage and leak detection
  - Response time measurement across protocols
  - Registry lookup performance validation

### Running Tests

#### All Tests
```bash
cargo test
```

#### Specific Test Suite
```bash
cargo test --test agent_router_test
cargo test --test data_structures_test
cargo test --test mcp_server_test
cargo test --test grpc_integration_test
```

#### Specific Test
```bash
cargo test test_subprocess_agent
```

#### With Output
```bash
cargo test -- --nocapture
```

#### Performance Tests Only
```bash
cargo test --test performance_test --release
```

### Test Coverage Metrics

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

**Total**: 457+ tests with comprehensive coverage across all system components

## 🚀 Deployment Guide

### Overview

MagicTunnel provides flexible deployment options for different environments and scale requirements.

### Quick Start

#### Local Development
```bash
# Clone and build
git clone https://github.com/gouravd/magictunnel
cd magictunnel
cargo build --release

# Run with default configuration
./target/release/magictunnel --config config.yaml

# Run in stdio mode for Claude Desktop
./target/release/magictunnel --stdio --config config.yaml

# Connect client to ws://localhost:3000/mcp/ws
```

#### Docker
```bash
# Build Docker image
docker build -t magictunnel .

# Run with configuration
docker run -p 3000:3000 -v $(pwd)/config.yaml:/app/config.yaml magictunnel
```

### Configuration

#### Basic Configuration (config.yaml)
```yaml
server:
  host: "0.0.0.0"
  port: 3000
  websocket: true
  timeout: 30

registry:
  type: "file"
  paths:
    - "./capabilities"
  hot_reload: true

external_mcp:
  enabled: true
  config_file: "external-mcp-servers.yaml"
  capabilities_output_dir: "./capabilities/external-mcp"
  refresh_interval_minutes: 5

logging:
  level: "info"
  format: "json"
```

#### Environment Variables
Override any configuration setting using environment variables:

```bash
# Server settings
export MCP_HOST="0.0.0.0"
export MCP_PORT="8080"
export MCP_WEBSOCKET="true"
export MCP_TIMEOUT="60"

# Registry settings
export MCP_REGISTRY_TYPE="file"
export MCP_REGISTRY_PATHS="./capabilities,./tools"
export MCP_HOT_RELOAD="true"

# Logging settings
export MCP_LOG_LEVEL="debug"
export MCP_LOG_FORMAT="json"
```

### Docker Deployment

#### Dockerfile
```dockerfile
FROM rust:1.70 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/magictunnel .
COPY config.yaml .
COPY capabilities/ ./capabilities/
EXPOSE 3000 4000
CMD ["./magictunnel", "--config", "config.yaml"]
```

#### Docker Compose
```yaml
version: '3.8'
services:
  magictunnel:
    build: .
    ports:
      - "3000:3000"  # HTTP/WebSocket
      - "4000:4000"  # gRPC
    volumes:
      - ./config.yaml:/app/config.yaml
      - ./capabilities:/app/capabilities
      - ./external-mcp-servers.yaml:/app/external-mcp-servers.yaml
    environment:
      - MCP_LOG_LEVEL=info
      - MCP_HOT_RELOAD=true
    restart: unless-stopped
```

### Kubernetes Deployment

#### ConfigMap
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: magictunnel-config
data:
  config.yaml: |
    server:
      host: "0.0.0.0"
      port: 3000
      websocket: true
    registry:
      type: "file"
      paths: ["/app/capabilities"]
    logging:
      level: "info"
      format: "json"
```

#### Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: magictunnel
spec:
  replicas: 3
  selector:
    matchLabels:
      app: magictunnel
  template:
    metadata:
      labels:
        app: magictunnel
    spec:
      containers:
      - name: magictunnel
        image: magictunnel:latest
        ports:
        - containerPort: 3000
        - containerPort: 4000
        volumeMounts:
        - name: config
          mountPath: /app/config.yaml
          subPath: config.yaml
        env:
        - name: MCP_LOG_LEVEL
          value: "info"
      volumes:
      - name: config
        configMap:
          name: magictunnel-config
```

#### Service
```yaml
apiVersion: v1
kind: Service
metadata:
  name: magictunnel-service
spec:
  selector:
    app: magictunnel
  ports:
  - name: http
    port: 80
    targetPort: 3000
  - name: grpc
    port: 4000
    targetPort: 4000
  type: LoadBalancer
```

### Security Configuration

#### TLS/SSL Setup
```yaml
server:
  tls_mode: "application"  # or "reverse_proxy"
  cert_file: "/path/to/cert.pem"
  key_file: "/path/to/key.pem"
  host: "0.0.0.0"
  port: 443
```

#### CORS Configuration
```yaml
server:
  cors:
    allow_origins: ["https://app.example.com"]
    allow_methods: ["GET", "POST"]
    allow_headers: ["Content-Type", "Authorization"]
```

### Monitoring and Observability

#### Health Checks
```bash
# Health check endpoint
curl http://localhost:3000/health

# Detailed status
curl http://localhost:3000/status
```

#### Metrics Endpoint
```bash
curl http://localhost:3000/metrics
```

#### Logging Configuration
```yaml
logging:
  level: "info"                    # debug|info|notice|warning|error|critical|alert|emergency
  format: "json"                   # json|text
  file: "/var/log/magictunnel.log"  # Optional file output
  rate_limit:
    enabled: true
    max_messages_per_minute: 100
```

## 📚 Documentation

- **[docs/AUTHENTICATION.md](docs/AUTHENTICATION.md)**: Complete authentication system guide
- **[docs/TLS_ARCHITECTURE.md](docs/TLS_ARCHITECTURE.md)**: TLS/SSL architecture and implementation guide
- **[docs/TLS_DEPLOYMENT_GUIDE.md](docs/TLS_DEPLOYMENT_GUIDE.md)**: Complete TLS deployment guide for all scenarios
- **[TODO.md](TODO.md)**: Detailed implementation roadmap and progress tracking
- **[capabilities/testing/README.md](capabilities/testing/README.md)**: Testing framework documentation

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on:
- Code style and standards
- Testing requirements
- Pull request process
- Issue reporting

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Related Projects

- [Model Context Protocol](https://github.com/modelcontextprotocol/specification) - Official MCP specification
- [Magic Beans Orchestrator](https://github.com/your-org/orchestrator) - Advanced agent orchestration system
- [MCP Servers](https://github.com/modelcontextprotocol/servers) - Official MCP server implementations

---

Sample Claude config

{
  "mcpServers": {
    "magictunnel": {
      "command": "/<path to project>/magictunnel",
      "args": [
        "--stdio",
        "--config", "/<path to project>/magictunnel-config.yaml"
      ],
      "env": {
        "MCP_REGISTRY_PATHS": "<path to project>/capabilities",
        "OPENAI_API_KEY": "ask-your-admin",
        "MAGICTUNNEL_ENV": "development|staging|production",
        "PATH": "<path_to_node_modules and Python_binaries>n",
        "SMART_DISCOVERY_MODE": "llm_based"
      },
      "cwd": "/<path_to_project>/magictunnel"
    }
  }
}

## 🎯 Key Design Principles

- **Flexible File Organization**: Support any number of capability files in any structure
- **Simple Tool Definitions**: No complex atomic/composite classifications
- **Routing-Focused**: Core purpose is routing tool calls, not orchestration
- **MCP Compliant**: Full compatibility with MCP protocol standards
- **Hot-Reloadable**: Dynamic capability discovery and updates
- **Comprehensive Testing**: All streaming protocols validated with performance benchmarks
- **Enterprise Ready**: Authentication, monitoring, and production deployment support
- **Developer Friendly**: Clear documentation, comprehensive examples, and intuitive configuration

**Built with ❤️ for the intelligent agent ecosystem**
