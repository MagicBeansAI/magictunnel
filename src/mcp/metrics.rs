//! MCP Metrics Collection and Health Monitoring
//!
//! This module provides comprehensive metrics collection and health monitoring
//! for External MCP services, enabling real-time observability and alerting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Health status levels for MCP services
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// All checks passing, low latency
    Healthy,
    /// Some issues but functional (high latency, occasional errors)
    Degraded,
    /// Significant issues (high error rate, very high latency)
    Unhealthy,
    /// Not responding or crashed
    Down,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded", 
            HealthStatus::Unhealthy => "unhealthy",
            HealthStatus::Down => "down",
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            HealthStatus::Down => 4,
            HealthStatus::Unhealthy => 3,
            HealthStatus::Degraded => 2,
            HealthStatus::Healthy => 1,
        }
    }
}

/// Types of health checks performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckType {
    /// Active MCP protocol ping
    Active,
    /// Passive monitoring of requests
    Passive,
    /// Synthetic transaction test
    Synthetic,
}

/// Result of a health check operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub response_time_ms: Option<u64>,
    pub error_details: Option<String>,
    pub last_checked: DateTime<Utc>,
    pub check_type: HealthCheckType,
    pub consecutive_failures: u32,
}

/// Core metrics for an MCP service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServiceMetrics {
    pub service_name: String,
    pub timestamp: DateTime<Utc>,
    
    // Performance Metrics
    pub request_latencies_ms: VecDeque<f64>,
    pub requests_per_minute: u64,
    pub success_rate: f64,
    pub error_rate: f64,
    pub avg_response_time_ms: f64,
    
    // Health Metrics
    pub current_status: HealthStatus,
    pub consecutive_failures: u32,
    pub last_successful_request: Option<DateTime<Utc>>,
    pub uptime_percentage: f64,
    
    // Resource Metrics
    pub memory_usage_mb: Option<f64>,
    pub cpu_usage_percent: Option<f64>,
    
    // Request Distribution
    pub request_types: HashMap<String, u64>,
    pub error_types: HashMap<String, u64>,
    
    // Historical tracking
    pub total_requests: u64,
    pub total_errors: u64,
    pub service_start_time: Option<DateTime<Utc>>,
}

impl McpServiceMetrics {
    pub fn new(service_name: String) -> Self {
        let now = Utc::now();
        Self {
            service_name,
            timestamp: now,
            request_latencies_ms: VecDeque::with_capacity(1000), // Keep last 1000 requests
            requests_per_minute: 0,
            success_rate: 1.0,
            error_rate: 0.0,
            avg_response_time_ms: 0.0,
            current_status: HealthStatus::Down, // Start as down until proven healthy
            consecutive_failures: 0,
            last_successful_request: None,
            uptime_percentage: 0.0,
            memory_usage_mb: None,
            cpu_usage_percent: None,
            request_types: HashMap::new(),
            error_types: HashMap::new(),
            total_requests: 0,
            total_errors: 0,
            service_start_time: Some(now),
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self, latency_ms: f64, request_type: &str) {
        self.request_latencies_ms.push_back(latency_ms);
        if self.request_latencies_ms.len() > 1000 {
            self.request_latencies_ms.pop_front();
        }
        
        *self.request_types.entry(request_type.to_string()).or_insert(0) += 1;
        self.total_requests += 1;
        self.last_successful_request = Some(Utc::now());
        self.consecutive_failures = 0;
        
        self.update_derived_metrics();
    }

    /// Record a failed request
    pub fn record_error(&mut self, error_type: &str, request_type: &str) {
        *self.error_types.entry(error_type.to_string()).or_insert(0) += 1;
        *self.request_types.entry(request_type.to_string()).or_insert(0) += 1;
        self.total_errors += 1;
        self.total_requests += 1;
        self.consecutive_failures += 1;
        
        self.update_derived_metrics();
    }

    /// Update calculated metrics
    fn update_derived_metrics(&mut self) {
        // Calculate average response time
        if !self.request_latencies_ms.is_empty() {
            self.avg_response_time_ms = self.request_latencies_ms.iter().sum::<f64>() / self.request_latencies_ms.len() as f64;
        }
        
        // Calculate success and error rates
        if self.total_requests > 0 {
            self.error_rate = self.total_errors as f64 / self.total_requests as f64;
            self.success_rate = 1.0 - self.error_rate;
        }
        
        // Update timestamp
        self.timestamp = Utc::now();
    }

    /// Update health status based on current metrics
    pub fn update_health_status(&mut self, thresholds: &McpHealthThresholds) {
        let new_status = if self.consecutive_failures >= thresholds.max_consecutive_failures {
            HealthStatus::Down
        } else if self.error_rate > thresholds.error_rate_critical {
            HealthStatus::Unhealthy
        } else if self.avg_response_time_ms > thresholds.response_time_critical_ms as f64 {
            HealthStatus::Unhealthy
        } else if self.error_rate > thresholds.error_rate_warning || 
                  self.avg_response_time_ms > thresholds.response_time_warning_ms as f64 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        if new_status != self.current_status {
            info!("🔄 [METRICS] Service '{}' status changed: {:?} -> {:?}", 
                  self.service_name, self.current_status, new_status);
            self.current_status = new_status;
        }
    }

    /// Calculate uptime percentage over the last period
    pub fn calculate_uptime(&mut self, period_hours: f64) {
        if let Some(start_time) = self.service_start_time {
            let elapsed_hours = (Utc::now() - start_time).num_seconds() as f64 / 3600.0;
            let actual_period = period_hours.min(elapsed_hours);
            
            if actual_period > 0.0 {
                // Simple uptime calculation based on health status
                // In a real implementation, this would track actual downtime
                let downtime_factor = match self.current_status {
                    HealthStatus::Healthy => 0.0,
                    HealthStatus::Degraded => 0.1,
                    HealthStatus::Unhealthy => 0.3,
                    HealthStatus::Down => 1.0,
                };
                
                let uptime_pct = (1.0_f64 - downtime_factor) * 100.0_f64;
                self.uptime_percentage = uptime_pct.max(0.0).min(100.0);
            }
        }
    }
}

/// Thresholds for health status determination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHealthThresholds {
    pub response_time_warning_ms: u64,
    pub response_time_critical_ms: u64,
    pub error_rate_warning: f64,
    pub error_rate_critical: f64,
    pub max_consecutive_failures: u32,
}

impl Default for McpHealthThresholds {
    fn default() -> Self {
        Self {
            response_time_warning_ms: 2000,  // 2 seconds
            response_time_critical_ms: 5000, // 5 seconds
            error_rate_warning: 0.05,        // 5%
            error_rate_critical: 0.15,       // 15%
            max_consecutive_failures: 5,
        }
    }
}

/// Timestamped metrics for historical storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedMetrics {
    pub timestamp: DateTime<Utc>,
    pub metrics: McpServiceMetrics,
}

/// Metrics storage and management
pub struct McpMetricsStorage {
    /// In-memory storage for recent metrics (last 24 hours)
    recent_metrics: Arc<RwLock<HashMap<String, VecDeque<TimestampedMetrics>>>>,
    /// Maximum number of metrics entries per service
    max_entries_per_service: usize,
    /// Retention period for in-memory metrics
    retention_hours: u64,
}

impl McpMetricsStorage {
    pub fn new(max_entries_per_service: usize, retention_hours: u64) -> Self {
        Self {
            recent_metrics: Arc::new(RwLock::new(HashMap::new())),
            max_entries_per_service,
            retention_hours,
        }
    }

    /// Store metrics for a service
    pub async fn store_metrics(&self, metrics: McpServiceMetrics) {
        let mut storage = self.recent_metrics.write().await;
        let service_metrics = storage.entry(metrics.service_name.clone()).or_insert_with(VecDeque::new);
        
        // Add new metrics
        service_metrics.push_back(TimestampedMetrics {
            timestamp: metrics.timestamp,
            metrics,
        });
        
        // Trim old entries
        while service_metrics.len() > self.max_entries_per_service {
            service_metrics.pop_front();
        }
        
        // Remove entries older than retention period
        let cutoff_time = Utc::now() - chrono::Duration::hours(self.retention_hours as i64);
        while let Some(front) = service_metrics.front() {
            if front.timestamp < cutoff_time {
                service_metrics.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get latest metrics for a service
    pub async fn get_latest_metrics(&self, service_name: &str) -> Option<McpServiceMetrics> {
        let storage = self.recent_metrics.read().await;
        storage.get(service_name)
            .and_then(|metrics| metrics.back())
            .map(|timestamped| timestamped.metrics.clone())
    }

    /// Get metrics history for a service
    pub async fn get_metrics_history(&self, service_name: &str, hours: u64) -> Vec<TimestampedMetrics> {
        let storage = self.recent_metrics.read().await;
        let cutoff_time = Utc::now() - chrono::Duration::hours(hours as i64);
        
        storage.get(service_name)
            .map(|metrics| {
                metrics.iter()
                    .filter(|m| m.timestamp >= cutoff_time)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all services with their latest metrics
    pub async fn get_all_latest_metrics(&self) -> HashMap<String, McpServiceMetrics> {
        let storage = self.recent_metrics.read().await;
        let mut result = HashMap::new();
        
        for (service_name, metrics) in storage.iter() {
            if let Some(latest) = metrics.back() {
                result.insert(service_name.clone(), latest.metrics.clone());
            }
        }
        
        result
    }

    /// Get metrics summary across all services
    pub async fn get_metrics_summary(&self) -> McpMetricsSummary {
        let storage = self.recent_metrics.read().await;
        let mut summary = McpMetricsSummary::default();
        
        for (service_name, metrics) in storage.iter() {
            if let Some(latest) = metrics.back() {
                summary.total_services += 1;
                
                match latest.metrics.current_status {
                    HealthStatus::Healthy => summary.healthy_services += 1,
                    HealthStatus::Degraded => summary.degraded_services += 1,
                    HealthStatus::Unhealthy => summary.unhealthy_services += 1,
                    HealthStatus::Down => summary.down_services += 1,
                }
                
                // Update aggregate metrics
                summary.total_requests += latest.metrics.total_requests;
                summary.total_errors += latest.metrics.total_errors;
                
                if latest.metrics.avg_response_time_ms > 0.0 {
                    summary.avg_response_times.push(latest.metrics.avg_response_time_ms);
                }
            }
        }
        
        // Calculate overall averages
        if !summary.avg_response_times.is_empty() {
            summary.overall_avg_response_time_ms = 
                summary.avg_response_times.iter().sum::<f64>() / summary.avg_response_times.len() as f64;
        }
        
        if summary.total_requests > 0 {
            summary.overall_error_rate = summary.total_errors as f64 / summary.total_requests as f64;
        }
        
        summary.last_updated = Utc::now();
        summary
    }
}

/// Summary metrics across all MCP services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMetricsSummary {
    pub total_services: u32,
    pub healthy_services: u32,
    pub degraded_services: u32,
    pub unhealthy_services: u32,
    pub down_services: u32,
    pub total_requests: u64,
    pub total_errors: u64,
    pub overall_error_rate: f64,
    pub overall_avg_response_time_ms: f64,
    pub last_updated: DateTime<Utc>,
    
    // Internal calculation helper
    #[serde(skip)]
    avg_response_times: Vec<f64>,
}

impl Default for McpMetricsSummary {
    fn default() -> Self {
        Self {
            total_services: 0,
            healthy_services: 0,
            degraded_services: 0,
            unhealthy_services: 0,
            down_services: 0,
            total_requests: 0,
            total_errors: 0,
            overall_error_rate: 0.0,
            overall_avg_response_time_ms: 0.0,
            last_updated: Utc::now(),
            avg_response_times: Vec::new(),
        }
    }
}

/// Main metrics collector for MCP services
pub struct McpMetricsCollector {
    /// Metrics storage
    storage: Arc<McpMetricsStorage>,
    /// Current metrics for each service
    service_metrics: Arc<RwLock<HashMap<String, McpServiceMetrics>>>,
    /// Health thresholds configuration
    thresholds: McpHealthThresholds,
}

impl McpMetricsCollector {
    pub fn new(thresholds: McpHealthThresholds) -> Self {
        Self {
            storage: Arc::new(McpMetricsStorage::new(1440, 24)), // 1440 entries (1 per minute for 24h)
            service_metrics: Arc::new(RwLock::new(HashMap::new())),
            thresholds,
        }
    }

    /// Initialize metrics tracking for a service
    pub async fn initialize_service(&self, service_name: &str) {
        let mut metrics_map = self.service_metrics.write().await;
        if !metrics_map.contains_key(service_name) {
            let metrics = McpServiceMetrics::new(service_name.to_string());
            metrics_map.insert(service_name.to_string(), metrics);
            info!("📊 [METRICS] Initialized metrics tracking for service: {}", service_name);
        }
    }

    /// Record a successful request
    pub async fn record_request_success(&self, service_name: &str, latency_ms: f64, request_type: &str) {
        let mut metrics_map = self.service_metrics.write().await;
        if let Some(metrics) = metrics_map.get_mut(service_name) {
            metrics.record_success(latency_ms, request_type);
            metrics.update_health_status(&self.thresholds);
            
            // Store updated metrics
            let metrics_clone = metrics.clone();
            drop(metrics_map);
            self.storage.store_metrics(metrics_clone).await;
            
            debug!("✅ [METRICS] Recorded success for '{}': {}ms", service_name, latency_ms);
        }
    }

    /// Record a failed request
    pub async fn record_request_error(&self, service_name: &str, error_type: &str, request_type: &str) {
        let mut metrics_map = self.service_metrics.write().await;
        if let Some(metrics) = metrics_map.get_mut(service_name) {
            metrics.record_error(error_type, request_type);
            metrics.update_health_status(&self.thresholds);
            
            // Store updated metrics
            let metrics_clone = metrics.clone();
            drop(metrics_map);
            self.storage.store_metrics(metrics_clone).await;
            
            warn!("❌ [METRICS] Recorded error for '{}': {} ({})", service_name, error_type, request_type);
        }
    }

    /// Update health status based on external health check
    pub async fn update_health_status(&self, service_name: &str, status: HealthStatus, response_time_ms: Option<u64>) {
        let mut metrics_map = self.service_metrics.write().await;
        if let Some(metrics) = metrics_map.get_mut(service_name) {
            // If external check shows service as down/unhealthy, increment failures
            if matches!(status, HealthStatus::Down | HealthStatus::Unhealthy) {
                metrics.consecutive_failures += 1;
            } else {
                metrics.consecutive_failures = 0;
            }
            
            metrics.current_status = status.clone();
            metrics.timestamp = Utc::now();
            
            if let Some(latency) = response_time_ms {
                // Add health check latency to the metrics
                metrics.request_latencies_ms.push_back(latency as f64);
                if metrics.request_latencies_ms.len() > 1000 {
                    metrics.request_latencies_ms.pop_front();
                }
                metrics.update_derived_metrics();
            }
            
            // Store updated metrics
            let metrics_clone = metrics.clone();
            drop(metrics_map);
            self.storage.store_metrics(metrics_clone).await;
            
            debug!("🔄 [METRICS] Updated health status for '{}': {:?}", service_name, status);
        }
    }

    /// Get current metrics for a service
    pub async fn get_service_metrics(&self, service_name: &str) -> Option<McpServiceMetrics> {
        self.storage.get_latest_metrics(service_name).await
    }

    /// Get metrics for all services
    pub async fn get_all_metrics(&self) -> HashMap<String, McpServiceMetrics> {
        self.storage.get_all_latest_metrics().await
    }

    /// Get metrics summary
    pub async fn get_summary(&self) -> McpMetricsSummary {
        self.storage.get_metrics_summary().await
    }

    /// Get storage reference for advanced queries
    pub fn storage(&self) -> Arc<McpMetricsStorage> {
        Arc::clone(&self.storage)
    }
}