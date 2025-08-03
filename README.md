# MagicTunnel

**Smart MCP Proxy** - One intelligent tool that discovers the right tool for any request. Now with **Complete MCP 2025-06-18 Smart Discovery Integration** featuring server-side LLM request generation, external MCP protection, and enterprise-grade enhancement pipeline.

![MagicTunnel](docs/images/magictunnel-1.png)

## The Problem
MCP clients get overwhelmed with 50+ tools. Users can't find the right tool for their task.

## The Solution  
MagicTunnel provides **one smart tool** that:
1. **🧠 Analyzes** your natural language request with hybrid AI intelligence
2. **🔍 Discovers** the best tool using pre-generated enhanced descriptions  
3. **🔧 Maps** parameters automatically with LLM-powered elicitation
4. **⚡ Executes** with sub-second response times and graceful degradation
5. **🛡️ Protects** external MCP tools while respecting their original capabilities

## 🎉 New: Enterprise-Grade Smart Discovery System

**✅ MCP 2025-06-18 Compliant** with complete sampling/elicitation integration:

- **🧠 Server-side LLM Request Generation**: OpenAI, Anthropic, and Ollama integration for enhanced tool descriptions
- **🔄 Event-driven Enhancement Pipeline**: Real-time tool enhancement with pre-generation at startup
- **🛡️ External MCP Protection**: Automatic detection and capability inheritance from external MCP servers
- **⚡ Performance Optimized**: Pre-generated enhancements maintain sub-second response times
- **🔧 CLI Management**: Complete visibility management with MCP capability override warnings
- **📊 Version Management**: Automatic capability file versioning with rollback support
- **⚠️ Graceful Degradation**: 100% reliability with fallback to base descriptions

## Quick Start

### Full Stack Setup (Recommended)
```bash
# Clone and build
git clone https://github.com/your-org/magictunnel.git
cd magictunnel

# Quick setup with smart discovery (Ollama + development mode)
make build-release-semantic && make pregenerate-embeddings-ollama MAGICTUNNEL_ENV=development

# Run MagicTunnel with Web Dashboard & Supervisor
./magictunnel-supervisor

# Access Web Dashboard
open http://localhost:5173/dashboard

# Test smart discovery via API
curl -X POST http://localhost:3001/v1/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "smart_tool_discovery", 
    "arguments": {"request": "ping google.com"}
  }'
```

### Lightweight Setup (MCP Server Only)
```bash
# Run standalone MCP server (no web dashboard)
./magictunnel
```

### Setup with Smart Discovery (Recommended)
For the best experience with local semantic searc (Requires Ollama embedding model):

```bash
# Install Ollama (optional - for local semantic search)
curl -fsSL https://ollama.ai/install.sh | sh
ollama pull nomic-embed-text

# Build with semantic search support
make build-release-semantic

# Pre-generate embeddings for faster startup
make pregenerate-embeddings-ollama

# Run with smart discovery
make run-release-ollama
```

📚 **[Complete Setup Guide](docs/quickstart.md)** - Detailed 5-minute tutorial with web dashboard and all options

## Example Usage

Instead of knowing which specific tool to use:
```json
// ❌ Before: Need to know exact tool names
{"name": "network_ping", "arguments": {"host": "google.com"}}
{"name": "filesystem_read", "arguments": {"path": "/etc/hosts"}}
{"name": "database_query", "arguments": {"sql": "SELECT * FROM users"}}
```

Just describe what you want:
```json
// ✅ After: Natural language requests
{"name": "smart_tool_discovery", "arguments": {"request": "ping google.com"}}
{"name": "smart_tool_discovery", "arguments": {"request": "read the hosts file"}}  
{"name": "smart_tool_discovery", "arguments": {"request": "get all users from database"}}
```

## Configuration

Create `magictunnel-config.yaml`:
```yaml
server:
  host: "127.0.0.1"
  port: 8080

registry:
  paths: ["./capabilities"]

smart_discovery:
  enabled: true
  tool_selection_mode: "rule_based"  # or "llm_based"
```

## Web Dashboard

MagicTunnel includes a comprehensive web dashboard for management and monitoring:

### Access Dashboard
```bash
# Start with supervisor (includes web dashboard)
./target/release/magictunnel-supervisor

# Open in browser
open http://localhost:5173/dashboard
```

### Dashboard Features
- 📊 **Real-time Monitoring**: System status, performance metrics, and uptime tracking
- 🔧 **Tool Management**: Browse, test, and manage all available MCP tools
- 📈 **Tool Analytics**: Track tool usage patterns, execution metrics, and discovery rankings
- 📋 **Configuration Management**: Edit configuration files with validation and backup
- 📝 **Live Logs**: Real-time log viewer with filtering, search, and export
- 🔍 **MCP Testing**: Interactive JSON-RPC command testing interface
- ⚙️ **Service Control**: Start, stop, and restart services via web interface

## Add Your Tools

### Manual Tool Creation

Create `capabilities/my-tools.yaml`:
```yaml
tools:
  - name: "ping"
    description: "Test network connectivity to a host"
    input_schema:
      type: object
      properties:
        host:
          type: string
          description: "Hostname or IP address to ping"
    routing:
      type: "command"
      command: "ping"
      args: ["-c", "4", "{host}"]
```

### Generate Tools from APIs

Automatically generate tools from your existing APIs (always produces Enhanced MCP 2025-06-18 format):

```bash
# Generate Enhanced MCP 2025-06-18 format tools
./target/release/openapi-generator \
  --spec https://api.example.com/openapi.json \
  --output capabilities/api-tools.yaml \
  --base-url https://api.example.com \
  --auth-type bearer --auth-token $API_TOKEN

# Generate from gRPC services  
./target/release/grpc-generator \
  --proto service.proto \
  --output capabilities/grpc-tools.yaml \
  --endpoint localhost:50051

# Generate from GraphQL schemas
./target/release/graphql-generator \
  --schema schema.graphql \
  --endpoint https://api.example.com/graphql \
  --output capabilities/graphql-tools.yaml

# Unified CLI for all formats (always enhanced)
./target/release/magictunnel-cli openapi \
  --spec openapi.json \
  --base-url https://api.example.com \
  --output tools.yaml
```

📖 **[Complete API Generation Guide](docs/tools.md#generating-tools-from-apis)** - Detailed CLI documentation with all options and examples

### CLI Management Tools

MagicTunnel includes powerful CLI tools for managing tool visibility and MCP capabilities:

```bash
# Tool Visibility Management
./target/release/magictunnel-visibility status --detailed
./target/release/magictunnel-visibility hide-tool tool_name
./target/release/magictunnel-visibility show-all --confirm

# MCP Capability Override Warnings (NEW!)
./target/release/magictunnel-visibility show-mcp-warnings --detailed

# Example output:
# MCP Capability Override Warnings
# ================================
# Total external MCP tools: 15
# Tools with original capabilities: 8
# Capability override warnings: 3
# 
# File: capabilities/external-server.yaml
#   🔗 weather_tool (external_mcp)
#       ✅ Has original sampling capabilities
#       ❌ No original MCP 2025-06-18 capabilities detected
#   ⚠️  weather_tool: Tool has original sampling capabilities but local enhancement is enabled
```

🔧 **[Complete CLI Reference](docs/cli.md)** - All CLI tools and management commands

## Features

- ✨ **Enhanced MCP 2025-06-18 Format**: Latest MCP specification with AI-enhanced discovery, security sandboxing, and enterprise monitoring
- ✅ **Smart Discovery**: AI-powered tool selection with natural language interface  
- 🖥️ **Web Dashboard**: Real-time monitoring, tool management, and configuration
- 🔧 **Supervisor Architecture**: Process management with automatic restart and health monitoring
- ✅ **MCP Compatible**: Works with Claude, GPT-4, any MCP client
- 🌐 **Protocol Gateway**: HTTP, SSE, WebSocket, Streamable HTTP protocol translation for network MCP services
- 🔄 **Dual Transport Support**: HTTP+SSE (deprecated) and Streamable HTTP (MCP 2025-06-18) with graceful migration
- ✅ **Easy Setup**: Single binary, YAML configuration  
- ✅ **Extensible**: Add tools without coding
- 🔒 **Enterprise Security**: Comprehensive security and access control system
  - **Security Sandboxing**: 5-level classification (Safe/Restricted/Privileged/Dangerous/Blocked)
  - **Tool Allowlisting**: Explicit control over tool, resource, and prompt access
  - **RBAC**: Role-based access control with hierarchical permissions
  - **Audit Logging**: Complete audit trail for compliance and monitoring
  - **Request Sanitization**: Content filtering and secret detection
  - **Security Policies**: Organization-wide policy engine with flexible conditions

## MCP 2025-06-18 Specification Compliance ✅

MagicTunnel is **fully compliant** with the latest MCP 2025-06-18 specification:

### 🔐 **Authentication & Security**
- **✅ OAuth 2.1 Framework**: Complete upgrade with PKCE support
- **✅ Resource Indicators (RFC 8707)**: Enhanced token security with resource scoping
- **✅ Enhanced Security Model**: MCP-specific consent flows and capability permissions

### 🌐 **Transport Layer**
- **✅ Dual Transport Support**: 
  - `/mcp/stream` - HTTP+SSE (deprecated, maintained for compatibility)
  - `/mcp/streamable` - Streamable HTTP (preferred, MCP 2025-06-18)
- **✅ Enhanced Batching**: JSON-RPC batch processing with NDJSON streaming
- **✅ Graceful Migration**: Automatic upgrade recommendations with deprecation headers

### 🛡️ **Advanced Capabilities**
- **✅ Sampling Capabilities**: Server-initiated LLM interactions
- **✅ Elicitation Features**: Structured user data requests
- **✅ Roots Capability**: Filesystem boundary management
- **✅ Tool Approval Workflows**: Granular permission controls
- **✅ Enhanced Cancellation Support**: Token-based request cancellation with graceful cleanup
- **✅ Granular Progress Tracking**: Real-time monitoring of long-running operations with sub-operations
- **✅ Runtime Tool Validation**: Security sandboxing with classification-based policies

**Migration Path**: Existing clients continue working with HTTP+SSE while new clients can leverage the enhanced Streamable HTTP transport for better performance and features.

## Documentation

### Core Documentation
- [🚀 Quick Start](docs/quickstart.md) - 5-minute setup guide with all options
- [🖥️ Web Dashboard](docs/web-dashboard.md) - Complete web interface guide
- [🔧 Supervisor System](docs/supervisor.md) - Process management and monitoring
- [🧠 Smart Discovery](docs/smart-discovery.md) - Intelligent tool discovery
- [🌐 Protocol Compatibility](docs/PROTOCOL_COMPATIBILITY.md) - Network MCP protocol translation

### Advanced Documentation
- [📖 Full Guide](docs/guide.md) - Complete documentation
- [🔧 Configuration](docs/config.md) - Configuration options
- [🛠️ Adding Tools](docs/tools.md) - How to add your own tools
- [🏗️ Architecture](docs/architecture.md) - Technical architecture
- [🔌 API Reference](docs/api.md) - Complete API documentation
- [🧪 Testing](docs/testing.md) - Testing and validation
- [🚀 Deployment](docs/deploy.md) - Production deployment
- [🔒 Security](docs/security.md) - Enterprise security features and configuration
- [🔒 Security CLI](docs/security-cli.md) - Complete CLI reference for security management
- [📊 Observability](docs/MCP_OBSERVABILITY_ARCHITECTURE.md) - Metrics and monitoring

### MCP 2025-06-18 Compliance Documentation
- [⚡ Cancellation System](docs/mcp-cancellation.md) - Enhanced request cancellation with token management
- [📊 Progress Tracking](docs/mcp-progress.md) - Granular progress monitoring for long-running operations  
- [🛡️ Tool Validation](docs/mcp-tool-validation.md) - Runtime security sandboxing and validation
- [🔐 OAuth 2.1 & Security](docs/mcp-security.md) - Authentication and resource indicators
- [🌐 Transport Layer](docs/mcp-transport.md) - Streamable HTTP and enhanced batching

### Development Documentation
- [📋 Frontend Development](docs/frontend_todo.md) - Frontend implementation roadmap
- [🔢 Version Management](docs/VERSION_MANAGEMENT.md) - Development workflow and versioning

[📚 View All Documentation](docs/)

## License

MIT License - see [LICENSE](LICENSE) for details.

![MagicTunnel Dashboards](docs/images/magictunnel-3.png)
![MagicTunnel Dashboards](docs/images/magictunnel-4.png)
![MagicTunnel Dashboards](docs/images/magictunnel-5.png)
![MagicTunnel Dashboards](docs/images/magictunnel-6.png)
![MagicTunnel Dashboards](docs/images/magictunnel-9.png)
![MagicTunnel Dashboards](docs/images/magictunnel-11.png)
![MagicTunnel Dashboards](docs/images/magictunnel-12.png)
![MagicTunnel Dashboards](docs/images/magictunnel-13.png)
![MagicTunnel Dashboards](docs/images/magictunnel-15.png)
![MagicTunnel Dashboards](docs/images/magictunnel-16.png)

[More Images](docs/images/)