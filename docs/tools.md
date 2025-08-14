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

## Generating Tools from APIs

MagicTunnel provides CLI tools to automatically generate MCP capability files from existing API specifications. This allows you to quickly integrate your internal APIs without manual YAML creation.

### Available Generators

| Generator | API Format | Binary Name | Description |
|-----------|------------|-------------|-------------|
| **Unified CLI** | All formats | `magictunnel-cli` | Unified interface for OpenAPI, gRPC, and GraphQL generation |

### OpenAPI/Swagger Generator

Generate tools from OpenAPI (Swagger) specifications:

```bash
# Basic usage
magictunnel-cli openapi \
  --spec path/to/openapi.json \
  --output capabilities/api-tools.yaml \
  --base-url https://api.example.com

# With authentication (Bearer token)
magictunnel-cli openapi \
  --spec https://api.example.com/openapi.json \
  --output capabilities/api-tools.yaml \
  --base-url https://api.example.com \
  --auth-type bearer \
  --auth-token $API_TOKEN

# With API key authentication
magictunnel-cli openapi \
  --spec openapi.yaml \
  --output capabilities/api-tools.yaml \
  --base-url https://api.example.com \
  --auth-type apikey \
  --auth-token $API_KEY \
  --auth-header "X-API-Key"

# With tool name prefix and specific methods
magictunnel-cli openapi \
  --spec openapi.json \
  --output capabilities/api-tools.yaml \
  --base-url https://api.example.com \
  --prefix "myapi_" \
  --methods "GET,POST,PUT" \
  --naming "method-path"

# Including deprecated operations
magictunnel-cli openapi \
  --spec openapi.json \
  --output capabilities/api-tools.yaml \
  --base-url https://api.example.com \
  --include-deprecated
```

#### OpenAPI Generator Options

| Option | Description | Default |
|--------|-------------|---------|
| `--spec` | OpenAPI specification file (JSON/YAML) or URL | Required |
| `--output` | Output capability file | Required |
| `--base-url` | Base URL for the API | Required |
| `--prefix` | Tool name prefix | None |
| `--auth-type` | Authentication type (none, bearer, apikey, basic) | `none` |
| `--auth-token` | Authentication token | None |
| `--auth-header` | API key header name | `X-API-Key` |
| `--auth-username` | Username for basic auth | None |
| `--auth-password` | Password for basic auth | None |
| `--naming` | Naming convention (operation-id, method-path) | `operation-id` |
| `--methods` | HTTP methods to include (comma-separated) | All |
| `--include-deprecated` | Include deprecated operations | false |

### gRPC Generator

Generate tools from gRPC protobuf service definitions:

```bash
# Basic usage
magictunnel-cli grpc \
  --proto path/to/service.proto \
  --output capabilities/grpc-tools.yaml \
  --endpoint localhost:50051

# With TLS and authentication
magictunnel-cli grpc \
  --proto service.proto \
  --output capabilities/grpc-tools.yaml \
  --endpoint api.example.com:443 \
  --tls \
  --auth-type bearer \
  --auth-token $GRPC_TOKEN

# With service filtering and metadata
magictunnel-cli grpc \
  --proto service.proto \
  --output capabilities/grpc-tools.yaml \
  --endpoint localhost:50051 \
  --service-filter "UserService,OrderService" \
  --prefix "mygrpc_" \
  --timeout 30

# With streaming strategy
magictunnel-cli grpc \
  --proto service.proto \
  --output capabilities/grpc-tools.yaml \
  --endpoint localhost:50051 \
  --streaming-strategy "buffered"
```

#### gRPC Generator Options

| Option | Description | Default |
|--------|-------------|---------|
| `--proto` | Protobuf (.proto) service definition file | Required |
| `--output` | Output capability file | Required |
| `--endpoint` | gRPC service endpoint | Required |
| `--prefix` | Tool name prefix | None |
| `--service-filter` | Services to include (comma-separated) | All |
| `--method-filter` | Methods to include (comma-separated) | All |
| `--tls` | Use TLS connection | false |
| `--auth-type` | Authentication type | `none` |
| `--auth-token` | Authentication token | None |
| `--timeout` | Request timeout in seconds | 30 |
| `--streaming-strategy` | Handle streaming (buffered, streamed, error) | `buffered` |

### GraphQL Generator

Generate tools from GraphQL schemas:

```bash
# From GraphQL SDL file
magictunnel-cli graphql \
  --schema schema.graphql \
  --endpoint https://api.example.com/graphql \
  --output capabilities/graphql-tools.yaml

# From introspection JSON
magictunnel-cli graphql \
  --schema introspection.json \
  --format json \
  --endpoint https://api.example.com/graphql \
  --output capabilities/graphql-tools.yaml

# With authentication and filtering
magictunnel-cli graphql \
  --schema schema.graphql \
  --endpoint https://api.example.com/graphql \
  --output capabilities/graphql-tools.yaml \
  --auth-type bearer \
  --auth-token $GRAPHQL_TOKEN \
  --operations "query,mutation" \
  --prefix "gql_"

# Exclude deprecated fields
magictunnel-cli graphql \
  --schema schema.graphql \
  --endpoint https://api.example.com/graphql \
  --output capabilities/graphql-tools.yaml \
  --exclude-deprecated
```

#### GraphQL Generator Options

| Option | Description | Default |
|--------|-------------|---------|
| `--schema` | GraphQL schema file (SDL or JSON) | Required |
| `--endpoint` | GraphQL endpoint URL | Required |
| `--output` | Output capability file | Required |
| `--format` | Schema format: sdl or json (auto-detected) | Auto |
| `--prefix` | Tool name prefix | None |
| `--operations` | Operation types (query, mutation, subscription) | All |
| `--auth-type` | Authentication type | `none` |
| `--auth-token` | Authentication token | None |
| `--exclude-deprecated` | Exclude deprecated fields | false |
| `--max-depth` | Maximum query depth | 10 |

### Unified CLI (magictunnel-cli)

The unified CLI provides a single interface for all generators with additional utilities:

```bash
# Generate from different sources
magictunnel-cli graphql --schema schema.graphql --endpoint https://api.example.com/graphql --output capabilities.yaml
magictunnel-cli grpc --proto service.proto --endpoint localhost:50051 --output capabilities.yaml  
magictunnel-cli openapi --spec openapi.json --base-url https://api.example.com --output capabilities.yaml

# Initialize configuration file
magictunnel-cli init --output generator-config.toml

# Use configuration file
magictunnel-cli generate --config generator-config.toml

# Merge multiple capability files
magictunnel-cli merge --input api1.yaml,api2.yaml,grpc.yaml --output combined.yaml

# Validate capability files
magictunnel-cli validate --input capabilities.yaml --strict

# Get help for specific subcommand
magictunnel-cli graphql --help
```

#### Configuration File Format (TOML)

```toml
# generator-config.toml
[general]
output_dir = "capabilities"
prefix = "mycompany_"

[openapi]
spec = "https://api.example.com/openapi.json"
base_url = "https://api.example.com"
auth_type = "bearer"
auth_token = "${API_TOKEN}"
methods = ["GET", "POST", "PUT", "DELETE"]
naming = "operation-id"

[grpc]
proto = "service.proto"
endpoint = "api.example.com:443"
tls = true
auth_type = "bearer"
auth_token = "${GRPC_TOKEN}"
services = ["UserService", "OrderService"]

[graphql]
schema = "schema.graphql"
endpoint = "https://api.example.com/graphql"
auth_type = "bearer"
auth_token = "${GRAPHQL_TOKEN}"
operations = ["query", "mutation"]
```

### Examples by Use Case

#### Internal REST API

```bash
# Generate tools for your company's REST API
magictunnel-cli openapi \
  --spec https://internal-api.company.com/openapi.json \
  --output capabilities/internal-api.yaml \
  --base-url https://internal-api.company.com \
  --auth-type bearer \
  --auth-token $INTERNAL_API_TOKEN \
  --prefix "company_"
```

#### Microservices gRPC APIs

```bash
# Generate tools for user service
magictunnel-cli grpc \
  --proto protos/user_service.proto \
  --output capabilities/user-service.yaml \
  --endpoint user-service:50051 \
  --prefix "user_"

# Generate tools for order service  
magictunnel-cli grpc \
  --proto protos/order_service.proto \
  --output capabilities/order-service.yaml \
  --endpoint order-service:50051 \
  --prefix "order_"

# Merge all services into one file
./target/release/magictunnel-cli merge \
  --input capabilities/user-service.yaml,capabilities/order-service.yaml \
  --output capabilities/microservices.yaml
```

#### GraphQL API with Complex Schema

```bash
# Generate tools from large GraphQL schema
magictunnel-cli graphql \
  --schema large-schema.graphql \
  --endpoint https://api.example.com/graphql \
  --output capabilities/graphql-api.yaml \
  --auth-type bearer \
  --auth-token $GRAPHQL_TOKEN \
  --operations "query,mutation" \
  --exclude-deprecated \
  --max-depth 5 \
  --prefix "api_"
```

### Generated Tool Structure

All generators create tools with this structure:

```yaml
tools:
  - name: "api_create_user"
    description: "Create a new user account with email and profile information"
    input_schema:
      type: object
      properties:
        email:
          type: string
          format: email
          description: "User's email address"
        name:
          type: string
          description: "User's full name"
        age:
          type: integer
          minimum: 13
          description: "User's age (must be 13 or older)"
    routing:
      type: http
      method: POST
      url: "https://api.example.com/users"
      headers:
        Content-Type: "application/json"
        Authorization: "Bearer {auth_token}"
      body: |
        {
          "email": "{email}",
          "name": "{name}",
          "age": {age}
        }
```

### Best Practices for Generated Tools

1. **Review Generated Files**: Always review generated capability files before using them in production

2. **Add Descriptions**: Enhance generated descriptions with domain-specific context:

```yaml
# Generated (generic)
description: "Create user"

# Enhanced (specific)
description: "Create a new customer account in the billing system with email verification"
```

3. **Security Configuration**: Set up proper authentication and never commit secrets:

```bash
# Use environment variables for secrets
export API_TOKEN="your-secret-token"
magictunnel-cli openapi --auth-token $API_TOKEN ...
```

4. **Tool Organization**: Use prefixes and organize by service:

```yaml
# Good organization
tools:
  - name: "billing_create_invoice"
  - name: "billing_get_invoice" 
  - name: "user_create_account"
  - name: "user_get_profile"
```

5. **Capability File Management**: Keep generated files separate from manually created ones:

```
capabilities/
├── manual/
│   ├── custom-tools.yaml
│   └── utility-tools.yaml
├── generated/
│   ├── billing-api.yaml
│   ├── user-service.yaml
│   └── graphql-api.yaml
└── combined/
    └── all-tools.yaml
```

### Testing Generated Tools

After generating tools, test them:

```bash
# Test specific generated tool
curl -X POST http://localhost:8080/v1/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "api_create_user",
    "arguments": {
      "email": "test@example.com",
      "name": "Test User",
      "age": 25
    }
  }'

# Test via smart discovery
curl -X POST http://localhost:8080/v1/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "smart_tool_discovery",
    "arguments": {
      "request": "create a new user account for john@example.com"
    }
  }'
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

5. **Generated tool authentication failed**
   - Verify API credentials are correct
   - Check authentication method matches API requirements
   - Test authentication outside of MagicTunnel first

6. **gRPC connection failed**
   - Verify gRPC service is running
   - Check if TLS is required (`--tls` flag)
   - Ensure protobuf file matches service version

7. **GraphQL introspection failed**
   - Check if introspection is enabled on the endpoint
   - Verify authentication for introspection queries
   - Try using a pre-exported schema file instead

### Debug Mode

```bash
# Enable debug logging for tool execution
RUST_LOG=debug magictunnel --config your-config.yaml

# Debug specific generator
RUST_LOG=debug magictunnel-cli openapi --spec openapi.json --output test.yaml --base-url https://api.example.com
```