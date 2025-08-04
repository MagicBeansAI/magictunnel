# MagicTunnel LLM Management CLI

## Overview

The `magictunnel-llm` CLI provides comprehensive management capabilities for all LLM-powered services in MagicTunnel. This unified tool replaces the need for separate management interfaces for each LLM service and includes built-in safety features to protect external MCP content.

## Installation

Build the CLI tool from source:

```bash
cargo build --release --bin magictunnel-llm
```

The binary will be available at `./target/release/magictunnel-llm`.

## Configuration

The CLI uses the same configuration file as the main MagicTunnel service:

```bash
magictunnel-llm --config magictunnel-config.yaml [command]
```

### Configuration Sections Used

The CLI automatically detects and uses the following configuration sections:

- `sampling` - Enhanced tool descriptions
- `elicitation` - Parameter validation
- `prompt_generation` - Tool prompt generation
- `resource_generation` - Tool resource generation
- `content_storage` - Persistent content storage
- `enhancement_storage` - Enhanced tool storage
- `external_content` - External MCP content management
- `external_mcp` - External MCP server configuration

## Service Coverage

| Service | Purpose | CLI Support |
|---------|---------|-------------|
| Sampling Service | Enhanced tool descriptions | ✅ Full |
| Elicitation Service | Parameter validation | ✅ Full |
| Prompt Generator | Tool prompt generation | ✅ Full |
| Resource Generator | Tool resource generation | ✅ Full |
| Enhancement Pipeline | Sampling + Elicitation | ✅ Full |
| Provider Management | LLM provider monitoring | ✅ Full |
| Bulk Operations | Cross-service management | ✅ Full |

## Commands

### Global Options

```bash
magictunnel-llm [OPTIONS] <COMMAND>

Options:
  -c, --config <CONFIG>    Configuration file path [default: magictunnel-config.yaml]
  -v, --verbose           Enable verbose logging
  -f, --format <FORMAT>   Output format (json, yaml, table) [default: table]
  -h, --help              Print help
  -V, --version           Print version
```

### Sampling Commands

Manage enhanced tool descriptions:

```bash
# Generate enhanced description for a tool
magictunnel-llm sampling generate --tool example_tool [--force] [--provider openai]

# List tools with enhanced descriptions
magictunnel-llm sampling list [--filter pattern] [--show-meta]

# Test tool enhancement service connectivity
magictunnel-llm tool-enhancement test [--all-providers]
```

**Examples:**
```bash
# Generate enhanced description with specific provider
magictunnel-llm sampling generate --tool networking_ping --provider openai

# List all enhanced tools with metadata
magictunnel-llm sampling list --show-meta --format json

# Test all configured LLM providers
magictunnel-llm sampling test --all-providers
```

### Elicitation Commands

Manage parameter validation and elicitation:

```bash
# Generate parameter elicitation for a tool
magictunnel-llm elicitation generate --tool example_tool [--type parameter|validation|discovery] [--force]

# Validate tool parameters
magictunnel-llm elicitation validate --tool example_tool [--parameters '{"key": "value"}']

# Test elicitation service
magictunnel-llm elicitation test
```

**Examples:**
```bash
# Generate parameter validation rules
magictunnel-llm elicitation generate --tool api_call --type validation

# Validate specific parameters
magictunnel-llm elicitation validate --tool api_call --parameters '{"url": "https://api.example.com"}'
```

### Prompt Management

Generate and manage tool prompts:

```bash
# Generate prompts for a tool
magictunnel-llm prompts generate --tool example_tool [--types usage,validation,troubleshooting] [--force]

# List generated prompts
magictunnel-llm prompts list [--tool example_tool] [--show-content]

# Export prompts
magictunnel-llm prompts export --tool example_tool --output prompts.json

# Check for external MCP conflicts
magictunnel-llm prompts check-external
```

**Examples:**
```bash
# Generate usage and validation prompts
magictunnel-llm prompts generate --tool http_request --types usage,validation

# Export all prompts for a tool
magictunnel-llm prompts export --tool http_request --output ./exports/http_prompts.json

# Check for external MCP tools before generation
magictunnel-llm prompts check-external
```

### Resource Management

Generate and manage tool resources:

```bash
# Generate resources for a tool
magictunnel-llm resources generate --tool example_tool [--types documentation,examples,schema,configuration] [--force]

# List generated resources
magictunnel-llm resources list [--tool example_tool]

# Export resources
magictunnel-llm resources export --tool example_tool --output ./resources/

# Check for external MCP conflicts
magictunnel-llm resources check-external
```

**Examples:**
```bash
# Generate documentation and examples
magictunnel-llm resources generate --tool database_query --types documentation,examples

# List all resources for a specific tool
magictunnel-llm resources list --tool database_query --format json

# Export resources to directory
magictunnel-llm resources export --tool database_query --output ./db_resources/
```

### Enhancement Management

Manage the sampling + elicitation pipeline:

```bash
# Regenerate all enhancements
magictunnel-llm enhancements regenerate [--tool example_tool] [--force] [--batch-size 10]

# List enhanced tools
magictunnel-llm enhancements list [--detailed]

# Cleanup old enhancements
magictunnel-llm enhancements cleanup [--max-age 30] [--dry-run]

# Show storage statistics
magictunnel-llm enhancements stats

# Export enhancements
magictunnel-llm enhancements export --output ./exports/
```

**Examples:**
```bash
# Regenerate enhancements for all tools
magictunnel-llm enhancements regenerate --batch-size 5

# Clean up enhancements older than 30 days (dry run first)
magictunnel-llm enhancements cleanup --max-age 30 --dry-run
magictunnel-llm enhancements cleanup --max-age 30

# Show detailed enhancement statistics
magictunnel-llm enhancements stats --format json
```

### Provider Management

Monitor and test LLM providers:

```bash
# List configured LLM providers
magictunnel-llm providers list

# Test provider connectivity
magictunnel-llm providers test [--provider openai]

# Show provider usage statistics
magictunnel-llm providers stats [--hours 24]
```

**Examples:**
```bash
# List all providers in table format
magictunnel-llm providers list --format table

# Test specific provider
magictunnel-llm providers test --provider anthropic

# Show 24-hour usage statistics
magictunnel-llm providers stats --hours 24 --format json
```

### Bulk Operations

Cross-service management operations:

```bash
# Regenerate everything for all tools
magictunnel-llm bulk regenerate-all [--include-external] [--force] [--batch-size 5]

# Health check for all LLM services
magictunnel-llm bulk health-check

# Clean up all old generated content
magictunnel-llm bulk cleanup [--max-age 30] [--dry-run]

# Export all LLM-generated content
magictunnel-llm bulk export --output ./full-export/
```

**Examples:**
```bash
# Complete system health check
magictunnel-llm bulk health-check --format json

# Regenerate all content including external tools (with warnings)
magictunnel-llm bulk regenerate-all --include-external --batch-size 3

# Export everything for backup
magictunnel-llm bulk export --output ./backup-$(date +%Y%m%d)/
```

## External MCP Protection

The CLI includes comprehensive protection against accidentally overwriting content from external MCP servers:

### Automatic Detection

The CLI automatically detects tools from external MCP servers and warns before generation:

```bash
⚠️  WARNING: Tool 'external_tool' is from external MCP server
⚠️  Generated prompts may conflict with server-provided content
⚠️  Consider fetching prompts from the external MCP server instead
```

### Check Commands

Before generating content, use check commands to identify external tools:

```bash
# Check for external tools that might conflict
magictunnel-llm prompts check-external
magictunnel-llm resources check-external
```

### Force Override

When necessary, use the `--force` flag to override warnings:

```bash
# Generate content for external tool with explicit override
magictunnel-llm prompts generate --tool external_tool --force
```

## Output Formats

The CLI supports multiple output formats for different use cases:

### Table Format (Default)

Human-readable tables with visual indicators:

```bash
magictunnel-llm providers list
┌──────────────────────────┬──────────┬─────────────────────────────┐
│ Service                  │ Status   │ Details                     │
├──────────────────────────┼──────────┼─────────────────────────────┤
│ Sampling Service         │ ✅ Healthy │ LLM providers configured    │
│ Elicitation Service      │ ✅ Healthy │ LLM providers configured    │
└──────────────────────────┴──────────┴─────────────────────────────┘
```

### JSON Format

Machine-readable for scripts and automation:

```bash
magictunnel-llm providers list --format json
[
  {
    "service": "Sampling Service",
    "status": "Healthy",
    "details": "LLM providers configured"
  },
  {
    "service": "Elicitation Service", 
    "status": "Healthy",
    "details": "LLM providers configured"
  }
]
```

### YAML Format

Configuration-friendly format:

```bash
magictunnel-llm providers list --format yaml
- service: Sampling Service
  status: Healthy
  details: LLM providers configured
- service: Elicitation Service
  status: Healthy
  details: LLM providers configured
```

## Error Handling

The CLI provides detailed error messages and suggestions:

```bash
# Tool not found
❌ Failed to generate prompts for tool 'nonexistent': Tool 'nonexistent' not found

# Service not configured
❌ Prompt generation service not configured. Please check your configuration file.

# External MCP warning
⚠️  Tool 'external_tool' is from external MCP server. Use --force to override.
```

## Logging

Enable verbose logging for troubleshooting:

```bash
magictunnel-llm --verbose bulk health-check
```

This will output detailed debug information about service initialization, API calls, and operation progress.

## Common Workflows

### Daily Operations

```bash
# Morning health check
magictunnel-llm bulk health-check

# Check for new tools needing enhancement
magictunnel-llm enhancements list --detailed

# Generate content for new tools
magictunnel-llm prompts generate --tool new_tool --types usage,validation
magictunnel-llm resources generate --tool new_tool --types documentation,examples
```

### Maintenance Operations

```bash
# Weekly cleanup of old content
magictunnel-llm bulk cleanup --max-age 30 --dry-run
magictunnel-llm bulk cleanup --max-age 30

# Monthly backup export
magictunnel-llm bulk export --output ./backups/monthly-$(date +%Y%m)/

# Quarterly enhancement regeneration
magictunnel-llm enhancements regenerate --force --batch-size 10
```

### External MCP Management

```bash
# Before working with external tools
magictunnel-llm prompts check-external
magictunnel-llm resources check-external

# Fetch content from external servers instead of generating
# (Use external content manager directly for this)
```

## Troubleshooting

### Service Not Available

If a service is not available, check your configuration:

```bash
# Check which services are configured
magictunnel-llm providers list

# Verify configuration file
magictunnel-llm --config magictunnel-config.yaml bulk health-check
```

### External MCP Issues

If external MCP warnings are appearing incorrectly:

1. Verify the tool's source in capability files
2. Check external MCP server configuration
3. Use `--force` flag if generation is intentional

### Performance Issues

For better performance with large numbers of tools:

1. Use smaller batch sizes: `--batch-size 3`
2. Enable verbose logging to identify bottlenecks: `--verbose`
3. Run operations during off-peak hours

## Integration

### Scripting

The CLI is designed for automation:

```bash
#!/bin/bash
# Automated enhancement script

# Health check first
if ! magictunnel-llm bulk health-check --format json | jq -e '.[] | select(.status != "Healthy")' > /dev/null; then
    echo "All services healthy, proceeding with enhancement"
    
    # Generate enhancements for all tools
    magictunnel-llm enhancements regenerate --batch-size 5
    
    # Export for backup
    magictunnel-llm bulk export --output "./backups/$(date +%Y%m%d)/"
else
    echo "Some services unhealthy, skipping enhancement"
    exit 1
fi
```

### Monitoring Integration

Use health checks in monitoring systems:

```bash
# Check service health and exit with appropriate code
magictunnel-llm bulk health-check --format json | jq -e '.[] | select(.status != "Healthy")' && exit 1 || exit 0
```

### CI/CD Integration

Include in deployment pipelines:

```bash
# Pre-deployment health check
magictunnel-llm bulk health-check

# Post-deployment enhancement generation
magictunnel-llm enhancements regenerate --batch-size 10
```

This CLI tool provides enterprise-grade management capabilities for all LLM services while maintaining safety and ease of use.