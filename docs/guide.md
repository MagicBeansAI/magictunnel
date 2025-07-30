# MagicTunnel Complete Guide

*This file contains the comprehensive documentation that was previously in the main README.*

## What is MagicTunnel?

MagicTunnel is an intelligent bridge between Model Context Protocol (MCP) clients and various backend tools and services. It solves the "tool discovery problem" by providing a single smart tool that can understand natural language requests and automatically find and execute the appropriate backend tool.

## Core Concept: Smart Tool Discovery

Instead of exposing 50+ individual tools to MCP clients (causing choice paralysis), MagicTunnel exposes **one intelligent tool** called `smart_tool_discovery` that:

1. **Analyzes** natural language requests
2. **Discovers** the best matching tool from available capabilities  
3. **Maps** parameters from natural language to tool schema
4. **Executes** the selected tool with proper parameters
5. **Returns** results with discovery metadata

## Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │───▶│  MagicTunnel    │───▶│  Backend Tools  │
│ (Claude/GPT-4)  │    │                 │    │ (Commands/APIs) │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │                        
                              ▼                        
                       ┌─────────────────┐             
                       │ Tool Registry   │             
                       │ (YAML configs)  │             
                       └─────────────────┘             
```

### Key Components

1. **MCP Server**: Implements MCP protocol for client communication
2. **Smart Discovery Engine**: AI-powered tool selection and parameter mapping
3. **Tool Registry**: YAML-based tool definitions and routing configurations
4. **Backend Router**: Routes tool calls to appropriate handlers (commands, HTTP APIs, etc.)

## Installation & Setup

### Requirements
- Rust 1.70+
- Optional: Docker for containerized deployment

### Build from Source
```bash
git clone https://github.com/your-org/magictunnel.git
cd magictunnel
cargo build --release
```

### Run
```bash
./target/release/magictunnel --config magictunnel-config.yaml
```

## Configuration

### Basic Configuration (`magictunnel-config.yaml`)

```yaml
# Server configuration
server:
  host: "127.0.0.1"
  port: 8080
  
# Tool registry
registry:
  paths: ["./capabilities"]
  hot_reload: true
  
# Smart discovery settings
smart_discovery:
  enabled: true
  tool_selection_mode: "rule_based"  # or "llm_based"
  default_confidence_threshold: 0.5
  
# Optional: LLM-based discovery
llm_tool_selection:
  enabled: false
  provider: "openai"  # openai, anthropic, ollama
  model: "gpt-4"
```

### Tool Definitions

Create tool definitions in YAML files under your configured registry paths:

```yaml
# capabilities/networking.yaml
tools:
  - name: "ping"
    description: "Test network connectivity to a host"
    input_schema:
      type: object
      properties:
        host:
          type: string
          description: "Hostname or IP address to ping"
        count:
          type: integer
          description: "Number of ping packets to send"
          default: 4
    routing:
      type: "command"
      command: "ping"
      args: ["-c", "{count}", "{host}"]

  - name: "curl_get"
    description: "Make HTTP GET request to a URL"
    input_schema:
      type: object
      properties:
        url:
          type: string
          description: "URL to request"
    routing:
      type: "http"
      method: "GET"
      url: "{url}"
```

## Usage Examples

### Basic Usage

```bash
# Test connectivity
curl -X POST http://localhost:8080/v1/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "smart_tool_discovery",
    "arguments": {"request": "ping google.com"}
  }'

# File operations  
curl -X POST http://localhost:8080/v1/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "smart_tool_discovery",
    "arguments": {"request": "list files in current directory"}
  }'
```

### Integration with MCP Clients

#### Claude Desktop
Add to your MCP configuration:
```json
{
  "mcpServers": {
    "magictunnel": {
      "command": "/path/to/magictunnel",
      "args": ["--stdio"]
    }
  }
}
```

#### Custom MCP Client
```python
import json
import requests

def call_magictunnel(request_text):
    response = requests.post(
        "http://localhost:8080/v1/mcp/call",
        json={
            "name": "smart_tool_discovery",
            "arguments": {"request": request_text}
        }
    )
    return response.json()

# Use it
result = call_magictunnel("check if google.com is reachable")
print(result)
```

## Advanced Features

### Tool Selection Modes

1. **Rule-based** (default): Fast keyword matching
2. **LLM-based**: AI-powered semantic understanding

### Parameter Mapping

MagicTunnel can extract parameters from natural language:

```
"ping google.com 10 times" → {"host": "google.com", "count": 10}
"get weather for San Francisco" → {"location": "San Francisco"}  
"read file /etc/hosts" → {"path": "/etc/hosts"}
```

### Routing Types

- **command**: Execute shell commands
- **http**: Make HTTP requests
- **external_mcp**: Forward to other MCP servers
- **function**: Call Rust functions

## API Reference

### MCP Protocol Endpoints

- `POST /v1/mcp/call` - Execute MCP tool calls
- `POST /v1/mcp/list_tools` - List available tools
- `GET /v1/mcp/resources` - List available resources

### Management Endpoints

- `GET /health` - Health check
- `GET /status` - System status
- `POST /reload` - Reload configuration

## Deployment

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/magictunnel /usr/local/bin/
EXPOSE 8080
CMD ["magictunnel"]
```

### Production Considerations

- Use reverse proxy (nginx) for TLS termination
- Configure proper logging levels
- Set up monitoring and metrics collection
- Use process manager (systemd) for service management

## Troubleshooting

### Common Issues

1. **Tool not found**: Check tool definitions in registry
2. **Parameter mapping failed**: Ensure clear parameter descriptions
3. **Backend timeout**: Adjust routing timeout settings
4. **Permission errors**: Verify command permissions and file access

### Debug Mode

```bash
RUST_LOG=debug magictunnel --config config.yaml
```

## Contributing

See the [original detailed README](../README-BACKUP.md) for comprehensive development information, architecture details, and contribution guidelines.

## License

MIT License - see [LICENSE](../LICENSE) for details.