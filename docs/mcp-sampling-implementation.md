# MCP Sampling Implementation

## Overview

MagicTunnel provides comprehensive MCP 2025-06-18 Sampling support, enabling external MCP servers to request LLM-powered content generation through the unified proxy system. The Sampling implementation supports bidirectional communication, multi-provider LLM integration, and intelligent routing strategies.

**Current Implementation Status**: Proxy-only implementation complete with external MCP server support. Advanced MagicTunnel-initiated sampling features planned for future enhancement.

## Architecture

### Core Components

```
External MCP Server → MagicTunnel Server → Sampling Service → LLM Provider → Response
                                       ↑
                                Client Forwarding (when configured)
```

### Key Files

- **`src/mcp/sampling.rs`** - Complete Sampling service implementation (1,800+ lines)
- **`src/mcp/types/sampling.rs`** - MCP 2025-06-18 compliant type definitions
- **`src/mcp/server.rs`** - MCP server handlers for `sampling/createMessage`
- **`src/mcp/external_process.rs`** - Bidirectional communication handling
- **`src/mcp/request_forwarder.rs`** - Request forwarding infrastructure

## Implementation Details

### 1. Sampling Service (`src/mcp/sampling.rs`)

The core Sampling service provides comprehensive LLM integration with multi-provider support:

```rust
pub struct SamplingService {
    config: SamplingConfig,
    llm_client: LLMClient,
    rate_limiter: Arc<Mutex<RateLimiter>>,
    request_validator: Arc<RequestValidator>,
    content_filter: Arc<ContentFilter>,
    audit_logger: Arc<AuditLogger>,
}
```

#### Key Features

- **Multi-Provider Support**: OpenAI, Anthropic, Ollama, and custom API endpoints
- **Rate Limiting**: Configurable rate limiting per user/IP with token bucket algorithm
- **Content Filtering**: Content moderation and safety checks for requests and responses
- **Audit Logging**: Comprehensive audit trail for all sampling operations
- **Security Integration**: Request validation and content sanitization
- **Performance Optimization**: Async processing with configurable timeouts

### 2. MCP Protocol Integration

#### Request Handling

```rust
// In src/mcp/server.rs
"sampling/createMessage" => {
    let params = request.params.unwrap_or(json!({}));
    match serde_json::from_value::<SamplingRequest>(params) {
        Ok(sampling_request) => {
            match self.handle_sampling_request(sampling_request).await {
                Ok(response) => {
                    if let Some(ref id) = request.id {
                        self.create_success_response(id, json!(response))
                    } else {
                        self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                    }
                }
                Err(e) => self.create_error_response(
                    request.id.as_ref(),
                    McpErrorCode::InternalError,
                    &format!("Sampling failed: {}", e)
                ),
            }
        }
        Err(e) => self.create_error_response(
            request.id.as_ref(),
            McpErrorCode::InvalidParams,
            &format!("Invalid sampling parameters: {}", e)
        ),
    }
}
```

#### Request Types

The Sampling service supports the complete MCP 2025-06-18 `sampling/createMessage` specification:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingRequest {
    pub max_tokens: Option<u32>,
    pub messages: Vec<SamplingMessage>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub metadata: Option<Value>,
    pub model_preferences: Option<ModelPreferences>,
}
```

### 3. Bidirectional Communication

#### External MCP Server Support

External MCP servers can send sampling requests through stdio, WebSocket, or HTTP transports:

```rust
// In src/mcp/external_process.rs - Bidirectional request handling
if let Ok(request) = serde_json::from_str::<McpRequest>(&line) {
    match request.method.as_str() {
        "sampling/createMessage" => {
            Self::handle_sampling_request_from_external(
                &request_forwarder,
                &stdin_sender,
                request,
                &server_name,
                &original_client_id,
            ).await;
        }
        // ... other methods
    }
}
```

#### Request Forwarding

The RequestForwarder trait enables bidirectional communication:

```rust
#[async_trait]
pub trait RequestForwarder: Send + Sync {
    async fn forward_sampling_request(
        &self,
        request: SamplingRequest,
        source_server: &str,
        original_client_id: &str,
    ) -> Result<SamplingResponse>;
}
```

### 4. Strategy-Based Routing

MagicTunnel supports intelligent routing strategies for sampling requests:

#### Available Strategies

1. **ClientForwarded** - Forward to the original MCP client (only supported strategy)

#### Configuration

```yaml
sampling:
  enabled: true
  default_sampling_strategy: "client_forwarded"
  providers:
    openai:
      api_key: "${OPENAI_API_KEY}"
      model: "gpt-4"
    anthropic:
      api_key: "${ANTHROPIC_API_KEY}"
      model: "claude-3-sonnet-20240229"
    ollama:
      base_url: "http://localhost:11434"
      model: "llama2"
```

### 5. LLM Provider Integration

#### OpenAI Integration

```rust
async fn make_openai_request(&self, request: &SamplingRequest) -> Result<SamplingResponse> {
    let client = reqwest::Client::new();
    let openai_request = json!({
        "model": self.config.openai.model,
        "messages": request.messages,
        "max_tokens": request.max_tokens,
        "temperature": request.temperature,
        "top_p": request.top_p,
        "stop": request.stop
    });
    
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", self.config.openai.api_key))
        .json(&openai_request)
        .send()
        .await?;
        
    // Process response and convert to SamplingResponse
    self.process_openai_response(response).await
}
```

#### Multi-Provider Fallback

```rust
async fn execute_with_fallback(&self, request: &SamplingRequest) -> Result<SamplingResponse> {
    for provider in &self.config.providers {
        match provider {
            LLMProvider::OpenAI(config) => {
                if let Ok(response) = self.make_openai_request(request).await {
                    return Ok(response);
                }
            }
            LLMProvider::Anthropic(config) => {
                if let Ok(response) = self.make_anthropic_request(request).await {
                    return Ok(response);
                }
            }
            LLMProvider::Ollama(config) => {
                if let Ok(response) = self.make_ollama_request(request).await {
                    return Ok(response);
                }
            }
        }
    }
    Err(SamplingError::AllProvidersFailed)
}
```

## Transport Protocol Support

### All Transports Supported

Sampling works across all MagicTunnel transport mechanisms:

1. **Stdio** - Process-based communication (Claude Desktop, Cursor)
2. **WebSocket** - Real-time bidirectional communication 
3. **HTTP-SSE** - Server-Sent Events (deprecated)
4. **Streamable HTTP** - NDJSON streaming (MCP 2025-06-18 preferred)

### Unified Handler

All transports use the same `server.handle_mcp_request()` method, ensuring consistent behavior:

```rust
// All transports route through this unified handler
pub async fn handle_mcp_request(&self, request: McpRequest) -> Result<Option<String>> {
    match request.method.as_str() {
        "sampling/createMessage" => {
            // Unified sampling handling for all transports
            self.handle_sampling_request(sampling_request).await
        }
        // ... other methods
    }
}
```

## Security Features

### 1. Request Validation

```rust
async fn validate_request(&self, request: &SamplingRequest) -> Result<()> {
    // Validate message content
    for message in &request.messages {
        self.validate_message_content(&message.content).await?;
    }
    
    // Check rate limits
    self.check_rate_limit(&request.metadata).await?;
    
    // Validate token limits
    if let Some(max_tokens) = request.max_tokens {
        if max_tokens > self.config.max_tokens_limit {
            return Err(SamplingError::TokenLimitExceeded);
        }
    }
    
    Ok(())
}
```

### 2. Content Filtering

```rust
async fn apply_content_filter(&self, request: &SamplingRequest) -> Result<()> {
    // Check for prohibited content
    for message in &request.messages {
        if self.content_filter.is_prohibited(&message.content).await? {
            return Err(SamplingError::ContentFiltered);
        }
    }
    
    // Scan for PII and sensitive data
    self.content_filter.scan_for_pii(request).await?;
    
    Ok(())
}
```

### 3. Audit Logging

```rust
async fn log_sampling_request(&self, request: &SamplingRequest, response: &SamplingResponse) {
    let audit_entry = AuditEntry {
        timestamp: Utc::now(),
        event_type: "sampling_request",
        user_id: request.metadata.get("user_id").cloned(),
        request_id: request.metadata.get("request_id").cloned(),
        provider: response.metadata.get("provider").cloned(),
        token_count: response.usage.total_tokens,
        success: true,
    };
    
    self.audit_logger.log(audit_entry).await;
}
```

## Configuration

### Complete Configuration Example

```yaml
sampling:
  enabled: true
  default_strategy: "hybrid"
  
  # Rate limiting
  rate_limiting:
    enabled: true
    requests_per_minute: 60
    burst_size: 10
  
  # Content filtering
  content_filtering:
    enabled: true
    block_pii: true
    content_policies: ["safe", "educational"]
  
  # LLM Providers
  providers:
    openai:
      enabled: true
      api_key: "${OPENAI_API_KEY}"
      model: "gpt-4"
      max_tokens: 4096
      timeout_seconds: 30
    
    anthropic:
      enabled: true
      api_key: "${ANTHROPIC_API_KEY}"
      model: "claude-3-sonnet-20240229"
      max_tokens: 4096
      timeout_seconds: 30
    
    ollama:
      enabled: true
      base_url: "http://localhost:11434"
      model: "llama2"
      timeout_seconds: 60
  
  # Security settings
  security:
    max_message_size: 1048576  # 1MB
    max_messages_per_request: 100
    require_authentication: false
    allowed_models: ["gpt-4", "claude-3-sonnet-20240229", "llama2"]
  
  # Audit logging
  audit:
    enabled: true
    log_requests: true
    log_responses: false  # Don't log response content for privacy
    retention_days: 90
```

## Usage Examples

### 1. External MCP Server Request

An external MCP server can send sampling requests:

```json
{
  "jsonrpc": "2.0",
  "method": "sampling/createMessage",
  "params": {
    "maxTokens": 1000,
    "messages": [
      {
        "role": "user",
        "content": {
          "type": "text",
          "text": "Explain quantum computing in simple terms"
        }
      }
    ],
    "temperature": 0.7,
    "metadata": {
      "source": "external_server",
      "priority": "normal"
    }
  },
  "id": "sampling-123"
}
```

### 2. Client Response

MagicTunnel responds with processed LLM output:

```json
{
  "jsonrpc": "2.0",
  "id": "sampling-123",
  "result": {
    "role": "assistant",
    "content": {
      "type": "text",
      "text": "Quantum computing is like having a very special kind of computer..."
    },
    "model": "gpt-4",
    "stopReason": "endTurn",
    "usage": {
      "inputTokens": 15,
      "outputTokens": 245,
      "totalTokens": 260
    },
    "metadata": {
      "provider": "openai",
      "processing_time_ms": 1234,
      "strategy": "magictunnel_handled"
    }
  }  
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingError {
    pub code: SamplingErrorCode,
    pub message: String,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SamplingErrorCode {
    InvalidRequest,
    ContentFiltered,
    RateLimitExceeded,
    ProviderUnavailable,
    TokenLimitExceeded,
    InternalError,
}
```

### Error Response Example

```json
{
  "jsonrpc": "2.0",
  "id": "sampling-123", 
  "error": {
    "code": -32602,
    "message": "Sampling failed: Rate limit exceeded",
    "data": {
      "error_code": "RateLimitExceeded",
      "details": {
        "limit": 60,
        "window": "1 minute",
        "retry_after": 45
      }
    }
  }
}
```

## Performance Considerations

### 1. Async Processing

All LLM requests are processed asynchronously to prevent blocking:

```rust
async fn handle_sampling_request(&self, request: SamplingRequest) -> Result<SamplingResponse> {
    // Process multiple providers concurrently when using parallel strategy
    let futures = providers.iter().map(|provider| {
        self.make_provider_request(provider, &request)
    });
    
    let results = futures::future::join_all(futures).await;
    self.select_best_response(results).await
}
```

### 2. Connection Pooling

HTTP clients use connection pooling for better performance:

```rust
lazy_static! {
    static ref HTTP_CLIENT: Client = Client::builder()
        .pool_max_connections_per_host(10)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
}
```

### 3. Caching

Response caching for repeated requests:

```rust
async fn get_cached_response(&self, request: &SamplingRequest) -> Option<SamplingResponse> {
    let cache_key = self.generate_cache_key(request);
    self.response_cache.get(&cache_key).await
}
```

## Monitoring and Metrics

### Key Metrics

- **Request Rate**: Sampling requests per second
- **Response Time**: Average LLM response time by provider
- **Success Rate**: Percentage of successful requests
- **Token Usage**: Token consumption by provider and model
- **Error Rate**: Error frequency by type
- **Provider Health**: Availability and performance of each LLM provider

### Monitoring Integration

```rust
// Metrics collection
self.metrics.increment_counter("sampling_requests_total", &[
    ("provider", provider_name),
    ("model", model_name),
    ("status", "success"),
]);

self.metrics.record_histogram("sampling_response_time", response_time, &[
    ("provider", provider_name),
]);
```

## Testing

### Unit Tests

```bash
# Test sampling service
cargo test sampling

# Test specific provider integration
cargo test sampling::providers::openai

# Test bidirectional communication
cargo test bidirectional_sampling
```

### Integration Tests

```bash
# Test end-to-end sampling flow
cargo test --test sampling_integration

# Test with real LLM providers (requires API keys)
OPENAI_API_KEY="your-key" cargo test --test sampling_e2e
```

## Future Enhancements

### MagicTunnel-Initiated Sampling (Priority Enhancement)

**Current Status**: Proxy-only implementation complete. MagicTunnel can forward external MCP sampling requests but does not initiate its own sampling requests.

**Planned Implementation**: LLM-Assisted Sampling Request Generation

#### Intelligent Triggers
1. **Error-Based Triggers**: Tool execution failures that could benefit from LLM assistance
2. **Parameter Ambiguity**: When parameter mapping fails or produces low confidence results
3. **Workflow Optimization**: During sequential tool execution chains that need guidance
4. **Performance Enhancement**: For improving tool execution quality and results
5. **Security Assistance**: Security-related scenarios requiring LLM guidance

#### Implementation Approach
1. **Rule-Based Triggers**: Pattern matching on error messages and execution context (no LLM calls)
2. **Smart Discovery Integration**: Leverage existing confidence scores and enhancement data
3. **Template-Based Request Generation**: Pre-written templates with variable substitution
4. **Configuration-Driven**: Comprehensive trigger configuration system
5. **Rate Limiting**: Prevent excessive LLM usage with intelligent throttling

#### Benefits
- **Proactive Assistance**: Help users before they get stuck with workflow guidance
- **Error Recovery**: Intelligent error resolution with contextual LLM assistance
- **Quality Improvement**: Continuous improvement of tool execution results and user experience

**Timeline**: 2-3 months after core features complete

### Additional Planned Features

1. **Streaming Responses**: Support for streaming LLM responses
2. **Custom Model Fine-tuning**: Integration with custom model endpoints
3. **Multi-modal Support**: Image and audio content in sampling requests
4. **Advanced Caching**: Semantic caching for similar requests
5. **Load Balancing**: Intelligent load balancing across multiple provider instances

### Research Areas

1. **Prompt Optimization**: Automatic prompt optimization for better results
2. **Cost Optimization**: Dynamic provider selection based on cost/performance
3. **Quality Scoring**: Automatic quality assessment of LLM responses
4. **Federated Learning**: Privacy-preserving model improvement

## Conclusion

The MCP Sampling implementation in MagicTunnel provides enterprise-grade LLM integration with comprehensive security, monitoring, and performance features. The bidirectional communication architecture enables seamless integration with external MCP servers while maintaining compatibility with all transport protocols.

The system is designed for production use with robust error handling, comprehensive logging, and flexible configuration options. The multi-provider architecture ensures reliability and allows for cost optimization and performance tuning.