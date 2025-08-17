# API Reference

## Overview

MagicTunnel provides multiple API interfaces for different client needs and performance requirements.

## Supported Protocols

### 1. WebSocket (Primary)
- **Endpoint**: `ws://localhost:3001/mcp/ws`
- **Protocol**: JSON-RPC 2.0
- **Features**: Real-time bidirectional communication, tool streaming
- **Best For**: Interactive clients, real-time applications

### 2. HTTP REST API
- **Base URL**: `http://localhost:3001`
- **Protocol**: Standard HTTP with JSON
- **Features**: Simple request/response, easy integration
- **Best For**: Simple integrations, testing, curl-based workflows

### 3. Server-Sent Events (SSE) - Deprecated
- **Stream Endpoint**: `GET /mcp/sse`
- **Messages Endpoint**: `POST /mcp/sse/messages`
- **Protocol**: Server-Sent Events with JSON for streaming, HTTP POST for requests
- **Features**: Bidirectional communication using two endpoints
- **Status**: Deprecated in favor of Streamable HTTP
- **Best For**: Legacy clients that require SSE support

### 4. gRPC Streaming
- **Port**: HTTP port + 1000 (default: 4000)
- **Protocol**: gRPC with Protocol Buffers
- **Features**: High-performance binary streaming, type safety
- **Best For**: High-throughput applications, microservice integration

## WebSocket API (JSON-RPC 2.0)

### Connection
```javascript
const ws = new WebSocket('ws://localhost:3001/mcp/ws');
ws.onopen = () => console.log('Connected to MagicTunnel');
```

### List Tools
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

### Call Tool
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

## HTTP REST API

### Health Check
```bash
curl http://localhost:3001/health
```

Response:
```json
{"status": "healthy", "timestamp": "2024-01-15T10:30:00Z"}
```

### List Tools
```bash
curl http://localhost:3001/tools
```

### Call Tool
```bash
curl -X POST http://localhost:3001/tools/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "execute_command",
    "arguments": {"command": "echo test"}
  }'
```

## Agent Configuration Examples

### Subprocess Agent
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

### HTTP Agent
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

### LLM Agent
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

### WebSocket Agent
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

## Custom GPT Integration

MagicTunnel now provides **complete OpenAPI 3.1.0 compatibility** for seamless integration with ChatGPT Custom GPTs and other OpenAI-compatible systems.

### Dual OpenAPI Specification Endpoints

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

### Features
- **🔧 OpenAPI 3.1.0 Generation**: Latest OpenAPI standard with enhanced JSON Schema support
- **📊 Dual Endpoints**: Choose between full tools access or smart discovery only
- **🎯 Custom GPT Optimized**: Smart discovery endpoint stays under 30-operation limit
- **🎯 Tool Execution Endpoints**: Each tool available at `/dashboard/api/tools/{name}/execute`
- **📋 Complete Documentation**: Full OpenAPI spec with descriptions, parameters, and response schemas
- **🔗 Custom GPT Ready**: Direct integration with ChatGPT Custom GPT Actions
- **⚡ Real-time Updates**: OpenAPI specs reflect current enabled tools dynamically

### Custom GPT Setup (Recommended)

**Option 1: Smart Discovery Only (Recommended)**
1. **Get Smart OpenAPI Spec**: `curl http://localhost:3001/dashboard/api/openapi-smart.json > smart-spec.json`
2. **Create Custom GPT**: Upload the smart discovery OpenAPI specification to ChatGPT Custom GPT Actions
3. **Configure Instructions**: Add instructions for using natural language with smart_tool_discovery
4. **Test Integration**: Access all MagicTunnel tools through intelligent discovery

**Option 2: Full Tools Access (For Advanced Users)**
1. **Get Full OpenAPI Spec**: `curl http://localhost:3001/dashboard/api/openapi.json > full-spec.json`
2. **Note**: May exceed Custom GPT's 30-operation limit depending on enabled tools
3. **Create Custom GPT**: Upload if under operation limit, otherwise use smart discovery

### Custom GPT Instructions Template
```
You have access to MagicTunnel's comprehensive toolkit through smart discovery. Use the smartToolDiscovery action with natural language requests like:

- "check system status and disk usage"
- "read the contents of package.json"
- "ping google.com to test connectivity"
- "make GET request to https://api.github.com/user"

Always explain which tool was discovered and executed for transparency.
```

### Example Tool Execution
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

## Management APIs Overview

MagicTunnel provides comprehensive REST APIs for managing and monitoring all aspects of the system. These APIs enable advanced UI development, system integration, and automation.

### API Categories

#### **Resource Management APIs**
- **Base Path**: `/dashboard/api/resources/management/`
- **Purpose**: Manage MCP resources, content, and providers
- **Documentation**: [Complete Resource Management API Reference](prompt-resource-management.md#resource-management-apis)

**Key Endpoints**:
- `GET /status` - System health and configuration
- `GET /resources` - List resources with filtering and pagination  
- `POST /resources/{uri}/read` - Read resource content with options
- `POST /validate` - Validate resource URIs and accessibility
- `GET /statistics` - Comprehensive usage analytics

#### **Enhancement Pipeline APIs**
- **Base Path**: `/dashboard/api/enhancements/pipeline/`
- **Purpose**: Manage LLM-powered tool enhancement pipeline
- **Documentation**: [Complete Enhancement Pipeline API Reference](automatic-llm-generation-workflow.md#enhancement-pipeline-management-apis)

**Key Endpoints**:
- `GET /status` - Pipeline health and configuration
- `GET /tools` - Enhanced tools listing with metadata
- `POST /tools/{name}/enhance` - Trigger individual tool enhancement
- `GET /jobs` - Track enhancement job status and history
- `POST /batch` - Batch enhancement processing
- `GET /statistics` - Performance metrics and analytics

#### **Prompt Management APIs**
- **Base Path**: `/dashboard/api/prompts/management/`
- **Purpose**: Manage MCP prompts and templates
- **Features**: CRUD operations, template management, content generation

#### **Provider Management APIs**
- **Base Path**: `/dashboard/api/providers/`
- **Purpose**: Manage LLM providers (OpenAI, Anthropic, Ollama)
- **Features**: Health monitoring, configuration, rate limit tracking

### Authentication Patterns

All Management APIs support multiple authentication methods:

**API Key Authentication**:
```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
     "http://localhost:3001/dashboard/api/resources/management/status"
```

**Session Token Authentication**:
```bash
curl -H "X-Session-Token: YOUR_SESSION_TOKEN" \
     "http://localhost:3001/dashboard/api/enhancements/pipeline/status"
```

**JWT Authentication**:
```bash
curl -H "Authorization: JWT YOUR_JWT_TOKEN" \
     "http://localhost:3001/dashboard/api/prompts/management/list"
```

### Common Response Patterns

#### Success Response Format
```json
{
  "success": true,
  "data": {
    // API-specific data
  },
  "metadata": {
    "timestamp": "2025-08-03T10:30:00Z",
    "request_id": "req_123456789",
    "processing_time_ms": 45
  }
}
```

#### Error Response Format
```json
{
  "error": {
    "code": "RESOURCE_NOT_FOUND",
    "message": "Resource not found",
    "details": "The specified resource URI does not exist or is not accessible",
    "timestamp": "2025-08-03T10:30:00Z",
    "request_id": "req_123456789",
    "help_url": "https://docs.example.com/errors/RESOURCE_NOT_FOUND"
  }
}
```

#### Pagination Response Format
```json
{
  "data": [
    // Array of items
  ],
  "pagination": {
    "total": 150,
    "returned": 25,
    "limit": 25,
    "offset": 50,
    "has_more": true,
    "cursor": "eyJvZmZzZXQiOjc1fQ=="
  }
}
```

### Common Query Parameters

Most Management APIs support these standard query parameters:

- `limit` (integer): Maximum results per page (default: 50, max: 1000)
- `offset` (integer): Skip number of results for pagination
- `cursor` (string): Pagination cursor for efficient large dataset traversal
- `filter` (string): Text filter for names, descriptions, or content
- `sort` (string): Sort field (name, created_at, updated_at)
- `order` (string): Sort order (asc, desc)
- `include_metadata` (boolean): Include detailed metadata in responses
- `format` (string): Response format (json, csv, xml)

### Rate Limiting

Management APIs implement rate limiting to ensure system stability:

**Rate Limit Headers**:
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 995
X-RateLimit-Reset: 1691067000
X-RateLimit-Window: 3600
```

**Rate Limit Exceeded Response**:
```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "API rate limit exceeded",
    "retry_after_seconds": 300,
    "limit": 1000,
    "window_seconds": 3600
  }
}
```

### Management API Quick Start

#### 1. System Health Check
```bash
# Check overall system health
curl -X GET "http://localhost:3001/dashboard/api/resources/management/status"
curl -X GET "http://localhost:3001/dashboard/api/enhancements/pipeline/status"
```

#### 2. Resource Management
```bash
# List available resources
curl -X GET "http://localhost:3001/dashboard/api/resources/management/resources?limit=10"

# Read specific resource
curl -X POST "http://localhost:3001/dashboard/api/resources/management/resources/file%3A%2F%2F%2Fdocs%2Fapi.md/read" \
     -H "Content-Type: application/json" \
     -d '{"max_length": 1000}'
```

#### 3. Enhancement Pipeline
```bash
# List enhanced tools
curl -X GET "http://localhost:3001/dashboard/api/enhancements/pipeline/tools?enhancement_status=enhanced"

# Trigger tool enhancement
curl -X POST "http://localhost:3001/dashboard/api/enhancements/pipeline/tools/network_ping/enhance" \
     -H "Content-Type: application/json" \
     -d '{"force_regenerate": false}'
```

#### 4. Statistics and Analytics
```bash
# Get resource statistics
curl -X GET "http://localhost:3001/dashboard/api/resources/management/statistics"

# Get enhancement pipeline metrics
curl -X GET "http://localhost:3001/dashboard/api/enhancements/pipeline/statistics"
```

### Integration Examples

#### Frontend Dashboard Integration
```javascript
// Resource management dashboard
class ResourceManager {
  async getResources(filter = '', limit = 50) {
    const response = await fetch(
      `/dashboard/api/resources/management/resources?filter=${filter}&limit=${limit}`,
      {
        headers: {
          'Authorization': `Bearer ${this.apiKey}`,
          'Content-Type': 'application/json'
        }
      }
    );
    return response.json();
  }

  async readResource(uri, options = {}) {
    const response = await fetch(
      `/dashboard/api/resources/management/resources/${encodeURIComponent(uri)}/read`,
      {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${this.apiKey}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(options)
      }
    );
    return response.json();
  }
}
```

#### Enhancement Pipeline Monitoring
```javascript
// Real-time enhancement monitoring
class EnhancementMonitor {
  async monitorEnhancement(toolName) {
    // Start enhancement
    const job = await this.startEnhancement(toolName);
    
    // Poll job status
    return new Promise((resolve, reject) => {
      const pollInterval = setInterval(async () => {
        const status = await this.getJobStatus(job.job_id);
        
        if (status.status === 'completed') {
          clearInterval(pollInterval);
          resolve(status.result);
        } else if (status.status === 'failed') {
          clearInterval(pollInterval);
          reject(new Error(status.error));
        }
      }, 1000);
    });
  }
}
```

### Performance and Caching

Management APIs implement multiple levels of caching for optimal performance:

**Response Caching**:
- Static data cached for 5 minutes
- Dynamic data cached for 30 seconds
- Statistics cached for 1 minute

**Cache Headers**:
```
Cache-Control: public, max-age=300
ETag: "a1b2c3d4e5f6g7h8i9j0"
Last-Modified: Wed, 03 Aug 2025 10:30:00 GMT
```

**Conditional Requests**:
```bash
# Use ETag for conditional requests
curl -H "If-None-Match: a1b2c3d4e5f6g7h8i9j0" \
     "http://localhost:3001/dashboard/api/resources/management/resources"
```

Management APIs provide the foundation for building sophisticated UIs and integrations while maintaining high performance and reliability.

## MCP Logging and Notifications API

### MCP Logging System

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
curl -X POST http://localhost:3001/mcp/logging/setLevel \
  -H "Content-Type: application/json" \
  -d '{"level": "debug"}'

# Set log level for specific logger
curl -X POST http://localhost:3001/mcp/logging/setLevel \
  -H "Content-Type: application/json" \
  -d '{"level": "info", "logger": "agent-router"}'
```

**Rate Limiting**: The logging system implements rate limiting (100 messages per minute per logger) to prevent DoS attacks and log flooding.

### MCP Notifications System

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

## Streaming Protocol Examples

### WebSocket Connection
```javascript
const ws = new WebSocket('ws://localhost:3001/mcp/ws');
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('Received:', message);
};
```

### Server-Sent Events (Bidirectional)
```javascript
// 1. Establish SSE stream for receiving notifications and responses
const eventSource = new EventSource('/mcp/sse');
eventSource.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('SSE notification:', data);
  
  // Handle different message types
  if (data.method === 'notifications/initialized') {
    console.log('SSE connection initialized');
  } else if (data.method === 'notifications/tools/list_changed') {
    console.log('Tools list changed notification received');
  }
};

// 2. Send MCP requests via the messages endpoint
async function sendMcpRequest(method, params = {}) {
  const request = {
    jsonrpc: "2.0",
    id: Date.now().toString(),
    method: method,
    params: params
  };
  
  const response = await fetch('/mcp/sse/messages', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(request)
  });
  
  return await response.json();
}

// Example usage
async function initializeAndListTools() {
  // Initialize MCP session
  const initResponse = await sendMcpRequest('initialize', {
    protocolVersion: "2024-11-05",
    capabilities: {
      roots: { listChanged: true },
      sampling: {}
    },
    clientInfo: {
      name: "SSE Test Client",
      version: "1.0.0"
    }
  });
  
  console.log('Initialize response:', initResponse);
  
  // List available tools
  const toolsResponse = await sendMcpRequest('tools/list');
  console.log('Available tools:', toolsResponse.result.tools);
  
  // Call a tool
  const callResponse = await sendMcpRequest('tools/call', {
    name: "smart_tool_discovery",
    arguments: {
      request: "ping google.com"
    }
  });
  
  console.log('Tool call result:', callResponse.result);
}

// Start the bidirectional SSE communication
initializeAndListTools();
```

### HTTP Streaming
```bash
curl -N http://localhost:3001/mcp/call/stream \
  -H "Content-Type: application/json" \
  -d '{"name": "long_running_task", "arguments": {}}'
```

### gRPC Client Example
```rust
let mut client = McpServiceClient::connect("http://localhost:4001").await?;
let request = tonic::Request::new(ListToolsRequest {});
let response = client.list_tools(request).await?;
```

## Error Responses

### Standard Error Format
```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": {
      "details": "Missing required parameter 'name'",
      "timestamp": "2024-01-15T10:30:00Z"
    }
  }
}
```

### Common Error Codes
- `-32700`: Parse error
- `-32600`: Invalid Request
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error
- `-32000` to `-32099`: Server error range

## Authentication

### API Key Authentication
```bash
curl -H "Authorization: Bearer your-api-key" \
  http://localhost:3001/tools
```

### JWT Authentication
```bash
curl -H "Authorization: Bearer your-jwt-token" \
  http://localhost:3001/tools
```

### OAuth 2.0
```bash
# Get authorization URL
curl http://localhost:3001/auth/oauth/authorize?provider=github

# Exchange code for token
curl -X POST http://localhost:3001/auth/oauth/callback \
  -d "code=authorization_code&state=state_value"
```

## Rate Limiting

### Headers
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1609459200
```

### Rate Limit Response
```json
{
  "error": {
    "code": 429,
    "message": "Rate limit exceeded",
    "retry_after": 60
  }
}
```

## Health and Status Endpoints

### Health Check
```bash
curl http://localhost:3001/health
```

Response:
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "uptime": 3600,
  "version": "0.2.48"
}
```

### Detailed Status
```bash
curl http://localhost:3001/status
```

Response:
```json
{
  "server": {
    "status": "running",
    "uptime": 3600,
    "connections": {
      "websocket": 5,
      "http": 12
    }
  },
  "registry": {
    "tools_loaded": 83,
    "last_reload": "2024-01-15T10:25:00Z"
  },
  "external_mcp": {
    "servers_connected": 3,
    "tools_proxied": 25
  }
}
```

### Metrics
```bash
curl http://localhost:3001/metrics
```

Response:
```json
{
  "requests_total": 1250,
  "requests_per_second": 2.5,
  "average_response_time": 150,
  "tools_executed": {
    "smart_tool_discovery": 450,
    "execute_command": 200,
    "http_request": 150
  },
  "errors": {
    "total": 25,
    "rate": 0.02
  }
}
```

## WebSocket Events

### Connection Events
```json
// On connect
{
  "jsonrpc": "2.0",
  "method": "connection/established",
  "params": {
    "session_id": "sess_123",
    "capabilities": ["tools", "resources", "prompts"]
  }
}

// On disconnect  
{
  "jsonrpc": "2.0",
  "method": "connection/closed",
  "params": {
    "reason": "client_disconnect",
    "code": 1000
  }
}
```

### Tool Execution Events
```json
// Tool execution started
{
  "jsonrpc": "2.0",
  "method": "tools/execution_started",
  "params": {
    "tool_name": "execute_command",
    "execution_id": "exec_456"
  }
}

// Tool execution completed
{
  "jsonrpc": "2.0",
  "method": "tools/execution_completed",
  "params": {
    "execution_id": "exec_456",
    "duration_ms": 150,
    "success": true
  }
}
```

## SDK Examples

### JavaScript/Node.js
```javascript
const WebSocket = require('ws');

class MagicTunnelClient {
  constructor(url) {
    this.ws = new WebSocket(url);
    this.requestId = 0;
    this.pending = new Map();
  }

  async callTool(name, arguments) {
    const id = ++this.requestId;
    
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      
      this.ws.send(JSON.stringify({
        jsonrpc: "2.0",
        id,
        method: "tools/call",
        params: { name, arguments }
      }));
    });
  }
}

// Usage
const client = new MagicTunnelClient('ws://localhost:3001/mcp/ws');
const result = await client.callTool('smart_tool_discovery', {
  request: 'ping google.com'
});
```

### Python
```python
import asyncio
import websockets
import json

class MagicTunnelClient:
    def __init__(self, url):
        self.url = url
        self.request_id = 0
        
    async def call_tool(self, name, arguments):
        async with websockets.connect(self.url) as websocket:
            self.request_id += 1
            
            message = {
                "jsonrpc": "2.0",
                "id": self.request_id,
                "method": "tools/call",
                "params": {
                    "name": name,
                    "arguments": arguments
                }
            }
            
            await websocket.send(json.dumps(message))
            response = await websocket.recv()
            return json.loads(response)

# Usage
client = MagicTunnelClient('ws://localhost:3001/mcp/ws')
result = await client.call_tool('smart_tool_discovery', {
    'request': 'ping google.com'
})
```

### Rust
```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ws_stream, _) = connect_async("ws://localhost:3001/mcp/ws").await?;
    let (mut write, mut read) = ws_stream.split();

    // Send tool call
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "smart_tool_discovery",
            "arguments": {
                "request": "ping google.com"
            }
        }
    });

    write.send(Message::Text(request.to_string())).await?;
    
    // Read response
    if let Some(msg) = read.next().await {
        let response = msg?;
        println!("Response: {}", response);
    }

    Ok(())
}
```