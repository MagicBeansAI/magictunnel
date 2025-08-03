//! MCP Runtime Tool Description Validation and Security Sandboxing
//!
//! Provides comprehensive runtime validation of tool descriptions and security sandboxing
//! according to MCP 2025-06-18 specification with enhanced security measures.

use crate::error::{ProxyError, Result};
use crate::mcp::types::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use regex::Regex;

/// Runtime tool validator with security sandboxing
pub struct RuntimeToolValidator {
    /// Validation configuration
    config: ValidationConfig,
    /// Cached validation results
    validation_cache: Arc<RwLock<HashMap<String, CachedValidationResult>>>,
    /// Security sandbox policies
    sandbox_policies: Arc<RwLock<HashMap<String, SandboxPolicy>>>,
    /// Runtime validation statistics
    stats: Arc<RwLock<ValidationStats>>,
    /// Schema validators for different tool types
    schema_validators: HashMap<String, Arc<jsonschema::JSONSchema>>,
}

/// Configuration for runtime tool validation
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Enable runtime validation
    pub enabled: bool,
    /// Enable security sandboxing
    pub enable_sandboxing: bool,
    /// Maximum tool description size (bytes)
    pub max_description_size: usize,
    /// Maximum number of parameters per tool
    pub max_parameters: usize,
    /// Maximum parameter name length
    pub max_parameter_name_length: usize,
    /// Maximum parameter description length
    pub max_parameter_description_length: usize,
    /// Validation cache TTL (seconds)
    pub cache_ttl_seconds: u64,
    /// Enable strict JSON schema validation
    pub strict_schema_validation: bool,
    /// Security validation level
    pub security_level: SecurityLevel,
    /// Blocked tool name patterns
    pub blocked_tool_patterns: Vec<String>,
    /// Blocked parameter patterns
    pub blocked_parameter_patterns: Vec<String>,
    /// Required security headers
    pub required_security_headers: Vec<String>,
    /// Maximum execution timeout (seconds)
    pub max_execution_timeout_seconds: Option<u64>,
    /// Enable parameter sanitization
    pub enable_parameter_sanitization: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_sandboxing: true,
            max_description_size: 10240, // 10KB
            max_parameters: 50,
            max_parameter_name_length: 100,
            max_parameter_description_length: 500,
            cache_ttl_seconds: 3600, // 1 hour
            strict_schema_validation: true,
            security_level: SecurityLevel::High,
            blocked_tool_patterns: vec![
                r"(?i)(rm|del|delete).*-rf".to_string(),
                r"(?i)(format|fdisk|mkfs)".to_string(),
                r"(?i)(sudo|su)".to_string(),
                r"(?i)(passwd|useradd|userdel)".to_string(),
                r"(?i)(iptables|firewall)".to_string(),
                r"(?i)(exec|eval|system)".to_string(),
            ],
            blocked_parameter_patterns: vec![
                r"(?i)(password|passwd|secret|key|token)".to_string(),
                r"(?i)(api[_-]?key|auth[_-]?token)".to_string(),
                r"(?i)(private[_-]?key|ssh[_-]?key)".to_string(),
                r"(?i)(--force|--delete|--remove)".to_string(),
            ],
            required_security_headers: vec![
                "X-MCP-Security-Level".to_string(),
                "X-MCP-Tool-Validation".to_string(),
            ],
            max_execution_timeout_seconds: Some(300), // 5 minutes
            enable_parameter_sanitization: true,
        }
    }
}

/// Security validation levels
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SecurityLevel {
    /// Minimal security checks
    Low,
    /// Standard security validation
    Medium,
    /// Comprehensive security validation
    High,
    /// Maximum security with strict sandboxing
    Maximum,
}

/// Cached validation result
#[derive(Debug, Clone)]
struct CachedValidationResult {
    /// Validation result
    result: ValidationResult,
    /// Cache timestamp
    cached_at: Instant,
    /// TTL for this result
    ttl: Duration,
}

/// Tool validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the tool is valid
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<ValidationError>,
    /// Security warnings
    pub warnings: Vec<SecurityWarning>,
    /// Security classification
    pub security_classification: SecurityClassification,
    /// Sandbox policy recommendations
    pub sandbox_recommendations: Vec<SandboxRecommendation>,
    /// Validation metadata
    pub metadata: HashMap<String, Value>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Error severity
    pub severity: ErrorSeverity,
    /// Field path (if applicable)
    pub field_path: Option<String>,
    /// Suggested fix
    pub suggested_fix: Option<String>,
}

/// Security warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityWarning {
    /// Warning code
    pub code: String,
    /// Warning message
    pub message: String,
    /// Security impact level
    pub impact_level: SecurityImpact,
    /// Recommended action
    pub recommended_action: Option<String>,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical error
    Critical,
}

/// Security impact levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityImpact {
    /// Low impact
    Low,
    /// Medium impact
    Medium,
    /// High impact
    High,
    /// Critical impact
    Critical,
}

/// Security classification for tools
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum SecurityClassification {
    /// Safe for general use
    Safe,
    /// Requires basic validation
    Restricted,
    /// Requires elevated permissions
    Privileged,
    /// Potentially dangerous
    Dangerous,
    /// Blocked - not allowed
    Blocked,
}

/// Sandbox policy for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Policy ID
    pub id: String,
    /// Tool name pattern this policy applies to
    pub tool_pattern: String,
    /// Allowed operations
    pub allowed_operations: Vec<String>,
    /// Blocked operations
    pub blocked_operations: Vec<String>,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Network restrictions
    pub network_restrictions: NetworkRestrictions,
    /// File system restrictions
    pub filesystem_restrictions: FilesystemRestrictions,
    /// Environment restrictions
    pub environment_restrictions: EnvironmentRestrictions,
}

/// Resource limits for sandboxed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU time (seconds)
    pub max_cpu_time: Option<u64>,
    /// Maximum memory usage (bytes)
    pub max_memory: Option<u64>,
    /// Maximum disk usage (bytes)
    pub max_disk: Option<u64>,
    /// Maximum number of processes
    pub max_processes: Option<u32>,
    /// Maximum execution time (seconds)
    pub max_execution_time: Option<u64>,
}

/// Network access restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRestrictions {
    /// Allow network access
    pub allow_network: bool,
    /// Allowed domains
    pub allowed_domains: Vec<String>,
    /// Blocked domains
    pub blocked_domains: Vec<String>,
    /// Allowed ports
    pub allowed_ports: Vec<u16>,
    /// Blocked ports
    pub blocked_ports: Vec<u16>,
}

/// Filesystem access restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemRestrictions {
    /// Allow filesystem access
    pub allow_filesystem: bool,
    /// Allowed paths
    pub allowed_paths: Vec<String>,
    /// Blocked paths
    pub blocked_paths: Vec<String>,
    /// Read-only paths
    pub readonly_paths: Vec<String>,
    /// Maximum file size (bytes)
    pub max_file_size: Option<u64>,
}

/// Environment variable restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentRestrictions {
    /// Allowed environment variables
    pub allowed_env_vars: Vec<String>,
    /// Blocked environment variables
    pub blocked_env_vars: Vec<String>,
    /// Clear environment before execution
    pub clear_environment: bool,
}

/// Sandbox recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxRecommendation {
    /// Recommendation type
    pub recommendation_type: RecommendationType,
    /// Recommendation message
    pub message: String,
    /// Priority level
    pub priority: RecommendationPriority,
    /// Implementation details
    pub implementation: Option<Value>,
}

/// Types of sandbox recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    /// Resource limit recommendation
    ResourceLimit,
    /// Network restriction recommendation
    NetworkRestriction,
    /// Filesystem restriction recommendation
    FilesystemRestriction,
    /// Environment restriction recommendation
    EnvironmentRestriction,
    /// General security recommendation
    SecurityHardening,
}

/// Recommendation priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Validation statistics
#[derive(Debug, Default, Serialize, Clone)]
pub struct ValidationStats {
    /// Total validations performed
    pub total_validations: u64,
    /// Valid tools
    pub valid_tools: u64,
    /// Invalid tools
    pub invalid_tools: u64,
    /// Tools by security classification
    pub tools_by_classification: HashMap<String, u64>,
    /// Common validation errors
    pub common_errors: HashMap<String, u64>,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Average validation time (milliseconds)
    pub avg_validation_time_ms: f64,
}

impl RuntimeToolValidator {
    /// Create a new runtime tool validator
    pub fn new(config: ValidationConfig) -> Result<Self> {
        let mut validator = Self {
            config,
            validation_cache: Arc::new(RwLock::new(HashMap::new())),
            sandbox_policies: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ValidationStats::default())),
            schema_validators: HashMap::new(),
        };

        // Initialize built-in schema validators
        validator.initialize_schema_validators()?;

        Ok(validator)
    }

    /// Initialize schema validators for different tool types
    fn initialize_schema_validators(&mut self) -> Result<()> {
        // Basic tool schema
        let basic_tool_schema = json!({
            "type": "object",
            "required": ["name", "description"],
            "properties": {
                "name": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 100,
                    "pattern": "^[a-zA-Z0-9_-]+$"
                },
                "description": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 1000
                },
                "inputSchema": {
                    "type": "object"
                }
            },
            "additionalProperties": false
        });

        if let Ok(schema) = jsonschema::JSONSchema::compile(&basic_tool_schema) {
            self.schema_validators.insert("basic".to_string(), Arc::new(schema));
        }

        // Network tool schema (more restrictive)
        let network_tool_schema = json!({
            "type": "object",
            "required": ["name", "description", "security_level"],
            "properties": {
                "name": {
                    "type": "string",
                    "pattern": "^[a-zA-Z0-9_-]+$"
                },
                "description": {
                    "type": "string",
                    "minLength": 10
                },
                "security_level": {
                    "type": "string",
                    "enum": ["safe", "restricted", "privileged"]
                },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "format": "uri"
                        }
                    }
                }
            }
        });

        if let Ok(schema) = jsonschema::JSONSchema::compile(&network_tool_schema) {
            self.schema_validators.insert("network".to_string(), Arc::new(schema));
        }

        info!("Initialized {} schema validators", self.schema_validators.len());
        Ok(())
    }

    /// Initialize with default sandbox policies
    pub async fn initialize_with_policies(mut self) -> Result<Self> {
        self.load_default_sandbox_policies().await?;
        Ok(self)
    }

    /// Load default sandbox policies
    async fn load_default_sandbox_policies(&self) -> Result<()> {
        let mut policies = self.sandbox_policies.write().await;

        // Safe tools policy
        policies.insert("safe_tools".to_string(), SandboxPolicy {
            id: "safe_tools".to_string(),
            tool_pattern: r"^(echo|ls|pwd|date|uptime)$".to_string(),
            allowed_operations: vec!["read".to_string(), "query".to_string()],
            blocked_operations: vec!["write".to_string(), "delete".to_string(), "execute".to_string()],
            resource_limits: ResourceLimits {
                max_cpu_time: Some(5),
                max_memory: Some(10 * 1024 * 1024), // 10MB
                max_disk: Some(0), // No disk writes
                max_processes: Some(1),
                max_execution_time: Some(10),
            },
            network_restrictions: NetworkRestrictions {
                allow_network: false,
                allowed_domains: vec![],
                blocked_domains: vec!["*".to_string()],
                allowed_ports: vec![],
                blocked_ports: vec![],
            },
            filesystem_restrictions: FilesystemRestrictions {
                allow_filesystem: true,
                allowed_paths: vec!["/tmp".to_string(), "/var/tmp".to_string()],
                blocked_paths: vec!["/etc".to_string(), "/root".to_string(), "/home".to_string()],
                readonly_paths: vec!["/".to_string()],
                max_file_size: Some(1024 * 1024), // 1MB
            },
            environment_restrictions: EnvironmentRestrictions {
                allowed_env_vars: vec!["PATH".to_string(), "USER".to_string()],
                blocked_env_vars: vec!["AWS_SECRET_ACCESS_KEY".to_string(), "SSH_PRIVATE_KEY".to_string()],
                clear_environment: true,
            },
        });

        // Network tools policy
        policies.insert("network_tools".to_string(), SandboxPolicy {
            id: "network_tools".to_string(),
            tool_pattern: r"^(curl|wget|ping|nslookup|dig)$".to_string(),
            allowed_operations: vec!["read".to_string(), "network_request".to_string()],
            blocked_operations: vec!["write".to_string(), "delete".to_string(), "system_execute".to_string()],
            resource_limits: ResourceLimits {
                max_cpu_time: Some(30),
                max_memory: Some(50 * 1024 * 1024), // 50MB
                max_disk: Some(10 * 1024 * 1024), // 10MB
                max_processes: Some(3),
                max_execution_time: Some(60),
            },
            network_restrictions: NetworkRestrictions {
                allow_network: true,
                allowed_domains: vec!["*.com".to_string(), "*.org".to_string()],
                blocked_domains: vec!["localhost".to_string(), "127.0.0.1".to_string(), "10.*".to_string()],
                allowed_ports: vec![80, 443, 53],
                blocked_ports: vec![22, 23, 21, 3389],
            },
            filesystem_restrictions: FilesystemRestrictions {
                allow_filesystem: true,
                allowed_paths: vec!["/tmp".to_string(), "/var/tmp".to_string()],
                blocked_paths: vec!["/etc".to_string(), "/root".to_string()],
                readonly_paths: vec!["/usr".to_string(), "/bin".to_string()],
                max_file_size: Some(10 * 1024 * 1024), // 10MB
            },
            environment_restrictions: EnvironmentRestrictions {
                allowed_env_vars: vec!["PATH".to_string(), "HTTP_PROXY".to_string()],
                blocked_env_vars: vec!["AWS_SECRET_ACCESS_KEY".to_string()],
                clear_environment: false,
            },
        });

        info!("Loaded {} default sandbox policies", policies.len());
        Ok(())
    }

    /// Validate a tool description at runtime
    pub async fn validate_tool(&self, tool: &Tool) -> Result<ValidationResult> {
        if !self.config.enabled {
            return Ok(ValidationResult {
                valid: true,
                errors: vec![],
                warnings: vec![],
                security_classification: SecurityClassification::Safe,
                sandbox_recommendations: vec![],
                metadata: HashMap::new(),
            });
        }

        let start_time = Instant::now();
        let tool_id = format!("{}:{}", tool.name, self.calculate_tool_hash(tool));

        // Check cache first
        if let Some(cached) = self.check_cache(&tool_id).await {
            self.update_stats(true, start_time.elapsed()).await;
            return Ok(cached.result);
        }

        // Perform validation
        let mut result = ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
            security_classification: SecurityClassification::Safe,
            sandbox_recommendations: vec![],
            metadata: HashMap::new(),
        };

        // Basic structure validation
        self.validate_basic_structure(tool, &mut result).await;

        // Security validation
        self.validate_security(tool, &mut result).await;

        // Schema validation
        if self.config.strict_schema_validation {
            self.validate_schema(tool, &mut result).await;
        }

        // Parameter validation
        self.validate_parameters(tool, &mut result).await;

        // Generate sandbox recommendations
        self.generate_sandbox_recommendations(tool, &mut result).await;

        // Determine final validity
        result.valid = result.errors.iter().all(|e| !matches!(e.severity, ErrorSeverity::Error | ErrorSeverity::Critical));

        // Cache result
        self.cache_result(&tool_id, &result).await;

        // Update statistics
        self.update_stats(false, start_time.elapsed()).await;

        debug!("Validated tool '{}' - Valid: {}, Errors: {}, Warnings: {}", 
               tool.name, result.valid, result.errors.len(), result.warnings.len());

        Ok(result)
    }

    /// Validate basic tool structure
    async fn validate_basic_structure(&self, tool: &Tool, result: &mut ValidationResult) {
        // Check tool name
        if tool.name.is_empty() {
            result.errors.push(ValidationError {
                code: "EMPTY_TOOL_NAME".to_string(),
                message: "Tool name cannot be empty".to_string(),
                severity: ErrorSeverity::Critical,
                field_path: Some("name".to_string()),
                suggested_fix: Some("Provide a non-empty tool name".to_string()),
            });
        } else if tool.name.len() > 100 {
            result.errors.push(ValidationError {
                code: "TOOL_NAME_TOO_LONG".to_string(),
                message: format!("Tool name is {} characters, maximum is 100", tool.name.len()),
                severity: ErrorSeverity::Error,
                field_path: Some("name".to_string()),
                suggested_fix: Some("Shorten the tool name to 100 characters or less".to_string()),
            });
        }

        // Check tool description
        if tool.description.as_ref().map_or(true, |d| d.is_empty()) {
            result.errors.push(ValidationError {
                code: "EMPTY_DESCRIPTION".to_string(),
                message: "Tool description cannot be empty".to_string(),
                severity: ErrorSeverity::Error,
                field_path: Some("description".to_string()),
                suggested_fix: Some("Provide a meaningful description".to_string()),
            });
        } else if tool.description.as_ref().map_or(0, |d| d.len()) > self.config.max_description_size {
            result.errors.push(ValidationError {
                code: "DESCRIPTION_TOO_LONG".to_string(),
                message: format!("Description is {} bytes, maximum is {}", 
                                tool.description.as_ref().map_or(0, |d| d.len()), self.config.max_description_size),
                severity: ErrorSeverity::Warning,
                field_path: Some("description".to_string()),
                suggested_fix: Some("Shorten the description".to_string()),
            });
        }

        // Check input schema size
        if !tool.input_schema.is_null() {
            let input_schema = &tool.input_schema;
            let schema_str = serde_json::to_string(input_schema).unwrap_or_default();
            if schema_str.len() > self.config.max_description_size {
                result.errors.push(ValidationError {
                    code: "INPUT_SCHEMA_TOO_LARGE".to_string(),
                    message: format!("Input schema is {} bytes, maximum is {}", 
                                    schema_str.len(), self.config.max_description_size),
                    severity: ErrorSeverity::Warning,
                    field_path: Some("inputSchema".to_string()),
                    suggested_fix: Some("Simplify the input schema".to_string()),
                });
            }
        }
    }

    /// Validate tool security
    async fn validate_security(&self, tool: &Tool, result: &mut ValidationResult) {
        // Check against blocked tool patterns
        for pattern in &self.config.blocked_tool_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(&tool.name) {
                    result.security_classification = SecurityClassification::Blocked;
                    result.errors.push(ValidationError {
                        code: "BLOCKED_TOOL_PATTERN".to_string(),
                        message: format!("Tool name '{}' matches blocked pattern: {}", tool.name, pattern),
                        severity: ErrorSeverity::Critical,
                        field_path: Some("name".to_string()),
                        suggested_fix: Some("Use a different tool name that doesn't match security patterns".to_string()),
                    });
                }
            }
        }

        // Check description for security concerns
        let dangerous_keywords = [
            "system", "exec", "eval", "shell", "cmd", "command",
            "delete", "remove", "format", "destroy", "wipe",
            "password", "secret", "key", "token", "credential"
        ];

        for keyword in &dangerous_keywords {
            if tool.description.as_ref().map_or(false, |d| d.to_lowercase().contains(keyword)) {
                result.warnings.push(SecurityWarning {
                    code: "SECURITY_KEYWORD_DETECTED".to_string(),
                    message: format!("Tool description contains potentially dangerous keyword: '{}'", keyword),
                    impact_level: SecurityImpact::Medium,
                    recommended_action: Some("Review tool functionality for security implications".to_string()),
                });

                // Upgrade security classification
                if result.security_classification == SecurityClassification::Safe {
                    result.security_classification = SecurityClassification::Restricted;
                }
            }
        }

        // Classify tool based on name patterns
        if tool.name.contains("admin") || tool.name.contains("root") || tool.name.contains("sudo") {
            result.security_classification = SecurityClassification::Privileged;
            result.warnings.push(SecurityWarning {
                code: "PRIVILEGED_TOOL_DETECTED".to_string(),
                message: "Tool appears to require elevated privileges".to_string(),
                impact_level: SecurityImpact::High,
                recommended_action: Some("Ensure proper authorization before execution".to_string()),
            });
        }

        if tool.name.contains("network") || tool.name.contains("http") || tool.name.contains("curl") {
            if result.security_classification == SecurityClassification::Safe {
                result.security_classification = SecurityClassification::Restricted;
            }
            result.warnings.push(SecurityWarning {
                code: "NETWORK_TOOL_DETECTED".to_string(),
                message: "Tool appears to make network requests".to_string(),
                impact_level: SecurityImpact::Medium,
                recommended_action: Some("Apply network restrictions and monitoring".to_string()),
            });
        }
    }

    /// Validate tool schema
    async fn validate_schema(&self, tool: &Tool, result: &mut ValidationResult) {
        if !tool.input_schema.is_null() {
            let input_schema = &tool.input_schema;
            // Use appropriate schema validator based on tool type
            let validator_type = self.determine_validator_type(tool);
            
            if let Some(validator) = self.schema_validators.get(&validator_type) {
                if let Err(validation_errors) = validator.validate(input_schema) {
                    for error in validation_errors {
                        result.errors.push(ValidationError {
                            code: "SCHEMA_VALIDATION_ERROR".to_string(),
                            message: format!("Schema validation error: {}", error),
                            severity: ErrorSeverity::Error,
                            field_path: Some("inputSchema".to_string()),
                            suggested_fix: Some("Fix the JSON schema according to the error message".to_string()),
                        });
                    }
                }
            }

            // Check for recursive schemas (potential DoS)
            if self.is_recursive_schema(input_schema) {
                result.warnings.push(SecurityWarning {
                    code: "RECURSIVE_SCHEMA_DETECTED".to_string(),
                    message: "Schema appears to be recursive, which could cause performance issues".to_string(),
                    impact_level: SecurityImpact::Medium,
                    recommended_action: Some("Simplify schema to avoid deep recursion".to_string()),
                });
            }
        }
    }

    /// Validate tool parameters
    async fn validate_parameters(&self, tool: &Tool, result: &mut ValidationResult) {
        if !tool.input_schema.is_null() {
            let input_schema = &tool.input_schema;
            if let Some(properties) = input_schema.get("properties") {
                if let Some(props_obj) = properties.as_object() {
                    if props_obj.len() > self.config.max_parameters {
                        result.errors.push(ValidationError {
                            code: "TOO_MANY_PARAMETERS".to_string(),
                            message: format!("Tool has {} parameters, maximum is {}", 
                                           props_obj.len(), self.config.max_parameters),
                            severity: ErrorSeverity::Error,
                            field_path: Some("inputSchema.properties".to_string()),
                            suggested_fix: Some("Reduce the number of parameters".to_string()),
                        });
                    }

                    for (param_name, param_schema) in props_obj {
                        // Check parameter name length
                        if param_name.len() > self.config.max_parameter_name_length {
                            result.errors.push(ValidationError {
                                code: "PARAMETER_NAME_TOO_LONG".to_string(),
                                message: format!("Parameter name '{}' is {} characters, maximum is {}", 
                                               param_name, param_name.len(), self.config.max_parameter_name_length),
                                severity: ErrorSeverity::Warning,
                                field_path: Some(format!("inputSchema.properties.{}", param_name)),
                                suggested_fix: Some("Use a shorter parameter name".to_string()),
                            });
                        }

                        // Check for blocked parameter patterns
                        for pattern in &self.config.blocked_parameter_patterns {
                            if let Ok(regex) = Regex::new(pattern) {
                                if regex.is_match(param_name) {
                                    result.warnings.push(SecurityWarning {
                                        code: "BLOCKED_PARAMETER_PATTERN".to_string(),
                                        message: format!("Parameter name '{}' matches blocked pattern: {}", param_name, pattern),
                                        impact_level: SecurityImpact::High,
                                        recommended_action: Some("Use a different parameter name".to_string()),
                                    });
                                }
                            }
                        }

                        // Check parameter description length
                        if let Some(description) = param_schema.get("description") {
                            if let Some(desc_str) = description.as_str() {
                                if desc_str.len() > self.config.max_parameter_description_length {
                                    result.warnings.push(SecurityWarning {
                                        code: "PARAMETER_DESCRIPTION_TOO_LONG".to_string(),
                                        message: format!("Parameter '{}' description is {} characters, recommended maximum is {}", 
                                                       param_name, desc_str.len(), self.config.max_parameter_description_length),
                                        impact_level: SecurityImpact::Low,
                                        recommended_action: Some("Shorten parameter description".to_string()),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Generate sandbox recommendations based on tool analysis
    async fn generate_sandbox_recommendations(&self, tool: &Tool, result: &mut ValidationResult) {
        if !self.config.enable_sandboxing {
            return;
        }

        // Resource limit recommendations
        if result.security_classification >= SecurityClassification::Restricted {
            result.sandbox_recommendations.push(SandboxRecommendation {
                recommendation_type: RecommendationType::ResourceLimit,
                message: "Apply strict resource limits for restricted tool".to_string(),
                priority: RecommendationPriority::High,
                implementation: Some(json!({
                    "max_memory": 50 * 1024 * 1024, // 50MB
                    "max_cpu_time": 30,
                    "max_execution_time": 60
                })),
            });
        }

        // Network restrictions for network tools
        if tool.name.contains("curl") || tool.name.contains("wget") || tool.name.contains("http") {
            result.sandbox_recommendations.push(SandboxRecommendation {
                recommendation_type: RecommendationType::NetworkRestriction,
                message: "Apply network restrictions for network-enabled tool".to_string(),
                priority: RecommendationPriority::Medium,
                implementation: Some(json!({
                    "blocked_domains": ["localhost", "127.0.0.1", "10.*", "192.168.*"],
                    "allowed_ports": [80, 443],
                    "max_connections": 5
                })),
            });
        }

        // Filesystem restrictions for file tools
        if tool.name.contains("file") || tool.name.contains("read") || tool.name.contains("write") {
            result.sandbox_recommendations.push(SandboxRecommendation {
                recommendation_type: RecommendationType::FilesystemRestriction,
                message: "Apply filesystem restrictions for file-accessing tool".to_string(),
                priority: RecommendationPriority::Medium,
                implementation: Some(json!({
                    "allowed_paths": ["/tmp", "/var/tmp"],
                    "blocked_paths": ["/etc", "/root", "/home"],
                    "readonly_paths": ["/usr", "/bin"],
                    "max_file_size": 10 * 1024 * 1024 // 10MB
                })),
            });
        }

        // Environment restrictions for privileged tools
        if result.security_classification >= SecurityClassification::Privileged {
            result.sandbox_recommendations.push(SandboxRecommendation {
                recommendation_type: RecommendationType::EnvironmentRestriction,
                message: "Clear environment and restrict variables for privileged tool".to_string(),
                priority: RecommendationPriority::High,
                implementation: Some(json!({
                    "clear_environment": true,
                    "allowed_env_vars": ["PATH", "USER"],
                    "blocked_env_vars": ["*SECRET*", "*KEY*", "*TOKEN*"]
                })),
            });
        }
    }

    /// Get sandbox policy for a tool
    pub async fn get_sandbox_policy(&self, tool_name: &str) -> Option<SandboxPolicy> {
        let policies = self.sandbox_policies.read().await;
        
        for policy in policies.values() {
            if let Ok(regex) = Regex::new(&policy.tool_pattern) {
                if regex.is_match(tool_name) {
                    return Some(policy.clone());
                }
            }
        }
        
        None
    }

    /// Add custom sandbox policy
    pub async fn add_sandbox_policy(&self, policy: SandboxPolicy) -> Result<()> {
        let mut policies = self.sandbox_policies.write().await;
        policies.insert(policy.id.clone(), policy);
        Ok(())
    }

    /// Check validation cache
    async fn check_cache(&self, tool_id: &str) -> Option<CachedValidationResult> {
        let cache = self.validation_cache.read().await;
        if let Some(cached) = cache.get(tool_id) {
            if cached.cached_at.elapsed() < cached.ttl {
                return Some(cached.clone());
            }
        }
        None
    }

    /// Cache validation result
    async fn cache_result(&self, tool_id: &str, result: &ValidationResult) {
        let mut cache = self.validation_cache.write().await;
        cache.insert(tool_id.to_string(), CachedValidationResult {
            result: result.clone(),
            cached_at: Instant::now(),
            ttl: Duration::from_secs(self.config.cache_ttl_seconds),
        });

        // Clean old cache entries periodically
        if cache.len() > 1000 {
            let now = Instant::now();
            cache.retain(|_, cached| now.duration_since(cached.cached_at) < cached.ttl);
        }
    }

    /// Update validation statistics
    async fn update_stats(&self, cache_hit: bool, validation_time: Duration) {
        let mut stats = self.stats.write().await;
        stats.total_validations += 1;
        
        if cache_hit {
            // Update cache hit rate
            let total_hits = (stats.cache_hit_rate * (stats.total_validations - 1) as f64) + 1.0;
            stats.cache_hit_rate = total_hits / stats.total_validations as f64;
        } else {
            // Update average validation time
            let total_time = (stats.avg_validation_time_ms * (stats.total_validations - 1) as f64) + 
                            validation_time.as_millis() as f64;
            stats.avg_validation_time_ms = total_time / stats.total_validations as f64;
        }
    }

    /// Calculate tool hash for caching
    fn calculate_tool_hash(&self, tool: &Tool) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        tool.name.hash(&mut hasher);
        tool.description.hash(&mut hasher);
        if !tool.input_schema.is_null() {
            let schema = &tool.input_schema;
            serde_json::to_string(schema).unwrap_or_default().hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }

    /// Determine appropriate validator type for tool
    fn determine_validator_type(&self, tool: &Tool) -> String {
        if tool.name.contains("network") || tool.name.contains("http") || tool.name.contains("curl") {
            "network".to_string()
        } else {
            "basic".to_string()
        }
    }

    /// Check if schema is recursive
    fn is_recursive_schema(&self, schema: &Value) -> bool {
        fn check_recursive_refs(schema: &Value, path: &mut HashSet<String>) -> bool {
            match schema {
                Value::Object(obj) => {
                    if let Some(ref_val) = obj.get("$ref") {
                        if let Some(ref_str) = ref_val.as_str() {
                            if path.contains(ref_str) {
                                return true;
                            }
                            path.insert(ref_str.to_string());
                        }
                    }
                    
                    for (_, value) in obj {
                        if check_recursive_refs(value, path) {
                            return true;
                        }
                    }
                }
                Value::Array(arr) => {
                    for value in arr {
                        if check_recursive_refs(value, path) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
            false
        }
        
        let mut path = HashSet::new();
        check_recursive_refs(schema, &mut path)
    }

    /// Get validation statistics
    pub async fn get_stats(&self) -> ValidationStats {
        (*self.stats.read().await).clone()
    }

    /// Clear validation cache
    pub async fn clear_cache(&self) {
        let mut cache = self.validation_cache.write().await;
        cache.clear();
        info!("Validation cache cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_basic_tool_validation() {
        let validator = RuntimeToolValidator::new(ValidationConfig::default()).unwrap();
        
        let tool = Tool {
            name: "test_tool".to_string(),
            description: Some("A test tool for validation".to_string()),
            title: None,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test input"
                    }
                }
            }),
            output_schema: None,
            annotations: None,
        };

        let result = validator.validate_tool(&tool).await.unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_blocked_tool_validation() {
        let validator = RuntimeToolValidator::new(ValidationConfig::default()).unwrap();
        
        let tool = Tool {
            name: "rm_dangerous".to_string(),
            description: Some("A dangerous deletion tool".to_string()),
            title: None,
            input_schema: serde_json::Value::Null,
            output_schema: None,
            annotations: None,
        };

        let result = validator.validate_tool(&tool).await.unwrap();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.security_classification, SecurityClassification::Blocked);
    }

    #[tokio::test]
    async fn test_parameter_validation() {
        let validator = RuntimeToolValidator::new(ValidationConfig::default()).unwrap();
        
        let tool = Tool {
            name: "test_tool".to_string(),
            description: Some("Test tool".to_string()),
            title: None,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "password": {
                        "type": "string",
                        "description": "User password"
                    }
                }
            }),
            output_schema: None,
            annotations: None,
        };

        let result = validator.validate_tool(&tool).await.unwrap();
        assert!(!result.warnings.is_empty());
        
        // Should have warning about blocked parameter pattern
        let has_param_warning = result.warnings.iter()
            .any(|w| w.code == "BLOCKED_PARAMETER_PATTERN");
        assert!(has_param_warning);
    }

    #[tokio::test]
    async fn test_validation_caching() {
        let validator = RuntimeToolValidator::new(ValidationConfig::default()).unwrap();
        
        let tool = Tool {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            title: None,
            input_schema: serde_json::Value::Null,
            output_schema: None,
            annotations: None,
        };

        // First validation
        let start = Instant::now();
        let _result1 = validator.validate_tool(&tool).await.unwrap();
        let first_duration = start.elapsed();

        // Second validation (should be cached)
        let start = Instant::now();
        let _result2 = validator.validate_tool(&tool).await.unwrap();
        let second_duration = start.elapsed();

        // Second validation should be significantly faster due to caching
        assert!(second_duration < first_duration);
    }
}