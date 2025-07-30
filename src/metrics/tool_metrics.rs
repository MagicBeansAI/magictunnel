//! Tool-level Metrics Collection and Analysis
//!
//! This module provides comprehensive metrics collection and analysis for individual tools,
//! tracking usage patterns, performance metrics, discovery rankings, and execution statistics.

use chrono::{DateTime, Utc, Timelike, Datelike};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs;
use tracing::{debug, info, warn, error};

/// Execution result for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolExecutionResult {
    /// Successful execution
    Success {
        output_size: usize,
        output_type: String, // "text", "json", "binary", etc.
    },
    /// Failed execution with error details
    Error {
        error_type: String,
        error_message: String,
        is_timeout: bool,
    },
    /// Execution was cancelled/interrupted
    Cancelled,
}

/// Smart discovery ranking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRanking {
    /// Position in smart discovery results (1-based)
    pub position: u32,
    /// Confidence score from smart discovery
    pub confidence_score: f64,
    /// Discovery method used ("rule_based", "llm_based", "semantic")
    pub discovery_method: String,
    /// Query/request that led to this ranking
    pub query: String,
    /// Timestamp of the discovery
    pub timestamp: DateTime<Utc>,
}

/// Individual tool execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRecord {
    /// Unique execution ID
    pub execution_id: String,
    /// Tool name
    pub tool_name: String,
    /// Execution start time
    pub start_time: DateTime<Utc>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Execution result
    pub result: ToolExecutionResult,
    /// Input parameters (anonymized for privacy)
    pub input_hash: String,
    /// Discovery context if this was from smart discovery
    pub discovery_context: Option<DiscoveryRanking>,
    /// Source of execution ("direct", "smart_discovery", "api", "mcp")
    pub execution_source: String,
    /// MCP server/service that executed this tool (if applicable)
    pub service_source: Option<String>,
}

/// Aggregated metrics for a specific tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    /// Tool name
    pub tool_name: String,
    /// Tool category/group
    pub category: String,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
    
    // Execution Statistics
    /// Total number of executions
    pub total_executions: u64,
    /// Number of successful executions
    pub successful_executions: u64,
    /// Number of failed executions
    pub failed_executions: u64,
    /// Number of timeouts
    pub timeout_count: u64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    
    // Performance Metrics
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,
    /// Median execution time in milliseconds
    pub median_execution_time_ms: f64,
    /// 95th percentile execution time
    pub p95_execution_time_ms: f64,
    /// Recent execution times (sliding window)
    pub recent_execution_times: VecDeque<u64>,
    
    // Discovery & Ranking Metrics
    /// Number of times this tool appeared in top 30 discovery results
    pub top_30_appearances: u64,
    /// Number of times this tool appeared in top 10 discovery results
    pub top_10_appearances: u64,
    /// Number of times this tool appeared in top 3 discovery results
    pub top_3_appearances: u64,
    /// Average confidence score when discovered
    pub avg_confidence_score: f64,
    /// Average position in discovery results
    pub avg_discovery_position: f64,
    /// Recent confidence scores (sliding window)
    pub recent_confidence_scores: VecDeque<f64>,
    
    // Usage Patterns
    /// Most common execution source
    pub primary_execution_source: String,
    /// Execution sources distribution
    pub execution_sources: HashMap<String, u64>,
    /// Most common error types
    pub error_types: HashMap<String, u64>,
    /// Most active time periods (hour of day)
    pub usage_by_hour: HashMap<u8, u64>, // 0-23
    /// Most active days of week
    pub usage_by_day: HashMap<u8, u64>, // 0-6 (Sunday-Saturday)
    
    // Temporal Metrics
    /// First time this tool was executed
    pub first_execution: Option<DateTime<Utc>>,
    /// Last time this tool was executed
    pub last_execution: Option<DateTime<Utc>>,
    /// Last time this tool was successfully executed
    pub last_successful_execution: Option<DateTime<Utc>>,
    
    // Quality Metrics
    /// Average output size in bytes
    pub avg_output_size: f64,
    /// Output type distribution
    pub output_types: HashMap<String, u64>,
}

impl ToolMetrics {
    /// Create new tool metrics
    pub fn new(tool_name: String, category: String) -> Self {
        Self {
            tool_name,
            category,
            last_updated: Utc::now(),
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            timeout_count: 0,
            success_rate: 0.0,
            avg_execution_time_ms: 0.0,
            median_execution_time_ms: 0.0,
            p95_execution_time_ms: 0.0,
            recent_execution_times: VecDeque::with_capacity(1000),
            top_30_appearances: 0,
            top_10_appearances: 0,
            top_3_appearances: 0,
            avg_confidence_score: 0.0,
            avg_discovery_position: 0.0,
            recent_confidence_scores: VecDeque::with_capacity(1000),
            primary_execution_source: "unknown".to_string(),
            execution_sources: HashMap::new(),
            error_types: HashMap::new(),
            usage_by_hour: HashMap::new(),
            usage_by_day: HashMap::new(),
            first_execution: None,
            last_execution: None,
            last_successful_execution: None,
            avg_output_size: 0.0,
            output_types: HashMap::new(),
        }
    }
    
    /// Record a tool execution
    pub fn record_execution(&mut self, record: &ToolExecutionRecord) {
        self.last_updated = Utc::now();
        self.total_executions += 1;
        self.last_execution = Some(record.start_time);
        
        // Update execution times
        self.recent_execution_times.push_back(record.duration_ms);
        if self.recent_execution_times.len() > 1000 {
            self.recent_execution_times.pop_front();
        }
        
        // Update execution sources
        *self.execution_sources.entry(record.execution_source.clone()).or_insert(0) += 1;
        
        // Update temporal patterns
        let hour = record.start_time.hour() as u8;
        let day = record.start_time.weekday().num_days_from_sunday() as u8;
        *self.usage_by_hour.entry(hour).or_insert(0) += 1;
        *self.usage_by_day.entry(day).or_insert(0) += 1;
        
        // Set first execution if this is the first
        if self.first_execution.is_none() {
            self.first_execution = Some(record.start_time);
        }
        
        // Handle execution result
        match &record.result {
            ToolExecutionResult::Success { output_size, output_type } => {
                self.successful_executions += 1;
                self.last_successful_execution = Some(record.start_time);
                
                // Update output metrics
                *self.output_types.entry(output_type.clone()).or_insert(0) += 1;
                self.avg_output_size = (self.avg_output_size * (self.successful_executions - 1) as f64 + *output_size as f64) / self.successful_executions as f64;
            }
            ToolExecutionResult::Error { error_type, is_timeout, .. } => {
                self.failed_executions += 1;
                *self.error_types.entry(error_type.clone()).or_insert(0) += 1;
                if *is_timeout {
                    self.timeout_count += 1;
                }
            }
            ToolExecutionResult::Cancelled => {
                self.failed_executions += 1;
                *self.error_types.entry("cancelled".to_string()).or_insert(0) += 1;
            }
        }
        
        // Record discovery ranking if available
        if let Some(discovery) = &record.discovery_context {
            self.record_discovery_ranking(discovery);
        }
        
        // Update derived metrics
        self.update_derived_metrics();
    }
    
    /// Record a discovery ranking (when tool appears in discovery results)
    pub fn record_discovery_ranking(&mut self, ranking: &DiscoveryRanking) {
        // Update ranking counts based on position
        if ranking.position <= 30 {
            self.top_30_appearances += 1;
        }
        if ranking.position <= 10 {
            self.top_10_appearances += 1;
        }
        if ranking.position <= 3 {  
            self.top_3_appearances += 1;
        }
        
        // Update confidence scores
        self.recent_confidence_scores.push_back(ranking.confidence_score);
        if self.recent_confidence_scores.len() > 1000 {
            self.recent_confidence_scores.pop_front();
        }
        
        // Update average confidence score
        let total_discoveries = self.top_30_appearances;
        if total_discoveries > 0 {
            self.avg_confidence_score = (self.avg_confidence_score * (total_discoveries - 1) as f64 + ranking.confidence_score) / total_discoveries as f64;
            self.avg_discovery_position = (self.avg_discovery_position * (total_discoveries - 1) as f64 + ranking.position as f64) / total_discoveries as f64;
        }
    }
    
    /// Update derived metrics (success rate, percentiles, etc.)
    fn update_derived_metrics(&mut self) {
        // Update success rate
        if self.total_executions > 0 {
            self.success_rate = self.successful_executions as f64 / self.total_executions as f64;
        }
        
        // Update execution time statistics
        if !self.recent_execution_times.is_empty() {
            let mut times: Vec<u64> = self.recent_execution_times.iter().cloned().collect();
            times.sort_unstable();
            
            // Calculate average
            self.avg_execution_time_ms = times.iter().sum::<u64>() as f64 / times.len() as f64;
            
            // Calculate median
            let mid = times.len() / 2;
            self.median_execution_time_ms = if times.len() % 2 == 0 {
                (times[mid - 1] + times[mid]) as f64 / 2.0
            } else {
                times[mid] as f64
            };
            
            // Calculate 95th percentile
            let p95_idx = ((times.len() as f64) * 0.95) as usize;
            if p95_idx < times.len() {
                self.p95_execution_time_ms = times[p95_idx] as f64;
            }
        }
        
        // Update primary execution source
        if let Some((source, _)) = self.execution_sources.iter().max_by_key(|(_, count)| *count) {
            self.primary_execution_source = source.clone();
        }
    }
}

/// Tool metrics summary for overview dashboards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetricsSummary {
    /// Total number of tools tracked
    pub total_tools: u32,
    /// Number of actively used tools (executed in last 24h)
    pub active_tools: u32,
    /// Number of tools with high success rate (>95%)
    pub high_performing_tools: u32,
    /// Number of tools with low success rate (<50%)
    pub low_performing_tools: u32,
    /// Total executions across all tools
    pub total_executions: u64,
    /// Total successful executions
    pub total_successful_executions: u64,
    /// Overall success rate
    pub overall_success_rate: f64,
    /// Average execution time across all tools
    pub avg_execution_time_ms: f64,
    /// Most popular tool (by execution count)
    pub most_popular_tool: Option<String>,
    /// Most reliable tool (by success rate)
    pub most_reliable_tool: Option<String>,
    /// Fastest tool (by avg execution time)
    pub fastest_tool: Option<String>,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Main tool metrics collector
/// Persistent storage structure for tool metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetricsStorage {
    /// Metrics for each tool
    pub tool_metrics: HashMap<String, ToolMetrics>,
    /// Recent execution records
    pub execution_history: VecDeque<ToolExecutionRecord>,
    /// Last saved timestamp
    pub last_saved: DateTime<Utc>,
}

pub struct ToolMetricsCollector {
    /// Metrics storage for each tool
    tool_metrics: Arc<RwLock<HashMap<String, ToolMetrics>>>,
    /// Recent execution records (for detailed analysis)
    execution_history: Arc<RwLock<VecDeque<ToolExecutionRecord>>>,
    /// Maximum number of execution records to keep
    max_history_size: usize,
    /// Path to persistent storage file
    storage_path: Option<String>,
}

impl ToolMetricsCollector {
    /// Create a new tool metrics collector
    pub fn new(max_history_size: usize) -> Self {
        Self {
            tool_metrics: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history_size))),
            max_history_size,
            storage_path: None,
        }
    }

    /// Create a new tool metrics collector with persistent storage
    pub async fn new_with_storage<P: AsRef<Path>>(max_history_size: usize, storage_path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let storage_path_str = storage_path.as_ref().to_string_lossy().to_string();
        
        let mut collector = Self {
            tool_metrics: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history_size))),
            max_history_size,
            storage_path: Some(storage_path_str.clone()),
        };

        // Try to load existing data
        if let Err(e) = collector.load_from_disk().await {
            warn!("Failed to load tool metrics from disk: {}. Starting with empty metrics.", e);
        }

        Ok(collector)
    }

    /// Save metrics to disk
    pub async fn save_to_disk(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref storage_path) = self.storage_path {
            let tool_metrics = self.tool_metrics.read().await.clone();
            let execution_history = self.execution_history.read().await.clone();
            
            let storage = ToolMetricsStorage {
                tool_metrics,
                execution_history,
                last_saved: Utc::now(),
            };

            // Create directory if it doesn't exist
            if let Some(parent_dir) = Path::new(storage_path).parent() {
                fs::create_dir_all(parent_dir).await?;
            }

            let json_data = serde_json::to_string_pretty(&storage)?;
            fs::write(storage_path, json_data).await?;
            
            debug!("Saved tool metrics to {}", storage_path);
        }
        Ok(())
    }

    /// Load metrics from disk
    pub async fn load_from_disk(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref storage_path) = self.storage_path {
            if Path::new(storage_path).exists() {
                let json_data = fs::read_to_string(storage_path).await?;
                let storage: ToolMetricsStorage = serde_json::from_str(&json_data)?;
                
                // Update in-memory data
                *self.tool_metrics.write().await = storage.tool_metrics;
                *self.execution_history.write().await = storage.execution_history;
                
                info!("Loaded tool metrics from {}", storage_path);
            }
        }
        Ok(())
    }
    
    /// Initialize metrics tracking for a tool
    pub async fn initialize_tool(&self, tool_name: &str, category: &str) {
        let mut metrics = self.tool_metrics.write().await;
        if !metrics.contains_key(tool_name) {
            metrics.insert(tool_name.to_string(), ToolMetrics::new(tool_name.to_string(), category.to_string()));
            info!("📊 [TOOL_METRICS] Initialized metrics tracking for tool: {}", tool_name);
        }
    }
    
    /// Record a tool execution
    pub async fn record_execution(&self, record: ToolExecutionRecord) {
        // Ensure tool metrics exist
        {
            let mut metrics = self.tool_metrics.write().await;
            if !metrics.contains_key(&record.tool_name) {
                metrics.insert(record.tool_name.clone(), ToolMetrics::new(record.tool_name.clone(), "unknown".to_string()));
            }
        }
        
        // Update tool metrics
        {
            let mut metrics = self.tool_metrics.write().await;
            if let Some(tool_metrics) = metrics.get_mut(&record.tool_name) {
                tool_metrics.record_execution(&record);
            }
        }
        
        // Store execution record
        {
            let mut history = self.execution_history.write().await;
            history.push_back(record.clone());
            if history.len() > self.max_history_size {
                history.pop_front();
            }
        }
        
        match &record.result {
            ToolExecutionResult::Success { .. } => {
                debug!("✅ [TOOL_METRICS] Recorded successful execution for '{}': {}ms", record.tool_name, record.duration_ms);
            }
            ToolExecutionResult::Error { error_type, .. } => {
                debug!("❌ [TOOL_METRICS] Recorded failed execution for '{}': {} ({}ms)", record.tool_name, error_type, record.duration_ms);
            }
            ToolExecutionResult::Cancelled => {
                debug!("⏹️ [TOOL_METRICS] Recorded cancelled execution for '{}': {}ms", record.tool_name, record.duration_ms);
            }
        }

        // Save to disk (async, don't block on failure)
        if let Err(e) = self.save_to_disk().await {
            warn!("Failed to save tool metrics to disk: {}", e);
        }
    }
    
    /// Record a discovery ranking (when tool appears in smart discovery results)
    pub async fn record_discovery_ranking(&self, tool_name: &str, ranking: DiscoveryRanking) {
        {
            let mut metrics = self.tool_metrics.write().await;
            if let Some(tool_metrics) = metrics.get_mut(tool_name) {
                tool_metrics.record_discovery_ranking(&ranking);
                debug!("📈 [TOOL_METRICS] Recorded discovery ranking for '{}': position {} with confidence {:.2}", 
                       tool_name, ranking.position, ranking.confidence_score);
            }
        }

        // Save to disk (async, don't block on failure)
        if let Err(e) = self.save_to_disk().await {
            warn!("Failed to save tool metrics to disk: {}", e);
        }
    }
    
    /// Get metrics for a specific tool
    pub async fn get_tool_metrics(&self, tool_name: &str) -> Option<ToolMetrics> {
        let metrics = self.tool_metrics.read().await;
        metrics.get(tool_name).cloned()
    }
    
    /// Get metrics for all tools
    pub async fn get_all_tool_metrics(&self) -> HashMap<String, ToolMetrics> {
        let metrics = self.tool_metrics.read().await;
        metrics.clone()
    }
    
    /// Get tool metrics summary
    pub async fn get_summary(&self) -> ToolMetricsSummary {
        let metrics = self.tool_metrics.read().await;
        let now = Utc::now();
        let yesterday = now - chrono::Duration::hours(24);
        
        let mut summary = ToolMetricsSummary {
            total_tools: metrics.len() as u32,
            active_tools: 0,
            high_performing_tools: 0,
            low_performing_tools: 0,
            total_executions: 0,
            total_successful_executions: 0,
            overall_success_rate: 0.0,
            avg_execution_time_ms: 0.0,
            most_popular_tool: None,
            most_reliable_tool: None,
            fastest_tool: None,
            last_updated: now,
        };
        
        let mut most_popular_count = 0u64;
        let mut best_success_rate = 0.0f64;
        let mut fastest_time = f64::MAX;
        let mut total_avg_time = 0.0f64;
        let mut tools_with_executions = 0u32;
        
        for (tool_name, tool_metrics) in metrics.iter() {
            // Count active tools (executed in last 24h)
            if let Some(last_exec) = tool_metrics.last_execution {
                if last_exec > yesterday {
                    summary.active_tools += 1;
                }
            }
            
            // Count performance categories
            if tool_metrics.success_rate > 0.95 && tool_metrics.total_executions > 10 {
                summary.high_performing_tools += 1;
            } else if tool_metrics.success_rate < 0.5 && tool_metrics.total_executions > 10 {
                summary.low_performing_tools += 1;
            }
            
            // Aggregate execution statistics
            summary.total_executions += tool_metrics.total_executions;
            summary.total_successful_executions += tool_metrics.successful_executions;
            
            // Track most popular tool
            if tool_metrics.total_executions > most_popular_count {
                most_popular_count = tool_metrics.total_executions;
                summary.most_popular_tool = Some(tool_name.clone());
            }
            
            // Track most reliable tool (with minimum executions)
            if tool_metrics.total_executions > 10 && tool_metrics.success_rate > best_success_rate {
                best_success_rate = tool_metrics.success_rate;
                summary.most_reliable_tool = Some(tool_name.clone());
            }
            
            // Track fastest tool (with minimum executions)
            if tool_metrics.total_executions > 5 && tool_metrics.avg_execution_time_ms < fastest_time {
                fastest_time = tool_metrics.avg_execution_time_ms;
                summary.fastest_tool = Some(tool_name.clone());
            }
            
            // Aggregate average execution time
            if tool_metrics.total_executions > 0 {
                total_avg_time += tool_metrics.avg_execution_time_ms;
                tools_with_executions += 1;
            }
        }
        
        // Calculate overall metrics
        if summary.total_executions > 0 {
            summary.overall_success_rate = summary.total_successful_executions as f64 / summary.total_executions as f64;
        }
        
        if tools_with_executions > 0 {
            summary.avg_execution_time_ms = total_avg_time / tools_with_executions as f64;
        }
        
        summary
    }
    
    /// Get recent execution history
    pub async fn get_recent_executions(&self, limit: Option<usize>) -> Vec<ToolExecutionRecord> {
        let history = self.execution_history.read().await;
        let limit = limit.unwrap_or(100).min(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }
    
    /// Get top performing tools by various metrics
    pub async fn get_top_tools(&self, metric: &str, limit: usize) -> Vec<(String, f64)> {
        let metrics = self.tool_metrics.read().await;
        let mut tools: Vec<(String, f64)> = metrics.iter()
            .filter(|(_, m)| m.total_executions > 0) // Only include tools with executions
            .filter_map(|(name, m)| {
                let value = match metric {
                    "executions" => m.total_executions as f64,
                    "success_rate" => if m.total_executions > 10 { m.success_rate } else { return None }, // Filter out tools with ≤10 executions
                    "avg_execution_time" => if m.avg_execution_time_ms > 0.0 { 1.0 / m.avg_execution_time_ms } else { 0.0 }, // Invert for "fastest"
                    "discovery_appearances" => m.top_30_appearances as f64,
                    "avg_confidence" => if m.avg_confidence_score > 0.0 { m.avg_confidence_score } else { return None }, // Filter out tools with no confidence score
                    _ => return None, // Filter out invalid metrics
                };
                Some((name.clone(), value))
            })
            .collect();
        
        tools.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        tools.into_iter().take(limit).collect()
    }
}