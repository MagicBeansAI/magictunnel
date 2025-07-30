# SSE MCP Server Testing Guide

This guide shows how to test MagicTunnel's SSE MCP client implementation against real SSE MCP servers.

## Available SSE MCP Servers for Testing

### 1. FastAPI SSE MCP Server (Recommended)

**Repository**: https://github.com/ragieai/fastapi-sse-mcp

**Setup:**
```bash
# Clone the repository
git clone https://github.com/ragieai/fastapi-sse-mcp.git
cd fastapi-sse-mcp

# Install dependencies
uv init fastapi_sse --bare
cd fastapi_sse
uv add fastapi mcp

# Create the server (see repository for full implementation)
# Run the server
uvicorn app.main:app --reload --port 8000
```

**SSE Endpoint**: `http://localhost:8000/sse`

**Available Tools**: Echo tool, file operations, and other sample tools

### 2. Python Sample SSE MCP Server

**Repository**: https://github.com/edom18/MCP-SSE-Server-Sample

**Setup:**
```bash
# Clone the repository
git clone https://github.com/edom18/MCP-SSE-Server-Sample.git
cd MCP-SSE-Server-Sample

# Install dependencies (assuming uv is installed)
uv run mcp_server_sample --port 8080 --transport sse
```

**SSE Endpoint**: `http://localhost:8080/sse`

## Testing with MagicTunnel

### Step 1: Configure MagicTunnel for SSE

Create or update your `magictunnel-config.yaml`:

```yaml
server:
  host: "127.0.0.1"
  port: 8080

external_mcp_servers:
  # Process-based servers (existing)
  mcp_servers:
    example_process_server:
      command: ["uvx", "--from", "mcp-server-filesystem", "mcp-server-filesystem"]
      args: ["/tmp"]
      cwd: "/tmp"
      env:
        LOG_LEVEL: "DEBUG"

  # Network-based SSE servers (new)
  sse_services:
    fastapi_sse_server:
      enabled: true
      base_url: "http://localhost:8000"
      sse_endpoint: "/sse"
      auth: none
      heartbeat_interval: 30
      reconnect: true
      max_reconnect_attempts: 5
      reconnect_delay_ms: 1000
      max_reconnect_delay_ms: 30000
      pong_timeout: 10

    sample_sse_server:
      enabled: false  # Enable only one at a time for testing
      base_url: "http://localhost:8080"
      sse_endpoint: "/sse"
      auth: none
      heartbeat_interval: 30
      reconnect: true
      max_reconnect_attempts: 5
      reconnect_delay_ms: 1000
      max_reconnect_delay_ms: 30000
      pong_timeout: 10

smart_discovery:
  enabled: true
  tool_selection_mode: "rule_based"
  confidence_threshold: 0.7

registry:
  paths: ["./capabilities"]
```

### Step 2: Start the Test SSE Server

Choose one of the servers above and start it:

```bash
# Option 1: FastAPI server
cd fastapi-sse-mcp
uvicorn app.main:app --reload --port 8000

# Option 2: Python sample server
cd MCP-SSE-Server-Sample
uv run mcp_server_sample --port 8080 --transport sse
```

### Step 3: Start MagicTunnel

```bash
# Build MagicTunnel
cargo build --release

# Start MagicTunnel
./target/release/magictunnel --config magictunnel-config.yaml
```

### Step 4: Test the SSE Connection

#### Method 1: Direct API Testing

```bash
# Test smart discovery to find SSE tools
curl -X POST http://localhost:8080/v1/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "smart_tool_discovery",
    "arguments": {
      "request": "list available tools from SSE server"
    }
  }'

# Test a specific SSE tool (example: echo tool)
curl -X POST http://localhost:8080/v1/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "smart_tool_discovery",
    "arguments": {
      "request": "echo hello world"
    }
  }'
```

#### Method 2: MCP Client Testing

```bash
# List all available tools (should include SSE server tools)
curl -X POST http://localhost:8080/v1/mcp/list_tools \
  -H "Content-Type: application/json" \
  -d '{}'

# Call a specific tool
curl -X POST http://localhost:8080/v1/mcp/call_tool \
  -H "Content-Type: application/json" \
  -d '{
    "name": "echo",
    "arguments": {
      "message": "Hello from SSE MCP server!"
    }
  }'
```

### Step 5: Monitor Connection Status

Check MagicTunnel logs for SSE connection status:

```bash
# Look for these log messages:
# ✅ "SSE service connected successfully"
# ✅ "Discovered X tools from service Y"
# ❌ "Failed to connect to SSE service"
# 🔄 "SSE service status changed: Healthy"

# Enable debug logging for detailed SSE connection information
RUST_LOG=magictunnel::mcp::clients::sse_client=debug ./target/release/magictunnel
```

### Step 6: Test Web Dashboard

If using the supervisor:

```bash
# Start with web dashboard
./target/release/magictunnel-supervisor

# Open dashboard
open http://localhost:5173/dashboard

# Navigate to:
# - Services tab to see SSE connection status
# - Tools tab to see tools from SSE servers
# - Logs tab to monitor SSE connection events
```

## Troubleshooting

### Common Issues

1. **Connection Refused**
   ```
   Failed to connect to SSE service: Connection refused
   ```
   - Ensure the SSE server is running on the specified port
   - Check that the base_url and sse_endpoint are correct

2. **Authentication Required**
   ```
   Failed to connect to SSE service: 401 Unauthorized
   ```
   - Update auth configuration in magictunnel-config.yaml
   - Note: Many test servers don't require authentication

3. **No Tools Discovered**
   ```
   SSE service connected but no tools found
   ```
   - Check that the SSE server properly implements MCP protocol
   - Verify the server has tools configured

4. **Connection Drops Frequently**
   ```
   SSE connection lost, attempting reconnect
   ```
   - Adjust heartbeat_interval and timeout settings
   - Check network stability

### Debug Configuration

For detailed debugging, use this configuration:

```yaml
# Add to magictunnel-config.yaml
logging:
  level: debug
  modules:
    - "magictunnel::mcp::clients::sse_client"
    - "magictunnel::mcp::network_service_manager"
```

## Expected Behavior

When working correctly, you should see:

1. **Startup**: SSE client connects to remote server
2. **Discovery**: Tools from SSE server appear in MagicTunnel's tool registry
3. **Capability Files**: Generated in `./capabilities/network-sse-{service_id}.yaml`
4. **Tool Execution**: Smart discovery can find and execute SSE server tools
5. **Health Monitoring**: Connection status tracked and reported
6. **Reconnection**: Automatic reconnection on connection loss

## Performance Testing

Test SSE performance with concurrent requests:

```bash
# Test concurrent tool calls
for i in {1..10}; do
  curl -X POST http://localhost:8080/v1/mcp/call \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"smart_tool_discovery\", \"arguments\": {\"request\": \"echo test $i\"}}" &
done
wait
```

This should demonstrate MagicTunnel's SSE client handling multiple concurrent requests through the single-session SSE connection with request queuing.