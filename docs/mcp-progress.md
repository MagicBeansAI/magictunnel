# Granular Progress Tracking System

MagicTunnel implements comprehensive progress tracking for long-running operations according to MCP 2025-06-18 specification with real-time monitoring and sub-operation support.

## Overview

The Granular Progress Tracking System provides:
- **Real-time Progress Updates**: Live progress monitoring with percentage and step tracking
- **Sub-operation Support**: Detailed tracking of complex multi-step operations
- **Event Broadcasting**: Real-time progress event notifications
- **History Tracking**: Complete audit trail of progress snapshots
- **Configurable Granularity**: Basic to verbose progress levels

## Implementation

### Core Components

- **File**: `src/mcp/progress.rs`
- **Integration**: `src/mcp/server.rs` (ProgressTracker)
- **Example**: `examples/progress_tracking_example.rs`
- **Configuration**: `ProgressConfig` with granularity and timeouts

### Key Features

1. **Progress Sessions**
   ```rust
   pub struct ProgressSession {
       pub id: String,
       pub operation_id: String,
       pub created_at: DateTime<Utc>,
       pub state: ProgressState,
       pub sub_operations: Vec<SubOperation>,
       pub history: Vec<ProgressSnapshot>,
   }
   ```

2. **Progress States**
   ```rust
   pub enum ProgressState {
       Initializing,
       InProgress { percentage: f64, current_step: String, eta_seconds: Option<u64> },
       Paused { reason: String, paused_at_percentage: f64 },
       Completed { completed_at: DateTime<Utc>, result_summary: Option<String> },
       Failed { failed_at: DateTime<Utc>, error_message: String },
       Cancelled { cancelled_at: DateTime<Utc>, reason: String },
   }
   ```

3. **Sub-operations**
   ```rust
   pub struct SubOperation {
       pub id: String,
       pub name: String,
       pub state: SubOperationState,
       pub percentage: f64,
       pub started_at: Option<DateTime<Utc>>,
       pub ended_at: Option<DateTime<Utc>>,
       pub metadata: HashMap<String, Value>,
   }
   ```

## API Usage

### Create Progress Tracker
```rust
let config = ProgressConfig {
    enabled: true,
    max_concurrent_sessions: 1000,
    update_interval_ms: 500,
    session_timeout_seconds: 3600,
    enable_detailed_events: true,
    granularity_level: ProgressGranularity::Detailed,
};

let progress_tracker = ProgressTracker::new(config);
```

### Basic Progress Tracking
```rust
// Create progress session
let session_id = progress_tracker.create_session(
    "file_processing".to_string(),
    json!({
        "file_name": "large_dataset.csv",
        "file_size": 1024000
    }).as_object().unwrap().clone().into_iter().collect(),
).await?;

// Update progress
let progress_state = ProgressState::InProgress {
    percentage: 50.0,
    current_step: "Processing row 5000".to_string(),
    total_steps: Some(10),
    current_step_number: Some(5),
    eta_seconds: Some(120),
};

progress_tracker.update_progress(
    &session_id,
    progress_state,
    Vec::new(),
    HashMap::new(),
).await?;

// Complete session
progress_tracker.complete_session(
    &session_id,
    Some("File processed successfully".to_string()),
).await?;
```

### Advanced Progress with Sub-operations
```rust
// Create session for complex operation
let session_id = progress_tracker.create_session(
    "ml_training_pipeline".to_string(),
    json!({
        "pipeline_name": "neural_network_training",
        "dataset_size": 50000,
        "model_type": "transformer"
    }).as_object().unwrap().clone().into_iter().collect(),
).await?;

// Define sub-operations
let sub_operations = vec![
    SubOperation {
        id: "data_loading".to_string(),
        name: "Data Loading".to_string(),
        state: SubOperationState::Pending,
        percentage: 0.0,
        started_at: None,
        ended_at: None,
        metadata: json!({"expected_records": 50000}).as_object().unwrap().clone().into_iter().collect(),
    },
    SubOperation {
        id: "preprocessing".to_string(),
        name: "Data Preprocessing".to_string(),
        state: SubOperationState::Pending,
        percentage: 0.0,
        started_at: None,
        ended_at: None,
        metadata: json!({"steps": ["normalization", "feature_extraction"]}).as_object().unwrap().clone().into_iter().collect(),
    },
    SubOperation {
        id: "training".to_string(),
        name: "Model Training".to_string(),
        state: SubOperationState::Pending,
        percentage: 0.0,
        started_at: None,
        ended_at: None,
        metadata: json!({"epochs": 100, "batch_size": 32}).as_object().unwrap().clone().into_iter().collect(),
    },
];

// Add sub-operations to session
progress_tracker.add_sub_operations(&session_id, sub_operations).await?;

// Start and update sub-operations
progress_tracker.start_sub_operation(&session_id, "data_loading").await?;

// Update sub-operation progress
let sub_update = SubOperationUpdate {
    id: "data_loading".to_string(),
    state: SubOperationState::Active,
    percentage: 75.0,
    metadata: json!({"records_loaded": 37500}).as_object().unwrap().clone().into_iter().collect(),
};

progress_tracker.update_progress(
    &session_id,
    ProgressState::InProgress {
        percentage: 25.0, // Overall progress
        current_step: "Loading data...".to_string(),
        total_steps: Some(3),
        current_step_number: Some(1),
        eta_seconds: Some(300),
    },
    vec![sub_update],
    HashMap::new(),
).await?;

// Complete sub-operation
progress_tracker.complete_sub_operation(
    &session_id,
    "data_loading",
    json!({"records_loaded": 50000, "load_time_seconds": 45}).as_object().unwrap().clone().into_iter().collect(),
).await?;
```

### Event Subscription
```rust
let mut event_receiver = progress_tracker.subscribe_to_events();

tokio::spawn(async move {
    while let Ok(event) = event_receiver.recv().await {
        match event.event_type {
            ProgressEventType::SessionCreated => {
                info!("Progress session created: {}", event.session_id);
            }
            ProgressEventType::ProgressUpdated => {
                info!("Progress updated for session: {}", event.session_id);
            }
            ProgressEventType::SubOperationStarted => {
                info!("Sub-operation started in session: {}", event.session_id);
            }
            ProgressEventType::SessionCompleted => {
                info!("Progress session completed: {}", event.session_id);
            }
            _ => {
                debug!("Progress event: {:?}", event.event_type);
            }
        }
    }
});
```

## Configuration

### Basic Configuration
```yaml
# magictunnel-config.yaml
mcp_2025_security:
  progress:
    enabled: true
    max_concurrent_sessions: 1000
    update_interval_ms: 500
    session_timeout_seconds: 3600
    enable_detailed_events: true
    granularity_level: "Detailed"
```

### Advanced Configuration
```yaml
mcp_2025_security:
  progress:
    enabled: true
    max_concurrent_sessions: 5000
    update_interval_ms: 100  # Fast updates
    session_timeout_seconds: 7200  # 2 hours
    enable_detailed_events: true
    cleanup_interval_seconds: 300  # 5 minutes
    granularity_level: "Verbose"  # Most detailed tracking
```

### Granularity Levels
```rust
pub enum ProgressGranularity {
    Basic,      // Start, complete, error only
    Standard,   // Add percentage updates
    Detailed,   // Add step-by-step tracking
    Verbose,    // Add sub-operation tracking
}
```

## Statistics and Monitoring

### Available Metrics
```rust
pub struct ProgressStats {
    pub active_sessions: usize,
    pub sessions_by_state: HashMap<String, usize>,
    pub avg_session_duration_seconds: f64,
    pub total_completed: usize,
    pub total_failed: usize,
    pub total_cancelled: usize,
}
```

### Get Statistics
```rust
let stats = progress_tracker.get_stats().await;
println!("Active sessions: {}", stats.active_sessions);
println!("Completed sessions: {}", stats.total_completed);
println!("Average duration: {:.2} seconds", stats.avg_session_duration_seconds);

// Sessions by state
for (state, count) in &stats.sessions_by_state {
    println!("{}: {}", state, count);
}
```

## MCP Server Integration

### Access via MCP Server
```rust
// Create progress session
let session_id = mcp_server.create_progress_session(
    operation_id,
    metadata
).await?;

// Update progress
mcp_server.update_progress(
    &session_id,
    progress_state,
    sub_operation_updates,
    metadata
).await?;

// Complete session
mcp_server.complete_progress_session(
    &session_id,
    Some("Operation completed successfully".to_string())
).await?;

// Get session details
let session = mcp_server.get_progress_session(&session_id).await;

// Subscribe to events
let event_receiver = mcp_server.subscribe_to_progress_events();

// Get statistics
let stats = mcp_server.get_progress_stats().await;
```

## Integration with Long-Running Operations

### Example: File Processing
```rust
pub async fn process_large_file(
    progress_tracker: &ProgressTracker,
    file_path: &str,
) -> Result<(), ProxyError> {
    // Create progress session
    let session_id = progress_tracker.create_session(
        "file_processing".to_string(),
        json!({
            "file_path": file_path,
            "operation_type": "batch_processing"
        }).as_object().unwrap().clone().into_iter().collect(),
    ).await?;

    let total_lines = count_file_lines(file_path).await?;
    let mut processed_lines = 0;

    // Process file with progress updates
    let file = tokio::fs::File::open(file_path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        // Process line
        process_line(&line).await?;
        processed_lines += 1;

        // Update progress every 100 lines
        if processed_lines % 100 == 0 {
            let percentage = (processed_lines as f64 / total_lines as f64) * 100.0;
            let eta_seconds = if processed_lines > 0 {
                let elapsed = /* calculate elapsed time */;
                let rate = processed_lines as f64 / elapsed;
                Some(((total_lines - processed_lines) as f64 / rate) as u64)
            } else {
                None
            };

            progress_tracker.update_progress(
                &session_id,
                ProgressState::InProgress {
                    percentage,
                    current_step: format!("Processing line {}", processed_lines),
                    total_steps: Some(total_lines as u32),
                    current_step_number: Some(processed_lines as u32),
                    eta_seconds,
                },
                Vec::new(),
                json!({
                    "lines_processed": processed_lines,
                    "lines_per_second": /* calculate rate */
                }).as_object().unwrap().clone().into_iter().collect(),
            ).await?;
        }
    }

    // Complete the session
    progress_tracker.complete_session(
        &session_id,
        Some(format!("Processed {} lines successfully", processed_lines)),
    ).await?;

    Ok(())
}
```

### Example: Multi-Stage Pipeline
```rust
pub async fn ml_training_pipeline(
    progress_tracker: &ProgressTracker,
    config: &MLConfig,
) -> Result<(), ProxyError> {
    let session_id = progress_tracker.create_session(
        "ml_training".to_string(),
        json!({
            "model_type": config.model_type,
            "dataset_size": config.dataset_size,
            "epochs": config.epochs
        }).as_object().unwrap().clone().into_iter().collect(),
    ).await?;

    // Define pipeline stages
    let stages = vec![
        ("data_loading", "Loading training data"),
        ("preprocessing", "Preprocessing data"),
        ("training", "Training model"),
        ("validation", "Validating model"),
        ("saving", "Saving model"),
    ];

    let sub_operations: Vec<SubOperation> = stages.iter().map(|(id, name)| {
        SubOperation {
            id: id.to_string(),
            name: name.to_string(),
            state: SubOperationState::Pending,
            percentage: 0.0,
            started_at: None,
            ended_at: None,
            metadata: HashMap::new(),
        }
    }).collect();

    progress_tracker.add_sub_operations(&session_id, sub_operations).await?;

    // Execute stages
    for (stage_idx, (stage_id, stage_name)) in stages.iter().enumerate() {
        progress_tracker.start_sub_operation(&session_id, stage_id).await?;

        // Execute stage with sub-operation progress updates
        match stage_id {
            &"data_loading" => {
                execute_data_loading(&progress_tracker, &session_id, stage_id, config).await?;
            }
            &"preprocessing" => {
                execute_preprocessing(&progress_tracker, &session_id, stage_id, config).await?;
            }
            &"training" => {
                execute_training(&progress_tracker, &session_id, stage_id, config).await?;
            }
            &"validation" => {
                execute_validation(&progress_tracker, &session_id, stage_id, config).await?;
            }
            &"saving" => {
                execute_saving(&progress_tracker, &session_id, stage_id, config).await?;
            }
            _ => {}
        }

        // Complete sub-operation
        progress_tracker.complete_sub_operation(
            &session_id,
            stage_id,
            json!({
                "stage": stage_name,
                "completed_at": chrono::Utc::now()
            }).as_object().unwrap().clone().into_iter().collect(),
        ).await?;

        // Update overall progress
        let overall_percentage = ((stage_idx + 1) as f64 / stages.len() as f64) * 100.0;
        progress_tracker.update_progress(
            &session_id,
            ProgressState::InProgress {
                percentage: overall_percentage,
                current_step: format!("Completed {}", stage_name),
                total_steps: Some(stages.len() as u32),
                current_step_number: Some((stage_idx + 1) as u32),
                eta_seconds: None,
            },
            Vec::new(),
            HashMap::new(),
        ).await?;
    }

    progress_tracker.complete_session(
        &session_id,
        Some("ML training pipeline completed successfully".to_string()),
    ).await?;

    Ok(())
}
```

## Error Handling and Failure States

### Handling Failures
```rust
pub async fn handle_operation_failure(
    progress_tracker: &ProgressTracker,
    session_id: &str,
    error: &ProxyError,
) -> Result<(), ProxyError> {
    progress_tracker.fail_session(
        session_id,
        error.to_string(),
        Some("OPERATION_ERROR".to_string()),
    ).await?;

    Ok(())
}
```

### Timeout Handling
```rust
pub async fn operation_with_timeout(
    progress_tracker: &ProgressTracker,
    timeout_seconds: u64,
) -> Result<(), ProxyError> {
    let session_id = progress_tracker.create_session(
        "timed_operation".to_string(),
        HashMap::new(),
    ).await?;

    let result = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        long_running_operation(&session_id),
    ).await;

    match result {
        Ok(Ok(_)) => {
            progress_tracker.complete_session(&session_id, None).await?;
        }
        Ok(Err(e)) => {
            progress_tracker.fail_session(&session_id, e.to_string(), None).await?;
        }
        Err(_) => {
            progress_tracker.fail_session(
                &session_id,
                "Operation timed out".to_string(),
                Some("TIMEOUT".to_string()),
            ).await?;
        }
    }

    Ok(())
}
```

## Best Practices

### 1. Progress Update Frequency
```rust
// Good: Update every significant milestone
if processed_items % 100 == 0 {
    update_progress(percentage).await?;
}

// Avoid: Too frequent updates (performance impact)
// update_progress(percentage).await?; // On every item

// Avoid: Too infrequent updates (poor UX)
// Only update at start and end
```

### 2. Meaningful Progress Information
```rust
// Good: Descriptive current step
ProgressState::InProgress {
    percentage: 45.0,
    current_step: "Processing customer records batch 4/10".to_string(),
    total_steps: Some(10),
    current_step_number: Some(4),
    eta_seconds: Some(120),
}

// Avoid: Generic progress
// current_step: "Processing...".to_string(),
```

### 3. Sub-operation Organization
```rust
// Good: Logical sub-operations
let sub_operations = vec![
    ("download", "Download data"),
    ("validate", "Validate data integrity"),
    ("transform", "Transform to target format"),
    ("upload", "Upload to destination"),
];

// Avoid: Too granular sub-operations
// Every small step as a sub-operation
```

### 4. Error Recovery
```rust
pub async fn resilient_operation(
    progress_tracker: &ProgressTracker,
    session_id: &str,
) -> Result<(), ProxyError> {
    for attempt in 1..=3 {
        match risky_operation().await {
            Ok(result) => {
                progress_tracker.update_progress(
                    session_id,
                    ProgressState::InProgress {
                        percentage: 100.0,
                        current_step: "Operation succeeded".to_string(),
                        total_steps: None,
                        current_step_number: None,
                        eta_seconds: None,
                    },
                    Vec::new(),
                    json!({"attempt": attempt, "result": "success"}).as_object().unwrap().clone().into_iter().collect(),
                ).await?;
                return Ok(());
            }
            Err(e) if attempt < 3 => {
                progress_tracker.update_progress(
                    session_id,
                    ProgressState::InProgress {
                        percentage: (attempt as f64 / 3.0) * 100.0,
                        current_step: format!("Retry attempt {} failed, retrying...", attempt),
                        total_steps: Some(3),
                        current_step_number: Some(attempt as u32),
                        eta_seconds: Some(30),
                    },
                    Vec::new(),
                    json!({"attempt": attempt, "error": e.to_string()}).as_object().unwrap().clone().into_iter().collect(),
                ).await?;
                
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                progress_tracker.fail_session(
                    session_id,
                    format!("Operation failed after {} attempts: {}", attempt, e),
                    Some("MAX_RETRIES_EXCEEDED".to_string()),
                ).await?;
                return Err(e);
            }
        }
    }
    
    Ok(())
}
```

## Testing

### Unit Tests
The progress tracking system includes comprehensive unit tests:
- Session creation and management
- Progress state transitions
- Sub-operation handling
- Event broadcasting
- Statistics calculation

### Integration Tests
See `tests/mcp_2025_06_18_compliance_test.rs` for integration tests and `examples/progress_tracking_example.rs` for complete usage examples.

### Test Example
```rust
#[tokio::test]
async fn test_progress_session_lifecycle() {
    let progress_tracker = ProgressTracker::new(ProgressConfig::default());
    
    // Create session
    let session_id = progress_tracker.create_session(
        "test_operation".to_string(),
        HashMap::new(),
    ).await.unwrap();
    
    // Update progress
    progress_tracker.update_progress(
        &session_id,
        ProgressState::InProgress {
            percentage: 50.0,
            current_step: "Halfway done".to_string(),
            total_steps: Some(2),
            current_step_number: Some(1),
            eta_seconds: Some(60),
        },
        Vec::new(),
        HashMap::new(),
    ).await.unwrap();
    
    // Complete session
    progress_tracker.complete_session(
        &session_id,
        Some("Test completed".to_string()),
    ).await.unwrap();
    
    // Verify session state
    let session = progress_tracker.get_session(&session_id).await.unwrap();
    assert!(matches!(session.state, ProgressState::Completed { .. }));
}
```

## Performance Considerations

### Memory Management
- Sessions are automatically cleaned up after timeout
- History snapshots are limited to prevent memory growth
- Background cleanup tasks maintain system health

### Concurrency
- Thread-safe design supports thousands of concurrent sessions
- Non-blocking event broadcasting
- Efficient session lookup with HashMap indexing

### Network Efficiency
- Configurable update intervals prevent network flooding
- Event batching for high-frequency updates
- Optional event filtering by importance

## Related Documentation
- [Enhanced Cancellation](mcp-cancellation.md) - Integration with cancellation system
- [Tool Validation](mcp-tool-validation.md) - Progress tracking in validation processes
- [MCP 2025-06-18 Specification](https://spec.modelcontextprotocol.io/specification/2025-06-18/)
- [Architecture Guide](architecture.md) - System architecture overview