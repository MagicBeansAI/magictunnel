# Configuration Guide

## Overview

MagicTunnel uses a **hierarchical YAML configuration** structure to organize settings by concern and precedence level. This provides clean separation between global infrastructure, MCP protocol settings, smart discovery features, and tool-specific overrides.

## Hierarchical Configuration Structure

The new configuration follows a 4-tier hierarchy:

- 🌍 **Global**: Infrastructure, authentication, logging, defaults
- 📡 **MCP**: Protocol services, sampling, elicitation  
- 🤖 **Discovery**: Smart discovery, tool enhancement, visibility
- 🛠️ **Tools**: Individual tool overrides and routing

## Basic Configuration

Create `magictunnel-config.yaml`:

```yaml
# Global infrastructure settings (Tier 1)
global:
  # Server infrastructure
  server:
    host: "127.0.0.1"
    port: 8080
    timeout: 30
    websocket: true
  
  # Tool registry configuration
  registry:
    type: "file"
    paths: ["./capabilities"]
    hot_reload: true
    validation:
      strict: true
      allow_unknown_fields: false
  
  # Authentication and security
  auth:
    enabled: true
    server_level:
      ApiKey:
        keys: ["your-api-key"]
  
  # Default settings (applied across all levels)
  defaults:
    timeout: 30
    retry_attempts: 3
    retry_delay_ms: 1000

# MCP protocol services (Tier 2)  
mcp:
  # Protocol-specific timeout override
  timeout: 25
  max_retries: 2
  
  # MCP client configuration
  mcp_client:
    connection_timeout_ms: 5000
    request_timeout_ms: 30000
  
  # Sampling service (MCP 2025-06-18)
  sampling:
    enabled: true
    default_model: "gpt-4"
  
  # Elicitation service (MCP 2025-06-18)  
  elicitation:
    enabled: true
    default_model: "gpt-4"

# Smart Discovery AI services (Tier 3)
discovery:
  # Smart discovery configuration
  smart_discovery:
    enabled: true
    tool_selection_mode: "hybrid"  # rule_based | semantic | llm_based | hybrid
    default_confidence_threshold: 0.7
  
  # Tool enhancement services
  tool_enhancement:
    enabled: true
    default_model: "gpt-4"
  
  # Tool visibility management
  visibility:
    hide_individual_tools: false
    default_hidden: false
    
# Tool-specific overrides (Tier 4)
tools:
  network_ping:
    timeout: 10      # Override global default
    max_retries: 1   # Override MCP default
    hidden: false    # Override visibility default
```

## Configuration Precedence Rules

The hierarchical configuration follows a **strict precedence cascade**:

**Tool** > **MCP** > **Global** > **Default**

### Precedence Examples

```yaml
global:
  defaults:
    timeout: 30        # Base timeout for all tools
mcp:
  timeout: 25          # Override for MCP protocol operations  
tools:
  slow_tool:
    timeout: 60        # Override for specific tool
```

**Resolution for `slow_tool`**: 60 seconds (tool-specific override)  
**Resolution for any other tool**: 25 seconds (MCP-level override)  
**Resolution with no MCP config**: 30 seconds (global default)

### Override Behavior

- **Tool-level settings** always take highest precedence
- **MCP-level settings** override global defaults for protocol operations
- **Global settings** provide system-wide defaults
- **Missing settings** fall back through the hierarchy

## Migration from Flat Configuration

If you have an existing flat configuration, use the migration tool:

```bash
# Migrate existing config
cargo run --bin magictunnel-config-migrate -- convert \
  --input config.yaml \
  --output magictunnel-config.yaml \
  --validate

# Preview migration without writing
cargo run --bin magictunnel-config-migrate -- preview \
  --input config.yaml

# Validate hierarchical config
cargo run --bin magictunnel-config-migrate -- validate \
  --config magictunnel-config.yaml
```

### Migration Mapping

The migration tool automatically restructures your configuration:

| Flat Structure | Hierarchical Structure |
|----------------|----------------------|
| `server.*` | `global.server.*` |
| `registry.*` | `global.registry.*` |
| `auth.*` | `global.auth.*` |
| `logging.*` | `global.logging.*` |
| `smart_discovery.*` | `discovery.smart_discovery.*` |
| `tool_enhancement.*` | `discovery.tool_enhancement.*` |
| `sampling.*` | `mcp.sampling.*` |
| `elicitation.*` | `mcp.elicitation.*` |

**Note**: The old flat configuration format is no longer supported. Use the migration tool to convert your existing configurations.

## Configuration Sections

### Server Configuration

```yaml
server:
  host: "127.0.0.1"           # Bind address
  port: 8080                  # Port number
  timeout: 30                 # Request timeout in seconds
  websocket: true             # Enable WebSocket support
```

### Registry Configuration

```yaml
registry:
  paths: ["./capabilities"]   # Directories to scan for tool definitions
  hot_reload: true           # Automatically reload on file changes
  validation:
    strict: true             # Strict schema validation
    allow_unknown_fields: false
```

### Smart Discovery Configuration

```yaml
smart_discovery:
  enabled: true
  tool_selection_mode: "rule_based"     # "rule_based" or "llm_based"
  default_confidence_threshold: 0.5     # Minimum confidence for tool selection
  semantic_search:
    enabled: true
    similarity_threshold: 0.7
```

### LLM Integration (Optional)

```yaml
llm_tool_selection:
  enabled: true
  provider: "openai"          # openai, anthropic, ollama
  model: "gpt-4"
  api_key: "${OPENAI_API_KEY}" # Use environment variable
```

## Environment Variables

Set these environment variables for LLM integration:

```bash
# OpenAI
export OPENAI_API_KEY="your-api-key"

# Anthropic
export ANTHROPIC_API_KEY="your-api-key"  

# Ollama (local)
export OLLAMA_BASE_URL="http://localhost:11434"
```

## Advanced Configuration

### External MCP Integration

```yaml
external_mcp:
  enabled: true
  config_file: "external-mcp-servers.yaml"
  capabilities_output_dir: "./generated-capabilities"
  refresh_interval_minutes: 5
```

### Logging Configuration

```yaml
logging:
  level: "info"              # debug, info, warn, error
  format: "json"             # json or pretty
```

## Configuration Validation

Validate your configuration:

```bash
# Check configuration syntax
magictunnel --config magictunnel-config.yaml --validate

# Dry run with configuration
magictunnel --config magictunnel-config.yaml --dry-run
```

## Hot Reload

When `hot_reload: true` is enabled, MagicTunnel automatically detects changes to:
- Configuration files
- Tool definition files in registry paths
- External MCP server configurations

No restart required for most configuration changes.

## Production Configuration

For production deployments:

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  
logging:
  level: "warn"
  format: "json"
  
registry:
  hot_reload: false          # Disable for production
  validation:
    strict: true
```

## Troubleshooting

### Common Issues

1. **Configuration not found**: Ensure file exists and path is correct
2. **Invalid YAML**: Check syntax with online YAML validator
3. **Permission errors**: Verify file permissions for config and capabilities directories
4. **Hot reload not working**: Check file system events support

### Debug Configuration

```bash
# Debug configuration loading
RUST_LOG=debug magictunnel --config your-config.yaml
```