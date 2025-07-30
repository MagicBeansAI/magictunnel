# Adding Tools Guide

## Overview

MagicTunnel tools are defined in YAML files that specify the tool's interface and how to route requests to backend implementations.

## Basic Tool Definition

Create a file in your registry path (e.g., `capabilities/my-tools.yaml`):

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
        count:
          type: integer
          description: "Number of ping packets to send"
          default: 4
    routing:
      type: "command"
      command: "ping"
      args: ["-c", "{count}", "{host}"]
```

## Tool Schema

Every tool must have:

- **name**: Unique identifier for the tool
- **description**: Clear description for smart discovery
- **input_schema**: JSON Schema defining parameters
- **routing**: How to execute the tool

## Routing Types

### 1. Command Execution

Execute shell commands:

```yaml
routing:
  type: "command"
  command: "ls"
  args: ["-la", "{path}"]
  working_dir: "/tmp"         # Optional
  timeout: 30                 # Optional timeout in seconds
```

### 2. HTTP Requests

Make HTTP API calls:

```yaml
routing:
  type: "http"
  method: "GET"               # GET, POST, PUT, DELETE
  url: "https://api.example.com/users/{id}"
  headers:                    # Optional
    Authorization: "Bearer {token}"
    Content-Type: "application/json"
  body: |                     # Optional (for POST/PUT)
    {"name": "{name}", "email": "{email}"}
```

### 3. External MCP Servers

Forward to other MCP servers:

```yaml
routing:
  type: "external_mcp" 
  server: "filesystem-server"
  tool: "read_file"
  parameter_mapping:          # Optional parameter transformation
    file_path: "{path}"
```

### 4. Function Calls

Call Rust functions (advanced):

```yaml
routing:
  type: "function"
  module: "my_module"
  function: "process_data"
```

## Parameter Substitution

Use `{parameter_name}` to inject parameters:

```yaml
# Simple substitution
args: ["--host", "{host}", "--port", "{port}"]

# Default values
args: ["--count", "{count|4}", "--timeout", "{timeout|30}"]

# Array indexing
args: ["{hosts[0]}", "{hosts[1]}"]

# Conditional substitution
args: 
  - "--verbose"
  - "{verbose?--debug:--quiet}"
```

## Schema Types

### String Parameters

```yaml
properties:
  name:
    type: string
    description: "User name"
    minLength: 1
    maxLength: 50
    pattern: "^[a-zA-Z]+$"    # Optional regex
```

### Integer Parameters

```yaml
properties:
  count:
    type: integer
    description: "Number of items"
    minimum: 1
    maximum: 100
    default: 10
```

### Boolean Parameters

```yaml
properties:
  verbose:
    type: boolean
    description: "Enable verbose output"
    default: false
```

### Array Parameters

```yaml
properties:
  tags:
    type: array
    description: "List of tags"
    items:
      type: string
    minItems: 1
    maxItems: 10
```

### Object Parameters

```yaml
properties:
  config:
    type: object
    description: "Configuration object"
    properties:
      host:
        type: string
      port:
        type: integer
    required: ["host"]
```

## Tool Categories

Organize tools by category for better discovery:

```yaml
tools:
  - name: "ping"
    description: "Test network connectivity"
    category: "networking"
    # ... rest of definition
    
  - name: "traceroute"  
    description: "Trace network path"
    category: "networking"
    # ... rest of definition
```

## Advanced Features

### Environment Variables

```yaml
routing:
  type: "command"
  command: "my-script"
  args: ["{input}"]
  env:
    API_KEY: "${MY_API_KEY}"
    DEBUG: "true"
```

### Conditional Routing

Route based on parameters:

```yaml
routing:
  type: "conditional"
  conditions:
    - if: "{method} == 'GET'"
      then:
        type: "http"
        method: "GET"
        url: "https://api.example.com/{endpoint}"
    - else:
        type: "command"
        command: "curl"
        args: ["-X", "{method}", "https://api.example.com/{endpoint}"]
```

### Error Handling

```yaml
routing:
  type: "command"
  command: "my-command"
  args: ["{input}"]
  error_handling:
    retry_attempts: 3
    retry_delay: 1000         # milliseconds
    timeout: 30               # seconds
```

## Testing Tools

Test your tool definitions:

```bash
# Test tool directly
curl -X POST http://localhost:8080/v1/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "ping",
    "arguments": {"host": "google.com", "count": 2}
  }'

# Test via smart discovery
curl -X POST http://localhost:8080/v1/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "smart_tool_discovery",
    "arguments": {"request": "ping google.com twice"}
  }'
```

## Best Practices

### 1. Clear Descriptions

```yaml
# Good
description: "Test network connectivity by sending ICMP ping packets to a host"

# Bad  
description: "Ping tool"
```

### 2. Comprehensive Schemas

```yaml
# Include helpful descriptions and constraints
properties:
  host:
    type: string
    description: "Hostname or IP address to ping (e.g., google.com, 8.8.8.8)"
    pattern: "^[a-zA-Z0-9.-]+$"
  count:
    type: integer
    description: "Number of ping packets to send (1-10)"
    minimum: 1
    maximum: 10
    default: 4
```

### 3. Meaningful Names

```yaml
# Good - describes what it does
name: "ping_host"

# Bad - too generic
name: "network_tool"
```

### 4. Error Messages

```yaml
routing:
  type: "command"
  command: "ping"
  args: ["-c", "{count}", "{host}"]
  error_mapping:
    1: "Network is unreachable"
    2: "Host not found"
```

## Troubleshooting

### Common Issues

1. **Tool not found by smart discovery**
   - Check description keywords
   - Add more descriptive text
   - Verify tool is in registry path

2. **Parameter substitution failed**
   - Check parameter names match schema
   - Verify required parameters are provided
   - Test with simple static values first

3. **Command execution failed**
   - Test command manually first
   - Check file permissions
   - Verify command is in PATH

4. **Schema validation errors**
   - Validate JSON Schema syntax
   - Test with online schema validators
   - Check parameter types match usage

### Debug Mode

```bash
# Enable debug logging for tool execution
RUST_LOG=debug magictunnel --config your-config.yaml
```