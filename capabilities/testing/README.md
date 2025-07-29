# Agent Routing Testing Capabilities

This directory contains comprehensive testing capabilities that demonstrate the full power and flexibility of the MCP Proxy Agent Routing system. These capabilities showcase all nine agent types (subprocess, HTTP, gRPC, SSE, GraphQL, LLM, WebSocket, Database, MCP Proxy) with advanced parameter substitution, error handling, and real-world integration scenarios.

## Overview

The Agent Routing system provides a unified interface for executing different types of operations through various agents:

- **Subprocess Agent**: Execute local commands and scripts
- **HTTP Agent**: Make HTTP requests to APIs and web services
- **gRPC Agent**: Call gRPC services and microservices ✅ **NEW**
- **SSE Agent**: Subscribe to Server-Sent Events streams ✅ **NEW**
- **GraphQL Agent**: Execute GraphQL queries and mutations ✅ **NEW**
- **LLM Agent**: Integrate with language models (OpenAI, Anthropic)
- **WebSocket Agent**: Real-time bidirectional communication
- **Database Agent**: Execute SQL queries (PostgreSQL, SQLite) ✅ **NEW**
- **MCP Proxy Agent**: Route to external MCP servers ✅ **NEW**

## Testing Capability Files

### 1. `agent_routing_showcase.yaml`
**Comprehensive demonstration of all agent types with advanced features**

This file showcases:
- **Advanced Parameter Substitution**: Complex templating with conditionals, loops, and environment variables
- **All Nine Agent Types**: Complete examples for subprocess, HTTP, gRPC, SSE, GraphQL, LLM, WebSocket, Database, and MCP Proxy agents
- **Dynamic Configuration**: Runtime configuration based on input parameters
- **Environment Integration**: Using environment variables and system context

**Key Tools:**
- `advanced_file_search`: Subprocess with regex patterns and environment variables
- `api_health_checker`: HTTP with dynamic headers and retry logic
- `intelligent_code_reviewer`: LLM with configurable models and structured output
- `realtime_chat_bot`: WebSocket with session management

### 2. `error_handling_showcase.yaml`
**Robust error handling, timeouts, and edge cases**

This file demonstrates:
- **Timeout Scenarios**: Configurable timeouts and timeout handling
- **Retry Logic**: Automatic retries with backoff strategies
- **Parameter Validation**: Edge cases with special characters and malformed input
- **Network Error Simulation**: Various network failure scenarios
- **Resource Management**: Memory limits and resource exhaustion testing

**Key Tools:**
- `timeout_test_subprocess`: Test subprocess timeout handling
- `http_retry_test`: HTTP retry logic with unreliable endpoints
- `parameter_edge_cases`: Special characters and Unicode handling
- `network_error_simulation`: Simulate various network failures

### 3. `performance_showcase.yaml`
**Performance testing and benchmarking capabilities**

This file includes:
- **Load Testing**: High-throughput scenarios for all agent types
- **Latency Measurement**: Response time analysis
- **Resource Monitoring**: CPU, memory, and I/O utilization
- **Concurrent Execution**: Parallel processing capabilities
- **Stress Testing**: System limits and breaking points

**Key Tools:**
- `subprocess_benchmark`: CPU/IO/memory intensive workloads
- `http_latency_test`: HTTP request latency measurement
- `llm_response_time_test`: LLM performance across different prompt types
- `websocket_connection_test`: WebSocket throughput testing

### 4. `real_world_integrations.yaml`
**Practical, production-ready integration examples**

This file provides:
- **DevOps Integration**: Docker, Kubernetes, CI/CD pipelines
- **Cloud Services**: AWS S3, monitoring systems
- **Database Operations**: Multi-database query execution
- **Notification Systems**: Slack, webhooks, alerting
- **Security Tools**: Code scanning, vulnerability assessment

**Key Tools:**
- `docker_container_manager`: Complete Docker operations
- `kubernetes_deployment`: K8s resource management
- `aws_s3_operations`: S3 file operations
- `slack_notification`: Rich Slack messaging
- `security_scanner`: Multi-tool security scanning

## Parameter Substitution Features

The Agent Routing system supports advanced parameter substitution:

### Basic Substitution
```yaml
command: "echo"
args: ["Hello {{name}}"]
```

### Conditional Logic
```yaml
args: ["{{case_sensitive ? '' : '-i'}}"]
```

### Array Iteration
```yaml
args: ["{{#each info_types}}--include={{this}}{{/each}}"]
```

### Environment Variables
```yaml
env:
  API_KEY: "{{env.OPENAI_API_KEY}}"
```

### Complex JSON Construction
```yaml
body: |
  {
    "event_type": "{{event_type}}",
    "timestamp": "{{now}}",
    "metadata": {{metadata}}
  }
```

## Agent Configuration Examples

### Subprocess Agent
```yaml
routing:
  type: "subprocess"
  config:
    command: "find"
    args: ["{{directory}}", "-name", "{{pattern}}"]
    timeout: 60
    env:
      SEARCH_PATH: "{{directory}}"
```

### HTTP Agent
```yaml
routing:
  type: "http"
  config:
    method: "POST"
    url: "{{api_endpoint}}"
    headers:
      Authorization: "Bearer {{token}}"
      Content-Type: "application/json"
    body: "{{request_payload}}"
    timeout: 30
    retry_attempts: 3
```

### LLM Agent
```yaml
routing:
  type: "llm"
  config:
    provider: "openai"
    model: "{{model_name}}"
    api_key: "{{env.OPENAI_API_KEY}}"
    system_prompt: "You are a {{role}} assistant."
    timeout: 120
```

### WebSocket Agent
```yaml
routing:
  type: "websocket"
  config:
    url: "{{websocket_url}}"
    headers:
      Authorization: "Bearer {{auth_token}}"
    message: |
      {
        "action": "{{action}}",
        "data": {{payload}}
      }
    timeout: 60
```

## Testing Scenarios

### 1. Basic Functionality Testing
Run tools from `agent_routing_showcase.yaml` to verify:
- Parameter substitution works correctly
- All agent types execute successfully
- Output format is as expected

### 2. Error Handling Testing
Use tools from `error_handling_showcase.yaml` to test:
- Timeout scenarios
- Network failures
- Invalid parameters
- Resource limits

### 3. Performance Testing
Execute tools from `performance_showcase.yaml` to measure:
- Response times
- Throughput capabilities
- Resource utilization
- Concurrent execution limits

### 4. Integration Testing
Deploy tools from `real_world_integrations.yaml` to validate:
- External service connectivity
- Authentication mechanisms
- Data format compatibility
- Production readiness

## Environment Variables

Set these environment variables for full functionality:

```bash
# LLM Providers
export OPENAI_API_KEY="your-openai-key"
export ANTHROPIC_API_KEY="your-anthropic-key"

# Cloud Services
export AWS_ACCESS_KEY_ID="your-aws-key"
export AWS_SECRET_ACCESS_KEY="your-aws-secret"
export AWS_DEFAULT_REGION="us-west-2"

# Monitoring
export GRAFANA_API_KEY="your-grafana-key"
export SLACK_WEBHOOK_URL="your-slack-webhook"

# Databases
export DATABASE_URL="postgresql://user:pass@host:port/db"
export PGPASSWORD="your-pg-password"
```

## Usage Examples

### Testing Basic Agent Routing
```bash
# Test subprocess agent
magictunnel call advanced_file_search '{"pattern": "*.rs", "directory": "src"}'

# Test HTTP agent
magictunnel call api_health_checker '{"endpoints": ["https://api.github.com"]}'

# Test LLM agent
magictunnel call intelligent_code_reviewer '{"code": "def hello(): print(\"world\")", "language": "python"}'
```

### Performance Benchmarking
```bash
# Benchmark subprocess performance
magictunnel call subprocess_benchmark '{"workload_type": "cpu_intensive", "iterations": 50}'

# Test HTTP latency
magictunnel call http_latency_test '{"target_url": "https://httpbin.org/get", "request_count": 100}'
```

### Error Scenario Testing
```bash
# Test timeout handling
magictunnel call timeout_test_subprocess '{"delay_seconds": 15}'

# Test network errors
magictunnel call network_error_simulation '{"error_type": "timeout"}'
```

## Best Practices

1. **Parameter Validation**: Always validate input parameters before routing
2. **Timeout Configuration**: Set appropriate timeouts for different operation types
3. **Error Handling**: Implement comprehensive error handling and retry logic
4. **Resource Management**: Monitor and limit resource usage
5. **Security**: Sanitize inputs and use secure authentication methods
6. **Monitoring**: Track performance metrics and error rates
7. **Documentation**: Document all tools and their expected behavior

## Contributing

When adding new testing capabilities:

1. Follow the existing YAML structure and naming conventions
2. Include comprehensive parameter validation
3. Add both success and failure test scenarios
4. Document expected behavior and prerequisites
5. Test with various input combinations
6. Consider security implications of new tools
