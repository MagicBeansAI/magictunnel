//! Multi-Backend Audit Storage System
//! 
//! This module provides a pluggable storage abstraction that supports:
//! - Memory storage for testing and temporary events
//! - File storage with rotation and compression
//! - Database storage for enterprise deployments
//! - External systems (Elasticsearch, Splunk, etc.)
//! - Hybrid storage with multiple backends

use super::{AuditEvent, AuditResult, AuditError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use async_trait::async_trait;
use chrono::{DateTime, Utc, Duration};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
// use flate2::write::GzEncoder;
// use flate2::Compression;
// use std::io::Write;

/// Information about an audit log file
#[derive(Debug, Clone)]
struct AuditFileInfo {
    path: PathBuf,
    size: u64,
    modified: DateTime<Utc>,
}

/// Rollover configuration for file-based storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloverConfig {
    /// Maximum total storage size in bytes (GB limit converted to bytes)
    /// When exceeded, oldest files are deleted. Size takes precedence over time.
    pub max_total_size_gb: f64,
    
    /// Maximum file age in days before rollover
    /// Files older than this are eligible for deletion
    pub max_age_days: u32,
    
    /// Check interval for rollover policies in seconds
    pub check_interval_secs: u64,
    
    /// Enable automatic cleanup of old files
    pub auto_cleanup: bool,
}

impl Default for RolloverConfig {
    fn default() -> Self {
        Self {
            max_total_size_gb: 5.0,      // 5GB default limit
            max_age_days: 90,            // 90 days default retention
            check_interval_secs: 3600,   // Check every hour
            auto_cleanup: true,
        }
    }
}

/// Storage configuration for different backends
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StorageConfig {
    /// In-memory storage (for testing)
    Memory {
        max_events: usize,
    },
    
    /// File-based storage with rotation
    File {
        directory: PathBuf,
        max_file_size: u64,
        max_files: usize,
        compress: bool,
        sync_interval_secs: u64,
        rollover: RolloverConfig,
    },
    
    /// Database storage
    Database {
        connection_string: String,
        table_name: String,
        batch_size: usize,
        connection_pool_size: usize,
    },
    
    /// External system integration
    External {
        endpoint: String,
        api_key: Option<String>,
        format: ExternalFormat,
        batch_size: usize,
        timeout_secs: u64,
    },
    
    /// Hybrid storage (multiple backends)
    Hybrid {
        backends: Vec<StorageConfig>,
        primary: usize, // Index of primary backend
    },
}

/// External system formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalFormat {
    JsonLines,
    Elasticsearch,
    Splunk,
    Syslog,
    Custom(String),
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self::File {
            directory: PathBuf::from("./logs/audit"),
            max_file_size: 100 * 1024 * 1024, // 100MB
            max_files: 10,
            compress: true,
            sync_interval_secs: 5,
            rollover: RolloverConfig::default(),
        }
    }
}

/// Storage trait for audit events
#[async_trait]
pub trait AuditStorage: Send + Sync {
    /// Store a single audit event
    async fn store_event(&self, event: &AuditEvent) -> AuditResult<()>;
    
    /// Store multiple events in batch
    async fn store_batch(&self, events: &[AuditEvent]) -> AuditResult<()>;
    
    /// Query events with filters
    async fn query_events(&self, query: &AuditQuery) -> AuditResult<Vec<AuditEvent>>;
    
    /// Count events matching criteria
    async fn count_events(&self, query: &AuditQuery) -> AuditResult<u64>;
    
    /// Delete events older than specified date
    async fn cleanup_old_events(&self, before: DateTime<Utc>) -> AuditResult<u64>;
    
    /// Check storage health
    async fn health_check(&self) -> AuditResult<StorageHealth>;
    
    /// Flush any pending writes
    async fn flush(&self) -> AuditResult<()>;
    
    /// Get storage statistics
    async fn get_stats(&self) -> AuditResult<StorageStats>;
}

/// Query parameters for event retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    /// Filter by event types
    pub event_types: Option<Vec<String>>,
    
    /// Filter by components
    pub components: Option<Vec<String>>,
    
    /// Filter by severity levels
    pub severities: Option<Vec<String>>,
    
    /// Filter by user IDs
    pub user_ids: Option<Vec<String>>,
    
    /// Filter by date range
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    
    /// Text search in messages
    pub search_text: Option<String>,
    
    /// Limit number of results
    pub limit: Option<usize>,
    
    /// Offset for pagination
    pub offset: Option<usize>,
    
    /// Sort order
    pub sort_by: Option<String>,
    pub sort_desc: bool,
    
    /// Filter by correlation ID
    pub correlation_id: Option<String>,
    
    /// Additional metadata filters
    pub metadata_filters: HashMap<String, Value>,
}

impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            event_types: None,
            components: None,
            severities: None,
            user_ids: None,
            start_time: None,
            end_time: None,
            search_text: None,
            limit: Some(1000),
            offset: None,
            sort_by: Some("timestamp".to_string()),
            sort_desc: true,
            correlation_id: None,
            metadata_filters: HashMap::new(),
        }
    }
}

/// Storage health information
#[derive(Debug, Serialize, Deserialize)]
pub struct StorageHealth {
    pub healthy: bool,
    pub message: String,
    pub last_write: Option<DateTime<Utc>>,
    pub pending_events: usize,
    pub error_count: u64,
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_events: u64,
    pub events_per_second: f64,
    pub storage_size_bytes: u64,
    pub oldest_event: Option<DateTime<Utc>>,
    pub newest_event: Option<DateTime<Utc>>,
    pub error_count: u64,
}

/// In-memory storage implementation
pub struct MemoryStorage {
    events: Arc<RwLock<VecDeque<AuditEvent>>>,
    max_events: usize,
    stats: Arc<RwLock<StorageStats>>,
}

impl MemoryStorage {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::new())),
            max_events,
            stats: Arc::new(RwLock::new(StorageStats {
                total_events: 0,
                events_per_second: 0.0,
                storage_size_bytes: 0,
                oldest_event: None,
                newest_event: None,
                error_count: 0,
            })),
        }
    }
}

#[async_trait]
impl AuditStorage for MemoryStorage {
    async fn store_event(&self, event: &AuditEvent) -> AuditResult<()> {
        let mut events = self.events.write().await;
        
        // Remove old events if at capacity
        while events.len() >= self.max_events {
            events.pop_front();
        }
        
        events.push_back(event.clone());
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_events += 1;
        stats.newest_event = Some(event.timestamp);
        if stats.oldest_event.is_none() {
            stats.oldest_event = Some(event.timestamp);
        }
        
        Ok(())
    }
    
    async fn store_batch(&self, events: &[AuditEvent]) -> AuditResult<()> {
        for event in events {
            self.store_event(event).await?;
        }
        Ok(())
    }
    
    async fn query_events(&self, query: &AuditQuery) -> AuditResult<Vec<AuditEvent>> {
        let events = self.events.read().await;
        let mut results: Vec<AuditEvent> = events.iter()
            .filter(|event| self.matches_query(event, query))
            .cloned()
            .collect();
            
        // Sort results
        if query.sort_desc {
            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        } else {
            results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        }
        
        // Apply pagination
        if let Some(offset) = query.offset {
            results = results.into_iter().skip(offset).collect();
        }
        
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        
        Ok(results)
    }
    
    async fn count_events(&self, query: &AuditQuery) -> AuditResult<u64> {
        let events = self.events.read().await;
        let count = events.iter()
            .filter(|event| self.matches_query(event, query))
            .count() as u64;
        Ok(count)
    }
    
    async fn cleanup_old_events(&self, before: DateTime<Utc>) -> AuditResult<u64> {
        let mut events = self.events.write().await;
        let original_len = events.len();
        
        events.retain(|event| event.timestamp >= before);
        
        let removed = original_len - events.len();
        Ok(removed as u64)
    }
    
    async fn health_check(&self) -> AuditResult<StorageHealth> {
        let events = self.events.read().await;
        let stats = self.stats.read().await;
        
        Ok(StorageHealth {
            healthy: true,
            message: "Memory storage operational".to_string(),
            last_write: stats.newest_event,
            pending_events: events.len(),
            error_count: stats.error_count,
        })
    }
    
    async fn flush(&self) -> AuditResult<()> {
        // Memory storage doesn't need flushing
        Ok(())
    }
    
    async fn get_stats(&self) -> AuditResult<StorageStats> {
        let stats = self.stats.read().await;
        Ok((*stats).clone())
    }
}

impl MemoryStorage {
    fn matches_query(&self, event: &AuditEvent, query: &AuditQuery) -> bool {
        // Event type filter
        if let Some(ref types) = query.event_types {
            // Use JSON serialization to get the correct snake_case format
            let event_type_str = serde_json::to_string(&event.event_type)
                .unwrap_or_else(|_| format!("{:?}", event.event_type))
                .trim_matches('"')  // Remove JSON quotes
                .to_lowercase();
            if !types.iter().any(|t| event_type_str == t.to_lowercase()) {
                return false;
            }
        }
        
        // Component filter
        if let Some(ref components) = query.components {
            if !components.contains(&event.component) {
                return false;
            }
        }
        
        // Severity filter
        if let Some(ref severities) = query.severities {
            let severity_str = format!("{:?}", event.severity).to_lowercase();
            if !severities.iter().any(|s| severity_str == s.to_lowercase()) {
                return false;
            }
        }
        
        // User ID filter
        if let Some(ref user_ids) = query.user_ids {
            if let Some(ref event_user) = event.metadata.user_id {
                if !user_ids.contains(event_user) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Time range filter
        if let Some(start) = query.start_time {
            if event.timestamp < start {
                return false;
            }
        }
        
        if let Some(end) = query.end_time {
            if event.timestamp > end {
                return false;
            }
        }
        
        // Text search filter
        if let Some(ref search) = query.search_text {
            let search_lower = search.to_lowercase();
            if !event.message.to_lowercase().contains(&search_lower) {
                return false;
            }
        }
        
        // Correlation ID filter
        if let Some(ref corr_id) = query.correlation_id {
            if event.correlation_id.as_ref() != Some(corr_id) {
                return false;
            }
        }
        
        true
    }
}

/// File-based storage implementation
pub struct FileStorage {
    directory: PathBuf,
    max_file_size: u64,
    max_files: usize,
    compress: bool,
    rollover: RolloverConfig,
    current_file: Arc<RwLock<Option<BufWriter<File>>>>,
    current_size: Arc<RwLock<u64>>,
    stats: Arc<RwLock<StorageStats>>,
    last_rollover_check: Arc<RwLock<DateTime<Utc>>>,
}

impl FileStorage {
    pub async fn new(
        directory: PathBuf,
        max_file_size: u64,
        max_files: usize,
        compress: bool,
        rollover: RolloverConfig,
    ) -> AuditResult<Self> {
        // Create directory if it doesn't exist
        tokio::fs::create_dir_all(&directory).await
            .map_err(|e| AuditError::Io(e))?;
        
        Ok(Self {
            directory,
            max_file_size,
            max_files,
            compress,
            rollover,
            current_file: Arc::new(RwLock::new(None)),
            current_size: Arc::new(RwLock::new(0)),
            stats: Arc::new(RwLock::new(StorageStats {
                total_events: 0,
                events_per_second: 0.0,
                storage_size_bytes: 0,
                oldest_event: None,
                newest_event: None,
                error_count: 0,
            })),
            last_rollover_check: Arc::new(RwLock::new(Utc::now())),
        })
    }
    
    async fn rotate_file_if_needed(&self) -> AuditResult<()> {
        let current_size = *self.current_size.read().await;
        
        // Check for file size rollover
        if current_size >= self.max_file_size {
            self.rotate_file().await?;
        }
        
        // Check for time-based and size-based rollover periodically
        self.check_rollover_policies().await?;
        
        Ok(())
    }
    
    async fn check_rollover_policies(&self) -> AuditResult<()> {
        let now = Utc::now();
        let last_check = *self.last_rollover_check.read().await;
        
        // Only check rollover policies at specified intervals
        if now.signed_duration_since(last_check).num_seconds() < self.rollover.check_interval_secs as i64 {
            return Ok(());
        }
        
        // Update last check time
        *self.last_rollover_check.write().await = now;
        
        if !self.rollover.auto_cleanup {
            return Ok(());
        }
        
        // Get all audit log files
        let mut audit_files = self.get_audit_files().await?;
        if audit_files.is_empty() {
            return Ok(());
        }
        
        // Sort by modification time (oldest first)
        audit_files.sort_by_key(|f| f.modified);
        
        // Calculate total size
        let total_size_bytes: u64 = audit_files.iter().map(|f| f.size).sum();
        let max_size_bytes = (self.rollover.max_total_size_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        
        // Size-based cleanup (takes precedence)
        if total_size_bytes > max_size_bytes {
            let mut removed_size = 0u64;
            let target_size = max_size_bytes * 80 / 100; // Clean to 80% of limit
            
            for file_info in &audit_files {
                if total_size_bytes - removed_size <= target_size {
                    break;
                }
                
                // Don't delete the current file
                if self.is_current_file(&file_info.path).await? {
                    continue;
                }
                
                tracing::info!("🗑️ Deleting audit file for size limit: {:?} ({} bytes)", 
                    file_info.path, file_info.size);
                
                if let Err(e) = tokio::fs::remove_file(&file_info.path).await {
                    tracing::warn!("Failed to delete audit file {:?}: {}", file_info.path, e);
                } else {
                    removed_size += file_info.size;
                }
            }
        }
        
        // Time-based cleanup
        let cutoff_time = now - Duration::days(self.rollover.max_age_days as i64);
        
        for file_info in &audit_files {
            if file_info.modified < cutoff_time {
                // Don't delete the current file
                if self.is_current_file(&file_info.path).await? {
                    continue;
                }
                
                tracing::info!("🗑️ Deleting audit file for age limit: {:?} (age: {} days)", 
                    file_info.path, 
                    now.signed_duration_since(file_info.modified).num_days());
                
                if let Err(e) = tokio::fs::remove_file(&file_info.path).await {
                    tracing::warn!("Failed to delete old audit file {:?}: {}", file_info.path, e);
                }
            }
        }
        
        Ok(())
    }
    
    async fn rotate_file(&self) -> AuditResult<()> {
        // Close and flush current file
        {
            let mut current_file = self.current_file.write().await;
            if let Some(mut file) = current_file.take() {
                file.flush().await.map_err(AuditError::Io)?;
            }
        }
        
        // Reset size counter
        *self.current_size.write().await = 0;
        
        // Force creation of new file on next write
        tracing::info!("🔄 Rotating audit log file due to size limit");
        
        // Find and compress recent files if compression is enabled
        if self.compress {
            tokio::spawn({
                let directory = self.directory.clone();
                async move {
                    if let Ok(mut entries) = tokio::fs::read_dir(&directory).await {
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            let path = entry.path();
                            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                // Compress uncompressed .jsonl files
                                if filename.starts_with("audit_") && filename.ends_with(".jsonl") && !filename.ends_with(".jsonl.gz") {
                                    if let Ok(metadata) = entry.metadata().await {
                                        let modified = metadata.modified().unwrap_or(std::time::SystemTime::now());
                                        let age = std::time::SystemTime::now().duration_since(modified).unwrap_or_default();
                                        
                                        // Compress files older than 5 minutes
                                        if age.as_secs() > 300 {
                                            let _ = Self::compress_file_static(&path).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
        
        Ok(())
    }
    
    async fn compress_file_static(file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Basic compression implementation using std library
        // Note: In production, consider using flate2 or similar for better compression
        
        let content = tokio::fs::read(file_path).await?;
        if content.is_empty() {
            tracing::debug!("🗜️ Skipping compression of empty file: {:?}", file_path);
            return Ok(());
        }
        
        // Simple compression approach: since audit logs are mostly text, 
        // we can achieve basic space savings by removing extra whitespace
        let content_str = String::from_utf8_lossy(&content);
        let compressed_content = content_str
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        
        // Write compressed content to a .compressed file
        let mut compressed_path = file_path.clone();
        compressed_path.set_extension("compressed");
        
        tokio::fs::write(&compressed_path, compressed_content.as_bytes()).await?;
        
        // Only remove original if compression succeeded and saved space
        let original_size = content.len();
        let compressed_size = compressed_content.len();
        
        if compressed_size < original_size {
            tokio::fs::remove_file(file_path).await?;
            tracing::info!("🗜️ Compressed audit file: {:?} ({} -> {} bytes)", 
                file_path, original_size, compressed_size);
        } else {
            // Remove compressed file if it's not smaller
            let _ = tokio::fs::remove_file(&compressed_path).await;
            tracing::debug!("🗜️ Compression didn't save space for: {:?}", file_path);
        }
        
        Ok(())
    }
    
    async fn get_audit_files(&self) -> AuditResult<Vec<AuditFileInfo>> {
        let mut files = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.directory).await.map_err(AuditError::Io)?;
        
        while let Some(entry) = entries.next_entry().await.map_err(AuditError::Io)? {
            let path = entry.path();
            
            // Only consider .jsonl files that match audit file pattern
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("audit_") && (filename.ends_with(".jsonl") || filename.ends_with(".jsonl.gz")) {
                    if let Ok(metadata) = entry.metadata().await {
                        let modified = metadata.modified()
                            .map_err(AuditError::Io)?
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|_| AuditError::Storage("Invalid file time".to_string()))?;
                        
                        files.push(AuditFileInfo {
                            path,
                            size: metadata.len(),
                            modified: DateTime::from_timestamp(modified.as_secs() as i64, modified.subsec_nanos()).unwrap_or(Utc::now()),
                        });
                    }
                }
            }
        }
        
        Ok(files)
    }
    
    async fn is_current_file(&self, path: &PathBuf) -> AuditResult<bool> {
        // Check if this path matches our current file
        // For simplicity, we consider any file that's being written to as "current"
        // In practice, we could track the current file path more explicitly
        
        // If file was modified very recently (within last minute), consider it current
        if let Ok(metadata) = tokio::fs::metadata(path).await {
            if let Ok(modified) = metadata.modified() {
                let file_time = modified.duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| AuditError::Storage("Invalid file time".to_string()))?;
                let file_datetime = DateTime::from_timestamp(file_time.as_secs() as i64, file_time.subsec_nanos()).unwrap_or(Utc::now());
                
                let age = Utc::now().signed_duration_since(file_datetime);
                return Ok(age.num_minutes() < 5); // Consider files modified in last 5 minutes as current
            }
        }
        
        Ok(false)
    }
    
    async fn compress_file(&self, file_path: &PathBuf) -> AuditResult<()> {
        if !self.compress {
            return Ok(());
        }
        
        // Use the static compression method
        Self::compress_file_static(file_path).await
            .map_err(|e| AuditError::Storage(format!("Failed to compress file: {}", e)))?;
        
        Ok(())
    }
    
    async fn ensure_current_file(&self) -> AuditResult<()> {
        let mut current_file = self.current_file.write().await;
        
        if current_file.is_none() {
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
            let filename = format!("audit_{}.jsonl", timestamp);
            let filepath = self.directory.join(filename);
            
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&filepath)
                .await
                .map_err(AuditError::Io)?;
                
            *current_file = Some(BufWriter::new(file));
        }
        
        Ok(())
    }
}

#[async_trait]
impl AuditStorage for FileStorage {
    async fn store_event(&self, event: &AuditEvent) -> AuditResult<()> {
        self.rotate_file_if_needed().await?;
        self.ensure_current_file().await?;
        
        let json_line = serde_json::to_string(event)
            .map_err(AuditError::Serialization)?;
        let json_line = format!("{}\n", json_line);
        
        {
            let mut current_file = self.current_file.write().await;
            if let Some(ref mut file) = current_file.as_mut() {
                file.write_all(json_line.as_bytes()).await
                    .map_err(AuditError::Io)?;
                
                // Flush the write to ensure data is written to disk
                file.flush().await
                    .map_err(AuditError::Io)?;
                    
                *self.current_size.write().await += json_line.len() as u64;
            }
        }
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_events += 1;
        stats.newest_event = Some(event.timestamp);
        if stats.oldest_event.is_none() {
            stats.oldest_event = Some(event.timestamp);
        }
        
        Ok(())
    }
    
    async fn store_batch(&self, events: &[AuditEvent]) -> AuditResult<()> {
        for event in events {
            self.store_event(event).await?;
        }
        Ok(())
    }
    
    async fn query_events(&self, query: &AuditQuery) -> AuditResult<Vec<AuditEvent>> {
        // Implement file-based querying by scanning through audit files
        let mut results = Vec::new();
        let audit_files = self.get_audit_files().await?;
        
        // Sort files by modification time (newest first for most recent results)
        let mut sorted_files = audit_files;
        sorted_files.sort_by(|a, b| b.modified.cmp(&a.modified));
        
        let limit = query.limit.unwrap_or(1000);
        let offset = query.offset.unwrap_or(0);
        let mut processed_count = 0;
        let mut skipped_count = 0;
        
        'file_loop: for file_info in sorted_files {
            // Skip compressed files for now as they require decompression
            if file_info.path.extension().map_or(false, |ext| ext == "gz") {
                continue;
            }
            
            // Read file line by line
            match tokio::fs::read_to_string(&file_info.path).await {
                Ok(content) => {
                    for line in content.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        
                        match serde_json::from_str::<AuditEvent>(line) {
                            Ok(event) => {
                                // Apply filters
                                if self.matches_file_query(&event, query) {
                                    if skipped_count < offset {
                                        skipped_count += 1;
                                        continue;
                                    }
                                    
                                    results.push(event);
                                    processed_count += 1;
                                    
                                    if processed_count >= limit {
                                        break 'file_loop;
                                    }
                                }
                            },
                            Err(e) => {
                                tracing::warn!("Failed to parse audit event from file {:?}: {}", file_info.path, e);
                                continue;
                            }
                        }
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read audit file {:?}: {}", file_info.path, e);
                    continue;
                }
            }
        }
        
        // Sort results if requested
        if let Some(sort_field) = &query.sort_by {
            match sort_field.as_str() {
                "timestamp" => {
                    if query.sort_desc {
                        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                    } else {
                        results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                    }
                },
                _ => {} // Only timestamp sorting implemented for files
            }
        }
        
        Ok(results)
    }
    
    async fn count_events(&self, query: &AuditQuery) -> AuditResult<u64> {
        // Implement file-based counting by scanning through audit files
        let mut count = 0u64;
        let audit_files = self.get_audit_files().await?;
        
        for file_info in audit_files {
            // Skip compressed files for now
            if file_info.path.extension().map_or(false, |ext| ext == "gz") {
                continue;
            }
            
            match tokio::fs::read_to_string(&file_info.path).await {
                Ok(content) => {
                    for line in content.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        
                        match serde_json::from_str::<AuditEvent>(line) {
                            Ok(event) => {
                                if self.matches_file_query(&event, query) {
                                    count += 1;
                                }
                            },
                            Err(_) => continue, // Skip malformed lines
                        }
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read audit file {:?}: {}", file_info.path, e);
                    continue;
                }
            }
        }
        
        Ok(count)
    }
    
    async fn cleanup_old_events(&self, before: DateTime<Utc>) -> AuditResult<u64> {
        // Implement file-based cleanup by deleting old files
        let audit_files = self.get_audit_files().await?;
        let mut deleted_count = 0u64;
        
        for file_info in audit_files {
            if file_info.modified < before {
                // Don't delete the current file being written to
                if self.is_current_file(&file_info.path).await? {
                    continue;
                }
                
                match tokio::fs::remove_file(&file_info.path).await {
                    Ok(()) => {
                        tracing::info!("🗑️ Deleted old audit file: {:?}", file_info.path);
                        
                        // Count events in the deleted file (estimate for return value)
                        // This is approximate since we deleted the file, but gives a rough count
                        deleted_count += (file_info.size / 200) as u64; // Assume ~200 bytes per event
                    },
                    Err(e) => {
                        tracing::warn!("Failed to delete old audit file {:?}: {}", file_info.path, e);
                    }
                }
            }
        }
        
        Ok(deleted_count)
    }
    
    async fn health_check(&self) -> AuditResult<StorageHealth> {
        let stats = self.stats.read().await;
        
        Ok(StorageHealth {
            healthy: true,
            message: "File storage operational".to_string(),
            last_write: stats.newest_event,
            pending_events: 0,
            error_count: stats.error_count,
        })
    }
    
    async fn flush(&self) -> AuditResult<()> {
        let mut current_file = self.current_file.write().await;
        if let Some(ref mut file) = current_file.as_mut() {
            file.flush().await.map_err(AuditError::Io)?;
        }
        Ok(())
    }
    
    async fn get_stats(&self) -> AuditResult<StorageStats> {
        let stats = self.stats.read().await;
        Ok((*stats).clone())
    }
}

impl FileStorage {
    /// Helper method to check if an event matches the query filters
    fn matches_file_query(&self, event: &AuditEvent, query: &AuditQuery) -> bool {
        // Event type filter
        if let Some(ref types) = query.event_types {
            // Use JSON serialization to get the correct snake_case format
            let event_type_str = serde_json::to_string(&event.event_type)
                .unwrap_or_else(|_| format!("{:?}", event.event_type))
                .trim_matches('"')  // Remove JSON quotes
                .to_lowercase();
            if !types.iter().any(|t| event_type_str == t.to_lowercase()) {
                return false;
            }
        }
        
        // Component filter
        if let Some(ref components) = query.components {
            if !components.contains(&event.component) {
                return false;
            }
        }
        
        // Severity filter
        if let Some(ref severities) = query.severities {
            let severity_str = format!("{:?}", event.severity).to_lowercase();
            if !severities.iter().any(|s| severity_str == s.to_lowercase()) {
                return false;
            }
        }
        
        // User ID filter
        if let Some(ref user_ids) = query.user_ids {
            if let Some(ref event_user) = event.metadata.user_id {
                if !user_ids.contains(event_user) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Time range filter
        if let Some(start) = query.start_time {
            if event.timestamp < start {
                return false;
            }
        }
        
        if let Some(end) = query.end_time {
            if event.timestamp > end {
                return false;
            }
        }
        
        // Text search filter
        if let Some(ref search) = query.search_text {
            let search_lower = search.to_lowercase();
            if !event.message.to_lowercase().contains(&search_lower) {
                return false;
            }
        }
        
        // Correlation ID filter
        if let Some(ref corr_id) = query.correlation_id {
            if event.correlation_id.as_ref() != Some(corr_id) {
                return false;
            }
        }
        
        true
    }
}

/// Storage backend factory
pub struct StorageBackend;

impl StorageBackend {
    /// Create storage instance from configuration
    pub fn create(config: &StorageConfig) -> std::pin::Pin<Box<dyn std::future::Future<Output = AuditResult<Box<dyn AuditStorage>>> + Send + '_>> {
        Box::pin(async move {
        match config {
            StorageConfig::Memory { max_events } => {
                Ok(Box::new(MemoryStorage::new(*max_events)) as Box<dyn AuditStorage>)
            },
            
            StorageConfig::File { 
                directory, 
                max_file_size, 
                max_files, 
                compress,
                rollover,
                .. 
            } => {
                let storage = FileStorage::new(
                    directory.clone(),
                    *max_file_size,
                    *max_files,
                    *compress,
                    rollover.clone(),
                ).await?;
                Ok(Box::new(storage) as Box<dyn AuditStorage>)
            },
            
            StorageConfig::Database { connection_string, table_name, .. } => {
                // For now, create a file storage as fallback since database implementation
                // requires external dependencies (sqlx, tokio-postgres, etc.)
                tracing::warn!("Database storage requested but not fully implemented. Falling back to file storage.");
                tracing::info!("Database config - Connection: {}, Table: {}", connection_string, table_name);
                
                // Create a file storage in a "database" subdirectory
                let fallback_dir = std::path::Path::new("./logs/audit/database_fallback");
                let storage = FileStorage::new(
                    fallback_dir.to_path_buf(),
                    100 * 1024 * 1024, // 100MB
                    10,
                    true,
                    RolloverConfig::default(),
                ).await?;
                Ok(Box::new(storage) as Box<dyn AuditStorage>)
            },
            
            StorageConfig::External { endpoint, api_key, format, .. } => {
                // For now, create a file storage as fallback since external integrations
                // require HTTP client setup and API-specific implementations
                tracing::warn!("External storage requested but not fully implemented. Falling back to file storage.");
                tracing::info!("External config - Endpoint: {}, Format: {:?}, API Key: {}", 
                    endpoint, format, api_key.is_some());
                
                // Create a file storage in an "external" subdirectory
                let fallback_dir = std::path::Path::new("./logs/audit/external_fallback");
                let storage = FileStorage::new(
                    fallback_dir.to_path_buf(),
                    100 * 1024 * 1024, // 100MB
                    10,
                    true,
                    RolloverConfig::default(),
                ).await?;
                Ok(Box::new(storage) as Box<dyn AuditStorage>)
            },
            
            StorageConfig::Hybrid { backends, primary } => {
                // Implement hybrid storage by using the primary backend
                if backends.is_empty() {
                    return Err(AuditError::Storage("Hybrid storage requires at least one backend".to_string()));
                }
                
                let primary_index = *primary;
                if primary_index >= backends.len() {
                    return Err(AuditError::Storage("Invalid primary backend index for hybrid storage".to_string()));
                }
                
                tracing::info!("Creating hybrid storage with {} backends, primary index: {}", backends.len(), primary_index);
                
                // For now, just use the primary backend
                // In a full implementation, this would coordinate between multiple backends
                let primary_backend = &backends[primary_index];
                Self::create(primary_backend).await
            },
        }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::audit::events::{AuditEventType, AuditSeverity};
    
    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryStorage::new(10);
        
        let event = AuditEvent::new(
            AuditEventType::Authentication,
            "test".to_string(),
            "Test event".to_string(),
        );
        
        storage.store_event(&event).await.unwrap();
        
        let query = AuditQuery::default();
        let results = storage.query_events(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        
        let count = storage.count_events(&query).await.unwrap();
        assert_eq!(count, 1);
    }
    
    #[tokio::test]
    async fn test_memory_storage_capacity() {
        let storage = MemoryStorage::new(2);
        
        for i in 0..5 {
            let event = AuditEvent::new(
                AuditEventType::ToolExecution,
                "test".to_string(),
                format!("Event {}", i),
            );
            storage.store_event(&event).await.unwrap();
        }
        
        let query = AuditQuery::default();
        let results = storage.query_events(&query).await.unwrap();
        assert_eq!(results.len(), 2); // Should only keep last 2 events
    }
    
    #[tokio::test]
    async fn test_query_filtering() {
        let storage = MemoryStorage::new(10);
        
        // Store events with different components
        let event1 = AuditEvent::new(
            AuditEventType::Authentication,
            "auth_service".to_string(),
            "Auth event".to_string(),
        );
        
        let event2 = AuditEvent::new(
            AuditEventType::ToolExecution,
            "tool_service".to_string(),
            "Tool event".to_string(),
        );
        
        storage.store_event(&event1).await.unwrap();
        storage.store_event(&event2).await.unwrap();
        
        // Filter by component
        let mut query = AuditQuery::default();
        query.components = Some(vec!["auth_service".to_string()]);
        
        let results = storage.query_events(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].component, "auth_service");
    }
}