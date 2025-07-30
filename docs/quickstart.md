# 🚀 Quick Start Guide

Get MagicTunnel running in under 5 minutes!

> **💡 Pro Tip**: Jump to [Section 4: Smart Discovery with Pre-Generated Embeddings](#4-smart-discovery-with-pre-generated-embeddings-recommended) for the fastest, most modern setup with natural language tool discovery!

## Prerequisites

- **Rust** (1.70+): [Install Rust](https://rustup.rs/)
- **Git**: For cloning the repository

### Optional Dependencies

#### For Local Semantic Search (Smart Discovery)
- **Ollama** (recommended for local development):
  ```bash
  # Install Ollama
  curl -fsSL https://ollama.ai/install.sh | sh
  
  # Pull embedding model
  ollama pull nomic-embed-text
  ```

#### For External MCP Servers
- **Python 3.8+** (for Python-based MCP servers):
  ```bash
  # Option 1: Using pip
  pip install mcp-filesystem mcp-git mcp-sqlite
  
  # Option 2: Using uv (recommended - faster)
  uv pip install mcp-filesystem mcp-git mcp-sqlite
  
  # Option 3: Using uvx (for isolated tool execution)
  uvx --from mcp-filesystem mcp-server-filesystem
  uvx --from mcp-git mcp-server-git
  uvx --from mcp-sqlite mcp-server-sqlite
  ```

- **Node.js 18+** (for Node.js-based MCP servers):
  ```bash
  # Install Node.js MCP servers
  npm install -g @modelcontextprotocol/server-filesystem
  npm install -g @modelcontextprotocol/server-git
  npm install -g @modelcontextprotocol/server-sqlite
  ```

## 1. Clone & Build

```bash
# Clone the repository
git clone https://github.com/MagicBeansAI/magictunnel.git
cd magictunnel

# Build the project
cargo build --release
```

## 2. Basic Configuration

Create a configuration file from the template:

```bash
# Copy the template configuration
cp config.yaml.template config.yaml
```

The template includes comprehensive examples and documentation. For a quick start, the default settings work well:

```yaml
# Basic MagicTunnel Configuration
server:
  host: "0.0.0.0"
  port: 3000
  websocket: true
  timeout: 30

# Capability registry (where your API tools are defined)
registry:
  type: "file"
  paths:
    - "./capabilities"
  hot_reload: true

# External MCP server integration (optional)
external_mcp:
  enabled: false
  config_file: "external-mcp-servers.yaml"
  conflict_resolution: "local_first"
  discovery:
    enabled: false
    refresh_interval_minutes: 60

# Authentication (optional - disabled by default)
# auth:
#   type: "api_key"
#   api_keys:
#     - name: "admin"
#       key: "your-secure-api-key-here"

# Logging
logging:
  level: "info"
  format: "text"
```

## 3. Environment Setup (Optional)

MagicTunnel supports environment-specific configurations for different deployment scenarios. This is particularly useful for managing API keys and different settings across development, staging, and production environments.

### Quick Environment Setup

For development with API keys:

```bash
# Copy the environment template
cp .env.example .env

# Edit .env and add your API keys
# OPENAI_API_KEY=sk-your-key-here
# ANTHROPIC_API_KEY=sk-ant-your-key-here
```

### Environment-Specific Configuration

For more advanced setups, you can use environment-specific files:

```bash
# Development environment
cp .env.development .env.local
# Edit .env.local with development API keys

# Production environment  
cp .env.production .env.local
# Edit .env.local with production API keys
```

### Environment File Loading Order

The system loads environment files in this order (later files override earlier ones):

1. `.env` - Base environment file
2. `.env.{environment}` - Environment-specific (e.g., `.env.production`)
3. `.env.local` - Local overrides (git-ignored)
4. System environment variables
5. CLI arguments

### Environment Detection

The system determines the environment using:
- `MAGICTUNNEL_ENV` environment variable (highest precedence)
- `ENV` environment variable
- `NODE_ENV` environment variable
- Defaults to `"development"`

### Run with Specific Environment

```bash
# Run in development mode
MAGICTUNNEL_ENV=development cargo run --release --bin magictunnel

# Run in production mode
MAGICTUNNEL_ENV=production cargo run --release --bin magictunnel
```

## 4. Smart Discovery with Pre-Generated Embeddings (Recommended)

For the best experience and fastest startup, use Smart Discovery with pre-generated embeddings:

### Quick Setup

#### **Recommended Quick Start (Ollama + Development Mode)**
For the fastest development setup with full smart discovery:

```bash
# One-line setup for development with Ollama embeddings
make build-release-semantic && make pregenerate-embeddings-ollama MAGICTUNNEL_ENV=development

# Then start with supervisor (includes web dashboard)
./target/release/magictunnel-supervisor

# Access dashboard at http://localhost:5173/dashboard
```

#### **Alternative Setup Options**

Choose your preferred embedding model:

```bash
# Build with semantic search support
make build-release-semantic

# Option A: Ollama - RECOMMENDED FOR LOCAL DEVELOPMENT (real embeddings)
ollama pull nomic-embed-text          # First time setup
make pregenerate-embeddings-ollama MAGICTUNNEL_ENV=development

# Option B: OpenAI models (requires API key) - RECOMMENDED FOR PRODUCTION  
make pregenerate-embeddings-openai OPENAI_API_KEY=your-openai-key MAGICTUNNEL_ENV=production

# Option C: Fallback models (WARNING: hash-based fallbacks, very limited functionality)
make pregenerate-embeddings-local    # all-MiniLM-L6-v2 hash fallback
make pregenerate-embeddings-hq       # all-mpnet-base-v2 hash fallback

# Run with smart discovery and fast startup
make run-release-semantic OPENAI_API_KEY=your-openai-key  # Only if using OpenAI
# OR (for local models)
./target/release/magictunnel-supervisor  # Recommended with web dashboard
# OR (standalone)
cargo run --bin magictunnel --release -- --config magictunnel-config.yaml
```

### Environment Configuration

Add semantic search environment variables to your `.env` file based on your chosen model:

```bash
# For OpenAI models
OPENAI_API_KEY=sk-your-openai-key-here
MAGICTUNNEL_SEMANTIC_MODEL=openai:text-embedding-3-small

# For Ollama (RECOMMENDED - real semantic embeddings)
OLLAMA_BASE_URL=http://localhost:11434
MAGICTUNNEL_SEMANTIC_MODEL=ollama:nomic-embed-text

# For fallback models (WARNING: hash-based, not real embeddings)
MAGICTUNNEL_SEMANTIC_MODEL=all-MiniLM-L6-v2     # Hash fallback - poor semantic matching
MAGICTUNNEL_SEMANTIC_MODEL=all-mpnet-base-v2    # Hash fallback - poor semantic matching

# For Ollama
OLLAMA_BASE_URL=http://localhost:11434
MAGICTUNNEL_SEMANTIC_MODEL=ollama:nomic-embed-text

# For custom embedding API
EMBEDDING_API_URL=http://your-server:8080
MAGICTUNNEL_SEMANTIC_MODEL=external:api

# Common settings
MAGICTUNNEL_DISABLE_SEMANTIC=false
```

### Model Selection Guide

| Use Case | Recommended Model | Command | Status |
|----------|-------------------|---------|--------|
| **🏆 Local Development** | `ollama:nomic-embed-text` | `make pregenerate-embeddings-ollama MAGICTUNNEL_ENV=development` | ✅ **Real embeddings** |
| **🏆 Production** | `openai:text-embedding-3-small` | `make pregenerate-embeddings-openai MAGICTUNNEL_ENV=production` | ✅ **Real embeddings** |
| **Custom API** | `external:api` | `make pregenerate-embeddings-external` | ✅ **Real embeddings** |
| **Testing Only** | `all-MiniLM-L6-v2` | `make pregenerate-embeddings-local` | ⚠️ **Hash fallback** |
| **Testing Only** | `all-mpnet-base-v2` | `make pregenerate-embeddings-hq` | ⚠️ **Hash fallback** |

### Benefits

- ⚡ **Faster Startup**: Embeddings pre-computed, no runtime delays
- 🧠 **Smart Interface**: Single `smart_tool_discovery` tool for all requests
- 🔍 **Natural Language**: Use plain English instead of tool names
- 🚀 **Production Ready**: Perfect for containers and CI/CD
- 💰 **Cost Control**: Local models have no API costs

### Example Usage

Instead of discovering individual tools, use natural language:

```json
{
  "name": "smart_tool_discovery",
  "arguments": {
    "request": "read the config.yaml file"
  }
}
```

```json
{
  "name": "smart_tool_discovery", 
  "arguments": {
    "request": "make HTTP POST request to https://api.example.com/data with JSON body"
  }
}
```

## 5. Run MagicTunnel

MagicTunnel provides two deployment options:

### Option A: Full Stack with Web Dashboard & Supervisor (Recommended)

For the complete experience with web-based management and monitoring:

```bash
# Start the MagicTunnel Supervisor (manages the main server)
cargo run --release --bin magictunnel-supervisor

# Or run the binary directly
./target/release/magictunnel-supervisor
```

You should see:
```
[INFO] MagicTunnel Supervisor starting on 0.0.0.0:8081
[INFO] Starting main MagicTunnel server...
[INFO] MagicTunnel starting on 0.0.0.0:3000
[INFO] gRPC server starting on 0.0.0.0:4000
[INFO] Web dashboard available at http://localhost:3000/dashboard
[INFO] Loaded 5 capabilities from registry
[INFO] MCP server ready for connections
```

**🎯 Access the Web Dashboard**: Open http://localhost:5173/dashboard in your browser

**Dashboard Features:**
- 📊 **Real-time Monitoring**: Live system status, performance metrics, and uptime tracking
- 🔧 **Tool Management**: Browse, test, and manage all available MCP tools
- 📈 **Tool Analytics**: Track tool usage patterns, execution metrics, and discovery rankings
- 📋 **Configuration Management**: Edit configuration files with validation and backup
- 📝 **Live Logs**: Real-time log viewer with filtering, search, and export
- 🔍 **MCP Testing**: Interactive MCP command testing interface
- ⚙️ **Service Control**: Start, stop, and restart services via web interface

### Option B: Standalone Server (Development/Testing)

For lightweight usage without the web dashboard:

```bash
# Start MagicTunnel directly
cargo run --release --bin magictunnel

# Or run the binary directly
./target/release/magictunnel
```

You should see:
```
[INFO] MagicTunnel starting on 0.0.0.0:3000
[INFO] gRPC server starting on 0.0.0.0:4000
[INFO] Loaded 5 capabilities from registry
[INFO] MCP server ready for connections
```

## 6. Test with Claude Desktop

Add to your Claude Desktop MCP configuration (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "magictunnel": {
      "command": "/path/to/your/magictunnel/target/release/magictunnel",
      "args": [
        "--stdio",
        "--config", "/path/to/your/magictunnel/config.yaml"
      ],
      "env": {
        "RUST_LOG": "info"
      },
      "cwd": "/path/to/your/magictunnel"
    }
  }
}
```

**Note**: Replace `/path/to/your/magictunnel` with the actual path to your MagicTunnel directory.

Restart Claude Desktop and you should see the MagicTunnel tools available!

## 7. Test with Cursor

For Cursor IDE, add to your MCP configuration file (`~/Library/Application Support/Cursor/mcp_servers.json` on macOS):

```json
{
  "mcpServers": {
    "magictunnel": {
      "command": "cargo",
      "args": [
        "run",
        "--release",
        "--bin",
        "magictunnel",
        "--",
        "--stdio"
      ],
      "cwd": "/path/to/your/magictunnel"
    }
  }
}
```

Restart Cursor and your MCP tools will be available in the chat interface.

## 8. Connect to External MCP Servers (Optional)

MagicTunnel can spawn and manage external MCP servers using the same configuration format as Claude Desktop. This allows you to combine tools from multiple MCP servers seamlessly:

### Protocol Compatibility

MagicTunnel also supports connecting to network-based MCP services (HTTP, SSE, WebSocket) that can be configured alongside process-based servers. For detailed information about protocol translation and compatibility, see the [Protocol Compatibility Guide](PROTOCOL_COMPATIBILITY.md).

### Configure External MCP Servers

Create an external MCP server configuration file:

```bash
# Create external MCP configuration
cat > external-mcp-servers.yaml << 'EOF'
# External MCP Servers Configuration (Claude Desktop Compatible)
mcpServers:
  # Node.js MCP server
  filesystem:
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
    env:
      NODE_ENV: production

  # Python MCP server using uv
  git-uv:
    command: uv
    args: ["run", "mcp-server-git", "--repository", "/path/to/your/repo"]
    env:
      PYTHONPATH: "/usr/local/lib/python3.11/site-packages"

  # Python MCP server using uvx (isolated execution)
  git-uvx:
    command: uvx
    args: ["--from", "mcp-git", "mcp-server-git", "--repository", "/path/to/your/repo"]
    env:
      PYTHONPATH: "/usr/local/lib/python3.11/site-packages"

  # Python MCP server using traditional pip install
  sqlite:
    command: python
    args: ["-m", "mcp_sqlite", "/path/to/database.db"]
    env:
      PYTHONPATH: "/usr/local/lib/python3.11/site-packages"

  # Example with Docker container
  postgres:
    command: docker
    args: ["run", "--rm", "-i", "mcp-postgres-server"]
    env:
      DATABASE_URL: "postgresql://user:pass@localhost:5432/db"
EOF
```

Update your main configuration to enable external MCP:

```yaml
# Add to config.yaml
external_mcp:
  enabled: true
  config_file: "external-mcp-servers.yaml"
  capabilities_output_dir: "./capabilities/external-mcp"
  refresh_interval_minutes: 5
  containers:
    runtime: "docker"  # or "podman"
```

### Restart MagicTunnel

```bash
# Restart with external MCP configuration
cargo run --release --bin magictunnel -- --config config.yaml
```

You should see:
```
[INFO] MagicTunnel starting on 0.0.0.0:3000
[INFO] gRPC server starting on 0.0.0.0:4000
[INFO] External MCP discovery enabled
[INFO] Spawned filesystem MCP server (5 tools)
[INFO] Spawned git MCP server (8 tools)
[INFO] Total tools available: 15 (local) + 13 (external) = 28 tools
```

### Verify External MCP Connections

```bash
# Check external MCP status
curl http://localhost:3000/mcp/external/status

# List all available tools (local + external)
curl http://localhost:3000/mcp/tools

# Check specific server status
curl http://localhost:3000/mcp/external/servers
```

## 9. Generate Your First API Tools

### Using the Unified Generator CLI

MagicTunnel includes a unified generator CLI (`mcp-generator`) that can generate capability files from various sources:

```bash
# Initialize a configuration file
cargo run --bin mcp-generator init --output mcp-generator.toml

# Generate tools from an OpenAPI specification (supports OpenAPI 3.0 & Swagger 2.0)
cargo run --bin mcp-generator openapi \
  --spec "https://petstore.swagger.io/v2/swagger.json" \
  --base-url "https://petstore.swagger.io/v2" \
  --prefix "petstore" \
  --output "capabilities/petstore.yaml"

# Generate tools from a GraphQL schema
cargo run --bin mcp-generator graphql \
  --schema "schema.graphql" \
  --endpoint "https://api.github.com/graphql" \
  --prefix "github" \
  --auth-type "bearer" \
  --auth-token "your_token_here" \
  --output "capabilities/github.yaml"

# Generate tools from a gRPC protobuf definition
cargo run --bin mcp-generator grpc \
  --proto "service.proto" \
  --endpoint "localhost:50051" \
  --prefix "grpc" \
  --output "capabilities/grpc.yaml"
```

For more advanced usage, including configuration files and merging capabilities, see the [Unified Generator CLI Documentation](unified_generator_cli.md).

### CLI Tools Overview

MagicTunnel provides several CLI tools for different purposes:

| Tool | Purpose | Example Usage |
|------|---------|---------------|
| `magictunnel-supervisor` | **Supervisor with web dashboard** | `cargo run --bin magictunnel-supervisor` |
| `magictunnel` | Main MCP server (standalone) | `cargo run --bin magictunnel` |
| `magictunnel-visibility` | Tool visibility management | `cargo run --bin magictunnel-visibility -- status` |
| `mcp-generator` | Unified capability generator | `cargo run --bin mcp-generator -- openapi --spec url` |
| `openapi_generator` | OpenAPI/Swagger generator | `cargo run --bin openapi_generator -- --spec url` |
| `graphql_generator` | GraphQL schema generator | `cargo run --bin graphql_generator -- --schema file` |

## 10. Verify Everything Works

### Via Web Dashboard (Recommended)
Open http://localhost:5173/dashboard in your browser and:
- Check the **System Status** page for overall health
- Browse **Tools** to see all available capabilities  
- Use **MCP Testing** to test tool execution
- View **Logs** for real-time system activity

### Via API Endpoints
```bash
# Check MagicTunnel health
curl http://localhost:3000/health

# List available capabilities
curl http://localhost:3000/mcp/tools

# Check external MCP server status (if configured)
curl http://localhost:3000/mcp/external/status

# Access web dashboard API
curl http://localhost:3000/dashboard/api/system/info
```

## 11. Smart Tool Discovery & Visibility Management

MagicTunnel features a **Smart Tool Discovery System** that provides a clean interface by hiding tools by default while keeping them accessible through intelligent discovery. ⭐ **Now enhanced with hybrid AI intelligence for superior tool matching.**

### Check Tool Visibility Status

```bash
# Check current tool visibility status
cargo run --bin magictunnel-visibility -- -c config.yaml status

# Show detailed status with per-file breakdown
cargo run --bin magictunnel-visibility -- -c config.yaml status --detailed
```

You should see output like:
```
Tool Visibility Status
=====================

Overall Summary
===============
Total tools: 83
Visible tools: 0
Hidden tools: 83
Capability files: 15
```

### Manage Tool Visibility

```bash
# Show specific tools when needed
cargo run --bin magictunnel-visibility -- -c config.yaml show-tool git_status

# Hide tools again
cargo run --bin magictunnel-visibility -- -c config.yaml hide-tool git_status

# Show all tools in a capability file
cargo run --bin magictunnel-visibility -- -c config.yaml show-file capabilities/dev/git_tools.yaml

# Hide all tools in a capability file
cargo run --bin magictunnel-visibility -- -c config.yaml hide-file capabilities/dev/git_tools.yaml

# Show all tools globally (for debugging)
cargo run --bin magictunnel-visibility -- -c config.yaml show-all

# Hide all tools globally (Smart Tool Discovery mode)
cargo run --bin magictunnel-visibility -- -c config.yaml hide-all
```

### Configure Visibility Behavior

Add visibility configuration to your `config.yaml`:

```yaml
# Smart Tool Discovery Configuration
visibility:
  hide_individual_tools: true      # Hide individual tools when smart discovery is enabled
  expose_smart_discovery_only: true # Only expose smart_tool_discovery
  allow_override: true             # Allow individual tools to override hidden setting
  default_hidden: false            # Default hidden state for new tools
```

### Benefits of Enhanced Smart Tool Discovery ⭐

1. **Clean Interface**: Users see a clean tool list without clutter
2. **Hybrid AI Intelligence**: 3-layer matching (Semantic 30% + Rule-based 15% + LLM 55%)
3. **Complete Coverage**: All 94 tools evaluated by all enabled methods
4. **Cost Optimized**: Multi-criteria LLM candidate selection (30 tools max)
5. **Enhanced Accuracy**: Sequential execution ensures comprehensive tool evaluation
6. **Full Functionality**: All tools remain accessible through discovery
7. **Easy Management**: CLI-based visibility control
8. **Scalable**: Works with any number of tools

## Configuration Options

### Disable Smart Discovery
If you don't want smart discovery and prefer to see individual tools, add this to your `config.yaml`:

```yaml
smart_discovery:
  enabled: false

visibility:
  hide_individual_tools: false
  expose_smart_discovery_only: false
```

### Configure OpenAI for Semantic Search
To use OpenAI embeddings instead of local Ollama:

```yaml
smart_discovery:
  enabled: true
  semantic_search:
    enabled: true
    model_name: "openai:text-embedding-3-small"

# Set environment variable
export OPENAI_API_KEY="your-openai-api-key"
```

### Configure External MCP Integration
To integrate with external MCP servers, create `external-mcp-servers.yaml`:

```yaml
mcpServers:
  # Node.js MCP server
  filesystem:
    command: "npx"
    args: ["@modelcontextprotocol/server-filesystem", "/path/to/allowed/directory"]
  
  # Python MCP server using uvx (recommended for isolation)
  git-uvx:
    command: "uvx"
    args: ["--from", "mcp-git", "mcp-server-git", "--repository", "/path/to/repo"]
  
  # Python MCP server using uv
  git-uv:
    command: "uv" 
    args: ["run", "mcp-server-git", "--repository", "/path/to/repo"]
```

Then enable in your main config:
```yaml
external_mcp:
  enabled: true
  config_file: "external-mcp-servers.yaml"
```

## Common Issues

### "Connection refused" error
- Make sure MagicTunnel is running on the correct port
- Check firewall settings
- Verify the configuration file is correct

### "No capabilities loaded" warning
- Check that capability files exist in the configured directories
- Verify YAML syntax in capability files
- Check file permissions

### Claude Desktop not showing tools
- Restart Claude Desktop after configuration changes
- Check the MCP configuration file syntax
- Verify MagicTunnel is accessible from Claude Desktop

### External MCP server connection issues
- Verify the MCP server command is available (e.g., `npx`, `uv`, `uvx`, `docker`)
- Check that the MCP server process can be spawned
- Ensure the MCP server supports the MCP protocol version
- Check environment variables and working directory settings
- For `uvx`: Ensure the package is available on PyPI and the tool name is correct
- For `uv`: Ensure the virtual environment or project has the required dependencies

### Tool visibility issues
- **Tools not appearing**: Check if tools are hidden with `cargo run --bin magictunnel-visibility -- status`
- **Smart discovery not working**: Verify that tools are properly hidden and discovery is enabled
- **CLI tool errors**: Ensure the configuration file path is correct and accessible
- **Permission issues**: Check file permissions for capability files when using visibility management

## Example Use Cases

- **API Integration**: Connect Claude to REST APIs, GraphQL services
- **Workflow Automation**: Chain multiple API calls together
- **Data Access**: Query databases, search services, file systems
- **Notifications**: Send messages via Slack, email, webhooks
- **Development Tools**: Git operations, CI/CD triggers, deployment

## Next Steps

- [🖥️ Web Dashboard Guide](web-dashboard.md) - Complete web interface documentation
- [🔧 Supervisor Guide](supervisor.md) - Process management and monitoring system
- [🔧 Configuration Guide](config.md) - Configure MagicTunnel for your needs
- [🛠️ Adding Tools](tools.md) - Learn all the ways to add tools
- [🧠 Smart Discovery](smart-discovery.md) - Learn about intelligent tool discovery
- [🌐 Protocol Compatibility](PROTOCOL_COMPATIBILITY.md) - Network MCP protocol translation
- [📋 Frontend Development](frontend_todo.md) - Frontend implementation status and roadmap
- [🔢 Version Management](VERSION_MANAGEMENT.md) - Development workflow and versioning
- [🚀 Deployment](deploy.md) - Deploy to production

---

**Need help?** Open an issue or start a discussion on GitHub!