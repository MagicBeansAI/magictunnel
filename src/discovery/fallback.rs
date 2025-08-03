//! Fallback strategies for Smart Discovery Service
//!
//! This module provides fallback mechanisms when the primary smart discovery
//! process fails or returns insufficient results.

use crate::discovery::types::*;
use crate::error::{ProxyError, Result};
use crate::registry::types::ToolDefinition;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Fallback strategy configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FallbackConfig {
    /// Enable fallback strategies
    pub enabled: bool,
    /// Minimum confidence threshold for fallback
    pub min_confidence_threshold: f64,
    /// Maximum number of fallback suggestions
    pub max_fallback_suggestions: usize,
    /// Enable fuzzy matching fallback
    pub enable_fuzzy_fallback: bool,
    /// Enable keyword matching fallback
    pub enable_keyword_fallback: bool,
    /// Enable category-based fallback
    pub enable_category_fallback: bool,
    /// Enable partial match fallback
    pub enable_partial_match_fallback: bool,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_confidence_threshold: 0.3,
            max_fallback_suggestions: 5,
            enable_fuzzy_fallback: true,
            enable_keyword_fallback: true,
            enable_category_fallback: true,
            enable_partial_match_fallback: true,
        }
    }
}

/// Fallback strategy type
#[derive(Debug, Clone, PartialEq)]
pub enum FallbackStrategy {
    /// Fuzzy string matching
    FuzzyMatch,
    /// Keyword-based matching
    KeywordMatch,
    /// Category-based matching
    CategoryMatch,
    /// Partial name matching
    PartialMatch,
    /// Most popular tools
    PopularTools,
    /// Recently used tools
    RecentlyUsed,
    /// Similar tools based on usage patterns
    SimilarTools,
}

/// Fallback suggestion with strategy information
#[derive(Debug, Clone)]
pub struct FallbackSuggestion {
    /// Tool name
    pub tool_name: String,
    /// Confidence score for this suggestion
    pub confidence_score: f64,
    /// Fallback strategy used
    pub strategy: FallbackStrategy,
    /// Reasoning for this suggestion
    pub reasoning: String,
    /// Whether this meets the minimum threshold
    pub meets_threshold: bool,
}

/// Fallback result containing suggestions and metadata
#[derive(Debug, Clone)]
pub struct FallbackResult {
    /// List of fallback suggestions
    pub suggestions: Vec<FallbackSuggestion>,
    /// Total number of strategies attempted
    pub strategies_attempted: usize,
    /// Whether any suggestions meet the threshold
    pub has_viable_suggestions: bool,
    /// Error message if all strategies failed
    pub error_message: Option<String>,
}

/// Enhanced error information for smart discovery
#[derive(Debug, Clone)]
pub struct SmartDiscoveryError {
    /// Primary error that occurred
    pub primary_error: String,
    /// Error category
    pub category: ErrorCategory,
    /// Fallback result if available
    pub fallback_result: Option<FallbackResult>,
    /// User-friendly error message
    pub user_message: String,
    /// Suggested actions for the user
    pub suggested_actions: Vec<String>,
}

/// Error categories for better error handling
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    /// No tools found matching the request
    NoToolsFound,
    /// Tools found but confidence too low
    LowConfidence,
    /// LLM parameter extraction failed
    ParameterExtractionFailed,
    /// Tool execution failed
    ToolExecutionFailed,
    /// Configuration or system error
    SystemError,
    /// Network or connectivity error
    NetworkError,
    /// Authentication or authorization error
    AuthError,
    /// Rate limiting or quota exceeded
    RateLimitError,
    /// Tools found but constraint violations prevent usage
    ConstraintViolation,
}

/// Fallback strategy manager
pub struct FallbackManager {
    /// Configuration for fallback strategies
    config: FallbackConfig,
    /// Tool usage statistics for popularity-based fallback
    usage_stats: HashMap<String, u64>,
    /// Recently used tools for recency-based fallback
    recent_tools: Vec<String>,
    /// Common failure patterns for learning
    failure_patterns: HashMap<String, FailurePattern>,
}

/// Pattern of common failures for learning
#[derive(Debug, Clone)]
pub struct FailurePattern {
    /// The failed request pattern
    pub request_pattern: String,
    /// Number of times this pattern has failed
    pub failure_count: u64,
    /// Error categories that occurred
    pub error_categories: Vec<ErrorCategory>,
    /// Successful resolutions for this pattern
    pub successful_resolutions: Vec<String>,
    /// Suggested improvements based on learning
    pub learned_suggestions: Vec<String>,
}

impl FallbackManager {
    /// Create a new fallback manager
    pub fn new(config: FallbackConfig) -> Self {
        Self {
            config,
            usage_stats: HashMap::new(),
            recent_tools: Vec::new(),
            failure_patterns: HashMap::new(),
        }
    }

    /// Create a fallback manager with default configuration
    pub fn new_with_defaults() -> Self {
        Self::new(FallbackConfig::default())
    }

    /// Check if fallback is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Execute fallback strategies for a failed discovery request
    pub fn execute_fallback(
        &mut self,
        request: &SmartDiscoveryRequest,
        available_tools: &[(String, ToolDefinition)],
        primary_error: &str,
    ) -> FallbackResult {
        if !self.config.enabled {
            return FallbackResult {
                suggestions: Vec::new(),
                strategies_attempted: 0,
                has_viable_suggestions: false,
                error_message: Some("Fallback strategies are disabled".to_string()),
            };
        }

        debug!("Executing fallback strategies for request: {}", request.request);

        let mut suggestions = Vec::new();
        let mut strategies_attempted = 0;

        // Strategy 1: Fuzzy matching
        if self.config.enable_fuzzy_fallback {
            strategies_attempted += 1;
            let fuzzy_suggestions = self.fuzzy_match_fallback(request, available_tools);
            suggestions.extend(fuzzy_suggestions);
        }

        // Strategy 2: Keyword matching
        if self.config.enable_keyword_fallback {
            strategies_attempted += 1;
            let keyword_suggestions = self.keyword_match_fallback(request, available_tools);
            suggestions.extend(keyword_suggestions);
        }

        // Strategy 3: Category-based matching
        if self.config.enable_category_fallback {
            strategies_attempted += 1;
            let category_suggestions = self.category_match_fallback(request, available_tools);
            suggestions.extend(category_suggestions);
        }

        // Strategy 4: Partial matching
        if self.config.enable_partial_match_fallback {
            strategies_attempted += 1;
            let partial_suggestions = self.partial_match_fallback(request, available_tools);
            suggestions.extend(partial_suggestions);
        }

        // Strategy 5: Popular tools fallback
        let popular_suggestions = self.popular_tools_fallback(available_tools);
        suggestions.extend(popular_suggestions);
        strategies_attempted += 1;

        // Strategy 6: Recently used tools
        let recent_suggestions = self.recent_tools_fallback(available_tools);
        suggestions.extend(recent_suggestions);
        strategies_attempted += 1;

        // Remove duplicates and sort by confidence
        suggestions.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.dedup_by(|a, b| a.tool_name == b.tool_name);

        // Limit number of suggestions
        suggestions.truncate(self.config.max_fallback_suggestions);

        let has_viable_suggestions = suggestions.iter().any(|s| s.meets_threshold);

        let error_message = if suggestions.is_empty() {
            Some(format!("No fallback suggestions found after {} strategies", strategies_attempted))
        } else {
            None
        };

        info!("Fallback strategies generated {} suggestions from {} strategies", suggestions.len(), strategies_attempted);

        FallbackResult {
            suggestions,
            strategies_attempted,
            has_viable_suggestions,
            error_message,
        }
    }

    /// Fuzzy matching fallback strategy
    fn fuzzy_match_fallback(
        &self,
        request: &SmartDiscoveryRequest,
        available_tools: &[(String, ToolDefinition)],
    ) -> Vec<FallbackSuggestion> {
        let mut suggestions = Vec::new();
        let request_lower = request.request.to_lowercase();

        for (tool_name, tool_def) in available_tools {
            let tool_name_lower = tool_name.to_lowercase();
            let tool_desc_lower = tool_def.description.to_lowercase();

            // Simple fuzzy matching based on character similarity
            let name_similarity = self.calculate_fuzzy_similarity(&request_lower, &tool_name_lower);
            let desc_similarity = self.calculate_fuzzy_similarity(&request_lower, &tool_desc_lower);
            
            let confidence = (name_similarity * 0.7 + desc_similarity * 0.3).max(0.0).min(1.0);

            if confidence >= self.config.min_confidence_threshold {
                suggestions.push(FallbackSuggestion {
                    tool_name: tool_name.clone(),
                    confidence_score: confidence,
                    strategy: FallbackStrategy::FuzzyMatch,
                    reasoning: format!("Fuzzy match with {:.2} similarity", confidence),
                    meets_threshold: confidence >= self.config.min_confidence_threshold,
                });
            }
        }

        debug!("Fuzzy matching found {} suggestions", suggestions.len());
        suggestions
    }

    /// Keyword matching fallback strategy
    fn keyword_match_fallback(
        &self,
        request: &SmartDiscoveryRequest,
        available_tools: &[(String, ToolDefinition)],
    ) -> Vec<FallbackSuggestion> {
        let mut suggestions = Vec::new();
        let request_lower = request.request.to_lowercase();

        // Extract keywords from request
        let keywords = self.extract_keywords(&request_lower);

        for (tool_name, tool_def) in available_tools {
            let tool_name_lower = tool_name.to_lowercase();
            let tool_desc_lower = tool_def.description.to_lowercase();
            
            let mut matches = 0;
            let mut total_keywords = keywords.len();

            for keyword in &keywords {
                if tool_name_lower.contains(keyword) || tool_desc_lower.contains(keyword) {
                    matches += 1;
                }
            }

            if total_keywords > 0 && matches > 0 {
                let confidence = (matches as f64 / total_keywords as f64) * 0.8; // Cap at 0.8 for keyword matching
                
                if confidence >= self.config.min_confidence_threshold {
                    suggestions.push(FallbackSuggestion {
                        tool_name: tool_name.clone(),
                        confidence_score: confidence,
                        strategy: FallbackStrategy::KeywordMatch,
                        reasoning: format!("Keyword match: {}/{} keywords matched", matches, total_keywords),
                        meets_threshold: confidence >= self.config.min_confidence_threshold,
                    });
                }
            }
        }

        debug!("Keyword matching found {} suggestions", suggestions.len());
        suggestions
    }

    /// Category-based matching fallback strategy
    fn category_match_fallback(
        &self,
        request: &SmartDiscoveryRequest,
        available_tools: &[(String, ToolDefinition)],
    ) -> Vec<FallbackSuggestion> {
        let mut suggestions = Vec::new();
        let request_lower = request.request.to_lowercase();

        // Define category keywords
        let categories = vec![
            ("file", vec!["file", "read", "write", "save", "load", "document", "path"]),
            ("http", vec!["http", "api", "request", "web", "url", "fetch", "post", "get"]),
            ("database", vec!["database", "db", "sql", "query", "select", "insert", "update"]),
            ("search", vec!["search", "find", "lookup", "grep", "query", "filter"]),
            ("ai", vec!["ai", "llm", "generate", "chat", "completion", "prompt"]),
            ("system", vec!["system", "process", "execute", "run", "command"]),
        ];

        // Determine request category
        let mut request_category = None;
        for (category, keywords) in &categories {
            for keyword in keywords {
                if request_lower.contains(keyword) {
                    request_category = Some(*category);
                    break;
                }
            }
            if request_category.is_some() {
                break;
            }
        }

        if let Some(category) = request_category {
            let empty_keywords = vec![];
            let category_keywords = categories.iter()
                .find(|(cat, _)| cat == &category)
                .map(|(_, keywords)| keywords)
                .unwrap_or(&empty_keywords);

            for (tool_name, tool_def) in available_tools {
                let tool_name_lower = tool_name.to_lowercase();
                let tool_desc_lower = tool_def.description.to_lowercase();
                
                let mut matches = 0;
                for keyword in category_keywords {
                    if tool_name_lower.contains(keyword) || tool_desc_lower.contains(keyword) {
                        matches += 1;
                    }
                }

                if matches > 0 {
                    let confidence = (matches as f64 / category_keywords.len() as f64) * 0.6; // Cap at 0.6 for category matching
                    
                    if confidence >= self.config.min_confidence_threshold {
                        suggestions.push(FallbackSuggestion {
                            tool_name: tool_name.clone(),
                            confidence_score: confidence,
                            strategy: FallbackStrategy::CategoryMatch,
                            reasoning: format!("Category '{}' match with {} keywords", category, matches),
                            meets_threshold: confidence >= self.config.min_confidence_threshold,
                        });
                    }
                }
            }
        }

        debug!("Category matching found {} suggestions", suggestions.len());
        suggestions
    }

    /// Partial matching fallback strategy
    fn partial_match_fallback(
        &self,
        request: &SmartDiscoveryRequest,
        available_tools: &[(String, ToolDefinition)],
    ) -> Vec<FallbackSuggestion> {
        let mut suggestions = Vec::new();
        let request_lower = request.request.to_lowercase();

        // Split request into words
        let request_words: Vec<&str> = request_lower.split_whitespace().collect();

        for (tool_name, tool_def) in available_tools {
            let tool_name_lower = tool_name.to_lowercase();
            let tool_desc_lower = tool_def.description.to_lowercase();
            
            let mut matches = 0;
            for word in &request_words {
                if word.len() >= 3 { // Only consider words with 3+ characters
                    if tool_name_lower.contains(word) || tool_desc_lower.contains(word) {
                        matches += 1;
                    }
                }
            }

            if matches > 0 && request_words.len() > 0 {
                let confidence = (matches as f64 / request_words.len() as f64) * 0.5; // Cap at 0.5 for partial matching
                
                if confidence >= self.config.min_confidence_threshold {
                    suggestions.push(FallbackSuggestion {
                        tool_name: tool_name.clone(),
                        confidence_score: confidence,
                        strategy: FallbackStrategy::PartialMatch,
                        reasoning: format!("Partial match: {}/{} words matched", matches, request_words.len()),
                        meets_threshold: confidence >= self.config.min_confidence_threshold,
                    });
                }
            }
        }

        debug!("Partial matching found {} suggestions", suggestions.len());
        suggestions
    }

    /// Popular tools fallback strategy
    fn popular_tools_fallback(
        &self,
        available_tools: &[(String, ToolDefinition)],
    ) -> Vec<FallbackSuggestion> {
        let mut suggestions = Vec::new();

        // Get top 3 most popular tools
        let mut popular_tools: Vec<_> = available_tools
            .iter()
            .map(|(name, def)| (name, def, self.usage_stats.get(name).unwrap_or(&0)))
            .collect();

        popular_tools.sort_by(|a, b| b.2.cmp(a.2));

        for (tool_name, _tool_def, usage_count) in popular_tools.into_iter().take(3) {
            let confidence = if *usage_count > 0 {
                0.4 // Base confidence for popular tools
            } else {
                0.3 // Lower confidence for unused tools
            };

            if confidence >= self.config.min_confidence_threshold {
                suggestions.push(FallbackSuggestion {
                    tool_name: tool_name.clone(),
                    confidence_score: confidence,
                    strategy: FallbackStrategy::PopularTools,
                    reasoning: format!("Popular tool (used {} times)", usage_count),
                    meets_threshold: confidence >= self.config.min_confidence_threshold,
                });
            }
        }

        debug!("Popular tools fallback found {} suggestions", suggestions.len());
        suggestions
    }

    /// Recently used tools fallback strategy
    fn recent_tools_fallback(
        &self,
        available_tools: &[(String, ToolDefinition)],
    ) -> Vec<FallbackSuggestion> {
        let mut suggestions = Vec::new();

        // Get top 3 most recent tools
        for tool_name in self.recent_tools.iter().take(3) {
            if available_tools.iter().any(|(name, _)| name == tool_name) {
                suggestions.push(FallbackSuggestion {
                    tool_name: tool_name.clone(),
                    confidence_score: 0.35,
                    strategy: FallbackStrategy::RecentlyUsed,
                    reasoning: "Recently used tool".to_string(),
                    meets_threshold: 0.35 >= self.config.min_confidence_threshold,
                });
            }
        }

        debug!("Recently used tools fallback found {} suggestions", suggestions.len());
        suggestions
    }

    /// Calculate fuzzy similarity between two strings
    fn calculate_fuzzy_similarity(&self, s1: &str, s2: &str) -> f64 {
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }

        // Simple character-based similarity
        let s1_chars: std::collections::HashSet<char> = s1.chars().collect();
        let s2_chars: std::collections::HashSet<char> = s2.chars().collect();

        let intersection = s1_chars.intersection(&s2_chars).count();
        let union = s1_chars.union(&s2_chars).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Extract keywords from a text string
    fn extract_keywords(&self, text: &str) -> Vec<String> {
        // Simple keyword extraction - split on whitespace and filter out short/common words
        let stop_words = vec!["the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "i", "you", "he", "she", "it", "we", "they", "me", "him", "her", "us", "them", "my", "your", "his", "her", "its", "our", "their", "this", "that", "these", "those", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "can"];

        text.split_whitespace()
            .filter(|word| word.len() >= 3 && !stop_words.contains(word))
            .map(|word| word.to_string())
            .collect()
    }

    /// Record tool usage for popularity tracking
    pub fn record_tool_usage(&mut self, tool_name: &str) {
        *self.usage_stats.entry(tool_name.to_string()).or_insert(0) += 1;
        
        // Add to recent tools (remove if already exists to avoid duplicates)
        self.recent_tools.retain(|name| name != tool_name);
        self.recent_tools.insert(0, tool_name.to_string());
        
        // Keep only last 10 recent tools
        self.recent_tools.truncate(10);
    }
    
    /// Record a failure pattern for learning
    pub fn record_failure_pattern(
        &mut self,
        request: &str,
        error_category: ErrorCategory,
    ) {
        let pattern_key = self.extract_request_pattern(request);
        
        // Update the pattern data first
        {
            let pattern = self.failure_patterns.entry(pattern_key.clone()).or_insert_with(|| {
                FailurePattern {
                    request_pattern: pattern_key.clone(),
                    failure_count: 0,
                    error_categories: Vec::new(),
                    successful_resolutions: Vec::new(),
                    learned_suggestions: Vec::new(),
                }
            });
            
            pattern.failure_count += 1;
            if !pattern.error_categories.contains(&error_category) {
                pattern.error_categories.push(error_category.clone());
            }
        }
        
        // Now generate learned suggestions outside the mutable borrow scope
        let (error_categories, request_pattern) = {
            let pattern = &self.failure_patterns[&pattern_key];
            (pattern.error_categories.clone(), pattern.request_pattern.clone())
        };
        
        let learned_suggestions = self.generate_learned_suggestions(&error_categories, &request_pattern);
        
        // Update the learned suggestions
        if let Some(pattern) = self.failure_patterns.get_mut(&pattern_key) {
            pattern.learned_suggestions = learned_suggestions;
        }
    }
    
    /// Record a successful resolution for a request pattern
    pub fn record_successful_resolution(
        &mut self,
        original_request: &str,
        successful_request: &str,
        resolved_tool: &str,
    ) {
        let pattern_key = self.extract_request_pattern(original_request);
        
        if let Some(pattern) = self.failure_patterns.get_mut(&pattern_key) {
            let resolution = format!("'{}' → '{}' (tool: {})", original_request, successful_request, resolved_tool);
            if !pattern.successful_resolutions.contains(&resolution) {
                pattern.successful_resolutions.push(resolution);
                // Limit to last 5 successful resolutions
                pattern.successful_resolutions.truncate(5);
            }
        }
    }
    
    /// Extract a pattern from a request for learning purposes
    fn extract_request_pattern(&self, request: &str) -> String {
        let request_lower = request.to_lowercase();
        
        // Extract key words and concepts
        let key_words = vec![
            "read", "write", "search", "find", "create", "delete", "update",
            "file", "directory", "folder", "path",
            "http", "api", "request", "url", "web",
            "database", "db", "sql", "query",
            "ping", "network", "server", "host",
            "generate", "ai", "llm", "chat"
        ];
        
        let found_words: Vec<&str> = key_words.iter()
            .filter(|word| request_lower.contains(*word))
            .copied()
            .collect();
        
        if found_words.is_empty() {
            "generic_request".to_string()
        } else {
            found_words.join("_")
        }
    }
    
    /// Generate learned suggestions based on failure patterns
    fn generate_learned_suggestions(
        &self,
        error_categories: &[ErrorCategory],
        pattern: &str,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Pattern-specific suggestions
        if pattern.contains("file") {
            suggestions.push("Try: 'read the config.yaml file' instead of 'read file'".to_string());
            suggestions.push("Include the full file path: '/path/to/file.txt'".to_string());
        }
        
        if pattern.contains("search") {
            suggestions.push("Try: 'search for \"error\" in log files' instead of 'search logs'".to_string());
            suggestions.push("Include what you're searching for and where".to_string());
        }
        
        if pattern.contains("http") || pattern.contains("api") {
            suggestions.push("Try: 'make a GET request to https://api.example.com/data'".to_string());
            suggestions.push("Include the full URL and HTTP method".to_string());
        }
        
        // Error category specific suggestions
        for category in error_categories {
            match category {
                ErrorCategory::NoToolsFound => {
                    suggestions.push("Use more specific action words like 'read', 'write', 'search'".to_string());
                }
                ErrorCategory::LowConfidence => {
                    suggestions.push("Add more context about what you want to accomplish".to_string());
                }
                ErrorCategory::ParameterExtractionFailed => {
                    suggestions.push("Include specific values like file names, URLs, or search terms".to_string());
                }
                ErrorCategory::ConstraintViolation => {
                    suggestions.push("Look for tools without the specific limitations that prevented the match".to_string());
                    suggestions.push("Try a more general version of your request to avoid constraint conflicts".to_string());
                }
                _ => {}
            }
        }
        
        suggestions.truncate(3); // Keep top 3 suggestions
        suggestions
    }
    
    /// Get failure patterns for analysis
    pub fn get_failure_patterns(&self) -> &HashMap<String, FailurePattern> {
        &self.failure_patterns
    }
    
    /// Get learned suggestions for a request pattern
    pub fn get_learned_suggestions(&self, request: &str) -> Vec<String> {
        let pattern_key = self.extract_request_pattern(request);
        
        self.failure_patterns.get(&pattern_key)
            .map(|pattern| pattern.learned_suggestions.clone())
            .unwrap_or_default()
    }

    /// Create enhanced error information
    pub fn create_enhanced_error(
        &self,
        primary_error: &str,
        category: ErrorCategory,
        fallback_result: Option<FallbackResult>,
    ) -> SmartDiscoveryError {
        let (user_message, suggested_actions) = self.generate_user_guidance(&category, &fallback_result);

        SmartDiscoveryError {
            primary_error: primary_error.to_string(),
            category,
            fallback_result,
            user_message,
            suggested_actions,
        }
    }

    /// Generate user-friendly guidance based on error category
    fn generate_user_guidance(
        &self,
        category: &ErrorCategory,
        fallback_result: &Option<FallbackResult>,
    ) -> (String, Vec<String>) {
        let has_suggestions = fallback_result.as_ref()
            .map(|fr| !fr.suggestions.is_empty())
            .unwrap_or(false);

        match category {
            ErrorCategory::NoToolsFound => {
                let message = if has_suggestions {
                    "🔍 No exact matches found, but I discovered some similar tools that might help you accomplish your goal."
                } else {
                    "❌ I couldn't find any tools matching your request. Let me help you refine your search."
                };

                let actions = if has_suggestions {
                    vec![
                        "📋 Review the suggested alternatives below - one might be exactly what you need".to_string(),
                        "🔄 Try rephrasing your request using keywords from the suggested tools".to_string(),
                        "💡 Be more specific about your end goal (e.g., 'read a config file' vs 'process data')".to_string(),
                    ]
                } else {
                    vec![
                        "📝 Try describing what you want to accomplish in different words".to_string(),
                        "🎯 Be more specific about the action (read, write, search, create, etc.)".to_string(),
                        "🏷️ Include relevant keywords like 'file', 'database', 'API', 'web', etc.".to_string(),
                        "📖 Use simple, common terms rather than technical jargon".to_string(),
                    ]
                };

                (message.to_string(), actions)
            }
            ErrorCategory::LowConfidence => {
                let message = if has_suggestions {
                    "⚠️ I found some potential matches, but I'm not very confident they're what you're looking for. Please review these options carefully."
                } else {
                    "🤔 I found some tools that might be related, but none seem like a strong match for your request."
                };

                let actions = if has_suggestions {
                    vec![
                        "👀 Carefully review the tools listed below - their descriptions might clarify if they're useful".to_string(),
                        "✏️ Try rephrasing your request with more specific details about what you want to do".to_string(),
                        "📝 Add context about your goal (e.g., 'I want to analyze log files for errors')".to_string(),
                        "🎯 Lower your confidence threshold if you want to see more options".to_string(),
                        "💬 Answer the clarification questions below to help me understand better".to_string(),
                    ]
                } else {
                    vec![
                        "📋 Try browsing available tools to see what's possible".to_string(),
                        "🔍 Use more common, descriptive words in your request".to_string(),
                        "💭 Think about the core action you want to perform and mention it explicitly".to_string(),
                        "❓ Try answering: What do you want to accomplish? What type of data or files are involved?".to_string(),
                    ]
                };

                (message.to_string(), actions)
            }
            ErrorCategory::ParameterExtractionFailed => {
                let message = "✅ Good news! I found the right tool for you, but I need help understanding the specific details you want to use.";

                let actions = vec![
                    "📋 Include specific values in your request (e.g., file names, URLs, search terms)".to_string(),
                    "📖 Check what parameters this tool needs and mention them in your request".to_string(),
                    "💬 Try rephrasing like: 'use [tool] to [action] with [specific details]'".to_string(),
                    "🔍 Be more explicit about file paths, URLs, or other required values".to_string(),
                ];

                (message.to_string(), actions)
            }
            ErrorCategory::ToolExecutionFailed => {
                let message = "✅ I successfully found and configured the right tool, but something went wrong during execution.";

                let actions = vec![
                    "⚙️ Check that the tool's dependencies and services are properly configured".to_string(),
                    "🔄 Try the same request again - sometimes temporary issues resolve themselves".to_string(),
                    "🔧 Verify that any required external services or databases are running".to_string(),
                    "📝 Try adjusting your parameters slightly - the values might need refinement".to_string(),
                ];

                (message.to_string(), actions)
            }
            ErrorCategory::NetworkError => {
                let message = "🌐 I'm having trouble connecting to external services needed for your request.";

                let actions = vec![
                    "🔌 Check your internet connection is working properly".to_string(),
                    "⏳ Wait a few minutes and try again - external services may be temporarily down".to_string(),
                    "📶 If you're behind a firewall or VPN, verify that external API access is allowed".to_string(),
                    "👥 Contact your network administrator if this persists in a corporate environment".to_string(),
                ];

                (message.to_string(), actions)
            }
            ErrorCategory::SystemError => {
                let message = "🛠️ Something went wrong on my end while processing your request. This isn't your fault!";

                let actions = vec![
                    "🔄 Please try your request again - temporary issues often resolve themselves".to_string(),
                    "⏱️ If it fails again, wait a moment and retry - the system might be busy".to_string(),
                    "📝 Try rephrasing your request slightly in case there's a parsing issue".to_string(),
                    "🆘 If this keeps happening, please report it - there may be a system issue".to_string(),
                ];

                (message.to_string(), actions)
            }
            ErrorCategory::AuthError => {
                let message = "🔐 I found the right tool, but there's an authentication issue preventing me from using it.";

                let actions = vec![
                    "🔑 Check that your API keys or credentials are properly configured".to_string(),
                    "📝 Verify you have the necessary permissions to access this tool or service".to_string(),
                    "⏰ Some tokens expire - check if your credentials need to be refreshed".to_string(),
                    "👥 Contact your administrator if you need additional permissions".to_string(),
                ];

                (message.to_string(), actions)
            }
            ErrorCategory::RateLimitError => {
                let message = "📊 I found the right tool, but we've hit a usage limit. No worries - just need to pace ourselves!";

                let actions = vec![
                    "⏲️ Wait a few minutes before trying again - limits usually reset quickly".to_string(),
                    "😊 Space out your requests if you're making many in a row".to_string(),
                    "📈 Consider upgrading your service plan if you need higher limits".to_string(),
                    "🕰️ Check the service's rate limit documentation to understand the restrictions".to_string(),
                ];

                (message.to_string(), actions)
            }
            ErrorCategory::ConstraintViolation => {
                let message = if has_suggestions {
                    "🚨 I found tools that match your request, but they have constraints that prevent them from fulfilling it. Here are some alternatives."
                } else {
                    "🚨 The tools I found have limitations that conflict with your request. Let me help you find a better approach."
                };

                let actions = if has_suggestions {
                    vec![
                        "📋 Review the alternative tools below - they might work better for your needs".to_string(),
                        "🔍 Look for tools without the specific limitations mentioned".to_string(),
                        "✏️ Try modifying your request to work within the available tool constraints".to_string(),
                        "💡 Consider breaking your request into smaller parts that different tools can handle".to_string(),
                    ]
                } else {
                    vec![
                        "🎯 Try a more general version of your request (e.g., 'search web' instead of 'search academic papers')".to_string(),
                        "🔧 Look for tools that explicitly support your specific requirements".to_string(),
                        "📝 Modify your approach to work with the available tool capabilities".to_string(),
                        "🔄 Consider using multiple tools in sequence to achieve your goal".to_string(),
                    ]
                };

                (message.to_string(), actions)
            }
        }
    }

    /// Get fallback configuration
    pub fn get_config(&self) -> &FallbackConfig {
        &self.config
    }

    /// Get usage statistics
    pub fn get_usage_stats(&self) -> &HashMap<String, u64> {
        &self.usage_stats
    }

    /// Get recent tools
    pub fn get_recent_tools(&self) -> &Vec<String> {
        &self.recent_tools
    }
    
    /// Get learning statistics
    pub fn get_learning_stats(&self) -> serde_json::Value {
        let total_patterns = self.failure_patterns.len();
        let total_failures = self.failure_patterns.values().map(|p| p.failure_count).sum::<u64>();
        let patterns_with_resolutions = self.failure_patterns.values()
            .filter(|p| !p.successful_resolutions.is_empty())
            .count();
        
        let top_failure_patterns: Vec<_> = {
            let mut patterns: Vec<_> = self.failure_patterns.iter().collect();
            patterns.sort_by(|a, b| b.1.failure_count.cmp(&a.1.failure_count));
            patterns.into_iter().take(5).map(|(pattern, data)| {
                serde_json::json!({
                    "pattern": pattern,
                    "failure_count": data.failure_count,
                    "suggestions": data.learned_suggestions
                })
            }).collect()
        };
        
        serde_json::json!({
            "total_patterns_tracked": total_patterns,
            "total_failures_recorded": total_failures,
            "patterns_with_resolutions": patterns_with_resolutions,
            "learning_enabled": true,
            "top_failure_patterns": top_failure_patterns
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::types::ToolDefinition;

    fn create_test_tools() -> Vec<(String, ToolDefinition)> {
        vec![
            ("file_read".to_string(), ToolDefinition {
                name: "file_read".to_string(),
                description: "Read content from a file".to_string(),
                input_schema: serde_json::json!({}),
                routing: crate::registry::RoutingConfig {
                    r#type: "test".to_string(),
                    config: serde_json::json!({}),
                },
                annotations: None,
                hidden: false,
                enabled: true,
                prompt_refs: Vec::new(),
                resource_refs: Vec::new(),
            }),
            ("http_request".to_string(), ToolDefinition {
                name: "http_request".to_string(),
                description: "Make HTTP requests to web APIs".to_string(),
                input_schema: serde_json::json!({}),
                routing: crate::registry::RoutingConfig {
                    r#type: "test".to_string(),
                    config: serde_json::json!({}),
                },
                annotations: None,
                hidden: false,
                enabled: true,
                prompt_refs: Vec::new(),
                resource_refs: Vec::new(),
            }),
        ]
    }

    #[test]
    fn test_fallback_config_default() {
        let config = FallbackConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_confidence_threshold, 0.3);
        assert_eq!(config.max_fallback_suggestions, 5);
    }

    #[test]
    fn test_fuzzy_similarity() {
        let manager = FallbackManager::new_with_defaults();
        
        let similarity = manager.calculate_fuzzy_similarity("read", "file_read");
        assert!(similarity > 0.0);
        
        let similarity = manager.calculate_fuzzy_similarity("http", "http_request");
        assert!(similarity > 0.0);
        
        let similarity = manager.calculate_fuzzy_similarity("xyz", "abc");
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_keyword_extraction() {
        let manager = FallbackManager::new_with_defaults();
        
        let keywords = manager.extract_keywords("read a file from disk");
        assert!(keywords.contains(&"read".to_string()));
        assert!(keywords.contains(&"file".to_string()));
        assert!(keywords.contains(&"disk".to_string()));
        assert!(!keywords.contains(&"a".to_string())); // Stop word
    }

    #[test]
    fn test_tool_usage_tracking() {
        let mut manager = FallbackManager::new_with_defaults();
        
        manager.record_tool_usage("file_read");
        manager.record_tool_usage("file_read");
        manager.record_tool_usage("http_request");
        
        assert_eq!(manager.usage_stats.get("file_read"), Some(&2));
        assert_eq!(manager.usage_stats.get("http_request"), Some(&1));
        
        assert_eq!(manager.recent_tools[0], "http_request");
        assert_eq!(manager.recent_tools[1], "file_read");
    }

    #[test]
    fn test_enhanced_error_creation() {
        let manager = FallbackManager::new_with_defaults();
        
        let error = manager.create_enhanced_error(
            "Test error",
            ErrorCategory::NoToolsFound,
            None,
        );
        
        assert_eq!(error.primary_error, "Test error");
        assert_eq!(error.category, ErrorCategory::NoToolsFound);
        assert!(!error.suggested_actions.is_empty());
    }

    #[test]
    fn test_fallback_execution() {
        let mut manager = FallbackManager::new_with_defaults();
        let tools = create_test_tools();
        
        let request = SmartDiscoveryRequest {
            request: "read file".to_string(),
            context: None,
            preferred_tools: None,
            confidence_threshold: None,
            include_error_details: None,
            sequential_mode: None,
        };
        
        let result = manager.execute_fallback(&request, &tools, "No matches found");
        
        assert!(result.strategies_attempted > 0);
        assert!(!result.suggestions.is_empty());
    }
}