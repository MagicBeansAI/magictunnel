# Unified Capability Generator CLI

The `mcp-generator` is a unified command-line tool for generating MCP capability files from various sources, including GraphQL schemas, gRPC/protobuf service definitions, and OpenAPI specifications. This tool streamlines the process of creating capability files by providing a consistent interface and configuration format across different generator types.

## Overview

The unified capability generator CLI offers several advantages over using individual generator tools:

- **Consistent Interface**: Use the same command structure and options across different generator types
- **Configuration File Support**: Define complex configurations in a single TOML file
- **Utility Commands**: Merge, validate, and manage capability files with built-in commands
- **Authentication Handling**: Consistent authentication configuration across all generators
- **Extensibility**: Easy to add support for new generator types in the future

This tool is designed to simplify the workflow for creating and managing MCP capability files, making it easier to integrate different API types into your MCP ecosystem.

## Installation

The `mcp-generator` is included in the MCP Proxy package. After building the project, you can use it directly:

```bash
cargo build --release
./target/release/mcp-generator --help
```

## Usage

The CLI provides several subcommands for different generator types and operations:

```
mcp-generator [SUBCOMMAND]
```

### Available Subcommands

- `graphql`: Generate capabilities from GraphQL schema
- `grpc`: Generate capabilities from gRPC/protobuf
- `openapi`: Generate capabilities from OpenAPI specification
- `init`: Initialize a new configuration file
- `merge`: Merge multiple capability files into one
- `validate`: Validate capability files

Each subcommand has its own set of options. Use `--help` with any subcommand to see its specific options:

```bash
mcp-generator graphql --help
mcp-generator grpc --help
mcp-generator openapi --help
mcp-generator init --help
mcp-generator merge --help
mcp-generator validate --help
```

## Configuration File

The `mcp-generator` supports using both YAML and TOML configuration files to specify generator options, with YAML being the preferred format for consistency with the main MCP Proxy configuration. This is especially useful for complex configurations or when you need to generate capabilities from multiple sources.

### Creating a Configuration File

To create a new configuration file:

```bash
# Create YAML configuration (recommended)
mcp-generator init --output mcp-generator.yaml

# Create TOML configuration (still supported)
mcp-generator init --output mcp-generator.toml
```

This will create a template configuration file that you can edit to suit your needs.

### Using a Configuration File

To use a configuration file:

```bash
# Using YAML configuration (recommended)
mcp-generator graphql --config mcp-generator.yaml
mcp-generator grpc --config mcp-generator.yaml
mcp-generator openapi --config mcp-generator.yaml

# Using TOML configuration (still supported)
mcp-generator graphql --config mcp-generator.toml
mcp-generator grpc --config mcp-generator.toml
mcp-generator openapi --config mcp-generator.toml
```

## Generator Subcommands

### GraphQL Generator

The GraphQL generator creates capability files from GraphQL schemas, supporting both SDL (Schema Definition Language) and JSON introspection formats.

#### Command-line Options

```bash
mcp-generator graphql [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-s, --schema <FILE>` | GraphQL schema file (SDL or JSON introspection) |
| `-e, --endpoint <URL>` | GraphQL endpoint URL |
| `-o, --output <FILE>` | Output capability file (YAML) |
| `-p, --prefix <PREFIX>` | Tool name prefix |
| `-a, --auth-type <TYPE>` | Authentication type: none, bearer, apikey (default: none) |
| `-t, --auth-token <TOKEN>` | Authentication token (for bearer or apikey) |
| `--auth-header <HEADER>` | Authentication header name (for apikey) (default: Authorization) |
| `-c, --config <FILE>` | Configuration file (TOML) |

#### Example

```bash
mcp-generator graphql \
  --schema schema.graphql \
  --endpoint https://api.example.com/graphql \
  --output graphql-capabilities.yaml \
  --prefix graphql \
  --auth-type bearer \
  --auth-token YOUR_TOKEN
```

### gRPC Generator

The gRPC generator creates capability files from gRPC/protobuf service definitions.

#### Command-line Options

```bash
mcp-generator grpc [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-p, --proto <FILE>` | Protobuf (.proto) file containing service definitions |
| `-o, --output <FILE>` | Output capability file (YAML) |
| `-e, --endpoint <ENDPOINT>` | gRPC service endpoint (e.g., localhost:50051) |
| `--prefix <PREFIX>` | Tool name prefix |
| `--service-filter <SERVICES>` | Comma-separated list of service names to include |
| `--method-filter <METHODS>` | Comma-separated list of method names to include |
| `--server-streaming <STRATEGY>` | Strategy for server streaming methods (polling, pagination, agent-level) (default: polling) |
| `--client-streaming <STRATEGY>` | Strategy for client streaming methods (polling, pagination, agent-level) (default: polling) |
| `--bidirectional-streaming <STRATEGY>` | Strategy for bidirectional streaming methods (polling, pagination, agent-level) (default: polling) |

#### Streaming Strategies

The gRPC generator supports different strategies for handling streaming methods:

- **polling**: Client periodically polls for updates (simplest approach)
- **pagination**: Streaming results are paginated for easier consumption
- **agent-level**: Streaming is handled at the agent level, with the agent responsible for managing the stream
| `--include-method-options` | Include method options in tool definitions |
| `--separate-streaming-tools` | Generate separate tools for streaming methods |
| `-a, --auth-type <TYPE>` | Authentication type (none, bearer, apikey, basic, oauth) (default: none) |
| `--auth-token <TOKEN>` | Authentication token (for bearer/apikey/oauth auth) |
| `--auth-header <HEADER>` | Authentication header name (for apikey auth) (default: X-API-Key) |
| `--auth-username <USERNAME>` | Username (for basic auth) |
| `--auth-password <PASSWORD>` | Password (for basic auth) |
| `--auth-token-type <TYPE>` | Token type (for oauth auth) (default: Bearer) |
| `-c, --config <FILE>` | Configuration file (TOML) |

#### Example

```bash
mcp-generator grpc \
  --proto service.proto \
  --endpoint localhost:50051 \
  --output grpc-capabilities.yaml \
  --prefix grpc \
  --server-streaming polling \
  --client-streaming agent-level \
  --bidirectional-streaming pagination
```

### OpenAPI Generator

The OpenAPI generator creates capability files from OpenAPI 3.0 and Swagger 2.0 specifications (JSON or YAML). It features advanced schema parsing, comprehensive reference resolution, and inheritance pattern support.

#### Command-line Options

```bash
mcp-generator openapi [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-s, --spec <FILE>` | OpenAPI specification file (JSON or YAML) |
| `-o, --output <FILE>` | Output capability file (YAML) |
| `-u, --base-url <URL>` | Base URL for the API |
| `-p, --prefix <PREFIX>` | Tool name prefix |
| `-a, --auth-type <TYPE>` | Authentication type (none, bearer, apikey, basic) (default: none) |
| `-t, --auth-token <TOKEN>` | Authentication token (for bearer/apikey auth) |
| `--auth-header <HEADER>` | Authentication header name (for apikey auth) (default: X-API-Key) |
| `--auth-username <USERNAME>` | Username (for basic auth) |
| `--auth-password <PASSWORD>` | Password (for basic auth) |
| `-n, --naming <CONVENTION>` | Naming convention (operation-id, method-path) (default: operation-id) |
| `-m, --methods <METHODS>` | Comma-separated list of HTTP methods to include (e.g., GET,POST) |
| `--include-deprecated` | Include deprecated operations |
| `-c, --config <FILE>` | Configuration file (TOML) |

#### Example

```bash
mcp-generator openapi \
  --spec openapi.json \
  --base-url https://api.example.com \
  --output openapi-capabilities.yaml \
  --prefix api \
  --naming operation-id \
  --methods GET,POST,PUT,DELETE
```

## Utility Subcommands

### Initialize Configuration

Creates a new configuration file template.

```bash
mcp-generator init [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-o, --output <FILE>` | Output configuration file (default: mcp-generator.yaml) |

Example:
```bash
mcp-generator init --output my-config.yaml
```

### Merge Capability Files

Merges multiple capability files into a single file.

```bash
mcp-generator merge [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-i, --input <FILES>` | Input capability files (comma-separated) |
| `-o, --output <FILE>` | Output merged capability file |
| `-s, --strategy <STRATEGY>` | Merge strategy for handling duplicates (keep-first, keep-last, rename, error) (default: error) |

Example:
```bash
mcp-generator merge \
  --input graphql-capabilities.yaml,grpc-capabilities.yaml \
  --output merged-capabilities.yaml \
  --strategy rename
```

### Validate Capability Files

Validates capability files for correctness and compliance with the MCP specification.

```bash
mcp-generator validate [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-i, --input <FILES>` | Input capability files to validate (comma-separated) |
| `-s, --strict` | Enable strict validation |

Example:
```bash
mcp-generator validate \
  --input capabilities.yaml \
  --strict
```

## Configuration File Structure

The configuration file uses YAML format (recommended) and has the following structure. You can generate a template using the `init` command and customize it for your needs.

### Basic Structure

```yaml
# Global settings applied to all generators
global:
  tool_prefix: "mcp"
  output_dir: "./capabilities"

  # Global authentication configuration
  auth:
    auth_type:
      type: "bearer"
      token: "YOUR_TOKEN"
    headers:
      X-Custom-Header: "value"

# Output format settings
output:
  format: "yaml"  # or "json"
  pretty: true
  directory: "./capabilities"
  file_pattern: "{name}-capabilities.{ext}"

# GraphQL generator configuration
graphql:
  endpoint: "https://api.example.com/graphql"
  tool_prefix: "graphql"  # Overrides global.tool_prefix
  include_deprecated: false
  include_descriptions: true
  separate_mutation_query: true

  # GraphQL-specific authentication
  auth:
    auth_type:
      type: "api_key"
      key: "YOUR_API_KEY"
      header: "X-API-Key"
    headers: {}

# gRPC generator configuration
grpc:
  endpoint: "localhost:50051"
  tool_prefix: "grpc"  # Overrides global.tool_prefix
  service_filter:
    - "UserService"
    - "ProductService"
  method_filter:
    - "GetUser"
    - "ListProducts"
  server_streaming_strategy: "polling"
  client_streaming_strategy: "agent-level"
  bidirectional_streaming_strategy: "pagination"
  include_method_options: true
  separate_streaming_tools: false

# OpenAPI generator configuration
openapi:
  base_url: "https://api.example.com"
  tool_prefix: "api"  # Overrides global.tool_prefix
  naming_convention: "operation-id"
  methods:
    - "GET"
    - "POST"
    - "PUT"
    - "DELETE"
  include_deprecated: false
```

### Authentication Configuration

The configuration file supports various authentication methods using YAML tag format.

#### Authentication Format

Authentication types use a structured format with a `type` field to specify the authentication method:

#### Supported Authentication Methods

#### Bearer Token

```yaml
global:
  auth:
    auth_type:
      type: "bearer"
      token: "YOUR_TOKEN"
    headers: {}
```

#### API Key

```yaml
global:
  auth:
    auth_type:
      type: "api_key"
      key: "YOUR_API_KEY"
      header: "X-API-Key"
    headers: {}
```

#### Basic Authentication

```yaml
global:
  auth:
    auth_type:
      type: "basic"
      username: "user"
      password: "pass"
    headers: {}
```

#### OAuth

```yaml
global:
  auth:
    auth_type:
      type: "oauth"
      token: "YOUR_TOKEN"
      token_type: "Bearer"
    headers: {}
```

#### Custom Headers

```toml
[global.auth]
headers = { "X-Custom-Header" = "value", "Another-Header" = "another-value" }
```

## Common Workflows

Here are some common workflows and examples to help you get started with the unified generator CLI.

### Generate from Multiple Sources

Using a configuration file, you can generate capability files from multiple sources:

1. Create a configuration file:
   ```bash
   mcp-generator init --output config.toml
   ```

2. Edit the configuration file to include settings for GraphQL, gRPC, and OpenAPI generators.

3. Generate capabilities:
   ```bash
   mcp-generator graphql --config config.yaml
   mcp-generator grpc --config config.yaml
   mcp-generator openapi --config config.yaml
   ```

### Merge and Validate

1. Generate capabilities from different sources:
   ```bash
   mcp-generator graphql --schema schema.graphql --endpoint https://api.example.com/graphql --output graphql.yaml
   mcp-generator openapi --spec openapi.json --base-url https://api.example.com --output openapi.yaml
   ```

2. Merge the capability files:
   ```bash
   mcp-generator merge --input graphql.yaml,openapi.yaml --output merged.yaml --strategy rename
   ```

3. Validate the merged file:
   ```bash
   mcp-generator validate --input merged.yaml --strict
   ```

### Generate and Test Capabilities

1. Generate capabilities from an OpenAPI specification:
   ```bash
   mcp-generator openapi \
     --spec examples/swagger2_petstore.json \
     --base-url https://api.example.com/v1 \
     --output petstore.yaml \
     --prefix pet
   ```

2. Validate the generated file:
   ```bash
   mcp-generator validate --input petstore.yaml
   ```

3. Use the capability file with an MCP server:
   ```bash
   mcp-server --capability-file petstore.yaml
   ```

### Filter and Customize Generation

1. Generate capabilities with filtered methods (OpenAPI):
   ```bash
   mcp-generator openapi \
     --spec api.json \
     --base-url https://api.example.com \
     --output filtered.yaml \
     --methods GET,POST \
     --naming method-path
   ```

2. Generate capabilities with filtered services (gRPC):
   ```bash
   mcp-generator grpc \
     --proto service.proto \
     --endpoint localhost:50051 \
     --output filtered.yaml \
     --service-filter UserService \
     --method-filter GetUser,ListUsers
   ```

## Troubleshooting

### Common Issues

1. **Schema Parsing Errors**:
   - Ensure your GraphQL schema, protobuf file, or OpenAPI specification is valid
   - For GraphQL, check if you're using the correct format (SDL or JSON introspection)
   - For OpenAPI, ensure it's a valid v3.0 or Swagger 2.0 specification

2. **Authentication Errors**:
   - Verify that your authentication tokens and credentials are correct
   - Check that you're using the appropriate authentication type for the API

3. **Merge Conflicts**:
   - When merging capability files, use the appropriate merge strategy
   - Consider using `--strategy rename` to avoid conflicts with duplicate tool names

4. **Validation Failures**:
   - Address all validation issues before using the capability file
   - Use non-strict validation during development to see all issues at once

### Debugging Tips

- Use the `--help` option with any command to see detailed usage information
- Examine the generated capability files to ensure they match your expectations
- For configuration file issues, start with the template generated by `init`

## Backward Compatibility

The unified CLI maintains backward compatibility with the existing generator CLIs:
- `graphql-generator`
- `grpc-generator`
- `openapi-generator`

You can continue to use these individual CLIs if you prefer, but the unified CLI provides a more consistent interface and additional features like configuration file support.

## Advanced Usage

### Environment Variable Support

The CLI respects environment variables for sensitive information like authentication tokens:

```bash
export MCP_AUTH_TOKEN="your-secret-token"
mcp-generator graphql --auth-type bearer --auth-token "$MCP_AUTH_TOKEN" ...
```

### Scripting and Automation

The CLI is designed to be easily used in scripts and CI/CD pipelines:

```bash
#!/bin/bash
# Generate capabilities from multiple sources and merge them
mcp-generator graphql --config config.toml
mcp-generator grpc --config config.toml
mcp-generator openapi --config config.toml

# Merge the generated files
mcp-generator merge \
  --input graphql-capabilities.yaml,grpc-capabilities.yaml,openapi-capabilities.yaml \
  --output merged.yaml \
  --strategy rename

# Validate the merged file
mcp-generator validate --input merged.yaml --strict
```

## See Also

- Example configuration file: `examples/mcp-generator-config.yaml`
- Example usage scripts: `examples/generator-examples/`
- GraphQL generator documentation: `docs/graphql_schema_generator.md`
- gRPC generator documentation: `docs/grpc_generator.md`
- OpenAPI generator documentation: `docs/openapi_generator.md`
- MCP specification: `docs/mcp_specification.md`