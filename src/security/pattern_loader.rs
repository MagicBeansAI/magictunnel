//! Pattern configuration loader for MagicTunnel enterprise allowlist system
//!
//! Loads capability-level and global-level pattern rules from YAML configuration files

use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn, error};
use crate::security::allowlist_types::{PatternRule, AllowlistRule, AllowlistAction, AllowlistPattern};

/// Configuration structure for capability patterns file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityPatternsConfig {
    pub capability_patterns: Vec<CapabilityPatternEntry>,
    pub metadata: Option<PatternMetadata>,
    pub testing: Option<PatternTestConfig>,
}

/// Configuration structure for global patterns file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPatternsConfig {
    pub global_patterns: Vec<GlobalPatternEntry>,
    pub metadata: Option<PatternMetadata>,
    pub testing: Option<PatternTestConfig>,
}

/// Individual pattern entry from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityPatternEntry {
    pub name: String,
    pub pattern: PatternDefinition,
    pub action: AllowlistAction,
    pub reason: String,
    pub priority: i32,
    pub enabled: bool,
}

/// Individual global pattern entry from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPatternEntry {
    pub name: String,
    pub pattern: PatternDefinition,
    pub action: AllowlistAction,
    pub reason: String,
    pub priority: i32,
    pub enabled: bool,
}

/// Pattern definition from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDefinition {
    #[serde(rename = "type")]
    pub pattern_type: String,
    pub value: String,
}

/// Pattern metadata from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMetadata {
    pub version: String,
    pub created: String,
    pub description: String,
    pub total_patterns: Option<u32>,
    pub coverage: Option<String>,
}

/// Pattern testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternTestConfig {
    pub enabled: bool,
    pub test_cases: Vec<PatternTestCase>,
}

/// Individual pattern test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternTestCase {
    pub tool_name: String,
    pub expected_match: Option<String>,
    pub expected_action: String,
}

/// Pattern loader for capability and global patterns
pub struct PatternLoader {
    security_dir: PathBuf,
}

impl PatternLoader {
    /// Create new pattern loader with security directory
    pub fn new<P: AsRef<Path>>(security_dir: P) -> Self {
        Self {
            security_dir: security_dir.as_ref().to_path_buf(),
        }
    }
    
    /// Load capability-level patterns from configuration file
    pub fn load_capability_patterns(&self) -> Result<Vec<PatternRule>, Box<dyn std::error::Error>> {
        let file_path = self.security_dir.join("capability-patterns.yaml");
        
        if !file_path.exists() {
            debug!("Capability patterns file not found at {:?}, using empty patterns", file_path);
            return Ok(Vec::new());
        }
        
        debug!("Loading capability patterns from {:?}", file_path);
        
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read capability patterns file: {}", e))?;
            
        let config: CapabilityPatternsConfig = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse capability patterns YAML: {}", e))?;
            
        let mut patterns = Vec::new();
        
        for entry in config.capability_patterns {
            if !entry.enabled {
                debug!("Skipping disabled capability pattern: {}", entry.name);
                continue;
            }
            
            let pattern = self.parse_pattern_definition(&entry.pattern)?;
            
            let rule = AllowlistRule {
                action: entry.action,
                reason: Some(entry.reason),
                pattern: Some(pattern),
                priority: Some(entry.priority),
                name: Some(entry.name.clone()),
                enabled: true,
            };
            
            patterns.push(PatternRule { rule });
        }
        
        // Sort by priority (lower numbers = higher priority, so ascending order)
        patterns.sort_by(|a, b| {
            let a_priority = a.rule.priority.unwrap_or(999);
            let b_priority = b.rule.priority.unwrap_or(999);
            a_priority.cmp(&b_priority)
        });
        
        debug!("Loaded {} capability patterns", patterns.len());
        Ok(patterns)
    }
    
    /// Load global-level patterns from configuration file
    pub fn load_global_patterns(&self) -> Result<Vec<PatternRule>, Box<dyn std::error::Error>> {
        let file_path = self.security_dir.join("global-patterns.yaml");
        
        if !file_path.exists() {
            debug!("Global patterns file not found at {:?}, using empty patterns", file_path);
            return Ok(Vec::new());
        }
        
        debug!("Loading global patterns from {:?}", file_path);
        
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read global patterns file: {}", e))?;
            
        let config: GlobalPatternsConfig = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse global patterns YAML: {}", e))?;
            
        let mut patterns = Vec::new();
        
        for entry in config.global_patterns {
            if !entry.enabled {
                debug!("Skipping disabled global pattern: {}", entry.name);
                continue;
            }
            
            let pattern = self.parse_pattern_definition(&entry.pattern)?;
            
            let rule = AllowlistRule {
                action: entry.action,
                reason: Some(entry.reason),
                pattern: Some(pattern),
                priority: Some(entry.priority),
                name: Some(entry.name.clone()),
                enabled: true,
            };
            
            patterns.push(PatternRule { rule });
        }
        
        // Sort by priority (lower numbers = higher priority, so ascending order)
        patterns.sort_by(|a, b| {
            let a_priority = a.rule.priority.unwrap_or(999);
            let b_priority = b.rule.priority.unwrap_or(999);
            a_priority.cmp(&b_priority)
        });
        
        debug!("Loaded {} global patterns", patterns.len());
        Ok(patterns)
    }
    
    /// Parse pattern definition from YAML into AllowlistPattern
    fn parse_pattern_definition(&self, pattern_def: &PatternDefinition) -> Result<AllowlistPattern, Box<dyn std::error::Error>> {
        match pattern_def.pattern_type.as_str() {
            "regex" => Ok(AllowlistPattern::Regex { 
                value: pattern_def.value.clone() 
            }),
            "wildcard" => Ok(AllowlistPattern::Wildcard { 
                value: pattern_def.value.clone() 
            }),
            "exact" => Ok(AllowlistPattern::Exact { 
                value: pattern_def.value.clone() 
            }),
            _ => Err(format!("Unknown pattern type: {}", pattern_def.pattern_type).into()),
        }
    }
    
    /// Test all patterns against their test cases
    pub fn test_patterns(&self) -> Result<PatternTestResults, Box<dyn std::error::Error>> {
        let mut results = PatternTestResults::new();
        
        // Test capability patterns
        if let Ok(capability_config) = self.load_capability_config() {
            if let Some(test_config) = capability_config.testing {
                if test_config.enabled {
                    let patterns = self.load_capability_patterns()?;
                    results.capability_results = self.run_pattern_tests(&patterns, &test_config.test_cases)?;
                }
            }
        }
        
        // Test global patterns
        if let Ok(global_config) = self.load_global_config() {
            if let Some(test_config) = global_config.testing {
                if test_config.enabled {
                    let patterns = self.load_global_patterns()?;
                    results.global_results = self.run_pattern_tests(&patterns, &test_config.test_cases)?;
                }
            }
        }
        
        Ok(results)
    }
    
    /// Load raw capability configuration for testing
    fn load_capability_config(&self) -> Result<CapabilityPatternsConfig, Box<dyn std::error::Error>> {
        let file_path = self.security_dir.join("capability-patterns.yaml");
        let content = fs::read_to_string(&file_path)?;
        Ok(serde_yaml::from_str(&content)?)
    }
    
    /// Load raw global configuration for testing
    fn load_global_config(&self) -> Result<GlobalPatternsConfig, Box<dyn std::error::Error>> {
        let file_path = self.security_dir.join("global-patterns.yaml");
        let content = fs::read_to_string(&file_path)?;
        Ok(serde_yaml::from_str(&content)?)
    }
    
    /// Run pattern tests against loaded patterns
    fn run_pattern_tests(&self, patterns: &[PatternRule], test_cases: &[PatternTestCase]) -> Result<Vec<PatternTestResult>, Box<dyn std::error::Error>> {
        use regex::Regex;
        
        let mut results = Vec::new();
        
        for test_case in test_cases {
            let mut matched_rule = None;
            
            // Find first matching pattern (highest priority)
            for pattern_rule in patterns {
                if let Some(ref pattern) = pattern_rule.rule.pattern {
                    let regex_str = match pattern {
                        AllowlistPattern::Regex { value } => value.clone(),
                        AllowlistPattern::Wildcard { value } => {
                            format!("^{}$", value.replace('*', ".*").replace('?', "."))
                        },
                        AllowlistPattern::Exact { value } => {
                            format!("^{}$", regex::escape(value))
                        },
                    };
                    
                    if let Ok(regex) = Regex::new(&regex_str) {
                        if regex.is_match(&test_case.tool_name) {
                            matched_rule = Some(pattern_rule.rule.name.as_ref().unwrap_or(&"unnamed".to_string()).clone());
                            break;
                        }
                    }
                }
            }
            
            let expected_match = test_case.expected_match.as_ref().map(|s| s.as_str());
            let actual_match = matched_rule.as_ref().map(|s| s.as_str());
            
            let result = PatternTestResult {
                tool_name: test_case.tool_name.clone(),
                expected_match: test_case.expected_match.clone(),
                actual_match: matched_rule.clone(),
                expected_action: test_case.expected_action.clone(),
                passed: expected_match == actual_match,
            };
            
            results.push(result);
        }
        
        Ok(results)
    }
}

/// Results of pattern testing
#[derive(Debug, Clone)]
pub struct PatternTestResults {
    pub capability_results: Vec<PatternTestResult>,
    pub global_results: Vec<PatternTestResult>,
}

impl PatternTestResults {
    pub fn new() -> Self {
        Self {
            capability_results: Vec::new(),
            global_results: Vec::new(),
        }
    }
    
    pub fn total_tests(&self) -> usize {
        self.capability_results.len() + self.global_results.len()
    }
    
    pub fn passed_tests(&self) -> usize {
        self.capability_results.iter().filter(|r| r.passed).count() +
        self.global_results.iter().filter(|r| r.passed).count()
    }
    
    pub fn success_rate(&self) -> f64 {
        let total = self.total_tests();
        if total == 0 {
            return 1.0;
        }
        self.passed_tests() as f64 / total as f64
    }
}

/// Individual pattern test result
#[derive(Debug, Clone)]
pub struct PatternTestResult {
    pub tool_name: String,
    pub expected_match: Option<String>,
    pub actual_match: Option<String>,
    pub expected_action: String,
    pub passed: bool,
}