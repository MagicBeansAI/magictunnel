//! MCP Elicitation service implementation
//!
//! Handles elicitation requests for structured data collection according to MCP 2025-06-18 specification

use crate::config::Config;
use crate::error::Result;
use crate::mcp::types::elicitation::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use chrono::{DateTime, Utc};

/// Configuration for elicitation service
#[derive(Debug, Clone)]
pub struct ElicitationConfig {
    /// Whether elicitation is enabled
    pub enabled: bool,
    /// Maximum allowed schema complexity
    pub max_schema_complexity: SchemaComplexity,
    /// Default timeout for elicitation requests (seconds)
    pub default_timeout_seconds: u32,
    /// Maximum timeout allowed (seconds)
    pub max_timeout_seconds: u32,
    /// Rate limiting configuration
    pub rate_limit: Option<ElicitationRateLimit>,
    /// Security configuration
    pub security: ElicitationSecurityConfig,
    /// Schema validation configuration
    pub schema_validation: SchemaValidationConfig,
}

/// Rate limiting configuration for elicitation
#[derive(Debug, Clone)]
pub struct ElicitationRateLimit {
    /// Requests per minute per user
    pub requests_per_minute: u32,
    /// Burst size
    pub burst_size: u32,
    /// Window size in seconds
    pub window_seconds: u32,
}

/// Security configuration for elicitation
#[derive(Debug, Clone)]
pub struct ElicitationSecurityConfig {
    /// Whether to enable security checks
    pub enabled: bool,
    /// Blocked schema patterns (regex)
    pub blocked_schema_patterns: Vec<String>,
    /// Blocked field names
    pub blocked_field_names: Vec<String>,
    /// Maximum number of fields allowed
    pub max_fields: usize,
    /// Minimum privacy level required
    pub min_privacy_level: ElicitationPrivacyLevel,
    /// Whether to log all elicitation requests
    pub log_requests: bool,
}

/// Schema validation configuration
#[derive(Debug, Clone)]
pub struct SchemaValidationConfig {
    /// Whether to enable strict validation
    pub strict_validation: bool,
    /// Allowed JSON schema types
    pub allowed_types: Vec<String>,
    /// Maximum string length for fields
    pub max_string_length: usize,
    /// Maximum number for numeric fields
    pub max_number_value: f64,
    /// Minimum number for numeric fields
    pub min_number_value: f64,
}

/// Rate limiting state for users
#[derive(Debug, Clone)]
struct ElicitationRateLimitState {
    /// Request count in current window
    request_count: u32,
    /// Window start time
    window_start: DateTime<Utc>,
    /// Last request time
    last_request: DateTime<Utc>,
}

/// Pending elicitation request
#[derive(Debug, Clone)]
struct PendingElicitation {
    /// Original request
    request: ElicitationRequest,
    /// User ID who made the request
    user_id: Option<String>,
    /// Request timestamp
    created_at: DateTime<Utc>,
    /// Expiry time
    expires_at: DateTime<Utc>,
}

/// MCP Elicitation service
pub struct ElicitationService {
    /// Service configuration
    config: ElicitationConfig,
    /// Rate limiting state by user
    rate_limits: Arc<RwLock<HashMap<String, ElicitationRateLimitState>>>,
    /// Pending elicitation requests by ID
    pending_requests: Arc<RwLock<HashMap<String, PendingElicitation>>>,
    /// JSON schema validator (optional)
    schema_validator: Option<Arc<jsonschema::JSONSchema>>,
}

impl ElicitationService {
    /// Create a new elicitation service
    pub fn new(config: ElicitationConfig) -> Result<Self> {
        Ok(Self {
            config,
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            schema_validator: None,
        })
    }

    /// Create elicitation service from main config
    pub fn from_config(config: &Config) -> Result<Self> {
        let elicitation_config = ElicitationConfig {
            enabled: config.smart_discovery.as_ref()
                .map(|sd| sd.enabled)
                .unwrap_or(false),
            max_schema_complexity: SchemaComplexity::WithArrays,
            default_timeout_seconds: 300, // 5 minutes
            max_timeout_seconds: 1800, // 30 minutes
            rate_limit: Some(ElicitationRateLimit {
                requests_per_minute: 10,
                burst_size: 3,
                window_seconds: 60,
            }),
            security: ElicitationSecurityConfig {
                enabled: true,
                blocked_schema_patterns: vec![
                    r"(?i)(password|secret|key|token|credential)".to_string(),
                    r"(?i)(ssn|social.*security|credit.*card|bank.*account)".to_string(),
                    r"(?i)(private.*key|api.*key|auth.*token)".to_string(),
                ],
                blocked_field_names: vec![
                    "password".to_string(),
                    "secret".to_string(),
                    "private_key".to_string(),
                    "api_key".to_string(),
                    "auth_token".to_string(),
                    "ssn".to_string(),
                    "social_security_number".to_string(),
                    "credit_card".to_string(),
                    "bank_account".to_string(),
                ],
                max_fields: 20,
                min_privacy_level: ElicitationPrivacyLevel::Internal,
                log_requests: true,
            },
            schema_validation: SchemaValidationConfig {
                strict_validation: true,
                allowed_types: vec![
                    "string".to_string(),
                    "number".to_string(),
                    "integer".to_string(),
                    "boolean".to_string(),
                    "array".to_string(),
                ],
                max_string_length: 1000,
                max_number_value: 1e12,
                min_number_value: -1e12,
            },
        };

        Self::new(elicitation_config)
    }

    /// Handle elicitation request
    pub async fn handle_elicitation_request(
        &self,
        request: ElicitationRequest,
        user_id: Option<&str>,
    ) -> std::result::Result<String, ElicitationError> { // Returns request ID
        if !self.config.enabled {
            return Err(ElicitationError {
                code: ElicitationErrorCode::InvalidRequest,
                message: "Elicitation is not enabled".to_string(),
                details: None,
            });
        }

        // Check rate limits
        if let Some(user_id) = user_id {
            if let Err(e) = self.check_rate_limit(user_id).await {
                return Err(e);
            }
        }

        // Validate request
        if let Err(e) = self.validate_request(&request).await {
            return Err(e);
        }

        // Validate schema (always present in requested_schema)
        if let Err(e) = self.validate_schema(&request.requested_schema).await {
            return Err(e);
        }

        // Apply security checks
        if let Err(e) = self.apply_security_checks(&request).await {
            return Err(e);
        }

        // Generate request ID and store pending request
        let request_id = uuid::Uuid::new_v4().to_string();
        let timeout_seconds = request.timeout_seconds.unwrap_or(self.config.default_timeout_seconds);
        
        let pending = PendingElicitation {
            request: request.clone(),
            user_id: user_id.map(|s| s.to_string()),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::seconds(timeout_seconds as i64),
        };

        {
            let mut pending_requests = self.pending_requests.write().await;
            pending_requests.insert(request_id.clone(), pending);
        }

        // Log the request if configured
        if self.config.security.log_requests {
            info!("Elicitation request created: {} for user: {:?}", request_id, user_id);
            debug!("Elicitation request details: {:?}", request);
        }

        Ok(request_id)
    }

    /// Handle elicitation response
    pub async fn handle_elicitation_response(
        &self,
        request_id: &str,
        response: ElicitationResponse,
    ) -> std::result::Result<(), ElicitationError> {
        // Get pending request
        let pending = {
            let mut pending_requests = self.pending_requests.write().await;
            match pending_requests.remove(request_id) {
                Some(pending) => pending,
                None => {
                    return Err(ElicitationError {
                        code: ElicitationErrorCode::InvalidRequest,
                        message: format!("No pending elicitation request found with ID: {}", request_id),
                        details: None,
                    });
                }
            }
        };

        // Check if request has expired
        if Utc::now() > pending.expires_at {
            return Err(ElicitationError {
                code: ElicitationErrorCode::Timeout,
                message: "Elicitation request has expired".to_string(),
                details: None,
            });
        }

        // Validate response based on action
        match response.action {
            ElicitationAction::Accept => {
                if let Some(data) = &response.data {
                    // Validate data against schema (using requested_schema)
                    if let Err(e) = self.validate_data_against_schema(data, &pending.request.requested_schema).await {
                        return Err(e);
                    }
                } else {
                    return Err(ElicitationError {
                        code: ElicitationErrorCode::ValidationFailed,
                        message: "Accept response must include data".to_string(),
                        details: None,
                    });
                }
            }
            ElicitationAction::Decline | ElicitationAction::Cancel => {
                // No data validation needed for decline/cancel
            }
        }

        info!("Elicitation request {} completed with action: {}", request_id, response.action);
        Ok(())
    }

    /// Check rate limits for user
    async fn check_rate_limit(&self, user_id: &str) -> std::result::Result<(), ElicitationError> {
        if let Some(rate_limit) = &self.config.rate_limit {
            let mut limits = self.rate_limits.write().await;
            let now = Utc::now();
            
            let state = limits.entry(user_id.to_string()).or_insert(ElicitationRateLimitState {
                request_count: 0,
                window_start: now,
                last_request: now,
            });

            // Check if we need to reset the window
            let window_duration = chrono::Duration::seconds(rate_limit.window_seconds as i64);
            if now.signed_duration_since(state.window_start) > window_duration {
                state.request_count = 0;
                state.window_start = now;
            }

            // Check rate limit
            if state.request_count >= rate_limit.requests_per_minute {
                return Err(ElicitationError {
                    code: ElicitationErrorCode::RateLimitExceeded,
                    message: format!("Rate limit exceeded: {} requests per {} seconds", 
                        rate_limit.requests_per_minute, rate_limit.window_seconds),
                    details: Some(json!({
                        "limit": rate_limit.requests_per_minute,
                        "window_seconds": rate_limit.window_seconds,
                        "reset_time": state.window_start + window_duration
                    }).as_object().unwrap().clone().into_iter().collect()),
                });
            }

            // Update state
            state.request_count += 1;
            state.last_request = now;
        }

        Ok(())
    }

    /// Validate elicitation request
    async fn validate_request(&self, request: &ElicitationRequest) -> std::result::Result<(), ElicitationError> {
        // Check message is not empty
        if request.message.trim().is_empty() {
            return Err(ElicitationError {
                code: ElicitationErrorCode::InvalidRequest,
                message: "Elicitation message cannot be empty".to_string(),
                details: None,
            });
        }

        // Check timeout
        if let Some(timeout) = request.timeout_seconds {
            if timeout > self.config.max_timeout_seconds {
                return Err(ElicitationError {
                    code: ElicitationErrorCode::InvalidRequest,
                    message: format!("Timeout {} exceeds maximum allowed {}", 
                        timeout, self.config.max_timeout_seconds),
                    details: None,
                });
            }
        }

        // Check privacy level if context is provided
        if let Some(context) = &request.context {
            if let Some(privacy_level) = &context.privacy_level {
                // Ensure privacy level meets minimum requirements
                match (&self.config.security.min_privacy_level, privacy_level) {
                    (ElicitationPrivacyLevel::Restricted, ElicitationPrivacyLevel::Public) |
                    (ElicitationPrivacyLevel::Restricted, ElicitationPrivacyLevel::Internal) |
                    (ElicitationPrivacyLevel::Restricted, ElicitationPrivacyLevel::Confidential) |
                    (ElicitationPrivacyLevel::Confidential, ElicitationPrivacyLevel::Public) |
                    (ElicitationPrivacyLevel::Confidential, ElicitationPrivacyLevel::Internal) |
                    (ElicitationPrivacyLevel::Internal, ElicitationPrivacyLevel::Public) => {
                        return Err(ElicitationError {
                            code: ElicitationErrorCode::SecurityViolation,
                            message: format!("Privacy level {:?} does not meet minimum requirement {:?}", 
                                privacy_level, self.config.security.min_privacy_level),
                            details: None,
                        });
                    }
                    _ => {} // Privacy level is acceptable
                }
            }
        }

        Ok(())
    }

    /// Validate JSON schema
    async fn validate_schema(&self, schema: &Value) -> std::result::Result<(), ElicitationError> {
        let validation_result = self.analyze_schema(schema);

        if !validation_result.valid {
            return Err(ElicitationError {
                code: ElicitationErrorCode::InvalidSchema,
                message: format!("Invalid schema: {}", validation_result.errors.join(", ")),
                details: Some(json!({
                    "errors": validation_result.errors
                }).as_object().unwrap().clone().into_iter().collect()),
            });
        }

        // Check complexity
        if validation_result.complexity > self.config.max_schema_complexity {
            return Err(ElicitationError {
                code: ElicitationErrorCode::SchemaTooComplex,
                message: format!("Schema complexity {:?} exceeds maximum allowed {:?}", 
                    validation_result.complexity, self.config.max_schema_complexity),
                details: None,
            });
        }

        // Check if schema is flat for MCP compliance
        if !validation_result.is_flat && self.config.max_schema_complexity == SchemaComplexity::Simple {
            return Err(ElicitationError {
                code: ElicitationErrorCode::SchemaTooComplex,
                message: "Only flat object schemas are allowed (no nested objects)".to_string(),
                details: None,
            });
        }

        Ok(())
    }

    /// Analyze schema complexity and structure
    fn analyze_schema(&self, schema: &Value) -> SchemaValidationResult {
        let mut errors = Vec::new();
        let mut is_flat = true;
        let mut complexity = SchemaComplexity::Simple;

        // Basic schema validation
        if !schema.is_object() {
            errors.push("Schema must be an object".to_string());
            return SchemaValidationResult {
                valid: false,
                errors,
                is_flat: false,
                complexity: SchemaComplexity::Complex,
            };
        }

        let schema_obj = schema.as_object().unwrap();

        // Check schema type
        if let Some(schema_type) = schema_obj.get("type") {
            if let Some(type_str) = schema_type.as_str() {
                if type_str != "object" {
                    errors.push("Root schema type must be 'object'".to_string());
                }
            }
        }

        // Analyze properties
        if let Some(properties) = schema_obj.get("properties") {
            if let Some(props_obj) = properties.as_object() {
                if props_obj.len() > self.config.security.max_fields {
                    errors.push(format!("Too many properties: {} (max: {})", 
                        props_obj.len(), self.config.security.max_fields));
                }

                for (field_name, field_schema) in props_obj {
                    // Check for blocked field names
                    if self.config.security.blocked_field_names.contains(field_name) {
                        errors.push(format!("Blocked field name: {}", field_name));
                    }

                    // Analyze field schema
                    if let Some(field_obj) = field_schema.as_object() {
                        if let Some(field_type) = field_obj.get("type") {
                            if let Some(type_str) = field_type.as_str() {
                                match type_str {
                                    "object" => {
                                        is_flat = false;
                                        complexity = SchemaComplexity::Nested;
                                    }
                                    "array" => {
                                        if complexity == SchemaComplexity::Simple {
                                            complexity = SchemaComplexity::WithArrays;
                                        }
                                        // Check array items
                                        if let Some(items) = field_obj.get("items") {
                                            if let Some(items_obj) = items.as_object() {
                                                if let Some(items_type) = items_obj.get("type") {
                                                    if let Some(items_type_str) = items_type.as_str() {
                                                        if items_type_str == "object" {
                                                            is_flat = false;
                                                            complexity = SchemaComplexity::Nested;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        // Primitive types - check if allowed
                                        if !self.config.schema_validation.allowed_types.contains(&type_str.to_string()) {
                                            errors.push(format!("Type '{}' not allowed", type_str));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        SchemaValidationResult {
            valid: errors.is_empty(),
            errors,
            is_flat,
            complexity,
        }
    }

    /// Apply security checks to elicitation request
    async fn apply_security_checks(&self, request: &ElicitationRequest) -> std::result::Result<(), ElicitationError> {
        if !self.config.security.enabled {
            return Ok(());
        }

        // Check schema patterns
        let schema_str = serde_json::to_string(&request.requested_schema).unwrap_or_default();
        for pattern in &self.config.security.blocked_schema_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(&schema_str) {
                    return Err(ElicitationError {
                        code: ElicitationErrorCode::SecurityViolation,
                        message: "Requested schema contains blocked patterns".to_string(),
                        details: Some(json!({
                            "pattern": pattern
                        }).as_object().unwrap().clone().into_iter().collect()),
                    });
                }
            }
        }

        // Check message content
        for pattern in &self.config.security.blocked_schema_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(&request.message) {
                    return Err(ElicitationError {
                        code: ElicitationErrorCode::SecurityViolation,
                        message: "Message contains blocked patterns".to_string(),
                        details: Some(json!({
                            "pattern": pattern
                        }).as_object().unwrap().clone().into_iter().collect()),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate user data against schema
    async fn validate_data_against_schema(
        &self,
        data: &Value,
        schema: &Value,
    ) -> std::result::Result<(), ElicitationError> {
        // Basic validation - check if data matches expected structure
        if let Some(schema_obj) = schema.as_object() {
            if let Some(properties) = schema_obj.get("properties") {
                if let Some(props_obj) = properties.as_object() {
                    if let Some(data_obj) = data.as_object() {
                        // Check required fields
                        if let Some(required) = schema_obj.get("required") {
                            if let Some(required_array) = required.as_array() {
                                for required_field in required_array {
                                    if let Some(field_name) = required_field.as_str() {
                                        if !data_obj.contains_key(field_name) {
                                            return Err(ElicitationError {
                                                code: ElicitationErrorCode::ValidationFailed,
                                                message: format!("Required field '{}' is missing", field_name),
                                                details: None,
                                            });
                                        }
                                    }
                                }
                            }
                        }

                        // Validate field types
                        for (field_name, field_value) in data_obj {
                            if let Some(field_schema) = props_obj.get(field_name) {
                                if let Err(e) = self.validate_field_value(field_name, field_value, field_schema) {
                                    return Err(e);
                                }
                            }
                        }
                    } else {
                        return Err(ElicitationError {
                            code: ElicitationErrorCode::ValidationFailed,
                            message: "Data must be an object".to_string(),
                            details: None,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate individual field value against its schema
    fn validate_field_value(
        &self,
        field_name: &str,
        value: &Value,
        field_schema: &Value,
    ) -> std::result::Result<(), ElicitationError> {
        if let Some(schema_obj) = field_schema.as_object() {
            if let Some(field_type) = schema_obj.get("type") {
                if let Some(type_str) = field_type.as_str() {
                    let valid = match type_str {
                        "string" => {
                            if let Some(s) = value.as_str() {
                                s.len() <= self.config.schema_validation.max_string_length
                            } else {
                                false
                            }
                        }
                        "number" => {
                            if let Some(n) = value.as_f64() {
                                n >= self.config.schema_validation.min_number_value &&
                                n <= self.config.schema_validation.max_number_value
                            } else {
                                false
                            }
                        }
                        "integer" => {
                            if let Some(n) = value.as_i64() {
                                n as f64 >= self.config.schema_validation.min_number_value &&
                                (n as f64) <= self.config.schema_validation.max_number_value
                            } else {
                                false
                            }
                        }
                        "boolean" => value.is_boolean(),
                        "array" => value.is_array(),
                        _ => false,
                    };

                    if !valid {
                        return Err(ElicitationError {
                            code: ElicitationErrorCode::ValidationFailed,
                            message: format!("Field '{}' has invalid type or value", field_name),
                            details: Some(json!({
                                "field": field_name,
                                "expected_type": type_str,
                                "actual_value": value
                            }).as_object().unwrap().clone().into_iter().collect()),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Get pending elicitation requests for a user
    pub async fn get_pending_requests(&self, user_id: Option<&str>) -> Vec<(String, ElicitationRequest)> {
        let pending_requests = self.pending_requests.read().await;
        let now = Utc::now();

        pending_requests
            .iter()
            .filter(|(_, pending)| {
                // Filter by user and check expiry
                now <= pending.expires_at &&
                (user_id.is_none() || pending.user_id.as_deref() == user_id)
            })
            .map(|(id, pending)| (id.clone(), pending.request.clone()))
            .collect()
    }

    /// Clean up expired requests
    pub async fn cleanup_expired_requests(&self) {
        let mut pending_requests = self.pending_requests.write().await;
        let now = Utc::now();
        
        let expired_ids: Vec<String> = pending_requests
            .iter()
            .filter(|(_, pending)| now > pending.expires_at)
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired_ids {
            pending_requests.remove(&id);
            debug!("Cleaned up expired elicitation request: {}", id);
        }
    }

    /// Generate elicitation request for missing tool parameters (server-side generation)
    pub async fn generate_parameter_elicitation_request(
        &self,
        tool_name: &str,
        tool_description: &str,
        tool_schema: &Value,
        missing_parameters: &[String],
        user_request: &str,
    ) -> std::result::Result<ElicitationRequest, ElicitationError> {
        if !self.config.enabled {
            return Err(ElicitationError {
                code: ElicitationErrorCode::InvalidRequest,
                message: "Elicitation is not enabled".to_string(),
                details: None,
            });
        }

        info!("📝 Generating elicitation request for missing parameters: {} (missing: {:?})", 
              tool_name, missing_parameters);

        // Create structured schema for missing parameters only
        let elicitation_schema = self.create_parameter_schema(tool_schema, missing_parameters)?;

        // Create user-friendly message explaining what we need
        let message = if missing_parameters.len() == 1 {
            format!(
                "To execute the '{}' tool for your request \"{}\", \
                I need the following parameter: {}. \
                \n\
                Tool description: {} \
                \n\
                Please provide the missing parameter.",
                tool_name, user_request, missing_parameters[0], tool_description
            )
        } else {
            format!(
                "To execute the '{}' tool for your request \"{}\", \
                I need the following parameters: {}. \
                \n\
                Tool description: {} \
                \n\
                Please provide the missing parameters.",
                tool_name, user_request, missing_parameters.join(", "), tool_description
            )
        };

        let mut request = ElicitationRequest::new(message, elicitation_schema);
        
        // Set appropriate timeout for parameter collection
        request.timeout_seconds = Some(self.config.default_timeout_seconds);
        
        // Add context for better UX
        request.context = Some(ElicitationContext::new()
            .with_source(format!("magictunnel_tool_{}", tool_name))
            .with_reason("parameter_collection".to_string())
            .with_usage(format!("Execute {} tool for user request: {}", tool_name, user_request))
            .with_retention("temporary".to_string())
            .with_privacy_level(ElicitationPrivacyLevel::Internal));
        
        // Add metadata
        request.metadata = Some(json!({
            "original_request": user_request,
            "missing_parameters": missing_parameters,
            "tool_description": tool_description,
            "generated_by": "magictunnel_elicitation_service",
            "timestamp": Utc::now().to_rfc3339()
        }).as_object().unwrap().clone().into_iter().collect());

        debug!("Generated elicitation request for '{}' missing parameters", tool_name);
        Ok(request)
    }

    /// Generate elicitation request for parameter validation and enhancement
    pub async fn generate_parameter_validation_request(
        &self,
        tool_name: &str,
        tool_schema: &Value,
        provided_parameters: &HashMap<String, Value>,
        validation_issues: &[String],
    ) -> std::result::Result<ElicitationRequest, ElicitationError> {
        if !self.config.enabled {
            return Err(ElicitationError {
                code: ElicitationErrorCode::InvalidRequest,
                message: "Elicitation is not enabled".to_string(),
                details: None,
            });
        }

        info!("⚙️ Generating elicitation request for parameter validation: {} (issues: {})", 
              tool_name, validation_issues.len());

        // Create schema for parameters that need validation/correction
        let elicitation_schema = self.create_validation_schema(tool_schema, provided_parameters, validation_issues)?;

        let message = format!(
            "The parameters provided for the '{}' tool have some issues that need to be resolved: \
            \n\
            Issues found: \
            {} \
            \n\
            Current parameters: {} \
            \n\
            Please provide corrected values for the problematic parameters.",
            tool_name,
            validation_issues.iter().enumerate()
                .map(|(i, issue)| format!("{}. {}", i + 1, issue))
                .collect::<Vec<_>>()
                .join("\n"),
            serde_json::to_string_pretty(provided_parameters).unwrap_or_default()
        );

        let mut request = ElicitationRequest::new(message, elicitation_schema);
        
        request.timeout_seconds = Some(self.config.default_timeout_seconds);
        request.context = Some(ElicitationContext::new()
            .with_source(format!("magictunnel_tool_{}", tool_name))
            .with_reason("parameter_validation".to_string())
            .with_usage(format!("Validate parameters for {} tool", tool_name))
            .with_retention("temporary".to_string())
            .with_privacy_level(ElicitationPrivacyLevel::Internal));
        
        // Add metadata
        request.metadata = Some(json!({
            "validation_issues": validation_issues,
            "provided_parameters": provided_parameters,
            "generated_by": "magictunnel_elicitation_service",
            "timestamp": Utc::now().to_rfc3339()
        }).as_object().unwrap().clone().into_iter().collect());

        debug!("Generated elicitation request for '{}' parameter validation", tool_name);
        Ok(request)
    }

    /// Generate elicitation request for tool capability discovery
    pub async fn generate_capability_discovery_request(
        &self,
        tool_name: &str,
        base_description: &str,
        discovery_areas: &[String],
    ) -> std::result::Result<ElicitationRequest, ElicitationError> {
        if !self.config.enabled {
            return Err(ElicitationError {
                code: ElicitationErrorCode::InvalidRequest,
                message: "Elicitation is not enabled".to_string(),
                details: None,
            });
        }

        info!("🔍 Generating elicitation request for capability discovery: {} (areas: {:?})", 
              tool_name, discovery_areas);

        // Create schema for capability information
        let elicitation_schema = self.create_capability_schema(discovery_areas)?;

        let message = format!(
            "Help enhance the '{}' tool by providing additional information about its capabilities. \
            \n\
            Current description: {} \
            \n\
            Please provide information about the following areas: {} \
            \n\
            This will help improve tool discovery and make it more useful for users.",
            tool_name,
            base_description,
            discovery_areas.join(", ")
        );

        let mut request = ElicitationRequest::new(message, elicitation_schema);
        
        request.timeout_seconds = Some(self.config.default_timeout_seconds * 2); // Longer timeout for discovery
        request.context = Some(ElicitationContext::new()
            .with_source(format!("magictunnel_tool_{}", tool_name))
            .with_reason("capability_discovery".to_string())
            .with_usage(format!("Enhance capabilities for {} tool", tool_name))
            .with_retention("long_term".to_string())
            .with_privacy_level(ElicitationPrivacyLevel::Internal));
        
        // Add metadata
        request.metadata = Some(json!({
            "discovery_areas": discovery_areas,
            "base_description": base_description,
            "generated_by": "magictunnel_elicitation_service",
            "timestamp": Utc::now().to_rfc3339()
        }).as_object().unwrap().clone().into_iter().collect());

        debug!("Generated elicitation request for '{}' capability discovery", tool_name);
        Ok(request)
    }

    /// Create JSON schema for missing parameters
    fn create_parameter_schema(
        &self,
        tool_schema: &Value,
        missing_parameters: &[String],
    ) -> std::result::Result<Value, ElicitationError> {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        if let Some(tool_properties) = tool_schema.get("properties") {
            if let Some(props_obj) = tool_properties.as_object() {
                for param_name in missing_parameters {
                    if let Some(param_schema) = props_obj.get(param_name) {
                        properties.insert(param_name.clone(), param_schema.clone());
                        
                        // Check if this parameter is required in the original schema
                        if tool_schema.get("required")
                            .and_then(|r| r.as_array())
                            .map(|arr| arr.iter().any(|v| v.as_str() == Some(param_name)))
                            .unwrap_or(false) {
                            required.push(Value::String(param_name.clone()));
                        }
                    }
                }
            }
        }

        let schema = json!({
            "type": "object",
            "properties": properties,
            "required": required
        });

        Ok(schema)
    }

    /// Create JSON schema for parameter validation
    fn create_validation_schema(
        &self,
        tool_schema: &Value,
        provided_parameters: &HashMap<String, Value>,
        validation_issues: &[String],
    ) -> std::result::Result<Value, ElicitationError> {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        // Extract parameter names from validation issues (simple heuristic)
        let problematic_params: Vec<String> = validation_issues.iter()
            .filter_map(|issue| {
                // Look for parameter names in validation messages
                provided_parameters.keys()
                    .find(|&param| issue.contains(param))
                    .map(|s| s.clone())
            })
            .collect();

        if let Some(tool_properties) = tool_schema.get("properties") {
            if let Some(props_obj) = tool_properties.as_object() {
                for param_name in &problematic_params {
                    if let Some(param_schema) = props_obj.get(param_name) {
                        properties.insert(param_name.clone(), param_schema.clone());
                        required.push(Value::String(param_name.clone()));
                    }
                }
            }
        }

        let schema = json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "description": "Corrected parameter values"
        });

        Ok(schema)
    }

    /// Create JSON schema for capability discovery
    fn create_capability_schema(
        &self,
        discovery_areas: &[String],
    ) -> std::result::Result<Value, ElicitationError> {
        let mut properties = serde_json::Map::new();

        for area in discovery_areas {
            let property_schema = match area.as_str() {
                "enhanced_keywords" => json!({
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Additional keywords that users might use to find this tool"
                }),
                "usage_patterns" => json!({
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Common usage patterns or scenarios for this tool"
                }),
                "parameter_examples" => json!({
                    "type": "object",
                    "additionalProperties": {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    "description": "Example values for each parameter"
                }),
                "enhanced_categories" => json!({
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Categories or tags for better tool classification"
                }),
                _ => json!({
                    "type": "string",
                    "description": format!("Information about {}", area)
                }),
            };
            
            properties.insert(area.clone(), property_schema);
        }

        let schema = json!({
            "type": "object",
            "properties": properties,
            "description": "Tool capability enhancement information"
        });

        Ok(schema)
    }

    /// Execute server-generated elicitation request (internal processing)
    pub async fn process_server_generated_request(
        &self,
        request: ElicitationRequest,
    ) -> std::result::Result<String, ElicitationError> {
        info!("🚀 Processing server-generated elicitation request");
        
        // Use internal processing without rate limiting for server-generated requests
        self.handle_elicitation_request_internal(request).await
    }

    /// Internal elicitation request handling (bypasses rate limiting for server requests)
    async fn handle_elicitation_request_internal(
        &self,
        request: ElicitationRequest,
    ) -> std::result::Result<String, ElicitationError> {
        // Use regular handler but without rate limiting for system requests
        self.handle_elicitation_request(request, Some("system")).await
    }

    /// Get service status
    pub async fn get_status(&self) -> Value {
        let pending_count = self.pending_requests.read().await.len();
        
        json!({
            "enabled": self.config.enabled,
            "pending_requests": pending_count,
            "max_schema_complexity": format!("{:?}", self.config.max_schema_complexity),
            "default_timeout_seconds": self.config.default_timeout_seconds,
            "server_side_generation": true,
            "supported_elicitations": [
                "parameter_collection",
                "parameter_validation",
                "capability_discovery"
            ],
            "rate_limit": self.config.rate_limit.as_ref().map(|rl| json!({
                "requests_per_minute": rl.requests_per_minute,
                "window_seconds": rl.window_seconds
            }))
        })
    }
}

impl Default for ElicitationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_schema_complexity: SchemaComplexity::Simple,
            default_timeout_seconds: 300,
            max_timeout_seconds: 1800,
            rate_limit: Some(ElicitationRateLimit {
                requests_per_minute: 10,
                burst_size: 3,
                window_seconds: 60,
            }),
            security: ElicitationSecurityConfig {
                enabled: true,
                blocked_schema_patterns: vec![],
                blocked_field_names: vec![],
                max_fields: 10,
                min_privacy_level: ElicitationPrivacyLevel::Internal,
                log_requests: true,
            },
            schema_validation: SchemaValidationConfig {
                strict_validation: true,
                allowed_types: vec![
                    "string".to_string(),
                    "number".to_string(),
                    "integer".to_string(),
                    "boolean".to_string(),
                ],
                max_string_length: 500,
                max_number_value: 1e6,
                min_number_value: -1e6,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_elicitation_service_creation() {
        let config = ElicitationConfig::default();
        let service = ElicitationService::new(config).unwrap();
        
        let status = service.get_status().await;
        assert_eq!(status["enabled"], false);
    }

    #[tokio::test]
    async fn test_schema_validation() {
        let config = ElicitationConfig::default();
        let service = ElicitationService::new(config).unwrap();

        // Valid flat schema
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        });

        let result = service.validate_schema(&schema).await;
        assert!(result.is_ok());

        // Invalid nested schema (when max complexity is Simple)
        let nested_schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    }
                }
            }
        });

        let result = service.validate_schema(&nested_schema).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_elicitation_request_handling() {
        let mut config = ElicitationConfig::default();
        config.enabled = true;
        let service = ElicitationService::new(config).unwrap();

        let request = ElicitationRequest::new(
            "Please provide your name".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "required": ["name"]
            })
        );

        let result = service.handle_elicitation_request(request, Some("test_user")).await;
        assert!(result.is_ok());
        
        let request_id = result.unwrap();
        assert!(!request_id.is_empty());
    }
}