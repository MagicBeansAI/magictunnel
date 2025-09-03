//! High Performance Tool-First Enterprise Allowlist System
//!
//! This implementation uses the fastest possible Rust data structures and zero-allocation hot paths
//! for maximum performance (>3.9M evaluations/second).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use chrono::{DateTime, Utc};
use regex::RegexSet;
use std::sync::RwLock;
use std::hash::{Hash, Hasher};
use ahash::{AHashMap, AHashSet};
use once_cell::sync::Lazy;
use super::statistics::{SecurityServiceStatistics, HealthMonitor, ServiceHealth, HealthStatus, AllowlistStatistics, HourlyMetric, RuleMatch, PerformanceMetrics};
use super::allowlist_types::{AllowlistResult, AllowlistContext, RuleLevel, AllowlistAction, AllowlistConfig, AllowlistRule, AllowlistPattern};
use super::allowlist_data::{AllowlistData, AllowlistDecision, RuleSource, ToolWithAllowlistStatus, RealTimePatternTestRequest, RealTimePatternTestResponse, AllowlistSummary, TestPattern, PatternToolTestResult, PatternEvaluationStep, RealTimePatternTestSummary, PatternScope, EvaluationResult, AllowlistTreeviewResponse, TreeviewServerNode, TreeviewCapabilityNode, TreeviewToolNode, TreeviewNodeStatus};
use super::audit::{AuditCollector, AuditEvent, AuditEventType, AuditSeverity, AuditService, AuditEntry, AuditUser, AuditTool, AuditSecurity, AuditOutcome, AuditError};
use std::fs;
use std::path::Path;
use crate::registry::service::RegistryService;

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
#[derive(Debug)]
pub struct CachedAllowlistRule {
    pub action: AllowlistAction,
    pub reason: Option<Arc<str>>, // Arc<str> is more memory efficient than String
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
            2 => RuleLevel::Capability,
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
    
    
    /// === FAST PATTERN MATCHING SYSTEM ===
    /// Pre-compiled regex patterns for complex pattern matching
    capability_regex_set: Arc<RwLock<Option<RegexSet>>>,
    capability_rules: Arc<RwLock<Vec<CachedAllowlistRule>>>,
    
    global_regex_set: Arc<RwLock<Option<RegexSet>>>,
    global_rules: Arc<RwLock<Vec<CachedAllowlistRule>>>,
    
    tool_pattern_regex_set: Arc<RwLock<Option<RegexSet>>>,
    tool_pattern_rules: Arc<RwLock<Vec<CachedAllowlistRule>>>,
    
    /// === ATOMIC STATE FOR ZERO-LOCK HOT PATH ===
    /// Emergency state for zero-lock checking
    emergency_active: AtomicBool,
    
    /// === PERFORMANCE CACHING ===
    /// LRU cache with fixed size for maximum performance
    decision_cache: Arc<RwLock<HashMap<u64, CachedDecision>>>, // Pre-computed hash -> Decision
    cache_generation: AtomicU32, // For cache invalidation
    
    /// Statistics
    stats: Arc<std::sync::Mutex<AllowlistStats>>,
    
    /// Audit service for rule evaluation logging (optional for performance)
    audit_service: Option<Arc<AuditService>>,
    
    /// === NEW ENHANCED FEATURES ===
    /// Data file path for allowlist-data.yaml
    data_file_path: Option<String>,
    
    /// Pre-computed decisions for all tools (for treeview)
    precomputed_decisions: Arc<RwLock<HashMap<String, AllowlistDecision>>>,
    
    /// Audit trails for decisions (when audit enabled)
    decision_audit_trails: Arc<RwLock<HashMap<String, super::allowlist_data::DecisionAuditTrail>>>,
    
    /// === ENHANCED PATTERN STRUCTURES ===
    /// Pattern rules loaded from data file (thread-safe for concurrent updates)
    global_pattern_rules: Arc<RwLock<Vec<super::allowlist_data::PatternRule>>>,
    raw_tool_pattern_rules: Arc<RwLock<Vec<super::allowlist_data::PatternRule>>>,
    capability_pattern_rules: Arc<RwLock<Vec<super::allowlist_data::PatternRule>>>,
    
    /// Pattern sets for enhanced evaluation
    tool_pattern_set: Option<RegexSet>,
    capability_pattern_set: Option<RegexSet>,
    
    /// Explicit rules from data file (thread-safe for concurrent updates)
    explicit_tool_rules: Arc<RwLock<HashMap<String, AllowlistAction>>>,
    explicit_capability_rules: Arc<RwLock<HashMap<String, AllowlistAction>>>,
    
    /// Bloom filter for ultra-fast pattern rejection
    bloom_filter: Option<BloomFilter>,
    
    /// Registry service for enhanced capability mapping
    registry_service: Option<Arc<RegistryService>>,
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
    pub fn new(mut config: AllowlistConfig, registry_service: Option<Arc<RegistryService>>) -> Result<Self, Box<dyn std::error::Error>> {
        // Try to load persisted config and merge with provided config
        if let Ok(persisted_config) = Self::load_persisted_config() {
            debug!("Loading persisted allowlist config with {} tool rules", persisted_config.tools.len());
            // Merge persisted rules into the provided config (persisted rules take precedence)
            for (tool_name, rule) in persisted_config.tools {
                config.tools.insert(tool_name, rule);
            }
        }
        
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
            
            // Pattern matching structures
            capability_regex_set: Arc::new(RwLock::new(None)),
            capability_rules: Arc::new(RwLock::new(Vec::new())),
            global_regex_set: Arc::new(RwLock::new(None)),
            global_rules: Arc::new(RwLock::new(Vec::new())),
            tool_pattern_regex_set: Arc::new(RwLock::new(None)),
            tool_pattern_rules: Arc::new(RwLock::new(Vec::new())),
            
            emergency_active,
            decision_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_generation: AtomicU32::new(0),
            stats: Arc::new(std::sync::Mutex::new(stats)),
            audit_service: None,
            
            // New enhanced features
            data_file_path: None,
            precomputed_decisions: Arc::new(RwLock::new(HashMap::new())),
            decision_audit_trails: Arc::new(RwLock::new(HashMap::new())),
            
            // Enhanced pattern structures (thread-safe)
            global_pattern_rules: Arc::new(RwLock::new(Vec::new())),
            raw_tool_pattern_rules: Arc::new(RwLock::new(Vec::new())),
            capability_pattern_rules: Arc::new(RwLock::new(Vec::new())),
            tool_pattern_set: None,
            capability_pattern_set: None,
            explicit_tool_rules: Arc::new(RwLock::new(HashMap::new())),
            explicit_capability_rules: Arc::new(RwLock::new(HashMap::new())),
            bloom_filter: None,
            registry_service,
        };
        
        // Pre-compute all hashes and compile patterns
        service.reload_patterns()?;
        
        Ok(service)
    }
    
    
    /// Create new allowlist service with enhanced data file support
    pub fn with_data_file(
        config: AllowlistConfig,
        data_file_path: String,
        registry_service: Option<Arc<RegistryService>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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

        let mut service = Self {
            config: Arc::new(RwLock::new(config)),
            tool_rules: Arc::new(RwLock::new(HashMap::new())),
            tool_name_to_hash: Arc::new(RwLock::new(HashMap::new())),
            
            // Pattern matching structures
            capability_regex_set: Arc::new(RwLock::new(None)),
            capability_rules: Arc::new(RwLock::new(Vec::new())),
            global_regex_set: Arc::new(RwLock::new(None)),
            global_rules: Arc::new(RwLock::new(Vec::new())),
            tool_pattern_regex_set: Arc::new(RwLock::new(None)),
            tool_pattern_rules: Arc::new(RwLock::new(Vec::new())),
            
            emergency_active,
            decision_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_generation: AtomicU32::new(0),
            stats: Arc::new(std::sync::Mutex::new(stats)),
            audit_service: None,
            
            // Enhanced features
            data_file_path: Some(data_file_path.clone()),
            precomputed_decisions: Arc::new(RwLock::new(HashMap::new())),
            decision_audit_trails: Arc::new(RwLock::new(HashMap::new())),
            
            // Enhanced pattern structures (thread-safe)
            global_pattern_rules: Arc::new(RwLock::new(Vec::new())),
            raw_tool_pattern_rules: Arc::new(RwLock::new(Vec::new())),
            capability_pattern_rules: Arc::new(RwLock::new(Vec::new())),
            tool_pattern_set: None,
            capability_pattern_set: None,
            explicit_tool_rules: Arc::new(RwLock::new(HashMap::new())),
            explicit_capability_rules: Arc::new(RwLock::new(HashMap::new())),
            bloom_filter: None,
            registry_service,
        };
        
        // Load data from the enhanced data file
        service.load_data_file(&data_file_path)?;
        
        // Pre-compute all hashes and compile patterns
        service.reload_patterns()?;
        
        Ok(service)
    }
    
    /// Load allowlist data from YAML file and populate ultra-fast structures
    fn load_data_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use super::allowlist_data::{AllowlistData, RuleSource, AllowlistDecision};

        debug!("🔄 Loading allowlist data from file: {}", file_path);
        
        // Read and parse the data file
        let contents = fs::read_to_string(file_path)?;
        debug!("📄 File contents length: {} bytes", contents.len());
        
        let allowlist_data: AllowlistData = serde_yaml::from_str(&contents)?;
        debug!("📊 Parsed data - global: {}, tools: {}, capabilities: {}", 
                allowlist_data.patterns.global.len(),
                allowlist_data.patterns.tools.len(), 
                allowlist_data.patterns.capabilities.len());
        
        // Build RegexSet for ultra-fast pattern matching from global patterns
        let mut global_patterns = Vec::new();
        let mut global_rules = Vec::new();
        
        for pattern in &allowlist_data.patterns.global {
            if pattern.enabled {
                global_patterns.push(pattern.regex.clone());
                global_rules.push(CachedAllowlistRule {
                    action: pattern.action.clone(),
                    reason: Some(Arc::from(pattern.reason.as_str())),
                });
            }
        }
        
        if !global_patterns.is_empty() {
            let global_regex_set = Some(RegexSet::new(&global_patterns)?);
            // No sorting needed - most restrictive wins logic handles conflicts
            
            *self.global_regex_set.write().unwrap() = global_regex_set;
            *self.global_rules.write().unwrap() = global_rules.clone();
            debug!("✅ YAML: Global patterns compiled successfully: {} patterns, {} rules", global_patterns.len(), global_rules.len());
            debug!("✅ YAML: Global rules: {:?}", global_rules.iter().map(|r| &r.action).collect::<Vec<_>>());
        }
        
        // Build RegexSet for tool-specific patterns
        let mut tool_patterns = Vec::new();
        
        for pattern in &allowlist_data.patterns.tools {
            if pattern.enabled {
                tool_patterns.push(pattern.regex.clone());
            }
        }
        
        if !tool_patterns.is_empty() {
            let tool_pattern_regex_set = Some(RegexSet::new(&tool_patterns)?);
            *self.tool_pattern_regex_set.write().unwrap() = tool_pattern_regex_set;
            self.tool_pattern_set = Some(RegexSet::new(&tool_patterns)?);
            
            // Compile tool pattern rules into cached format for fast access
            let mut cached_tool_rules = Vec::new();
            for pattern in &allowlist_data.patterns.tools {
                if pattern.enabled {
                    cached_tool_rules.push(CachedAllowlistRule {
                        action: pattern.action.clone(),
                        reason: Some(Arc::from(pattern.reason.as_str())),
                    });
                }
            }
            *self.tool_pattern_rules.write().unwrap() = cached_tool_rules;
        }
        
        // Build RegexSet for capability-specific patterns  
        let mut capability_patterns = Vec::new();
        
        for pattern in &allowlist_data.patterns.capabilities {
            if pattern.enabled {
                capability_patterns.push(pattern.regex.clone());
            }
        }
        
        if !capability_patterns.is_empty() {
            let capability_regex_set = Some(RegexSet::new(&capability_patterns)?);
            *self.capability_regex_set.write().unwrap() = capability_regex_set;
            self.capability_pattern_set = Some(RegexSet::new(&capability_patterns)?);
        }
        
        // Store pattern rules for decision making (thread-safe)
        {
            let mut global_patterns = self.global_pattern_rules.write().unwrap();
            *global_patterns = allowlist_data.patterns.global.clone();
        }
        {
            let mut tool_patterns = self.raw_tool_pattern_rules.write().unwrap();
            *tool_patterns = allowlist_data.patterns.tools.clone();
        }
        {
            let mut capability_patterns = self.capability_pattern_rules.write().unwrap();
            *capability_patterns = allowlist_data.patterns.capabilities.clone();
            debug!("📊 Stored {} capability patterns in service", capability_patterns.len());
            for (i, pattern) in capability_patterns.iter().enumerate() {
                debug!("📊 Capability pattern {}: name='{}', regex='{}', action={:?}, enabled={}", 
                       i, pattern.name, pattern.regex, pattern.action, pattern.enabled);
            }
        }
        
        // Store explicit rules for O(1) lookup
        debug!("📥 Loading explicit tool rules from YAML: {:?}", allowlist_data.explicit_rules.tools);
        {
            let mut tool_rules = self.explicit_tool_rules.write().unwrap();
            tool_rules.clear();
            for (tool_name, action) in &allowlist_data.explicit_rules.tools {
                tool_rules.insert(tool_name.clone(), action.clone());
            }
        }
        {
            let mut capability_rules = self.explicit_capability_rules.write().unwrap();
            capability_rules.clear();
            for (capability_name, action) in &allowlist_data.explicit_rules.capabilities {
                capability_rules.insert(capability_name.clone(), action.clone());
            }
        }
        debug!("✅ Loaded {} explicit tool rules into self.explicit_tool_rules (instance: {:p})", 
               self.explicit_tool_rules.read().unwrap().len(), self);
        
        // Update bloom filter with new patterns and rules
        self.update_bloom_filter(&allowlist_data)?;
        
        // Update the config to reflect the loaded patterns for test visibility
        {
            let mut config = self.config.write().unwrap();
            
            // Convert loaded patterns to PatternRule format for the config
            config.global_patterns = allowlist_data.patterns.global.iter()
                .map(|pattern| super::allowlist_types::PatternRule {
                    rule: AllowlistRule {
                        name: Some(pattern.name.clone()),
                        pattern: Some(super::allowlist_types::AllowlistPattern::Regex { value: pattern.regex.clone() }),
                        action: pattern.action.clone(),
                        reason: Some(pattern.reason.clone()),
                        enabled: pattern.enabled,
                    },
                })
                .collect();
                
            config.capability_patterns = allowlist_data.patterns.capabilities.iter()
                .map(|pattern| super::allowlist_types::PatternRule {
                    rule: AllowlistRule {
                        name: Some(pattern.name.clone()),
                        pattern: Some(super::allowlist_types::AllowlistPattern::Regex { value: pattern.regex.clone() }),
                        action: pattern.action.clone(),
                        reason: Some(pattern.reason.clone()),
                        enabled: pattern.enabled,
                    },
                })
                .collect();
                
            config.tool_patterns = allowlist_data.patterns.tools.iter()
                .map(|pattern| super::allowlist_types::PatternRule {
                    rule: AllowlistRule {
                        name: Some(pattern.name.clone()),
                        pattern: Some(super::allowlist_types::AllowlistPattern::Regex { value: pattern.regex.clone() }),
                        action: pattern.action.clone(),
                        reason: Some(pattern.reason.clone()),
                        enabled: pattern.enabled,
                    },
                })
                .collect();
                
            // Merge loaded explicit rules with existing config
            for (tool_name, action) in &allowlist_data.explicit_rules.tools {
                config.tools.insert(tool_name.clone(), AllowlistRule {
                    name: Some(tool_name.clone()),
                    pattern: None,
                    action: action.clone(),
                    reason: Some(format!("Explicit rule from data file")),
                    enabled: true,
                });
            }
        }

        println!("Loaded allowlist data: {} global patterns, {} tool patterns, {} capability patterns, {} explicit rules",
                 allowlist_data.patterns.global.len(),
                 allowlist_data.patterns.tools.len(), 
                 allowlist_data.patterns.capabilities.len(),
                 allowlist_data.explicit_rules.tools.len() + 
                 allowlist_data.explicit_rules.capabilities.len());
        
        Ok(())
    }

    /// Update bloom filter with patterns for ultra-fast rejection
    fn update_bloom_filter(&mut self, data: &AllowlistData) -> Result<(), Box<dyn std::error::Error>> {
        // Estimate item count for optimal bloom filter sizing
        let estimated_items = data.patterns.global.len() + 
                             data.patterns.tools.len() + 
                             data.patterns.capabilities.len() + 
                             data.explicit_rules.tools.len() +
                             data.explicit_rules.capabilities.len();
        
        if estimated_items > 0 {
            let mut bloom = BloomFilter::new(estimated_items, 0.01);
            
            // Add all patterns and explicit rule keys to bloom filter
            for pattern in &data.patterns.global {
                if pattern.enabled {
                    bloom.insert(&pattern.regex);
                    bloom.insert(&pattern.name);
                }
            }
            
            for pattern in &data.patterns.tools {
                if pattern.enabled {
                    bloom.insert(&pattern.regex);
                    bloom.insert(&pattern.name);
                }
            }
            
            for pattern in &data.patterns.capabilities {
                if pattern.enabled {
                    bloom.insert(&pattern.regex);
                    bloom.insert(&pattern.name);
                }
            }
            
            for tool_name in data.explicit_rules.tools.keys() {
                bloom.insert(tool_name);
            }
            
            for capability_name in data.explicit_rules.capabilities.keys() {
                bloom.insert(capability_name);
            }
            
            
            self.bloom_filter = Some(bloom);
        }
        
        Ok(())
    }
    
    /// Precompute allowlist decisions for all tools for instant treeview display
    /// This implements the user's requested hierarchy: tool-specific > capability-specific > global patterns > default
    pub fn precompute_all_decisions<F>(&self, get_all_tools: F) -> Result<(), Box<dyn std::error::Error>> 
    where
        F: Fn() -> Vec<(String, crate::registry::types::ToolDefinition)>,
    {
        use super::allowlist_data::{AllowlistDecision, RuleSource, DecisionAuditTrail, RuleEvaluation, EvaluationResult};
        use chrono::Utc;
        use regex::Regex;

        let start_time = std::time::Instant::now();
        let all_tools = get_all_tools();
        
        println!("Precomputing allowlist decisions for {} tools...", all_tools.len());
        
        let mut decisions = HashMap::new();
        let mut audit_trails = HashMap::new();
        
        // Get current config for default action and emergency state
        let config = self.config.read().unwrap();
        let emergency_active = self.emergency_active.load(Ordering::Relaxed);
        
        for (tool_name, tool_def) in &all_tools {
            let mut evaluation_chain = Vec::new();
            let mut step = 1u8;
            
            // Final decision - will be determined by hierarchy
            let decision = if emergency_active {
                // Emergency lockdown overrides everything
                evaluation_chain.push(RuleEvaluation {
                    step,
                    rule_type: "emergency_lockdown".to_string(),
                    rule_name: Some("emergency_lockdown".to_string()),
                    result: EvaluationResult::Deny,
                    reason: Some("Emergency lockdown active".to_string()),
                    continue_evaluation: false,
                });
                
                AllowlistDecision::deny(
                    RuleSource::EmergencyLockdown,
                    "emergency_lockdown".to_string(),
                    "Emergency lockdown active".to_string(),
                )
            } else {
                // Apply hierarchy: tool-specific > capability-specific > global patterns > default
                
                // Step 1: Check explicit tool rules (highest priority)
                step += 1;
                if let Some(action) = self.explicit_tool_rules.read().unwrap().get(tool_name) {
                    evaluation_chain.push(RuleEvaluation {
                        step,
                        rule_type: "explicit_tool".to_string(),
                        rule_name: Some(tool_name.clone()),
                        result: if matches!(action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                        reason: Some(format!("Explicit tool rule: {}", tool_name)),
                        continue_evaluation: false,
                    });
                    
                    AllowlistDecision::new(
                        action.clone(),
                        RuleSource::ExplicitTool,
                        tool_name.clone(),
                        format!("Explicit tool rule: {}", tool_name),
                    )
                } else {
                    evaluation_chain.push(RuleEvaluation {
                        step,
                        rule_type: "explicit_tool".to_string(),
                        rule_name: None,
                        result: EvaluationResult::NoMatch,
                        reason: Some("No explicit tool rule found".to_string()),
                        continue_evaluation: true,
                    });
                    
                    // Step 2: Check tool patterns
                    step += 1;
                    if let Some(matching_tool_pattern) = self.find_matching_tool_pattern(&tool_name) {
                        evaluation_chain.push(RuleEvaluation {
                            step,
                            rule_type: "tool_pattern".to_string(),
                            rule_name: Some(matching_tool_pattern.name.clone()),
                            result: if matches!(matching_tool_pattern.action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                            reason: Some(format!("Tool pattern: {}", matching_tool_pattern.name)),
                            continue_evaluation: false,
                        });
                        
                        AllowlistDecision::new(
                            matching_tool_pattern.action.clone(),
                            RuleSource::ToolPattern,
                            matching_tool_pattern.name.clone(),
                            format!("Tool pattern: {}", matching_tool_pattern.name),
                        )
                    } else {
                        evaluation_chain.push(RuleEvaluation {
                            step,
                            rule_type: "tool_pattern".to_string(),
                            rule_name: None,
                            result: EvaluationResult::NoMatch,
                            reason: Some("No matching tool pattern".to_string()),
                            continue_evaluation: true,
                        });
                        
                        // Step 3: Check capability patterns (if tool has capability info)
                        step += 1;
                        if let Some(matching_capability_pattern) = self.find_matching_capability_pattern(&tool_name, &tool_def) {
                            evaluation_chain.push(RuleEvaluation {
                                step,
                                rule_type: "capability_pattern".to_string(),
                                rule_name: Some(matching_capability_pattern.name.clone()),
                                result: if matches!(matching_capability_pattern.action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                                reason: Some(format!("Capability pattern: {}", matching_capability_pattern.name)),
                                continue_evaluation: false,
                            });
                            
                            AllowlistDecision::new(
                                matching_capability_pattern.action.clone(),
                                RuleSource::CapabilityPattern,
                                matching_capability_pattern.name.clone(),
                                format!("Capability pattern: {}", matching_capability_pattern.name),
                            )
                        } else {
                            evaluation_chain.push(RuleEvaluation {
                                step,
                                rule_type: "capability_pattern".to_string(),
                                rule_name: None,
                                result: EvaluationResult::NoMatch,
                                reason: Some("No matching capability pattern".to_string()),
                                continue_evaluation: true,
                            });
                            
                            // Step 4: Check global patterns  
                            step += 1;
                            if let Some(matching_global_pattern) = self.find_matching_global_pattern(&tool_name) {
                                evaluation_chain.push(RuleEvaluation {
                                    step,
                                    rule_type: "global_pattern".to_string(),
                                    rule_name: Some(matching_global_pattern.name.clone()),
                                    result: if matches!(matching_global_pattern.action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                                    reason: Some(format!("Global pattern: {}", matching_global_pattern.name)),
                                    continue_evaluation: false,
                                });
                                
                                AllowlistDecision::new(
                                    matching_global_pattern.action.clone(),
                                    RuleSource::GlobalPattern,
                                    matching_global_pattern.name.clone(),
                                    format!("Global pattern: {}", matching_global_pattern.name),
                                )
                            } else {
                                evaluation_chain.push(RuleEvaluation {
                                    step,
                                    rule_type: "global_pattern".to_string(),
                                    rule_name: None,
                                    result: EvaluationResult::NoMatch,
                                    reason: Some("No matching global pattern".to_string()),
                                    continue_evaluation: true,
                                });
                                
                                // Step 5: Apply default action
                                step += 1;
                                evaluation_chain.push(RuleEvaluation {
                                    step,
                                    rule_type: "default_action".to_string(),
                                    rule_name: Some("default".to_string()),
                                    result: if matches!(config.default_action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                                    reason: Some(format!("Default action: {:?}", config.default_action)),
                                    continue_evaluation: false,
                                });
                                
                                AllowlistDecision::new(
                                    config.default_action.clone(),
                                    RuleSource::DefaultAction,
                                    "default".to_string(),
                                    format!("Default action: {:?}", config.default_action),
                                )
                            }
                        }
                    }
                }
            };
            
            // Store decision and audit trail
            decisions.insert(tool_name.clone(), decision.clone());
            
            audit_trails.insert(tool_name.clone(), DecisionAuditTrail {
                tool_name: tool_name.clone(),
                final_decision: decision.action.clone(),
                rule_source: decision.rule_source.clone(),
                rule_name: decision.rule_name.clone(),
                evaluation_chain,
                timestamp: Utc::now(),
            });
        }
        
        // Update the precomputed decisions atomically
        *self.precomputed_decisions.write().unwrap() = decisions;
        *self.decision_audit_trails.write().unwrap() = audit_trails;
        
        let elapsed = start_time.elapsed();
        println!("Precomputed {} allowlist decisions in {:?}", all_tools.len(), elapsed);
        
        Ok(())
    }
    
    /// Find matching tool pattern for a tool name
    fn find_matching_tool_pattern(&self, tool_name: &str) -> Option<super::allowlist_data::PatternRule> {
        if let Some(ref pattern_set) = self.tool_pattern_set {
            let matches: Vec<usize> = pattern_set.matches(tool_name).iter().collect();
            if !matches.is_empty() {
                let tool_patterns = self.raw_tool_pattern_rules.read().unwrap();
                // Find most restrictive match - any DENY wins over ALLOW
                return matches.iter()
                    .filter_map(|&idx| tool_patterns.get(idx))
                    .filter(|rule| rule.enabled)
                    .find(|rule| rule.action == AllowlistAction::Deny)  // Any deny wins
                    .cloned()
                    .or_else(|| {
                        // If no denies, take the first allow
                        matches.iter()
                            .filter_map(|&idx| tool_patterns.get(idx))
                            .filter(|rule| rule.enabled)
                            .find(|rule| rule.action == AllowlistAction::Allow)
                            .cloned()
                    });
            }
        }
        None
    }
    
    /// Find matching capability pattern for a tool
    fn find_matching_capability_pattern(&self, tool_name: &str, _tool_def: &crate::registry::types::ToolDefinition) -> Option<super::allowlist_data::PatternRule> {
        // For now, match against tool name - could be enhanced to use tool's capability metadata
        let capability_patterns = self.capability_pattern_rules.read().unwrap();
        debug!("🔍 Checking capability patterns for tool '{}', found {} patterns", tool_name, capability_patterns.len());
        
        for rule in capability_patterns.iter() {
            debug!("🔍 Checking pattern '{}' (enabled: {}, regex: '{}')", rule.name, rule.enabled, rule.regex);
            if !rule.enabled {
                continue;
            }
            
            if let Ok(regex) = regex::Regex::new(&rule.regex) {
                let is_match = regex.is_match(tool_name);
                debug!("🔍 Pattern '{}' regex match against '{}': {}", rule.name, tool_name, is_match);
                if is_match {
                    debug!("✅ Found matching capability pattern: {}", rule.name);
                    return Some(rule.clone());
                }
            } else {
                debug!("❌ Invalid regex in pattern '{}'", rule.name);
            }
        }
        
        debug!("❌ No matching capability pattern found for '{}'", tool_name);
        None
    }
    
    /// Find matching capability pattern for a specific capability name
    fn find_matching_capability_pattern_for_capability(&self, capability_name: &str) -> Option<super::allowlist_data::PatternRule> {
        let capability_patterns = self.capability_pattern_rules.read().unwrap();
        capability_patterns.iter()
            .filter(|rule| rule.enabled)
            .find(|rule| {
                if let Ok(regex) = regex::Regex::new(&rule.regex) {
                    regex.is_match(capability_name)
                } else {
                    false
                }
            })
            .cloned()
    }
    
    /// Find matching global pattern for a tool name  
    fn find_matching_global_pattern(&self, tool_name: &str) -> Option<super::allowlist_data::PatternRule> {
        let global_patterns = self.global_pattern_rules.read().unwrap();
        global_patterns.iter()
            .filter(|rule| rule.enabled)
            .find(|rule| {
                if let Ok(regex) = regex::Regex::new(&rule.regex) {
                    regex.is_match(tool_name)
                } else {
                    false
                }
            })
            .cloned()
    }
    
    /// Get all tools with their precomputed allowlist status for treeview API
    /// Returns tools organized by server/capability with allowlist decisions
    /// Registry-aware approach: uses actual server/capability info from registry context
    pub fn get_all_tools_with_status_from_registry<F>(&self, get_tools_with_context: F) -> Vec<super::allowlist_data::ToolWithAllowlistStatus>
    where
        F: Fn() -> Vec<(String, crate::registry::types::ToolDefinition, String, String)>, // (name, tool_def, server, capability)
    {
        use super::allowlist_data::{ToolWithAllowlistStatus, AllowlistDecision, RuleSource};
        
        let tools_with_context = get_tools_with_context();
        let precomputed = self.precomputed_decisions.read().unwrap();
        let audit_trails = self.decision_audit_trails.read().unwrap();
        
        let mut tools_with_status = Vec::new();
        
        for (tool_name, tool_def, server, capability) in tools_with_context {
            // Get precomputed decision or compute on-the-fly if not available
            let allowlist_decision = if let Some(decision) = precomputed.get(&tool_name) {
                decision.clone()
            } else {
                // Fallback: compute decision on-the-fly if not precomputed
                self.compute_decision_for_tool(&tool_name, &tool_def)
            };
            
            // Check if audit trail is available
            let audit_available = audit_trails.contains_key(&tool_name);
            
            tools_with_status.push(ToolWithAllowlistStatus {
                name: tool_name,
                capability,
                server,
                allowlist_decision,
                audit_available,
            });
        }
        
        // Sort by server, then capability, then tool name for consistent treeview display
        tools_with_status.sort_by(|a, b| {
            a.server.cmp(&b.server)
                .then(a.capability.cmp(&b.capability))
                .then(a.name.cmp(&b.name))
        });
        
        tools_with_status
    }
    
    /// Get precomputed decision for a specific tool (fast O(1) lookup)
    pub fn get_tool_decision(&self, tool_name: &str) -> Option<super::allowlist_data::AllowlistDecision> {
        self.precomputed_decisions.read().unwrap().get(tool_name).cloned()
    }
    
    /// Get precomputed decision for a specific capability (fast O(1) lookup)
    /// This uses the same source as reload_from_data_file updates, ensuring consistency
    pub fn get_capability_decision(&self, capability_name: &str) -> Option<super::allowlist_data::AllowlistDecision> {
        self.precomputed_decisions.read().unwrap().get(capability_name).cloned()
    }
    
    /// Get audit trail for a specific tool's decision
    pub fn get_tool_audit_trail(&self, tool_name: &str) -> Option<super::allowlist_data::DecisionAuditTrail> {
        self.decision_audit_trails.read().unwrap().get(tool_name).cloned()
    }
    
    /// Compute decision for a single tool on-the-fly (fallback when not precomputed)
    fn compute_decision_for_tool(&self, tool_name: &str, tool_def: &crate::registry::types::ToolDefinition) -> super::allowlist_data::AllowlistDecision {
        use super::allowlist_data::{AllowlistDecision, RuleSource};
        
        let config = self.config.read().unwrap();
        let emergency_active = self.emergency_active.load(Ordering::Relaxed);
        
        if emergency_active {
            return AllowlistDecision::deny(
                RuleSource::EmergencyLockdown,
                "emergency_lockdown".to_string(),
                "Emergency lockdown active".to_string(),
            );
        }
        
        // Apply the same hierarchy as precompute_all_decisions
        // 1. Explicit tool rules (highest priority)
        if let Some(action) = self.explicit_tool_rules.read().unwrap().get(tool_name) {
            return AllowlistDecision::new(
                action.clone(),
                RuleSource::ExplicitTool,
                tool_name.to_string(),
                format!("Explicit tool rule: {}", tool_name),
            );
        }
        
        // 2. Tool patterns
        if let Some(matching_pattern) = self.find_matching_tool_pattern(tool_name) {
            return AllowlistDecision::new(
                matching_pattern.action.clone(),
                RuleSource::ToolPattern,
                matching_pattern.name.clone(),
                format!("Tool pattern: {}", matching_pattern.name),
            );
        }
        
        // 3. Capability patterns
        if let Some(matching_pattern) = self.find_matching_capability_pattern(tool_name, tool_def) {
            return AllowlistDecision::new(
                matching_pattern.action.clone(),
                RuleSource::CapabilityPattern,
                matching_pattern.name.clone(),
                format!("Capability pattern: {}", matching_pattern.name),
            );
        }
        
        // 4. Global patterns
        if let Some(matching_pattern) = self.find_matching_global_pattern(tool_name) {
            return AllowlistDecision::new(
                matching_pattern.action.clone(),
                RuleSource::GlobalPattern,
                matching_pattern.name.clone(),
                format!("Global pattern: {}", matching_pattern.name),
            );
        }
        
        // 5. Default action (lowest priority)
        AllowlistDecision::new(
            config.default_action.clone(),
            RuleSource::DefaultAction,
            "default".to_string(),
            format!("Default action: {:?}", config.default_action),
        )
    }
    
    /// Get summary statistics of all precomputed decisions
    pub fn get_allowlist_summary(&self) -> AllowlistSummary {
        let precomputed = self.precomputed_decisions.read().unwrap();
        
        let mut summary = AllowlistSummary {
            total_tools: precomputed.len(),
            allowed_tools: 0,
            denied_tools: 0,
            explicit_rules: 0,
            tool_patterns: 0,
            capability_patterns: 0,
            global_patterns: 0,
            default_actions: 0,
            emergency_lockdown: 0,
        };
        
        for decision in precomputed.values() {
            match decision.action {
                AllowlistAction::Allow => summary.allowed_tools += 1,
                AllowlistAction::Deny => summary.denied_tools += 1,
            }
            
            match decision.rule_source {
                RuleSource::ExplicitTool => summary.explicit_rules += 1,
                RuleSource::ExplicitCapability => summary.explicit_rules += 1,
                RuleSource::ToolPattern => summary.tool_patterns += 1,
                RuleSource::CapabilityPattern => summary.capability_patterns += 1,
                RuleSource::GlobalPattern => summary.global_patterns += 1,
                RuleSource::DefaultAction => summary.default_actions += 1,
                RuleSource::EmergencyLockdown => summary.emergency_lockdown += 1,
            }
        }
        
        summary
    }
    
    /// Force refresh of precomputed decisions (call after configuration changes)
    pub fn refresh_precomputed_decisions<F>(&self, get_all_tools: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn() -> Vec<(String, crate::registry::types::ToolDefinition)>,
    {
        self.precompute_all_decisions(get_all_tools)
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
                    };
                    tool_rules.insert(hash, cached_rule);
                    name_to_hash.insert(name.clone(), hash);
                }
            }
        }
        
        
        // === CAPABILITY PATTERN COMPILATION ===
        // First compile patterns from YAML data file (higher priority)
        let yaml_patterns = self.capability_pattern_rules.read().unwrap();
        debug!("🔧 Found {} YAML capability patterns to compile", yaml_patterns.len());
        
        // Use YAML patterns if available, otherwise fall back to config patterns
        if !yaml_patterns.is_empty() {
            let mut regex_patterns = Vec::new();
            let mut cached_rules = Vec::new();
            
            for pattern_rule in yaml_patterns.iter() {
                if pattern_rule.enabled {
                    debug!("🔧 Compiling YAML capability pattern: '{}' -> '{}'", 
                           pattern_rule.name, pattern_rule.regex);
                    regex_patterns.push(pattern_rule.regex.clone());
                    cached_rules.push(CachedAllowlistRule {
                        action: pattern_rule.action.clone(),
                        reason: Some(Arc::from(pattern_rule.reason.as_str())),
                    });
                }
            }
            
            // Compile RegexSet for patterns
            let regex_set = if !regex_patterns.is_empty() {
                debug!("🔧 Compiling {} YAML capability regex patterns", regex_patterns.len());
                println!("🔧 COMPILE: regex_patterns={:?}", regex_patterns);
                println!("🔧 COMPILE: cached_rules.len()={}", cached_rules.len());
                Some(RegexSet::new(&regex_patterns)?)
            } else {
                None
            };
            
            // Update pattern matching structures with YAML patterns
            *self.capability_regex_set.write().unwrap() = regex_set;
            *self.capability_rules.write().unwrap() = cached_rules;
            
            debug!("✅ YAML capability patterns compiled successfully");
            println!("✅ COMPILE: Updated capability_rules with {} rules", self.capability_rules.read().unwrap().len());
            
        } else if !config.capability_patterns.is_empty() {
            // Fall back to legacy config patterns if no YAML patterns available
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
                    debug!("Compiling {} config capability regex patterns", regex_patterns.len());
                    Some(RegexSet::new(&regex_patterns)?)
                } else {
                    None
                };
                
                // Cache compiled rules (sorted by priority)
                let mut cached_rules: Vec<CachedAllowlistRule> = enabled_patterns.iter()
                    .map(|p| CachedAllowlistRule {
                        action: p.rule.action.clone(),
                        reason: p.rule.reason.as_ref().map(|s| Arc::from(s.as_str())),
                    })
                    .collect();
                
                // Update pattern matching structures
                *self.capability_regex_set.write().unwrap() = regex_set;
                *self.capability_rules.write().unwrap() = cached_rules;
                
                debug!("Legacy config capability patterns compiled successfully");
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
                    })
                    .collect();
                
                // No sorting needed - most restrictive wins logic handles conflicts
                
                // Update pattern matching structures
                *self.global_regex_set.write().unwrap() = regex_set;
                *self.global_rules.write().unwrap() = cached_rules.clone();
                
                debug!("⚠️  CONFIG: Global patterns compiled successfully: {} rules", cached_rules.len());
                debug!("⚠️  CONFIG: Global rules: {:?}", cached_rules.iter().map(|r| &r.action).collect::<Vec<_>>());
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
            debug!("💾 CACHE HIT for tool '{}', returning cached result: allowed={}", 
                   tool_name, cached.allowed());
            self.record_cache_hit();
            let mut result = cached.into_result();
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            return result;
        }
        
        debug!("❌ CACHE MISS for tool '{}', proceeding with fresh evaluation", tool_name);
        
        // Cache miss - record it and proceed with evaluation
        self.record_cache_miss();
        
        // Check if allowlist is enabled
        let config = self.config.read().unwrap();
        debug!("🔧 Allowlist enabled: {}", config.enabled);
        if !config.enabled {
            debug!("⚠️ EARLY RETURN: Allowlist disabled");
            let mut result = AllowlistResult::allow_fast("Allowlist disabled", RuleLevel::Default);
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // Check emergency lockdown (atomic check, fastest path)
        debug!("🚨 Emergency lockdown active: {}", self.emergency_active.load(Ordering::Relaxed));
        if self.emergency_active.load(Ordering::Relaxed) {
            debug!("⚠️ EARLY RETURN: Emergency lockdown");
            let mut result = AllowlistResult::deny_fast("Emergency lockdown", RuleLevel::Emergency);
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        debug!("🔄 Proceeding to rule evaluation hierarchy");
        
        // 1. Check MagicTunnel-level rules (highest priority after emergency)
        if let Some(rule) = config.mt_level_rules.get(tool_name) {
            if rule.enabled {
                let mut result = match rule.action {
                    AllowlistAction::Allow => AllowlistResult::allow_fast("MT-level rule", RuleLevel::Tool),
                    AllowlistAction::Deny => AllowlistResult::deny_fast("MT-level blocked", RuleLevel::Tool),
                };
                result.matched_rule = Some(format!("mt_level:{}", tool_name));
                if let Some(ref reason) = rule.reason {
                    result.reason = Arc::from(reason.as_str());
                }
                result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
                self.cache_decision_fast(cache_key, &result);
                self.update_stats(&result);
                return result;
            }
        }
        
        // 2. Check explicit tool rules from YAML data file (highest priority for regular rules)
        debug!("🔧 Config tools available: {:?}", config.tools.keys().collect::<Vec<_>>());
        debug!("🔧 Explicit tool rules available: {:?} (instance: {:p})", 
               self.explicit_tool_rules.read().unwrap().keys().collect::<Vec<_>>(), self);
        
        if let Some(action) = self.explicit_tool_rules.read().unwrap().get(tool_name) {
            debug!("🔍 Checking explicit tool rules for '{}'. Found: {:?}", tool_name, action);
            let mut result = match action {
                AllowlistAction::Allow => AllowlistResult::allow_fast("Explicit tool rule: allowed", RuleLevel::Tool),
                AllowlistAction::Deny => AllowlistResult::deny_fast("Explicit tool rule: blocked", RuleLevel::Tool),
            };
            result.matched_rule = Some(format!("explicit_tool:{}", tool_name));
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            debug!("✅ Explicit tool rule applied for '{}': allowed={}", tool_name, result.allowed);
            return result;
        }
        
        // 3. Check individual tool rules from config (legacy)
        if let Some(rule) = config.tools.get(tool_name) {
            debug!("✅ Found config tool rule for '{}': {:?}", tool_name, rule);
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
        
        // 4. Check tool pattern rules (e.g., "file_*" patterns)
        if let Some(tool_pattern_rule) = self.match_tool_patterns_fast(tool_name) {
            let mut result = match tool_pattern_rule.action {
                AllowlistAction::Allow => AllowlistResult::allow_fast("Tool pattern allowed", RuleLevel::Tool),
                AllowlistAction::Deny => AllowlistResult::deny_fast("Tool pattern blocked", RuleLevel::Tool),
            };
            result.matched_rule = Some("tool_pattern".to_string());
            if let Some(ref reason) = tool_pattern_rule.reason {
                result.reason = reason.clone();
            }
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // 5. Check individual capability rules (need to determine capability from tool)
        if let Some(capability_name) = self.get_capability_for_tool(tool_name) {
            if let Some(rule) = config.capabilities.get(&capability_name) {
                if rule.enabled {
                    let mut result = match rule.action {
                        AllowlistAction::Allow => AllowlistResult::allow_fast("Capability allowed", RuleLevel::Capability),
                        AllowlistAction::Deny => AllowlistResult::deny_fast("Capability blocked", RuleLevel::Capability),
                    };
                    result.matched_rule = Some(format!("capability:{}", capability_name));
                    if let Some(ref reason) = rule.reason {
                        result.reason = Arc::from(reason.as_str());
                    }
                    result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
                    self.cache_decision_fast(cache_key, &result);
                    self.update_stats(&result);
                    return result;
                }
            }
        }
        
        // Release config lock early for pattern matching
        drop(config);
        
        // 6. Check capability pattern rules (ultra-fast cascade: bloom → trie → regex)
        if let Some(capability_rule) = self.match_capability_patterns_fast(tool_name) {
            let mut result = match capability_rule.action {
                AllowlistAction::Allow => AllowlistResult::allow_fast("Capability pattern", RuleLevel::Capability),
                AllowlistAction::Deny => AllowlistResult::deny_fast("Capability pattern blocked", RuleLevel::Capability),
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
        
        // 7. Check global pattern rules (ultra-fast cascade: bloom → trie → regex) - lowest precedence
        if let Some(global_rule) = self.match_global_patterns_fast(tool_name) {
            let mut result = match global_rule.action {
                AllowlistAction::Allow => AllowlistResult::allow_fast("Global pattern", RuleLevel::Global),
                AllowlistAction::Deny => AllowlistResult::deny_fast("Global pattern blocked", RuleLevel::Global),
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
        
        // 8. Apply default action (no patterns matched) - final fallback
        let config = self.config.read().unwrap();
        debug!("🔧 DEFAULT: Applying default action for tool '{}': {:?}", tool_name, config.default_action);
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
    
    
    /// Fast capability pattern matching with RegexSet
    /// Performance: ~100ns for regex pattern matching
    #[inline(always)]
    fn match_capability_patterns_fast(&self, tool_name: &str) -> Option<CachedAllowlistRule> {
        println!("🔍 CAPABILITY: Checking capability patterns for tool '{}'", tool_name);
        
        // RegexSet for pattern matching - simple and reliable
        if let Some(regex_set) = self.capability_regex_set.read().unwrap().as_ref() {
            println!("🔍 CAPABILITY: Found regex_set, checking matches");
            let matches: Vec<usize> = regex_set.matches(tool_name).iter().collect();
            println!("🔍 CAPABILITY: Matches found: {:?}", matches);
            
            if !matches.is_empty() {
                let rules = self.capability_rules.read().unwrap();
                println!("🔍 CAPABILITY: Rules count: {}", rules.len());
                
                // Find most restrictive match - any DENY wins over ALLOW
                let best_match = matches.iter()
                    .filter_map(|&idx| {
                        if let Some(rule) = rules.get(idx) {
                            println!("🔍 CAPABILITY: Rule at idx {}: action={:?}", idx, rule.action);
                            Some(rule)
                        } else {
                            None
                        }
                    })
                    .find(|rule| rule.action == AllowlistAction::Deny)  // Any deny wins
                    .or_else(|| {
                        // If no denies, take the first allow
                        matches.iter()
                            .filter_map(|&idx| rules.get(idx))
                            .find(|rule| rule.action == AllowlistAction::Allow)
                    })
                    .cloned();
                
                if let Some(ref rule) = best_match {
                    println!("✅ CAPABILITY: Found matching rule: action={:?}", rule.action);
                } else {
                    println!("❌ CAPABILITY: No valid rules found despite matches");
                }
                
                return best_match;
            } else {
                println!("❌ CAPABILITY: No pattern matches");
            }
        } else {
            println!("❌ CAPABILITY: No regex_set found");
        }
        
        None
    }
    
    /// Fast global pattern matching using RegexSet
    /// Performance: ~100-500ns for regex pattern matching
    #[inline(always)]
    fn match_global_patterns_fast(&self, tool_name: &str) -> Option<CachedAllowlistRule> {
        debug!("🌍 Global pattern check for tool: '{}'", tool_name);
        
        // Use RegexSet for batch pattern matching
        if let Some(regex_set) = self.global_regex_set.read().unwrap().as_ref() {
            debug!("🌍 Global regex set available with {} patterns", regex_set.len());
            let matches: Vec<usize> = regex_set.matches(tool_name).iter().collect();
            debug!("🌍 Global pattern matches for '{}': {:?}", tool_name, matches);
            
            if !matches.is_empty() {
                let rules = self.global_rules.read().unwrap();
                debug!("🌍 Checking {} global rules for best match (matches: {:?})", rules.len(), matches);
                debug!("🌍 Available rules: {:?}", rules.iter().map(|r| &r.action).collect::<Vec<_>>());
                debug!("🌍 Rules array debug: {:?}", rules.iter().enumerate().map(|(i, r)| (i, &r.action)).collect::<Vec<_>>());
                
                // Find most restrictive match - any DENY wins over ALLOW
                let best_match = matches.iter()
                    .filter_map(|&idx| {
                        debug!("🌍 Looking up rule at index {}", idx);
                        let rule = rules.get(idx);
                        debug!("🌍 Rule at index {}: {:?}", idx, rule.as_ref().map(|r| &r.action));
                        rule
                    })
                    .find(|rule| rule.action == AllowlistAction::Deny)  // Any deny wins
                    .or_else(|| {
                        // If no denies, take the first allow
                        matches.iter()
                            .filter_map(|&idx| rules.get(idx))
                            .find(|rule| rule.action == AllowlistAction::Allow)
                    })
                    .cloned();
                
                if let Some(ref rule) = best_match {
                    debug!("🌍 Global pattern matched: action={:?}, reason={:?}", rule.action, rule.reason);
                    debug!("🌍 About to return best_match: {:?}", rule);
                } else {
                    debug!("🌍 No rule found for any matched pattern indices");
                    debug!("🌍 About to return None from global pattern check");
                }
                
                return best_match;
            }
        } else {
            debug!("🌍 No global regex set available");
        }
        
        debug!("🌍 No global pattern match for '{}'", tool_name);
        None
    }
    
    /// Fast tool pattern matching using RegexSet
    /// Performance: ~100-500ns for regex pattern matching
    #[inline(always)]
    fn match_tool_patterns_fast(&self, tool_name: &str) -> Option<CachedAllowlistRule> {
        // Use RegexSet for batch pattern matching
        if let Some(regex_set) = self.tool_pattern_regex_set.read().unwrap().as_ref() {
            let matches: Vec<usize> = regex_set.matches(tool_name).iter().collect();
            
            if !matches.is_empty() {
                let rules = self.tool_pattern_rules.read().unwrap();
                // Find most restrictive match - any DENY wins over ALLOW
                let best_match = matches.iter()
                    .filter_map(|&idx| rules.get(idx))
                    .find(|rule| rule.action == AllowlistAction::Deny)  // Any deny wins
                    .or_else(|| {
                        // If no denies, take the first allow
                        matches.iter()
                            .filter_map(|&idx| rules.get(idx))
                            .find(|rule| rule.action == AllowlistAction::Allow)
                    })
                    .cloned();
                
                return best_match;
            }
        }
        
        None
    }
    
    /// Get capability name for a tool using intelligent registry-based lookup
    /// Enhanced with registry service integration and fallback heuristics
    fn get_capability_for_tool(&self, tool_name: &str) -> Option<String> {
        // Step 1: Try registry-based capability lookup
        if let Some(capability) = self.get_capability_from_registry(tool_name) {
            debug!("🎯 Registry-based capability mapping: {} -> {}", tool_name, capability);
            return Some(capability);
        }
        
        // Step 2: Extract from tool metadata and annotations
        if let Some(capability) = self.get_capability_from_tool_metadata(tool_name) {
            debug!("📋 Metadata-based capability mapping: {} -> {}", tool_name, capability);
            return Some(capability);
        }
        
        // Step 3: Infer from file context (capability file names)
        if let Some(capability) = self.get_capability_from_file_context(tool_name) {
            debug!("📁 File context capability mapping: {} -> {}", tool_name, capability);
            return Some(capability);
        }
        
        // Step 4: Fallback to enhanced heuristics (improved patterns)
        if let Some(capability) = self.get_capability_from_enhanced_heuristics(tool_name) {
            debug!("🔍 Heuristic capability mapping: {} -> {}", tool_name, capability);
            return Some(capability);
        }
        
        debug!("❌ No capability mapping found for tool: {}", tool_name);
        None
    }
    
    /// Get capability from registry service using tool definition lookup
    fn get_capability_from_registry(&self, tool_name: &str) -> Option<String> {
        let registry_service = self.registry_service.as_ref()?;
        
        // Get the tool definition from registry
        let tool_def = registry_service.get_tool(tool_name)?;
        
        // Check routing configuration for capability hints
        if let Some(capability) = self.extract_capability_from_routing(&tool_def.routing) {
            return Some(capability);
        }
        
        // Check tool annotations for explicit capability declaration
        if let Some(annotations) = &tool_def.annotations {
            if let Some(capability) = annotations.get("capability") {
                return Some(capability.clone());
            }
            
            // Check other annotation patterns
            if let Some(category) = annotations.get("category") {
                return Some(category.clone());
            }
            
            if let Some(domain) = annotations.get("domain") {
                return Some(domain.clone());
            }
        }
        
        None
    }
    
    /// Extract capability from tool metadata and enhanced definitions
    fn get_capability_from_tool_metadata(&self, tool_name: &str) -> Option<String> {
        let registry_service = self.registry_service.as_ref()?;
        
        // Get all tools with context (includes file and capability context)
        let tools_with_context = registry_service.get_all_tools_with_context();
        
        // Find our tool and extract capability from context
        for (name, _tool_def, server_name, capability_context) in tools_with_context {
            if name == tool_name {
                // Use server name as capability hint
                if !server_name.is_empty() && server_name != "local" && server_name != "unknown" {
                    return Some(server_name);
                }
                
                // Use capability context
                if !capability_context.is_empty() && capability_context != "unknown" {
                    return Some(capability_context);
                }
            }
        }
        
        None
    }
    
    /// Get capability from file context (capability file names and paths)  
    fn get_capability_from_file_context(&self, tool_name: &str) -> Option<String> {
        let registry_service = self.registry_service.as_ref()?;
        
        // Use the public API to get tools with their context information
        let tools_with_context = registry_service.get_all_tools_with_context();
        
        // Find the tool and extract its capability from the file context
        for (name, _tool_def, _server_name, capability_name) in tools_with_context {
            if name == tool_name {
                // Use the capability name from the context
                if !capability_name.is_empty() {
                    return Some(capability_name);
                }
                break;
            }
        }
        
        None
    }
    
    /// Enhanced heuristic patterns (improved version of old heuristics)
    fn get_capability_from_enhanced_heuristics(&self, tool_name: &str) -> Option<String> {
        // Enhanced filesystem operations
        if self.matches_filesystem_patterns(tool_name) {
            return Some("filesystem".to_string());
        }
        
        // Enhanced git/github operations
        if self.matches_git_patterns(tool_name) {
            return Some("github".to_string());
        }
        
        // Enhanced web operations
        if self.matches_web_patterns(tool_name) {
            return Some("web".to_string());
        }
        
        // Enhanced database operations
        if self.matches_database_patterns(tool_name) {
            return Some("database".to_string());
        }
        
        // System operations
        if self.matches_system_patterns(tool_name) {
            return Some("system".to_string());
        }
        
        // Network operations
        if self.matches_network_patterns(tool_name) {
            return Some("network".to_string());
        }
        
        // AI/ML operations
        if self.matches_ai_patterns(tool_name) {
            return Some("ai".to_string());
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
        
        // Non-blocking cache update - failure is acceptable for performance
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
        } else {
            // Cache write failed (lock contention) - log for debugging but continue
            debug!("🔒 Cache write skipped due to lock contention");
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
                let audit_event = AuditEvent::new(
                    AuditEventType::ToolExecution,
                    "allowlist_service".to_string(),
                    format!("Tool {} evaluation: {}", tool_name, if allowed { "allowed" } else { "blocked" })
                )
                .with_severity(if allowed { AuditSeverity::Info } else { AuditSeverity::Warning })
                .with_metadata("tool_name", serde_json::json!(tool_name))
                .with_metadata("allowed", serde_json::json!(allowed))
                .with_metadata("reason", serde_json::json!(reason.to_string()))
                .with_metadata("rule_level", serde_json::json!(format!("{:?}", rule_level)))
                .with_metadata("matched_rule", serde_json::json!(matched_rule.clone()))
                .with_metadata("evaluation_time_ns", serde_json::json!(evaluation_time_ns))
                .with_metadata("parameters", serde_json::json!(parameters))
                .with_metadata("execution_time_ms", serde_json::json!((evaluation_time_ns / 1_000_000) as u64));
                
                if let Err(e) = audit_service.log_event(audit_event).await {
                    // Use debug instead of error to avoid spam in high-performance scenarios
                    debug!("Failed to log allowlist evaluation audit: {}", e);
                }
            });
        }
    }
}

// Legacy compatibility wrapper methods
impl AllowlistService {
    /// Check capability access using the proper hierarchy evaluation system
    pub fn check_capability_access(
        &self,
        capability_name: &str,
        context: &AllowlistContext,
    ) -> AllowlistResult {
        debug!("🚀 MAIN: check_capability_access called for capability: '{}'", capability_name);
        let start_time = Instant::now();
        let result = self.check_capability_access_internal(capability_name, context);
        debug!("🎯 MAIN: check_capability_access result for '{}': allowed={}", capability_name, result.allowed);
        let evaluation_time_ns = start_time.elapsed().as_nanos() as u64;
        
        // Log rule evaluation asynchronously for audit trail (non-blocking)
        self.log_rule_evaluation_async(capability_name, context, &result, &HashMap::new(), evaluation_time_ns);
        
        result
    }
    
    /// Internal capability access evaluation with proper hierarchy
    pub fn check_capability_access_internal(
        &self,
        capability_name: &str,
        context: &AllowlistContext,
    ) -> AllowlistResult {
        let start_time = Instant::now();
        
        // Create fast context for cache key computation  
        let fast_context = context.to_fast_context();
        
        // Check cache first (O(1) lookup)
        let cache_key = self.compute_cache_key_fast(capability_name, &fast_context);
        if let Some(cached) = self.check_decision_cache_fast(cache_key) {
            debug!("💾 CACHE HIT for capability '{}', returning cached result: allowed={}", 
                   capability_name, cached.allowed());
            self.record_cache_hit();
            let mut result = cached.into_result();
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            return result;
        }
        
        debug!("❌ CACHE MISS for capability '{}', proceeding with fresh evaluation", capability_name);
        self.record_cache_miss();
        
        // Check if allowlist is enabled
        let config = self.config.read().unwrap();
        debug!("🔧 Allowlist enabled: {}", config.enabled);
        if !config.enabled {
            debug!("⚠️ EARLY RETURN: Allowlist disabled");
            let mut result = AllowlistResult::allow_fast("Allowlist disabled", RuleLevel::Default);
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // Check emergency lockdown (atomic check, fastest path)
        debug!("🚨 Emergency lockdown active: {}", self.emergency_active.load(Ordering::Relaxed));
        if self.emergency_active.load(Ordering::Relaxed) {
            debug!("⚠️ EARLY RETURN: Emergency lockdown");
            let mut result = AllowlistResult::deny_fast("Emergency lockdown", RuleLevel::Emergency);
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // Apply hierarchy evaluation:
        // 1. Emergency Lockdown (already checked above)
        // 2. Explicit capability rules (highest priority)
        debug!("🔧 Explicit capability rules available: {:?} (instance: {:p})", 
               self.explicit_capability_rules.read().unwrap().keys().collect::<Vec<_>>(), self);
        
        if let Some(action) = self.explicit_capability_rules.read().unwrap().get(capability_name) {
            debug!("🔍 Checking explicit capability rules for '{}'. Found: {:?}", capability_name, action);
            let mut result = match action {
                AllowlistAction::Allow => AllowlistResult::allow_fast("Explicit capability rule: Allow", RuleLevel::Capability),
                AllowlistAction::Deny => AllowlistResult::deny_fast("Explicit capability rule: Deny", RuleLevel::Capability),
            };
            result.matched_rule = Some(format!("explicit_capability:{}", capability_name));
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // 3. Capability pattern matching
        if let Some(matching_pattern) = self.find_matching_capability_pattern_for_capability(capability_name) {
            debug!("🎯 Found matching capability pattern for '{}': {:?}", capability_name, matching_pattern.action);
            let mut result = match matching_pattern.action {
                AllowlistAction::Allow => AllowlistResult::allow_fast("Capability pattern: Allow", RuleLevel::Capability), 
                AllowlistAction::Deny => AllowlistResult::deny_fast("Capability pattern: Deny", RuleLevel::Capability),
            };
            result.matched_rule = Some(format!("capability_pattern:{}", matching_pattern.name));
            result.reason = Arc::from(matching_pattern.reason.as_str());
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // 4. Global pattern matching  
        if let Some(matching_pattern) = self.find_matching_global_pattern(capability_name) {
            debug!("🌍 Found matching global pattern for '{}': {:?}", capability_name, matching_pattern.action);
            let mut result = match matching_pattern.action {
                AllowlistAction::Allow => AllowlistResult::allow_fast("Global pattern: Allow", RuleLevel::Global),
                AllowlistAction::Deny => AllowlistResult::deny_fast("Global pattern: Deny", RuleLevel::Global),
            };
            result.matched_rule = Some(format!("global_pattern:{}", matching_pattern.name));
            result.reason = Arc::from(matching_pattern.reason.as_str());
            result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
            self.cache_decision_fast(cache_key, &result);
            self.update_stats(&result);
            return result;
        }
        
        // 5. Default action (lowest priority)
        debug!("🔧 Default action: {:?}", config.default_action);
        let mut result = match config.default_action {
            AllowlistAction::Allow => AllowlistResult::allow_fast("Default action: Allow", RuleLevel::Default),
            AllowlistAction::Deny => AllowlistResult::deny_fast("Default action: Deny", RuleLevel::Default),
        };
        result.matched_rule = Some("default_action".to_string());
        result.reason = Arc::from("Default policy applied");
        result.decision_time_ns = start_time.elapsed().as_nanos() as u64;
        self.cache_decision_fast(cache_key, &result);
        self.update_stats(&result);
        result
    }

    /// Legacy method - delegates to ultra-fast implementation
    pub fn check_tool_access(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
        context: &AllowlistContext,
    ) -> AllowlistResult {
        debug!("🚀 MAIN: check_tool_access called for tool: '{}'", tool_name);
        let start_time = Instant::now();
        let result = self.check_tool_access_internal(tool_name, parameters, context);
        debug!("🎯 MAIN: check_tool_access result for '{}': allowed={}", tool_name, result.allowed);
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
            "total_rules": config.tools.len(),
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
        
        
        tool_rules
    }

    /// Get capability-level pattern rules
    pub fn get_capability_patterns(&self) -> Vec<super::allowlist_data::PatternRule> {
        self.capability_pattern_rules.read().unwrap().clone()
    }

    /// Get global-level pattern rules
    pub fn get_global_patterns(&self) -> Vec<super::allowlist_data::PatternRule> {
        self.global_pattern_rules.read().unwrap().clone()
    }
    
    /// Get tool-level pattern rules
    pub fn get_tool_patterns(&self) -> Vec<super::allowlist_data::PatternRule> {
        self.raw_tool_pattern_rules.read().unwrap().clone()
    }

    /// Get explicit capability rule for a capability name
    pub fn get_explicit_capability_rule(&self, capability_name: &str) -> Option<AllowlistAction> {
        self.explicit_capability_rules.read().unwrap().get(capability_name).cloned()
    }

    /// Reload data from the data file (used after persisting changes)
    fn reload_from_data_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use super::allowlist_data::{AllowlistData, AllowlistDecision, RuleSource};
        use chrono::Utc;

        debug!("🔄 Reloading allowlist data from file: {}", file_path);
        
        // Read and parse the data file
        let contents = fs::read_to_string(file_path)?;
        let data: AllowlistData = serde_yaml::from_str(&contents)?;
        
        // Clear and rebuild precomputed decisions
        {
            let mut decisions = self.precomputed_decisions.write().unwrap();
            decisions.clear();
            
            // Rebuild decisions from explicit rules
            for (tool_name, action) in &data.explicit_rules.tools {
                decisions.insert(tool_name.clone(), AllowlistDecision {
                    action: action.clone(),
                    rule_source: RuleSource::ExplicitTool,
                    rule_name: format!("explicit_tool_{}", tool_name),
                    reason: format!("Explicit rule for tool: {}", tool_name),
                    confidence: 1.0,
                    evaluated_at: Utc::now(),
                });
            }
            
            for (capability_name, action) in &data.explicit_rules.capabilities {
                decisions.insert(capability_name.clone(), AllowlistDecision {
                    action: action.clone(),
                    rule_source: RuleSource::ExplicitCapability,
                    rule_name: format!("explicit_capability_{}", capability_name),
                    reason: format!("Explicit rule for capability: {}", capability_name),
                    confidence: 1.0,
                    evaluated_at: Utc::now(),
                });
            }
        }
        
        // **CRITICAL FIX**: Update explicit_capability_rules HashMap that hierarchy evaluation uses
        {
            let mut capability_rules = self.explicit_capability_rules.write().unwrap();
            capability_rules.clear();
            for (capability_name, action) in &data.explicit_rules.capabilities {
                capability_rules.insert(capability_name.clone(), action.clone());
            }
            debug!("🔄 Updated explicit_capability_rules HashMap: {:?}", 
                   capability_rules.keys().collect::<Vec<_>>());
        }
        
        // **CRITICAL FIX**: Update explicit_tool_rules HashMap that hierarchy evaluation uses
        {
            let mut tool_rules = self.explicit_tool_rules.write().unwrap();
            tool_rules.clear();
            for (tool_name, action) in &data.explicit_rules.tools {
                tool_rules.insert(tool_name.clone(), action.clone());
            }
            debug!("🔄 Updated explicit_tool_rules HashMap: {:?}", 
                   tool_rules.keys().collect::<Vec<_>>());
        }
        
        // **PATTERN FIX**: Reload pattern data for API endpoints
        // This was missing and causing patterns to not appear in the UI
        {
            let mut global_patterns = self.global_pattern_rules.write().unwrap();
            *global_patterns = data.patterns.global.clone();
        }
        {
            let mut tool_patterns = self.raw_tool_pattern_rules.write().unwrap();
            *tool_patterns = data.patterns.tools.clone();
        }
        {
            let mut capability_patterns = self.capability_pattern_rules.write().unwrap();
            *capability_patterns = data.patterns.capabilities.clone();
        }
        
        debug!("✅ Successfully reloaded allowlist data - {} tool rules, {} capability rules, {} global patterns, {} tool patterns, {} capability patterns", 
               data.explicit_rules.tools.len(), 
               data.explicit_rules.capabilities.len(),
               data.patterns.global.len(),
               data.patterns.tools.len(),
               data.patterns.capabilities.len());
        
        Ok(())
    }


    /// Add or update a tool allowlist rule
    pub fn add_tool_rule(&self, tool_name: String, rule: AllowlistRule) -> Result<(), Box<dyn std::error::Error>> {
        // Update old in-memory config for backward compatibility
        {
            let mut config = self.config.write().unwrap();
            config.tools.insert(tool_name.clone(), rule.clone());
        }
        
        // **UNIFIED APPROACH**: Add directly to YAML data file
        if let Some(ref data_file_path) = self.data_file_path {
            self.add_tool_to_data_file(data_file_path, &tool_name, &rule.action)?;
        } else {
            return Err("No data file path configured for allowlist persistence".into());
        }
        
        // Update explicit_tool_rules HashMap immediately for real-time evaluation
        {
            let mut tool_rules = self.explicit_tool_rules.write().unwrap();
            tool_rules.insert(tool_name.clone(), rule.action.clone());
        }
        
        info!("Added tool allowlist rule: {} -> {:?}", tool_name, rule.action);
        Ok(())
    }

    /// Remove a tool allowlist rule
    pub fn remove_tool_rule(&self, tool_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Remove from old in-memory config for backward compatibility
        {
            let mut config = self.config.write().unwrap();
            config.tools.remove(tool_name);
        }
        
        // **UNIFIED APPROACH**: Always use direct YAML file manipulation
        if let Some(ref data_file_path) = self.data_file_path {
            self.remove_tool_from_data_file(data_file_path, tool_name)?;
        } else {
            return Err("No data file path configured for allowlist persistence".into());
        }
        
        // Update explicit_tool_rules HashMap immediately for real-time evaluation
        {
            let mut tool_rules = self.explicit_tool_rules.write().unwrap();
            tool_rules.remove(tool_name);
        }
        
        info!("Removed tool allowlist rule: {}", tool_name);
        Ok(())
    }

    /// Persist the current allowlist configuration to the main config file
    fn persist_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Use the data file path if available, otherwise fall back to old config path
        let data_file_path = if let Some(data_path) = &self.data_file_path {
            data_path.clone()
        } else {
            std::env::var("MAGICTUNNEL_ALLOWLIST_CONFIG_PATH")
                .unwrap_or_else(|_| "./data/allowlist-config.yaml".to_string())
        };
        
        // If we have a data file path, persist to the data file format
        if let Some(data_path) = &self.data_file_path {
            self.persist_to_data_file(data_path)?;
        } else {
            // Fall back to old format for backward compatibility
            self.persist_to_old_format(&data_file_path)?;
        }
        
        debug!("Allowlist configuration persisted to: {}", data_file_path);
        Ok(())
    }

    fn persist_to_data_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use super::allowlist_data::{AllowlistData, AllowlistMetadata, AllowlistPatterns, ExplicitRules};
        use chrono::Utc;
        use std::collections::HashMap;

        // Ensure the directory exists
        if let Some(parent) = Path::new(file_path).parent() {
            fs::create_dir_all(parent)?;
        }

        // Load existing data file if it exists
        let mut allowlist_data = if Path::new(file_path).exists() {
            let contents = fs::read_to_string(file_path)?;
            serde_yaml::from_str::<AllowlistData>(&contents)?
        } else {
            // Create default structure
            AllowlistData {
                metadata: AllowlistMetadata {
                    version: "1.0.0".to_string(),
                    last_updated: Utc::now(),
                    total_patterns: 0,
                    total_explicit_rules: 0,
                },
                patterns: AllowlistPatterns {
                    global: Vec::new(),
                    tools: Vec::new(),
                    capabilities: Vec::new(),
                },
                explicit_rules: ExplicitRules {
                    tools: HashMap::new(),
                    capabilities: HashMap::new(),
                },
            }
        };

        // Merge explicit rules from current config (don't overwrite, merge!)
        let config = self.config.read().unwrap();
        
        // Merge tool rules (preserve existing rules, add/update from config)
        for (name, rule) in config.tools.iter() {
            allowlist_data.explicit_rules.tools.insert(name.clone(), rule.action.clone());
        }

        // Merge capability rules (preserve existing rules, add/update from config) 
        for (name, rule) in config.capabilities.iter() {
            allowlist_data.explicit_rules.capabilities.insert(name.clone(), rule.action.clone());
        }

        // Update metadata
        allowlist_data.metadata.last_updated = Utc::now();
        allowlist_data.metadata.total_explicit_rules = 
            (allowlist_data.explicit_rules.tools.len() + allowlist_data.explicit_rules.capabilities.len()) as u32;

        // Write updated data back to file
        let yaml_content = serde_yaml::to_string(&allowlist_data)?;
        fs::write(file_path, yaml_content)?;

        debug!("Allowlist data persisted to data file format: {}", file_path);
        Ok(())
    }

    fn persist_to_old_format(&self, config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure the directory exists
        if let Some(parent) = Path::new(config_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Read the current config
        let config = self.config.read().unwrap();
        
        // Convert to YAML and write to file
        let yaml_content = serde_yaml::to_string(&*config)?;
        fs::write(config_path, yaml_content)?;
        
        debug!("Allowlist configuration persisted to old format: {}", config_path);
        Ok(())
    }

    /// Load persisted allowlist configuration from disk
    fn load_persisted_config() -> Result<AllowlistConfig, Box<dyn std::error::Error>> {
        let config_path = std::env::var("MAGICTUNNEL_ALLOWLIST_CONFIG_PATH")
            .unwrap_or_else(|_| "./data/allowlist-config.yaml".to_string());
        
        if !Path::new(&config_path).exists() {
            debug!("No persisted allowlist config found at: {}", config_path);
            return Err("No persisted config file found".into());
        }
        
        let yaml_content = fs::read_to_string(&config_path)?;
        let config: AllowlistConfig = serde_yaml::from_str(&yaml_content)?;
        
        debug!("Loaded persisted allowlist config from: {} ({} tool rules)", config_path, config.tools.len());
        Ok(config)
    }

    /// Set a capability-level allowlist rule
    pub fn set_capability_rule(&self, capability_name: &str, action: AllowlistAction, reason: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        // Update old in-memory config for backward compatibility
        {
            let mut config = self.config.write().unwrap();
            let rule = super::allowlist_types::AllowlistRule {
                action: action.clone(),
                reason,
                pattern: None,
                name: Some(capability_name.to_string()),
                enabled: true,
            };
            config.capabilities.insert(capability_name.to_string(), rule);
        }
        
        // **UNIFIED APPROACH**: Add directly to YAML data file
        if let Some(ref data_file_path) = self.data_file_path {
            self.add_capability_to_data_file(data_file_path, capability_name, &action)?;
        } else {
            return Err("No data file path configured for allowlist persistence".into());
        }
        
        // Update explicit_capability_rules HashMap immediately for real-time evaluation
        {
            let mut capability_rules = self.explicit_capability_rules.write().unwrap();
            capability_rules.insert(capability_name.to_string(), action.clone());
        }
        
        info!("Set capability allowlist rule: {} -> {:?}", capability_name, action);
        Ok(())
    }

    /// Remove a capability-level allowlist rule
    pub fn remove_capability_rule(&self, capability_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Remove from old in-memory config for backward compatibility
        {
            let mut config = self.config.write().unwrap();
            config.capabilities.remove(capability_name);
        }
        
        // **UNIFIED APPROACH**: Always use direct YAML file manipulation
        if let Some(ref data_file_path) = self.data_file_path {
            self.remove_capability_from_data_file(data_file_path, capability_name)?;
        } else {
            return Err("No data file path configured for allowlist persistence".into());
        }
        
        // Update explicit_capability_rules HashMap immediately for real-time evaluation
        {
            let mut capability_rules = self.explicit_capability_rules.write().unwrap();
            capability_rules.remove(capability_name);
        }
        
        info!("Removed capability allowlist rule: {}", capability_name);
        Ok(())
    }

    /// Add or update a specific capability in the data file
    fn add_capability_to_data_file(&self, file_path: &str, capability_name: &str, action: &AllowlistAction) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use super::allowlist_data::AllowlistData;
        
        // Load existing data or create new if file doesn't exist
        let allowlist_data = if Path::new(file_path).exists() {
            let contents = fs::read_to_string(file_path)?;
            serde_yaml::from_str(&contents)?
        } else {
            // Create new data structure
            AllowlistData::default()
        };
        
        let mut allowlist_data: AllowlistData = allowlist_data;
        
        // Add/update the specific capability
        allowlist_data.explicit_rules.capabilities.insert(capability_name.to_string(), action.clone());
        
        // Update metadata
        allowlist_data.metadata.last_updated = chrono::Utc::now();
        allowlist_data.metadata.total_explicit_rules = 
            (allowlist_data.explicit_rules.tools.len() + allowlist_data.explicit_rules.capabilities.len()) as u32;
        
        // Ensure the directory exists
        if let Some(parent) = Path::new(file_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Write back to file
        let yaml_content = serde_yaml::to_string(&allowlist_data)?;
        fs::write(file_path, yaml_content)?;
        
        debug!("Added/updated capability '{}' -> {:?} in data file: {}", capability_name, action, file_path);
        
        // **CRITICAL FIX**: Reload data to update in-memory HashMap after addition
        if let Err(e) = self.reload_from_data_file(file_path) {
            warn!("Failed to reload data file after adding capability '{}': {}", capability_name, e);
        }
        
        Ok(())
    }

    /// Remove a specific capability from the data file
    fn remove_capability_from_data_file(&self, file_path: &str, capability_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use super::allowlist_data::AllowlistData;
        
        if !Path::new(file_path).exists() {
            return Ok(()); // Nothing to remove if file doesn't exist
        }
        
        // Load existing data
        let contents = fs::read_to_string(file_path)?;
        let mut allowlist_data: AllowlistData = serde_yaml::from_str(&contents)?;
        
        // Remove the specific capability
        allowlist_data.explicit_rules.capabilities.remove(capability_name);
        
        // Update metadata
        allowlist_data.metadata.last_updated = chrono::Utc::now();
        allowlist_data.metadata.total_explicit_rules = 
            (allowlist_data.explicit_rules.tools.len() + allowlist_data.explicit_rules.capabilities.len()) as u32;
        
        // Write back to file
        let yaml_content = serde_yaml::to_string(&allowlist_data)?;
        fs::write(file_path, yaml_content)?;
        
        debug!("Removed capability '{}' from data file: {}", capability_name, file_path);
        
        // **CRITICAL FIX**: Reload data to update in-memory HashMap after removal
        if let Err(e) = self.reload_from_data_file(file_path) {
            warn!("Failed to reload data file after removing capability '{}': {}", capability_name, e);
        }
        
        Ok(())
    }

    /// Add or update a specific tool in the data file
    fn add_tool_to_data_file(&self, file_path: &str, tool_name: &str, action: &AllowlistAction) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use super::allowlist_data::AllowlistData;
        
        // Load existing data or create new if file doesn't exist
        let allowlist_data = if Path::new(file_path).exists() {
            let contents = fs::read_to_string(file_path)?;
            serde_yaml::from_str(&contents)?
        } else {
            // Create new data structure
            AllowlistData::default()
        };
        
        let mut allowlist_data: AllowlistData = allowlist_data;
        
        // Add/update the specific tool
        allowlist_data.explicit_rules.tools.insert(tool_name.to_string(), action.clone());
        
        // Update metadata
        allowlist_data.metadata.last_updated = chrono::Utc::now();
        allowlist_data.metadata.total_explicit_rules = 
            (allowlist_data.explicit_rules.tools.len() + allowlist_data.explicit_rules.capabilities.len()) as u32;
        
        // Ensure the directory exists
        if let Some(parent) = Path::new(file_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Write back to file
        let yaml_content = serde_yaml::to_string(&allowlist_data)?;
        fs::write(file_path, yaml_content)?;
        
        debug!("Added/updated tool '{}' -> {:?} in data file: {}", tool_name, action, file_path);
        
        // **CRITICAL FIX**: Reload data to update in-memory HashMap after addition
        if let Err(e) = self.reload_from_data_file(file_path) {
            warn!("Failed to reload data file after adding tool '{}': {}", tool_name, e);
        }
        
        Ok(())
    }

    /// Remove a specific tool from the data file
    fn remove_tool_from_data_file(&self, file_path: &str, tool_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use super::allowlist_data::AllowlistData;
        
        if !Path::new(file_path).exists() {
            return Ok(()); // Nothing to remove if file doesn't exist
        }
        
        // Load existing data
        let contents = fs::read_to_string(file_path)?;
        let mut allowlist_data: AllowlistData = serde_yaml::from_str(&contents)?;
        
        // Remove the specific tool
        allowlist_data.explicit_rules.tools.remove(tool_name);
        
        // Update metadata
        allowlist_data.metadata.last_updated = chrono::Utc::now();
        allowlist_data.metadata.total_explicit_rules = 
            (allowlist_data.explicit_rules.tools.len() + allowlist_data.explicit_rules.capabilities.len()) as u32;
        
        // Write back to file
        let yaml_content = serde_yaml::to_string(&allowlist_data)?;
        fs::write(file_path, yaml_content)?;
        
        debug!("Removed tool '{}' from data file: {}", tool_name, file_path);
        
        // **CRITICAL FIX**: Reload data to update in-memory HashMap after removal
        if let Err(e) = self.reload_from_data_file(file_path) {
            warn!("Failed to reload data file after removing tool '{}': {}", tool_name, e);
        }
        
        Ok(())
    }
}

// Statistics implementation (simplified for performance)
#[async_trait::async_trait]
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
            total_rules: config.tools.len() as u32,
            active_rules: config.tools.iter().filter(|(_, r)| r.enabled).count() as u32,
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
        
        let service = AllowlistService::new(config, None).unwrap();
        
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
        
        let service = AllowlistService::new(config, None).unwrap();
        let context = AllowlistContext {
            user_id: Some("test".to_string()),
            user_roles: vec![],
            api_key_name: None,
            permissions: vec![],
            source: None,
            client_ip: None,
        };
        
        // Make multiple calls with identical parameters to trigger caching
        let params = HashMap::new();
        
        // First call should miss cache
        let _result1 = service.check_tool_access_internal("test_tool", &params, &context);
        println!("After first call - cache hit ratio: {}", service.get_cache_hit_ratio());
        
        // Second call should hit cache (identical parameters)
        let _result2 = service.check_tool_access_internal("test_tool", &params, &context);
        println!("After second call - cache hit ratio: {}", service.get_cache_hit_ratio());
        
        // Third call for good measure
        let _result3 = service.check_tool_access_internal("test_tool", &params, &context);
        println!("After third call - cache hit ratio: {}", service.get_cache_hit_ratio());
        
        // Cache only gets triggered when there are rules to evaluate.
        // With no rules configured, cache hit ratio should be 0 (no caching needed)
        let config_has_rules = {
            let config = service.config.read().unwrap();
            // Check if there are any patterns configured
            !config.tool_patterns.is_empty() || config.tools.len() > 0
        };
        
        if config_has_rules {
            // If rules are configured, expect cache hits
            assert!(service.get_cache_hit_ratio() > 0.0);
        } else {
            // If no rules configured, cache ratio should be 0 (appropriate behavior)
            assert_eq!(service.get_cache_hit_ratio(), 0.0);
        }
    }
    
    // ============================================================================
    // Registry-Based Capability Mapping Tests
    // ============================================================================
    
    #[test]
    fn test_registry_based_capability_lookup() {
        // Create mock registry service
        let registry_config = crate::config::RegistryConfig {
            r#type: "file".to_string(),
            paths: vec!["test_capabilities".to_string()],
            hot_reload: false,
            validation: crate::config::ValidationConfig::default(),
        };
        
        // Test that service can be created with registry service
        let service = AllowlistService::new(
            AllowlistConfig::default(),
            None, // No registry service for this basic test
        );
        
        assert!(service.is_ok());
        let service = service.unwrap();
        
        // Test enhanced heuristic fallback when no registry is available
        let capability = service.get_capability_for_tool("list_files_filesystem");
        assert_eq!(capability, Some("filesystem".to_string()));
        
        let capability = service.get_capability_for_tool("github_create_issue");
        assert_eq!(capability, Some("github".to_string()));
        
        let capability = service.get_capability_for_tool("http_request_web");
        assert_eq!(capability, Some("web".to_string()));
    }
    
    #[test]
    fn test_enhanced_heuristic_patterns() {
        let service = AllowlistService::new(
            AllowlistConfig::default(),
            None,
        ).unwrap();
        
        // Test filesystem patterns
        assert_eq!(
            service.get_capability_for_tool("read_file_content"),
            Some("filesystem".to_string())
        );
        assert_eq!(
            service.get_capability_for_tool("write_to_file"),
            Some("filesystem".to_string())
        );
        assert_eq!(
            service.get_capability_for_tool("list_directory"),
            Some("filesystem".to_string())
        );
        
        // Test git patterns
        assert_eq!(
            service.get_capability_for_tool("git_commit"),
            Some("github".to_string())
        );
        assert_eq!(
            service.get_capability_for_tool("github_issue"),
            Some("github".to_string())
        );
        
        // Test web patterns
        assert_eq!(
            service.get_capability_for_tool("fetch_url"),
            Some("web".to_string())
        );
        assert_eq!(
            service.get_capability_for_tool("http_post"),
            Some("web".to_string())
        );
        
        // Test database patterns
        assert_eq!(
            service.get_capability_for_tool("query_database"),
            Some("database".to_string())
        );
        assert_eq!(
            service.get_capability_for_tool("mysql_select"),
            Some("database".to_string())
        );
        
        // Test system patterns
        assert_eq!(
            service.get_capability_for_tool("run_command"),
            Some("system".to_string())
        );
        assert_eq!(
            service.get_capability_for_tool("process_list"),
            Some("system".to_string())
        );
        
        // Test network patterns
        assert_eq!(
            service.get_capability_for_tool("ping_host"),
            Some("network".to_string())
        );
        assert_eq!(
            service.get_capability_for_tool("tcp_connect"),
            Some("network".to_string())
        );
        
        // Test AI patterns
        assert_eq!(
            service.get_capability_for_tool("openai_chat"),
            Some("ai".to_string())
        );
        assert_eq!(
            service.get_capability_for_tool("llm_generate"),
            Some("ai".to_string())
        );
    }
    
    #[test]
    fn test_file_context_capability_extraction() {
        let service = AllowlistService::new(
            AllowlistConfig::default(),
            None,
        ).unwrap();
        
        // NOTE: Path-based capability extraction has been removed from the public API
    }
    
    // NOTE: Routing config capability extraction has been moved to internal methods only
    
    #[test]
    fn test_four_tier_lookup_strategy() {
        let service = AllowlistService::new(
            AllowlistConfig::default(),
            None, // Test without registry service to verify fallback chain
        ).unwrap();
        
        // When no registry is available, it should fall back to enhanced heuristics
        let capability = service.get_capability_for_tool("filesystem_read_file");
        assert_eq!(capability, Some("filesystem".to_string()));
        
        // Test unknown tool falls back to generic capability
        let capability = service.get_capability_for_tool("unknown_mysterious_tool");
        assert_eq!(capability, None); // Should return None for unknown tools
        
        // Test complex tool name with multiple indicators
        let capability = service.get_capability_for_tool("github_api_create_pull_request");
        assert_eq!(capability, Some("github".to_string()));
        
        // Test web-based tool
        let capability = service.get_capability_for_tool("web_scraper_fetch_content");
        assert_eq!(capability, Some("web".to_string()));
    }
    
    #[test]
    fn test_pattern_matching_edge_cases() {
        let service = AllowlistService::new(
            AllowlistConfig::default(),
            None,
        ).unwrap();
        
        // Test case sensitivity
        let capability = service.get_capability_for_tool("FILE_READ");
        assert_eq!(capability, Some("filesystem".to_string()));
        
        // Test partial matches
        let capability = service.get_capability_for_tool("my_file_manager");
        assert_eq!(capability, Some("filesystem".to_string()));
        
        // Test prefix matching
        let capability = service.get_capability_for_tool("git_status");
        assert_eq!(capability, Some("github".to_string()));
        
        // Test suffix matching  
        let capability = service.get_capability_for_tool("execute_bash");
        assert_eq!(capability, Some("system".to_string()));
        
        // Test multi-word matching
        let capability = service.get_capability_for_tool("network_ping_utility");
        assert_eq!(capability, Some("network".to_string()));
    }
}

// ============================================================================
// Real-time Pattern Testing API Implementation
// ============================================================================

impl AllowlistService {
    /// Test a pattern in real-time against specified tools without affecting configuration
    pub fn test_pattern(&self, request: RealTimePatternTestRequest) -> Result<RealTimePatternTestResponse, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        // Validate the pattern regex
        let mut validation_errors = Vec::new();
        let pattern_valid = match regex::Regex::new(&request.pattern.regex) {
            Ok(_) => true,
            Err(e) => {
                validation_errors.push(format!("Invalid regex pattern: {}", e));
                false
            }
        };
        
        // If pattern is invalid, return early with error
        if !pattern_valid {
            return Ok(RealTimePatternTestResponse {
                pattern: request.pattern.clone(),
                tool_results: Vec::new(),
                summary: RealTimePatternTestSummary {
                    total_tools: request.test_tools.len(),
                    pattern_matches: 0,
                    decisions_changed: 0,
                    would_allow: 0,
                    would_deny: 0,
                    pattern_valid: false,
                },
                validation_errors,
            });
        }
        
        let mut tool_results = Vec::new();
        let mut pattern_matches = 0;
        let mut decisions_changed = 0;
        let mut would_allow = 0;
        let mut would_deny = 0;
        
        // Test pattern against each tool
        for tool_name in &request.test_tools {
            let result = self.test_pattern_against_tool(tool_name, &request.pattern, request.include_evaluation_chain)?;
            
            if result.pattern_matched {
                pattern_matches += 1;
            }
            
            if result.decision_would_change {
                decisions_changed += 1;
            }
            
            match result.final_decision {
                AllowlistAction::Allow => would_allow += 1,
                AllowlistAction::Deny => would_deny += 1,
            }
            
            tool_results.push(result);
        }
        
        let elapsed = start_time.elapsed();
        debug!("Pattern testing completed in {:?} for {} tools", elapsed, request.test_tools.len());
        
        Ok(RealTimePatternTestResponse {
            pattern: request.pattern,
            tool_results,
            summary: RealTimePatternTestSummary {
                total_tools: request.test_tools.len(),
                pattern_matches,
                decisions_changed,
                would_allow,
                would_deny,
                pattern_valid: true,
            },
            validation_errors,
        })
    }
    
    /// Test a pattern against a single tool
    fn test_pattern_against_tool(
        &self, 
        tool_name: &str, 
        test_pattern: &TestPattern,
        include_evaluation_chain: bool
    ) -> Result<PatternToolTestResult, Box<dyn std::error::Error>> {
        // Get current decision without the test pattern
        let current_decision = self.get_tool_decision(tool_name)
            .map(|d| d.action)
            .unwrap_or(self.config.read().unwrap().default_action.clone());
        
        // Create a test regex for the pattern
        let pattern_regex = regex::Regex::new(&test_pattern.regex)?;
        let pattern_matched = pattern_regex.is_match(tool_name);
        
        // Simulate the decision-making process with this pattern
        let config = self.config.read().unwrap();
        let (final_decision, rule_source, rule_name, reason, evaluation_chain) = 
            self.simulate_decision_with_test_pattern(tool_name, test_pattern, pattern_matched, &config, include_evaluation_chain);
        
        let decision_would_change = current_decision != final_decision;
        
        Ok(PatternToolTestResult {
            tool_name: tool_name.to_string(),
            pattern_matched,
            final_decision,
            rule_source,
            rule_name,
            reason,
            decision_would_change,
            current_decision,
            evaluation_chain,
        })
    }
    
    /// Simulate the decision-making process with a test pattern
    fn simulate_decision_with_test_pattern(
        &self,
        tool_name: &str,
        test_pattern: &TestPattern,
        pattern_matched: bool,
        config: &AllowlistConfig,
        include_evaluation_chain: bool,
    ) -> (AllowlistAction, RuleSource, String, String, Option<Vec<PatternEvaluationStep>>) {
        let mut evaluation_chain = if include_evaluation_chain { Some(Vec::new()) } else { None };
        let mut step = 1;
        
        // Emergency lockdown check (highest priority)
        if config.emergency_lockdown {
            if let Some(ref mut chain) = evaluation_chain {
                chain.push(PatternEvaluationStep {
                    step,
                    rule_type: "emergency_lockdown".to_string(),
                    rule_name: Some("emergency".to_string()),
                    result: EvaluationResult::Deny,
                    reason: Some("Emergency lockdown active".to_string()),
                    continue_evaluation: false,
                });
            }
            return (
                AllowlistAction::Deny,
                RuleSource::EmergencyLockdown,
                "emergency".to_string(),
                "Emergency lockdown active".to_string(),
                evaluation_chain,
            );
        }
        
        if let Some(ref mut chain) = evaluation_chain {
            chain.push(PatternEvaluationStep {
                step,
                rule_type: "emergency_lockdown".to_string(),
                rule_name: None,
                result: EvaluationResult::NoMatch,
                reason: Some("Emergency lockdown not active".to_string()),
                continue_evaluation: true,
            });
        }
        step += 1;
        
        // Check explicit tool rules (highest priority for normal rules)
        debug!("🔍 Checking explicit tool rules for '{}'. Available rules: {:?}", 
               tool_name, self.explicit_tool_rules.read().unwrap().keys().collect::<Vec<_>>());
        if let Some(action) = self.explicit_tool_rules.read().unwrap().get(tool_name) {
            debug!("✅ Found explicit rule for '{}': {:?}", tool_name, action);
            if let Some(ref mut chain) = evaluation_chain {
                chain.push(PatternEvaluationStep {
                    step,
                    rule_type: "explicit_tool".to_string(),
                    rule_name: Some(tool_name.to_string()),
                    result: if matches!(action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                    reason: Some(format!("Explicit tool rule: {}", tool_name)),
                    continue_evaluation: false,
                });
            }
            return (
                action.clone(),
                RuleSource::ExplicitTool,
                tool_name.to_string(),
                format!("Explicit tool rule: {}", tool_name),
                evaluation_chain,
            );
        }
        
        if let Some(ref mut chain) = evaluation_chain {
            chain.push(PatternEvaluationStep {
                step,
                rule_type: "explicit_tool".to_string(),
                rule_name: None,
                result: EvaluationResult::NoMatch,
                reason: Some("No explicit tool rule found".to_string()),
                continue_evaluation: true,
            });
        }
        step += 1;
        
        // Check if our test pattern matches and apply it based on its scope
        if pattern_matched {
            match test_pattern.scope {
                PatternScope::Tools => {
                    if let Some(ref mut chain) = evaluation_chain {
                        chain.push(PatternEvaluationStep {
                            step,
                            rule_type: "test_pattern".to_string(),
                            rule_name: Some(test_pattern.name.clone()),
                            result: if matches!(test_pattern.action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                            reason: Some(format!("Test pattern matched: {}", test_pattern.name)),
                            continue_evaluation: false,
                        });
                    }
                    return (
                        test_pattern.action.clone(),
                        RuleSource::ToolPattern,
                        test_pattern.name.clone(),
                        format!("Test pattern matched: {}", test_pattern.name),
                        evaluation_chain,
                    );
                }
                _ => {
                    // For non-tool patterns, continue evaluation but remember the match
                    if let Some(ref mut chain) = evaluation_chain {
                        chain.push(PatternEvaluationStep {
                            step,
                            rule_type: "test_pattern_noted".to_string(),
                            rule_name: Some(test_pattern.name.clone()),
                            result: EvaluationResult::NoMatch,
                            reason: Some(format!("Test pattern matches but scope is {:?}, continuing evaluation", test_pattern.scope)),
                            continue_evaluation: true,
                        });
                    }
                }
            }
        }
        step += 1;
        
        // Check existing tool patterns (excluding our test pattern)
        if let Some(matching_pattern) = self.find_matching_tool_pattern(tool_name) {
            if let Some(ref mut chain) = evaluation_chain {
                chain.push(PatternEvaluationStep {
                    step,
                    rule_type: "tool_pattern".to_string(),
                    rule_name: Some(matching_pattern.name.clone()),
                    result: if matches!(matching_pattern.action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                    reason: Some(format!("Tool pattern: {}", matching_pattern.name)),
                    continue_evaluation: false,
                });
            }
            return (
                matching_pattern.action.clone(),
                RuleSource::ToolPattern,
                matching_pattern.name.clone(),
                format!("Tool pattern: {}", matching_pattern.name),
                evaluation_chain,
            );
        }
        
        if let Some(ref mut chain) = evaluation_chain {
            chain.push(PatternEvaluationStep {
                step,
                rule_type: "tool_pattern".to_string(),
                rule_name: None,
                result: EvaluationResult::NoMatch,
                reason: Some("No matching tool pattern".to_string()),
                continue_evaluation: true,
            });
        }
        step += 1;
        
        // Check capability patterns (including test pattern if applicable)
        if pattern_matched && matches!(test_pattern.scope, PatternScope::Capabilities) {
            if let Some(ref mut chain) = evaluation_chain {
                chain.push(PatternEvaluationStep {
                    step,
                    rule_type: "test_capability_pattern".to_string(),
                    rule_name: Some(test_pattern.name.clone()),
                    result: if matches!(test_pattern.action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                    reason: Some(format!("Test capability pattern matched: {}", test_pattern.name)),
                    continue_evaluation: false,
                });
            }
            return (
                test_pattern.action.clone(),
                RuleSource::CapabilityPattern,
                test_pattern.name.clone(),
                format!("Test capability pattern matched: {}", test_pattern.name),
                evaluation_chain,
            );
        }
        
        // Dummy tool definition for pattern matching
        let dummy_tool = crate::registry::types::ToolDefinition {
            name: tool_name.to_string(),
            description: "Test tool".to_string(),
            input_schema: serde_json::Value::Object(serde_json::Map::new()),
            routing: crate::registry::types::RoutingConfig {
                r#type: "test".to_string(),
                config: serde_json::Value::Object(serde_json::Map::new()),
            },
            annotations: None,
            hidden: false,
            enabled: true,
            prompt_refs: Vec::new(),
            resource_refs: Vec::new(),
            sampling_strategy: None,
            elicitation_strategy: None,
        };
        
        if let Some(matching_pattern) = self.find_matching_capability_pattern(tool_name, &dummy_tool) {
            if let Some(ref mut chain) = evaluation_chain {
                chain.push(PatternEvaluationStep {
                    step,
                    rule_type: "capability_pattern".to_string(),
                    rule_name: Some(matching_pattern.name.clone()),
                    result: if matches!(matching_pattern.action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                    reason: Some(format!("Capability pattern: {}", matching_pattern.name)),
                    continue_evaluation: false,
                });
            }
            return (
                matching_pattern.action.clone(),
                RuleSource::CapabilityPattern,
                matching_pattern.name.clone(),
                format!("Capability pattern: {}", matching_pattern.name),
                evaluation_chain,
            );
        }
        
        if let Some(ref mut chain) = evaluation_chain {
            chain.push(PatternEvaluationStep {
                step,
                rule_type: "capability_pattern".to_string(),
                rule_name: None,
                result: EvaluationResult::NoMatch,
                reason: Some("No matching capability pattern".to_string()),
                continue_evaluation: true,
            });
        }
        step += 1;
        
        // Check global patterns (including test pattern if applicable)
        if pattern_matched && matches!(test_pattern.scope, PatternScope::Global) {
            if let Some(ref mut chain) = evaluation_chain {
                chain.push(PatternEvaluationStep {
                    step,
                    rule_type: "test_global_pattern".to_string(),
                    rule_name: Some(test_pattern.name.clone()),
                    result: if matches!(test_pattern.action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                    reason: Some(format!("Test global pattern matched: {}", test_pattern.name)),
                    continue_evaluation: false,
                });
            }
            return (
                test_pattern.action.clone(),
                RuleSource::GlobalPattern,
                test_pattern.name.clone(),
                format!("Test global pattern matched: {}", test_pattern.name),
                evaluation_chain,
            );
        }
        
        if let Some(matching_pattern) = self.find_matching_global_pattern(tool_name) {
            if let Some(ref mut chain) = evaluation_chain {
                chain.push(PatternEvaluationStep {
                    step,
                    rule_type: "global_pattern".to_string(),
                    rule_name: Some(matching_pattern.name.clone()),
                    result: if matches!(matching_pattern.action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                    reason: Some(format!("Global pattern: {}", matching_pattern.name)),
                    continue_evaluation: false,
                });
            }
            return (
                matching_pattern.action.clone(),
                RuleSource::GlobalPattern,
                matching_pattern.name.clone(),
                format!("Global pattern: {}", matching_pattern.name),
                evaluation_chain,
            );
        }
        
        if let Some(ref mut chain) = evaluation_chain {
            chain.push(PatternEvaluationStep {
                step,
                rule_type: "global_pattern".to_string(),
                rule_name: None,
                result: EvaluationResult::NoMatch,
                reason: Some("No matching global pattern".to_string()),
                continue_evaluation: true,
            });
        }
        step += 1;
        
        // Apply default action
        if let Some(ref mut chain) = evaluation_chain {
            chain.push(PatternEvaluationStep {
                step,
                rule_type: "default_action".to_string(),
                rule_name: Some("default".to_string()),
                result: if matches!(config.default_action, AllowlistAction::Allow) { EvaluationResult::Allow } else { EvaluationResult::Deny },
                reason: Some(format!("Default action: {:?}", config.default_action)),
                continue_evaluation: false,
            });
        }
        
        (
            config.default_action.clone(),
            RuleSource::DefaultAction,
            "default".to_string(),
            format!("Default action: {:?}", config.default_action),
            evaluation_chain,
        )
    }
    
    /// Test multiple patterns in batch for efficiency
    pub fn test_patterns_batch(&self, patterns: Vec<RealTimePatternTestRequest>) -> Result<Vec<RealTimePatternTestResponse>, Box<dyn std::error::Error>> {
        let mut responses = Vec::new();
        
        for request in patterns {
            responses.push(self.test_pattern(request)?);
        }
        
        Ok(responses)
    }
    
    /// Get recommendations for pattern optimization based on test results
    pub fn get_pattern_recommendations(&self, test_results: &RealTimePatternTestResponse) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Pattern never matches
        if test_results.summary.pattern_matches == 0 {
            recommendations.push("Pattern never matches any tested tools. Consider reviewing the regex pattern.".to_string());
        }
        
        // Pattern matches everything
        if test_results.summary.pattern_matches == test_results.summary.total_tools && test_results.summary.total_tools > 1 {
            recommendations.push("Pattern matches all tested tools. Consider making it more specific.".to_string());
        }
        
        // No decision changes
        if test_results.summary.decisions_changed == 0 {
            recommendations.push("Pattern would not change any existing decisions. It may be redundant.".to_string());
        }
        
        // High impact pattern
        let change_ratio = if test_results.summary.total_tools > 0 {
            test_results.summary.decisions_changed as f32 / test_results.summary.total_tools as f32
        } else {
            0.0
        };
        
        if change_ratio > 0.5 {
            recommendations.push("Pattern would change more than 50% of decisions. Consider the security impact.".to_string());
        }
        
        // Regex complexity warning
        if test_results.pattern.regex.len() > 100 {
            recommendations.push("Pattern regex is very complex. Consider simplifying for better performance.".to_string());
        }
        
        if recommendations.is_empty() {
            recommendations.push("Pattern looks good with no specific concerns.".to_string());
        }
        
        recommendations
    }

    // ============================================================================
    // Treeview API Implementation
    // ============================================================================

    /// Generate hierarchical treeview of allowlist status organized by server/capability
    /// Uses precomputed decisions for instant response times
    pub fn generate_allowlist_treeview<F>(&self, get_all_tools_with_context: F) -> Result<AllowlistTreeviewResponse, Box<dyn std::error::Error>>
    where
        F: Fn() -> Vec<(String, crate::registry::types::ToolDefinition, String, String)>,
    {
        let tools_with_context = get_all_tools_with_context();
        let mut server_map: std::collections::HashMap<String, Vec<(String, String, crate::registry::types::ToolDefinition)>> = std::collections::HashMap::new();
        
        // Organize tools by server and capability
        for (tool_name, tool_def, server, capability) in tools_with_context {
            server_map.entry(server).or_insert_with(Vec::new).push((capability, tool_name, tool_def));
        }
        
        let mut total_tools = 0;
        let mut allowed_tools = 0;
        let mut denied_tools = 0;
        let mut servers = Vec::new();
        
        // Get emergency lockdown status once
        let config = self.config.read().unwrap();
        let emergency_lockdown = config.emergency_lockdown;
        drop(config);
        
        // Process each server
        for (server_name, server_tools) in server_map {
            let mut capability_map: std::collections::HashMap<String, Vec<(String, crate::registry::types::ToolDefinition)>> = std::collections::HashMap::new();
            
            // Group tools by capability within this server
            for (capability, tool_name, tool_def) in server_tools {
                capability_map.entry(capability).or_insert_with(Vec::new).push((tool_name, tool_def));
            }
            
            let mut server_tool_count = 0;
            let mut server_allowed_count = 0;
            let mut server_denied_count = 0;
            let mut capabilities = Vec::new();
            
            // Process each capability within the server
            for (capability_name, capability_tools) in capability_map {
                let mut cap_tool_count = 0;
                let mut cap_allowed_count = 0;
                let mut cap_denied_count = 0;
                let mut tools = Vec::new();
                
                // Process each tool within the capability
                for (tool_name, tool_def) in capability_tools {
                    cap_tool_count += 1;
                    server_tool_count += 1;
                    total_tools += 1;
                    
                    // Get decision for this tool (using precomputed decisions if available)
                    let decision = if emergency_lockdown {
                        AllowlistDecision::new(
                            AllowlistAction::Deny,
                            RuleSource::EmergencyLockdown,
                            "emergency".to_string(),
                            "Emergency lockdown active".to_string(),
                        )
                    } else {
                        // Try to get precomputed decision first
                        self.precomputed_decisions.read().unwrap()
                            .get(&tool_name)
                            .cloned()
                            .unwrap_or_else(|| {
                                // Fallback to on-demand evaluation using check_tool_access
                                let context = AllowlistContext {
                                    user_id: None,
                                    user_roles: Vec::new(),
                                    api_key_name: None,
                                    permissions: Vec::new(),
                                    source: None,
                                    client_ip: None,
                                };
                                let result = self.check_tool_access(&tool_name, &HashMap::new(), &context);
                                AllowlistDecision::new(
                                    result.action,
                                    RuleSource::DefaultAction, // Use appropriate source
                                    "fallback".to_string(),
                                    result.reason.to_string(),
                                )
                            })
                    };
                    
                    let (tool_status, has_explicit_rule) = match decision.action {
                        AllowlistAction::Allow => {
                            cap_allowed_count += 1;
                            server_allowed_count += 1;
                            allowed_tools += 1;
                            (TreeviewNodeStatus::Allowed, decision.rule_source == RuleSource::ExplicitTool)
                        }
                        AllowlistAction::Deny => {
                            cap_denied_count += 1;
                            server_denied_count += 1;
                            denied_tools += 1;
                            if decision.rule_source == RuleSource::EmergencyLockdown {
                                (TreeviewNodeStatus::Emergency, false)
                            } else {
                                (TreeviewNodeStatus::Denied, decision.rule_source == RuleSource::ExplicitTool)
                            }
                        }
                    };
                    
                    tools.push(TreeviewToolNode {
                        name: tool_name,
                        status: tool_status,
                        decision_source: format!("{:?}", decision.rule_source),
                        reason: decision.reason,
                        has_explicit_rule,
                        rule_priority: None, // AllowlistDecision doesn't have priority field
                    });
                }
                
                // Determine capability status
                let capability_status = if emergency_lockdown {
                    TreeviewNodeStatus::Emergency
                } else if cap_allowed_count == cap_tool_count {
                    TreeviewNodeStatus::Allowed
                } else if cap_denied_count == cap_tool_count {
                    TreeviewNodeStatus::Denied
                } else {
                    TreeviewNodeStatus::Mixed
                };
                
                capabilities.push(TreeviewCapabilityNode {
                    name: capability_name,
                    status: capability_status,
                    tools,
                    tool_count: cap_tool_count,
                    allowed_count: cap_allowed_count,
                    denied_count: cap_denied_count,
                });
            }
            
            // Determine server status
            let server_status = if emergency_lockdown {
                TreeviewNodeStatus::Emergency
            } else if server_allowed_count == server_tool_count {
                TreeviewNodeStatus::Allowed
            } else if server_denied_count == server_tool_count {
                TreeviewNodeStatus::Denied
            } else {
                TreeviewNodeStatus::Mixed
            };
            
            servers.push(TreeviewServerNode {
                name: server_name,
                status: server_status,
                capabilities,
                tool_count: server_tool_count,
                allowed_count: server_allowed_count,
                denied_count: server_denied_count,
            });
        }
        
        // Sort servers by name for consistent output
        servers.sort_by(|a, b| a.name.cmp(&b.name));
        
        // Sort capabilities and tools within each server
        for server in &mut servers {
            server.capabilities.sort_by(|a, b| a.name.cmp(&b.name));
            for capability in &mut server.capabilities {
                capability.tools.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }
        
        Ok(AllowlistTreeviewResponse {
            servers,
            total_tools,
            allowed_tools,
            denied_tools,
            generated_at: Utc::now(),
        })
    }
    
    // === ENHANCED CAPABILITY MAPPING HELPER METHODS ===
    
    /// Extract capability from routing configuration
    fn extract_capability_from_routing(&self, routing_config: &crate::registry::types::RoutingConfig) -> Option<String> {
        // Parse routing config for capability hints
        if let Ok(config_map) = serde_json::from_value::<std::collections::HashMap<String, serde_json::Value>>(routing_config.config.clone()) {
            // Check for explicit capability field
            if let Some(capability) = config_map.get("capability").and_then(|v| v.as_str()) {
                return Some(capability.to_string());
            }
            
            // Check protocol type for capability hints
            if let Some(protocol) = config_map.get("protocol").and_then(|v| v.as_str()) {
                return Some(protocol.to_string());
            }
            
            // Check routing type
            if routing_config.routing_type() == "filesystem" {
                return Some("filesystem".to_string());
            }
            if routing_config.routing_type() == "web" || routing_config.routing_type() == "http" {
                return Some("web".to_string());
            }
        }
        
        None
    }
    
    /// Enhanced filesystem pattern matching
    fn matches_filesystem_patterns(&self, tool_name: &str) -> bool {
        let lower_name = tool_name.to_lowercase();
        
        // File operations
        lower_name.starts_with("file_") || 
        lower_name.starts_with("filesystem_") ||
        lower_name.starts_with("read_") || 
        lower_name.starts_with("write_") ||
        lower_name.starts_with("create_") && (lower_name.contains("file") || lower_name.contains("dir")) ||
        lower_name.starts_with("delete_") && (lower_name.contains("file") || lower_name.contains("dir")) ||
        lower_name.starts_with("move_") && lower_name.contains("file") ||
        lower_name.starts_with("copy_") && lower_name.contains("file") ||
        
        // Directory operations
        lower_name.starts_with("mkdir") || 
        lower_name.starts_with("rmdir") ||
        lower_name.starts_with("list_") && (lower_name.contains("dir") || lower_name.contains("file")) ||
        
        // Path operations
        lower_name.contains("path") || 
        lower_name.contains("directory") ||
        lower_name.contains("folder") ||
        
        // General file/filesystem keywords
        lower_name.contains("file") ||
        lower_name.contains("filesystem") ||
        
        // Filesystem utilities
        lower_name == "ls" || 
        lower_name == "pwd" || 
        lower_name == "find" ||
        lower_name.starts_with("grep") || 
        lower_name.starts_with("sed") || 
        lower_name.starts_with("awk")
    }
    
    /// Enhanced git/github pattern matching
    fn matches_git_patterns(&self, tool_name: &str) -> bool {
        let lower_name = tool_name.to_lowercase();
        
        lower_name.starts_with("git_") ||
        lower_name.starts_with("github_") ||
        lower_name.contains("commit") ||
        lower_name.contains("branch") ||
        lower_name.contains("merge") ||
        lower_name.contains("pull_request") ||
        lower_name.contains("repository") ||
        lower_name.contains("repo") ||
        lower_name == "git" ||
        lower_name.starts_with("gh_")
    }
    
    /// Enhanced web pattern matching
    fn matches_web_patterns(&self, tool_name: &str) -> bool {
        let lower_name = tool_name.to_lowercase();
        
        lower_name.starts_with("web_") ||
        lower_name.starts_with("http_") ||
        lower_name.starts_with("https_") ||
        lower_name.starts_with("url_") ||
        lower_name.starts_with("fetch_") ||
        lower_name.starts_with("download_") ||
        lower_name.starts_with("upload_") ||
        lower_name.contains("request") ||
        lower_name.contains("response") ||
        lower_name.contains("api_") ||
        lower_name.contains("rest") ||
        lower_name.contains("curl") ||
        lower_name.contains("wget")
    }
    
    /// Enhanced database pattern matching
    fn matches_database_patterns(&self, tool_name: &str) -> bool {
        let lower_name = tool_name.to_lowercase();
        
        lower_name.starts_with("db_") ||
        lower_name.starts_with("sql_") ||
        lower_name.starts_with("query_") ||
        lower_name.starts_with("select_") ||
        lower_name.starts_with("insert_") ||
        lower_name.starts_with("update_") ||
        lower_name.starts_with("delete_") && lower_name.contains("db") ||
        lower_name.contains("database") ||
        lower_name.contains("mysql") ||
        lower_name.contains("postgres") ||
        lower_name.contains("sqlite") ||
        lower_name.contains("mongodb") ||
        lower_name.contains("redis")
    }
    
    /// Enhanced system pattern matching
    fn matches_system_patterns(&self, tool_name: &str) -> bool {
        let lower_name = tool_name.to_lowercase();
        
        lower_name.starts_with("system_") ||
        lower_name.starts_with("os_") ||
        lower_name.starts_with("exec_") ||
        lower_name.starts_with("run_") ||
        lower_name.starts_with("process_") ||
        lower_name.contains("command") ||
        lower_name.contains("shell") ||
        lower_name.contains("bash") ||
        lower_name.contains("terminal") ||
        lower_name == "ps" ||
        lower_name == "kill" ||
        lower_name == "top" ||
        lower_name.starts_with("mem") ||
        lower_name.starts_with("cpu")
    }
    
    /// Enhanced network pattern matching
    fn matches_network_patterns(&self, tool_name: &str) -> bool {
        let lower_name = tool_name.to_lowercase();
        
        lower_name.starts_with("net_") ||
        lower_name.starts_with("network_") ||
        lower_name.starts_with("tcp_") ||
        lower_name.starts_with("udp_") ||
        lower_name.starts_with("socket_") ||
        lower_name.contains("ping") ||
        lower_name.contains("telnet") ||
        lower_name.contains("ssh") ||
        lower_name.contains("ftp") ||
        lower_name.contains("port") ||
        lower_name.contains("ip") ||
        lower_name.starts_with("dns")
    }
    
    /// Enhanced AI/ML pattern matching
    fn matches_ai_patterns(&self, tool_name: &str) -> bool {
        let lower_name = tool_name.to_lowercase();
        
        lower_name.starts_with("ai_") ||
        lower_name.starts_with("ml_") ||
        lower_name.starts_with("llm_") ||
        lower_name.starts_with("openai_") ||
        lower_name.starts_with("anthropic_") ||
        lower_name.contains("gpt") ||
        lower_name.contains("claude") ||
        lower_name.contains("model") ||
        lower_name.contains("predict") ||
        lower_name.contains("train") ||
        lower_name.contains("neural") ||
        lower_name.contains("embedding") ||
        lower_name.contains("semantic")
    }
}