# Smart Discovery Examples

This directory contains comprehensive examples demonstrating various use cases and patterns for MagicTunnel's Smart Discovery system.

## Overview

Smart Discovery allows you to use natural language to find and execute the right tool for any task. Instead of memorizing tool names and parameters, you simply describe what you want to accomplish.

## Example Files

### [basic-usage.json](./basic-usage.json)
Fundamental Smart Discovery patterns covering:
- **File Operations**: Reading, writing, searching files
- **HTTP Operations**: API calls, web requests, authentication
- **Database Operations**: Queries, inserts, updates
- **System Operations**: Monitoring, commands, log analysis

### [advanced-workflows.json](./advanced-workflows.json)
Complex real-world workflows including:
- **Configuration Management**: Load, validate, apply settings
- **API Integration Testing**: Connectivity, auth, functional tests
- **Data Processing Pipeline**: ETL workflows with validation
- **CI/CD Pipeline**: Build, test, deploy, monitor
- **Security Audit**: Code scanning, compliance checking

### [error-handling-examples.json](./error-handling-examples.json)
Comprehensive error scenarios and recovery strategies:
- **Tool Not Found**: When no suitable tool exists
- **Missing Parameters**: When required info cannot be extracted
- **Ambiguous Requests**: When multiple tools could match
- **LLM Service Errors**: When parameter extraction fails
- **System Errors**: Configuration and service issues

## Quick Start Examples

### Basic File Operation
```json
{
  "name": "smart_tool_discovery",
  "arguments": {
    "request": "read the config.yaml file"
  }
}
```

### HTTP API Call
```json
{
  "name": "smart_tool_discovery", 
  "arguments": {
    "request": "make GET request to https://api.example.com/health",
    "context": "Service health monitoring"
  }
}
```

### Database Query
```json
{
  "name": "smart_tool_discovery",
  "arguments": {
    "request": "get all users from database",
    "context": "User management dashboard"
  }
}
```

### System Monitoring
```json
{
  "name": "smart_tool_discovery",
  "arguments": {
    "request": "check system health and resource usage"
  }
}
```

## Usage Patterns

### 1. Action-Oriented Requests
Use clear action verbs to describe what you want to do:
- "read [file]"
- "write [data] to [file]"
- "make [HTTP method] request to [URL]" 
- "query [table] for [conditions]"
- "check [service] status"

### 2. Context for Clarity
Add context to provide additional details:
```json
{
  "request": "process user data",
  "context": "Validate customer CSV from S3, transform to JSON, load to PostgreSQL"
}
```

### 3. Preferred Tools
Guide discovery by specifying preferred tools:
```json
{
  "request": "make API call",
  "preferred_tools": ["http_client", "rest_api"],
  "confidence_threshold": 0.6
}
```

### 4. Error Handling
Enable detailed errors for debugging:
```json
{
  "request": "complex operation",
  "include_error_details": true
}
```

## Integration Examples

### Claude Desktop
```javascript
const toolCall = {
  name: "smart_tool_discovery",
  arguments: {
    request: "read the package.json file",
    context: "Need to check project dependencies"
  }
};
```

### HTTP Client
```bash
curl -X POST http://localhost:8080/v1/call_tool \
  -H "Content-Type: application/json" \
  -d '{
    "name": "smart_tool_discovery",
    "arguments": {
      "request": "check if API endpoint is responding",
      "context": "Health check for payment service"
    }
  }'
```

### Python SDK
```python
from magictunnel import SmartDiscoveryClient

client = SmartDiscoveryClient("http://localhost:8080")
response = client.discover_and_execute(
    request="read configuration from YAML file",
    context="Application startup sequence"
)
```

## Best Practices

### Request Writing
1. **Be Specific**: Include file names, URLs, and specific parameters
2. **Use Action Verbs**: Start with clear actions (read, write, send, get)
3. **Provide Context**: Add relevant details that improve understanding
4. **Include Formats**: Specify data formats (JSON, CSV, YAML) when relevant

### Error Handling
1. **Graceful Degradation**: Handle errors with fallback strategies
2. **User Feedback**: Provide clear guidance when requests fail
3. **Retry Logic**: Implement exponential backoff for retryable errors
4. **Debug Mode**: Use detailed errors during development

### Performance
1. **Cache Utilization**: Similar requests benefit from caching
2. **Confidence Tuning**: Use appropriate thresholds for your use case
3. **Batch Operations**: Group related requests when possible
4. **Monitor Metrics**: Track success rates and response times

## Common Workflows

### Configuration Management
1. Read configuration file
2. Validate structure and required fields
3. Apply settings to application
4. Verify configuration is working

### API Testing
1. Check basic connectivity
2. Test authentication
3. Verify core functionality
4. Test error handling

### Data Pipeline
1. Extract data from source
2. Transform and validate
3. Load to destination
4. Generate processing report

### Deployment
1. Build application
2. Run test suite
3. Deploy to staging
4. Validate deployment
5. Deploy to production
6. Monitor health

## Troubleshooting

### "No suitable tool found"
- Check available tools in your registry
- Try lowering confidence threshold
- Rephrase request with different terminology
- Add more context about your goal

### "Missing required parameter"
- Include specific details in your request
- Use context to provide parameter information
- Check error message for required parameters

### "Ambiguous request"
- Be more specific about the operation type
- Use preferred_tools to limit scope
- Add context to clarify intent

### "Request too vague"
- Use clear action verbs
- Include specific objects and targets
- Break complex requests into simpler steps

## Support and Documentation

- **API Reference**: [docs/sections/smart-discovery/API_REFERENCE.md](../../docs/sections/smart-discovery/API_REFERENCE.md)
- **User Guide**: [docs/sections/smart-discovery/USER_GUIDE.md](../../docs/sections/smart-discovery/USER_GUIDE.md)
- **Configuration**: [config.yaml.template](../../config.yaml.template)
- **Integration Tests**: [tests/smart_discovery_integration_test.rs](../../tests/smart_discovery_integration_test.rs)

## Contributing Examples

To add new examples:

1. Create JSON files following the existing structure
2. Include clear descriptions and expected results
3. Add error scenarios and recovery strategies
4. Update this README with new example categories
5. Test examples against current Smart Discovery implementation

## Example Categories

| Category | File | Description |
|----------|------|-------------|
| Basic Usage | `basic-usage.json` | Fundamental patterns for common operations |
| Advanced Workflows | `advanced-workflows.json` | Complex multi-step real-world scenarios |
| Error Handling | `error-handling-examples.json` | Error scenarios and recovery strategies |
| Integration | `integration-examples/` | Platform-specific integration examples |
| Performance | `performance-examples.json` | Optimization patterns and benchmarks |
| Security | `security-examples.json` | Secure usage patterns and best practices |

---

Smart Discovery makes MagicTunnel incredibly powerful and easy to use. These examples should help you get started and explore the full capabilities of the system.