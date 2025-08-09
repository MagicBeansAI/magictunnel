# MagicTunnel - Guide for Claude Code

## Overview

MagicTunnel is an intelligent bridge between MCP (Model Context Protocol) clients and diverse agents/endpoints. It provides a single, smart tool discovery interface that can find the right tool for any request, map parameters, and proxy the call automatically.

**Current Version**: 0.3.11 - **Multi-Mode Architecture & Unified Status Banner System Complete** ✅

## Quick Start

### Build and Run
```bash
# Build the project
make build-release-semantic && make pregenerate-embeddings-ollama MAGICTUNNEL_ENV=development

# Run with default magictunnel-config.yaml (auto-detected)
./target/release/magictunnel

# Run with environment variable overrides
MAGICTUNNEL_RUNTIME_MODE=advanced MAGICTUNNEL_SMART_DISCOVERY=true ./target/release/magictunnel

# Run in stdio mode for MCP clients (Claude Desktop, Cursor)
./target/release/magictunnel --stdio

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

# Run in different modes
MAGICTUNNEL_RUNTIME_MODE=proxy cargo run
MAGICTUNNEL_RUNTIME_MODE=advanced cargo run

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

### Multi-Mode Service Architecture

MagicTunnel implements a **two-tier service architecture** that separates core MCP functionality from advanced enterprise features:

#### **Core/Proxy Services** (Available in both Proxy and Advanced modes):
1. **MCP Server** - Core protocol handling with built-in MCP authentication (API keys, OAuth, JWT)
2. **Registry Service** - Tool management and capability discovery
3. **Smart Discovery Service** - Intelligent tool routing and parameter mapping
4. **Core LLM Services** - Sampling, elicitation, and tool enhancement services for basic AI functionality
5. **Web Dashboard** - Basic web interface via MCP server endpoints
6. **MCP Authentication** - Protocol-level authentication middleware

#### **Advanced Services** (Enterprise features, Advanced mode only):
1. **Enterprise Security Suite**:
   - Tool Allowlisting - Enterprise tool security controls
   - Advanced RBAC (Role-Based Access Control)
   - Request Sanitization - Enterprise request filtering
   - Comprehensive audit logging and analytics
   - Security Policies - Organization-wide policy management
   - Emergency lockdown capabilities
2. **Future: MagicTunnel Authentication** - User authentication for MagicTunnel itself (separate from MCP protocol auth)

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
- `src/mcp/types/capabilities.rs` - Client capability tracking with minimum intersection capability advertisement
- `src/registry/service.rs` - Capability registry management with visibility support
- `src/bin/magictunnel-visibility.rs` - CLI tool for visibility management
- `src/main.rs` - Application entry point

### Documentation
- `docs/ROUTING_ARCHITECTURE.md` - Detailed architecture documentation with Phase 4 completion status
- `docs/BIDIRECTIONAL_COMMUNICATION_FLOW.md` - **Complete MCP 2025-06-18 bidirectional communication flow** ✅
- `CHANGELOG.md` - Version history and changes (current: 0.3.8)
- `README.md` - Comprehensive project overview with current status
- `how_to_run.md` - Quick setup guide with examples

## Common Development Patterns

### Multi-Mode Service Architecture Usage

**Core/Proxy Services** (always available):
- MCP protocol authentication (API keys, OAuth, JWT)
- Core LLM services (sampling, elicitation, tool enhancement)
- Smart discovery and tool routing
- Web dashboard basic functionality

**Advanced Services** (advanced mode only):
- Enterprise security features (allowlisting, RBAC, audit, sanitization, policies, emergency lockdown)
- Advanced analytics and monitoring
- Security policies and emergency controls

### Adding New Tool Support
1. Create capability definition in `capabilities/` directory (YAML format)
2. Tool will be automatically discovered and included in registry
3. Smart discovery will handle parameter mapping and routing
4. Use `hidden: true` to hide from main tool list while keeping discoverable

### Service Development Guidelines

**For Core Services** (available in both modes):
- Implement in `src/mcp/`, `src/registry/`, or `src/discovery/`
- Include MCP protocol authentication if needed
- Ensure compatibility with both proxy and advanced modes
- Focus on essential MCP functionality

**For Advanced Services** (advanced mode only):
- Implement in `src/security/` for security features
- Use `src/services/advanced_services.rs` for service management
- Add configuration checks for advanced mode
- Provide graceful degradation when unavailable

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

### **Environment Variables (v0.3.10)**
MagicTunnel now supports comprehensive environment variable configuration:

```bash
# Runtime Mode Control
export MAGICTUNNEL_RUNTIME_MODE=proxy       # proxy | advanced
export MAGICTUNNEL_CONFIG_PATH=./config.yaml # Custom config file path
export MAGICTUNNEL_SMART_DISCOVERY=true     # Enable/disable smart discovery

# Run with environment overrides
./target/release/magictunnel
```

### **Default Configuration (v0.3.10)**
- **New Default Config Name**: `magictunnel-config.yaml` (replaces `config.yaml`)
- **Auto-Detection**: Automatically detects and uses `magictunnel-config.yaml` if present
- **Built-in Defaults**: Minimal proxy mode defaults when no config file found
- **Environment Priority**: Environment variables override config file settings

## Startup Flow and Service Architecture

### Complete Startup Sequence (v0.3.10)

MagicTunnel implements a sophisticated startup flow that conditionally loads services based on runtime mode:

```
1. Configuration Resolution (main.rs:92-113):
   ├─ Load config file (magictunnel-config.yaml or config.yaml)
   ├─ Apply environment variable overrides (highest priority)
   ├─ Determine runtime mode (proxy vs advanced)
   └─ Validate configuration for selected mode

2. Service Loading (main.rs:154-161):
   ├─ ServiceLoader::load_services(resolution)
   ├─ RuntimeMode detection from ConfigResolution
   └─ Conditional service container creation

3. Service Container Strategy:
   Proxy Mode:
   ├─ ProxyServices::new() → Core services only
   └─ ServiceContainer { proxy_services: Some, advanced_services: None }
   
   Advanced Mode:
   ├─ ProxyServices::new() → Core services (foundation)
   ├─ AdvancedServices::new(&proxy_services) → Enterprise features
   └─ ServiceContainer { proxy_services: Some, advanced_services: Some }

4. Backend API Integration (ModeApiHandler):
   ├─ /api/mode → Runtime mode detection for frontend
   ├─ /api/config → Configuration validation status
   └─ /api/services/status → Service health monitoring

5. Frontend Mode Awareness:
   ├─ mode.ts store fetches backend mode information
   ├─ ModeAwareLayout subscribes to mode stores
   ├─ Components filter UI elements based on showAdvancedFeatures
   └─ Progressive enhancement based on available services
```

### Service Loading Architecture

**ServiceLoader Implementation** (`src/services/service_loader.rs`):
- **load_services()**: Main entry point that determines runtime mode and delegates loading
- **load_proxy_services()**: Loads core MCP functionality (ProxyServices container)
- **load_advanced_services()**: Loads ProxyServices + AdvancedServices (no duplication)
- **validate_service_dependencies()**: Ensures proper service health and dependencies

**Service Containers**:
- **ProxyServices**: MCP Server, Registry, Smart Discovery, Tool Enhancement, Web Dashboard integration
- **AdvancedServices**: Enterprise Security Suite (Allowlisting, RBAC, Audit, Sanitization, Policies, Emergency Lockdown)
- **ServiceContainer**: Wrapper that manages both containers with runtime mode awareness

### Frontend Mode-Aware Architecture

**Mode Detection & Integration**:
```
Backend (Rust):
src/web/mode_api.rs → ModeApiHandler
├─ create_ui_config() → Mode-specific UI configuration
├─ create_navigation_sections() → Dynamic navigation structure
└─ create_status_indicators() → Health monitoring indicators

Frontend (Svelte):
frontend/src/lib/stores/mode.ts → Mode detection store
├─ fetchAll() → Loads mode, config, and service status
├─ Derived stores: runtimeMode, isAdvancedMode, navigationSections
└─ Auto-refresh every 30 seconds

Components:
├─ ModeAwareLayout.svelte → Central mode integration
├─ TopBar.svelte → Mode-aware user menu, notifications, search
├─ Sidebar.svelte → Mode-aware navigation and status indicators
└─ Progressive enhancement based on showAdvancedFeatures
```

**UI Filtering Logic**:
- **Navigation**: Items with `requires_advanced: true` hidden in proxy mode
- **Status Indicators**: Security/Auth indicators hidden in proxy mode
- **User Menu**: Security, Analytics, User Management links hidden in proxy mode
- **Notifications**: Security and audit alerts filtered in proxy mode
- **Search Results**: Advanced pages (security, LLM services) hidden in proxy mode

## Recent Major Changes

### Version 0.3.11 (Current) - Multi-Mode Architecture & Unified Status Banner System Complete ✅

#### **🎨 Unified Status Banner System**
- **Modern UI Design**: Replaced bulky proxy mode alerts with clean, minimal status bar system
- **Dynamic Status Messages**: Real-time updates during restart/mode switch operations with color-coded types
- **Space Efficient**: 60% reduction in visual space while maintaining clarity and impact
- **Consistent Experience**: All status messages (proxy mode, restart, mode switch, errors) use unified design
- **Responsive & Accessible**: Clean layout on all devices with proper dark mode support

#### **📊 Dashboard Layout Enhancement** 
- **System Management Repositioning**: Moved critical controls (restart, mode switch, health check) to top of dashboard
- **Better Information Hierarchy**: Management actions now appear before status information for improved UX
- **Unified Restart Behavior**: Both restart and mode switch operations now have identical clean page reload behavior

#### **🔧 Status Message Examples**:
```
[●] Running in Proxy Mode • Core features only
[●] Restarting System (15s remaining) • System restarting... Checking server readiness.
[●] Mode Switch Complete • Successfully switched to advanced mode and system is online.
[●] Restart Failed • Failed to restart system: Connection timeout
```

### Version 0.3.10 - Multi-Mode Architecture Implementation Complete ✅

#### **🏗️ Multi-Mode Architecture Complete**
- **Pure Config-Driven Architecture**: All behavior controlled via config file and environment variables
- **Environment Variable Integration**: Complete MAGICTUNNEL_RUNTIME_MODE, CONFIG_PATH, and SMART_DISCOVERY support
- **Default Config Resolution**: magictunnel-config.yaml auto-detection with built-in proxy defaults
- **Comprehensive Startup Logging**: Config resolution, feature status, and validation logging with detailed output
- **Configuration Validation System**: Mode-specific validators with helpful error messages for proxy and advanced modes
- **Conditional Service Loading**: ProxyServices vs AdvancedServices based on runtime mode
- **Frontend Mode Awareness**: Mode detection API with progressive enhancement for advanced features

#### **🎯 Runtime Mode System**
- **Proxy Mode (Default)**: Zero-config, minimal dependencies, fast startup for basic MCP proxy functionality
- **Advanced Mode**: Full-featured, enterprise-ready with comprehensive management and security features
- **Environment Override Priority**: Environment variables take precedence over config file settings
- **Smart Service Loading**: Only loads required services based on runtime mode selection

#### **⚙️ Configuration Examples**
```bash
# Environment variable override (highest priority)
export MAGICTUNNEL_RUNTIME_MODE=advanced  # proxy | advanced
export MAGICTUNNEL_CONFIG_PATH=./my-config.yaml
export MAGICTUNNEL_SMART_DISCOVERY=true
./magictunnel
```

```yaml
# magictunnel-config.yaml (new default config name)
deployment:
  runtime_mode: "proxy"  # "proxy" | "advanced"
  
smart_discovery:
  enabled: true  # Can be overridden by MAGICTUNNEL_SMART_DISCOVERY
```

### Version 0.3.9 - Enterprise Security UI & Enhanced System Metrics Complete ✅

#### **🎨 Enterprise Security UI Implementation Complete**
- **Complete 5-Phase Security UI**: All enterprise security features now have professional web interfaces
  - **Phase 1**: Security navigation integration and API layer ✅
  - **Phase 2**: Tool allowlisting UI with rule management ✅
  - **Phase 3**: RBAC management UI with role hierarchy ✅
  - **Phase 4**: Audit logging UI with search and monitoring ✅
  - **Phase 5**: Request sanitization UI with policy management ✅
- **Security Management Pages**: Complete implementation in `/frontend/src/routes/security/`
- **Enterprise-Grade Interface**: Professional UI components for allowlisting, RBAC, audit logging, and sanitization

#### **📊 Enhanced System Metrics Implementation Complete**
- **Process-Specific Monitoring**: Real-time tracking of MagicTunnel and supervisor processes
  - **CPU Usage Tracking**: Individual process CPU percentage monitoring
  - **Memory Usage Tracking**: Process-specific memory consumption in MB
  - **Process Status**: Running/stopped status for each service process
- **Backend API Enhancement**: Extended `/dashboard/api/metrics` endpoint with process data
- **Frontend Integration**: Updated TopBar status dropdown and SystemMetricsCard components
- **Synchronized Data**: Shared store ensures consistent metrics across all UI components
- **Real System Detection**: Automatic system memory detection (32GB) replacing hardcoded values

#### **🚀 Modern UI Layout System Complete**
- **Professional Sidebar Navigation**: Collapsible navigation with 4 organized sections (Main, Security, MCP Services, Administration)
- **Advanced TopBar**: Search functionality, notifications system, system status monitoring, user management
- **Responsive Layout Container**: Mobile-friendly design with sidebar collapse and overlay support
- **Intelligent Breadcrumbs**: Route-based navigation with icons and responsive design
- **Enhanced HTML Template**: SEO optimization, accessibility features, cross-browser compatibility
- **Dark Mode Support**: Complete theme system with persistence and smooth transitions

#### **✨ Key Features Delivered**
- **Real-time System Monitoring**: Live CPU, memory, and connection tracking with process-specific details
- **Advanced Search System**: Intelligent page/tool search with live results
- **Notification Management**: Security alerts with severity levels and mark-as-read functionality
- **Mobile Responsive**: Touch-friendly interface with mobile menu overlay
- **Accessibility Compliance**: WCAG 2.1 support, keyboard navigation, screen reader compatibility
- **Component Architecture**: Event-driven communication with state management
- **Production Ready**: Professional enterprise-grade UI framework
- **Enhanced Metrics Display**: Both system totals and service-specific resource monitoring

### Version 0.3.8 - API Cleanup & MCP Architecture Fix Complete ✅

#### **🧹 Sampling Dashboard API Cleanup**
- **12 Unnecessary APIs Removed**: Cleaned up all `/dashboard/api/sampling/*` endpoints that were not required for true MCP protocol-level sampling
- **API Methods Removed**: `get_sampling_status`, `generate_sampling_request`, `list_sampling_tools`, and 8 service management methods
- **Helper Methods Cleaned**: Removed `get_tools_with_sampling`, `tool_has_sampling_enhancement`, `get_tool_sampling_enhancement`
- **Struct Types Removed**: Cleaned up 10+ sampling-related request/response struct types
- **Route Registrations Removed**: Cleaned up all sampling API route registrations
- **Documentation Updated**: Updated `docs/automatic-llm-generation-workflow.md` and `docs/llm-workflow.md` to reflect API changes

#### **🏗️ MCP 2025-06-18 Architecture Fix**
- **Incorrect Server Handlers Removed**: Removed `sampling/createMessage` and `elicitation/create` handlers from `server.rs`
- **Client Architecture Verified**: Confirmed clients (stdio, WebSocket, StreamableHTTP) correctly handle these methods
- **Proper Flow Established**: External MCP servers → Client handles createMessage → Forward via internal methods → Server routing
- **RequestForwarder Architecture**: Verified proper internal forwarding via `forward_sampling_request()` and `forward_elicitation_request()`

#### **🔧 Tool Enhancement Pipeline Fix**
- **Method Renaming**: Renamed `should_use_local_elicitation()` to `should_use_tool_enhancement()` in `src/discovery/enhancement.rs`
- **Logic Fix**: Removed smart discovery dependency - tool enhancement now runs on all enabled tools
- **External Tool Protection**: Simplified external tool logic with proper enabled tool checking
- **Architecture Clarification**: Clear separation between tool enhancement and MCP elicitation services

#### **🚀 Future Enhancement Planning**
- **LLM-Assisted Sampling**: Added comprehensive TODO comments for MagicTunnel-initiated sampling requests
- **Advanced Elicitation**: Added TODO comments for context-aware elicitation beyond parameter validation
- **Proxy-Only Strategy**: Current implementation focuses on proxy functionality with intelligent enhancement planned
- **Documentation Updates**: Updated sampling and elicitation documentation with future enhancement roadmap

### Version 0.3.2 - Advanced MCP Platform with LLM Integration ✅

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
# Runtime Mode Control (v0.3.10)
export MAGICTUNNEL_RUNTIME_MODE=advanced    # proxy | advanced
export MAGICTUNNEL_CONFIG_PATH=./config.yaml # Custom config file path
export MAGICTUNNEL_SMART_DISCOVERY=true     # Enable/disable smart discovery

# Enable debug logging
export RUST_LOG=debug

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

#### **UI and Enterprise Features Status** 
- ✅ **Enterprise Security UI**: Complete implementation with professional interface for all security features
- ✅ **Modern Layout System**: Professional sidebar navigation, advanced topbar, responsive design
- ✅ **Accessibility Framework**: WCAG 2.1 support with keyboard navigation and screen reader compatibility
- ⚠️ **LLM Services UI**: Frontend for sampling, elicitation, and enhancement management (planned)
- ⚠️ **Advanced Dashboards**: Additional enterprise management interfaces (planned)
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