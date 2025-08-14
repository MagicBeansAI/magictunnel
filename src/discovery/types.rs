//! Types for Smart Tool Discovery
//!
//! This module defines the core types used in the Smart Tool Discovery system,
//! including request/response structures, error types, and configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::registry::types::ToolDefinition;
use crate::mcp::types::elicitation::ElicitationRequest;

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

/// Enhanced tool definition with sampling and elicitation capabilities
/// 
/// This extends the base ToolDefinition with MCP 2025-06-18 sampling/elicitation
/// enhancements for improved smart discovery ranking and parameter collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedToolDefinition {
    /// Base tool definition (original from registry)
    pub base: ToolDefinition,
    
    /// Enhanced description generated via LLM (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_enhanced_description: Option<String>,
    
    /// Additional metadata generated via elicitation (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation_metadata: Option<ElicitationMetadata>,
    
    /// Source of the enhancement (base, sampling, elicitation, both)
    pub enhancement_source: EnhancementSource,
    
    /// When this enhancement was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhanced_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Whether this tool has been approved for use (if enhancement requires approval)
    pub approved: bool,
    
    /// Enhancement generation metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhancement_metadata: Option<EnhancementGenerationMetadata>,
    
    /// Whether this tool comes from an external/remote MCP server
    pub is_external_mcp: bool,
    
    /// External MCP server source information (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_mcp_source: Option<ExternalMcpSource>,
    
    /// Last generation timestamp for tracking freshness
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_generated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Information about external MCP server source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMcpSource {
    /// External MCP server identifier
    pub server_id: String,
    
    /// External MCP server name/description
    pub server_name: Option<String>,
    
    /// Whether the external MCP server supports sampling
    pub supports_sampling: bool,
    
    /// Whether the external MCP server supports elicitation
    pub supports_elicitation: bool,
    
    /// Last time we checked the external MCP server capabilities
    pub last_capability_check: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Connection type (websocket, external_mcp, etc.)
    pub connection_type: String,
}

/// Additional metadata collected via elicitation for better tool matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationMetadata {
    /// Enhanced keywords for rule-based matching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhanced_keywords: Option<Vec<String>>,
    
    /// Enhanced categories or tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhanced_categories: Option<Vec<String>>,
    
    /// Usage patterns or common use cases
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_patterns: Option<Vec<String>>,
    
    /// Parameter help text for complex parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_help: Option<HashMap<String, String>>,
    
    /// Example values for parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_examples: Option<HashMap<String, Vec<serde_json::Value>>>,
    
    /// Elicitation requests for missing parameter collection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation_requests: Option<Vec<ElicitationRequest>>,
}

/// Source of tool enhancement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EnhancementSource {
    /// Base tool only (no enhancement)
    Base,
    /// Enhanced via LLM description generation only
    LlmDescription,
    /// Enhanced via elicitation only
    Elicitation,
    /// Enhanced via both LLM description and elicitation
    Both,
    /// Enhanced manually by user
    Manual,
    /// Enhanced via external MCP server capabilities
    External,
}

/// Metadata about how the enhancement was generated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementGenerationMetadata {
    /// Model used for LLM description enhancement (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
    
    /// Confidence score of the LLM description enhancement (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_confidence: Option<f64>,
    
    /// Template used for elicitation enhancement (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation_template: Option<String>,
    
    /// Whether this enhancement required human review
    pub required_review: bool,
    
    /// Who approved this enhancement (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    
    /// When this enhancement was approved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Generation time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_time_ms: Option<u64>,
}

impl EnhancedToolDefinition {
    /// Create a new enhanced tool definition from a base tool
    pub fn from_base(base: ToolDefinition) -> Self {
        // Check if this tool comes from external/remote MCP
        let is_external = Self::is_external_mcp_tool(&base);
        let external_source = if is_external {
            Some(Self::extract_external_mcp_source(&base))
        } else {
            None
        };
        
        Self {
            base,
            llm_enhanced_description: None,
            elicitation_metadata: None,
            enhancement_source: if is_external {
                EnhancementSource::External
            } else {
                EnhancementSource::Base
            },
            enhanced_at: None,
            approved: true, // Base tools are always approved
            enhancement_metadata: None,
            is_external_mcp: is_external,
            external_mcp_source: external_source,
            last_generated_at: None,
        }
    }
    
    /// Check if a tool comes from external/remote MCP server
    fn is_external_mcp_tool(tool: &ToolDefinition) -> bool {
        matches!(tool.routing.r#type.as_str(), "external_mcp" | "websocket")
    }
    
    /// Extract external MCP source information from tool definition
    fn extract_external_mcp_source(tool: &ToolDefinition) -> ExternalMcpSource {
        let server_id = tool.routing.config
            .get("server_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
            
        let server_name = tool.routing.config
            .get("server_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
            
        ExternalMcpSource {
            server_id,
            server_name,
            supports_sampling: false, // Will be detected by capability check
            supports_elicitation: false, // Will be detected by capability check
            last_capability_check: None,
            connection_type: tool.routing.r#type.clone(),
        }
    }
    
    /// Check if this tool should skip automatic LLM enhancement generation
    pub fn should_skip_llm_generation(&self) -> bool {
        self.is_external_mcp
    }
    
    /// Get the effective description (enhanced if available, otherwise base)
    pub fn effective_description(&self) -> &str {
        self.llm_enhanced_description
            .as_ref()
            .unwrap_or(&self.base.description)
    }
    
    /// Get enhanced keywords for rule-based matching
    pub fn effective_keywords(&self) -> Vec<String> {
        let mut keywords = vec![self.base.name.clone()];
        
        // Add keywords from enhanced metadata if available
        if let Some(metadata) = &self.elicitation_metadata {
            if let Some(enhanced_keywords) = &metadata.enhanced_keywords {
                keywords.extend(enhanced_keywords.clone());
            }
        }
        
        keywords
    }
    
    /// Check if this tool has any enhancements
    pub fn is_enhanced(&self) -> bool {
        self.enhancement_source != EnhancementSource::Base
    }
    
    /// Get enhancement summary for display
    pub fn enhancement_summary(&self) -> String {
        match &self.enhancement_source {
            EnhancementSource::Base => "Base tool definition".to_string(),
            EnhancementSource::LlmDescription => "Enhanced with AI-generated description".to_string(),
            EnhancementSource::Elicitation => "Enhanced with structured metadata".to_string(),
            EnhancementSource::Both => "Enhanced with AI description and structured metadata".to_string(),
            EnhancementSource::Manual => "Manually enhanced by user".to_string(),
            EnhancementSource::External => "Enhanced by external MCP server".to_string(),
        }
    }
}