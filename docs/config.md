# Configuration Guide

## Overview

MagicTunnel uses YAML configuration files to define server settings, tool registries, and smart discovery behavior.

## Basic Configuration

Create `magictunnel-config.yaml`:

```yaml
# Server settings
server:
  host: "127.0.0.1"
  port: 8080
  timeout: 30

# Tool registry configuration  
registry:
  paths: ["./capabilities"]
  hot_reload: true
  validation:
    strict: true

# Smart discovery settings
smart_discovery:
  enabled: true
  tool_selection_mode: "rule_based"  # or "llm_based"
  default_confidence_threshold: 0.5

# Optional: LLM-based tool selection
llm_tool_selection:
  enabled: false
  provider: "openai"  # openai, anthropic, ollama
  model: "gpt-4"
  
# Optional: External MCP server integration
external_mcp:
  enabled: false
  config_file: "external-mcp-servers.yaml"
```

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