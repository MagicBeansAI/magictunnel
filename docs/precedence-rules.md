# Hierarchical Configuration Precedence Rules

This document explains how configuration values are resolved in MagicTunnel's hierarchical configuration system.

## Overview

The hierarchical configuration follows a **strict precedence cascade** across four tiers:

**Tool-Specific** > **Discovery Services** > **MCP Protocol** > **Global Infrastructure** > **System Default**

## Precedence Hierarchy

### Tier 4: Tool-Specific Overrides (Highest Precedence)
```yaml
tools:
  network_ping:
    timeout: 60        # Takes precedence over all lower tiers
    max_retries: 1     # Tool-specific override
    priority: 8        # Tool-specific setting
```

### Tier 3: Discovery Services
```yaml
discovery:
  smart_discovery:
    timeout: 45        # Overrides MCP and Global timeouts
    confidence_threshold: 0.8
  tool_enhancement:
    enabled: true
```

### Tier 2: MCP Protocol Services
```yaml
mcp:
  timeout: 25          # Overrides Global timeout
  max_retries: 2       # MCP-specific setting
  sampling:
    enabled: true
    default_model: "gpt-4"
```

### Tier 1: Global Infrastructure (Base Level)
```yaml
global:
  defaults:
    timeout: 30        # Base timeout for all services
    retry_attempts: 3  # Base retry setting
  server:
    host: "127.0.0.1"
    port: 3001
```

## Resolution Examples

### Example 1: Simple Override Chain

**Configuration:**
```yaml
global:
  defaults:
    timeout: 30
mcp:
  timeout: 25
tools:
  slow_tool:
    timeout: 60
```

**Resolution:**
- `slow_tool` timeout: **60 seconds** (tool-specific override)
- `fast_tool` timeout: **25 seconds** (MCP-level override)
- Any other service timeout: **25 seconds** (MCP-level default)

### Example 2: Mixed Setting Resolution

**Configuration:**
```yaml
global:
  defaults:
    timeout: 30
    max_retries: 3
    connection_timeout: 10
mcp:
  timeout: 25
  max_retries: 2
discovery:
  timeout: 35
tools:
  complex_tool:
    max_retries: 5
```

**Resolution for `complex_tool`:**
- `timeout`: **35 seconds** (Discovery tier - highest available)
- `max_retries`: **5** (Tool-specific override)
- `connection_timeout`: **10 seconds** (Global default - no overrides)

**Resolution for any other tool:**
- `timeout`: **35 seconds** (Discovery tier)
- `max_retries`: **2** (MCP tier)
- `connection_timeout`: **10 seconds** (Global default)

### Example 3: Complex Authentication Resolution

**Configuration:**
```yaml
global:
  auth:
    enabled: true
    server_level:
      ApiKey:
        keys: ["global-key"]
  defaults:
    auth_timeout: 30

mcp:
  auth_timeout: 15
  auth_retry_attempts: 2

tools:
  secure_tool:
    auth_required: true
    auth_timeout: 45
    auth_method: "oauth"
```

**Resolution for `secure_tool`:**
- `auth_timeout`: **45 seconds** (tool-specific override)
- `auth_retry_attempts`: **2** (MCP-level setting)
- `auth_method`: **oauth** (tool-specific override)
- `auth_enabled`: **true** (inherited from global)

## Advanced Precedence Rules

### 1. Partial Configuration Merging

When a configuration object exists at multiple tiers, they are merged with higher tiers taking precedence:

```yaml
global:
  server:
    host: "127.0.0.1"
    port: 3001
    timeout: 30
    websocket: true

mcp:
  server:
    port: 8080      # Overrides global port
    timeout: 25     # Overrides global timeout
    # host and websocket inherited from global
```

**Effective MCP server configuration:**
```yaml
server:
  host: "127.0.0.1"     # From global
  port: 8080            # From mcp (override)
  timeout: 25           # From mcp (override)
  websocket: true       # From global
```

### 2. Array and List Handling

Arrays are **replaced**, not merged:

```yaml
global:
  registry:
    paths: ["./capabilities", "./tools"]

tools:
  custom_tool:
    registry:
      paths: ["./custom-capabilities"]  # Completely replaces global paths
```

### 3. Environment Variable Override Priority

Environment variables have **highest precedence** and override any configuration tier:

```bash
export MAGICTUNNEL_TIMEOUT=120
```

This overrides any `timeout` setting at any tier.

## Tool-Specific Override Categories

### Basic Overrides
```yaml
tools:
  tool_name:
    timeout: 45           # Execution timeout
    max_retries: 5        # Retry attempts
    priority: 8           # Tool priority (0-255)
    enabled: true         # Tool enablement
    hidden: false         # Tool visibility
```

### Advanced Overrides
```yaml
tools:
  tool_name:
    security_level: "privileged"  # Security classification
    auth_required: true          # Authentication requirement
    rate_limit: 60              # Calls per minute
    
    parameters:                  # Tool-specific parameters
      custom_setting: "value"
      
    routing:                     # Tool-specific routing
      routing_type: "external_mcp"
      config:
        server_url: "http://localhost:8081"
```

## Precedence Debugging

### Enable Precedence Logging

```yaml
global:
  logging:
    level: "debug"
    
# Or via environment variable
export RUST_LOG=magictunnel::config=debug
```

### Check Effective Configuration

Use the migration tool to see resolved configuration:

```bash
# Preview effective configuration
cargo run --bin magictunnel-config-migrate -- preview \
  --input magictunnel-config.yaml

# Validate precedence resolution
cargo run --bin magictunnel-config-migrate -- validate \
  --config magictunnel-config.yaml \
  --debug-precedence
```

## Common Precedence Patterns

### Pattern 1: Global Defaults with Service Overrides
```yaml
global:
  defaults:
    timeout: 30
    max_retries: 3

mcp:
  timeout: 15          # Faster for protocol operations

discovery:
  timeout: 45          # Slower for AI operations

tools:
  critical_tool:
    timeout: 120       # Much longer for critical operations
```

### Pattern 2: Security Level Escalation
```yaml
global:
  security:
    default_level: "safe"

discovery:
  security:
    default_level: "restricted"  # Higher security for AI services

tools:
  system_admin:
    security_level: "privileged"  # Highest security for admin tools
```

### Pattern 3: Environment-Specific Overrides
```yaml
global:
  defaults:
    timeout: 30

# Development environment (via env vars)
# export MAGICTUNNEL_ENV=development
# export MAGICTUNNEL_TIMEOUT=60

# Production environment (via env vars)  
# export MAGICTUNNEL_ENV=production
# export MAGICTUNNEL_TIMEOUT=15
```

## Best Practices

### 1. Use Global Defaults Liberally
Set reasonable defaults at the global tier that work for most tools:

```yaml
global:
  defaults:
    timeout: 30
    max_retries: 3
    retry_delay_ms: 1000
```

### 2. Override at the Right Tier
- **MCP overrides**: For protocol-specific optimizations
- **Discovery overrides**: For AI service tuning  
- **Tool overrides**: Only for tools with specific requirements

### 3. Document Override Reasons
```yaml
tools:
  slow_external_api:
    timeout: 120      # External API is slow, needs longer timeout
    max_retries: 1    # Don't retry failed external calls
```

### 4. Test Precedence Resolution
Always test that your overrides work as expected:

```bash
# Test effective configuration
cargo run --bin magictunnel-config-migrate -- validate \
  --config magictunnel-config.yaml
```

## Troubleshooting Precedence Issues

### Issue: Setting Not Taking Effect

1. **Check tier location**: Ensure the setting is at the correct tier
2. **Verify property name**: Check for typos in property names
3. **Environment variables**: Check if env vars are overriding config
4. **Debug logging**: Enable debug logging to see resolution process

### Issue: Unexpected Override Behavior

1. **Check merge rules**: Arrays replace, objects merge
2. **Verify inheritance**: Some settings inherit, others don't
3. **Test isolation**: Test with minimal config to isolate issue
4. **Use validation**: Run config validation with debug output

### Issue: Performance Problems

1. **Check timeout cascade**: Ensure timeouts increase appropriately up the stack
2. **Review retry logic**: Avoid excessive retries at multiple tiers
3. **Monitor resolution**: Watch for expensive precedence resolution

## Migration from Flat Configuration

When migrating from flat configuration, precedence resolution helps maintain compatibility:

### Before (Flat)
```yaml
timeout: 30
mcp_timeout: 25
tool_timeout: 60
```

### After (Hierarchical)
```yaml
global:
  defaults:
    timeout: 30
mcp:
  timeout: 25
tools:
  specific_tool:
    timeout: 60
```

The hierarchical structure makes precedence explicit and maintainable.