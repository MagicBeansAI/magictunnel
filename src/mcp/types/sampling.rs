//! MCP Sampling types and structures
//!
//! Implements the sampling capability for MCP 2025-06-18 specification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sampling request for LLM message generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingRequest {
    /// Messages to be used as context for sampling
    pub messages: Vec<SamplingMessage>,
    /// Optional model preferences for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    /// Optional system prompt to guide sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Temperature for sampling (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Additional metadata for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Model preferences for sampling
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

/// Individual message in sampling conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingMessage {
    /// Role of the message sender
    pub role: SamplingMessageRole,
    /// Content of the message
    pub content: SamplingContent,
    /// Optional name for the message sender
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional metadata for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Role of a sampling message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SamplingMessageRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool/function call
    Tool,
}

/// Content types for sampling messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SamplingContent {
    /// Simple text content
    Text(String),
    /// Structured content with multiple parts
    Parts(Vec<SamplingContentPart>),
}

/// Individual content part for multimodal messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SamplingContentPart {
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

/// Response from sampling request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingResponse {
    /// Generated message from sampling
    pub message: SamplingMessage,
    /// Model used for sampling
    pub model: String,
    /// Stop reason for sampling
    pub stop_reason: SamplingStopReason,
    /// Usage statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<SamplingUsage>,
    /// Additional metadata from sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Reason why sampling stopped
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplingStopReason {
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

/// Usage statistics for sampling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingUsage {
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

/// Error in sampling operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingError {
    /// Error code
    pub code: SamplingErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Sampling error codes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SamplingErrorCode {
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
    /// Sampling was cancelled
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

impl std::fmt::Display for SamplingErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SamplingErrorCode::InvalidRequest => write!(f, "invalid_request"),
            SamplingErrorCode::ModelNotAvailable => write!(f, "model_not_available"),
            SamplingErrorCode::ContentFiltered => write!(f, "content_filtered"),
            SamplingErrorCode::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            SamplingErrorCode::QuotaExceeded => write!(f, "quota_exceeded"),
            SamplingErrorCode::SecurityViolation => write!(f, "security_violation"),
            SamplingErrorCode::InternalError => write!(f, "internal_error"),
            SamplingErrorCode::Cancelled => write!(f, "cancelled"),
            SamplingErrorCode::Timeout => write!(f, "timeout"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sampling_request_serialization() {
        let request = SamplingRequest {
            messages: vec![SamplingMessage {
                role: SamplingMessageRole::User,
                content: SamplingContent::Text("Hello".to_string()),
                name: None,
                metadata: None,
            }],
            model_preferences: Some(ModelPreferences::default()),
            system_prompt: Some("You are a helpful assistant".to_string()),
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop: None,
            metadata: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: SamplingRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request.messages.len(), deserialized.messages.len());
        assert_eq!(request.max_tokens, deserialized.max_tokens);
        assert_eq!(request.temperature, deserialized.temperature);
    }

    #[test]
    fn test_multimodal_content() {
        let content = SamplingContent::Parts(vec![
            SamplingContentPart::Text {
                text: "What do you see in this image?".to_string(),
            },
            SamplingContentPart::Image {
                source: ImageSource::Url {
                    url: "https://example.com/image.jpg".to_string(),
                },
                alt_text: Some("A sample image".to_string()),
            },
        ]);

        let json = serde_json::to_string(&content).unwrap();
        let deserialized: SamplingContent = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            SamplingContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    SamplingContentPart::Text { text } => {
                        assert_eq!(text, "What do you see in this image?");
                    }
                    _ => panic!("Expected text part"),
                }
            }
            _ => panic!("Expected parts content"),
        }
    }
}