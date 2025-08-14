# MCP Elicitation Implementation

## Overview

MagicTunnel provides comprehensive MCP 2025-06-18 Elicitation support, enabling structured data collection and parameter validation through bidirectional communication. The Elicitation system facilitates interactive parameter gathering, schema validation, and metadata extraction with intelligent routing strategies.

**Current Implementation Status**: Parameter validation elicitation complete and working. Advanced MagicTunnel-initiated elicitation features planned for future enhancement.

## Architecture

### Core Components

```
External MCP Server → MagicTunnel Server → Elicitation Service → Client Interaction → Response
                                       ↑
                               Local Processing (when configured)
```

### Key Files

- **`src/mcp/elicitation.rs`** - Complete Elicitation service implementation
- **`src/mcp/types/elicitation.rs`** - MCP 2025-06-18 compliant type definitions
- **`src/mcp/server.rs`** - MCP server handlers for elicitation methods
- **`src/mcp/external_process.rs`** - Bidirectional communication handling
- **`src/mcp/request_forwarder.rs`** - Request forwarding infrastructure

## Implementation Details

### 1. Elicitation Service (`src/mcp/elicitation.rs`)

The core Elicitation service provides structured data collection with validation and metadata extraction:

```rust
pub struct ElicitationService {
    config: ElicitationConfig,
    request_validator: Arc<RequestValidator>,
    schema_validator: Arc<SchemaValidator>,
    metadata_extractor: Arc<MetadataExtractor>,
    audit_logger: Arc<AuditLogger>,
    session_manager: Arc<ElicitationSessionManager>,
}
```

#### Key Features

- **Interactive Parameter Collection**: Multi-step parameter gathering from users
- **Schema Validation**: JSON Schema validation for collected data
- **Metadata Extraction**: Automatic extraction of parameter metadata and validation rules
- **Session Management**: Track elicitation sessions with state management
- **Audit Logging**: Comprehensive audit trail for all elicitation operations
- **Timeout Handling**: Configurable timeouts for elicitation requests

### 2. MCP Protocol Integration

#### Request Handling Methods

The Elicitation service implements the complete MCP 2025-06-18 elicitation protocol:

```rust
// In src/mcp/server.rs
"elicitation/create" => {
    let params = request.params.unwrap_or(json!({}));
    match serde_json::from_value::<ElicitationRequest>(params) {
        Ok(elicitation_request) => {
            match self.handle_elicitation_request(elicitation_request).await {
                Ok(request_id) => {
                    if let Some(ref id) = request.id {
                        self.create_success_response(id, json!({"request_id": request_id}))
                    } else {
                        self.create_error_response(None, McpErrorCode::InvalidRequest, "Request must have an ID")
                    }
                }
                Err(e) => self.create_error_response(
                    request.id.as_ref(),
                    McpErrorCode::InternalError,
                    &format!("Elicitation failed: {}", e)
                ),
            }
        }
        Err(e) => self.create_error_response(
            request.id.as_ref(),
            McpErrorCode::InvalidParams,
            &format!("Invalid elicitation parameters: {}", e)
        ),
    }
}

"elicitation/accept" => {
    let params = request.params.unwrap_or(json!({}));
    match self.handle_elicitation_accept(params).await {
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
            &format!("Elicitation accept failed: {}", e)
        ),
    }
}

"elicitation/reject" => {
    let params = request.params.unwrap_or(json!({}));
    match self.handle_elicitation_reject(params).await {
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
            &format!("Elicitation reject failed: {}", e)
        ),
    }
}

"elicitation/cancel" => {
    let params = request.params.unwrap_or(json!({}));
    match self.handle_elicitation_cancel(params).await {
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
            &format!("Elicitation cancel failed: {}", e)
        ),
    }
}
```

#### Request Types

The Elicitation service supports the complete MCP 2025-06-18 elicitation specification:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationRequest {
    pub message: String,
    pub requested_schema: Option<Value>,
    pub context: Option<ElicitationContext>,
    pub timeout_seconds: Option<u32>,
    pub priority: Option<ElicitationPriority>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationContext {
    pub tool_name: Option<String>,
    pub parameter_name: Option<String>,
    pub current_values: Option<Value>,
    pub validation_errors: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElicitationPriority {
    Low,
    Normal,
    High,
    Critical,
}
```

### 3. Bidirectional Communication

#### External MCP Server Support

External MCP servers can send elicitation requests through all transport mechanisms:

```rust
// In src/mcp/external_process.rs - Bidirectional request handling
if let Ok(request) = serde_json::from_str::<McpRequest>(&line) {
    match request.method.as_str() {
        "elicitation/request" => {
            Self::handle_elicitation_request_from_external(
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

The RequestForwarder trait enables bidirectional elicitation communication:

```rust
#[async_trait]
pub trait RequestForwarder: Send + Sync {
    async fn forward_elicitation_request(
        &self,
        request: ElicitationRequest,
        source_server: &str,
        original_client_id: &str,
    ) -> Result<ElicitationResponse>;
}
```

### 4. Strategy-Based Routing

MagicTunnel supports intelligent routing strategies for elicitation requests:

#### Available Strategies

1. **ClientForwarded** - Forward to the original MCP client for user interaction (only supported strategy)

#### Configuration

```yaml
elicitation:
  enabled: true
  default_strategy: "client_forwarded"  # Usually forward to user for interaction
  
  # Local processing capabilities
  local_processing:
    enabled: true
    auto_fill_common_parameters: true
    validate_schemas: true
    extract_metadata: true
  
  # Session management
  sessions:
    timeout_seconds: 300  # 5 minutes default
    max_concurrent_sessions: 100
    cleanup_interval_seconds: 60
```

### 5. Session Management

#### Elicitation Sessions

```rust
#[derive(Debug, Clone)]
pub struct ElicitationSession {
    pub id: String,
    pub request: ElicitationRequest,
    pub state: ElicitationState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub timeout_at: DateTime<Utc>,
    pub source_server: Option<String>,
    pub client_id: Option<String>,
    pub collected_data: Option<Value>,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElicitationState {
    Created,
    Pending,
    InProgress,
    Completed,
    Rejected,
    Cancelled,
    TimedOut,
}
```

#### Session Lifecycle

```rust
impl ElicitationSessionManager {
    async fn create_session(&self, request: ElicitationRequest) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let timeout_seconds = request.timeout_seconds.unwrap_or(self.config.default_timeout);
        
        let session = ElicitationSession {
            id: session_id.clone(),
            request,
            state: ElicitationState::Created,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            timeout_at: Utc::now() + Duration::seconds(timeout_seconds as i64),
            source_server: None,
            client_id: None,
            collected_data: None,
            validation_errors: Vec::new(),
        };
        
        self.sessions.write().await.insert(session_id.clone(), session);
        Ok(session_id)
    }
    
    async fn update_session_state(&self, session_id: &str, state: ElicitationState) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.state = state;
            session.updated_at = Utc::now();
            Ok(())
        } else {
            Err(ElicitationError::SessionNotFound)
        }
    }
}
```

### 6. Schema Validation

#### JSON Schema Integration

```rust
async fn validate_collected_data(&self, session: &ElicitationSession, data: &Value) -> Result<()> {
    if let Some(schema) = &session.request.requested_schema {
        // Compile JSON schema
        let compiled_schema = jsonschema::JSONSchema::compile(schema)
            .map_err(|e| ElicitationError::InvalidSchema(e.to_string()))?;
        
        // Validate data against schema
        if let Err(errors) = compiled_schema.validate(data) {
            let error_messages: Vec<String> = errors
                .map(|error| error.to_string())
                .collect();
            return Err(ElicitationError::ValidationFailed(error_messages));
        }
    }
    
    Ok(())
}
```

#### Custom Validation Rules

```rust
async fn apply_custom_validation(&self, request: &ElicitationRequest, data: &Value) -> Result<()> {
    // Apply tool-specific validation rules
    if let Some(tool_name) = &request.context.as_ref().and_then(|c| c.tool_name.as_ref()) {
        if let Some(validator) = self.tool_validators.get(tool_name) {
            validator.validate(data).await?;
        }
    }
    
    // Apply parameter-specific validation
    if let Some(param_name) = &request.context.as_ref().and_then(|c| c.parameter_name.as_ref()) {
        self.validate_parameter_constraints(param_name, data).await?;
    }
    
    Ok(())
}
```

### 7. Metadata Extraction

#### Automatic Metadata Generation

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterMetadata {
    pub name: String,
    pub data_type: String,
    pub description: Option<String>,
    pub required: bool,
    pub default_value: Option<Value>,
    pub validation_rules: Vec<ValidationRule>,
    pub constraints: Option<ParameterConstraints>,
    pub examples: Vec<Value>,
}

impl MetadataExtractor {
    async fn extract_parameter_metadata(&self, schema: &Value) -> Result<Vec<ParameterMetadata>> {
        let mut metadata = Vec::new();
        
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            for (name, prop_schema) in properties {
                let param_metadata = ParameterMetadata {
                    name: name.clone(),
                    data_type: self.extract_type(prop_schema),
                    description: prop_schema.get("description").and_then(|d| d.as_str()).map(String::from),
                    required: self.is_required(name, schema),
                    default_value: prop_schema.get("default").cloned(),
                    validation_rules: self.extract_validation_rules(prop_schema),
                    constraints: self.extract_constraints(prop_schema),
                    examples: self.extract_examples(prop_schema),
                };
                metadata.push(param_metadata);
            }
        }
        
        Ok(metadata)
    }
}
```

## Transport Protocol Support

### All Transports Supported

Elicitation works across all MagicTunnel transport mechanisms:

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
        "elicitation/create" => {
            // Unified elicitation handling for all transports
            self.handle_elicitation_request(elicitation_request).await
        }
        "elicitation/accept" => {
            self.handle_elicitation_accept(params).await
        }
        "elicitation/reject" => {
            self.handle_elicitation_reject(params).await
        }
        "elicitation/cancel" => {
            self.handle_elicitation_cancel(params).await
        }
        // ... other methods
    }
}
```

## Client Capability Integration

### Minimum Intersection Advertisement

MagicTunnel only advertises elicitation capabilities that both the server AND client support:

```rust
// In src/mcp/types/capabilities.rs
pub fn get_safe_external_advertisement(&self) -> serde_json::Value {
    // ... other capabilities ...
    
    // Elicitation - only if client supports BOTH create AND accept
    if self.supports_elicitation() {
        capabilities_obj.insert("elicitation".to_string(), json!({
            "create": true,
            "accept": true,
            "reject": true,
            "cancel": true
        }));
    }
    
    safe_capabilities
}

pub fn supports_elicitation(&self) -> bool {
    self.elicitation
        .as_ref()
        .map(|e| e.create && e.accept)  // Requires BOTH create and accept
        .unwrap_or(false)
}
```

### Capability Logging

```rust
pub fn log_capability_advertisement(&self, context: &str, advertised_capabilities: &serde_json::Value) {
    info!("🔧 Capability Advertisement for {}", context);
    
    debug!("📋 Client Capabilities Summary:");
    debug!("   • Elicitation: {}", self.supports_elicitation());
    
    // Log any capabilities NOT advertised due to client limitations
    if !self.supports_elicitation() {
        if let Some(elicit) = &self.elicitation {
            if !elicit.create {
                debug!("   • Elicitation: Client doesn't support elicitation creation");
            } else if !elicit.accept {
                debug!("   • Elicitation: Client doesn't support elicitation acceptance");
            }
        } else {
            debug!("   • Elicitation: Client has no elicitation capability");
        }
    }
}
```

## Configuration

### Complete Configuration Example

```yaml
elicitation:
  enabled: true
  default_strategy: "client_forwarded"
  
  # Local processing capabilities
  local_processing:
    enabled: true
    auto_fill_common_parameters: true
    validate_schemas: true
    extract_metadata: true
    use_llm_for_suggestions: false
  
  # Session management
  sessions:
    timeout_seconds: 300  # 5 minutes default
    max_concurrent_sessions: 100
    cleanup_interval_seconds: 60
    persistent_storage: false
  
  # Schema validation
  validation:
    strict_mode: true
    allow_additional_properties: false
    validate_formats: true
    custom_validators: []
  
  # Security settings
  security:
    max_message_size: 1048576  # 1MB
    max_schema_size: 65536     # 64KB
    require_authentication: false
    allowed_contexts: []  # Empty = allow all
  
  # Audit logging
  audit:
    enabled: true
    log_requests: true
    log_responses: false  # Don't log collected data for privacy
    log_rejections: true
    retention_days: 90
```

## Usage Examples

### 1. External MCP Server Request

An external MCP server requests parameter elicitation:

```json
{
  "jsonrpc": "2.0",
  "method": "elicitation/request",
  "params": {
    "message": "Please provide the database connection parameters",
    "requestedSchema": {
      "type": "object",
      "properties": {
        "host": {
          "type": "string",
          "description": "Database host address"
        },
        "port": {
          "type": "integer",
          "minimum": 1,
          "maximum": 65535,
          "default": 5432
        },
        "database": {
          "type": "string",
          "description": "Database name"
        },
        "username": {
          "type": "string",
          "description": "Database username"
        },
        "password": {
          "type": "string",
          "description": "Database password",
          "format": "password"
        }
      },
      "required": ["host", "database", "username", "password"]
    },
    "context": {
      "toolName": "database_query",
      "parameterName": "connection"
    },
    "timeoutSeconds": 300,
    "priority": "high"
  },
  "id": "elicitation-123"
}
```

### 2. Client Interaction Flow

#### Step 1: Create Response
```json
{
  "jsonrpc": "2.0",
  "id": "elicitation-123",
  "result": {
    "request_id": "elicit-uuid-456"
  }
}
```

#### Step 2: User Accepts and Provides Data
```json
{
  "jsonrpc": "2.0",
  "method": "elicitation/accept",
  "params": {
    "request_id": "elicit-uuid-456",
    "data": {
      "host": "localhost",
      "port": 5432,
      "database": "myapp",
      "username": "app_user",
      "password": "secure_password_123"
    }
  },
  "id": "accept-123"
}
```

#### Step 3: Final Response
```json
{
  "jsonrpc": "2.0",
  "id": "accept-123",
  "result": {
    "status": "completed",
    "data": {
      "host": "localhost",
      "port": 5432,
      "database": "myapp",  
      "username": "app_user",
      "password": "[REDACTED]"  // Sensitive data redacted in logs
    },
    "metadata": {
      "session_id": "elicit-uuid-456",
      "validation_passed": true,
      "processing_time_ms": 145
    }
  }
}
```

### 3. User Rejection Example

```json
{
  "jsonrpc": "2.0",
  "method": "elicitation/reject",
  "params": {
    "request_id": "elicit-uuid-456",
    "reason": "Cannot provide database credentials at this time"
  },
  "id": "reject-123"
}
```

### 4. Cancellation Example

```json
{
  "jsonrpc": "2.0",
  "method": "elicitation/cancel",
  "params": {
    "request_id": "elicit-uuid-456",
    "reason": "Request is no longer needed"
  },
  "id": "cancel-123"
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationError {
    pub code: ElicitationErrorCode,
    pub message: String,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElicitationErrorCode {
    InvalidRequest,
    SessionNotFound,
    SessionExpired,
    ValidationFailed,
    SchemaInvalid,
    TimeoutExceeded,
    UserRejected,
    InternalError,
}
```

### Error Response Examples

#### Validation Error
```json
{
  "jsonrpc": "2.0",
  "id": "accept-123",
  "error": {
    "code": -32602,
    "message": "Elicitation validation failed",
    "data": {
      "error_code": "ValidationFailed",
      "details": {
        "validation_errors": [
          "Property 'port' must be between 1 and 65535",
          "Property 'password' is required but missing"
        ]
      }
    }
  }
}
```

#### Session Timeout Error
```json
{
  "jsonrpc": "2.0",
  "id": "accept-123",
  "error": {
    "code": -32602,
    "message": "Elicitation session expired",
    "data": {
      "error_code": "TimeoutExceeded",
      "details": {
        "session_id": "elicit-uuid-456",
        "timeout_seconds": 300,
        "elapsed_seconds": 315
      }
    }
  }
}
```

## Integration with Tool Enhancement

### Smart Discovery Integration

Elicitation integrates with the smart discovery system for parameter validation:

```rust
// In src/discovery/enhancement.rs
fn should_use_local_elicitation(&self, tool_def: &ToolDefinition) -> bool {
    // Only use elicitation when smart discovery is disabled
    !self.smart_discovery_enabled && 
    self.config.elicitation.enabled &&
    tool_def.supports_elicitation()
}
```

### Parameter Mapping

```rust
async fn elicit_missing_parameters(&self, tool_def: &ToolDefinition, partial_args: &Value) -> Result<Value> {
    let missing_params = self.find_missing_required_parameters(tool_def, partial_args)?;
    
    if !missing_params.is_empty() {
        let elicitation_request = ElicitationRequest {
            message: format!("Please provide missing parameters for tool '{}'", tool_def.name),
            requested_schema: Some(self.build_parameter_schema(&missing_params)?),
            context: Some(ElicitationContext {
                tool_name: Some(tool_def.name.clone()),
                parameter_name: None,
                current_values: Some(partial_args.clone()),
                validation_errors: None,
            }),
            timeout_seconds: Some(180),
            priority: Some(ElicitationPriority::Normal),
            metadata: None,
        };
        
        let response = self.elicitation_service.create_request(elicitation_request).await?;
        // Wait for user to provide missing parameters
        self.wait_for_elicitation_completion(&response.request_id).await
    } else {
        Ok(partial_args.clone())
    }
}
```

## Performance Considerations

### 1. Session Cleanup

Automatic cleanup of expired sessions:

```rust
async fn cleanup_expired_sessions(&self) {
    let now = Utc::now();
    let mut sessions = self.sessions.write().await;
    
    sessions.retain(|_, session| {
        if session.timeout_at < now {
            // Log session timeout
            info!("Cleaning up expired elicitation session: {}", session.id);
            false
        } else {
            true
        }
    });
}
```

### 2. Memory Management

Efficient memory usage for large numbers of concurrent sessions:

```rust
// Use bounded collections to prevent memory exhaustion
const MAX_CONCURRENT_SESSIONS: usize = 1000;

async fn create_session(&self, request: ElicitationRequest) -> Result<String> {
    let current_count = self.sessions.read().await.len();
    if current_count >= MAX_CONCURRENT_SESSIONS {
        return Err(ElicitationError::TooManySessions);
    }
    
    // Continue with session creation...
}
```

### 3. Async Processing

Non-blocking session operations:

```rust
async fn handle_elicitation_request(&self, request: ElicitationRequest) -> Result<String> {
    // Process asynchronously to prevent blocking other requests
    let session_id = self.session_manager.create_session(request).await?;
    
    // Spawn background task for timeout handling
    let session_manager = Arc::clone(&self.session_manager);
    let timeout_session_id = session_id.clone();
    
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(request.timeout_seconds.unwrap_or(300))).await;
        session_manager.timeout_session(&timeout_session_id).await;
    });
    
    Ok(session_id)
}
```

## Testing

### Unit Tests

```bash
# Test elicitation service
cargo test elicitation

# Test session management
cargo test elicitation::session

# Test schema validation
cargo test elicitation::validation

# Test bidirectional communication
cargo test bidirectional_elicitation
```

### Integration Tests

```bash
# Test end-to-end elicitation flow
cargo test --test elicitation_integration

# Test client capability integration
cargo test --test capability_integration
```

### Test Examples

```rust
#[tokio::test]
async fn test_elicitation_create_and_accept() {
    let service = create_test_elicitation_service().await;
    
    // Create elicitation request
    let request = ElicitationRequest {
        message: "Please provide test parameters".to_string(),
        requested_schema: Some(json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "value": {"type": "integer"}
            },
            "required": ["name", "value"]
        })),
        context: None,
        timeout_seconds: Some(60),
        priority: Some(ElicitationPriority::Normal),
        metadata: None,
    };
    
    let session_id = service.create_request(request).await.unwrap();
    
    // Accept with valid data
    let accept_data = json!({
        "name": "test_param",
        "value": 42
    });
    
    let result = service.accept_request(&session_id, accept_data).await.unwrap();
    assert_eq!(result.status, ElicitationStatus::Completed);
}
```

## Monitoring and Metrics

### Key Metrics

- **Active Sessions**: Number of active elicitation sessions
- **Session Duration**: Average time from create to completion/rejection
- **Success Rate**: Percentage of successful elicitations (accepted vs rejected/timeout)
- **Validation Errors**: Frequency of validation failures
- **Response Time**: Time from request to first user interaction
- **Timeout Rate**: Percentage of sessions that timeout

### Monitoring Integration

```rust
// Metrics collection
self.metrics.gauge("elicitation_active_sessions", active_sessions as f64);

self.metrics.increment_counter("elicitation_requests_total", &[
    ("status", "created"),
    ("priority", priority.as_str()),
]);

self.metrics.record_histogram("elicitation_session_duration", duration, &[
    ("outcome", "accepted"),
]);
```

## Security Considerations

### 1. Data Privacy

```rust
// Sensitive data redaction in logs
fn redact_sensitive_data(&self, data: &Value) -> Value {
    let mut redacted = data.clone();
    
    // Redact known sensitive fields
    let sensitive_fields = ["password", "token", "key", "secret", "credential"];
    
    if let Some(obj) = redacted.as_object_mut() {
        for field in sensitive_fields {
            if obj.contains_key(field) {
                obj.insert(field.to_string(), json!("[REDACTED]"));
            }
        }
    }
    
    redacted
}
```

### 2. Input Validation

```rust
async fn validate_elicitation_request(&self, request: &ElicitationRequest) -> Result<()> {
    // Validate message length
    if request.message.len() > self.config.max_message_size {
        return Err(ElicitationError::MessageTooLarge);
    }
    
    // Validate schema size if provided
    if let Some(schema) = &request.requested_schema {
        let schema_size = serde_json::to_string(schema)?.len();
        if schema_size > self.config.max_schema_size {
            return Err(ElicitationError::SchemaTooLarge);
        }
    }
    
    // Validate timeout bounds
    if let Some(timeout) = request.timeout_seconds {
        if timeout > self.config.max_timeout_seconds {
            return Err(ElicitationError::TimeoutTooLarge);
        }
    }
    
    Ok(())
}
```

## Future Enhancements

### MagicTunnel-Initiated Elicitation (Priority Enhancement)

**Current Status**: Parameter validation elicitation complete and working. MagicTunnel can generate elicitation requests when tool parameter validation fails, but does not initiate advanced elicitation scenarios.

**Planned Implementation**: Advanced Elicitation Request Generation

#### Workflow Context Elicitation
1. **User Preferences**: Ask for user preferences during multi-step workflows
2. **Workflow Optimization**: Collect workflow optimization feedback
3. **Ambiguous Intentions**: Request clarification for unclear user intentions
4. **Quality Enhancement**: Collect user feedback for continuous improvement

#### Smart Parameter Suggestions
1. **Context-Aware Suggestions**: Generate intelligent parameter value suggestions based on context
2. **Complex Schema Help**: Provide contextual help for complex parameter schemas
3. **Workflow Recommendations**: Offer workflow-aware parameter recommendations
4. **Result Validation**: Request validation of tool execution results

#### Implementation Approach
1. **Template-Based Generation**: Pre-written elicitation templates with variable substitution
2. **Context Analysis**: Parse workflow context and execution history for relevant elicitation
3. **Smart Discovery Integration**: Leverage existing enhancement metadata for better elicitation
4. **Configuration-Driven**: Comprehensive elicitation trigger configuration system

#### Benefits
- **Improved Accuracy**: Better parameter collection through contextual understanding
- **User Experience**: Proactive assistance with workflow guidance
- **Quality Assurance**: Continuous improvement through user feedback collection
- **Workflow Intelligence**: Smart parameter suggestions based on usage patterns

**Timeline**: Planned after core sampling implementation (3-4 months)

### Additional Planned Features

1. **Multi-Step Elicitation**: Support for complex multi-step parameter collection
2. **Conditional Parameters**: Dynamic schema based on previously collected data
3. **Parameter Suggestions**: AI-powered parameter suggestions based on context
4. **Persistent Sessions**: Optional persistent storage for long-running elicitations
5. **Advanced Validation**: Custom validation rules with JavaScript expressions

### Research Areas

1. **Natural Language Processing**: Better understanding of elicitation requests
2. **User Experience**: Improved interaction patterns for parameter collection
3. **Privacy Preservation**: Enhanced techniques for sensitive data handling
4. **Federated Elicitation**: Cross-server parameter collection workflows

## Conclusion

The MCP Elicitation implementation in MagicTunnel provides comprehensive support for interactive parameter collection and validation. The system integrates seamlessly with the broader MCP ecosystem while providing robust security, session management, and transport protocol support.

The bidirectional communication architecture enables external MCP servers to request parameter elicitation from users through any supported transport mechanism, while the capability integration system ensures that elicitation requests are only sent to clients that support the required operations.

The implementation is designed for production use with comprehensive error handling, audit logging, and performance optimization, making it suitable for enterprise deployments requiring structured data collection and validation.