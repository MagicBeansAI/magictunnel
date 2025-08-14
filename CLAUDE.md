# MagicTunnel - Guide for Claude Code

## Overview

MagicTunnel is an intelligent bridge between MCP (Model Context Protocol) clients and diverse agents/endpoints. It provides a single, smart tool discovery interface that can find the right tool for any request, map parameters, and proxy the call automatically.

**Current Version**: 0.3.13 - **OAuth 2.1 FUNCTIONALLY COMPLETE & PRODUCTION-READY** ✅

## Quick Start

### Build and Run
```bash
# Build the project
make build-release-ollama && make pregenerate-embeddings-ollama

# Check for errors
cargo check

# Run (Advanced Mode with dashboard)
./magictunnel-supervisor

# Proxy mode only
export MAGICTUNNEL_RUNTIME_MODE=proxy && ./magictunnel-supervisor

# Ports: MagicTunnel (3001), Frontend (5173)

# Test service
curl -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"name": "smart_tool_discovery", "arguments": {"request": "ping google.com"}}'
```

### Development Commands
```bash
# Test, check, debug, cleanup
cargo test
cargo check
RUST_LOG=debug ./magictunnel-supervisor
pkill -f magictunnel

# Code quality
cargo clippy
cargo fmt

# Visibility management
cargo run --bin magictunnel-visibility -- -c config.yaml status
```

## Architecture

**Smart Tool Discovery and Proxy** system that reduces N tools to 1 intelligent proxy tool. Solves message limit problems in MCP systems with 50+ tools.

### Core Components

1. **MCP Server Interface** (`src/mcp/server.rs`) - Protocol implementation, stdio/HTTP modes
2. **Capability Registry** (`src/registry/`) - Tool definitions, aggregation, visibility management
3. **Agent Router** (`src/routing/`) - Tool routing, conflict resolution, parameter substitution
4. **External MCP Integration** (`src/mcp/external_*`) - Process management, bidirectional communication
5. **Smart Tool Discovery** (`src/discovery/`) - **CORE INNOVATION**: Single intelligent tool with:
   - Hybrid AI Intelligence (semantic + rule-based + LLM)
   - MCP 2025-06-18 enhanced descriptions and metadata
   - Parameter mapping with elicitation validation
   - Confidence scoring and semantic search
6. **Visibility Management** (`src/bin/magictunnel-visibility.rs`) - CLI tool control, all 83 tools hidden by default

### Multi-Mode Architecture

**Two-tier service architecture** separating core MCP functionality from enterprise features:

#### **Core/Proxy Services** (Both modes):
- MCP Server with authentication (API keys, OAuth, JWT)
- Registry Service for tool management
- Smart Discovery Service for intelligent routing
- Core LLM Services (sampling, elicitation, enhancement)
- Web Dashboard
- MCP Authentication middleware

#### **Advanced Services** (Enterprise only):
- **Enterprise Security Suite**: Tool allowlisting, RBAC, request sanitization, audit logging, security policies, emergency lockdown
- **Future**: MagicTunnel Authentication (separate from MCP protocol auth)

### Smart Discovery System (Core Innovation)

**One intelligent tool** (`smart_tool_discovery`) that:
1. Analyzes natural language requests with hybrid AI
2. Finds best tool using enhanced descriptions
3. Maps parameters with elicitation validation
4. Proxies call to actual tool
5. Returns results with metadata

**MCP 2025-06-18 Features:**
- Enhanced descriptions for better semantic matching
- Rich metadata (keywords, categories, use cases)
- Smart fallback when enhancement unavailable
- Sub-second response times with caching

**Discovery Modes:**
- `hybrid`: Semantic (30%) + rule-based (15%) + LLM (55%)
- `rule_based`: Fast keyword/pattern matching
- `semantic`: Embedding-based similarity
- `llm_based`: AI-powered selection (OpenAI/Anthropic/Ollama)

## Important Files

### Configuration
- `magictunnel-config.yaml` - Main config
- `config.yaml.template` - Config template
- `capabilities/` - Tool definitions (YAML)

### Key Sources
- `src/discovery/service.rs` - Smart discovery with hybrid AI
- `src/discovery/semantic.rs` - Semantic search
- `src/routing/substitution.rs` - Parameter substitution
- `src/mcp/clients/` - **Modern MCP 2025-06-18 clients** ✅
  - `http_client.rs`, `websocket_client.rs`, `sse_client.rs`, `streamable_http_client.rs`
- `src/mcp/server.rs` - MCP protocol
- `src/mcp/external_*.rs` - External MCP management
- `src/registry/service.rs` - Registry with visibility
- `src/bin/magictunnel-visibility.rs` - Visibility CLI
- `src/main.rs` - Entry point

### Documentation
- `docs/ROUTING_ARCHITECTURE.md`, `docs/BIDIRECTIONAL_COMMUNICATION_FLOW.md`
- `CHANGELOG.md`, `README.md`, `how_to_run.md`

## Development Patterns

### Service Usage

**Core/Proxy Services** (always available): MCP auth, LLM services, smart discovery, web dashboard
**Advanced Services** (enterprise only): Security suite, analytics, policies

### Adding Tools
1. Create YAML in `capabilities/`
2. Auto-discovered in registry
3. Smart discovery handles routing
4. Use `hidden: true` to hide from main list

### Service Guidelines

**Core Services**: Implement in `src/mcp/`, `src/registry/`, `src/discovery/`
**Advanced Services**: Implement in `src/security/`, check advanced mode, graceful degradation

### OAuth 2.1 Session Persistence

**Complete enterprise authentication** with session persistence:

```rust
use crate::auth::{UserContext, AuthResolver, MultiLevelAuthConfig};

let user_context = UserContext::new()?;
let auth_resolver = AuthResolver::with_user_context(config, user_context)?;

// Session file management
if let Some(session_file) = auth_resolver.get_session_file_path("oauth_tokens.json") {
    // Cross-platform secure storage
}
```

**Features**: Cross-platform (Keychain/CredentialManager/SecretService), secure storage (`~/.magictunnel/sessions/`), hostname isolation, graceful fallback

### Tool Visibility Management
```bash
# Status, individual tools, files, global
cargo run --bin magictunnel-visibility -- -c config.yaml status
cargo run --bin magictunnel-visibility -- -c config.yaml hide-tool/show-tool tool_name
cargo run --bin magictunnel-visibility -- -c config.yaml hide-file/show-file capabilities/file.yaml
cargo run --bin magictunnel-visibility -- -c config.yaml hide-all/show-all
```

### Debugging
```bash
# Debug logging
RUST_LOG=magictunnel::discovery=debug cargo run

# Test discovery
curl -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"name": "smart_tool_discovery", "arguments": {"request": "ping google.com"}}'
```

### Configuration
- Smart discovery in `smart_discovery` section
- Modes: `hybrid`, `rule_based`, `semantic`, `llm_based`
- Providers: OpenAI, Anthropic, Ollama
- Visibility with `default_hidden`

### Environment Variables
```bash
export MAGICTUNNEL_RUNTIME_MODE=proxy  # proxy | advanced
export MAGICTUNNEL_CONFIG_PATH=./config.yaml
export MAGICTUNNEL_SMART_DISCOVERY=true
./target/release/magictunnel
```

### Default Configuration
- **Default**: `magictunnel-config.yaml` (replaces `config.yaml`)
- **Auto-detection**: Uses `magictunnel-config.yaml` if present
- **Built-in defaults**: Proxy mode when no config
- **Priority**: Environment > config file

## Startup Flow

### Startup Sequence

1. **Config Resolution**: Load config, apply env overrides, determine mode
2. **Service Loading**: Conditional container creation based on mode
3. **Service Strategy**: 
   - Proxy: Core services only
   - Advanced: Core + Enterprise features
4. **API Integration**: Mode detection, config validation, health monitoring
5. **Frontend**: Mode-aware UI with progressive enhancement

### Service Architecture

**ServiceLoader** (`src/services/service_loader.rs`): Entry point, mode detection, validation

**Containers**:
- **ProxyServices**: MCP Server, Registry, Discovery, Enhancement, Dashboard
- **AdvancedServices**: Security Suite (RBAC, Audit, Sanitization, Policies)
- **ServiceContainer**: Mode-aware wrapper

### Frontend Mode Awareness

**Backend**: `src/web/mode_api.rs` - UI config, navigation, status indicators
**Frontend**: `mode.ts` store - mode detection, auto-refresh, progressive enhancement
**Components**: ModeAwareLayout, TopBar, Sidebar with advanced feature filtering

**UI Filtering**: Navigation, status indicators, user menu, notifications, search results filtered by mode

## Recent Changes

### Version 0.3.13 (Current) - OAuth 2.1 FUNCTIONALLY COMPLETE & PRODUCTION-READY ✅

#### **🎉 CRITICAL BREAKTHROUGH: Phase 6 MCP Protocol Integration** ✅
- **AuthenticationContext System**: Authentication flows through entire MCP pipeline to external API calls
- **Tool Execution Integration**: OAuth tokens now available in tools that call GitHub, Google Drive, etc.
- **Session Management**: Complete cross-platform session persistence with automatic recovery
- **Remote Session Isolation**: Mathematical impossibility of cross-deployment session access
- **Production-Ready**: **13,034+ lines** of enterprise-grade OAuth 2.1 code across all 6 phases

#### **Enterprise Authentication NOW FULLY FUNCTIONAL** ✅
- **4 Authentication Methods**: OAuth 2.1, Device Code Flow, API Keys, Service Accounts
- **MCP Protocol Integration**: Authentication context preserved throughout request lifecycle  
- **Multi-Platform Support**: Native credential storage (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- **Background Token Management**: Automatic refresh, rotation, lifecycle management
- **Enterprise Security**: Comprehensive validation, audit logging, secure storage
- **Remote Session Recovery**: Health monitoring and automatic session recovery across deployments

#### **From Architectural to Functional Completeness**:
- **Previous**: OAuth backend complete but authentication context lost before tool execution
- **Current**: **OAuth tokens flow through MCP protocol to external API calls in tools**
- **Impact**: Tools can now authenticate with external services (GitHub API, Google Drive, etc.)
- **Status**: **FUNCTIONALLY COMPLETE & PRODUCTION-READY**

### Version 0.3.12 - OAuth 2.1 Foundation & UI Enhancement ✅

#### **OAuth 2.1 Foundation (Phases 1-5)** ✅  
- **Phase 1 & 2**: Multi-level auth + session persistence (6,139+ lines)
- **Cross-Platform**: User context (macOS/Windows/Linux) 
- **Token Storage**: Native secure storage + filesystem fallback
- **Session Recovery**: Automatic restoration on restarts
- **Background Refresh**: Intelligent token lifecycle

#### **UI Improvements** ✅
- **Unified Status Banner**: Clean, minimal status system
- **Dashboard Enhancement**: System controls and better hierarchy

### Version 0.3.10 - Multi-Mode Architecture ✅

#### **Architecture Complete**
- **Config-Driven**: All behavior via config + environment variables
- **Environment Integration**: RUNTIME_MODE, CONFIG_PATH, SMART_DISCOVERY
- **Default Resolution**: Auto-detection of magictunnel-config.yaml
- **Startup Logging**: Config resolution, validation, feature status
- **Conditional Loading**: ProxyServices vs AdvancedServices
- **Frontend Awareness**: Mode detection API, progressive enhancement

#### **Runtime Modes**
- **Proxy (Default)**: Zero-config, minimal, fast startup
- **Advanced**: Full-featured enterprise with security
- **Priority**: Environment > config file
- **Smart Loading**: Only required services per mode

#### **Config Examples**
```bash
export MAGICTUNNEL_RUNTIME_MODE=advanced
export MAGICTUNNEL_CONFIG_PATH=./my-config.yaml
./magictunnel
```

```yaml
deployment:
  runtime_mode: "proxy"
smart_discovery:
  enabled: true
```

### Version 0.3.9 - Enterprise Security UI & Metrics ✅

#### **Security UI Complete**
- **5-Phase Implementation**: Navigation, allowlisting, RBAC, audit logging, sanitization
- **Professional Interface**: Complete `/frontend/src/routes/security/`
- **Enterprise Components**: Full management UI suite

#### **Enhanced Metrics**
- **Process Monitoring**: Real-time CPU/memory tracking
- **Individual Processes**: MagicTunnel + supervisor status
- **Backend Enhancement**: Extended `/dashboard/api/metrics`
- **Frontend Integration**: TopBar + SystemMetricsCard sync
- **Real Detection**: Automatic system memory (32GB)

#### **Modern UI System**
- **Professional Navigation**: 4-section collapsible sidebar
- **Advanced TopBar**: Search, notifications, status, user management
- **Responsive Design**: Mobile-friendly with overlay
- **Accessibility**: WCAG 2.1, keyboard navigation, screen readers
- **Dark Mode**: Complete theme system

#### **Key Features**
- Real-time monitoring, search system, notification management
- Mobile responsive, accessibility compliance
- Event-driven architecture, production ready

### Version 0.3.8 - API Cleanup & Architecture Fix ✅

#### **API Cleanup**
- **12 APIs Removed**: Cleaned `/dashboard/api/sampling/*` endpoints
- **Methods Cleaned**: Removed status, request generation, tool listing
- **Structs Removed**: 10+ sampling request/response types
- **Documentation Updated**: Workflow docs updated

#### **MCP Architecture Fix**
- **Server Handlers Removed**: Incorrect `sampling/createMessage` and `elicitation/create`
- **Client Architecture**: Verified proper stdio/WebSocket/StreamableHTTP handling
- **Flow Established**: External MCP → Client → Internal forwarding → Server
- **RequestForwarder**: Proper internal routing verified

#### **Enhancement Pipeline**
- **Method Renamed**: `should_use_local_elicitation()` → `should_use_tool_enhancement()`
- **Logic Fixed**: Removed smart discovery dependency
- **External Protection**: Simplified tool logic
- **Clear Separation**: Tool enhancement vs MCP elicitation services

#### **Future Planning**
- LLM-assisted sampling, advanced elicitation
- Proxy-focused strategy with intelligent enhancement

### Version 0.3.2 - Advanced MCP Platform ✅

#### **MCP 2025-06-18 Backend Complete**
- **Full Implementation**: Latest MCP spec with sampling/elicitation
- **OAuth 2.1**: Backend auth with PKCE and Resource Indicators
- **Transport**: HTTP+SSE (deprecated), Streamable HTTP (preferred)
- **Security**: MCP consent flows, capability validation
- **Streaming**: NDJSON, batching, session management

#### **LLM Generation Workflow**
- **Services**: AI-powered enhancement (OpenAI/Anthropic/Ollama)
- **Pipeline**: Coordinated sampling + elicitation
- **CLI**: `magictunnel-llm` management tool
- **Protection**: External MCP content detection
- **Performance**: Multi-level caching, rate limiting

#### **Backend APIs Complete**
- **25+ Endpoints**: Resource, enhancement, prompt, sampling, elicitation
- **Provider Management**: Config, testing, health monitoring
- **Analytics**: Comprehensive metrics and statistics
- **Batch Processing**: Enhanced operations

#### **Security & Tools**
- **Security CLI**: `magictunnel-security` policy management
- **Auth Framework**: OAuth 2.1, API keys, audit logging
- **CLI Suite**: Enterprise management tools
- **OpenAPI 3.1**: Custom GPT support
- **Claude Desktop**: Fixed compatibility issues

### Version 0.3.6 - Modern Architecture Migration ✅
- **Legacy Client Removal**: Migrated from monolithic `client.rs` to modular `clients/`
- **Modern Architecture**: 4 specialized implementations (HTTP, WebSocket, SSE, StreamableHTTP)
- **Test Migration**: Configuration validation replacing routing calls
- **Code Reduction**: Removed ~2,700 lines deprecated code
- **Production Ready**: Clean compilation, MCP 2025-06-18 compliance

### Version 0.3.4 - Test Infrastructure ✅
- **Complete Test Coverage**: 60+ functions across 6 suites
- **API Testing**: Elicitation, sampling, resource, prompt, discovery, integration
- **Enterprise Framework**: Realistic environments, comprehensive validation
- **Quality Assurance**: Production deployment ready

### Version 0.2.x - Foundation
- **Smart Discovery**: Hybrid AI intelligence
- **Visibility Management**: CLI tool, 83 tools hidden by default
- **Semantic Search**: Embedding-based matching
- **Parameter Substitution**: Array indexing (`{hosts[0]}`)
- **LLM Integration**: OpenAI/Anthropic/Ollama
- **Batch Processing**: Large catalog handling

### Migration Notes
- **v0.3.6**: Legacy `client.rs` → modern modular `clients/`
- External MCP integration replaced remote/local modules
- Smart discovery replaced individual tool exposure
- Unified config structure
- All tools hidden by default with smart discovery
- CLI visibility management

## Common Issues

### Build
- Rust 1.70+, check dependencies, `cargo clean` for cache issues

### Runtime
- **Low confidence**: Check hybrid AI in `src/discovery/service.rs`
- **Semantic search**: Verify OpenAI API key, embedding generation
- **Parameter errors**: Check array syntax in substitution.rs
- **External MCP**: File permissions, working directory
- **Visibility**: Use `magictunnel-visibility` CLI
- **Transport**: Use `/mcp/streamable` (preferred) or `/mcp/sse` (deprecated)

### Git
- Binary tracked: `git rm --cached magictunnel`

## Testing

```bash
cargo test                          # All tests
cargo test discovery                # Specific module
cargo test --test integration       # Integration tests
cargo test smart_discovery          # Smart discovery
cargo test visibility               # Visibility management
cargo test semantic                 # Semantic search
```

## CLI Tools

### 1. Main Server
```bash
cargo run --bin magictunnel -- --config config.yaml      # MCP server
cargo run --bin magictunnel -- --stdio --config config.yaml  # Claude Desktop
```

### 2. LLM Management 🆕
```bash
cargo run --bin magictunnel-llm -- bulk health-check                    # Health check
cargo run --bin magictunnel-llm -- sampling generate --tool example_tool # Enhanced descriptions
cargo run --bin magictunnel-llm -- elicitation generate --tool example_tool # Metadata extraction
cargo run --bin magictunnel-llm -- enhancements regenerate --batch-size 5   # Full pipeline
cargo run --bin magictunnel-llm -- prompts generate --tool example_tool     # Prompt generation
cargo run --bin magictunnel-llm -- providers test --all                     # Provider testing
```

### 3. Security Management 🆕
```bash
cargo run --bin magictunnel-security -- policies list                    # Policy management
cargo run --bin magictunnel-security -- allowlist add-tool tool_name     # Allowlist management
```

### 4. Visibility Management
```bash
cargo run --bin magictunnel-visibility -- -c config.yaml status          # Status
cargo run --bin magictunnel-visibility -- -c config.yaml hide-tool tool_name  # Individual tools
cargo run --bin magictunnel-visibility -- -c config.yaml hide-file file.yaml  # Entire files
cargo run --bin magictunnel-visibility -- -c config.yaml hide-all/show-all    # Global
```

## Environment Variables

```bash
# Runtime control
export MAGICTUNNEL_RUNTIME_MODE=advanced     # proxy | advanced
export MAGICTUNNEL_CONFIG_PATH=./config.yaml
export MAGICTUNNEL_SMART_DISCOVERY=true
export RUST_LOG=debug

# LLM providers
export OPENAI_API_KEY=your_key
export ANTHROPIC_API_KEY=your_key
export OLLAMA_BASE_URL=http://localhost:11434

# Semantic search
export MAGICTUNNEL_SEMANTIC_ENABLED=true
export MAGICTUNNEL_EMBEDDING_MODEL=text-embedding-3-small
```

## Current Status

### Implementation
- **Tools**: 83 across 15 files (0 visible, all via discovery)
- **Smart Discovery**: Hybrid AI with semantic search
- **CLI Management**: Full visibility control

### Backend Services ✅
- ✅ **MCP 2025-06-18**: Full spec with sampling/elicitation
- ✅ **Bidirectional Communication**: Complete routing (6 ProcessingStrategy variants)
- ✅ **LLM Generation**: AI-powered enhancement (multi-provider)
- ✅ **Management APIs**: 25+ REST endpoints
- ✅ **Security Framework**: Auth, policies, audit logging
- ✅ **Smart Discovery**: Hybrid AI + MCP enhanced metadata
- ✅ **Configuration**: Enhanced YAML format
- ✅ **External Protection**: Automatic detection/preservation
- ✅ **Performance**: Multi-level caching, async processing
- ✅ **CLI Tools**: Complete suite (llm, security, visibility)

### UI & Enterprise Features
- ✅ **Security UI**: Complete professional interface
- ✅ **Modern Layout**: Sidebar navigation, topbar, responsive
- ✅ **Accessibility**: WCAG 2.1, keyboard navigation, screen readers
- ⚠️ **LLM Services UI**: Frontend for enhancement management (planned)
- ⚠️ **Advanced Dashboards**: Enterprise interfaces (planned)
- ⚠️ **Review Workflows**: Content approval interfaces (planned)

## Status Summary

### **Advanced MCP Platform**
Sophisticated MCP platform with:
- **MCP 2025-06-18 compliance** with modern protocol features
- **LLM generation workflow** for intelligent tool enhancement
- **25+ REST endpoints** for comprehensive API management
- **Security framework** with auth, policies, audit logging
- **Advanced CLI tooling** for system management
- **Enhanced configuration** supporting complex deployments
- **Smart discovery** with enhanced metadata integration

### **Development Roadmap**
1. **UI Development**: LLM services, security management frontends
2. **Enterprise Features**: Visual builders, approval workflows, dashboards
3. **Accessibility**: WCAG 2.1 AA compliance
4. **Integration Testing**: End-to-end MCP 2025-06-18 testing

### **Migration Path**
1. **Config Migration**: Enhanced YAML with backward compatibility
2. **Service Integration**: Optional LLM with fallback
3. **CLI Access**: Enhanced management capabilities
4. **Progressive Enhancement**: Backend ready, UI following

### **Performance**
- **83+ tools** with smart discovery
- **Sub-second responses** with multi-level caching
- **Enterprise features** without performance impact
- **Horizontal scaling** with distributed caching

Powerful foundation for MCP workflows with Smart Discovery, LLM Enhancement, Security Framework, and Advanced CLI Tools.