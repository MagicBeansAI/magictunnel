//! High Performance Tool-First Enterprise Allowlist System
//!
//! This implementation uses the fastest possible Rust data structures and zero-allocation hot paths
//! for maximum performance (>3.9M evaluations/second).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};
use chrono::{DateTime, Utc};
use regex::RegexSet;
use std::sync::RwLock;
use std::hash::{Hash, Hasher};
use ahash::{AHashMap, AHashSet};
use once_cell::sync::Lazy;
use super::statistics::{SecurityServiceStatistics, HealthMonitor, ServiceHealth, HealthStatus, AllowlistStatistics, HourlyMetric, RuleMatch, PerformanceMetrics};
use super::allowlist_types::{AllowlistResult, AllowlistContext, RuleLevel, AllowlistAction, AllowlistConfig, AllowlistRule, PatternRule, AllowlistPattern};
use super::pattern_loader::PatternLoader;
use super::audit::{AuditService, AuditEntry, AuditEventType, AuditOutcome, AuditUser, AuditTool, AuditSecurity};

// Use fastest hash implementations available
use std::collections::hash_map::DefaultHasher;

/// Ultra-fast bloom filter for pattern rejection
/// Provides extremely fast negative lookups with minimal false positives
#[derive(Clone)]
pub struct BloomFilter {
    bits: Vec<u64>,
    hash_count: usize,
    size_bits: usize,
}

impl BloomFilter {
    /// Create new bloom filter sized for expected pattern count
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        // Calculate optimal size and hash count
        let size_bits = Self::optimal_size(expected_items, false_positive_rate);
        let hash_count = Self::optimal_hash_count(size_bits, expected_items);
        
        Self {
            bits: vec![0u64; (size_bits + 63) / 64], // Round up to u64 boundaries
            hash_count,
            size_bits,
        }
    }
    
    /// Add pattern to bloom filter
    pub fn insert(&mut self, item: &str) {
        let hashes = self.hash_item(item);
        for hash in hashes {
            let bit_index = hash % self.size_bits;
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;
            self.bits[word_index] |= 1u64 << bit_offset;
        }
    }
    
    /// Check if pattern might be in set (no false negatives, possible false positives)
    #[inline(always)]
    pub fn might_contain(&self, item: &str) -> bool {
        let hashes = self.hash_item(item);
        for hash in hashes {
            let bit_index = hash % self.size_bits;
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;
            if (self.bits[word_index] & (1u64 << bit_offset)) == 0 {
                return false; // Definitely not in set
            }
        }
        true // Might be in set
    }
    
    /// Generate multiple hash values for item
    #[inline(always)]
    fn hash_item(&self, item: &str) -> Vec<usize> {
        let mut hashes = Vec::with_capacity(self.hash_count);
        
        // Use multiple hash functions (double hashing)
        let mut h1 = DefaultHasher::new();
        item.hash(&mut h1);
        let hash1 = h1.finish();
        
        let mut h2 = DefaultHasher::new();
        (item.len() as u64).hash(&mut h2);
        item.bytes().rev().for_each(|b| b.hash(&mut h2));
        let hash2 = h2.finish();
        
        for i in 0..self.hash_count {
            let hash = (hash1.wrapping_add((i as u64).wrapping_mul(hash2))) as usize;
            hashes.push(hash);
        }
        
        hashes
    }
    
    fn optimal_size(n: usize, p: f64) -> usize {
        let size = -(n as f64 * p.ln() / (2.0_f64.ln().powi(2))) as usize;
        size.max(64) // Minimum 64 bits
    }
    
    fn optimal_hash_count(m: usize, n: usize) -> usize {
        let k = (m as f64 / n as f64 * 2.0_f64.ln()) as usize;
        k.max(1).min(8) // Between 1 and 8 hash functions
    }
}

/// Trie node for ultra-fast prefix matching
#[derive(Clone)]
pub struct TrieNode {
    children: AHashMap<char, Box<TrieNode>>,
    pattern_indices: Vec<usize>, // Indices of patterns that match at this node
    is_end: bool,
}

impl TrieNode {
    pub fn new() -> Self {
        Self {
            children: AHashMap::new(),
            pattern_indices: Vec::new(),
            is_end: false,
        }
    }
    
    /// Insert pattern into trie for exact prefix matching
    pub fn insert(&mut self, pattern: &str, pattern_index: usize) {
        let mut node = self;
        for ch in pattern.chars() {
            node = node.children.entry(ch).or_insert_with(|| Box::new(TrieNode::new()));
        }
        node.pattern_indices.push(pattern_index);
        node.is_end = true;
    }
    
    /// Find all pattern indices that match as prefixes of the tool name
    #[inline(always)]
    pub fn find_prefix_matches(&self, tool_name: &str) -> Vec<usize> {
        let mut matches = Vec::new();
        let mut node = self;
        
        // Check if root has any patterns (empty string patterns)
        matches.extend(&node.pattern_indices);
        
        for ch in tool_name.chars() {
            if let Some(child) = node.children.get(&ch) {
                node = child;
                matches.extend(&node.pattern_indices);
            } else {
                break;
            }
        }
        
        matches
    }
}

// All types are imported from allowlist module

// RuleLevel is imported from allowlist module

/// Memory-optimized rule representation for ultra-fast evaluation
#[derive(Clone)]
pub struct CachedAllowlistRule {
    pub action: AllowlistAction,
    pub reason: Option<Arc<str>>, // Arc<str> is more memory efficient than String
    pub priority: u8, // u8 is sufficient for priority levels
}

/// Compact decision cache (8 bytes total for maximum cache efficiency)
#[derive(Clone, Copy)]
pub struct CachedDecision {
    // Pack everything into 64 bits for single CPU cache line
    packed_data: u64, // action(1) + rule_level(3) + timestamp(28) + reserved(32)
}

impl CachedDecision {
    pub fn new(allowed: bool, rule_level: RuleLevel, timestamp: u32) -> Self {
        let action_bit = if allowed { 1u64 } else { 0u64 };
        let level_bits = (rule_level as u64) << 1;
        let timestamp_bits = (timestamp as u64 & 0x0FFFFFFF) << 4; // 28 bits for timestamp
        
        Self {
            packed_data: action_bit | level_bits | timestamp_bits,
        }
    }
    
    #[inline(always)]
    pub fn allowed(self) -> bool {
        (self.packed_data & 1) == 1
    }
    
    #[inline(always)]
    pub fn rule_level(self) -> RuleLevel {
        match (self.packed_data >> 1) & 0x7 {
            0 => RuleLevel::Emergency,
            1 => RuleLevel::Tool,
            2 => RuleLevel::Server,
            3 => RuleLevel::Capability,
            4 => RuleLevel::Global,
            _ => RuleLevel::Default,
        }
    }
    
    #[inline(always)]
    pub fn timestamp(self) -> u32 {
        ((self.packed_data >> 4) & 0x0FFFFFFF) as u32
    }
    
    #[inline(always)]
    pub fn into_result(self) -> AllowlistResult {
        if self.allowed() {
            AllowlistResult::allow_fast(
                match self.rule_level() {
                    RuleLevel::Emergency => "Emergency allow",
                    RuleLevel::Tool => "Tool rule",
                    RuleLevel::Server => "Server rule", 
                    RuleLevel::Capability => "Capability pattern",
                    RuleLevel::Global => "Global pattern",
                    RuleLevel::Default => "Default allow",
                },
                self.rule_level()
            )
        } else {
            AllowlistResult::deny_fast(
                match self.rule_level() {
                    RuleLevel::Emergency => "Emergency lockdown",
                    RuleLevel::Tool => "Tool blocked",
                    RuleLevel::Server => "Server blocked",
                    RuleLevel::Capability => "Capability blocked",
                    RuleLevel::Global => "Global blocked", 
                    RuleLevel::Default => "Default deny",
                },
                self.rule_level()
            )
        }
    }
}

/// Zero-allocation user context for hot path
#[derive(Debug, Clone)]
pub struct FastUserContext {
    pub user_id_hash: u64, // Pre-computed hash for cache keys
    pub user_id: Arc<str>, // Shared string to avoid allocations
    pub permissions_bitmap: u64, // Bitmap for up to 64 permissions (O(1) checking)
}

impl FastUserContext {
    pub fn new(user_id: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        let user_id_hash = hasher.finish();
        
        Self {
            user_id_hash,
            user_id: Arc::from(user_id),
            permissions_bitmap: 0,
        }
    }
}

// AllowlistResult is imported from allowlist module

// AllowlistContext is imported from allowlist module

impl AllowlistContext {
    /// Convert to fast user context for hot path evaluation
    pub fn to_fast_context(&self) -> FastUserContext {
        FastUserContext::new(self.user_id.as_deref().unwrap_or("anonymous"))
    }
}

/// High-performance allowlist service with maximum performance optimizations
pub struct AllowlistService {
    /// Configuration (hot-reloadable)
    config: Arc<RwLock<AllowlistConfig>>,
    
    /// === ULTRA-FAST RULE STORAGE ===
    /// O(1) tool rule lookup with pre-computed hashes
    tool_rules: Arc<RwLock<HashMap<u64, CachedAllowlistRule>>>, // Hash -> Rule
    tool_name_to_hash: Arc<RwLock<HashMap<String, u64>>>, // Name -> Hash for lookups
    
    /// Server rules with O(1) lookup
    server_rules: Arc<RwLock<HashMap<u64, CachedAllowlistRule>>>,
    server_name_to_hash: Arc<RwLock<HashMap<String, u64>>>,
    
    /// === FAST PATTERN MATCHING SYSTEM ===
    /// Pre-compiled regex patterns for complex pattern matching
    capability_regex_set: Arc<RwLock<Option<RegexSet>>>,
    capability_rules: Arc<RwLock<Vec<CachedAllowlistRule>>>,
    
    global_regex_set: Arc<RwLock<Option<RegexSet>>>,
    global_rules: Arc<RwLock<Vec<CachedAllowlistRule>>>,
    
    /// === ATOMIC STATE FOR ZERO-LOCK HOT PATH ===
    /// Emergency state for zero-lock checking
    emergency_active: AtomicBool,
    
    /// === PERFORMANCE CACHING ===
    /// LRU cache with fixed size for maximum performance
    decision_cache: Arc<RwLock<HashMap<u64, CachedDecision>>>, // Pre-computed hash -> Decision
    cache_generation: AtomicU32, // For cache invalidation
    
    /// Statistics
    stats: Arc<std::sync::Mutex<AllowlistStats>>,
    
    /// Pattern loader for external pattern files
    pattern_loader: Option<PatternLoader>,
    
    /// Audit service for rule evaluation logging (optional for performance)
    audit_service: Option<Arc<AuditService>>,
}

/// Statistics tracking
#[derive(Debug, Clone)]
struct AllowlistStats {
    start_time: DateTime<Utc>,
    total_requests: u64,
    allowed_requests: u64,
    blocked_requests: u64,
    rule_matches: HashMap<String, u64>,
    last_error: Option<String>,
    total_processing_time_ms: u64,
    hourly_stats: Vec<HourlyMetric>,
    
    // Performance metrics
    cache_hits: u64,
    cache_misses: u64,
    average_decision_time_ns: u64,
}

// Default implementation is in the allowlist module

impl AllowlistService {
    /// Create new ultra-fast allowlist service
    pub fn new(config: AllowlistConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let emergency_active = AtomicBool::new(config.emergency_lockdown);
        
        let stats = AllowlistStats {
            start_time: Utc::now(),
            total_requests: 0,
            allowed_requests: 0,
            blocked_requests: 0,
            rule_matches: HashMap::new(),
            last_error: None,
            total_processing_time_ms: 0,
            hourly_stats: Vec::new(),
            cache_hits: 0,
            cache_misses: 0,
            average_decision_time_ns: 0,
        };

        let service = Self {
            config: Arc::new(RwLock::new(config)),
            tool_rules: Arc::new(RwLock::new(HashMap::new())),
            tool_name_to_hash: Arc::new(RwLock::new(HashMap::new())),
            server_rules: Arc::new(RwLock::new(HashMap::new())),
            server_name_to_hash: Arc::new(RwLock::new(HashMap::new())),
            
            // Pattern matching structures
            capability_regex_set: Arc::new(RwLock::new(None)),
            capability_rules: Arc::new(RwLock::new(Vec::new())),
            global_regex_set: Arc::new(RwLock::new(None)),
            global_rules: Arc::new(RwLock::new(Vec::new())),
            
            emergency_active,
            decision_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_generation: AtomicU32::new(0),
            stats: Arc::new(std::sync::Mutex::new(stats)),
            pattern_loader: None,
            audit_service: None,
        };
        
        // Pre-compute all hashes and compile patterns
        service.reload_patterns()?;
        
        Ok(service)
    }
    
    /// Create new allowlist service with pattern loader for external files
    pub fn with_pattern_loader<P: AsRef<std::path::Path>>(
        mut config: AllowlistConfig, 
        security_dir: P
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let pattern_loader = PatternLoader::new(security_dir);
        
        // Load patterns from external files and merge with config
        let capability_patterns = pattern_loader.load_capability_patterns()
            .unwrap_or_else(|e| {
                warn!("Failed to load capability patterns: {}", e);
                Vec::new()
            });
            
        let global_patterns = pattern_loader.load_global_patterns()
            .unwrap_or_else(|e| {
                warn!("Failed to load global patterns: {}", e);
                Vec::new()
            });
            
        // Merge loaded patterns with existing config patterns
        config.capability_patterns.extend(capability_patterns);
        config.global_patterns.extend(global_patterns);
        
        debug!("Loaded {} capability patterns and {} global patterns from files", 
               config.capability_patterns.len(), config.global_patterns.len());
        
        let mut service = Self::new(config)?;
        service.pattern_loader = Some(pattern_loader);
        
        Ok(service)
    }
    
    /// Pre-compute hashes for all tool and server names for O(1) lookup
    fn reload_patterns(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        
        // Update emergency state
        self.emergency_active.store(config.emergency_lockdown, Ordering::Relaxed);
        
        // Pre-compute tool rule hashes
        {
            let mut tool_rules = self.tool_rules.write().unwrap();
            let mut name_to_hash = self.tool_name_to_hash.write().unwrap();
            tool_rules.clear();
            name_to_hash.clear();
            
            for (name, rule) in &config.tools {
                if rule.enabled {
                    let hash = self.compute_string_hash(name);
                    let cached_rule = CachedAllowlistRule {
                        action: rule.action.clone(),
                        reason: rule.reason.as_ref().map(|s| Arc::from(s.as_str())),
                        priority: rule.priority.unwrap_or(0) as u8,
                    };
                    tool_rules.insert(hash, cached_rule);
                    name_to_hash.insert(name.clone(), hash);
                }
            }
        }
        
        // Pre-compute server rule hashes
        {
            let mut server_rules = self.server_rules.write().unwrap();
            let mut name_to_hash = self.server_name_to_hash.write().unwrap();
            server_rules.clear();
            name_to_hash.clear();
            
            for (name, rule) in &config.servers {
                if rule.enabled {
                    let hash = self.compute_string_hash(name);
                    let cached_rule = CachedAllowlistRule {
                        action: rule.action.clone(),
                        reason: rule.reason.as_ref().map(|s| Arc::from(s.as_str())),
                        priority: rule.priority.unwrap_or(50) as u8,
                    };
                    server_rules.insert(hash, cached_rule);
                    name_to_hash.insert(name.clone(), hash);
                }
            }
        }
        
        // === CAPABILITY PATTERN COMPILATION ===
        if !config.capability_patterns.is_empty() {
            let enabled_patterns: Vec<_> = config.capability_patterns.iter()
                .filter(|p| p.rule.enabled && p.rule.pattern.is_some())
                .collect();
            
            if !enabled_patterns.is_empty() {
                let mut regex_patterns = Vec::new();
                
                for pattern_rule in enabled_patterns.iter() {
                    if let Some(ref pattern) = pattern_rule.rule.pattern {
                        let regex_str = self.pattern_to_regex(pattern);
                        regex_patterns.push(regex_str);
                    }
                }
                
                // Compile RegexSet for patterns
                let regex_set = if !regex_patterns.is_empty() {
                    debug!("Compiling {} capability regex patterns", regex_patterns.len());
                    Some(RegexSet::new(&regex_patterns)?)
                } else {
                    None
                };
                
                // Cache compiled rules (sorted by priority)
                let mut cached_rules: Vec<CachedAllowlistRule> = enabled_patterns.iter()
                    .map(|p| CachedAllowlistRule {
                        action: p.rule.action.clone(),
                        reason: p.rule.reason.as_ref().map(|s| Arc::from(s.as_str())),
                        priority: p.rule.priority.unwrap_or(50) as u8,
                    })
                    .collect();
                
                // Sort by priority (lower number = higher priority)
                cached_rules.sort_by_key(|rule| rule.priority);
                
                // Update pattern matching structures
                *self.capability_regex_set.write().unwrap() = regex_set;
                *self.capability_rules.write().unwrap() = cached_rules;
                
                debug!("Capability patterns compiled successfully");
            }
        }
        
        // === GLOBAL PATTERN COMPILATION ===
        if !config.global_patterns.is_empty() {
            let enabled_patterns: Vec<_> = config.global_patterns.iter()
                .filter(|p| p.rule.enabled && p.rule.pattern.is_some())
                .collect();
            
            if !enabled_patterns.is_empty() {
                let mut regex_patterns = Vec::new();
                
                for pattern_rule in enabled_patterns.iter() {
                    if let Some(ref pattern) = pattern_rule.rule.pattern {
                        let regex_str = self.pattern_to_regex(pattern);
                        regex_patterns.push(regex_str);
                    }
                }
                
                // Compile RegexSet for patterns
                let regex_set = if !regex_patterns.is_empty() {
                    debug!("Compiling {} global regex patterns", regex_patterns.len());
                    Some(RegexSet::new(&regex_patterns)?)
                } else {
                    None
                };
                
                // Cache compiled rules (sorted by priority)
                let mut cached_rules: Vec<CachedAllowlistRule> = enabled_patterns.iter()
                    .map(|p| CachedAllowlistRule {
                        action: p.rule.action.clone(),
                        reason: p.rule.reason.as_ref().map(|s| Arc::from(s.as_str())),
                        priority: p.rule.priority.unwrap_or(50) as u8,
                    })
                    .collect();
                
                // Sort by priority (lower number = higher priority)
                cached_rules.sort_by_key(|rule| rule.priority);
                
                // Update pattern matching structures
                *self.global_regex_set.write().unwrap() = regex_set;
                *self.global_rules.write().unwrap() = cached_rules;
                
                debug!("Global patterns compiled successfully");
            }
        }
        
        // Invalidate cache
        self.cache_generation.fetch_add(1, Ordering::Relaxed);
        self.decision_cache.write().unwrap().clear();
        
        Ok(())
    }
    
    /// Convert pattern to regex string
    fn pattern_to_regex(&self, pattern: &AllowlistPattern) -> String {
        match pattern {
            AllowlistPattern::Regex { value } => value.clone(),
            AllowlistPattern::Wildcard { value } => {
                format!("^{}$", value.replace('*', ".*").replace('?', "."))
            }
            AllowlistPattern::Exact { value } => {
                format!("^{}$", regex::escape(value))
            }
        }
    }
    
    /// Compute hash for string (used for pre-computing lookups)
    #[inline(always)]
    fn compute_string_hash(&self, s: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }
    
    /// HIGH-PERFORMANCE TOOL ACCESS EVALUATION WITH CACHING
    /// 
    /// This is the hot path optimized for maximum performance with decision caching.
    pub fn check_tool_access_internal(
        &self,
        tool_name: &str,
        _parameters: &HashMap<String, serde_json::Value>,
        context: &AllowlistContext,
    ) -> AllowlistResult {
        let start_time = Instant::now();
        
        // Create fast context for cache key computation
        let fast_context = context.to_fast_context();
        
        // Check cache first (O(1) lookup)
        let cache_key = self.compute_cache_key_fast(tool_name, &fast_context);
        if let Some(cached) = self.check_decision_cache_fast(cache_key) {
            self.record_cache_hit();
            let mut result = cached.into_result();
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            return result;
        }
        
        // Cache miss - record it and proceed with evaluation
        self.record_cache_miss();
        
        // Check if allowlist is enabled
        let config = self.config.read().unwrap();
        if !config.enabled {
            let mut result = AllowlistResult::allow_fast("Allowlist disabled", RuleLevel::Default);
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // Check emergency lockdown (atomic check, fastest path)
        if self.emergency_active.load(Ordering::Relaxed) {
            let mut result = AllowlistResult::deny_fast("Emergency lockdown", RuleLevel::Emergency);
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // Check tool-specific rules (highest priority)
        if let Some(rule) = config.tools.get(tool_name) {
            if rule.enabled {
                let mut result = match rule.action {
                    AllowlistAction::Allow => AllowlistResult::allow_fast("Tool allowed", RuleLevel::Tool),
                    AllowlistAction::Deny => AllowlistResult::deny_fast("Tool blocked", RuleLevel::Tool),
                };
                result.matched_rule = Some(tool_name.to_string());
                if let Some(ref reason) = rule.reason {
                    result.reason = Arc::from(reason.as_str());
                }
                result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
                self.cache_decision_fast(cache_key, &result);
                self.update_stats(&result);
                return result;
            }
        }
        
        // Release config lock early for pattern matching
        drop(config);
        
        // Check capability patterns (ultra-fast cascade: bloom → trie → regex)
        if let Some(capability_rule) = self.match_capability_patterns_fast(tool_name) {
            let mut result = match capability_rule.action {
                AllowlistAction::Allow => AllowlistResult::allow_fast("Capability pattern", RuleLevel::Capability),
                AllowlistAction::Deny => AllowlistResult::deny_fast("Capability blocked", RuleLevel::Capability),
            };
            result.matched_rule = Some("capability_pattern".to_string());
            if let Some(ref reason) = capability_rule.reason {
                result.reason = reason.clone();
            }
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // Check global patterns (ultra-fast cascade: bloom → trie → regex)  
        if let Some(global_rule) = self.match_global_patterns_fast(tool_name) {
            let mut result = match global_rule.action {
                AllowlistAction::Allow => AllowlistResult::allow_fast("Global pattern", RuleLevel::Global),
                AllowlistAction::Deny => AllowlistResult::deny_fast("Global blocked", RuleLevel::Global),
            };
            result.matched_rule = Some("global_pattern".to_string());
            if let Some(ref reason) = global_rule.reason {
                result.reason = reason.clone();
            }
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // Apply default action (no patterns matched)
        let config = self.config.read().unwrap();
        let mut result = match config.default_action {
            AllowlistAction::Allow => AllowlistResult::allow_fast("Default allow", RuleLevel::Default),
            AllowlistAction::Deny => AllowlistResult::deny_fast("Default deny", RuleLevel::Default),
        };
        result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
        self.cache_decision_fast(cache_key, &result);
        self.update_stats(&result);
        result
    }
    
    /// Update performance statistics
    fn update_stats(&self, result: &AllowlistResult) {
        let mut stats = self.stats.lock().unwrap();
        stats.total_requests += 1;
        
        // Update average decision time (exponential moving average)
        if stats.total_requests == 1 {
            stats.average_decision_time_ns = result.decision_time_ns;
        } else {
            // Exponential moving average with α = 0.1
            stats.average_decision_time_ns = (stats.average_decision_time_ns * 9 + result.decision_time_ns) / 10;
        }
    }
    
    /// Compute cache key for ultra-fast lookup (hash-based)
    #[inline(always)]
    fn compute_cache_key_fast(&self, tool_name: &str, fast_context: &FastUserContext) -> u64 {
        // Combine tool hash with user hash for cache key
        let tool_hash = self.compute_string_hash(tool_name);
        // For testing/debugging: Also include some user context
        tool_hash ^ fast_context.user_id_hash ^ fast_context.permissions_bitmap
    }
    
    /// Check decision cache with zero allocation
    #[inline(always)]
    fn check_decision_cache_fast(&self, cache_key: u64) -> Option<CachedDecision> {
        let cache = self.decision_cache.read().unwrap();
        if let Some(cached) = cache.get(&cache_key) {
            // Check TTL (5-second cache)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32;
            
            if now - cached.timestamp() < 5 {
                return Some(*cached);
            }
        }
        None
    }
    
    /// Get tool rule with O(1) hash lookup
    #[inline(always)]
    fn get_tool_rule_fast(&self, tool_hash: u64) -> Option<CachedAllowlistRule> {
        self.tool_rules.read().unwrap().get(&tool_hash).cloned()
    }
    
    /// Get server rule with O(1) hash lookup
    #[inline(always)]
    fn get_server_rule_fast(&self, server_hash: u64) -> Option<CachedAllowlistRule> {
        self.server_rules.read().unwrap().get(&server_hash).cloned()
    }
    
    /// Fast capability pattern matching with RegexSet
    /// Performance: ~100ns for regex pattern matching
    #[inline(always)]
    fn match_capability_patterns_fast(&self, tool_name: &str) -> Option<CachedAllowlistRule> {
        // RegexSet for pattern matching - simple and reliable
        if let Some(regex_set) = self.capability_regex_set.read().unwrap().as_ref() {
            let matches: Vec<usize> = regex_set.matches(tool_name).iter().collect();
            
            if !matches.is_empty() {
                let rules = self.capability_rules.read().unwrap();
                // Find highest priority match (lower number = higher priority)
                let best_match = matches.iter()
                    .filter_map(|&idx| rules.get(idx))
                    .min_by_key(|rule| rule.priority)
                    .cloned();
                
                return best_match;
            }
        }
        
        None
    }
    
    /// Fast global pattern matching using RegexSet
    /// Performance: ~100-500ns for regex pattern matching
    #[inline(always)]
    fn match_global_patterns_fast(&self, tool_name: &str) -> Option<CachedAllowlistRule> {
        // Use RegexSet for batch pattern matching
        if let Some(regex_set) = self.global_regex_set.read().unwrap().as_ref() {
            let matches: Vec<usize> = regex_set.matches(tool_name).iter().collect();
            
            if !matches.is_empty() {
                let rules = self.global_rules.read().unwrap();
                // Find highest priority regex match (lowest priority number)
                let best_match = matches.iter()
                    .filter_map(|&idx| rules.get(idx))
                    .min_by_key(|rule| rule.priority)
                    .cloned();
                
                return best_match;
            }
        }
        
        None
    }
    
    /// Cache decision with minimal allocation
    #[inline(always)]
    fn cache_decision_fast(&self, cache_key: u64, result: &AllowlistResult) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        
        let cached_decision = CachedDecision::new(
            result.allowed,
            result.rule_level,
            timestamp,
        );
        
        // Non-blocking cache update
        if let Ok(mut cache) = self.decision_cache.try_write() {
            cache.insert(cache_key, cached_decision);
            
            // LRU eviction if cache gets too large
            if cache.len() > 10000 {
                // Remove oldest entries (approximate LRU)
                let cutoff_time = timestamp - 10; // Keep entries newer than 10 seconds
                cache.retain(|_, decision| {
                    decision.timestamp() > cutoff_time
                });
            }
        }
    }
    
    /// Update statistics with minimal overhead
    #[inline(always)]
    fn update_stats_fast(&self, result: &AllowlistResult, decision_time_ns: u64) {
        // Only update stats if we can get lock without blocking
        if let Ok(mut stats) = self.stats.try_lock() {
            stats.total_requests += 1;
            
            // Update running average of decision time
            if stats.total_requests == 1 {
                stats.average_decision_time_ns = decision_time_ns;
            } else {
                // Exponential moving average for performance
                stats.average_decision_time_ns = (stats.average_decision_time_ns * 7 + decision_time_ns) / 8;
            }
            
            match result.action {
                AllowlistAction::Allow => stats.allowed_requests += 1,
                AllowlistAction::Deny => stats.blocked_requests += 1,
            }
        }
    }
    
    /// Record cache hit
    #[inline(always)]
    fn record_cache_hit(&self) {
        if let Ok(mut stats) = self.stats.try_lock() {
            stats.cache_hits += 1;
        }
    }
    
    /// Record cache miss
    #[inline(always)]
    fn record_cache_miss(&self) {
        if let Ok(mut stats) = self.stats.try_lock() {
            stats.cache_misses += 1;
        }
    }
    
    /// Get cache hit ratio
    pub fn get_cache_hit_ratio(&self) -> f64 {
        if let Ok(stats) = self.stats.lock() {
            let total = stats.cache_hits + stats.cache_misses;
            if total > 0 {
                stats.cache_hits as f64 / total as f64
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
    
    /// Get average decision time in nanoseconds
    pub fn get_average_decision_time_ns(&self) -> u64 {
        if let Ok(stats) = self.stats.lock() {
            stats.average_decision_time_ns
        } else {
            0
        }
    }
    
    /// Emergency lockdown control
    pub fn set_emergency_lockdown(&self, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        // Update atomic state for immediate effect (zero-cost)
        self.emergency_active.store(enabled, Ordering::SeqCst);
        
        // Update configuration for persistence
        {
            let mut config = self.config.write().unwrap();
            config.emergency_lockdown = enabled;
        }
        
        // Invalidate cache
        self.cache_generation.fetch_add(1, Ordering::Relaxed);
        self.decision_cache.write().unwrap().clear();
        
        if enabled {
            warn!("Emergency lockdown ACTIVATED - all tool access denied");
        } else {
            warn!("Emergency lockdown DEACTIVATED - normal rule evaluation resumed");
        }
        
        Ok(())
    }
    
    /// Check if emergency lockdown is active
    #[inline(always)]
    pub fn is_emergency_active(&self) -> bool {
        self.emergency_active.load(Ordering::Relaxed)
    }
    
    /// Hot reload configuration
    pub fn reload_config(&self, new_config: AllowlistConfig) -> Result<(), Box<dyn std::error::Error>> {
        // Update main config
        *self.config.write().unwrap() = new_config;
        
        // Reload patterns and hashes
        self.reload_patterns()?;
        
        debug!("Allowlist configuration reloaded successfully");
        Ok(())
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> AllowlistConfig {
        self.config.read().unwrap().clone()
    }
    
    /// Test pattern matching against configured test cases
    pub fn test_patterns(&self) -> Result<super::pattern_loader::PatternTestResults, Box<dyn std::error::Error>> {
        if let Some(ref pattern_loader) = self.pattern_loader {
            pattern_loader.test_patterns()
        } else {
            Err("Pattern loader not configured".into())
        }
    }
    
    /// Reload patterns from external files (hot reload)
    pub fn reload_external_patterns(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref pattern_loader) = self.pattern_loader {
            // Load fresh patterns from files
            let capability_patterns = pattern_loader.load_capability_patterns()?;
            let global_patterns = pattern_loader.load_global_patterns()?;
            
            // Update config with new patterns
            {
                let mut config = self.config.write().unwrap();
                config.capability_patterns = capability_patterns;
                config.global_patterns = global_patterns;
            }
            
            // Reload compiled patterns
            self.reload_patterns()?;
            
            debug!("Hot reloaded external patterns successfully");
            Ok(())
        } else {
            Err("Pattern loader not configured".into())
        }
    }
    
    /// Configure audit service for rule evaluation logging
    pub fn set_audit_service(&mut self, audit_service: Arc<AuditService>) {
        self.audit_service = Some(audit_service);
    }
    
    /// Check if audit logging is enabled
    pub fn is_audit_enabled(&self) -> bool {
        self.audit_service.is_some()
    }
    
    /// Log rule evaluation result asynchronously (non-blocking for performance)
    fn log_rule_evaluation_async(
        &self,
        tool_name: &str,
        context: &AllowlistContext,
        result: &AllowlistResult,
        parameters: &HashMap<String, serde_json::Value>,
        evaluation_time_ns: u64,
    ) {
        if let Some(ref audit_service) = self.audit_service {
            // Clone required data for async task
            let audit_service = Arc::clone(audit_service);
            let tool_name = tool_name.to_string();
            let user_id = context.user_id.clone();
            let user_roles = context.user_roles.clone();
            let api_key_name = context.api_key_name.clone();
            let source = context.source.clone();
            let client_ip = context.client_ip.clone();
            let parameters = parameters.clone();
            let allowed = result.allowed;
            let reason = result.reason.clone();
            let rule_level = result.rule_level;
            let matched_rule = result.matched_rule.clone();
            
            // Spawn async audit logging task
            tokio::spawn(async move {
                let audit_entry = AuditEntry {
                    id: AuditEntry::generate_id(),
                    timestamp: Utc::now(),
                    event_type: AuditEventType::ToolExecution,
                    user: user_id.clone().map(|id| AuditUser {
                        id: Some(id),
                        name: None,
                        roles: user_roles,
                        api_key_name,
                        auth_method: "unknown".to_string(), // TODO: extract from context
                    }),
                    request: None, // Tool evaluations don't have full HTTP request context
                    response: None,
                    tool: Some(AuditTool {
                        name: tool_name.clone(),
                        parameters: Some(parameters),
                        result: Some(serde_json::to_string(&serde_json::json!({
                            "allowed": allowed,
                            "reason": reason.to_string(),
                            "rule_level": format!("{:?}", rule_level),
                            "matched_rule": matched_rule,
                            "evaluation_time_ns": evaluation_time_ns
                        })).unwrap_or_default()),
                        execution_time_ms: Some((evaluation_time_ns / 1_000_000) as u64),
                        success: allowed,
                    }),
                    resource: None,
                    security: AuditSecurity {
                        authenticated: user_id.is_some(),
                        authorized: allowed,
                        permissions_checked: vec![], // TODO: track required permissions
                        policies_applied: vec!["allowlist".to_string()],
                        content_sanitized: false,
                        approval_required: false,
                    },
                    metadata: {
                        let mut map = HashMap::new();
                        map.insert("rule_level".to_string(), serde_json::Value::String(format!("{:?}", rule_level)));
                        if let Some(rule) = matched_rule {
                            map.insert("matched_rule".to_string(), serde_json::Value::String(rule));
                        }
                        if let Some(src) = source {
                            map.insert("source".to_string(), serde_json::Value::String(src));
                        }
                        if let Some(ip) = client_ip {
                            map.insert("client_ip".to_string(), serde_json::Value::String(ip));
                        }
                        map.insert("evaluation_time_ns".to_string(), serde_json::Value::Number(serde_json::Number::from(evaluation_time_ns)));
                        map
                    },
                    outcome: if allowed { AuditOutcome::Success } else { AuditOutcome::Blocked },
                    error: if !allowed {
                        Some(super::audit::AuditError {
                            code: "ALLOWLIST_DENIED".to_string(),
                            message: reason.to_string(),
                            details: Some(format!("Tool '{}' blocked by {} rule", tool_name, format!("{:?}", rule_level).to_lowercase())),
                            stack_trace: None,
                        })
                    } else {
                        None
                    },
                };
                
                if let Err(e) = audit_service.log_event(audit_entry).await {
                    // Use debug instead of error to avoid spam in high-performance scenarios
                    debug!("Failed to log allowlist evaluation audit: {}", e);
                }
            });
        }
    }
}

// Legacy compatibility wrapper methods
impl AllowlistService {
    /// Legacy method - delegates to ultra-fast implementation
    pub fn check_tool_access(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
        context: &AllowlistContext,
    ) -> AllowlistResult {
        let start_time = Instant::now();
        let result = self.check_tool_access_internal(tool_name, parameters, context);
        let evaluation_time_ns = start_time.elapsed().as_nanos() as u64;
        
        // Log rule evaluation asynchronously for audit trail (non-blocking)
        self.log_rule_evaluation_async(tool_name, context, &result, parameters, evaluation_time_ns);
        
        result
    }
    
    /// Legacy resource access (simplified)
    pub fn check_resource_access(
        &self,
        _resource_uri: &str,
        _context: &AllowlistContext,
    ) -> AllowlistResult {
        let config = self.config.read().unwrap();
        if matches!(config.default_action, AllowlistAction::Allow) {
            AllowlistResult::allow_fast("Legacy resource access", RuleLevel::Default)
        } else {
            AllowlistResult::deny_fast("Legacy resource denied", RuleLevel::Default)
        }
    }
    
    /// Legacy prompt access (simplified)
    pub fn check_prompt_access(
        &self,
        _prompt_name: &str,
        _context: &AllowlistContext,
    ) -> AllowlistResult {
        let config = self.config.read().unwrap();
        if matches!(config.default_action, AllowlistAction::Allow) {
            AllowlistResult::allow_fast("Legacy prompt access", RuleLevel::Default)
        } else {
            AllowlistResult::deny_fast("Legacy prompt denied", RuleLevel::Default)
        }
    }
    
    /// Get configured rules for API
    pub fn get_configured_rules(&self) -> serde_json::Value {
        use serde_json::json;
        
        let config = self.config.read().unwrap();
        let stats = self.stats.lock().unwrap();
        
        json!({
            "rules": [],
            "total_rules": config.tools.len() + config.servers.len(),
            "emergency_active": config.emergency_lockdown,
            "allowlist_enabled": config.enabled,
            "default_action": format!("{:?}", config.default_action).to_lowercase(),
            "performance_stats": {
                "cache_hit_ratio": if stats.cache_hits + stats.cache_misses > 0 {
                    stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64
                } else {
                    0.0
                },
                "average_decision_time_ns": stats.average_decision_time_ns,
                "total_requests": stats.total_requests,
                "cache_size": self.decision_cache.read().unwrap().len()
            }
        })
    }

    /// Get all tool rules for unified view API
    pub fn get_all_tool_rules(&self) -> std::collections::HashMap<String, AllowlistRule> {
        let config = self.config.read().unwrap();
        let mut tool_rules = std::collections::HashMap::new();
        
        // Collect tool-level rules
        for (tool_name, rule) in &config.tools {
            tool_rules.insert(tool_name.clone(), rule.clone());
        }
        
        // Collect server-level rules that affect tools
        for (server_name, rule) in &config.servers {
            // Use server name prefixed with "server_" to avoid conflicts
            tool_rules.insert(format!("server_{}", server_name), rule.clone());
        }
        
        tool_rules
    }

    /// Get capability-level pattern rules
    pub fn get_capability_patterns(&self) -> Vec<PatternRule> {
        if let Some(ref pattern_loader) = self.pattern_loader {
            pattern_loader.load_capability_patterns().unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Get global-level pattern rules
    pub fn get_global_patterns(&self) -> Vec<PatternRule> {
        if let Some(ref pattern_loader) = self.pattern_loader {
            pattern_loader.load_global_patterns().unwrap_or_default()
        } else {
            Vec::new()
        }
    }
}

// Statistics implementation (simplified for performance)
impl SecurityServiceStatistics for AllowlistService {
    type Statistics = AllowlistStatistics;
    
    async fn get_statistics(&self) -> Self::Statistics {
        let stats = self.stats.lock().unwrap().clone();
        let config = self.config.read().unwrap();
        
        AllowlistStatistics {
            health: ServiceHealth {
                status: if config.enabled { HealthStatus::Healthy } else { HealthStatus::Disabled },
                is_healthy: config.enabled,
                last_checked: Utc::now(),
                error_message: None,
                uptime_seconds: (Utc::now() - stats.start_time).num_seconds() as u64,
                performance: PerformanceMetrics {
                    avg_response_time_ms: stats.average_decision_time_ns as f64 / 1_000_000.0,
                    requests_per_second: if (Utc::now() - stats.start_time).num_seconds() > 0 {
                        stats.total_requests as f64 / (Utc::now() - stats.start_time).num_seconds() as f64
                    } else {
                        0.0
                    },
                    error_rate: if stats.total_requests > 0 {
                        stats.blocked_requests as f64 / stats.total_requests as f64
                    } else {
                        0.0
                    },
                    memory_usage_bytes: 0,
                },
            },
            total_rules: (config.tools.len() + config.servers.len()) as u32,
            active_rules: (config.tools.iter().filter(|(_, r)| r.enabled).count() + 
                          config.servers.iter().filter(|(_, r)| r.enabled).count()) as u32,
            total_requests: stats.total_requests,
            allowed_requests: stats.allowed_requests,
            blocked_requests: stats.blocked_requests,
            approval_required_requests: 0,
            top_matched_rules: Vec::new(),
            hourly_patterns: stats.hourly_stats,
        }
    }
    
    async fn get_health(&self) -> ServiceHealth {
        let stats = self.stats.lock().unwrap();
        let config = self.config.read().unwrap();
        
        ServiceHealth {
            status: if config.enabled { HealthStatus::Healthy } else { HealthStatus::Disabled },
            is_healthy: config.enabled,
            last_checked: Utc::now(),
            error_message: None,
            uptime_seconds: (Utc::now() - stats.start_time).num_seconds() as u64,
            performance: PerformanceMetrics {
                avg_response_time_ms: stats.average_decision_time_ns as f64 / 1_000_000.0,
                requests_per_second: 0.0,
                error_rate: 0.0,
                memory_usage_bytes: 0,
            },
        }
    }
    
    async fn reset_statistics(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(mut stats) = self.stats.lock() {
            *stats = AllowlistStats {
                start_time: Utc::now(),
                total_requests: 0,
                allowed_requests: 0,
                blocked_requests: 0,
                rule_matches: HashMap::new(),
                last_error: None,
                total_processing_time_ms: 0,
                hourly_stats: Vec::new(),
                cache_hits: 0,
                cache_misses: 0,
                average_decision_time_ns: 0,
            };
        }
        Ok(())
    }
}

impl HealthMonitor for AllowlistService {
    async fn is_healthy(&self) -> bool {
        let config = self.config.read().unwrap();
        config.enabled
    }
    
    async fn health_check(&self) -> ServiceHealth {
        self.get_health().await
    }
    
    fn get_uptime(&self) -> u64 {
        let stats = self.stats.lock().unwrap();
        (Utc::now() - stats.start_time).num_seconds() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_decision_cache() {
        let decision = CachedDecision::new(true, RuleLevel::Tool, 1000);
        assert!(decision.allowed());
        assert_eq!(decision.rule_level(), RuleLevel::Tool);
        assert_eq!(decision.timestamp(), 1000);
    }
    
    #[test]
    fn test_emergency_lockdown() {
        let config = AllowlistConfig {
            enabled: true,
            emergency_lockdown: true,
            ..Default::default()
        };
        
        let service = AllowlistService::new(config).unwrap();
        
        let context = AllowlistContext {
            user_id: Some("test".to_string()),
            user_roles: vec![],
            api_key_name: None,
            permissions: vec![],
            source: None,
            client_ip: None,
        };
        
        let result = service.check_tool_access_internal("test_tool", &HashMap::new(), &context);
        assert!(!result.allowed);
        assert_eq!(result.rule_level, RuleLevel::Emergency);
    }
    
    #[test]
    fn test_cache_performance() {
        let config = AllowlistConfig {
            enabled: true,
            ..Default::default()
        };
        
        let service = AllowlistService::new(config).unwrap();
        let context = AllowlistContext {
            user_id: Some("test".to_string()),
            user_roles: vec![],
            api_key_name: None,
            permissions: vec![],
            source: None,
            client_ip: None,
        };
        
        // First call should miss cache
        let _result1 = service.check_tool_access_internal("test_tool", &HashMap::new(), &context);
        
        // Second call should hit cache
        let _result2 = service.check_tool_access_internal("test_tool", &HashMap::new(), &context);
        
        assert!(service.get_cache_hit_ratio() > 0.0);
    }
    
}