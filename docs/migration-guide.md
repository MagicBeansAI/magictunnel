# Configuration Migration Guide

This guide helps you migrate from the flat configuration structure to the new hierarchical configuration structure introduced in MagicTunnel v0.3.22+.

## Overview

The hierarchical configuration provides:
- **Clear separation of concerns** by organizing settings into logical tiers
- **Precedence-based overrides** for flexible configuration management  
- **Better maintainability** with structured organization
- **Enhanced validation** with tier-specific validation rules

## Migration Process

### 1. Automatic Migration Tool

Use the built-in migration tool for seamless conversion:

```bash
# Basic migration
cargo run --bin magictunnel-config-migrate -- convert \
  --input config.yaml \
  --output magictunnel-config.yaml

# Migration with validation
cargo run --bin magictunnel-config-migrate -- convert \
  --input config.yaml \
  --output magictunnel-config.yaml \
  --validate

# Force overwrite existing output
cargo run --bin magictunnel-config-migrate -- convert \
  --input config.yaml \
  --output magictunnel-config.yaml \
  --force
```

### 2. Preview Changes

Before migrating, preview the conversion:

```bash
# See how your config will be restructured
cargo run --bin magictunnel-config-migrate -- preview \
  --input config.yaml

# Analyze structural differences
cargo run --bin magictunnel-config-migrate -- diff \
  --input config.yaml
```

### 3. Validation

Validate your hierarchical configuration:

```bash
# Validate converted config
cargo run --bin magictunnel-config-migrate -- validate \
  --config magictunnel-config.yaml
```

## Configuration Structure Mapping

### Before (Flat Structure)
```yaml
server:
  host: "0.0.0.0"
  port: 3001
  timeout: 30

registry:
  type: "file"
  paths: ["./capabilities"]
  hot_reload: true

auth:
  enabled: true
  type: "api_key"
  
smart_discovery:
  enabled: true
  tool_selection_mode: "hybrid"
  
sampling:
  enabled: true
  default_model: "gpt-4"
```

### After (Hierarchical Structure)
```yaml
global:
  server:
    host: "0.0.0.0"
    port: 3001
    timeout: 30
  registry:
    type: "file"
    paths: ["./capabilities"]
    hot_reload: true
  auth:
    enabled: true
    server_level:
      ApiKey:
        keys: ["your-key"]
  defaults:
    timeout: 30
    retry_attempts: 3

mcp:
  sampling:
    enabled: true
    default_model: "gpt-4"

discovery:
  smart_discovery:
    enabled: true
    tool_selection_mode: "hybrid"
```

## Tier-by-Tier Migration

### Tier 1: Global Infrastructure
**Purpose**: System-wide infrastructure settings

**Migrated sections:**
- `server` → `global.server`
- `registry` → `global.registry`  
- `auth` → `global.auth` (with structure changes)
- `logging` → `global.logging`
- `security` → `global.security`

**New additions:**
- `global.defaults` - Base settings for all tools
- `global.deployment` - Runtime mode configuration

### Tier 2: MCP Protocol Services  
**Purpose**: MCP 2025-06-18 protocol services

**Migrated sections:**
- `sampling` → `mcp.sampling`
- `elicitation` → `mcp.elicitation`
- `tool_enhancement` → `mcp.tool_enhancement` (MCP-tier only)

**Protocol configuration:**
- `mcp.mcp_client` - Connection settings
- `mcp.streamable_http` - Streamable HTTP transport
- `mcp.timeout` - Protocol-specific timeout override

### Tier 3: Discovery Services
**Purpose**: Smart Discovery and AI services

**Migrated sections:**
- `smart_discovery` → `discovery.smart_discovery`
- `visibility` → `discovery.visibility`
- `conflict_resolution` → `discovery.conflict_resolution`

**AI enhancement:**
- `discovery.tool_enhancement` - Discovery-tier enhancement
- `discovery.enhancement_storage` - Enhancement persistence

### Tier 4: Tool-Specific Overrides
**Purpose**: Individual tool configuration

**Tool overrides:**
```yaml
tools:
  my_tool:
    timeout: 45        # Override global/MCP timeout
    max_retries: 5     # Override global/MCP retries  
    priority: 8        # Tool priority (0-255)
    enabled: true      # Tool enablement
    hidden: false      # Tool visibility
    parameters:        # Tool-specific parameters
      custom_setting: "value"
    routing:           # Tool-specific routing
      routing_type: "external_mcp"
      config:
        server_url: "http://localhost:8081"
```

## Precedence Resolution

### How Precedence Works

1. **Tool-specific setting** (if present) → Use this value
2. **MCP-level setting** (if present) → Use this value  
3. **Global default** (if present) → Use this value
4. **System default** → Use hardcoded default

### Example Resolution

**Configuration:**
```yaml
global:
  defaults:
    timeout: 30
    max_retries: 3
mcp:
  timeout: 25
tools:
  slow_api:
    timeout: 60
  fast_api:
    max_retries: 1
```

**Resolution results:**
- `slow_api` timeout: **60** (tool override)
- `slow_api` max_retries: **3** (global default)
- `fast_api` timeout: **25** (MCP override)
- `fast_api` max_retries: **1** (tool override)
- `other_tool` timeout: **25** (MCP override)
- `other_tool` max_retries: **3** (global default)

## Common Migration Issues

### 1. Authentication Structure Changes

**Old format:**
```yaml
auth:
  enabled: true
  type: "api_key"
  api_keys:
    - "key1"
    - "key2"
```

**New format:**
```yaml
global:
  auth:
    enabled: true
    server_level:
      ApiKey:
        keys: ["key1", "key2"]
```

### 2. Smart Discovery Configuration

**Old format:**
```yaml
smart_discovery:
  enabled: true
  confidence_threshold: 0.7
```

**New format:**
```yaml
discovery:
  smart_discovery:
    enabled: true
    default_confidence_threshold: 0.7
```

### 3. MCP Services Integration

**Old format:**
```yaml
sampling:
  enabled: true
  model: "gpt-4"
```

**New format:**
```yaml
mcp:
  sampling:
    enabled: true
    default_model: "gpt-4"
```

## Troubleshooting Migration

### Validation Errors

If validation fails after migration:

1. **Check field names** - Some fields have been renamed for clarity
2. **Verify structure** - Ensure proper nesting under correct tiers
3. **Review precedence** - Make sure overrides are in the right tier

### Common Field Renames

| Old Name | New Name |
|----------|----------|
| `confidence_threshold` | `default_confidence_threshold` |
| `model` | `default_model` |
| `retry_attempts` | `max_retries` |

### Getting Help

If migration fails:

```bash
# Get detailed error information
cargo run --bin magictunnel-config-migrate -- validate \
  --config your-config.yaml

# Check structure differences
cargo run --bin magictunnel-config-migrate -- diff \
  --input old-config.yaml
```

## Backward Compatibility

**Important**: The flat configuration format is **no longer supported** in v0.3.22+. 

- All configurations must be migrated to hierarchical structure
- The migration tool ensures no data loss during conversion
- All existing functionality is preserved, only structure changes

## Next Steps

After migration:

1. **Test your setup** - Verify all services start correctly
2. **Review precedence** - Ensure override behavior matches expectations  
3. **Update documentation** - Update your team's configuration docs
4. **Archive old config** - Keep old config as backup until fully validated