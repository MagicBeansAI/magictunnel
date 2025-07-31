//! Request sanitization and content filtering for MagicTunnel
//!
//! Provides policies to sanitize and block MCP calls containing sensitive information,
//! similar to MCP Manager's content filtering capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;
use tracing::{debug, warn};

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

/// Request sanitization service
pub struct SanitizationService {
    config: SanitizationConfig,
    secret_patterns: HashMap<SecretType, Regex>,
    policy_patterns: HashMap<String, Vec<Regex>>,
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
        
        Ok(Self {
            config,
            secret_patterns,
            policy_patterns,
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