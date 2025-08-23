# Centralized Audit System

## Overview

MagicTunnel's Centralized Audit System provides enterprise-grade audit logging capabilities that go beyond traditional application logging. This system is designed to meet compliance, security, and operational requirements for production deployments.

## Key Features

### 🏗️ **Enterprise Architecture**
- **Centralized Collection**: Single audit collector for all components
- **Non-blocking Processing**: Async event processing with backpressure control
- **Multi-backend Storage**: Memory, File, Database, External systems support
- **Real-time Streaming**: WebSocket-based live monitoring
- **Structured Events**: Consistent schema across all components

### 🔒 **Security & Compliance**
- **Immutable Audit Trail**: Tamper-evident logging with integrity checks
- **Structured Data**: Searchable and filterable audit events
- **Retention Management**: Configurable data retention policies
- **Multi-tenancy**: Isolated audit trails for different organizations
- **Sensitive Data Protection**: Automatic masking and sanitization

### 🚀 **Performance & Scalability**
- **High Throughput**: Queue-based processing with batching
- **Resource Management**: Configurable worker threads and queue limits
- **Compression**: Optional compression for storage efficiency
- **Caching**: Intelligent caching for frequent queries
- **Health Monitoring**: Built-in metrics and health checks

## Architecture

### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Application   │────│ Audit Collector │────│   Storage       │
│   Components    │    │                 │    │   Backends      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌─────────────────┐
                       │  Real-time      │
                       │  Streaming      │
                       └─────────────────┘
```

1. **AuditCollector**: Central coordinator with async processing
2. **Event Schemas**: Structured audit event definitions
3. **Storage Abstraction**: Pluggable storage backends
4. **Streaming Service**: Real-time WebSocket event broadcasting
5. **Integration Helpers**: Easy service integration patterns

### Event Flow

```
Application → Event Queue → Batch Processor → Storage Backend
     │                                            │
     └──────────────── Real-time Stream ──────────┘
```

## Configuration

### Basic Configuration

```yaml
security:
  audit:
    enabled: true
    storage:
      type: file
      directory: "./logs/audit"
      max_file_size: 104857600
      max_files: 10
      compress: true
    event_types:
      - authentication
      - authorization
      - tool_execution
      - security_violation
      - oauth_flow
      - mcp_connection
      - admin_action
    retention_days: 90
```

### Advanced Configuration

```yaml
security:
  audit:
    enabled: true
    
    # Multi-backend Storage
    storage:
      type: hybrid
      backends:
        - type: file
          directory: "./logs/audit"
          max_file_size: 100000000
        - type: database
          connection_string: "postgresql://user:pass@localhost/audit"
          table_name: "audit_events"
        - type: external
          endpoint: "https://siem.company.com/audit"
          api_key: "${SIEM_API_KEY}"
          format: elasticsearch
      primary: 0  # Use file as primary
    
    # Real-time Streaming
    streaming:
      enabled: true
      max_connections: 1000
      buffer_size: 5000
      heartbeat_interval: 30
    
    # Performance Tuning
    performance:
      max_queue_size: 50000
      worker_threads: 4
      batch_size: 500
      flush_interval_secs: 3
      compression: true
    
    # Enterprise Features
    multi_tenant: true
    integrity_checks: true
```

## Event Types

### Standard Event Types

| Event Type | Description | Use Cases |
|------------|-------------|-----------|
| `authentication` | User login/logout events | Security monitoring, compliance |
| `authorization` | Permission checks and failures | Access control auditing |
| `tool_execution` | Tool calls and results | Usage tracking, compliance |
| `security_violation` | Security policy violations | Incident response, forensics |
| `oauth_flow` | OAuth 2.1 authentication flows | Identity management auditing |
| `mcp_connection` | MCP client/server connections | Infrastructure monitoring |
| `admin_action` | Administrative operations | Change management, compliance |
| `system_health` | Performance and health metrics | Operational monitoring |
| `error_occurred` | Application errors and exceptions | Troubleshooting, reliability |

### Custom Event Types

You can define custom event types for organization-specific requirements:

```rust
use magictunnel::security::audit::AuditEventType;

let custom_event = AuditEventType::Custom("data_export".to_string());
```

## Storage Backends

### File Storage

**Best for**: Small to medium deployments, development environments

```yaml
storage:
  type: file
  directory: "./logs/audit"
  max_file_size: 104857600  # 100MB
  max_files: 10
  compress: true
  sync_interval_secs: 5
```

**Features**:
- Automatic file rotation
- Gzip compression
- Local filesystem storage
- JSONL format for easy processing

### Database Storage

**Best for**: Production deployments, complex querying requirements

```yaml
storage:
  type: database
  connection_string: "postgresql://user:pass@localhost/audit"
  table_name: "audit_events"
  batch_size: 100
  connection_pool_size: 10
```

**Supported Databases**:
- PostgreSQL (recommended)
- MySQL/MariaDB
- SQLite (development only)

### External Storage

**Best for**: SIEM integration, centralized logging infrastructure

```yaml
storage:
  type: external
  endpoint: "https://elastic.company.com/_bulk"
  api_key: "${ELASTICSEARCH_API_KEY}"
  format: elasticsearch
  batch_size: 1000
  timeout_secs: 30
```

**Supported Formats**:
- Elasticsearch
- Splunk HEC
- Syslog (RFC 5424)
- JSON Lines
- Custom formats

### Hybrid Storage

**Best for**: Enterprise deployments requiring redundancy

```yaml
storage:
  type: hybrid
  backends:
    - type: file
      directory: "./logs/audit"
    - type: external
      endpoint: "https://siem.company.com/audit"
  primary: 0  # File storage as primary
```

## Real-time Streaming

### WebSocket API

Connect to audit stream for real-time monitoring:

```javascript
const ws = new WebSocket('ws://localhost:3001/audit/stream');

// Subscribe to specific event types
ws.send(JSON.stringify({
  type: 'subscribe',
  filters: {
    event_types: ['security_violation', 'authentication'],
    severities: ['error', 'critical']
  }
}));

// Receive real-time events
ws.onmessage = (event) => {
  const auditEvent = JSON.parse(event.data);
  console.log('Audit Event:', auditEvent);
};
```

### Subscription Filters

Filter events in real-time:

```json
{
  "event_types": ["authentication", "tool_execution"],
  "components": ["mcp_server", "oauth_manager"],
  "severities": ["warning", "error", "critical"],
  "user_ids": ["admin", "service_account"],
  "search_text": "failed login",
  "correlation_id": "req_12345"
}
```

## Integration Guide

### Service Integration

Implement the `AuditIntegration` trait for automatic audit logging:

```rust
use magictunnel::security::audit::integration::AuditIntegration;
use magictunnel::implement_audit_integration;

struct MyService;

implement_audit_integration!(MyService, "my_service");

impl MyService {
    async fn perform_action(&self) -> Result<()> {
        // Log the action
        self.emit_audit_event(
            AuditEventType::AdminAction,
            "User action performed".to_string(),
            AuditSeverity::Info,
            None,
        ).await?;
        
        // Perform the actual action
        Ok(())
    }
}
```

### Request Context Integration

Use request-scoped audit context for tracing:

```rust
use magictunnel::security::audit::integration::{AuditMiddleware, RequestAuditContext};

// Create request context
let context = AuditMiddleware::create_request_context(
    "req_12345".to_string(),
    Some("user123".to_string()),
    Some("session456".to_string()),
    Some("192.168.1.1".to_string()),
    Some("Mozilla/5.0...".to_string()),
);

// Log events with context
context.log_event(
    "auth_service",
    AuditEventType::Authentication,
    "User logged in successfully".to_string(),
    AuditSeverity::Info,
    None,
).await?;

// Log request completion
context.log_request_completion(
    "api_server",
    true,
    Some(200),
    None,
).await?;
```

### Helper Macros

Use convenient macros for common scenarios:

```rust
use magictunnel::{audit_log, audit_event};

// Simple audit logging
audit_log!(
    AuditEventType::ToolExecution,
    "smart_discovery",
    "Tool executed successfully"
);

// Audit logging with metadata
audit_log!(
    AuditEventType::SecurityViolation,
    "allowlist_service",
    "Tool access denied",
    "tool_name" => "dangerous_tool",
    "user_id" => "user123",
    "reason" => "not in allowlist"
);
```

## Query and Analysis

### Event Structure

All audit events follow a consistent structure:

```json
{
  "id": "evt_01234567-89ab-cdef-0123-456789abcdef",
  "timestamp": "2025-01-20T10:30:00.000Z",
  "event_type": "tool_execution",
  "component": "smart_discovery",
  "message": "Tool executed successfully",
  "metadata": {
    "user_id": "user123",
    "session_id": "session456",
    "request_id": "req_12345",
    "source_ip": "192.168.1.1",
    "custom": {
      "tool_name": "get_weather",
      "execution_time_ms": 1500
    }
  },
  "payload": {
    "tool_name": "get_weather",
    "parameters": {"location": "New York"},
    "result": "success"
  },
  "severity": "info",
  "correlation_id": "flow_789"
}
```

### Querying Events

```rust
use magictunnel::security::audit::storage::AuditQuery;

let query = AuditQuery {
    event_types: Some(vec!["authentication".to_string()]),
    start_time: Some(Utc::now() - Duration::hours(24)),
    end_time: Some(Utc::now()),
    user_ids: Some(vec!["user123".to_string()]),
    limit: Some(100),
    ..Default::default()
};

let events = audit_storage.query_events(&query).await?;
```

## Security Considerations

### Data Protection

- **Sensitive Data Masking**: Automatic detection and masking of sensitive information
- **Encryption at Rest**: AES-256-GCM encryption for file storage
- **Transport Security**: TLS 1.3 for external storage connections
- **Access Controls**: Role-based access to audit data

### Integrity Verification

```rust
// Enable integrity checks in configuration
audit:
  integrity_checks: true
```

Features:
- **Cryptographic Hashing**: SHA-256 checksums for each event
- **Chain Verification**: Linked event chains for tamper detection
- **Periodic Validation**: Automated integrity verification
- **Audit Trail**: Immutable audit of audit access

### Compliance Features

- **GDPR**: User data anonymization and deletion capabilities
- **SOX**: Financial transaction auditing and retention
- **HIPAA**: Healthcare data access logging and protection
- **PCI DSS**: Payment processing audit requirements

## Monitoring and Alerting

### Health Monitoring

```rust
// Check audit system health
let health = audit_collector.health_check().await?;
println!("Audit System Health: {:?}", health);
```

Health checks include:
- Queue depth and processing rates
- Storage backend connectivity
- Event processing latency
- Error rates and failure patterns

### Metrics and Statistics

```rust
// Get detailed statistics
let stats = audit_collector.get_stats().await;
println!("Events per second: {}", stats.events_per_second);
println!("Total events: {}", stats.total_events);
println!("Queue depth: {}", stats.queue_depth);
```

Available metrics:
- Event throughput (events/second)
- Processing latency (avg/max)
- Storage utilization
- Error rates by component
- Real-time client connections

### Alerting Integration

Configure alerts for critical events:

```yaml
audit:
  alerting:
    enabled: true
    channels:
      - type: webhook
        url: "https://alerts.company.com/audit"
        events: ["security_violation", "system_failure"]
      - type: email
        recipients: ["security@company.com"]
        events: ["emergency_lockdown"]
```

## Performance Tuning

### Queue Configuration

```yaml
performance:
  max_queue_size: 10000      # Increase for high-volume environments
  worker_threads: 4          # Scale with CPU cores
  batch_size: 100           # Optimize for storage backend
  flush_interval_secs: 5    # Balance latency vs. efficiency
```

### Storage Optimization

- **File Storage**: Use SSD storage, enable compression
- **Database Storage**: Optimize indices, partition tables
- **External Storage**: Configure proper batching and retries
- **Hybrid Storage**: Balance primary/backup storage costs

### Network Optimization

- **Streaming**: Limit connections, optimize buffer sizes
- **External Storage**: Use connection pooling, enable compression
- **Load Balancing**: Distribute audit load across instances

## Troubleshooting

### Common Issues

#### High Queue Depth
```
Problem: Events accumulating in queue
Solution: Increase worker threads or batch size
```

#### Storage Failures
```
Problem: Cannot write to storage backend
Solution: Check connectivity, disk space, permissions
```

#### Performance Issues
```
Problem: High processing latency
Solution: Optimize batch size, enable compression
```

### Debug Configuration

```yaml
logging:
  level: debug

audit:
  performance:
    debug_metrics: true
    log_queue_stats: true
```

### Diagnostic Commands

```bash
# Check audit system status
curl http://localhost:3001/audit/health

# Get audit statistics
curl http://localhost:3001/audit/stats

# Query recent events
curl "http://localhost:3001/audit/events?limit=10&event_type=security_violation"
```

## Migration Guide

### From Legacy Audit System

1. **Update Configuration**:
   ```yaml
   security:
     audit:
       enabled: true   # Modern audit system
   ```

2. **Update Code**:
   ```rust
   // Old way
   audit_service.log_event(entry).await?;
   
   // New way
   audit_log!(AuditEventType::Authentication, "component", "message");
   ```

3. **Migrate Data** (if needed):
   ```bash
   magictunnel-audit-migrate --from ./logs --to ./logs/audit
   ```

### Backward Compatibility

The centralized audit system maintains backward compatibility:
- Legacy audit events are automatically converted
- Existing audit queries continue to work
- Gradual migration path available

## Best Practices

### Event Design

1. **Use Structured Data**: Include relevant metadata
2. **Consistent Naming**: Follow naming conventions
3. **Correlation IDs**: Link related events
4. **Appropriate Severity**: Use correct severity levels

### Performance

1. **Batch Operations**: Use appropriate batch sizes
2. **Async Processing**: Never block on audit operations
3. **Resource Limits**: Configure appropriate queue sizes
4. **Monitor Health**: Regularly check system health

### Security

1. **Sensitive Data**: Never log passwords or secrets
2. **Access Control**: Restrict audit data access
3. **Integrity**: Enable integrity verification
4. **Retention**: Follow data retention policies

### Operational

1. **Regular Cleanup**: Configure appropriate retention
2. **Backup Strategy**: Ensure audit data is backed up
3. **Disaster Recovery**: Plan for audit system recovery
4. **Documentation**: Document custom event types

## API Reference

### Configuration Schema

```yaml
audit:
  enabled: boolean
  storage: StorageConfig
  event_types: string[]
  retention_days: number
  streaming: StreamingConfig
  performance: PerformanceConfig
  multi_tenant: boolean
  integrity_checks: boolean
```

### Event Schema

```typescript
interface AuditEvent {
  id: string;
  timestamp: string;
  event_type: string;
  component: string;
  message: string;
  metadata: AuditMetadata;
  payload: any;
  severity: 'debug' | 'info' | 'warning' | 'error' | 'critical';
  correlation_id?: string;
}
```

### REST API Endpoints

- `GET /audit/health` - System health check
- `GET /audit/stats` - System statistics
- `GET /audit/events` - Query audit events
- `POST /audit/events` - Create audit event
- `WS /audit/stream` - Real-time event stream

## Examples

### Complete Integration Example

```rust
use magictunnel::security::audit::{
    initialize_audit_system, AuditConfig, AuditEventType,
    integration::{AuditHelpers, AuditIntegration}
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize audit system
    let config = AuditConfig::default();
    initialize_audit_system(config).await?;
    
    // Log authentication event
    AuditHelpers::log_authentication(
        "auth_service",
        "user123",
        true,
        "oauth",
        Some("session456"),
        Some("192.168.1.1"),
    ).await?;
    
    // Log tool execution
    AuditHelpers::log_tool_execution(
        "smart_discovery",
        "get_weather",
        true,
        Some("user123"),
        Some("session456"),
        Some(1500),
        Some(&serde_json::json!({"location": "New York"})),
    ).await?;
    
    Ok(())
}
```

### Custom Event Type Example

```rust
use magictunnel::security::audit::{AuditEvent, AuditEventType, AuditSeverity};

// Create custom event
let event = AuditEvent::new(
    AuditEventType::Custom("data_export".to_string()),
    "data_service".to_string(),
    "User exported customer data".to_string(),
)
.with_severity(AuditSeverity::Warning)
.with_user("user123".to_string())
.with_payload(serde_json::json!({
    "export_type": "customer_data",
    "record_count": 1500,
    "file_format": "csv"
}));

// Log the event
if let Some(collector) = get_audit_collector() {
    collector.log_event(event).await?;
}
```

## Support

For additional support and examples:

- **Documentation**: [MagicTunnel Docs](https://docs.magictunnel.io)
- **GitHub Issues**: [Report Issues](https://github.com/MagicBeansAI/magictunnel/issues)
- **Community**: [Discord](https://discord.gg/magictunnel)
- **Enterprise Support**: enterprise@magictunnel.io