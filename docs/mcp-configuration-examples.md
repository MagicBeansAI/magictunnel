# MagicTunnel MCP Configuration Examples

> **📝 NOTE**: All configuration examples use **client_forwarded** strategy, which is the only supported routing strategy. MagicTunnel forwards all sampling/elicitation requests to the original client (Claude Desktop, etc.).

## Overview

This document provides practical configuration examples for different MCP (Model Context Protocol) use cases with MagicTunnel's MCP 2025-06-18 enhanced capabilities. Each example shows the correct YAML configuration structure based on the current codebase, including external MCP server management, sampling/elicitation services, and tool enhancement features.

**Key Configuration Areas:**
- **External MCP**: Discovery and management of external MCP servers (replaces legacy mcp_servers/remote_mcp)
- **Sampling Service**: MCP 2025-06-18 sampling capability configuration  
- **Elicitation Service**: Interactive parameter collection during tool execution
- **Tool Enhancement**: AI-powered tool description and metadata enhancement
- **Smart Discovery**: Intelligent tool discovery and routing system

## Configuration Templates

### 1. Development/Testing Setup

**Use Case**: Local development with basic MCP support
**Focus**: Simple setup with external MCP servers and debugging capabilities

```yaml
# magictunnel-dev.yaml
server:
  host: "127.0.0.1"
  port: 3001
  websocket: true
  timeout: 30

# Registry for capability files
registry:
  type: "file"
  paths:
    - "./capabilities"
    - "./capabilities/external-mcp"  # Auto-generated from external MCP servers
  hot_reload: true
  validation:
    strict: true
    allow_unknown_fields: false

# External MCP Server Management (replaces legacy mcp_servers)
external_mcp:
  enabled: true
  config_file: "./external-mcp-servers.yaml"
  capabilities_output_dir: "./capabilities/external-mcp"
  refresh_interval_minutes: 5  # Fast refresh for development

# MCP 2025-06-18 Sampling Service Configuration
sampling:
  enabled: true
  default_sampling_strategy: "client_forwarded"  # Forward to client (only supported strategy)
  llm_config:
    provider: "openai"
    model: "gpt-4o-mini"
    max_tokens: 4000
    api_key_env: "OPENAI_API_KEY"
    temperature: 0.7

# MCP 2025-06-18 Elicitation Service Configuration  
elicitation:
  enabled: true
  default_elicitation_strategy: "client_forwarded"

# Tool Enhancement Service (AI-powered tool descriptions)
tool_enhancement:
  enabled: true
  
  # LLM Configuration for tool enhancement requests
  llm_config:
    provider: "openai"
    model: "gpt-4o-mini"
    max_tokens: 2000

# Smart Discovery Configuration (Tool Selection Only)
smart_discovery:
  enabled: true
  tool_selection_mode: "hybrid"
  default_confidence_threshold: 0.7
  max_tools_to_consider: 10

# Development logging
logging:
  level: "debug"
  format: "pretty"
```

**Expected Behavior**:
- MagicTunnel handles all sampling/elicitation requests using its own LLMs
- No need to bother the original client (Claude Desktop) with questions
- Fast iteration with controlled LLM usage
- Detailed logging for development

### 2. Production Client-Forwarded Setup

**Use Case**: Production environment where original client handles all LLM interactions
**Focus**: Let Claude Desktop/original client handle sampling/elicitation for maximum quality

```yaml
# magictunnel-production.yaml
server:
  host: "0.0.0.0"
  port: 3001
  websocket: true
  timeout: 60  # Longer timeout for production

# Production registry configuration
registry:
  type: "file"
  paths:
    - "./capabilities"
    - "./capabilities/external-mcp"
  hot_reload: false  # Disable for production stability
  validation:
    strict: true
    allow_unknown_fields: false

# External MCP Server Management
external_mcp:
  enabled: true
  config_file: "./external-mcp-servers-production.yaml"
  capabilities_output_dir: "./capabilities/external-mcp"
  refresh_interval_minutes: 60  # Less frequent refresh for stability
  
  # External routing configuration for MCP 2025-06-18
  external_routing:
    enabled: true
    sampling:
      default_strategy: "client_forwarded"
      fallback_to_magictunnel: false  # Never fallback in production
      max_retry_attempts: 3
      timeout_seconds: 30
    elicitation:
      default_strategy: "client_forwarded" 
      fallback_to_magictunnel: false
      max_retry_attempts: 3
      timeout_seconds: 30

# MCP Client Configuration for external servers
mcp_client:
  connect_timeout_secs: 10
  request_timeout_secs: 60
  max_reconnect_attempts: 5
  reconnect_delay_secs: 2
  auto_reconnect: true
  protocol_version: "2025-06-18"
  client_name: "magictunnel-production"
  client_version: "0.3.4"

# Sampling Service - Forward to Client
sampling:
  enabled: true
  default_sampling_strategy: "client_forwarded"  # Always forward to client
  default_elicitation_strategy: "client_forwarded"
  # No llm_config needed since we're forwarding

# Elicitation Service - Forward to Client  
elicitation:
  enabled: true
  default_elicitation_strategy: "client_forwarded"

# Tool Enhancement - Disabled for production performance
tool_enhancement:
  enabled: false  # Disable to reduce overhead

# Smart Discovery with conservative settings (Tool Selection Only)
smart_discovery:
  enabled: true
  tool_selection_mode: "rule_based"  # Faster than hybrid for production
  default_confidence_threshold: 0.8  # Higher threshold for production
  max_tools_to_consider: 5

# Production logging
logging:
  level: "info"
  format: "json"  # Structured logging for production
```

**Expected Behavior**:
- All sampling/elicitation requests forwarded to original client (Claude Desktop)
- Client maintains full control over all LLM interactions
- Multiple server failover for reliability
- Clean separation between MCP tool routing and LLM interactions

### 3. Simplified Production Setup

**Use Case**: Production environment with consistent client-forwarded routing
**Focus**: Simplified configuration with all requests forwarded to original client

```yaml
# magictunnel-simplified-production.yaml
server:
  host: "0.0.0.0"
  port: 3001
  websocket: true
  timeout: 45

# Registry configuration
registry:
  type: "file"
  paths:
    - "./capabilities"
    - "./capabilities/external-mcp"
  hot_reload: true
  validation:
    strict: true
    allow_unknown_fields: false

# External MCP with mixed routing strategies
external_mcp:
  enabled: true
  config_file: "./external-mcp-servers-mixed.yaml"
  capabilities_output_dir: "./capabilities/external-mcp"
  refresh_interval_minutes: 30
  
  # Mixed external routing configuration
  external_routing:
    enabled: true
    sampling:
      default_strategy: "client_forwarded"  # Default: forward to client
      fallback_to_magictunnel: false  # Simplified proxy configuration
    elicitation:
      default_strategy: "client_forwarded"  # Default: MagicTunnel handles elicitation
      fallback_to_magictunnel: false  # Simplified proxy configuration

# MCP Client configuration
mcp_client:
  connect_timeout_secs: 15
  request_timeout_secs: 90
  max_reconnect_attempts: 3
  reconnect_delay_secs: 5
  auto_reconnect: true
  protocol_version: "2025-06-18"
  client_name: "magictunnel-mixed"
  client_version: "0.3.4"

# Sampling Service with mixed strategies
sampling:
  enabled: true
  default_sampling_strategy: "client_forwarded"  # Server-wide default
  default_elicitation_strategy: "client_forwarded"  # Server-wide default
  llm_config:
    provider: "anthropic"
    model: "claude-3-sonnet-20240229"
    max_tokens: 6000
    api_key_env: "ANTHROPIC_API_KEY"
    temperature: 0.7

# Elicitation Service  
elicitation:
  enabled: true
  default_elicitation_strategy: "client_forwarded"

# Tool Enhancement Service enabled for complex scenarios
tool_enhancement:
  enabled: true
  
  # LLM Configuration for tool enhancement requests
  llm_config:
    provider: "anthropic"
    model: "claude-3-sonnet-20240229"
    max_tokens: 4000

# Smart Discovery with full capabilities (Tool Selection Only)
smart_discovery:
  enabled: true
  tool_selection_mode: "hybrid"  # Use AI intelligence
  default_confidence_threshold: 0.75
  max_tools_to_consider: 8

# Balanced logging
logging:
  level: "info"
  format: "json"
```

**Expected Behavior**:
- All sampling/elicitation requests forwarded to original client (Claude Desktop)
- Consistent client-forwarded strategy across all external servers
- Simplified configuration suitable for production deployment
- Clean separation between MCP tool routing and LLM interactions

### 4. Quality-Focused Setup

**Use Case**: Maximum response quality using premium LLMs and comprehensive features
**Focus**: High-quality responses with all enhancement features enabled

```yaml
# magictunnel-quality-focused.yaml
server:
  host: "0.0.0.0"
  port: 3001
  websocket: true
  timeout: 120  # Extended timeout for quality processing

# Registry configuration
registry:
  type: "file"
  paths:
    - "./capabilities"
    - "./capabilities/external-mcp"
  hot_reload: true
  validation:
    strict: true
    allow_unknown_fields: false

# External MCP with quality-first routing
external_mcp:
  enabled: true
  config_file: "./external-mcp-servers-quality.yaml"
  capabilities_output_dir: "./capabilities/external-mcp"
  refresh_interval_minutes: 15  # Frequent refresh for quality updates
  
  # Quality-optimized external routing
  external_routing:
    enabled: true
    sampling:
      default_strategy: "client_forwarded"  # Forward to client (only supported strategy)
      fallback_to_magictunnel: true
      max_retry_attempts: 5  # More attempts for quality
      timeout_seconds: 90
    elicitation:
      default_strategy: "client_forwarded"  # Forward to client (only supported strategy)
      fallback_to_magictunnel: true
      max_retry_attempts: 5
      timeout_seconds: 120

# MCP Client with quality-focused settings
mcp_client:
  connect_timeout_secs: 30
  request_timeout_secs: 180  # Extended for quality processing
  max_reconnect_attempts: 8
  reconnect_delay_secs: 3
  auto_reconnect: true
  protocol_version: "2025-06-18"
  client_name: "magictunnel-quality"
  client_version: "0.3.4"

# Sampling Service with premium models
sampling:
  enabled: true
  default_sampling_strategy: "client_forwarded"  # Forward to client (only supported strategy)
  llm_config:
    provider: "openai"
    model: "gpt-4o"  # Premium model
    max_tokens: 16384  # Large context for detailed responses
    api_key_env: "OPENAI_API_KEY"
    temperature: 0.7
    additional_params:
      top_p: 0.9
      frequency_penalty: 0.1

# Elicitation Service with comprehensive capabilities
elicitation:
  enabled: true
  default_elicitation_strategy: "client_forwarded"

# Tool Enhancement Service - Full AI enhancement
tool_enhancement:
  enabled: true
  
  # LLM Configuration for tool enhancement requests
  llm_config:
    provider: "openai"
    model: "gpt-4o"  # Premium model for enhancement
    max_tokens: 8192  # Large context for detailed descriptions

# Smart Discovery with maximum intelligence (Tool Selection Only)
smart_discovery:
  enabled: true
  tool_selection_mode: "hybrid"  # Full AI intelligence
  default_confidence_threshold: 0.9  # High confidence for quality
  max_tools_to_consider: 15  # Consider more options
  max_high_quality_matches: 8
  high_quality_threshold: 0.95
  use_fuzzy_matching: true
  
  # LLM configuration for smart discovery
  llm_mapper:
    enabled: true
    provider: "openai"
    model: "gpt-4o-mini"  # Fast model for parameter mapping
    api_key_env: "OPENAI_API_KEY"
  
  llm_tool_selection:
    enabled: true
    provider: "anthropic"
    model: "claude-3-sonnet-20240229"
    api_key_env: "ANTHROPIC_API_KEY"
    
  semantic_search:
    enabled: true
    embedding_model: "text-embedding-3-large"  # High-quality embeddings
    api_key_env: "OPENAI_API_KEY"

# Quality-focused logging with detailed information
logging:
  level: "info"
  format: "json"
```

**Expected Behavior**:
- Hybrid processing combines multiple responses for best quality
- Premium LLM models used throughout (GPT-4, Claude-3-Sonnet)
- All enhancement features enabled for maximum capability
- Comprehensive smart discovery with multiple LLM providers
- Extended timeouts to allow thorough processing

### 5. Enterprise Security Setup

**Use Case**: Enterprise environment with security, compliance, and audit requirements
**Focus**: TLS encryption, authentication, audit trails, and security policies

```yaml
# magictunnel-enterprise.yaml
server:
  host: "0.0.0.0"
  port: 3001
  websocket: true
  timeout: 60
  
  # TLS Configuration for secure communication
  tls:
    mode: "application"  # Direct HTTPS/WSS termination
    cert_file: "/etc/ssl/certs/magictunnel.crt"
    key_file: "/etc/ssl/private/magictunnel.key"
    ca_file: "/etc/ssl/certs/corporate-ca.crt"
    behind_proxy: false
    trusted_proxies: []
    min_tls_version: "1.3"  # Enforce TLS 1.3
    hsts_enabled: true
    hsts_max_age: 31536000  # 1 year
    hsts_include_subdomains: true
    hsts_preload: true

# Authentication Configuration
auth:
  enabled: true
  type: "api_key"  # Use API key authentication for enterprise
  api_keys:
    keys:
      - key: "enterprise_admin_key_secure_32_chars_min"
        name: "Enterprise Admin"
        description: "Full administrative access"
        permissions: ["read", "write", "admin"]
        active: true
        expires_at: "2025-12-31T23:59:59Z"
      - key: "enterprise_readonly_key_secure_32_chars"
        name: "Enterprise ReadOnly"
        description: "Read-only access for monitoring"
        permissions: ["read"]
        active: true
    require_header: true
    header_name: "Authorization"
    header_format: "Bearer {key}"

# Registry with security validation
registry:
  type: "file"
  paths:
    - "./capabilities"
    - "./capabilities/external-mcp"
  hot_reload: false  # Disable for security stability
  validation:
    strict: true
    allow_unknown_fields: false

# External MCP with secure routing
external_mcp:
  enabled: true
  config_file: "./external-mcp-servers-enterprise.yaml"
  capabilities_output_dir: "./capabilities/external-mcp"
  refresh_interval_minutes: 120  # Less frequent for security
  
  # Secure external routing - client forwarded only
  external_routing:
    enabled: true
    sampling:
      default_strategy: "client_forwarded"  # Never use internal LLMs
      fallback_to_magictunnel: false  # No fallback for compliance
      max_retry_attempts: 2
      timeout_seconds: 45
    elicitation:
      default_strategy: "client_forwarded"  # Never use internal LLMs
      fallback_to_magictunnel: false  # No fallback for compliance
      max_retry_attempts: 2
      timeout_seconds: 60

# MCP Client with security-focused settings
mcp_client:
  connect_timeout_secs: 15
  request_timeout_secs: 90
  max_reconnect_attempts: 3
  reconnect_delay_secs: 5
  auto_reconnect: true
  protocol_version: "2025-06-18"
  client_name: "magictunnel-enterprise"
  client_version: "0.3.4"

# Sampling Service - Disabled for security
sampling:
  enabled: false  # Disable internal LLM usage for compliance

# Elicitation Service - Disabled for security
elicitation:
  enabled: false  # Disable internal LLM usage for compliance

# Tool Enhancement - Disabled for security
tool_enhancement:
  enabled: false  # Disable to prevent data leakage

# Smart Discovery with minimal features (Tool Selection Only)
smart_discovery:
  enabled: true
  tool_selection_mode: "rule_based"  # No LLM usage
  default_confidence_threshold: 0.9
  max_tools_to_consider: 3  # Conservative limit

# Security Configuration
security:
  enabled: true
  # Security policies would be defined here
  # Note: Actual SecurityConfig structure depends on implementation

# Enterprise logging with audit trails
logging:
  level: "info"
  format: "json"  # Structured logging for SIEM integration
```

**Expected Behavior**:
- All communications encrypted with TLS
- Authentication and authorization required
- Complete audit trails maintained
- Compliance with security standards

### 6. Research/Academic Setup

**Use Case**: Research environment with simplified proxy configuration
**Focus**: Client-forwarded routing with detailed logging for research data collection

```yaml
# magictunnel-research.yaml
server:
  host: "0.0.0.0"
  port: 3001
  websocket: true
  timeout: 300  # Extended timeout for research processing

# Registry configuration for research
registry:
  type: "file"
  paths:
    - "./capabilities"
    - "./capabilities/external-mcp"
    - "./capabilities/research"  # Additional research-specific tools
  hot_reload: true  # Enable for rapid experimentation
  validation:
    strict: false  # Allow some flexibility for research
    allow_unknown_fields: true  # Permit experimental fields

# External MCP for research environments
external_mcp:
  enabled: true
  config_file: "./external-mcp-servers-research.yaml"
  capabilities_output_dir: "./capabilities/external-mcp"
  refresh_interval_minutes: 10  # Frequent refresh for active research
  
  # Research-focused external routing with experimentation
  external_routing:
    enabled: true
    sampling:
      default_strategy: "client_forwarded"  # Simplified proxy configuration
      fallback_to_magictunnel: false
    elicitation:
      default_strategy: "client_forwarded"  # Simplified proxy configuration
      fallback_to_magictunnel: false

# MCP Client for research with extended capabilities
mcp_client:
  connect_timeout_secs: 45
  request_timeout_secs: 300  # 5 minutes for complex research operations
  max_reconnect_attempts: 10
  reconnect_delay_secs: 2
  auto_reconnect: true
  protocol_version: "2025-06-18"
  client_name: "magictunnel-research"
  client_version: "0.3.4"

# Sampling Service with multiple providers for comparison
sampling:
  enabled: true
  default_sampling_strategy: "client_forwarded"  # Forward to client (only supported strategy)
  default_elicitation_strategy: "client_forwarded"
  llm_config:
    provider: "openai"
    model: "gpt-4o"
    max_tokens: 32768  # Maximum context for research
    api_key_env: "OPENAI_API_KEY"
    temperature: 0.8  # Slightly higher for creativity
    additional_params:
      top_p: 0.95
      frequency_penalty: 0.0
      presence_penalty: 0.0

# Elicitation Service with maximum flexibility for research
elicitation:
  enabled: true
  default_elicitation_strategy: "client_forwarded"

# Tool Enhancement Service for research analysis
tool_enhancement:
  enabled: true
  
  # LLM Configuration for tool enhancement requests
  llm_config:
    provider: "anthropic"
    model: "claude-3-opus-20240229"  # Highest quality for research
    max_tokens: 16384

# Smart Discovery with full research capabilities (Tool Selection Only)
smart_discovery:
  enabled: true
  tool_selection_mode: "hybrid"  # Full AI intelligence for research
  default_confidence_threshold: 0.6  # Lower threshold for exploration
  max_tools_to_consider: 20  # Consider many options for research
  max_high_quality_matches: 10
  high_quality_threshold: 0.9
  use_fuzzy_matching: true
  
  # Multiple LLM providers for research comparison
  llm_mapper:
    enabled: true
    provider: "openai"
    model: "gpt-4o-mini"
    api_key_env: "OPENAI_API_KEY"
  
  llm_tool_selection:
    enabled: true
    provider: "anthropic"
    model: "claude-3-opus-20240229"
    api_key_env: "ANTHROPIC_API_KEY"
    
  semantic_search:
    enabled: true
    embedding_model: "text-embedding-3-large"
    api_key_env: "OPENAI_API_KEY"

# Research-specific logging with maximum detail
logging:
  level: "trace"  # Maximum logging detail for research
  format: "json"  # Structured for analysis
```

**Expected Behavior**:
- All requests forwarded to original client for consistency
- Simplified configuration suitable for research environments
- Maximum context sizes and timeouts for complex research tasks
- Flexible validation to support experimental tool development
- Comprehensive logging at trace level for detailed analysis

## Environment-Specific Considerations

### Development Environment
- External MCP enabled with fast refresh intervals (5 minutes)
- Tool enhancement and smart discovery fully enabled for experimentation
- Debug logging with pretty formatting for readability
- Hot reload enabled for rapid iteration
- MagicTunnel handles sampling/elicitation for controlled testing

### Staging Environment
- Production-like external MCP configuration with moderate refresh intervals (30 minutes)
- Client-forwarded strategies to test integration with actual clients
- JSON logging for structured analysis
- Security features enabled for integration testing
- Performance monitoring to validate production readiness

### Production Environment
- Stable external MCP configuration with infrequent refresh (60+ minutes)
- Client-forwarded strategies for optimal user experience
- Hot reload disabled for stability
- Security hardening with TLS, authentication, and audit trails
- Conservative smart discovery settings for reliability

### Research Environment
- Hybrid processing for comprehensive comparison and analysis
- Multiple LLM providers for research comparison
- Trace-level logging for maximum detail
- Flexible validation to support experimental features
- Extended timeouts and context limits for complex research tasks

## Migration Between Configurations

### From Development to Production

1. **Update External MCP Settings**: Increase refresh intervals, disable hot reload
2. **Change Routing Strategies**: Switch from `magictunnel_handled` to `client_forwarded`
3. **Enable Production Security**: Add TLS configuration, authentication, disable internal LLMs
4. **Update Logging**: Switch from `debug`/`pretty` to `info`/`json`
5. **Optimize Smart Discovery**: Use `rule_based` mode, increase confidence thresholds

### From Basic to Advanced

1. **Enable External MCP**: Set `external_mcp.enabled: true` and configure external-mcp-servers.yaml
2. **Add Smart Discovery**: Enable `smart_discovery` with appropriate mode (rule_based → hybrid)  
3. **Configure LLM Services**: Enable `sampling`, `elicitation`, and `tool_enhancement` services
4. **Add External Routing**: Configure `external_routing` with priority orders and fallback strategies
5. **Enable Advanced Features**: Add multiple LLM providers, hybrid processing strategies

## Configuration Validation

### Validation Tools

```bash
# Validate configuration file structure
magictunnel --config magictunnel-production.yaml --validate-config

# Test external MCP server connectivity (if configured)
magictunnel --config magictunnel-production.yaml --test-external-mcp

# Validate sampling/elicitation service configuration
magictunnel --config magictunnel-production.yaml --test-llm-services

# Check smart discovery configuration and test tool matching
magictunnel --config magictunnel-production.yaml --test-discovery
```

### Common Configuration Issues

1. **External MCP File Missing**: Ensure `external-mcp-servers.yaml` exists when `external_mcp.enabled: true`
2. **LLM API Key Missing**: Verify environment variables are set for enabled LLM providers
3. **Strategy Dependencies**: `magictunnel_handled` strategies require `llm_config` to be present
4. **Capability Directory Permissions**: Ensure `capabilities_output_dir` is writable for external MCP generation
5. **Port Conflicts**: Default port 3001 may conflict with other services

## Best Practices

### Configuration Management
- Use environment-specific configuration files (dev, staging, production)
- Version control all configuration changes and external-mcp-servers.yaml files
- Validate configurations with `--validate-config` before deployment
- Document routing strategy decisions and LLM provider choices

### Performance Optimization  
- Choose appropriate routing strategies: `client_forwarded` for production, `hybrid` for research
- Configure reasonable timeouts: external_routing timeouts < mcp_client timeouts
- Use `rule_based` smart discovery for production performance
- Set appropriate `refresh_interval_minutes` based on environment needs

### Security Considerations
- Enable TLS with `mode: "application"` for direct HTTPS/WSS termination
- Use API key authentication for enterprise deployments
- Disable internal LLM services (`sampling`, `elicitation`, `tool_enhancement`) for compliance
- Set `fallback_to_magictunnel: false` in external routing for strict compliance

### External MCP Management
- Use external-mcp-servers.yaml for Claude Desktop format compatibility
- Configure appropriate `capabilities_output_dir` with proper permissions
- Monitor external MCP server health and connectivity
- Test external routing strategies in staging before production deployment

### LLM Service Management
- Separate API keys by environment (OPENAI_API_KEY_DEV, OPENAI_API_KEY_PROD)
- Use conservative token limits and timeouts for cost control
- Monitor LLM API usage manually (no automated cost tracking available)
- Choose models appropriate for use case: gpt-4o-mini for development, gpt-4o for quality

## External MCP Server Configuration

These examples assume you have a corresponding `external-mcp-servers.yaml` file. Here's a basic template:

```yaml
# external-mcp-servers.yaml
mcpServers:
  filesystem:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
    env:
      PATH: "${PATH}"
  
  git:
    command: "uv"  
    args: ["run", "mcp-server-git", "--repository", "."]
    env:
      PATH: "${PATH}"
```

These configuration examples provide a solid foundation for deploying MagicTunnel with MCP 2025-06-18 capabilities in various environments. Customize them based on your specific requirements, external MCP servers, and LLM provider preferences.