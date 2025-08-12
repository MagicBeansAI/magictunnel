//! Caching mechanisms for Smart Discovery Service
//!
//! This module provides various caching strategies to improve performance
//! of the smart discovery system, including tool matching cache, LLM response
//! cache, and request deduplication.

use crate::discovery::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Serde helper for Duration as seconds
mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Cache entry with expiration time
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// Cached value
    pub value: T,
    /// When this entry was created
    pub created_at: Instant,
    /// When this entry expires
    pub expires_at: Instant,
    /// Number of times this entry has been accessed
    pub hit_count: u64,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry with TTL
    pub fn new(value: T, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            expires_at: now + ttl,
            hit_count: 0,
        }
    }

    /// Check if this entry is expired
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }

    /// Get the value and increment hit count
    pub fn get(&mut self) -> &T {
        self.hit_count += 1;
        &self.value
    }

    /// Get age of this entry
    pub fn age(&self) -> Duration {
        Instant::now() - self.created_at
    }
}

/// Configuration for discovery cache
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryCacheConfig {
    /// Maximum number of entries in tool matching cache
    pub max_tool_matches: usize,
    /// TTL for tool matching cache entries
    #[serde(with = "duration_secs")]
    pub tool_match_ttl: Duration,
    /// Maximum number of entries in LLM response cache
    pub max_llm_responses: usize,
    /// TTL for LLM response cache entries
    #[serde(with = "duration_secs")]
    pub llm_response_ttl: Duration,
    /// Maximum number of entries in registry cache
    pub max_registry_entries: usize,
    /// TTL for registry cache entries
    #[serde(with = "duration_secs")]
    pub registry_ttl: Duration,
    /// Enable/disable caching
    pub enabled: bool,
}

impl Default for DiscoveryCacheConfig {
    fn default() -> Self {
        Self {
            max_tool_matches: 1000,
            tool_match_ttl: Duration::from_secs(300), // 5 minutes
            max_llm_responses: 500,
            llm_response_ttl: Duration::from_secs(600), // 10 minutes
            max_registry_entries: 100,
            registry_ttl: Duration::from_secs(60), // 1 minute
            enabled: true,
        }
    }
}

/// Cache key for tool matching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolMatchCacheKey {
    /// The user request
    pub request: String,
    /// Optional context
    pub context: Option<String>,
    /// Confidence threshold
    pub confidence_threshold: String, // Serialized as string for hashing
    /// Tool selection mode (rule_based or llm_based)
    pub tool_selection_mode: String,
}

impl ToolMatchCacheKey {
    /// Create a new cache key from a smart discovery request
    pub fn from_request(request: &SmartDiscoveryRequest, tool_selection_mode: &str) -> Self {
        Self {
            request: request.request.clone(),
            context: request.context.clone(),
            confidence_threshold: format!("{:.2}", request.confidence_threshold.unwrap_or(0.7)),
            tool_selection_mode: tool_selection_mode.to_string(),
        }
    }
}

/// Cache key for LLM parameter extraction
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LlmCacheKey {
    /// The user request
    pub request: String,
    /// Tool name
    pub tool_name: String,
    /// Tool schema hash (to detect schema changes)
    pub schema_hash: String,
}

impl LlmCacheKey {
    /// Create a new LLM cache key
    pub fn new(request: &SmartDiscoveryRequest, tool_name: &str, schema_hash: &str) -> Self {
        Self {
            request: request.request.clone(),
            tool_name: tool_name.to_string(),
            schema_hash: schema_hash.to_string(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total cache evictions
    pub evictions: u64,
    /// Total cache entries
    pub entries: u64,
    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f64,
}

impl CacheStats {
    /// Update hit rate
    pub fn update_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        self.hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
    }

    /// Record a cache hit
    pub fn record_hit(&mut self) {
        self.hits += 1;
        self.update_hit_rate();
    }

    /// Record a cache miss
    pub fn record_miss(&mut self) {
        self.misses += 1;
        self.update_hit_rate();
    }

    /// Record a cache eviction
    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }
}

/// Smart Discovery Cache Manager
pub struct DiscoveryCache {
    /// Configuration
    config: DiscoveryCacheConfig,
    /// Tool matching cache
    tool_matches: Arc<RwLock<HashMap<ToolMatchCacheKey, CacheEntry<Vec<ToolMatch>>>>>,
    /// LLM parameter extraction cache
    llm_responses: Arc<RwLock<HashMap<LlmCacheKey, CacheEntry<ParameterExtraction>>>>,
    /// Registry tools cache
    registry_tools: Arc<RwLock<Option<CacheEntry<Vec<(String, crate::registry::types::ToolDefinition)>>>>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
}

impl DiscoveryCache {
    /// Create a new discovery cache
    pub fn new(config: DiscoveryCacheConfig) -> Self {
        Self {
            config,
            tool_matches: Arc::new(RwLock::new(HashMap::new())),
            llm_responses: Arc::new(RwLock::new(HashMap::new())),
            registry_tools: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Create a new discovery cache with default configuration
    pub fn new_with_defaults() -> Self {
        Self::new(DiscoveryCacheConfig::default())
    }

    /// Check if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get tool matches from cache
    pub async fn get_tool_matches(&self, key: &ToolMatchCacheKey) -> Option<Vec<ToolMatch>> {
        if !self.config.enabled {
            return None;
        }

        let mut cache = self.tool_matches.write().await;
        if let Some(entry) = cache.get_mut(key) {
            if entry.is_expired() {
                cache.remove(key);
                self.record_miss().await;
                debug!("Tool match cache entry expired for key: {:?}", key);
                None
            } else {
                let result = entry.get().clone();
                self.record_hit().await;
                debug!("Tool match cache hit for key: {:?}", key);
                Some(result)
            }
        } else {
            self.record_miss().await;
            debug!("Tool match cache miss for key: {:?}", key);
            None
        }
    }

    /// Store tool matches in cache
    pub async fn store_tool_matches(&self, key: ToolMatchCacheKey, matches: Vec<ToolMatch>) {
        if !self.config.enabled {
            return;
        }

        let mut cache = self.tool_matches.write().await;
        
        // Check if we need to evict entries
        if cache.len() >= self.config.max_tool_matches {
            self.evict_oldest_tool_matches(&mut cache).await;
        }

        let entry = CacheEntry::new(matches, self.config.tool_match_ttl);
        cache.insert(key, entry);
        
        debug!("Stored tool matches in cache, total entries: {}", cache.len());
    }

    /// Get LLM response from cache
    pub async fn get_llm_response(&self, key: &LlmCacheKey) -> Option<ParameterExtraction> {
        if !self.config.enabled {
            return None;
        }

        let mut cache = self.llm_responses.write().await;
        if let Some(entry) = cache.get_mut(key) {
            if entry.is_expired() {
                cache.remove(key);
                self.record_miss().await;
                debug!("LLM response cache entry expired for key: {:?}", key);
                None
            } else {
                let result = entry.get().clone();
                self.record_hit().await;
                debug!("LLM response cache hit for key: {:?}", key);
                Some(result)
            }
        } else {
            self.record_miss().await;
            debug!("LLM response cache miss for key: {:?}", key);
            None
        }
    }

    /// Store LLM response in cache
    pub async fn store_llm_response(&self, key: LlmCacheKey, response: ParameterExtraction) {
        if !self.config.enabled {
            return;
        }

        let mut cache = self.llm_responses.write().await;
        
        // Check if we need to evict entries
        if cache.len() >= self.config.max_llm_responses {
            self.evict_oldest_llm_responses(&mut cache).await;
        }

        let entry = CacheEntry::new(response, self.config.llm_response_ttl);
        cache.insert(key, entry);
        
        debug!("Stored LLM response in cache, total entries: {}", cache.len());
    }

    /// Get registry tools from cache
    pub async fn get_registry_tools(&self) -> Option<Vec<(String, crate::registry::types::ToolDefinition)>> {
        if !self.config.enabled {
            return None;
        }

        let mut cache = self.registry_tools.write().await;
        if let Some(entry) = cache.as_mut() {
            if entry.is_expired() {
                *cache = None;
                self.record_miss().await;
                debug!("Registry tools cache entry expired");
                None
            } else {
                let result = entry.get().clone();
                self.record_hit().await;
                debug!("Registry tools cache hit");
                Some(result)
            }
        } else {
            self.record_miss().await;
            debug!("Registry tools cache miss");
            None
        }
    }

    /// Store registry tools in cache
    pub async fn store_registry_tools(&self, tools: Vec<(String, crate::registry::types::ToolDefinition)>) {
        if !self.config.enabled {
            return;
        }

        let mut cache = self.registry_tools.write().await;
        let entry = CacheEntry::new(tools, self.config.registry_ttl);
        *cache = Some(entry);
        
        debug!("Stored registry tools in cache");
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        let mut tool_matches = self.tool_matches.write().await;
        let mut llm_responses = self.llm_responses.write().await;
        let mut registry_tools = self.registry_tools.write().await;

        tool_matches.clear();
        llm_responses.clear();
        *registry_tools = None;

        info!("Cleared all discovery caches");
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let tool_matches = self.tool_matches.read().await;
        let llm_responses = self.llm_responses.read().await;
        let registry_tools = self.registry_tools.read().await;
        
        let mut stats = self.stats.read().await.clone();
        stats.entries = tool_matches.len() as u64 + llm_responses.len() as u64 + 
                        if registry_tools.is_some() { 1 } else { 0 };
        
        stats
    }

    /// Record a cache hit
    async fn record_hit(&self) {
        let mut stats = self.stats.write().await;
        stats.record_hit();
    }

    /// Record a cache miss
    async fn record_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.record_miss();
    }

    /// Evict oldest tool match entries
    async fn evict_oldest_tool_matches(&self, cache: &mut HashMap<ToolMatchCacheKey, CacheEntry<Vec<ToolMatch>>>) {
        let evict_count = cache.len() / 4; // Evict 25% of entries
        
        let mut entries: Vec<_> = cache.iter().map(|(k, v)| (k.clone(), v.created_at)).collect();
        entries.sort_by_key(|(_, created_at)| *created_at);
        
        for (key, _) in entries.into_iter().take(evict_count) {
            cache.remove(&key);
        }
        
        let mut stats = self.stats.write().await;
        stats.evictions += evict_count as u64;
        
        debug!("Evicted {} tool match cache entries", evict_count);
    }

    /// Evict oldest LLM response entries
    async fn evict_oldest_llm_responses(&self, cache: &mut HashMap<LlmCacheKey, CacheEntry<ParameterExtraction>>) {
        let evict_count = cache.len() / 4; // Evict 25% of entries
        
        let mut entries: Vec<_> = cache.iter().map(|(k, v)| (k.clone(), v.created_at)).collect();
        entries.sort_by_key(|(_, created_at)| *created_at);
        
        for (key, _) in entries.into_iter().take(evict_count) {
            cache.remove(&key);
        }
        
        let mut stats = self.stats.write().await;
        stats.evictions += evict_count as u64;
        
        debug!("Evicted {} LLM response cache entries", evict_count);
    }
}

/// Create a simple hash for tool schema to detect changes
pub fn create_schema_hash(schema: &serde_json::Value) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    schema.to_string().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("test_value".to_string(), Duration::from_millis(100));
        assert!(!entry.is_expired());
        
        sleep(Duration::from_millis(150)).await;
        assert!(entry.is_expired());
    }

    #[tokio::test]
    async fn test_tool_match_cache() {
        let cache = DiscoveryCache::new_with_defaults();
        
        let key = ToolMatchCacheKey {
            request: "test request".to_string(),
            context: None,
            confidence_threshold: "0.70".to_string(),
            tool_selection_mode: "rule_based".to_string(),
        };
        
        // Test cache miss
        assert!(cache.get_tool_matches(&key).await.is_none());
        
        // Store in cache
        let matches = vec![ToolMatch {
            tool_name: "test_tool".to_string(),
            confidence_score: 0.8,
            reasoning: "test reasoning".to_string(),
            meets_threshold: true,
        }];
        
        cache.store_tool_matches(key.clone(), matches.clone()).await;
        
        // Test cache hit
        let cached_matches = cache.get_tool_matches(&key).await;
        assert!(cached_matches.is_some());
        assert_eq!(cached_matches.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = DiscoveryCache::new_with_defaults();
        
        let key = ToolMatchCacheKey {
            request: "test".to_string(),
            context: None,
            confidence_threshold: "0.70".to_string(),
            tool_selection_mode: "rule_based".to_string(),
        };
        
        // Should be a miss
        cache.get_tool_matches(&key).await;
        
        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.0);
        
        // Store and retrieve
        cache.store_tool_matches(key.clone(), vec![]).await;
        cache.get_tool_matches(&key).await;
        
        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[test]
    fn test_schema_hash() {
        let schema1 = serde_json::json!({"type": "object", "properties": {"name": {"type": "string"}}});
        let schema2 = serde_json::json!({"type": "object", "properties": {"name": {"type": "string"}}});
        let schema3 = serde_json::json!({"type": "object", "properties": {"age": {"type": "number"}}});
        
        let hash1 = create_schema_hash(&schema1);
        let hash2 = create_schema_hash(&schema2);
        let hash3 = create_schema_hash(&schema3);
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}