//! Request sanitization and content filtering for MagicTunnel
//!
//! Provides policies to sanitize and block MCP calls containing sensitive information,
//! similar to MCP Manager's content filtering capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use regex::Regex;
use chrono::{DateTime, Utc};
use tracing::{debug, warn};
use super::statistics::{SecurityServiceStatistics, HealthMonitor, ServiceHealth, HealthStatus, SanitizationStatistics, PolicyTrigger, PerformanceMetrics};

/// Configuration for request sanitization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationConfig {
    /// Whether sanitization is enabled
    pub enabled: bool,
    /// Sanitization policies
    pub policies: Vec<SanitizationPolicy>,
    /// Default action when no policy matches
    pub default_action: SanitizationAction,
    /// Whether to log sanitization actions
    pub log_actions: bool,
}

/// Sanitization policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationPolicy {
    /// Policy name for logging
    pub name: String,
    /// When this policy applies
    pub triggers: Vec<SanitizationTrigger>,
    /// Action to take
    pub action: SanitizationAction,
    /// Priority (higher numbers take precedence)
    pub priority: i32,
    /// Whether this policy is active
    pub enabled: bool,
}

/// Trigger conditions for sanitization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SanitizationTrigger {
    /// Detect secrets and sensitive data
    SecretDetection {
        /// Types of secrets to detect
        secret_types: Vec<SecretType>,
        /// Custom regex patterns
        custom_patterns: Option<Vec<String>>,
    },
    /// Content-based filtering
    ContentFilter {
        /// Regex patterns to match
        patterns: Vec<String>,
        /// Case sensitive matching
        case_sensitive: bool,
        /// Match against specific fields only
        target_fields: Option<Vec<String>>,
    },
    /// Size-based filtering
    SizeLimit {
        /// Maximum size in bytes
        max_size: usize,
        /// Apply to specific fields
        target_fields: Option<Vec<String>>,
    },
    /// Tool-specific filtering
    ToolFilter {
        /// Tool name patterns
        tool_patterns: Vec<String>,
        /// Parameter patterns to check
        parameter_patterns: Option<Vec<String>>,
    },
}

/// Types of secrets to detect
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SecretType {
    /// API keys
    ApiKey,
    /// AWS credentials
    AwsCredentials,
    /// Database connection strings
    DatabaseUrl,
    /// Private keys (RSA, SSH, etc.)
    PrivateKey,
    /// JWT tokens
    JwtToken,
    /// Email addresses
    Email,
    /// Phone numbers
    PhoneNumber,
    /// Credit card numbers
    CreditCard,
    /// Social security numbers
    Ssn,
    /// IP addresses
    IpAddress,
    /// File paths
    FilePath,
    /// URLs with credentials
    CredentialUrl,
}

/// Action to take when sanitization trigger is hit
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SanitizationAction {
    /// Block the request entirely
    Block {
        /// Custom error message
        message: Option<String>,
    },
    /// Sanitize by removing/masking content
    Sanitize {
        /// How to sanitize the content
        method: SanitizationMethod,
    },
    /// Log and allow (monitoring mode)
    LogAndAllow {
        /// Log level
        level: LogLevel,
    },
    /// Require approval before proceeding
    RequireApproval {
        /// Approval workflow
        workflow: ApprovalWorkflow,
    },
}

/// How to sanitize content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SanitizationMethod {
    /// Replace with mask characters
    Mask {
        /// Character to use for masking
        mask_char: char,
        /// Whether to preserve structure (e.g., keep dashes in phone numbers)
        preserve_structure: bool,
    },
    /// Remove entirely
    Remove,
    /// Replace with placeholder text
    Replace {
        /// Replacement text
        replacement: String,
    },
    /// Hash the content
    Hash {
        /// Hash algorithm
        algorithm: HashAlgorithm,
        /// Whether to include original length in output
        include_length: bool,
    },
}

/// Hash algorithms for sanitization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    Sha256,
    Sha512,
    Blake3,
}

/// Log levels for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Approval workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalWorkflow {
    /// Required approvers
    pub approvers: Vec<String>,
    /// Timeout for approval in seconds
    pub timeout_seconds: u64,
    /// Whether to allow override by admin
    pub admin_override: bool,
}

/// Result of sanitization
#[derive(Debug, Clone)]
pub struct SanitizationResult {
    /// Whether the request was modified
    pub modified: bool,
    /// Action taken
    pub action: SanitizationAction,
    /// Policies that matched
    pub matched_policies: Vec<String>,
    /// Details of what was sanitized
    pub sanitization_details: Vec<SanitizationDetail>,
    /// Whether request should be blocked
    pub should_block: bool,
    /// Whether approval is required
    pub requires_approval: bool,
}

/// Details of a specific sanitization action
#[derive(Debug, Clone)]
pub struct SanitizationDetail {
    /// What triggered the sanitization
    pub trigger: String,
    /// Field that was sanitized
    pub field: String,
    /// Original value (for logging only)
    pub original_value: Option<String>,
    /// Sanitized value
    pub sanitized_value: String,
    /// Method used
    pub method: String,
}

/// Statistics tracking for sanitization service
#[derive(Debug, Clone)]
struct SanitizationStats {
    start_time: DateTime<Utc>,
    total_requests: u64,
    sanitized_requests: u64,
    blocked_requests: u64,
    alerts_generated: u64,
    secrets_detected: u64,
    policy_triggers: HashMap<String, u64>,
    last_error: Option<String>,
    total_processing_time_ms: u64,
}

/// Request sanitization service
pub struct SanitizationService {
    config: SanitizationConfig,
    secret_patterns: HashMap<SecretType, Regex>,
    policy_patterns: HashMap<String, Vec<Regex>>,
    stats: Arc<Mutex<SanitizationStats>>,
}

impl Default for SanitizationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            policies: Vec::new(),
            default_action: SanitizationAction::LogAndAllow {
                level: LogLevel::Info,
            },
            log_actions: true,
        }
    }
}

impl SanitizationService {
    /// Create a new sanitization service
    pub fn new(config: SanitizationConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut secret_patterns = HashMap::new();
        let mut policy_patterns = HashMap::new();
        
        // Compile secret detection patterns
        secret_patterns.insert(
            SecretType::ApiKey,
            Regex::new(r"(?i)(api[_-]?key|token|secret)[\s\:=]*[a-zA-Z0-9_-]{16,}")?,
        );
        
        secret_patterns.insert(
            SecretType::AwsCredentials,
            Regex::new(r"(?i)(aws[_-]?(access[_-]?key|secret[_-]?key))[\s\:=]*[A-Z0-9]{16,}")?,
        );
        
        secret_patterns.insert(
            SecretType::DatabaseUrl,
            Regex::new(r"(?i)(mysql|postgres|mongodb|redis)://[^/]*:[^@]*@[^/]*")?,
        );
        
        secret_patterns.insert(
            SecretType::PrivateKey,
            Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")?,
        );
        
        secret_patterns.insert(
            SecretType::JwtToken,
            Regex::new(r"eyJ[A-Za-z0-9_-]*\.eyJ[A-Za-z0-9_-]*\.[A-Za-z0-9_-]*")?,
        );
        
        secret_patterns.insert(
            SecretType::Email,
            Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")?,
        );
        
        secret_patterns.insert(
            SecretType::PhoneNumber,
            Regex::new(r"(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}")?,
        );
        
        secret_patterns.insert(
            SecretType::CreditCard,
            Regex::new(r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|3[0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})\b")?,
        );
        
        secret_patterns.insert(
            SecretType::Ssn,
            Regex::new(r"\b\d{3}-?\d{2}-?\d{4}\b")?,
        );
        
        secret_patterns.insert(
            SecretType::IpAddress,
            Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b")?,
        );
        
        secret_patterns.insert(
            SecretType::FilePath,
            Regex::new(r"(?:[a-zA-Z]:[\\]|/)[a-zA-Z0-9._/-]+")?,
        );
        
        secret_patterns.insert(
            SecretType::CredentialUrl,
            Regex::new(r"(?i)https?://[^/]*:[^@]*@[^/]*")?,
        );
        
        // Compile policy patterns
        for policy in &config.policies {
            let mut patterns = Vec::new();
            for trigger in &policy.triggers {
                match trigger {
                    SanitizationTrigger::ContentFilter { patterns: pattern_list, .. } => {
                        for pattern in pattern_list {
                            patterns.push(Regex::new(pattern)?);
                        }
                    }
                    SanitizationTrigger::SecretDetection { custom_patterns: Some(custom), .. } => {
                        for pattern in custom {
                            patterns.push(Regex::new(pattern)?);
                        }
                    }
                    _ => {}
                }
            }
            if !patterns.is_empty() {
                policy_patterns.insert(policy.name.clone(), patterns);
            }
        }
        
        let stats = SanitizationStats {
            start_time: Utc::now(),
            total_requests: 0,
            sanitized_requests: 0,
            blocked_requests: 0,
            alerts_generated: 0,
            secrets_detected: 0,
            policy_triggers: HashMap::new(),
            last_error: None,
            total_processing_time_ms: 0,
        };

        Ok(Self {
            config,
            secret_patterns,
            policy_patterns,
            stats: Arc::new(Mutex::new(stats)),
        })
    }
    
    /// Sanitize request data
    pub fn sanitize_request(
        &self,
        request_data: &mut serde_json::Value,
        tool_name: Option<&str>,
    ) -> SanitizationResult {
        if !self.config.enabled {
            return SanitizationResult {
                modified: false,
                action: SanitizationAction::LogAndAllow { level: LogLevel::Info },
                matched_policies: Vec::new(),
                sanitization_details: Vec::new(),
                should_block: false,
                requires_approval: false,
            };
        }
        
        let mut result = SanitizationResult {
            modified: false,
            action: self.config.default_action.clone(),
            matched_policies: Vec::new(),
            sanitization_details: Vec::new(),
            should_block: false,
            requires_approval: false,
        };
        
        // Sort policies by priority (highest first)
        let mut policies = self.config.policies.clone();
        policies.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        for policy in &policies {
            if !policy.enabled {
                continue;
            }
            
            for trigger in &policy.triggers {
                if self.check_trigger(trigger, request_data, tool_name, &mut result) {
                    result.matched_policies.push(policy.name.clone());
                    result.action = policy.action.clone();
                    
                    match &policy.action {
                        SanitizationAction::Block { .. } => {
                            result.should_block = true;
                            return result;
                        }
                        SanitizationAction::Sanitize { method } => {
                            self.apply_sanitization(request_data, method, &trigger, &mut result);
                            result.modified = true;
                        }
                        SanitizationAction::RequireApproval { .. } => {
                            result.requires_approval = true;
                        }
                        SanitizationAction::LogAndAllow { level } => {
                            if self.config.log_actions {
                                match level {
                                    LogLevel::Debug => debug!("Sanitization policy '{}' matched", policy.name),
                                    LogLevel::Info => tracing::info!("Sanitization policy '{}' matched", policy.name),
                                    LogLevel::Warn => warn!("Sanitization policy '{}' matched", policy.name),
                                    LogLevel::Error => tracing::error!("Sanitization policy '{}' matched", policy.name),
                                }
                            }
                        }
                    }
                    
                    // First matching policy wins (highest priority)
                    break;
                }
            }
        }
        
        result
    }
    
    /// Check if a trigger condition is met
    fn check_trigger(
        &self,
        trigger: &SanitizationTrigger,
        data: &serde_json::Value,
        tool_name: Option<&str>,
        result: &mut SanitizationResult,
    ) -> bool {
        match trigger {
            SanitizationTrigger::SecretDetection { secret_types, .. } => {
                self.detect_secrets(data, secret_types, result)
            }
            SanitizationTrigger::ContentFilter { patterns, case_sensitive, target_fields } => {
                self.check_content_filter(data, patterns, *case_sensitive, target_fields.as_ref(), result)
            }
            SanitizationTrigger::SizeLimit { max_size, target_fields } => {
                self.check_size_limit(data, *max_size, target_fields.as_ref(), result)
            }
            SanitizationTrigger::ToolFilter { tool_patterns, parameter_patterns } => {
                self.check_tool_filter(tool_name, tool_patterns, parameter_patterns.as_ref(), data, result)
            }
        }
    }
    
    /// Detect secrets in data
    fn detect_secrets(
        &self,
        data: &serde_json::Value,
        secret_types: &[SecretType],
        result: &mut SanitizationResult,
    ) -> bool {
        let data_str = data.to_string();
        
        for secret_type in secret_types {
            if let Some(pattern) = self.secret_patterns.get(secret_type) {
                if pattern.is_match(&data_str) {
                    result.sanitization_details.push(SanitizationDetail {
                        trigger: format!("Secret detection: {:?}", secret_type),
                        field: "request_data".to_string(),
                        original_value: None, // Don't log sensitive data
                        sanitized_value: "[REDACTED]".to_string(),
                        method: "secret_detection".to_string(),
                    });
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Check content filter patterns
    fn check_content_filter(
        &self,
        data: &serde_json::Value,
        patterns: &[String],
        case_sensitive: bool,
        _target_fields: Option<&Vec<String>>,
        result: &mut SanitizationResult,
    ) -> bool {
        let data_str = if case_sensitive {
            data.to_string()
        } else {
            data.to_string().to_lowercase()
        };
        
        for pattern in patterns {
            let regex_pattern = if case_sensitive {
                pattern.clone()
            } else {
                format!("(?i){}", pattern)
            };
            
            if let Ok(regex) = Regex::new(&regex_pattern) {
                if regex.is_match(&data_str) {
                    result.sanitization_details.push(SanitizationDetail {
                        trigger: format!("Content filter: {}", pattern),
                        field: "request_data".to_string(),
                        original_value: None,
                        sanitized_value: "[FILTERED]".to_string(),
                        method: "content_filter".to_string(),
                    });
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Check size limits
    fn check_size_limit(
        &self,
        data: &serde_json::Value,
        max_size: usize,
        _target_fields: Option<&Vec<String>>,
        result: &mut SanitizationResult,
    ) -> bool {
        let data_size = data.to_string().len();
        
        if data_size > max_size {
            result.sanitization_details.push(SanitizationDetail {
                trigger: format!("Size limit exceeded: {} > {}", data_size, max_size),
                field: "request_data".to_string(),
                original_value: None,
                sanitized_value: format!("[TRUNCATED {} bytes]", data_size),
                method: "size_limit".to_string(),
            });
            return true;
        }
        
        false
    }
    
    /// Check tool-specific filters
    fn check_tool_filter(
        &self,
        tool_name: Option<&str>,
        tool_patterns: &[String],
        _parameter_patterns: Option<&Vec<String>>,
        _data: &serde_json::Value,
        result: &mut SanitizationResult,
    ) -> bool {
        if let Some(tool) = tool_name {
            for pattern in tool_patterns {
                if let Ok(regex) = Regex::new(pattern) {
                    if regex.is_match(tool) {
                        result.sanitization_details.push(SanitizationDetail {
                            trigger: format!("Tool filter: {}", pattern),
                            field: "tool_name".to_string(),
                            original_value: Some(tool.to_string()),
                            sanitized_value: "[FILTERED_TOOL]".to_string(),
                            method: "tool_filter".to_string(),
                        });
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// Apply sanitization method to data
    fn apply_sanitization(
        &self,
        data: &mut serde_json::Value,
        method: &SanitizationMethod,
        _trigger: &SanitizationTrigger,
        _result: &mut SanitizationResult,
    ) {
        match method {
            SanitizationMethod::Mask { mask_char, preserve_structure } => {
                // Apply masking logic
                self.mask_data(data, *mask_char, *preserve_structure);
            }
            SanitizationMethod::Remove => {
                // Remove sensitive data
                *data = serde_json::Value::Null;
            }
            SanitizationMethod::Replace { replacement } => {
                // Replace with placeholder
                *data = serde_json::Value::String(replacement.clone());
            }
            SanitizationMethod::Hash { algorithm, include_length } => {
                // Hash the data
                let hash = self.hash_data(data, algorithm);
                let new_value = if *include_length {
                    format!("{}[{}]", hash, data.to_string().len())
                } else {
                    hash
                };
                *data = serde_json::Value::String(new_value);
            }
        }
    }
    
    /// Mask sensitive data
    fn mask_data(&self, data: &mut serde_json::Value, mask_char: char, preserve_structure: bool) {
        match data {
            serde_json::Value::String(s) => {
                if preserve_structure {
                    // Keep certain characters like dashes, spaces
                    let mut masked = String::new();
                    for c in s.chars() {
                        if c.is_alphanumeric() {
                            masked.push(mask_char);
                        } else {
                            masked.push(c);
                        }
                    }
                    *s = masked;
                } else {
                    *s = mask_char.to_string().repeat(s.len().min(8));
                }
            }
            serde_json::Value::Object(obj) => {
                for value in obj.values_mut() {
                    self.mask_data(value, mask_char, preserve_structure);
                }
            }
            serde_json::Value::Array(arr) => {
                for value in arr.iter_mut() {
                    self.mask_data(value, mask_char, preserve_structure);
                }
            }
            _ => {}
        }
    }
    
    /// Hash data using specified algorithm
    fn hash_data(&self, data: &serde_json::Value, algorithm: &HashAlgorithm) -> String {
        let data_str = data.to_string();
        
        match algorithm {
            HashAlgorithm::Sha256 => {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(data_str.as_bytes());
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::Sha512 => {
                use sha2::{Sha512, Digest};
                let mut hasher = Sha512::new();
                hasher.update(data_str.as_bytes());
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::Blake3 => {
                let hash = blake3::hash(data_str.as_bytes());
                hash.to_hex().to_string()
            }
        }
    }
    
    /// Get all sanitization policies for API display
    pub fn get_policies_for_api(&self) -> serde_json::Value {
        use serde_json::json;
        
        let policies: Vec<serde_json::Value> = self.config.policies.iter().enumerate().map(|(index, policy)| {
            let trigger_types: Vec<String> = policy.triggers.iter().map(|trigger| {
                match trigger {
                    SanitizationTrigger::SecretDetection { .. } => "secret_detection".to_string(),
                    SanitizationTrigger::ContentFilter { .. } => "content_filter".to_string(),
                    SanitizationTrigger::SizeLimit { .. } => "size_limit".to_string(),
                    SanitizationTrigger::ToolFilter { .. } => "tool_filter".to_string(),
                }
            }).collect();
            
            json!({
                "id": (index + 1).to_string(),
                "name": policy.name,
                "enabled": policy.enabled,
                "priority": policy.priority,
                "trigger_types": trigger_types,
                "action_type": match &policy.action {
                    SanitizationAction::Block { .. } => "block",
                    SanitizationAction::LogAndAllow { .. } => "log_and_allow",
                    SanitizationAction::Sanitize { .. } => "sanitize",
                    SanitizationAction::RequireApproval { .. } => "require_approval",
                },
                "created_at": Utc::now(),
                "updated_at": Utc::now(),
                "description": format!("Policy with {} trigger(s)", policy.triggers.len())
            })
        }).collect();
        
        json!(policies)
    }
    
    /// Get secret detection rules from configured patterns
    pub fn get_secret_detection_rules(&self) -> serde_json::Value {
        use serde_json::json;
        
        let mut rules = Vec::new();
        let mut rule_id = 1;
        
        // Add predefined secret detection rules based on configured patterns
        for (secret_type, pattern) in &self.secret_patterns {
            rules.push(json!({
                "id": rule_id.to_string(),
                "name": format!("{:?} Detection", secret_type),
                "enabled": true,
                "pattern": pattern.as_str(),
                "severity": "high",
                "description": format!("Detects {} patterns in content", format!("{:?}", secret_type).to_lowercase()),
                "created_at": chrono::Utc::now(),
                "updated_at": chrono::Utc::now()
            }));
            rule_id += 1;
        }
        
        // Add custom patterns from policies
        for policy in &self.config.policies {
            for trigger in &policy.triggers {
                if let SanitizationTrigger::SecretDetection { custom_patterns: Some(patterns), .. } = trigger {
                    for pattern in patterns {
                        rules.push(json!({
                            "id": rule_id.to_string(),
                            "name": format!("Custom Secret Rule {}", rule_id),
                            "enabled": policy.enabled,
                            "pattern": pattern,
                            "severity": "medium",
                            "description": "Custom secret detection pattern",
                            "policy_name": policy.name,
                            "created_at": chrono::Utc::now(),
                            "updated_at": chrono::Utc::now()
                        }));
                        rule_id += 1;
                    }
                }
            }
        }
        
        json!(rules)
    }
    
    /// Get content filter rules from configured policies
    pub fn get_content_filter_rules(&self) -> serde_json::Value {
        use serde_json::json;
        
        let mut rules = Vec::new();
        let mut rule_id = 1;
        
        for policy in &self.config.policies {
            for trigger in &policy.triggers {
                if let SanitizationTrigger::ContentFilter { patterns, case_sensitive, target_fields } = trigger {
                    for pattern in patterns {
                        rules.push(json!({
                            "id": rule_id.to_string(),
                            "name": format!("Content Filter: {}", pattern.chars().take(50).collect::<String>()),
                            "enabled": policy.enabled,
                            "pattern": pattern,
                            "case_sensitive": case_sensitive,
                            "target_fields": target_fields.as_ref().unwrap_or(&vec!["all".to_string()]),
                            "action": match &policy.action {
                                SanitizationAction::Block { .. } => "block",
                                SanitizationAction::Sanitize { .. } => "sanitize",
                                SanitizationAction::LogAndAllow { .. } => "log_and_allow",
                                SanitizationAction::RequireApproval { .. } => "require_approval",
                            },
                            "policy_name": policy.name,
                            "description": format!("Content filtering rule from policy: {}", policy.name),
                            "created_at": chrono::Utc::now(),
                            "updated_at": chrono::Utc::now()
                        }));
                        rule_id += 1;
                    }
                }
            }
        }
        
        json!(rules)
    }
    
    /// Get security alerts related to sanitization events
    pub fn get_security_alerts(&self, query_params: &serde_json::Value) -> serde_json::Value {
        use serde_json::json;
        
        let stats = self.stats.lock().unwrap();
        let mut alerts = Vec::new();
        
        // Generate alerts based on statistics
        if stats.secrets_detected > 0 {
            alerts.push(json!({
                "id": "sanitization_secrets",
                "severity": "high",
                "category": "sanitization",
                "title": "Secrets Detected in Content",
                "description": format!("Detected {} potential secrets in content", stats.secrets_detected),
                "timestamp": stats.start_time,
                "count": stats.secrets_detected,
                "status": "active"
            }));
        }
        
        if stats.blocked_requests > stats.total_requests / 10 { // More than 10% blocked
            alerts.push(json!({
                "id": "sanitization_high_block_rate",
                "severity": "warning",
                "category": "sanitization", 
                "title": "High Content Block Rate",
                "description": format!("High rate of blocked requests: {}/{}", stats.blocked_requests, stats.total_requests),
                "timestamp": chrono::Utc::now(),
                "count": stats.blocked_requests,
                "status": "active"
            }));
        }
        
        // Filter by severity if requested
        if let Some(severity) = query_params.get("severity").and_then(|v| v.as_str()) {
            alerts.retain(|alert| {
                alert.get("severity").and_then(|s| s.as_str()) == Some(severity)
            });
        }
        
        json!(alerts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_secret_detection() {
        let config = SanitizationConfig {
            enabled: true,
            policies: vec![SanitizationPolicy {
                name: "api_key_detection".to_string(),
                triggers: vec![SanitizationTrigger::SecretDetection {
                    secret_types: vec![SecretType::ApiKey],
                    custom_patterns: None,
                }],
                action: SanitizationAction::Block { message: None },
                priority: 1,
                enabled: true,
            }],
            default_action: SanitizationAction::LogAndAllow { level: LogLevel::Info },
            log_actions: true,
        };
        
        let service = SanitizationService::new(config).unwrap();
        let mut request_data = serde_json::json!({
            "api_key": "sk-1234567890abcdef1234567890abcdef"
        });
        
        let result = service.sanitize_request(&mut request_data, Some("test_tool"));
        assert!(result.should_block);
        assert!(!result.matched_policies.is_empty());
    }
}

// Implementation of SecurityServiceStatistics trait for SanitizationService
impl SecurityServiceStatistics for SanitizationService {
    type Statistics = SanitizationStatistics;
    
    async fn get_statistics(&self) -> Self::Statistics {
        let stats = match self.stats.lock() {
            Ok(stats) => stats.clone(),
            Err(_) => {
                // If mutex is poisoned, return default stats
                SanitizationStats {
                    start_time: Utc::now(),
                    total_requests: 0,
                    sanitized_requests: 0,
                    blocked_requests: 0,
                    alerts_generated: 0,
                    secrets_detected: 0,
                    policy_triggers: HashMap::new(),
                    last_error: Some("Mutex poisoned".to_string()),
                    total_processing_time_ms: 0,
                }
            }
        };
        let service_health = self.get_health().await;
        
        // Get top triggered policies
        let mut policy_triggers: Vec<PolicyTrigger> = stats.policy_triggers.iter()
            .map(|(policy_name, count)| PolicyTrigger {
                policy_name: policy_name.clone(),
                policy_type: "content_filter".to_string(), // Simplified
                trigger_count: *count,
                action: "sanitize".to_string(), // Simplified
                effectiveness_rate: 1.0, // Would need actual tracking
            })
            .collect();
        policy_triggers.sort_by(|a, b| b.trigger_count.cmp(&a.trigger_count));
        policy_triggers.truncate(10); // Top 10 policies
        
        let detection_rate = if stats.total_requests > 0 {
            (stats.secrets_detected as f64 / stats.total_requests as f64)
        } else { 0.0 };
        
        SanitizationStatistics {
            health: service_health,
            total_policies: self.config.policies.len() as u32,
            active_policies: self.config.policies.iter().filter(|p| p.enabled).count() as u32,
            total_requests: stats.total_requests,
            sanitized_requests: stats.sanitized_requests,
            blocked_requests: stats.blocked_requests,
            alerts_generated: stats.alerts_generated,
            secrets_detected: stats.secrets_detected,
            detection_rate,
            top_policies: policy_triggers,
        }
    }
    
    async fn get_health(&self) -> ServiceHealth {
        let stats = match self.stats.lock() {
            Ok(stats) => stats.clone(),
            Err(_) => {
                // If mutex is poisoned, return default stats
                SanitizationStats {
                    start_time: Utc::now(),
                    total_requests: 0,
                    sanitized_requests: 0,
                    blocked_requests: 0,
                    alerts_generated: 0,
                    secrets_detected: 0,
                    policy_triggers: HashMap::new(),
                    last_error: Some("Mutex poisoned".to_string()),
                    total_processing_time_ms: 0,
                }
            }
        };
        let uptime_seconds = (Utc::now() - stats.start_time).num_seconds() as u64;
        
        let avg_response_time_ms = if stats.total_requests > 0 {
            stats.total_processing_time_ms as f64 / stats.total_requests as f64
        } else {
            0.0
        };
        
        let error_rate = if stats.total_requests > 0 {
            stats.blocked_requests as f64 / stats.total_requests as f64
        } else {
            0.0
        };
        
        let requests_per_second = if uptime_seconds > 0 {
            stats.total_requests as f64 / uptime_seconds as f64
        } else {
            0.0
        };
        
        let health_status = if stats.last_error.is_some() {
            HealthStatus::Error
        } else if self.config.enabled {
            HealthStatus::Healthy
        } else {
            HealthStatus::Disabled
        };
        
        ServiceHealth {
            status: health_status.clone(),
            is_healthy: matches!(health_status, HealthStatus::Healthy),
            last_checked: Utc::now(),
            error_message: stats.last_error.clone(),
            uptime_seconds,
            performance: PerformanceMetrics {
                avg_response_time_ms,
                requests_per_second,
                error_rate,
                memory_usage_bytes: 0, // Would need actual memory tracking
            },
        }
    }
    
    async fn reset_statistics(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self.stats.lock() {
            Ok(mut stats) => {
                *stats = SanitizationStats {
                    start_time: Utc::now(),
                    total_requests: 0,
                    sanitized_requests: 0,
                    blocked_requests: 0,
                    alerts_generated: 0,
                    secrets_detected: 0,
                    policy_triggers: HashMap::new(),
                    last_error: None,
                    total_processing_time_ms: 0,
                };
                Ok(())
            }
            Err(_) => Err("Mutex poisoned".into())
        }
    }
}

impl HealthMonitor for SanitizationService {
    async fn is_healthy(&self) -> bool {
        self.config.enabled && 
        self.stats.lock().map(|stats| stats.last_error.is_none()).unwrap_or(false)
    }
    
    async fn health_check(&self) -> ServiceHealth {
        self.get_health().await
    }
    
    fn get_uptime(&self) -> u64 {
        self.stats.lock()
            .map(|stats| (Utc::now() - stats.start_time).num_seconds() as u64)
            .unwrap_or(0)
    }
}