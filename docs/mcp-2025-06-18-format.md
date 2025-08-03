# MCP 2025-06-18 Enhanced Format

## Overview

MagicTunnel uses the **Enhanced MCP 2025-06-18 Format** as its single, unified format. This comprehensive format provides enterprise-grade features including AI-enhanced discovery, security sandboxing, performance monitoring, and intelligent parameter handling.

## Features

### ✨ AI-Enhanced Discovery
- **Semantic Intelligence**: Advanced keyword and semantic tag matching
- **Confidence Scoring**: Intelligent tool selection with confidence metrics  
- **Complexity Analysis**: Automatic complexity scoring for tool selection
- **Category Classification**: Hierarchical tool categorization

### 🔒 Security Sandboxing
- **5-Level Classification**: Safe, Restricted, Privileged, Dangerous, Blocked
- **Enterprise Security**: Advanced audit trails and compliance features
- **Risk Assessment**: Automatic risk factor identification
- **Environment Controls**: Per-environment security policies

### 📊 Performance Monitoring
- **Execution Tracking**: Detailed performance metrics and analytics
- **Health Checks**: Automated tool health monitoring
- **SLA Targets**: Configurable performance targets and alerting
- **Error Thresholds**: Automatic failure detection and reporting

### 🧠 Parameter Intelligence
- **Smart Defaults**: Intelligent parameter default values
- **Auto-completion**: Parameter value suggestions and completion
- **Validation Rules**: Advanced parameter validation and normalization
- **User Preferences**: Persistent user preference management

### ⚡ Enhanced Execution
- **Granular Progress**: Real-time progress tracking and cancellation
- **Retry Policies**: Configurable retry with exponential backoff
- **Rate Limiting**: Per-tool rate limiting and burst control
- **Intelligent Caching**: TTL-based caching with vary-by parameters

## Enhanced MCP 2025-06-18 Format Structure
```yaml
enhanced_tools:
  - name: "get_weather"
    core:
      description: "AI-enhanced weather service with MCP 2025-06-18 compliance"
      input_schema:
        type: "object"
        properties:
          location:
            type: "string"
            description: "City name, coordinates, or airport code"
            examples: ["New York", "40.7128,-74.0060", "JFK"]
          units:
            type: "string"
            enum: ["metric", "imperial", "kelvin"]
            default: "metric"
            description: "Temperature units"
        required: ["location"]
    
    # AI-Enhanced Discovery Configuration
    discovery:
      semantic_tags: ["weather", "temperature", "forecast", "climate", "conditions"]
      keywords: ["weather", "temperature", "rain", "snow", "forecast", "climate"]
      categories: ["weather", "environment", "information"]
      ai_description: "Comprehensive weather information service with location intelligence"
      complexity_score: 0.3
      confidence_boost: 0.1
      
    # Security Sandboxing Configuration  
    security:
      classification: "safe"      # safe|restricted|privileged|dangerous|blocked
      requires_approval: false
      audit_trail: true
      allowed_environments: ["development", "staging", "production"]
      risk_factors: ["external_api"]
      
    # Execution Configuration
    execution:
      timeout_seconds: 30
      retry_policy:
        max_attempts: 3
        backoff_strategy: "exponential"
        initial_delay_ms: 1000
      rate_limiting:
        max_calls_per_minute: 60
        burst_limit: 10
      caching:
        enabled: true
        ttl_seconds: 300
        vary_by: ["location", "units"]
        
    # Performance Monitoring
    monitoring:
      track_execution: true
      collect_metrics: true
      health_check_enabled: true
      sla_target_ms: 2000
      error_threshold: 0.05
      
    # Parameter Intelligence
    parameters:
      location:
        smart_defaults: true
        validation_rules: ["non_empty", "max_length:100"]
        auto_complete: true
        normalization: "trim_whitespace"
      units:
        smart_defaults: true
        user_preference_key: "weather_units"
        
    # Enterprise Integration
    enterprise:
      cost_center: "weather_services"
      business_impact: "low"
      compliance_tags: ["data_privacy", "gdpr"]
      
    # Tool Routing (same as legacy format)
    routing:
      type: "rest"
      url: "https://api.weather.com/v1/current"
      method: "GET"
      headers:
        "X-API-Key": "${WEATHER_API_KEY}"
        "User-Agent": "MagicTunnel/2025.06.18"
      query_params:
        q: "${location}"
        units: "${units}"
    
    # Visibility (supports both formats)
    hidden: false
```

## Configuration

Enhanced format is always enabled by default in your `config.yaml`:

# MCP 2025-06-18 Features
mcp_2025:
  enabled: true                     # Enable MCP 2025-06-18 enhanced features
  
  # Enhanced Cancellation Support
  cancellation:
    enabled: true                   # Enable granular cancellation support
    timeout_handling: "graceful"    # Timeout handling: graceful|immediate
    progress_updates: true          # Send progress updates during long operations
    
  # Runtime Tool Description Validation
  validation:
    enabled: true                   # Enable runtime validation
    startup_validation: true        # Validate all tools at startup
    on_demand_validation: true      # Enable on-demand validation via API
    schema_enforcement: "strict"    # Schema enforcement: strict|loose
    
  # Security Sandboxing
  security:
    enabled: true                   # Enable security sandboxing
    default_level: "safe"           # Default security level
    enterprise_mode: false         # Enable enterprise security features
    audit_logging: true            # Enable security audit logging
    
  # Performance Monitoring
  monitoring:
    enabled: true                   # Enable performance monitoring
    execution_tracking: true        # Track tool execution metrics
    performance_analytics: true     # Collect performance analytics
    health_checks: true            # Enable health check endpoints
```

## Security Classifications

### Safe (Default)
- **Risk Level**: Low
- **Examples**: Read-only operations, information gathering, public APIs
- **Restrictions**: None
- **Approval**: None required

### Restricted  
- **Risk Level**: Medium
- **Examples**: Limited writes, authenticated API calls, data transformations
- **Restrictions**: Rate limiting, logging required
- **Approval**: None required

### Privileged
- **Risk Level**: High  
- **Examples**: File system operations, database writes, system commands
- **Restrictions**: Enhanced logging, approval workflows
- **Approval**: Optional (configurable)

### Dangerous
- **Risk Level**: Very High
- **Examples**: System administration, destructive operations, sensitive data access
- **Restrictions**: Full audit trail, mandatory approval
- **Approval**: Always required

### Blocked
- **Risk Level**: Prohibited
- **Examples**: Malicious operations, policy violations
- **Restrictions**: Complete blocking
- **Approval**: Never allowed

## API-to-Tool Generation

All CLI generators produce Enhanced MCP 2025-06-18 format by default:

### GraphQL to Tools
```bash
cargo run --bin graphql_generator -- \
  --schema schema.graphql \
  --endpoint https://api.example.com/graphql \
  --output weather-graphql.yaml
```

### gRPC to Tools  
```bash
cargo run --bin grpc_generator -- \
  --proto weather.proto \
  --endpoint localhost:50051 \
  --output weather-grpc.yaml
```

### OpenAPI to Tools
```bash
cargo run --bin magictunnel-cli openapi \
  --spec openapi.yaml \
  --output weather-openapi.yaml
```

### Merge Tools
```bash
cargo run --bin magictunnel-cli merge \
  --input dir1/ dir2/ \
  --output merged.yaml
```

## Enterprise Features

### Cost Center Tracking
```yaml
enterprise:
  cost_center: "api_services"
  business_impact: "medium"         # low|medium|high|critical
  budget_category: "external_apis"
  owner_team: "platform_engineering"
```

### Compliance Integration
```yaml
enterprise:
  compliance_tags: ["gdpr", "hipaa", "pci_dss"]
  data_classification: "confidential"
  retention_policy: "7_years"
  audit_requirements: ["access_logs", "data_lineage"]
```

### Performance SLAs
```yaml
monitoring:
  sla_target_ms: 1000              # 1 second target response time
  availability_target: 0.999        # 99.9% availability target
  error_threshold: 0.01             # 1% error rate threshold
  alert_on_breach: true            # Alert when SLA is breached
```

## Migration Notes

MagicTunnel now exclusively uses the Enhanced MCP 2025-06-18 format. All existing capability files in the codebase have been migrated to this format, providing:

- **Consistent Experience**: All tools use the same advanced feature set
- **No Format Confusion**: Single format eliminates complexity
- **Future-Proof**: Built on the latest MCP 2025-06-18 specification
- **Enterprise Ready**: Full security, monitoring, and intelligence features

## Environment Variables

Control enhanced format features via environment variables:

```bash
# MCP 2025-06-18 Features (Enhanced Format Always Enabled)
export MCP_2025_ENABLED="true"
export MCP_2025_CANCELLATION_ENABLED="true"
export MCP_2025_VALIDATION_ENABLED="true"
export MCP_2025_SECURITY_ENABLED="true"
export MCP_2025_MONITORING_ENABLED="true"
export MCP_2025_ENTERPRISE_MODE="false"
```

## Best Practices

### 1. Security First
- Always classify tools with appropriate security levels
- Use `safe` classification for read-only operations
- Require approval for `dangerous` operations
- Enable audit logging for compliance

### 2. Performance Optimization
- Set realistic SLA targets based on tool complexity
- Enable caching for frequently used, stable data
- Configure appropriate retry policies
- Monitor execution metrics regularly

### 3. Intelligent Discovery
- Use descriptive semantic tags and keywords
- Set appropriate complexity scores (0.0-1.0)
- Provide clear AI descriptions
- Categorize tools logically

### 4. Parameter Intelligence
- Enable smart defaults for better user experience
- Add validation rules to prevent errors
- Use normalization for consistent data
- Implement auto-completion where helpful

### 5. Enterprise Integration
- Assign appropriate cost centers for tracking
- Tag tools with compliance requirements
- Set business impact levels accurately
- Define clear ownership and responsibility

## Troubleshooting

### Format Detection Issues
```bash
# Check format compatibility
cargo run --bin magictunnel-cli validate \
  --file capability-file.yaml \
  --verbose
```

### Security Classification Problems  
```bash
# Verify security settings
cargo run --bin magictunnel-cli security-check \
  --file capability-file.yaml \
  --level strict
```

### Performance Monitoring Issues
```bash
# Check monitoring configuration
cargo run --bin magictunnel-cli monitor \
  --tool tool_name \
  --metrics execution,performance
```

## Advanced Usage

### Custom Validation Rules
```yaml
parameters:
  email:
    validation_rules: 
      - "email_format"
      - "domain_whitelist:company.com,partner.org"
      - "max_length:100"
  password:
    validation_rules:
      - "min_length:8"
      - "require_uppercase"
      - "require_numbers"
      - "require_symbols"
```

### Dynamic Security Classification
```yaml
security:
  classification: "dynamic"          # Runtime classification based on input
  classification_rules:
    - condition: "input.action == 'read'"
      level: "safe"
    - condition: "input.action == 'write'"  
      level: "restricted"
    - condition: "input.action == 'delete'"
      level: "dangerous"
```

### Conditional Execution
```yaml
execution:
  conditions:
    - name: "business_hours"
      expression: "time.hour >= 9 && time.hour <= 17"
      action: "allow"
    - name: "weekend_block"
      expression: "time.weekday == 'saturday' || time.weekday == 'sunday'"
      action: "block"
      message: "Tool disabled on weekends"
```

## API Reference

The enhanced format exposes additional API endpoints:

### Tool Metadata
```bash
GET /api/tools/{tool_name}/metadata
```

### Security Information
```bash
GET /api/tools/{tool_name}/security
```

### Performance Metrics
```bash
GET /api/tools/{tool_name}/metrics
```

### Validation Status
```bash
POST /api/tools/{tool_name}/validate
```

For complete API documentation, see [API Reference](api.md).