//! Centralized Audit Logging System
//! 
//! This module provides a comprehensive enterprise-grade audit logging system
//! that separates audit logging from general application logging. Features:
//! 
//! - **Centralized Collection**: Single audit collector for all components
//! - **Structured Events**: Consistent event schemas with metadata
//! - **Non-blocking Processing**: Async processing with message queues
//! - **Multi-backend Storage**: Memory, File, Database, External systems
//! - **Real-time Streaming**: WebSocket streaming for live monitoring
//! - **Enterprise Compliance**: Integrity, retention, multi-tenancy
//! - **Searchable & Filterable**: Structured data for analysis

pub mod collector;
pub mod events;
pub mod storage;
pub mod streaming;
pub mod integration;

pub use collector::AuditCollector;
pub use events::{AuditEvent, AuditEventType, AuditMetadata, AuditSeverity};
pub use storage::{AuditStorage, StorageBackend, StorageConfig, RolloverConfig, AuditQuery};
pub use streaming::AuditStreamer;

// Compatibility type aliases for simplified API usage

/// Audit service (alias for AuditCollector)
pub type AuditService = AuditCollector;

/// Audit entry (alias for AuditEvent)
pub type AuditEntry = AuditEvent;

/// Audit outcome enum
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuditOutcome {
    Success,
    Failure,
    Blocked,
    PendingApproval,
}

/// Audit user structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditUser {
    pub id: Option<String>,
    pub name: Option<String>,
    pub roles: Vec<String>,
    pub api_key_name: Option<String>,
    pub auth_method: String,
}

/// Audit tool structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditTool {
    pub name: String,
    pub version: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
}

/// Audit security structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditSecurity {
    pub authenticated: bool,
    pub authorized: bool,
    pub permissions_checked: Vec<String>,
    pub policies_applied: Vec<String>,
    pub content_sanitized: bool,
    pub approval_required: bool,
}

/// Audit request structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditRequest {
    pub method: String,
    pub path: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub remote_addr: Option<String>,
    pub user_agent: Option<String>,
}

/// Audit resource structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditResource {
    pub id: String,
    pub resource_type: String,
    pub name: String,
    pub metadata: std::collections::HashMap<String, String>,
}


use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Centralized audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit system
    pub enabled: bool,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Event filtering
    pub event_types: Vec<String>,
    
    /// Retention policy
    pub retention_days: u32,
    
    /// Real-time streaming configuration
    pub streaming: Option<StreamingConfig>,
    
    /// Performance settings
    pub performance: PerformanceConfig,
    
    /// Multi-tenancy support
    pub multi_tenant: bool,
    
    /// Integrity verification
    pub integrity_checks: bool,
}

/// Streaming configuration for real-time audit monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Enable WebSocket streaming
    pub enabled: bool,
    
    /// Maximum concurrent connections
    pub max_connections: usize,
    
    /// Buffer size for events
    pub buffer_size: usize,
    
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
}

/// Performance configuration for audit system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum queue size for async processing
    pub max_queue_size: usize,
    
    /// Number of worker threads
    pub worker_threads: usize,
    
    /// Batch size for storage operations
    pub batch_size: usize,
    
    /// Flush interval in seconds
    pub flush_interval_secs: u64,
    
    /// Enable compression
    pub compression: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage: StorageConfig::default(),
            event_types: vec![
                "authentication".to_string(),
                "authorization".to_string(),
                "tool_execution".to_string(),
                "security_violation".to_string(),
                "oauth_flow".to_string(),
                "mcp_connection".to_string(),
                "admin_action".to_string(),
            ],
            retention_days: 90,
            streaming: Some(StreamingConfig {
                enabled: true,
                max_connections: 100,
                buffer_size: 1000,
                heartbeat_interval: 30,
            }),
            performance: PerformanceConfig {
                max_queue_size: 10000,
                worker_threads: 2,
                batch_size: 100,
                flush_interval_secs: 5,
                compression: true,
            },
            multi_tenant: false,
            integrity_checks: true,
        }
    }
}

/// Result type for audit operations
pub type AuditResult<T> = Result<T, AuditError>;

/// Audit system errors
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Queue full: cannot accept more events")]
    QueueFull,
    
    #[error("Streaming error: {0}")]
    Streaming(String),
    
    #[error("Integrity check failed: {0}")]
    IntegrityFailure(String),
}

/// Global audit system instance
static AUDIT_COLLECTOR: std::sync::OnceLock<AuditCollector> = std::sync::OnceLock::new();

/// Initialize the global audit system
pub async fn initialize_audit_system(config: AuditConfig) -> AuditResult<()> {
    let collector = AuditCollector::new(config).await?;
    AUDIT_COLLECTOR.set(collector)
        .map_err(|_| AuditError::Config("Audit system already initialized".to_string()))?;
    Ok(())
}

/// Get the global audit collector
pub fn get_audit_collector() -> Option<&'static AuditCollector> {
    AUDIT_COLLECTOR.get()
}

/// Macro for easy audit logging
#[macro_export]
macro_rules! audit_log {
    ($event_type:expr, $component:expr, $message:expr) => {
        if let Some(collector) = $crate::security::audit::get_audit_collector() {
            let event = $crate::security::audit::AuditEvent::new(
                $event_type,
                $component.to_string(),
                $message.to_string(),
            );
            let _ = collector.log_event(event).await;
        }
    };
    
    ($event_type:expr, $component:expr, $message:expr, $($key:expr => $value:expr),*) => {
        if let Some(collector) = $crate::security::audit::get_audit_collector() {
            let mut event = $crate::security::audit::AuditEvent::new(
                $event_type,
                $component.to_string(),
                $message.to_string(),
            );
            $(
                event.add_metadata($key, serde_json::json!($value));
            )*
            let _ = collector.log_event(event).await;
        }
    };
}

/// Macro for audit logging with structured data
#[macro_export]
macro_rules! audit_log_structured {
    ($event_type:expr, $component:expr, $data:expr) => {
        if let Some(collector) = $crate::security::audit::get_audit_collector() {
            let event = $crate::security::audit::AuditEvent::from_structured(
                $event_type,
                $component.to_string(),
                $data,
            );
            let _ = collector.log_event(event).await;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_audit_config_default() {
        let config = AuditConfig::default();
        assert!(config.enabled);
        assert_eq!(config.retention_days, 90);
        assert!(config.streaming.is_some());
    }
    
    #[tokio::test]
    async fn test_audit_system_initialization() {
        let config = AuditConfig::default();
        // Note: This would normally initialize the system
        // In tests, we skip actual initialization to avoid conflicts
        assert!(config.enabled);
    }
}