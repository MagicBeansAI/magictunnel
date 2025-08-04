# MagicTunnel MCP Sampling/Elicitation Routing Guide

## Overview

MagicTunnel implements a comprehensive 4-level routing system for MCP (Model Context Protocol) 2025-06-18 sampling and elicitation capabilities. This system provides granular control over how sampling/elicitation requests are processed, offering flexibility from individual tools to organization-wide defaults.

## Architecture

### 4-Level Routing Hierarchy

MagicTunnel processes sampling/elicitation requests through a sophisticated hierarchy:

```
📋 Routing Strategy Resolution (Highest → Lowest Priority)
├── 1. Tool-Level Overrides (ToolDefinition)
│   ├── sampling_strategy: Option<SamplingElicitationStrategy>
│   └── elicitation_strategy: Option<SamplingElicitationStrategy>
├── 2. External MCP Server-Level (ExternalRoutingStrategyConfig)
│   ├── server_strategies: HashMap<String, SamplingElicitationStrategy>
│   └── default_strategy: SamplingElicitationStrategy
├── 3. Server-Level Defaults (SamplingConfig/ElicitationConfig)
│   ├── sampling.default_sampling_strategy
│   └── elicitation.default_elicitation_strategy  
└── 4. Smart Discovery Defaults (SmartDiscoveryConfig)
    ├── smart_discovery.default_sampling_strategy
    └── smart_discovery.default_elicitation_strategy
```

### Core Processing Methods

1. **MagicTunnel-Handled**: MagicTunnel uses its own configured LLMs (OpenAI, Anthropic, Ollama)
2. **Client-Forwarded**: MagicTunnel forwards requests to original client (Claude Desktop, Cursor, etc.)
3. **Hybrid Strategies**: Smart combinations with fallback mechanisms
4. **Strategy Resolution**: Automatic hierarchy-based strategy selection

## Routing Strategies

### 1. MagictunnelHandled
- **Use Case**: MagicTunnel processes requests using its own LLM configuration
- **Behavior**: Uses configured OpenAI, Anthropic, or Ollama APIs
- **Benefits**: Dedicated processing power, configurable models, custom prompts
- **Configuration**:
```yaml
sampling:
  default_sampling_strategy: magictunnel_handled
  llm_config:
    provider: "openai"
    model: "gpt-4"
    api_key_env: "OPENAI_API_KEY"
```

### 2. ClientForwarded
- **Use Case**: Forward requests to the original MCP client
- **Behavior**: Sends JSON-RPC requests back to Claude Desktop, Cursor, etc.
- **Benefits**: Leverages client's LLM without additional API costs
- **Configuration**:
```yaml
elicitation:
  default_elicitation_strategy: client_forwarded
```

### 3. MagictunnelFirst (Recommended)
- **Use Case**: Try MagicTunnel processing first, fallback to client if needed
- **Behavior**: Attempts MagicTunnel-handled, falls back to client-forwarded on failure
- **Benefits**: Best of both worlds with intelligent fallback
- **Configuration**:
```yaml
smart_discovery:
  default_sampling_strategy: magictunnel_first
  default_elicitation_strategy: magictunnel_first
  proxy_timeout_secs: 30
```

### 4. ClientFirst
- **Use Case**: Prefer client processing with MagicTunnel backup
- **Behavior**: Try client-forwarded first, fallback to MagicTunnel-handled on failure
- **Benefits**: Cost-effective with guaranteed fallback
- **Configuration**:
```yaml
external_mcp:
  external_routing:
    sampling:
      default_strategy: client_first
    elicitation:
      default_strategy: client_first
```

### 5. Parallel (Infrastructure Ready)
- **Use Case**: Maximum speed and reliability
- **Behavior**: Send to both MagicTunnel and client simultaneously, return first success
- **Benefits**: Fastest possible response time
- **Status**: 🔄 Infrastructure ready, implementation pending
- **Configuration**:
```yaml
sampling:
  default_sampling_strategy: parallel
  llm_config:
    timeout_seconds: 15  # Shorter timeout for parallel mode
```

### 6. Hybrid (Infrastructure Ready)
- **Use Case**: Best quality responses with intelligent combination
- **Behavior**: Send to both, intelligently combine responses based on confidence
- **Benefits**: Enhanced quality through response merging
- **Status**: 🔄 Infrastructure ready, implementation pending
- **Configuration**:
```yaml
elicitation:
  default_elicitation_strategy: hybrid
  enable_response_combination: true
```

## Configuration Reference

### Complete Configuration Example

```yaml
# MagicTunnel Configuration with 4-Level Routing Hierarchy

# =============================================================================
# LEVEL 4: SMART DISCOVERY DEFAULTS (Lowest Priority)
# =============================================================================
smart_discovery:
  enabled: true
  default_sampling_strategy: magictunnel_first     # Fallback for unspecified tools
  default_elicitation_strategy: client_forwarded   # Fallback for unspecified tools

# =============================================================================
# LEVEL 3: SERVER-LEVEL DEFAULTS  
# =============================================================================
sampling:
  enabled: true
  default_sampling_strategy: magictunnel_handled   # Organization default
  llm_config:
    provider: "openai"
    model: "gpt-4"
    api_key_env: "OPENAI_API_KEY"
    timeout_seconds: 30

elicitation:
  enabled: true
  default_elicitation_strategy: client_first       # Organization default
  max_schema_complexity: "WithArrays"
  default_timeout_seconds: 300

# =============================================================================
# LEVEL 2: EXTERNAL MCP SERVER-LEVEL
# =============================================================================
external_mcp:
  enabled: true
  config_file: "./external_mcp_servers.json"
  
  external_routing:
    enabled: true
    sampling:
      default_strategy: magictunnel_handled        # Default for all external MCP servers
      server_strategies:                           # Per-server overrides
        "filesystem": client_forwarded             # Override for filesystem server
        "database": magictunnel_first              # Override for database server
      fallback_to_magictunnel: true
      max_retry_attempts: 3
      timeout_seconds: 30
      
    elicitation:
      default_strategy: client_forwarded           # Default for all external MCP servers
      server_strategies:                           # Per-server overrides
        "web_search": magictunnel_handled          # Override for web search server
      fallback_to_magictunnel: true
      max_retry_attempts: 2
      timeout_seconds: 60

# =============================================================================
# LEVEL 1: TOOL-LEVEL OVERRIDES (Highest Priority)
# Configured in individual capability YAML files
# =============================================================================
# Example: capabilities/custom/special_tool.yaml
# tools:
#   - name: "special_analysis_tool"
#     description: "Special tool with custom routing"
#     sampling_strategy: parallel          # Tool-specific override
#     elicitation_strategy: hybrid         # Tool-specific override
```

## Usage Examples

### Example 1: High-Performance Setup

**Scenario**: Maximum speed with reliable fallback

```yaml
# Server-level configuration for speed
sampling:
  default_sampling_strategy: magictunnel_first  # Fast MagicTunnel processing first
  llm_config:
    provider: "openai"
    model: "gpt-3.5-turbo"  # Faster model
    timeout_seconds: 15     # Quick timeout

# External server configuration
external_mcp:
  external_routing:
    sampling:
      default_strategy: parallel  # Use parallel when available
      timeout_seconds: 10      # Quick timeout for parallel mode
```

**Expected Behavior**:
- Both local and proxy processing run simultaneously
- First successful response is returned immediately
- 10-second timeout prevents slow proxy responses from blocking

### Example 2: Quality-Focused Setup

**Scenario**: Best possible response quality through combination

```yaml
# Server-level configuration for quality
sampling:
  default_sampling_strategy: hybrid           # Intelligent response combination
  llm_config:
    provider: "openai"
    model: "gpt-4"                           # High-quality model
    timeout_seconds: 45                       # Allow time for quality processing

elicitation:
  default_elicitation_strategy: hybrid        # Intelligent elicitation combination
  enable_hybrid_elicitation: true             # Enable response merging
```

**Expected Behavior**:
- Both MagicTunnel and client processing complete
- Responses are intelligently combined based on confidence scores
- Enhanced metadata provides detailed processing information

### Example 3: Cost-Optimized Setup

**Scenario**: Minimize external API costs while maintaining quality

```yaml
# Server-level configuration for cost optimization
sampling:
  default_sampling_strategy: client_first     # Try free client processing first
  llm_config:
    provider: "ollama"                       # Local LLM for fallback
    model: "llama2"                          # Cost-effective model
    timeout_seconds: 30                       # Quick timeout for cost control

elicitation:
  default_elicitation_strategy: client_forwarded  # Always use client (free)
```

**Expected Behavior**:
- Client processing attempted first (free)
- MagicTunnel LLM only used if client processing fails
- Elicitation always forwarded to client

### Example 4: Enterprise Reliability Setup

**Scenario**: Maximum reliability with multiple fallback layers

```yaml
# Server-level configuration for reliability
sampling:
  default_sampling_strategy: magictunnel_first  # Reliable MagicTunnel processing first
  llm_config:
    provider: "openai"
    model: "gpt-4"
    timeout_seconds: 60                         # Generous timeout for reliability

elicitation:
  default_elicitation_strategy: magictunnel_first  # Reliable MagicTunnel processing first
  max_schema_complexity: "Complex"                # Support complex schemas
  default_timeout_seconds: 300                    # 5-minute timeout for complex elicitation

# External MCP server configuration with fallbacks
external_mcp:
  external_routing:
    sampling:
      default_strategy: magictunnel_handled       # Consistent MagicTunnel processing
      server_strategies:
        "primary-server": magictunnel_first        # Try MagicTunnel first for primary
        "backup-server": client_first              # Try client first for backup
      fallback_to_magictunnel: true
      max_retry_attempts: 5
    elicitation:
      default_strategy: magictunnel_handled       # Consistent MagicTunnel processing
      fallback_to_magictunnel: true
      max_retry_attempts: 3
```

**Expected Behavior**:
- MagicTunnel LLM processing prioritized for consistency
- Multiple retry attempts with different strategies per server
- Ultimate fallback to alternative processing methods
- Detailed metadata for troubleshooting

## Request Flow Patterns

### ClientFirst Strategy Flow
```
Incoming Request
    ↓
Check Client Available?
    ↓ (Yes)
Forward to Client (Claude Desktop, etc.)
    ↓
Client Success? → Return Response
    ↓ (No)
MagicTunnel Fallback Enabled?
    ↓ (Yes)
Process with MagicTunnel LLM → Return Response
```

### Parallel Strategy Flow (Infrastructure Ready)
```
Incoming Request
    ↓
Start MagicTunnel LLM ← → Start Client Processing
    ↓                           ↓
MagicTunnel Complete?        Client Complete?
    ↓ (First Success)           ↓
Return Response (Cancel Other)
```

### Hybrid Strategy Flow (Infrastructure Ready)
```
Incoming Request
    ↓
Start MagicTunnel LLM ↔ Start Client Processing
    ↓                         ↓
Wait for Both to Complete
    ↓
Analyze Response Quality & Confidence
    ↓
Intelligently Combine Responses
    ↓
Return Enhanced Combined Response
```

## Response Metadata

### Enhanced Metadata Fields

Hybrid processing adds comprehensive metadata to responses:

```json
{
  "metadata": {
    "hybrid_processing_mode": "proxy_first_fallback",
    "hybrid_timestamp": "2025-01-15T10:30:00Z",
    "processing_server": "magictunnel-node-1",
    "original_client_id": "claude-desktop",
    "fallback_reason": "Proxy timeout after 30s",
    "super_charged_features": ["context_analysis", "multimodal_support"],
    "local_processing": true,
    "proxy_attempted": true,
    "external_servers_tried": ["filesystem-mcp-server", "github-mcp-server"]
  }
}
```

### Hybrid Combined Metadata

For hybrid strategy responses:

```json
{
  "metadata": {
    "hybrid_processing": "combined",
    "primary_source": "proxy",
    "secondary_source": "local", 
    "combined_responses": 2,
    "confidence_decision": "proxy_higher_confidence",
    "local_metadata": { /* local response metadata */ },
    "proxy_metadata": { /* proxy response metadata */ }
  }
}
```

## Performance Considerations

### Strategy Performance Characteristics

| Strategy | Latency | Reliability | Quality | Cost |
|----------|---------|-------------|---------|------|
| MagictunnelHandled | Medium | High | High | API Cost |
| ClientForwarded | Variable | Medium | High | Free |
| ClientFirst | Low | High | Good | Low |
| MagictunnelFirst | Medium | Highest | High | Medium |
| Parallel | Lowest | Highest | High | Higher |
| Hybrid | Highest | Highest | Highest | Highest |

### Optimization Tips

1. **For Speed**: Use `Parallel` strategy with short timeouts
2. **For Cost**: Use `ClientFirst` or `ClientForwarded` strategies
3. **For Quality**: Use `Hybrid` strategy with quality metrics
4. **For Reliability**: Use `MagictunnelFirst` with fallback to client

## Troubleshooting

### Common Issues and Solutions

#### 1. Slow Response Times
**Problem**: Responses taking too long
**Solutions**:
- Reduce `proxy_timeout_secs`
- Use `Parallel` strategy
- Check proxy server performance

#### 2. High LLM API Costs
**Problem**: Too many LLM API calls from MagicTunnel
**Solutions**:
- Switch to `ClientFirst` or `ClientForwarded`
- Use Ollama for local LLM processing
- Implement request caching and shorter timeouts

#### 3. Low Response Quality
**Problem**: Responses not meeting quality expectations
**Solutions**:
- Use `Hybrid` strategy for combination
- Check proxy server capabilities
- Enable enhanced metadata for analysis

#### 4. Reliability Issues
**Problem**: Frequent processing failures
**Solutions**:
- Use `MagictunnelFirst` or `ClientFirst` for automatic fallbacks
- Configure multiple LLM providers in fallback order
- Increase `max_retry_attempts` in external routing config

## Monitoring and Observability

### Key Metrics to Monitor

1. **Response Times**: Track latency by strategy
2. **Success Rates**: Monitor fallback frequency
3. **Cost Metrics**: Track external API usage
4. **Quality Scores**: Analyze response confidence
5. **Error Rates**: Monitor processing failures

### Log Analysis

Enable debug logging to analyze routing decisions:

```bash
RUST_LOG=magictunnel::mcp::server=debug ./target/release/magictunnel
```

Look for log patterns:
- `🎯 Processing with routing strategy: magictunnel_first`
- `MagicTunnel processing completed successfully`
- `Client forwarding completed, got response`
- `MagicTunnel failed, falling back to client`
- `Hybrid strategy: combining responses from both sources`

## Advanced Configuration

### Custom Processing Logic

For advanced use cases, consider:

1. **Time-based Strategy Switching**: Different strategies for different times
2. **Load-based Fallback**: Switch to local during high load
3. **Quality Thresholds**: Use hybrid only for important requests
4. **Cost Budgets**: Switch to local when budget exceeded

### Integration with External Systems

Hybrid processing integrates with:

- **Monitoring Systems**: Prometheus metrics
- **Logging Platforms**: Structured JSON logs
- **Alert Systems**: Processing failure notifications
- **Cost Tracking**: API usage monitoring

## Migration Guide

### From Basic MCP to Routing Architecture

1. **Start with ClientForwarded**: Maintains existing behavior (free)
2. **Gradually Enable MagicTunnel**: Test MagicTunnel LLM quality
3. **Experiment with MagictunnelFirst**: Get fallback benefits
4. **Optimize Based on Metrics**: Choose best strategy for your use case

### Configuration Migration

```yaml
# Before (Basic MCP)
external_mcp:
  servers:
    - name: "server1"
      endpoint: "ws://localhost:8001/mcp"

# After (Routing Architecture)
external_mcp:
  servers:
    - name: "server1"
      endpoint: "ws://localhost:8001/mcp"
  
  external_routing:
    sampling:
      default_strategy: client_forwarded      # Start with free client processing
      server_strategies:
        "server1": magictunnel_first          # Override for specific server
      fallback_to_magictunnel: true
    elicitation:
      default_strategy: client_forwarded      # Start with free client processing
      fallback_to_magictunnel: true
```

This guide provides comprehensive coverage of MagicTunnel's MCP sampling/elicitation routing architecture, enabling you to configure and optimize request processing with granular control at tool, external MCP server, server, and smart discovery levels for your specific deployment needs.