//! MCP Elicitation types and structures
//!
//! Implements the elicitation capability for MCP 2025-06-18 specification
//! Elicitation allows servers to request structured data from users with schema validation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Elicitation request for structured data from user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationRequest {
    /// Human-readable message describing what data is needed
    pub message: String,
    /// JSON schema defining the structure of requested data
    pub requested_schema: serde_json::Value,
    /// Optional context information for the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ElicitationContext>,
    /// Optional timeout for the request in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u32>,
    /// Priority level for the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<ElicitationPriority>,
    /// Additional metadata for the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Context information for elicitation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationContext {
    /// Source tool or process requesting the data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Reason for the data request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Expected usage of the collected data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    /// Data retention policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention: Option<String>,
    /// Privacy level required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<ElicitationPrivacyLevel>,
}

/// Privacy level for elicitation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElicitationPrivacyLevel {
    /// Public data, no privacy concerns
    Public,
    /// Internal data, should be kept within organization
    Internal,
    /// Confidential data, requires special handling
    Confidential,
    /// Restricted data, highest level of protection
    Restricted,
}

/// Priority level for elicitation request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElicitationPriority {
    /// Low priority, can be delayed
    Low,
    /// Normal priority (default)
    Normal,
    /// High priority, should be handled quickly
    High,
    /// Urgent priority, immediate attention required
    Urgent,
}

/// Response to elicitation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationResponse {
    /// Action taken by the user
    pub action: ElicitationAction,
    /// Structured data provided by user (if action is Accept)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Optional reason for the action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Response metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Timestamp of the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Actions available in elicitation response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElicitationAction {
    /// User accepted and provided the requested data
    Accept,
    /// User declined to provide the data
    Decline,
    /// User cancelled the request
    Cancel,
}

/// Elicitation error for operation failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationError {
    /// Error code
    pub code: ElicitationErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Elicitation error codes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ElicitationErrorCode {
    /// Invalid request parameters
    InvalidRequest,
    /// Invalid schema provided
    InvalidSchema,
    /// Schema too complex (only flat objects allowed)
    SchemaTooComplex,
    /// Timeout waiting for user response
    Timeout,
    /// User declined to provide data
    UserDeclined,
    /// Request was cancelled
    Cancelled,
    /// Data validation failed
    ValidationFailed,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Security policy violation
    SecurityViolation,
    /// Internal server error
    InternalError,
}

/// Schema validation result
#[derive(Debug, Clone)]
pub struct SchemaValidationResult {
    /// Whether the schema is valid
    pub valid: bool,
    /// Validation errors if any
    pub errors: Vec<String>,
    /// Whether schema is a flat object (no nested objects)
    pub is_flat: bool,
    /// Detected schema complexity level
    pub complexity: SchemaComplexity,
}

/// Schema complexity levels
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SchemaComplexity {
    /// Simple flat object with primitive types only
    Simple,
    /// Contains arrays but no nested objects
    WithArrays,
    /// Contains nested objects (not allowed)
    Nested,
    /// Contains complex types or references
    Complex,
}

/// Data validation result
#[derive(Debug, Clone)]
pub struct DataValidationResult {
    /// Whether the data is valid against schema
    pub valid: bool,
    /// Validation errors if any
    pub errors: Vec<String>,
    /// Sanitized/cleaned data
    pub sanitized_data: Option<serde_json::Value>,
}

impl ElicitationRequest {
    /// Create a new elicitation request
    pub fn new(message: String, requested_schema: serde_json::Value) -> Self {
        Self {
            message,
            requested_schema,
            context: None,
            timeout_seconds: None,
            priority: Some(ElicitationPriority::Normal),
            metadata: None,
        }
    }

    /// Set context for the request
    pub fn with_context(mut self, context: ElicitationContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Set timeout for the request
    pub fn with_timeout(mut self, timeout_seconds: u32) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self
    }

    /// Set priority for the request
    pub fn with_priority(mut self, priority: ElicitationPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Add metadata to the request
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl ElicitationResponse {
    /// Create a new accept response with data
    pub fn accept(data: serde_json::Value) -> Self {
        Self {
            action: ElicitationAction::Accept,
            data: Some(data),
            reason: None,
            metadata: None,
            timestamp: Some(chrono::Utc::now()),
        }
    }

    /// Create a new decline response
    pub fn decline(reason: Option<String>) -> Self {
        Self {
            action: ElicitationAction::Decline,
            data: None,
            reason,
            metadata: None,
            timestamp: Some(chrono::Utc::now()),
        }
    }

    /// Create a new cancel response
    pub fn cancel(reason: Option<String>) -> Self {
        Self {
            action: ElicitationAction::Cancel,
            data: None,
            reason,
            metadata: None,
            timestamp: Some(chrono::Utc::now()),
        }
    }

    /// Add metadata to the response
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl ElicitationContext {
    /// Create a new elicitation context
    pub fn new() -> Self {
        Self {
            source: None,
            reason: None,
            usage: None,
            retention: None,
            privacy_level: Some(ElicitationPrivacyLevel::Internal),
        }
    }

    /// Set the source of the request
    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    /// Set the reason for the request
    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }

    /// Set the expected usage
    pub fn with_usage(mut self, usage: String) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Set the data retention policy
    pub fn with_retention(mut self, retention: String) -> Self {
        self.retention = Some(retention);
        self
    }

    /// Set privacy level
    pub fn with_privacy_level(mut self, privacy_level: ElicitationPrivacyLevel) -> Self {
        self.privacy_level = Some(privacy_level);
        self
    }
}

impl Default for ElicitationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ElicitationPriority {
    fn default() -> Self {
        ElicitationPriority::Normal
    }
}

impl Default for ElicitationPrivacyLevel {
    fn default() -> Self {
        ElicitationPrivacyLevel::Internal
    }
}

impl std::fmt::Display for ElicitationErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElicitationErrorCode::InvalidRequest => write!(f, "invalid_request"),
            ElicitationErrorCode::InvalidSchema => write!(f, "invalid_schema"),
            ElicitationErrorCode::SchemaTooComplex => write!(f, "schema_too_complex"),
            ElicitationErrorCode::Timeout => write!(f, "timeout"),
            ElicitationErrorCode::UserDeclined => write!(f, "user_declined"),
            ElicitationErrorCode::Cancelled => write!(f, "cancelled"),
            ElicitationErrorCode::ValidationFailed => write!(f, "validation_failed"),
            ElicitationErrorCode::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            ElicitationErrorCode::SecurityViolation => write!(f, "security_violation"),
            ElicitationErrorCode::InternalError => write!(f, "internal_error"),
        }
    }
}

impl std::fmt::Display for ElicitationAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElicitationAction::Accept => write!(f, "accept"),
            ElicitationAction::Decline => write!(f, "decline"),
            ElicitationAction::Cancel => write!(f, "cancel"),
        }
    }
}

impl std::error::Error for ElicitationError {}

impl std::fmt::Display for ElicitationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_elicitation_request_creation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        });

        let request = ElicitationRequest::new(
            "Please provide your name and age".to_string(),
            schema.clone()
        );

        assert_eq!(request.message, "Please provide your name and age");
        assert_eq!(request.requested_schema, schema);
        assert_eq!(request.priority, Some(ElicitationPriority::Normal));
    }

    #[test]
    fn test_elicitation_response_creation() {
        let data = json!({"name": "John", "age": 30});
        let response = ElicitationResponse::accept(data.clone());

        assert!(matches!(response.action, ElicitationAction::Accept));
        assert_eq!(response.data, Some(data));
        assert!(response.timestamp.is_some());
    }

    #[test]
    fn test_elicitation_decline() {
        let response = ElicitationResponse::decline(Some("Privacy concerns".to_string()));

        assert!(matches!(response.action, ElicitationAction::Decline));
        assert_eq!(response.data, None);
        assert_eq!(response.reason, Some("Privacy concerns".to_string()));
    }

    #[test]
    fn test_elicitation_context_builder() {
        let context = ElicitationContext::new()
            .with_source("user_profile_tool".to_string())
            .with_reason("Complete user registration".to_string())
            .with_privacy_level(ElicitationPrivacyLevel::Confidential);

        assert_eq!(context.source, Some("user_profile_tool".to_string()));
        assert_eq!(context.reason, Some("Complete user registration".to_string()));
        assert_eq!(context.privacy_level, Some(ElicitationPrivacyLevel::Confidential));
    }

    #[test]
    fn test_serialization() {
        let request = ElicitationRequest::new(
            "Test message".to_string(),
            json!({"type": "object"})
        );

        let json_str = serde_json::to_string(&request).unwrap();
        let deserialized: ElicitationRequest = serde_json::from_str(&json_str).unwrap();

        assert_eq!(request.message, deserialized.message);
        assert_eq!(request.requested_schema, deserialized.requested_schema);
    }
}