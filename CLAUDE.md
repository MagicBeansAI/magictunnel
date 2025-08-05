# MagicTunnel - Guide for Claude Code

## Overview

MagicTunnel is an intelligent bridge between MCP (Model Context Protocol) clients and diverse agents/endpoints. It provides a single, smart tool discovery interface that can find the right tool for any request, map parameters, and proxy the call automatically.

**Current Version**: 0.3.7 - **Client Capability Tracking & Elicitation Logic Fix Complete** ✅

## Quick Start

### Build and Run
```bash
# Build the project
make build-release-semantic && make pregenerate-embeddings-ollama MAGICTUNNEL_ENV=development

# Run with custom config
./target/release/magictunnel --config magictunnel-config.yaml

# Run in stdio mode for MCP clients (Claude Desktop, Cursor)
./target/release/magictunnel --stdio --config magictunnel-config.yaml

# Test the service
curl -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"name": "smart_tool_discovery", "arguments": {"request": "ping google.com"}}'
```

### Development Commands
```bash
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

# Visibility management CLI
cargo run --bin magictunnel-visibility -- -c config.yaml status
```

## High-Level Architecture

MagicTunnel implements a **Smart Tool Discovery and Proxy** system that reduces N tools to 1 intelligent proxy tool. This solves the message limit problem in MCP systems where having many tools (50+) causes context overflow.

### Core Components

1. **MCP Server Interface** (`src/mcp/server.rs`)
   - Implements the Model Context Protocol for communication with MCP clients
   - Handles tool discovery, parameter mapping, and result proxying
   - Supports both stdio and HTTP modes

2. **Capability Registry** (`src/registry/`)
   - Manages tool definitions from multiple sources (OpenAPI, gRPC, GraphQL)
   - Handles tool aggregation and validation
   - Supports dynamic loading of capabilities
   - **Visibility Management**: Tools can be hidden/shown with `hidden` flag

3. **Agent Router** (`src/routing/`)
   - Routes tool calls to appropriate external agents/endpoints
   - Handles conflict resolution when multiple tools match
   - Implements parameter substitution with array indexing support

4. **External MCP Integration** (`src/mcp/external_*`)
   - Manages external MCP processes and agents
   - Handles websocket and stdio communication
   - Provides process lifecycle management
   - **Bidirectional Communication**: Complete MCP 2025-06-18 bidirectional routing with modern clients/ architecture

5. **Smart Tool Discovery System** (`src/discovery/`)
   - **THE CORE INNOVATION**: Single intelligent tool that discovers the right tool for any request
   - **Hybrid AI Intelligence**: Combines semantic search, rule-based matching, and LLM analysis
   - **MCP 2025-06-18 Enhanced**: Uses tool enhancement service for enhanced tool descriptions and elicitation service for metadata
   - **Parameter mapping**: Uses LLM to extract and map parameters from natural language with elicitation validation
   - **Confidence scoring**: Provides confidence scores for tool matches using enhanced descriptions
   - **Semantic Search**: Embedding-based tool matching using AI-enhanced descriptions
   - **Intelligent Elicitation**: Tool discovery elicitation only works when smart discovery is disabled (logical behavior)

6. **Visibility Management System** (`src/bin/magictunnel-visibility.rs`)
   - **CLI Tool**: Complete tool visibility control
   - **Hidden by Default**: All 83 tools across 15 capability files hidden by default
   - **Smart Discovery Mode**: Clean interface with full functionality through discovery

### Smart Discovery System (Key Innovation with MCP 2025-06-18 Enhancement)

The system provides **one intelligent tool** (`smart_tool_discovery`) that:
1. Analyzes natural language requests using hybrid AI intelligence
2. Finds the best matching tool using enhanced descriptions from tool enhancement service
3. Maps parameters from natural language to tool schema with elicitation service validation
4. Proxies the call to the actual tool
5. Returns results with discovery metadata and enhancement information

**MCP 2025-06-18 Integration:**
- **Enhanced Descriptions**: Uses tool enhancement service for AI-improved tool descriptions (better semantic matching)
- **Rich Metadata**: Leverages elicitation service for extracted keywords, categories, and use cases
- **Smart Fallback**: Gracefully degrades to base descriptions when enhancement services are unavailable
- **Performance Optimization**: Caches enhanced content to maintain sub-second response times

**Discovery modes:**
- `hybrid` (recommended): Combines semantic search (30%), rule-based (15%), and LLM analysis (55%) using enhanced descriptions
- `rule_based`: Fast keyword matching and pattern analysis with elicitation metadata
- `semantic`: Embedding-based similarity search using sampling-enhanced descriptions
- `llm_based`: AI-powered tool selection with OpenAI/Anthropic/Ollama APIs using enhanced metadata

## Important Files

### Configuration
- `magictunnel-config.yaml` - Main configuration file
- `config.yaml.template` - Template for configuration with comprehensive documentation
- `capabilities/` - Directory containing capability definitions (YAML format)

### Key Source Files
- `src/discovery/service.rs` - Smart discovery implementation with hybrid AI intelligence
- `src/discovery/semantic.rs` - Semantic search with embedding-based tool matching
- `src/routing/substitution.rs` - Parameter substitution with array indexing
- `src/mcp/clients/` - **Modern MCP 2025-06-18 client implementations** ✅
  - `http_client.rs` - HTTP MCP client with request/response handling
  - `websocket_client.rs` - WebSocket client with full-duplex communication
  - `sse_client.rs` - Server-Sent Events client with streaming support
  - `streamable_http_client.rs` - NDJSON streaming client (MCP 2025-06-18 preferred)
- `src/mcp/server.rs` - MCP protocol implementation
- `src/mcp/external_manager.rs` - External MCP server management with bidirectional routing
- `src/mcp/external_integration.rs` - External MCP integration layer with elicitation support
- `src/mcp/types/capabilities.rs` - Client capability tracking for MCP 2025-06-18 compliance
- `src/registry/service.rs` - Capability registry management with visibility support
- `src/bin/magictunnel-visibility.rs` - CLI tool for visibility management
- `src/main.rs` - Application entry point

### Documentation
- `docs/ROUTING_ARCHITECTURE.md` - Detailed architecture documentation with Phase 4 completion status
- `docs/BIDIRECTIONAL_COMMUNICATION_FLOW.md` - **Complete MCP 2025-06-18 bidirectional communication flow** ✅
- `CHANGELOG.md` - Version history and changes (current: 0.3.6)
- `README.md` - Comprehensive project overview with current status
- `how_to_run.md` - Quick setup guide with examples

## Common Development Patterns

### Adding New Tool Support
1. Create capability definition in `capabilities/` directory (YAML format)
2. Tool will be automatically discovered and included in registry
3. Smart discovery will handle parameter mapping and routing
4. Use `hidden: true` to hide from main tool list while keeping discoverable

### Managing Tool Visibility
```bash
# Check current visibility status
cargo run --bin magictunnel-visibility -- -c config.yaml status

# Hide/show individual tools
cargo run --bin magictunnel-visibility -- -c config.yaml hide-tool tool_name
cargo run --bin magictunnel-visibility -- -c config.yaml show-tool tool_name

# Hide/show entire capability files
cargo run --bin magictunnel-visibility -- -c config.yaml hide-file capabilities/file.yaml
cargo run --bin magictunnel-visibility -- -c config.yaml show-file capabilities/file.yaml

# Global visibility management
cargo run --bin magictunnel-visibility -- -c config.yaml hide-all
cargo run --bin magictunnel-visibility -- -c config.yaml show-all
```

### Debugging Smart Discovery
```bash
# Enable debug logging for discovery
RUST_LOG=magictunnel::discovery=debug cargo run

# Test specific tool discovery
curl -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"name": "smart_tool_discovery", "arguments": {"request": "ping google.com", "confidence_threshold": 0.5}}'
```

### Configuration Updates
- Smart discovery config is in `smart_discovery` section
- Discovery modes: `hybrid`, `rule_based`, `semantic`, `llm_based`
- LLM providers: OpenAI, Anthropic, Ollama
- Semantic search with embedding support
- Confidence thresholds and caching settings
- Visibility management with `default_hidden` setting

## Recent Major Changes

### Version 0.3.2 (Current) - Advanced MCP Platform with LLM Integration ✅

#### **🚀 MCP 2025-06-18 Specification Compliance (Backend Complete)**
- **Full MCP 2025-06-18 Implementation**: Complete backend implementation of latest MCP spec with MCP sampling and elicitation services
- **OAuth 2.1 Framework**: Backend authentication implementation with PKCE and Resource Indicators (RFC 8707)
- **Dual Transport Support**: HTTP+SSE (deprecated) and Streamable HTTP (preferred) with graceful migration
- **Enhanced Security Model**: Backend MCP-specific consent flows and capability validation
- **Streamable HTTP Transport**: NDJSON streaming, enhanced batching, and session management
- **Backward Compatibility**: Maintained HTTP+SSE support at `/mcp/stream` with deprecation guidance

#### **🤖 Automatic LLM Generation Workflow (Backend Complete)**
- **Sampling Service**: AI-powered tool description enhancement with OpenAI/Anthropic/Ollama support
- **Elicitation Service**: Automatic metadata extraction and parameter validation using structured LLM analysis
- **Enhancement Pipeline**: Coordinated sampling + elicitation with parallel processing and error handling
- **LLM Management CLI**: Unified `magictunnel-llm` tool for all LLM service management with external MCP protection
- **External MCP Protection**: Automatic detection and protection of external MCP server content with warnings
- **Performance Optimization**: Multi-level caching, rate limiting, and asynchronous processing for enterprise scale

#### **🎨 LLM Backend Management APIs (Complete)**
- **Resource Management APIs**: 7 comprehensive REST endpoints for resource browsing, reading, validation, and statistics
- **Enhancement Pipeline APIs**: 9 complete endpoints for tool enhancement management, job tracking, and cache control
- **Prompt Management APIs**: Complete backend implementation with full CRUD operations
- **Sampling Service APIs**: Full management interface for AI-powered tool enhancement with provider health monitoring
- **Elicitation Service APIs**: Complete metadata extraction and validation management with batch processing
- **Provider Management APIs**: LLM provider configuration, testing, and health monitoring across OpenAI/Anthropic/Ollama
- **Statistics and Analytics**: Comprehensive analytics for resource types, provider health, and enhancement metrics
- **Batch Processing Support**: Enhanced batch operations for tool enhancement and resource management

#### **🔒 Security Features (Backend Complete, UI In Progress)**
- **Security CLI**: `magictunnel-security` tool for policy management and security validation
- **Authentication Framework**: Backend OAuth 2.1 implementation and API key support
- **Configuration Security**: Granular security policy configuration support
- **Audit Framework**: Backend audit logging infrastructure
- ⚠️ **UI Pending**: Web-based security management interface and visual policy builder in development

#### **🎨 Frontend and UI (Partial Implementation)**
- **Basic Dashboard**: Existing web dashboard with tool management
- **Accessibility Planning**: WCAG 2.1 AA compliance requirements documented in TODO.md
- ⚠️ **LLM UI Pending**: Frontend for sampling, elicitation, and enhancement management needs implementation
- ⚠️ **Security UI Pending**: Visual policy builder and security management UI in development
- ⚠️ **Enterprise UI Pending**: Advanced enterprise management interfaces planned

#### **⚙️ Enhanced Configuration System (Complete)**
- **YAML Format Evolution**: Enhanced capability file format with metadata support and versioning
- **Service Configuration**: Comprehensive LLM provider and enhancement settings with validation
- **Security Configuration**: Backend security policy and configuration management
- **Performance Tuning**: Caching, batching, and optimization settings for enterprise deployments
- **Environment Management**: Advanced environment variable and deployment configuration support

#### **🛠️ Developer and Operations Tools (Backend Complete)**
- **Advanced CLI Tools**: Complete suite including `magictunnel-llm` and `magictunnel-security` for enterprise management
- **OpenAPI 3.1 Integration**: Complete Custom GPT support and API generation for seamless integrations
- **Enhanced Documentation**: Comprehensive documentation including automatic LLM generation workflow guide
- **Claude Desktop Compatibility**: Fixed Claude not working issues with full MCP compliance
- **Sequential Mode**: Enhanced sequential mode functionality

### Version 0.3.6 (Current) - Legacy Client Removal & Modern Architecture Migration Complete ✅
- **Complete Legacy Client Migration**: Successfully migrated from deprecated monolithic `client.rs` to modern modular `clients/` architecture
  - **Modern Client Architecture**: 4 specialized client implementations (HTTP, WebSocket, SSE, StreamableHTTP)
  - **Test Migration Complete**: All 4 test files migrated from legacy client to configuration validation
  - **Legacy Code Removal**: Removed ~2,700 lines of deprecated client.rs code
  - **Clean Architecture**: Only modern, specialized clients remain with MCP 2025-06-18 compliance
  - **Configuration-Based Testing**: Replaced routing calls with data structure and configuration validation
- **Migration Benefits**: Reduced codebase size, eliminated deprecation warnings, better maintainability
- **Production Readiness**: Clean compilation, all tests passing, modern architecture operational

### Version 0.3.4 - Configuration Documentation and Test Infrastructure ✅
- **Complete LLM Backend APIs Test Coverage**: 60+ test functions across 6 test suites
  - **Elicitation Service API Tests**: 10 comprehensive test functions covering metadata extraction and batch processing
  - **Sampling Service API Tests**: 12 comprehensive test functions covering tool enhancement and content generation
  - **Enhanced Resource Management API Tests**: 12 detailed test functions with filtering, pagination, and content reading
  - **Enhanced Prompt Management API Tests**: 14 comprehensive test functions covering CRUD operations and template management
  - **Enhanced Ranking and Discovery Tests**: 12 advanced test functions for updated ranking algorithms with LLM integration
  - **LLM Backend APIs Integration Tests**: 5 comprehensive integration test functions across all services
- **Test Infrastructure**: Complete API testing framework with realistic environments and comprehensive validation
- **Quality Assurance**: Enterprise-grade test coverage for production deployment

### Version 0.2.x (Previous Releases)
- **Smart Tool Discovery Complete**: Hybrid AI intelligence system
- **Visibility Management System**: Complete implementation with CLI tool
- **Semantic Search**: Embedding-based tool matching
- **Ultimate Smart Discovery Mode**: All 83 tools hidden by default
- **Parameter substitution**: Array indexing support (`{hosts[0]}`)
- **LLM API integration**: OpenAI, Anthropic, Ollama support
- **Batch processing**: Handle large tool catalogs efficiently
- **Enhanced keyword matching**: Better networking tool recognition

### Migration Notes
- **Legacy Client Removal (v0.3.6)**: Deprecated monolithic `client.rs` removed in favor of modern modular `clients/` architecture
- External MCP integration replaced remote/local MCP modules
- Smart discovery replaces individual tool exposure
- Configuration moved from separate files to unified config structure
- All tools now hidden by default with smart discovery interface
- Visibility management available through CLI tool

## Common Issues

### Build Issues
- Ensure Rust 1.70+ is installed
- Check that all dependencies are available
- Use `cargo clean` if encountering cache issues

### Runtime Issues
- **Smart discovery low confidence**: Check hybrid AI matching in `src/discovery/service.rs`
- **Semantic search not working**: Verify OpenAI API key and embedding generation
- **Parameter substitution errors**: Verify array indexing syntax in substitution.rs
- **External MCP not starting**: Check file permissions and working directory
- **Tool visibility issues**: Use `magictunnel-visibility` CLI to check/modify tool visibility
- **Transport compatibility**: Use `/mcp/streamable` (preferred) or `/mcp/stream` (deprecated) endpoints

### Git Issues
- Binary `magictunnel` is in `.gitignore` but may be tracked - use `git rm --cached magictunnel`

## Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test discovery

# Run integration tests
cargo test --test integration

# Test smart discovery specifically
cargo test smart_discovery

# Test visibility management
cargo test visibility

# Test semantic search
cargo test semantic
```

## CLI Tools

MagicTunnel includes several powerful CLI tools for comprehensive system management:

### 1. Main Server (`magictunnel`)
```bash
# Start the main MCP server
cargo run --bin magictunnel -- --config config.yaml

# Start with stdio mode for Claude Desktop
cargo run --bin magictunnel -- --stdio --config config.yaml
```

### 2. LLM Management (`magictunnel-llm`) 🆕
```bash
# Complete health check for all LLM services
cargo run --bin magictunnel-llm -- bulk health-check

# Generate enhanced descriptions
cargo run --bin magictunnel-llm -- sampling generate --tool example_tool

# Extract metadata and validation rules
cargo run --bin magictunnel-llm -- elicitation generate --tool example_tool

# Full enhancement pipeline
cargo run --bin magictunnel-llm -- enhancements regenerate --batch-size 5

# Generate prompts and resources
cargo run --bin magictunnel-llm -- prompts generate --tool example_tool
cargo run --bin magictunnel-llm -- resources generate --tool example_tool

# Provider management
cargo run --bin magictunnel-llm -- providers test --all
```

### 3. Security Management (`magictunnel-security`) 🆕
```bash
# Security policy management
cargo run --bin magictunnel-security -- policies list
cargo run --bin magictunnel-security -- policies validate

# Allowlist management
cargo run --bin magictunnel-security -- allowlist add-tool tool_name
cargo run --bin magictunnel-security -- allowlist status
```

### 4. Visibility Management (`magictunnel-visibility`)
```bash
# Check tool visibility status
cargo run --bin magictunnel-visibility -- -c config.yaml status

# Manage individual tools
cargo run --bin magictunnel-visibility -- -c config.yaml hide-tool tool_name
cargo run --bin magictunnel-visibility -- -c config.yaml show-tool tool_name

# Manage entire files
cargo run --bin magictunnel-visibility -- -c config.yaml hide-file capabilities/file.yaml
cargo run --bin magictunnel-visibility -- -c config.yaml show-file capabilities/file.yaml

# Global management
cargo run --bin magictunnel-visibility -- -c config.yaml hide-all
cargo run --bin magictunnel-visibility -- -c config.yaml show-all
```

## Environment Variables

```bash
# Enable debug logging
export RUST_LOG=debug

# Custom config path
export MAGICTUNNEL_CONFIG=./my-config.yaml

# LLM API keys (for smart discovery)
export OPENAI_API_KEY=your_key
export ANTHROPIC_API_KEY=your_key
export OLLAMA_BASE_URL=http://localhost:11434

# Semantic search configuration
export MAGICTUNNEL_SEMANTIC_ENABLED=true
export MAGICTUNNEL_EMBEDDING_MODEL=text-embedding-3-small
```

## Current Status

### Implementation Status
- **Total Tools**: 83 across 15 capability files
- **Visible Tools**: 0 (complete Smart Tool Discovery mode)
- **Hidden Tools**: 83 (all available through discovery)
- **Smart Discovery**: Hybrid AI intelligence with semantic search
- **CLI Management**: Full visibility control with real-time status

### Implementation Status Overview

#### **Backend Services Complete ✅**
- ✅ **MCP 2025-06-18 Backend**: Full specification implementation with MCP sampling and elicitation services
- ✅ **MCP Client Bidirectional Communication**: Complete routing implementation with all 6 ProcessingStrategy variants
- ✅ **Automatic LLM Generation**: AI-powered tool enhancement with multi-provider support (backend complete)
- ✅ **LLM Backend Management APIs**: Complete REST API implementation for all LLM services (25+ endpoints)
- ✅ **Security Framework**: Backend authentication, policy framework, and audit logging
- ✅ **Smart Tool Discovery**: Hybrid AI intelligence with MCP 2025-06-18 enhanced metadata integration
- ✅ **Advanced Configuration**: Enhanced YAML format with comprehensive settings
- ✅ **External MCP Protection**: Automatic detection and content preservation
- ✅ **Performance Optimization**: Multi-level caching and asynchronous processing
- ✅ **CLI Tools**: Complete suite including `magictunnel-llm` and `magictunnel-security`
- ✅ **Visibility Management**: Complete implementation with real-time control

#### **UI and Enterprise Features In Progress ⚠️**
- ⚠️ **LLM Services UI**: Frontend for sampling, elicitation, and enhancement management (planned)
- ⚠️ **Security Management UI**: Visual policy builder and allowlisting interface (in development)
- ⚠️ **Enterprise Dashboard**: Advanced enterprise management interfaces (planned)
- ⚠️ **Accessibility**: WCAG 2.1 AA compliance implementation (requirements documented)
- ⚠️ **Review Workflows**: Content approval and review interfaces for LLM-generated content (planned)

## Current Status Summary

### **🎯 Advanced MCP Platform with Complete Backend APIs**
MagicTunnel has evolved into a sophisticated MCP platform with:
- **Complete MCP 2025-06-18 backend compliance** with modern protocol features and services
- **Automatic LLM generation workflow** backend implementation for intelligent tool enhancement
- **Comprehensive LLM Backend Management APIs** with 25+ REST endpoints for all LLM services
- **Security framework** including authentication, policy management, and audit logging
- **Advanced CLI tooling** for comprehensive system management
- **Enhanced configuration system** supporting complex deployments
- **Smart tool discovery** with MCP 2025-06-18 enhanced metadata integration

### **🚧 Development Roadmap**
Current development priorities:
1. **UI Development**: Frontend interfaces for LLM services, security management, and enterprise features
2. **Enterprise Features**: Visual policy builders, content approval workflows, and advanced dashboards
3. **Accessibility Implementation**: WCAG 2.1 AA compliance across all UI components
4. **Integration Testing**: End-to-end testing of MCP 2025-06-18 features

### **🔄 Migration and Upgrade Path**
For existing installations:
1. **Configuration Migration**: Enhanced YAML format with backward compatibility
2. **Service Integration**: Optional LLM services with fallback to original descriptions
3. **CLI Access**: New management capabilities available through enhanced CLI tools
4. **Progressive Enhancement**: Backend features available immediately, UI features following

### **📊 Performance and Scale**
- **83+ tools** managed with smart discovery
- **Sub-second response times** with multi-level caching
- **Backend enterprise features** without performance impact
- **Horizontal scaling** with distributed caching support

This guide covers the MagicTunnel platform as currently implemented. The combination of Smart Tool Discovery, Automatic LLM Enhancement Backend, Security Framework, and Advanced CLI Tools provides a powerful foundation for MCP-based workflows, with comprehensive UI features planned for future releases.