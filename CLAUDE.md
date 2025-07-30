# MagicTunnel - Guide for Claude Code

## Overview

MagicTunnel is an intelligent bridge between MCP (Model Context Protocol) clients and diverse agents/endpoints. It provides a single, smart tool discovery interface that can find the right tool for any request, map parameters, and proxy the call automatically.

**Current Version**: 0.2.49 - **OpenAPI 3.1 Custom GPT Integration Complete** ✅

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

5. **Smart Tool Discovery System** (`src/discovery/`)
   - **THE CORE INNOVATION**: Single intelligent tool that discovers the right tool for any request
   - **Hybrid AI Intelligence**: Combines semantic search, rule-based matching, and LLM analysis
   - **Parameter mapping**: Uses LLM to extract and map parameters from natural language
   - **Confidence scoring**: Provides confidence scores for tool matches
   - **Semantic Search**: Optional embedding-based tool matching for enhanced accuracy

6. **Visibility Management System** (`src/bin/magictunnel-visibility.rs`)
   - **CLI Tool**: Complete tool visibility control
   - **Hidden by Default**: All 83 tools across 15 capability files hidden by default
   - **Smart Discovery Mode**: Clean interface with full functionality through discovery

### Smart Discovery System (Key Innovation)

The system provides **one intelligent tool** (`smart_tool_discovery`) that:
1. Analyzes natural language requests using hybrid AI intelligence
2. Finds the best matching tool using semantic search + rule-based + LLM analysis
3. Maps parameters from natural language to tool schema
4. Proxies the call to the actual tool
5. Returns results with discovery metadata

**Discovery modes:**
- `hybrid` (recommended): Combines semantic search (30%), rule-based (15%), and LLM analysis (55%)
- `rule_based`: Fast keyword matching and pattern analysis
- `semantic`: Embedding-based similarity search
- `llm_based`: AI-powered tool selection with OpenAI/Anthropic/Ollama APIs

## Important Files

### Configuration
- `magictunnel-config.yaml` - Main configuration file
- `config.yaml.template` - Template for configuration with comprehensive documentation
- `capabilities/` - Directory containing capability definitions (YAML format)

### Key Source Files
- `src/discovery/service.rs` - Smart discovery implementation with hybrid AI intelligence
- `src/discovery/semantic.rs` - Semantic search with embedding-based tool matching
- `src/routing/substitution.rs` - Parameter substitution with array indexing
- `src/mcp/server.rs` - MCP protocol implementation
- `src/registry/service.rs` - Capability registry management with visibility support
- `src/bin/magictunnel-visibility.rs` - CLI tool for visibility management
- `src/main.rs` - Application entry point

### Documentation
- `docs/ROUTING_ARCHITECTURE.md` - Detailed architecture documentation with Phase 4 completion status
- `CHANGELOG.md` - Version history and changes (current: 0.2.49)
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

### Version 0.2.49 (Current)
- **OpenAPI 3.1 Custom GPT Integration**: Complete integration support
- **Claude Desktop Compatibility**: Fixed Claude not working issues
- **Sequential Mode**: Fixed sequential mode functionality
- **UI Management**: Added UI management for Prompts and Resources
- **CLI Management**: Enhanced CLI management for Prompts and Resources

### Version <0.2.48
- **Smart Tool Discovery Complete**: Hybrid AI intelligence system
- **Visibility Management System**: Complete implementation with CLI tool
- **Semantic Search**: Embedding-based tool matching
- **Ultimate Smart Discovery Mode**: All 83 tools hidden by default
- **Parameter substitution**: Array indexing support (`{hosts[0]}`)
- **LLM API integration**: OpenAI, Anthropic, Ollama support
- **Batch processing**: Handle large tool catalogs efficiently
- **Enhanced keyword matching**: Better networking tool recognition

### Migration Notes
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

MagicTunnel includes several powerful CLI tools:

### 1. Main Server (`magictunnel`)
```bash
# Start the main MCP server
cargo run --bin magictunnel -- --config config.yaml

# Start with stdio mode for Claude Desktop
cargo run --bin magictunnel -- --stdio --config config.yaml
```

### 2. Visibility Management (`magictunnel-visibility`)
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

### Key Features Complete ✅
- ✅ **Smart Tool Discovery System**: Hybrid AI intelligence
- ✅ **Visibility Management**: Complete implementation with CLI
- ✅ **Semantic Search**: Embedding-based tool matching
- ✅ **External MCP Integration**: Full MCP server proxy support
- ✅ **OpenAPI 3.1 Integration**: Custom GPT support
- ✅ **Parameter Substitution**: Array indexing and complex mapping

This guide should help you understand and work with the MagicTunnel codebase effectively. The Smart Tool Discovery system with hybrid AI intelligence and complete visibility management is the core innovation that makes MagicTunnel unique in the MCP ecosystem.