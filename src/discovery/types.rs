//! Types for Smart Tool Discovery
//!
//! This module defines the core types used in the Smart Tool Discovery system,
//! including request/response structures, error types, and configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request structure for smart tool discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartDiscoveryRequest {
    /// The natural language request describing what the user wants to accomplish
    pub request: String,
    
    /// Optional additional context to help with tool discovery and parameter mapping
    pub context: Option<String>,
    
    /// Optional list of preferred tool names to consider first
    pub preferred_tools: Option<Vec<String>>,
    
    /// Minimum confidence score (0.0-1.0) for tool selection (default: 0.7)
    pub confidence_threshold: Option<f64>,
    
    /// Whether to include detailed error information (default: false for progressive disclosure)
    pub include_error_details: Option<bool>,
    
    /// Enable smart sequential execution for multi-step tasks (default: true)
    pub sequential_mode: Option<bool>,
}

/// Response structure for smart tool discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartDiscoveryResponse {
    /// Whether the discovery and execution was successful
    pub success: bool,
    
    /// The actual tool execution result (if successful)
    pub data: Option<serde_json::Value>,
    
    /// Error message (if not successful)
    pub error: Option<String>,
    
    /// Brief, user-friendly error summary
    pub error_summary: Option<String>,
    
    /// Detailed error information (only shown when requested)
    pub error_details: Option<ErrorDetails>,
    
    /// Metadata about the discovery process
    pub metadata: SmartDiscoveryMetadata,
    
    /// Recommended next step (for sequential execution)
    pub next_step: Option<NextStepRecommendation>,
}

/// Recommendation for the next step in a sequential workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextStepRecommendation {
    /// Suggested natural language request for the next step
    pub suggested_request: String,
    
    /// Brief explanation of why this is the recommended next step
    pub reasoning: String,
    
    /// Potential parameters based on the current step's results
    pub potential_inputs: Option<serde_json::Value>,
    
    /// List of alternative next steps the user could consider
    pub alternatives: Option<Vec<String>>,
}

/// Detailed error information for progressive disclosure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Technical error details
    pub technical_details: Option<String>,
    
    /// Diagnostic information
    pub diagnostics: Option<serde_json::Value>,
    
    /// Stack trace or debug info
    pub debug_info: Option<String>,
    
    /// Instructions on how to get more help
    pub help_instructions: Option<String>,
}

/// Information about a tool candidate discovered during smart discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCandidateInfo {
    /// Name of the tool
    pub tool_name: String,
    
    /// Confidence score for this tool (0.0-1.0)
    pub confidence_score: f64,
    
    /// Reasoning for why this tool was considered
    pub reasoning: String,
    
    /// Whether this tool meets the confidence threshold
    pub meets_threshold: bool,
}

/// Metadata about the discovery process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartDiscoveryMetadata {
    /// Indicates this result was proxied via smart discovery
    pub proxied_via: String,
    
    /// The actual tool that was discovered and executed
    pub original_tool: Option<String>,
    
    /// Confidence score for the tool match (0.0-1.0)
    pub confidence_score: f64,
    
    /// Human-readable explanation of why this tool was selected
    pub reasoning: Option<String>,
    
    /// The parameters that were mapped from the natural language request
    pub mapped_parameters: Option<HashMap<String, serde_json::Value>>,
    
    /// Status of parameter extraction
    pub extraction_status: Option<String>,
    
    /// All tool candidates that were considered during discovery with their confidence scores
    pub tool_candidates: Option<Vec<ToolCandidateInfo>>,
}

/// Error response structure for failed discovery (backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicSmartDiscoveryError {
    /// Whether the discovery was successful
    pub success: bool,
    
    /// Error message
    pub error: String,
    
    /// Suggestions for the user
    pub suggestions: Option<Vec<String>>,
    
    /// Help information for parameter issues
    pub parameter_help: Option<ParameterHelp>,
    
    /// Disambiguation information for ambiguous requests
    pub disambiguation: Option<DisambiguationInfo>,
    
    /// Clarification request for missing information
    pub clarification_request: Option<ClarificationRequest>,
    
    /// Smart suggestions with corrected requests
    pub smart_suggestions: Option<Vec<SmartSuggestion>>,
    
    /// Metadata about the failed discovery attempt
    pub metadata: SmartDiscoveryMetadata,
}

/// Help information for parameter-related issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterHelp {
    /// List of missing required parameters
    pub missing_required: Option<Vec<String>>,
    
    /// Information about each parameter
    pub parameter_info: Option<HashMap<String, ParameterInfo>>,
}

/// Information about a specific parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    /// Parameter type (string, number, boolean, etc.)
    #[serde(rename = "type")]
    pub param_type: String,
    
    /// Description of what this parameter does
    pub description: String,
    
    /// Whether this parameter is required
    pub required: bool,
    
    /// Example values for this parameter
    pub examples: Option<Vec<String>>,
    
    /// Suggestions for how to provide this parameter
    pub suggestions: Option<Vec<String>>,
}

/// Disambiguation information for ambiguous requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisambiguationInfo {
    /// List of possible tools that could match
    pub possible_tools: Vec<PossibleTool>,
}

/// Information about a possible tool match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PossibleTool {
    /// Tool name
    pub tool: String,
    
    /// Description of what this tool does
    pub description: String,
    
    /// Example usage
    pub example: String,
}

/// Clarification request for missing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationRequest {
    /// Message explaining what additional information is needed
    pub message: String,
    
    /// Specific information that is missing
    pub missing_info: Vec<String>,
    
    /// Interactive questions to help the user provide the missing information
    pub questions: Vec<ClarificationQuestion>,
}

/// Interactive question for clarification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationQuestion {
    /// The parameter name being asked about
    pub parameter: String,
    
    /// User-friendly question text
    pub question: String,
    
    /// Type of input expected (text, number, boolean, choice)
    pub input_type: String,
    
    /// Possible choices (for choice type)
    pub choices: Option<Vec<String>>,
    
    /// Example values
    pub examples: Vec<String>,
    
    /// Whether this parameter is required
    pub required: bool,
}

/// Smart suggestion with corrected request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartSuggestion {
    /// The corrected/suggested request
    pub corrected_request: String,
    
    /// Explanation of why this correction was made
    pub reasoning: String,
}

/// Tool match result from the discovery process
#[derive(Debug, Clone)]
pub struct ToolMatch {
    /// The tool name that was matched
    pub tool_name: String,
    
    /// Confidence score for this match (0.0-1.0)
    pub confidence_score: f64,
    
    /// Reason why this tool was selected
    pub reasoning: String,
    
    /// Whether this match meets the confidence threshold
    pub meets_threshold: bool,
}

/// Result of parameter extraction from natural language
#[derive(Debug, Clone)]
pub struct ParameterExtraction {
    /// The extracted parameters
    pub parameters: HashMap<String, serde_json::Value>,
    
    /// Status of the extraction process
    pub status: ExtractionStatus,
    
    /// Any warnings or notes about the extraction
    pub warnings: Vec<String>,
    
    /// Parameters that were set to default values
    pub used_defaults: HashMap<String, serde_json::Value>,
}

/// Status of parameter extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionStatus {
    /// All parameters extracted successfully
    Success,
    
    /// Some parameters extracted, but some are missing
    Incomplete,
    
    /// Parameter extraction failed completely
    Failed,
}

impl Default for SmartDiscoveryMetadata {
    fn default() -> Self {
        Self {
            proxied_via: "smart_tool_discovery".to_string(),
            original_tool: None,
            confidence_score: 0.0,
            reasoning: None,
            mapped_parameters: None,
            extraction_status: None,
            tool_candidates: None,
        }
    }
}