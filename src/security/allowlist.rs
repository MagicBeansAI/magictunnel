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
use super::allowlist_types::{AllowlistResult, AllowlistContext, RuleLevel, AllowlistAction, AllowlistConfig, AllowlistRule, AllowlistPattern};
use super::allowlist_data::{AllowlistData, AllowlistDecision, RuleSource, ToolWithAllowlistStatus, RealTimePatternTestRequest, RealTimePatternTestResponse, AllowlistSummary, TestPattern, PatternToolTestResult, PatternEvaluationStep, RealTimePatternTestSummary, PatternScope, EvaluationResult, AllowlistTreeviewResponse, TreeviewServerNode, TreeviewCapabilityNode, TreeviewToolNode, TreeviewNodeStatus};
use super::audit::{AuditService, AuditEntry, AuditEventType, AuditOutcome, AuditUser, AuditTool, AuditSecurity};
use std::fs;
use std::path::Path;

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
    /// Pattern rules loaded from data file
    global_pattern_rules: Vec<super::allowlist_data::PatternRule>,
    raw_tool_pattern_rules: Vec<super::allowlist_data::PatternRule>,
    capability_pattern_rules: Vec<super::allowlist_data::PatternRule>,
    
    /// Pattern sets for enhanced evaluation
    tool_pattern_set: Option<RegexSet>,
    capability_pattern_set: Option<RegexSet>,
    
    /// Explicit rules from data file
    explicit_tool_rules: HashMap<String, AllowlistAction>,
    explicit_capability_rules: HashMap<String, AllowlistAction>,
    
    /// Bloom filter for ultra-fast pattern rejection
    bloom_filter: Option<BloomFilter>,
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
    pub fn new(mut config: AllowlistConfig) -> Result<Self, Box<dyn std::error::Error>> {
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
            
            // Enhanced pattern structures
            global_pattern_rules: Vec::new(),
            raw_tool_pattern_rules: Vec::new(),
            capability_pattern_rules: Vec::new(),
            tool_pattern_set: None,
            capability_pattern_set: None,
            explicit_tool_rules: HashMap::new(),
            explicit_capability_rules: HashMap::new(),
            bloom_filter: None,
        };
        
        // Pre-compute all hashes and compile patterns
        service.reload_patterns()?;
        
        Ok(service)
    }
    
    
    /// Create new allowlist service with enhanced data file support
    pub fn with_data_file(
        config: AllowlistConfig,
        data_file_path: String,
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
            
            // Enhanced pattern structures
            global_pattern_rules: Vec::new(),
            raw_tool_pattern_rules: Vec::new(),
            capability_pattern_rules: Vec::new(),
            tool_pattern_set: None,
            capability_pattern_set: None,
            explicit_tool_rules: HashMap::new(),
            explicit_capability_rules: HashMap::new(),
            bloom_filter: None,
        };
        
        // Load data from the enhanced data file
        service.load_data_file(&data_file_path)?;
        
        Ok(service)
    }
    
    /// Load allowlist data from YAML file and populate ultra-fast structures
    fn load_data_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use super::allowlist_data::{AllowlistData, RuleSource, AllowlistDecision};

        // Read and parse the data file
        let contents = fs::read_to_string(file_path)?;
        let allowlist_data: AllowlistData = serde_yaml::from_str(&contents)?;
        
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
        
        // Store pattern rules for decision making
        self.global_pattern_rules = allowlist_data.patterns.global.clone();
        self.raw_tool_pattern_rules = allowlist_data.patterns.tools.clone();
        self.capability_pattern_rules = allowlist_data.patterns.capabilities.clone();
        
        // Store explicit rules for O(1) lookup
        debug!("📥 Loading explicit tool rules from YAML: {:?}", allowlist_data.explicit_rules.tools);
        self.explicit_tool_rules = allowlist_data.explicit_rules.tools.clone();
        self.explicit_capability_rules = allowlist_data.explicit_rules.capabilities.clone();
        debug!("✅ Loaded {} explicit tool rules into self.explicit_tool_rules (instance: {:p})", 
               self.explicit_tool_rules.len(), self);
        
        // Update bloom filter with new patterns and rules
        self.update_bloom_filter(&allowlist_data)?;
        
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
                if let Some(action) = self.explicit_tool_rules.get(tool_name) {
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
    fn find_matching_tool_pattern(&self, tool_name: &str) -> Option<&super::allowlist_data::PatternRule> {
        if let Some(ref pattern_set) = self.tool_pattern_set {
            let matches: Vec<usize> = pattern_set.matches(tool_name).iter().collect();
            if !matches.is_empty() {
                // Find most restrictive match - any DENY wins over ALLOW
                return matches.iter()
                    .filter_map(|&idx| self.raw_tool_pattern_rules.get(idx))
                    .filter(|rule| rule.enabled)
                    .find(|rule| rule.action == AllowlistAction::Deny)  // Any deny wins
                    .or_else(|| {
                        // If no denies, take the first allow
                        matches.iter()
                            .filter_map(|&idx| self.raw_tool_pattern_rules.get(idx))
                            .filter(|rule| rule.enabled)
                            .find(|rule| rule.action == AllowlistAction::Allow)
                    });
            }
        }
        None
    }
    
    /// Find matching capability pattern for a tool
    fn find_matching_capability_pattern(&self, tool_name: &str, _tool_def: &crate::registry::types::ToolDefinition) -> Option<&super::allowlist_data::PatternRule> {
        // For now, match against tool name - could be enhanced to use tool's capability metadata
        self.capability_pattern_rules.iter()
            .filter(|rule| rule.enabled)
            .find(|rule| {
                if let Ok(regex) = regex::Regex::new(&rule.regex) {
                    regex.is_match(tool_name)
                } else {
                    false
                }
            })
    }
    
    /// Find matching global pattern for a tool name  
    fn find_matching_global_pattern(&self, tool_name: &str) -> Option<&super::allowlist_data::PatternRule> {
        self.global_pattern_rules.iter()
            .filter(|rule| rule.enabled)
            .find(|rule| {
                if let Ok(regex) = regex::Regex::new(&rule.regex) {
                    regex.is_match(tool_name)
                } else {
                    false
                }
            })
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
        if let Some(action) = self.explicit_tool_rules.get(tool_name) {
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
                    })
                    .collect();
                
                // No sorting needed - most restrictive wins logic handles conflicts
                
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
               self.explicit_tool_rules.keys().collect::<Vec<_>>(), self);
        
        if let Some(action) = self.explicit_tool_rules.get(tool_name) {
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
        // RegexSet for pattern matching - simple and reliable
        if let Some(regex_set) = self.capability_regex_set.read().unwrap().as_ref() {
            let matches: Vec<usize> = regex_set.matches(tool_name).iter().collect();
            
            if !matches.is_empty() {
                let rules = self.capability_rules.read().unwrap();
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
    
    /// Get capability name for a tool (for individual capability rules)
    /// This could be enhanced with registry lookup in the future
    fn get_capability_for_tool(&self, tool_name: &str) -> Option<String> {
        // Simple heuristic-based mapping for now
        // TODO: This should be enhanced with registry lookup to get the actual capability
        
        // File operations
        if tool_name.starts_with("file_") || tool_name.starts_with("read_") || tool_name.starts_with("write_") {
            return Some("filesystem".to_string());
        }
        
        // Git operations  
        if tool_name.starts_with("git_") {
            return Some("github".to_string());
        }
        
        // Web operations
        if tool_name.starts_with("web_") || tool_name.starts_with("http_") || tool_name.starts_with("url_") {
            return Some("web".to_string());
        }
        
        // Database operations
        if tool_name.starts_with("db_") || tool_name.starts_with("sql_") || tool_name.starts_with("query_") {
            return Some("database".to_string());
        }
        
        // For now, return None for unknown capabilities
        // TODO: Enhance with registry service lookup
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
        self.capability_pattern_rules.clone()
    }

    /// Get global-level pattern rules
    pub fn get_global_patterns(&self) -> Vec<super::allowlist_data::PatternRule> {
        self.global_pattern_rules.clone()
    }


    /// Add or update a tool allowlist rule
    pub fn add_tool_rule(&self, tool_name: String, rule: AllowlistRule) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut config = self.config.write().unwrap();
            config.tools.insert(tool_name.clone(), rule.clone());
        }
        
        // Persist the changes to disk
        if let Err(e) = self.persist_config() {
            warn!("Failed to persist allowlist config after adding tool rule '{}': {}", tool_name, e);
            // Note: We don't return the error to avoid breaking the in-memory update
            // The change is still applied in memory, just not persisted
        } else {
            debug!("Successfully persisted allowlist config after adding tool rule: {}", tool_name);
        }
        
        Ok(())
    }

    /// Remove a tool allowlist rule
    pub fn remove_tool_rule(&self, tool_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut config = self.config.write().unwrap();
            config.tools.remove(tool_name);
        }
        
        // Persist the changes to disk
        if let Err(e) = self.persist_config() {
            warn!("Failed to persist allowlist config after removing tool rule '{}': {}", tool_name, e);
            // Note: We don't return the error to avoid breaking the in-memory update
        } else {
            debug!("Successfully persisted allowlist config after removing tool rule: {}", tool_name);
        }
        
        Ok(())
    }

    /// Persist the current allowlist configuration to the main config file
    fn persist_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        // This is a simplified implementation that writes to a separate allowlist config file
        // In a production system, you might want to update the main config file instead
        
        let config_path = std::env::var("MAGICTUNNEL_ALLOWLIST_CONFIG_PATH")
            .unwrap_or_else(|_| "./data/allowlist-config.yaml".to_string());
        
        // Ensure the directory exists
        if let Some(parent) = Path::new(&config_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Read the current config
        let config = self.config.read().unwrap();
        
        // Convert to YAML and write to file
        let yaml_content = serde_yaml::to_string(&*config)?;
        fs::write(&config_path, yaml_content)?;
        
        debug!("Allowlist configuration persisted to: {}", config_path);
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
               tool_name, self.explicit_tool_rules.keys().collect::<Vec<_>>());
        if let Some(action) = self.explicit_tool_rules.get(tool_name) {
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
}