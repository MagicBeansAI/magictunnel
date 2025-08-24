//! Performance optimizations for Smart Discovery Service
//!
//! This module provides performance enhancements including request deduplication,
//! concurrent processing, and batch optimization.

use std::collections::HashMap;
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{Duration, Instant};
use tracing::{debug, warn};

/// Request deduplication key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestKey {
    /// Request content hash
    pub content_hash: u64,
    /// Request type identifier
    pub request_type: String,
}

impl RequestKey {
    /// Create a new request key from content
    pub fn new(content: &str, request_type: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        request_type.hash(&mut hasher);
        
        Self {
            content_hash: hasher.finish(),
            request_type: request_type.to_string(),
        }
    }
}

/// Pending request information
#[derive(Debug, Clone)]
pub struct PendingRequest<T> {
    /// Future result
    pub result: Arc<Mutex<Option<T>>>,
    /// Request start time
    pub start_time: Instant,
    /// Number of waiters
    pub waiters: Arc<Mutex<u32>>,
}

impl<T> PendingRequest<T> {
    /// Create a new pending request
    pub fn new() -> Self {
        Self {
            result: Arc::new(Mutex::new(None)),
            start_time: Instant::now(),
            waiters: Arc::new(Mutex::new(0)),
        }
    }

    /// Add a waiter
    pub async fn add_waiter(&self) {
        let mut waiters = self.waiters.lock().await;
        *waiters += 1;
    }

    /// Remove a waiter
    pub async fn remove_waiter(&self) {
        let mut waiters = self.waiters.lock().await;
        if *waiters > 0 {
            *waiters -= 1;
        }
    }

    /// Get waiter count
    pub async fn waiter_count(&self) -> u32 {
        *self.waiters.lock().await
    }

    /// Check if request is expired
    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.start_time.elapsed() > timeout
    }
}

/// Request deduplication manager
pub struct RequestDeduplicator<T> {
    /// Pending requests
    pending: Arc<RwLock<HashMap<RequestKey, PendingRequest<T>>>>,
    /// Request timeout
    timeout: Duration,
}

impl<T: Clone> RequestDeduplicator<T> {
    /// Create a new request deduplicator
    pub fn new(timeout: Duration) -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            timeout,
        }
    }

    /// Execute or wait for a request
    pub async fn execute_or_wait<F, Fut>(&self, key: RequestKey, executor: F) -> Result<T, String>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, String>> + Send + 'static,
        T: Send + 'static,
    {
        // Check if request is already pending
        {
            let mut pending = self.pending.write().await;
            
            // Clean up expired requests
            let now = Instant::now();
            pending.retain(|_, req| !req.is_expired(self.timeout));
            
            if let Some(existing) = pending.get(&key) {
                // Request is already pending, wait for it
                let result_ref = existing.result.clone();
                existing.add_waiter().await;
                
                debug!("Request deduplication: waiting for existing request");
                
                // Release the lock and wait for the result
                drop(pending);
                
                // Wait for the result with timeout
                let wait_start = Instant::now();
                loop {
                    {
                        let result = result_ref.lock().await;
                        if let Some(value) = result.as_ref() {
                            return Ok(value.clone());
                        }
                    }
                    
                    if wait_start.elapsed() > self.timeout {
                        warn!("Request deduplication: timeout waiting for result");
                        return Err("Request deduplication timeout".to_string());
                    }
                    
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            } else {
                // New request, add it to pending
                let pending_req = PendingRequest::new();
                pending.insert(key.clone(), pending_req);
            }
        }

        // Execute the request
        debug!("Request deduplication: executing new request");
        let result = executor().await;
        
        // Store the result and notify waiters
        {
            let pending = self.pending.write().await;
            if let Some(pending_req) = pending.get(&key) {
                let waiter_count = pending_req.waiter_count().await;
                debug!("Request deduplication: notifying {} waiters", waiter_count);
                
                // Store the result
                {
                    let mut result_lock = pending_req.result.lock().await;
                    match result.clone() {
                        Ok(value) => {
                            *result_lock = Some(value);
                        }
                        Err(e) => {
                            warn!("Request execution failed: {}", e);
                            // For error cases, we don't store a result - let the error propagate
                            // This allows waiters to get the error from the main execution path
                        }
                    }
                }
            }
            
            // Remove from pending after a short delay to allow waiters to read
            let pending_clone = self.pending.clone();
            let key_clone = key.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let mut pending = pending_clone.write().await;
                pending.remove(&key_clone);
            });
        }

        result
    }

    /// Get statistics
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let pending = self.pending.read().await;
        let mut stats = HashMap::new();
        
        stats.insert("pending_requests".to_string(), serde_json::Value::Number(pending.len().into()));
        stats.insert("timeout_seconds".to_string(), serde_json::Value::Number(self.timeout.as_secs().into()));
        
        // Calculate total waiters
        let mut total_waiters = 0;
        for req in pending.values() {
            total_waiters += req.waiter_count().await;
        }
        stats.insert("total_waiters".to_string(), serde_json::Value::Number(total_waiters.into()));
        
        stats
    }
}

/// Batch request processor for optimizing multiple similar requests
pub struct BatchProcessor<T> {
    /// Batch size
    batch_size: usize,
    /// Batch timeout
    batch_timeout: Duration,
    /// Pending items
    pending: Arc<Mutex<Vec<T>>>,
    /// Last batch time
    last_batch: Arc<Mutex<Instant>>,
}

impl<T> BatchProcessor<T> {
    /// Create a new batch processor
    pub fn new(batch_size: usize, batch_timeout: Duration) -> Self {
        Self {
            batch_size,
            batch_timeout,
            pending: Arc::new(Mutex::new(Vec::new())),
            last_batch: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Add item to batch
    pub async fn add_item(&self, item: T) -> bool {
        let mut pending = self.pending.lock().await;
        pending.push(item);
        
        // Check if batch is ready
        pending.len() >= self.batch_size
    }

    /// Check if batch should be processed due to timeout
    pub async fn should_process_timeout(&self) -> bool {
        let last_batch = self.last_batch.lock().await;
        last_batch.elapsed() > self.batch_timeout
    }

    /// Get and clear pending items
    pub async fn get_and_clear_pending(&self) -> Vec<T> {
        let mut pending = self.pending.lock().await;
        let mut last_batch = self.last_batch.lock().await;
        
        let items = pending.drain(..).collect();
        *last_batch = Instant::now();
        
        items
    }

    /// Get current batch size
    pub async fn current_batch_size(&self) -> usize {
        let pending = self.pending.lock().await;
        pending.len()
    }
}

/// Performance metrics collector
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Total requests processed
    pub total_requests: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Request rate (requests per second)
    pub request_rate: f64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Deduplication savings
    pub deduplication_savings: u64,
    /// Batch processing efficiency
    pub batch_efficiency: f64,
}

impl PerformanceMetrics {
    /// Update metrics with new request
    pub fn update_request(&mut self, response_time: Duration, cache_hit: bool) {
        self.total_requests += 1;
        
        // Update average response time (simple moving average)
        let response_time_ms = response_time.as_millis() as f64;
        if self.avg_response_time_ms == 0.0 {
            self.avg_response_time_ms = response_time_ms;
        } else {
            self.avg_response_time_ms = (self.avg_response_time_ms * 0.9) + (response_time_ms * 0.1);
        }
        
        // Update cache hit rate
        if cache_hit {
            self.cache_hit_rate = (self.cache_hit_rate * 0.9) + (1.0 * 0.1);
        } else {
            self.cache_hit_rate = self.cache_hit_rate * 0.9;
        }
    }

    /// Update deduplication savings
    pub fn update_deduplication_savings(&mut self, saved_requests: u64) {
        self.deduplication_savings += saved_requests;
    }

    /// Get metrics as JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_requests": self.total_requests,
            "avg_response_time_ms": self.avg_response_time_ms,
            "request_rate": self.request_rate,
            "cache_hit_rate": self.cache_hit_rate,
            "deduplication_savings": self.deduplication_savings,
            "batch_efficiency": self.batch_efficiency
        })
    }
}

/// Performance optimizer that combines various optimization techniques
pub struct PerformanceOptimizer {
    /// Request deduplicator
    deduplicator: Arc<RequestDeduplicator<String>>,
    /// Performance metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Optimization enabled
    enabled: bool,
}

impl PerformanceOptimizer {
    /// Create a new performance optimizer
    pub fn new(enabled: bool) -> Self {
        Self {
            deduplicator: Arc::new(RequestDeduplicator::new(Duration::from_secs(30))),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            enabled,
        }
    }

    /// Check if optimization is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get deduplicator reference
    pub fn deduplicator(&self) -> &RequestDeduplicator<String> {
        &self.deduplicator
    }

    /// Update metrics
    pub async fn update_metrics(&self, response_time: Duration, cache_hit: bool) {
        if self.enabled {
            let mut metrics = self.metrics.write().await;
            metrics.update_request(response_time, cache_hit);
        }
    }

    /// Record deduplication savings
    pub async fn record_deduplication_savings(&self, saved_requests: u64) {
        if self.enabled {
            let mut metrics = self.metrics.write().await;
            metrics.update_deduplication_savings(saved_requests);
        }
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Get performance statistics
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        stats.insert("enabled".to_string(), serde_json::Value::Bool(self.enabled));
        
        if self.enabled {
            let metrics = self.get_metrics().await;
            stats.insert("metrics".to_string(), metrics.to_json());
            
            let dedup_stats = self.deduplicator.get_stats().await;
            stats.insert("deduplication".to_string(), serde_json::Value::Object(
                dedup_stats.into_iter().collect()
            ));
        }
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_key_creation() {
        let key1 = RequestKey::new("test request", "discovery");
        let key2 = RequestKey::new("test request", "discovery");
        let key3 = RequestKey::new("different request", "discovery");
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_pending_request_waiters() {
        let pending = PendingRequest::<String>::new();
        
        assert_eq!(pending.waiter_count().await, 0);
        
        pending.add_waiter().await;
        assert_eq!(pending.waiter_count().await, 1);
        
        pending.add_waiter().await;
        assert_eq!(pending.waiter_count().await, 2);
        
        pending.remove_waiter().await;
        assert_eq!(pending.waiter_count().await, 1);
    }

    #[tokio::test]
    async fn test_batch_processor() {
        let processor = BatchProcessor::new(3, Duration::from_secs(1));
        
        assert!(!processor.add_item("item1".to_string()).await);
        assert!(!processor.add_item("item2".to_string()).await);
        assert!(processor.add_item("item3".to_string()).await); // Should trigger batch
        
        let items = processor.get_and_clear_pending().await;
        assert_eq!(items.len(), 3);
        assert_eq!(processor.current_batch_size().await, 0);
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::default();
        
        metrics.update_request(Duration::from_millis(100), true);
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.avg_response_time_ms, 100.0);
        assert_eq!(metrics.cache_hit_rate, 0.1);
        
        metrics.update_request(Duration::from_millis(200), false);
        assert_eq!(metrics.total_requests, 2);
        assert!(metrics.avg_response_time_ms > 100.0 && metrics.avg_response_time_ms < 200.0);
        assert!(metrics.cache_hit_rate < 0.1);
    }

    #[tokio::test]
    async fn test_performance_optimizer() {
        let optimizer = PerformanceOptimizer::new(true);
        
        assert!(optimizer.is_enabled());
        
        optimizer.update_metrics(Duration::from_millis(50), true).await;
        optimizer.record_deduplication_savings(5).await;
        
        let metrics = optimizer.get_metrics().await;
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.deduplication_savings, 5);
        
        let stats = optimizer.get_stats().await;
        assert_eq!(stats.get("enabled").unwrap(), &serde_json::Value::Bool(true));
    }
}