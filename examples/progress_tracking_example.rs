//! Example of using the MagicTunnel Progress Tracking System
//! 
//! This example demonstrates how to track progress for long-running operations
//! in the MCP (Model Context Protocol) context.

use magictunnel::mcp::progress::{
    ProgressTracker, ProgressConfig, ProgressState, SubOperation, SubOperationState,
    SubOperationUpdate, ProgressGranularity
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("🔄 MagicTunnel Progress Tracking Example");
    println!("========================================");

    // Create progress tracker with detailed configuration
    let config = ProgressConfig {
        enabled: true,
        max_concurrent_sessions: 100,
        update_interval_ms: 100, // Fast updates for demo
        session_timeout_seconds: 300, // 5 minutes
        enable_detailed_events: true,
        cleanup_interval_seconds: 60,
        granularity_level: ProgressGranularity::Detailed,
    };

    let progress_tracker = ProgressTracker::new(config);

    // Subscribe to progress events for monitoring
    let mut event_receiver = progress_tracker.subscribe_to_events();
    
    // Spawn event listener task
    tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            println!("📡 Event: {:?} for session {}", event.event_type, event.session_id);
        }
    });

    // Example 1: Simple progress tracking
    println!("\n🔥 Example 1: Simple Progress Tracking");
    simple_progress_example(&progress_tracker).await?;

    // Example 2: Detailed progress with sub-operations
    println!("\n🔥 Example 2: Detailed Progress with Sub-operations");
    detailed_progress_example(&progress_tracker).await?;

    // Example 3: Progress failure handling
    println!("\n🔥 Example 3: Progress Failure Handling");
    failure_progress_example(&progress_tracker).await?;

    // Show final statistics
    let stats = progress_tracker.get_stats().await;
    println!("\n📊 Final Statistics:");
    println!("  Active sessions: {}", stats.active_sessions);
    println!("  Total completed: {}", stats.total_completed);
    println!("  Total failed: {}", stats.total_failed);
    println!("  Average duration: {:.2} seconds", stats.avg_session_duration_seconds);

    println!("\n✅ Progress tracking examples completed!");
    Ok(())
}

/// Example 1: Simple progress tracking for a basic operation
async fn simple_progress_example(tracker: &ProgressTracker) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Starting simple file processing operation...");

    // Create progress session
    let session_id = tracker.create_session(
        "file_processing".to_string(),
        json!({
            "file_name": "large_dataset.csv",
            "file_size": 1024000,
            "operation_type": "csv_analysis"
        }).as_object().unwrap().clone().into_iter().collect(),
    ).await?;

    println!("  📋 Created session: {}", session_id);

    // Simulate processing with progress updates
    for i in 0..=10 {
        let percentage = (i as f64 / 10.0) * 100.0;
        
        let state = if i == 10 {
            ProgressState::Completed {
                completed_at: chrono::Utc::now(),
                result_summary: Some("File processed successfully".to_string()),
                duration_seconds: 5,
            }
        } else {
            ProgressState::InProgress {
                percentage,
                current_step: format!("Processing row {}", i * 1000),
                total_steps: Some(10),
                current_step_number: Some(i),
                eta_seconds: Some((10 - i) as u64),
            }
        };

        tracker.update_progress(
            &session_id,
            state,
            Vec::new(),
            json!({
                "rows_processed": i * 1000,
                "current_memory_usage": 45.5 + (i as f64 * 2.3)
            }).as_object().unwrap().clone().into_iter().collect(),
        ).await?;

        println!("    📈 Progress: {:.1}%", percentage);
        sleep(Duration::from_millis(200)).await;
    }

    // Complete the session
    tracker.complete_session(&session_id, Some("CSV analysis completed successfully".to_string())).await?;
    println!("  ✅ Simple progress tracking completed");

    Ok(())
}

/// Example 2: Detailed progress tracking with sub-operations
async fn detailed_progress_example(tracker: &ProgressTracker) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Starting complex data pipeline operation...");

    // Create progress session with detailed metadata
    let session_id = tracker.create_session(
        "data_pipeline".to_string(),
        json!({
            "pipeline_name": "ml_training_pipeline",
            "dataset_size": 50000,
            "model_type": "neural_network",
            "estimated_duration": 300
        }).as_object().unwrap().clone().into_iter().collect(),
    ).await?;

    println!("  📋 Created detailed session: {}", session_id);

    // Define sub-operations
    let sub_operations = vec![
        SubOperation {
            id: "data_loading".to_string(),
            name: "Data Loading".to_string(),
            state: SubOperationState::Pending,
            percentage: 0.0,
            started_at: None,
            ended_at: None,
            metadata: json!({
                "data_sources": ["database", "api", "files"],
                "expected_records": 50000
            }).as_object().unwrap().clone().into_iter().collect(),
        },
        SubOperation {
            id: "data_preprocessing".to_string(),
            name: "Data Preprocessing".to_string(),
            state: SubOperationState::Pending,
            percentage: 0.0,
            started_at: None,
            ended_at: None,
            metadata: json!({
                "preprocessing_steps": ["normalization", "feature_extraction", "validation"]
            }).as_object().unwrap().clone().into_iter().collect(),
        },
        SubOperation {
            id: "model_training".to_string(),
            name: "Model Training".to_string(),
            state: SubOperationState::Pending,
            percentage: 0.0,
            started_at: None,
            ended_at: None,
            metadata: json!({
                "epochs": 100,
                "batch_size": 32,
                "learning_rate": 0.001
            }).as_object().unwrap().clone().into_iter().collect(),
        },
    ];

    // Add sub-operations to session
    tracker.add_sub_operations(&session_id, sub_operations).await?;

    // Execute sub-operations sequentially
    let sub_op_ids = ["data_loading", "data_preprocessing", "model_training"];
    
    for (idx, sub_op_id) in sub_op_ids.iter().enumerate() {
        println!("    🔧 Starting sub-operation: {}", sub_op_id);
        
        // Start sub-operation
        tracker.start_sub_operation(&session_id, sub_op_id).await?;
        
        // Simulate sub-operation progress
        for i in 0..=5 {
            let sub_percentage = (i as f64 / 5.0) * 100.0;
            let overall_percentage = ((idx as f64 + (i as f64 / 5.0)) / 3.0) * 100.0;
            
            let sub_update = SubOperationUpdate {
                id: sub_op_id.to_string(),
                state: SubOperationState::Active,
                percentage: sub_percentage,
                metadata: json!({
                    "current_batch": i,
                    "processing_speed": format!("{:.1} items/sec", 150.0 + (i as f64 * 20.0))
                }).as_object().unwrap().clone().into_iter().collect(),
            };

            let main_state = ProgressState::InProgress {
                percentage: overall_percentage,
                current_step: format!("Executing {}", sub_op_id.replace("_", " ")),
                total_steps: Some(3),
                current_step_number: Some(idx as u32 + 1),
                eta_seconds: Some(((3 - idx) * 5) as u64),
            };

            tracker.update_progress(
                &session_id,
                main_state,
                vec![sub_update],
                HashMap::new(),
            ).await?;
            
            sleep(Duration::from_millis(300)).await;
        }
        
        // Complete sub-operation
        tracker.complete_sub_operation(
            &session_id,
            sub_op_id,
            json!({
                "completion_time": chrono::Utc::now(),
                "records_processed": match *sub_op_id {
                    "data_loading" => 50000,
                    "data_preprocessing" => 48500, // Some records filtered
                    "model_training" => 48500,
                    _ => 0,
                }
            }).as_object().unwrap().clone().into_iter().collect(),
        ).await?;
        
        println!("    ✅ Completed sub-operation: {}", sub_op_id);
    }

    // Complete the entire session
    tracker.complete_session(&session_id, Some("ML pipeline executed successfully with 95.2% accuracy".to_string())).await?;
    println!("  ✅ Detailed progress tracking completed");

    Ok(())
}

/// Example 3: Progress failure handling
async fn failure_progress_example(tracker: &ProgressTracker) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Starting operation that will fail...");

    // Create progress session
    let session_id = tracker.create_session(
        "failing_operation".to_string(),
        json!({
            "operation_type": "network_sync",
            "target_server": "remote.example.com",
            "sync_size": 1000000
        }).as_object().unwrap().clone().into_iter().collect(),
    ).await?;

    println!("  📋 Created session for failing operation: {}", session_id);

    // Start with normal progress
    for i in 0..4 {
        let percentage = (i as f64 / 10.0) * 100.0;
        
        let state = ProgressState::InProgress {
            percentage,
            current_step: format!("Syncing batch {}", i + 1),
            total_steps: Some(10),
            current_step_number: Some(i + 1),
            eta_seconds: Some((10 - i) as u64 * 5),
        };

        tracker.update_progress(
            &session_id,
            state,
            Vec::new(),
            json!({
                "bytes_synced": i * 100000,
                "network_latency": 50 + (i * 10)
            }).as_object().unwrap().clone().into_iter().collect(),
        ).await?;

        println!("    📈 Progress: {:.1}%", percentage);
        sleep(Duration::from_millis(200)).await;
    }

    // Simulate failure
    tracker.fail_session(
        &session_id,
        "Network connection timeout after 3 retries".to_string(),
        Some("NETWORK_TIMEOUT".to_string()),
    ).await?;

    println!("  ❌ Operation failed as expected");
    println!("  ✅ Failure handling completed");

    Ok(())
}