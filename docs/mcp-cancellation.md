# Enhanced Cancellation System

MagicTunnel implements comprehensive request cancellation according to MCP 2025-06-18 specification with token-based management and graceful cleanup.

## Overview

The Enhanced Cancellation System provides:
- **Token-based Cancellation**: Secure cancellation with unique tokens
- **Graceful vs Force Cancellation**: Multiple cancellation strategies  
- **Event Broadcasting**: Real-time cancellation notifications
- **Automatic Cleanup**: Resource cleanup and timeout management
- **Statistics Tracking**: Comprehensive cancellation metrics

## Implementation

### Core Components

- **File**: `src/mcp/cancellation.rs`
- **Integration**: `src/mcp/server.rs` (CancellationManager)
- **Configuration**: `CancellationConfig` with timeouts and cleanup intervals

### Key Features

1. **Cancellation Tokens**
   ```rust
   pub struct CancellationToken {
       pub id: String,
       pub operation_id: String,
       pub created_at: DateTime<Utc>,
       pub expires_at: DateTime<Utc>,
       pub reason: Option<String>,
   }
   ```

2. **Cancellation Types**
   - `Graceful`: Allow operation to complete current step
   - `Force`: Immediate termination with cleanup

3. **Event System**
   ```rust
   pub enum CancellationEventType {
       CancellationRequested,
       CancellationExecuted,
       CancellationCompleted,
       CancellationExpired,
   }
   ```

## API Usage

### Create Cancellation Manager
```rust
let config = CancellationConfig {
    enabled: true,
    default_timeout_seconds: 300,
    max_concurrent_operations: 1000,
    cleanup_interval_seconds: 60,
    graceful_timeout_seconds: 30,
};

let cancellation_manager = CancellationManager::new(config);
```

### Request Cancellation
```rust
// Create cancellation token
let token = cancellation_manager.create_cancellation_token(
    "operation_123".to_string(),
    Some("User requested cancellation".to_string()),
).await?;

// Cancel operation
cancellation_manager.cancel_operation(
    &token.id,
    CancellationReason::UserRequested,
).await?;
```

### Check Cancellation
```rust
// In long-running operations
if cancellation_manager.is_cancelled("operation_123").await {
    // Handle cancellation
    return Err(ProxyError::mcp("Operation was cancelled"));
}
```

### Subscribe to Events
```rust
let mut event_receiver = cancellation_manager.subscribe_to_events();

tokio::spawn(async move {
    while let Ok(event) = event_receiver.recv().await {
        println!("Cancellation event: {:?}", event.event_type);
    }
});
```

## Configuration

### Basic Configuration
```yaml
# magictunnel-config.yaml
mcp_2025_security:
  cancellation:
    enabled: true
    default_timeout_seconds: 300
    max_concurrent_operations: 1000
    cleanup_interval_seconds: 60
    graceful_timeout_seconds: 30
    enable_detailed_events: true
```

### Advanced Configuration
```yaml
mcp_2025_security:
  cancellation:
    enabled: true
    default_timeout_seconds: 600
    max_concurrent_operations: 5000
    cleanup_interval_seconds: 30
    graceful_timeout_seconds: 60
    force_timeout_seconds: 10
    enable_detailed_events: true
    enable_cleanup_on_startup: true
    token_expiry_seconds: 3600
```

## Statistics and Monitoring

### Available Metrics
```rust
pub struct CancellationStats {
    pub active_operations: usize,
    pub total_cancelled: usize,
    pub graceful_cancellations: usize,
    pub force_cancellations: usize,
    pub expired_tokens: usize,
    pub avg_cancellation_time_ms: f64,
    pub operations_by_reason: HashMap<String, usize>,
}
```

### Get Statistics
```rust
let stats = cancellation_manager.get_stats().await;
println!("Active operations: {}", stats.active_operations);
println!("Total cancelled: {}", stats.total_cancelled);
```

## MCP Server Integration

### Access via MCP Server
```rust
// Get cancellation statistics
let stats = mcp_server.get_cancellation_stats().await;

// Create cancellation token
let token = mcp_server.create_cancellation_token(
    operation_id,
    Some("User cancellation".to_string())
).await?;

// Cancel operation
mcp_server.cancel_operation(&token.id, reason).await?;

// Subscribe to events
let event_receiver = mcp_server.subscribe_to_cancellation_events();
```

## Best Practices

### 1. Integration in Long-Running Operations
```rust
pub async fn long_running_operation(
    cancellation_manager: &CancellationManager,
    operation_id: &str,
) -> Result<(), ProxyError> {
    for i in 0..100 {
        // Check cancellation before expensive operations
        if cancellation_manager.is_cancelled(operation_id).await {
            return Err(ProxyError::mcp("Operation cancelled"));
        }
        
        // Perform work
        expensive_operation().await?;
        
        // Optional: Check again after work
        if cancellation_manager.is_cancelled(operation_id).await {
            cleanup_partial_work().await?;
            return Err(ProxyError::mcp("Operation cancelled during cleanup"));
        }
    }
    
    Ok(())
}
```

### 2. Graceful Cancellation Handling
```rust
pub async fn graceful_operation(
    cancellation_manager: &CancellationManager,
    operation_id: &str,
) -> Result<(), ProxyError> {
    let mut current_step = 0;
    let total_steps = 10;
    
    while current_step < total_steps {
        // Check for graceful cancellation
        if let Some(reason) = cancellation_manager.get_cancellation_reason(operation_id).await {
            match reason {
                CancellationReason::UserRequested => {
                    // Complete current step before cancelling
                    complete_current_step(current_step).await?;
                    return Err(ProxyError::mcp("Operation cancelled gracefully"));
                }
                CancellationReason::Timeout => {
                    // Force immediate cancellation
                    return Err(ProxyError::mcp("Operation timed out"));
                }
                _ => {
                    return Err(ProxyError::mcp("Operation cancelled"));
                }
            }
        }
        
        perform_step(current_step).await?;
        current_step += 1;
    }
    
    Ok(())
}
```

### 3. Event Handling
```rust
pub async fn setup_cancellation_monitoring(
    cancellation_manager: &CancellationManager,
) {
    let mut event_receiver = cancellation_manager.subscribe_to_events();
    
    tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            match event.event_type {
                CancellationEventType::CancellationRequested => {
                    info!("Cancellation requested for operation {}", event.operation_id);
                }
                CancellationEventType::CancellationExecuted => {
                    info!("Cancellation executed for operation {}", event.operation_id);
                }
                CancellationEventType::CancellationCompleted => {
                    info!("Cancellation completed for operation {}", event.operation_id);
                }
                CancellationEventType::CancellationExpired => {
                    warn!("Cancellation token expired for operation {}", event.operation_id);
                }
            }
        }
    });
}
```

## Error Handling

### Common Errors
- `CancellationTokenNotFound`: Token doesn't exist or has expired
- `OperationNotFound`: Operation ID not found in active operations
- `CancellationAlreadyRequested`: Cancellation already in progress
- `CancellationTimeout`: Graceful cancellation timed out

### Error Recovery
```rust
match cancellation_manager.cancel_operation(&token_id, reason).await {
    Ok(_) => {
        info!("Operation cancelled successfully");
    }
    Err(ProxyError::CancellationTokenNotFound) => {
        warn!("Cancellation token not found, operation may have completed");
    }
    Err(ProxyError::OperationNotFound) => {
        info!("Operation not found, may have already completed");
    }
    Err(e) => {
        error!("Failed to cancel operation: {}", e);
    }
}
```

## Testing

### Unit Tests
The cancellation system includes comprehensive unit tests in `src/mcp/cancellation.rs`:
- Token creation and expiry
- Graceful vs force cancellation
- Event broadcasting
- Statistics tracking
- Cleanup operations

### Integration Tests  
See `tests/mcp_2025_06_18_compliance_test.rs` for integration tests covering:
- MCP server integration
- API endpoint testing
- Event system validation
- Error handling verification

## Performance Considerations

### Memory Usage
- Tokens are automatically cleaned up on expiry
- Configurable cleanup intervals prevent memory leaks
- LRU-style eviction for large operation counts

### Concurrency
- Thread-safe design with `Arc<RwLock<>>`
- Non-blocking event broadcasting
- Efficient token lookup with HashMap indexing

### Scalability
- Supports thousands of concurrent operations
- Configurable limits prevent resource exhaustion
- Background cleanup tasks maintain performance

## Migration from Legacy Systems

### From Manual Cancellation
```rust
// Before: Manual cancellation tracking
let mut cancelled = Arc::new(AtomicBool::new(false));

// After: Token-based cancellation
let token = cancellation_manager.create_cancellation_token(
    operation_id,
    None,
).await?;
```

### Integration Timeline
1. **Phase 1**: Add cancellation manager to existing operations
2. **Phase 2**: Replace manual cancellation with token system
3. **Phase 3**: Add event handling and monitoring
4. **Phase 4**: Enable advanced features (graceful cancellation, statistics)

## Related Documentation
- [MCP 2025-06-18 Specification](https://spec.modelcontextprotocol.io/specification/2025-06-18/)
- [Progress Tracking](mcp-progress.md) - Cancellation integration with progress tracking
- [Tool Validation](mcp-tool-validation.md) - Security considerations
- [Architecture Guide](architecture.md) - System architecture overview