//! MCP Tool Enhancement types and structures
//!
//! Implements tool enhancement capabilities for improving tool descriptions, keywords, and examples.
//! This was previously called "sampling" but has been renamed to avoid confusion with
//! true MCP sampling (server→client LLM requests) which is implemented separately.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool enhancement request for LLM-powered tool description improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEnhancementRequest {
    /// Messages to be used as context for enhancement
    pub messages: Vec<ToolEnhancementMessage>,
    /// Optional model preferences for enhancement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    /// Optional system prompt to guide enhancement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Temperature for enhancement (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p parameter for enhancement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences for enhancement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Additional metadata for enhancement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Model preferences for tool enhancement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreferences {
    /// Intelligence/quality priority (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence: Option<f32>,
    /// Speed priority (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    /// Cost efficiency priority (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f32>,
    /// Preferred model names or patterns
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_models: Option<Vec<String>>,
    /// Excluded model names or patterns
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_models: Option<Vec<String>>,
}

/// Individual message in tool enhancement conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEnhancementMessage {
    /// Role of the message sender
    pub role: ToolEnhancementMessageRole,
    /// Content of the message
    pub content: ToolEnhancementContent,
    /// Optional name for the message sender
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional metadata for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Role of a tool enhancement message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolEnhancementMessageRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool/function call
    Tool,
}

/// Content types for tool enhancement messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolEnhancementContent {
    /// Simple text content
    Text(String),
    /// Structured content with multiple parts
    Parts(Vec<ToolEnhancementContentPart>),
}

/// Individual content part for multimodal enhancement messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolEnhancementContentPart {
    /// Text content part
    #[serde(rename = "text")]
    Text {
        /// Text content
        text: String,
    },
    /// Image content part
    #[serde(rename = "image")]
    Image {
        /// Image data (base64) or URL
        source: ImageSource,
        /// Optional alt text for accessibility
        #[serde(skip_serializing_if = "Option::is_none")]
        alt_text: Option<String>,
    },
    /// Audio content part
    #[serde(rename = "audio")]
    Audio {
        /// Audio data (base64) or URL
        source: AudioSource,
        /// Audio format (wav, mp3, etc.)
        format: String,
        /// Optional transcript
        #[serde(skip_serializing_if = "Option::is_none")]
        transcript: Option<String>,
    },
}

/// Image source for content parts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImageSource {
    /// Base64 encoded image data
    Data {
        /// MIME type of the image
        media_type: String,
        /// Base64 encoded data
        data: String,
    },
    /// Image URL
    Url {
        /// Image URL
        url: String,
    },
}

/// Audio source for content parts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AudioSource {
    /// Base64 encoded audio data
    Data {
        /// MIME type of the audio
        media_type: String,
        /// Base64 encoded data
        data: String,
    },
    /// Audio URL
    Url {
        /// Audio URL
        url: String,
    },
}

/// Response from tool enhancement request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEnhancementResponse {
    /// Generated message from enhancement
    pub message: ToolEnhancementMessage,
    /// Model used for enhancement
    pub model: String,
    /// Stop reason for enhancement
    pub stop_reason: ToolEnhancementStopReason,
    /// Usage statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ToolEnhancementUsage>,
    /// Additional metadata from enhancement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Reason why tool enhancement stopped
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolEnhancementStopReason {
    /// Reached maximum tokens
    MaxTokens,
    /// Hit a stop sequence
    StopSequence,
    /// Model decided to stop naturally
    EndTurn,
    /// Tool/function call
    ToolCall,
    /// Content was filtered
    ContentFilter,
    /// Error occurred
    Error,
}

/// Usage statistics for tool enhancement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEnhancementUsage {
    /// Input tokens used
    pub input_tokens: u32,
    /// Output tokens generated
    pub output_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
    /// Cost in USD (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

/// Error in tool enhancement operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEnhancementError {
    /// Error code
    pub code: ToolEnhancementErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Tool enhancement error codes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolEnhancementErrorCode {
    /// Invalid request parameters
    InvalidRequest,
    /// Model not available
    ModelNotAvailable,
    /// Content was filtered/blocked
    ContentFiltered,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Insufficient quota/credits
    QuotaExceeded,
    /// Security violation detected
    SecurityViolation,
    /// Internal server error
    InternalError,
    /// Enhancement was cancelled
    Cancelled,
    /// Request timeout
    Timeout,
}

impl Default for ModelPreferences {
    fn default() -> Self {
        Self {
            intelligence: Some(0.7),
            speed: Some(0.5),
            cost: Some(0.3),
            preferred_models: None,
            excluded_models: None,
        }
    }
}

impl std::fmt::Display for ToolEnhancementErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolEnhancementErrorCode::InvalidRequest => write!(f, "invalid_request"),
            ToolEnhancementErrorCode::ModelNotAvailable => write!(f, "model_not_available"),
            ToolEnhancementErrorCode::ContentFiltered => write!(f, "content_filtered"),
            ToolEnhancementErrorCode::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            ToolEnhancementErrorCode::QuotaExceeded => write!(f, "quota_exceeded"),
            ToolEnhancementErrorCode::SecurityViolation => write!(f, "security_violation"),
            ToolEnhancementErrorCode::InternalError => write!(f, "internal_error"),
            ToolEnhancementErrorCode::Cancelled => write!(f, "cancelled"),
            ToolEnhancementErrorCode::Timeout => write!(f, "timeout"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_enhancement_request_serialization() {
        let request = ToolEnhancementRequest {
            messages: vec![ToolEnhancementMessage {
                role: ToolEnhancementMessageRole::User,
                content: ToolEnhancementContent::Text("Enhance this tool description".to_string()),
                name: None,
                metadata: None,
            }],
            model_preferences: Some(ModelPreferences::default()),
            system_prompt: Some("You are a tool enhancement expert".to_string()),
            max_tokens: Some(500),
            temperature: Some(0.3),
            top_p: Some(0.9),
            stop: None,
            metadata: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ToolEnhancementRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request.messages.len(), deserialized.messages.len());
        assert_eq!(request.max_tokens, deserialized.max_tokens);
        assert_eq!(request.temperature, deserialized.temperature);
    }

    #[test]
    fn test_multimodal_content() {
        let content = ToolEnhancementContent::Parts(vec![
            ToolEnhancementContentPart::Text {
                text: "Analyze this tool schema".to_string(),
            },
            ToolEnhancementContentPart::Image {
                source: ImageSource::Url {
                    url: "https://example.com/schema-diagram.jpg".to_string(),
                },
                alt_text: Some("Tool schema diagram".to_string()),
            },
        ]);

        let json = serde_json::to_string(&content).unwrap();
        let deserialized: ToolEnhancementContent = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            ToolEnhancementContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    ToolEnhancementContentPart::Text { text } => {
                        assert_eq!(text, "Analyze this tool schema");
                    }
                    _ => panic!("Expected text part"),
                }
            }
            _ => panic!("Expected parts content"),
        }
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(ToolEnhancementErrorCode::InvalidRequest.to_string(), "invalid_request");
        assert_eq!(ToolEnhancementErrorCode::ModelNotAvailable.to_string(), "model_not_available");
        assert_eq!(ToolEnhancementErrorCode::ContentFiltered.to_string(), "content_filtered");
    }
}