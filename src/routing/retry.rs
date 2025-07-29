//! Retry logic for failed tool calls with configurable policies

use crate::error::{ProxyError, Result};
use crate::routing::types::{AgentResult, AgentType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn, error};

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries (in milliseconds)
    pub initial_delay_ms: u64,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Maximum delay between retries (in milliseconds)
    pub max_delay_ms: u64,
    /// Whether to use jitter to avoid thundering herd
    pub use_jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            backoff_multiplier: 2.0,
            max_delay_ms: 30000,
            use_jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy with custom settings
    pub fn new(max_attempts: u32, initial_delay_ms: u64, backoff_multiplier: f64) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            backoff_multiplier,
            max_delay_ms: 30000,
            use_jitter: true,
        }
    }

    /// Create a conservative retry policy (fewer attempts, longer delays)
    pub fn conservative() -> Self {
        Self {
            max_attempts: 2,
            initial_delay_ms: 2000,
            backoff_multiplier: 3.0,
            max_delay_ms: 60000,
            use_jitter: true,
        }
    }

    /// Create an aggressive retry policy (more attempts, shorter delays)
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 500,
            backoff_multiplier: 1.5,
            max_delay_ms: 15000,
            use_jitter: true,
        }
    }

    /// Calculate delay for a given attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.initial_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        let capped_delay = base_delay.min(self.max_delay_ms as f64);
        
        let final_delay = if self.use_jitter {
            // Add up to 25% jitter to avoid thundering herd
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            std::time::SystemTime::now().hash(&mut hasher);
            let seed = hasher.finish();
            let jitter = (seed % 25) as f64 / 100.0; // 0-25% jitter
            capped_delay * (1.0 + jitter)
        } else {
            capped_delay
        };

        Duration::from_millis(final_delay as u64)
    }
}

/// Retry configuration for different agent types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Default retry policy for all agent types
    pub default: RetryPolicy,
    /// Per-agent-type retry policies
    pub per_agent_type: HashMap<String, RetryPolicy>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        let mut per_agent_type = HashMap::new();
        
        // HTTP agents can be more aggressive with retries
        per_agent_type.insert("http".to_string(), RetryPolicy::aggressive());
        
        // LLM agents should be more conservative (API rate limits)
        per_agent_type.insert("llm".to_string(), RetryPolicy::conservative());
        
        // Database agents should be conservative (connection issues)
        per_agent_type.insert("database".to_string(), RetryPolicy::conservative());
        
        // Subprocess agents can use default policy
        // WebSocket agents can use default policy

        Self {
            default: RetryPolicy::default(),
            per_agent_type,
        }
    }
}

impl RetryConfig {
    /// Get retry policy for a specific agent type
    pub fn get_policy(&self, agent_type: &str) -> &RetryPolicy {
        self.per_agent_type.get(agent_type).unwrap_or(&self.default)
    }
}

/// Determines if an error should be retried
pub fn should_retry_error(error: &ProxyError) -> bool {
    match error {
        // Network-related errors should be retried
        ProxyError::Routing { message } if message.contains("timeout") => true,
        ProxyError::Routing { message } if message.contains("connection") => true,
        ProxyError::Routing { message } if message.contains("network") => true,

        // HTTP status codes that should be retried
        ProxyError::Routing { message } if message.contains("500") => true, // Internal Server Error
        ProxyError::Routing { message } if message.contains("502") => true, // Bad Gateway
        ProxyError::Routing { message } if message.contains("503") => true, // Service Unavailable
        ProxyError::Routing { message } if message.contains("504") => true, // Gateway Timeout
        ProxyError::Routing { message } if message.contains("429") => true, // Too Many Requests

        // Database connection errors should be retried
        ProxyError::Routing { message } if message.contains("database") && message.contains("connection") => true,

        // WebSocket connection errors should be retried
        ProxyError::Routing { message } if message.contains("WebSocket") && message.contains("connection") => true,

        // Don't retry authentication errors
        ProxyError::Routing { message } if message.contains("401") => false, // Unauthorized
        ProxyError::Routing { message } if message.contains("403") => false, // Forbidden
        ProxyError::Routing { message } if message.contains("authentication") => false,
        ProxyError::Routing { message } if message.contains("authorization") => false,
        ProxyError::Auth { .. } => false,

        // Don't retry client errors (4xx except 429)
        ProxyError::Routing { message } if message.contains("400") => false, // Bad Request
        ProxyError::Routing { message } if message.contains("404") => false, // Not Found
        ProxyError::Routing { message } if message.contains("422") => false, // Unprocessable Entity

        // Don't retry validation errors
        ProxyError::Validation { .. } => false,
        ProxyError::Config { .. } => false,

        // Retry HTTP and IO errors
        ProxyError::Http(_) => true,
        ProxyError::Io(_) => true,

        // Default: don't retry unknown errors
        _ => false,
    }
}

/// Retry executor that handles the retry logic
pub struct RetryExecutor {
    config: RetryConfig,
}

impl RetryExecutor {
    /// Create a new retry executor with default configuration
    pub fn new() -> Self {
        Self {
            config: RetryConfig::default(),
        }
    }

    /// Create a new retry executor with custom configuration
    pub fn with_config(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Execute a function with retry logic
    pub async fn execute_with_retry<F, Fut>(
        &self,
        agent_type: &AgentType,
        operation_name: &str,
        operation: F,
    ) -> Result<AgentResult>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<AgentResult>>,
    {
        let agent_type_name = agent_type.type_name();
        let policy = self.config.get_policy(&agent_type_name);
        
        debug!(
            operation = operation_name,
            agent_type = agent_type_name,
            max_attempts = policy.max_attempts,
            "Starting operation with retry logic"
        );

        let mut last_error = None;
        
        for attempt in 0..policy.max_attempts {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        debug!(
                            operation = operation_name,
                            agent_type = agent_type_name,
                            attempt = attempt + 1,
                            "Operation succeeded after retry"
                        );
                    }
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error.clone());
                    
                    // Check if we should retry this error
                    if !should_retry_error(&error) {
                        warn!(
                            operation = operation_name,
                            agent_type = agent_type_name,
                            attempt = attempt + 1,
                            error = %error,
                            "Error is not retryable, failing immediately"
                        );
                        return Err(error);
                    }
                    
                    // Check if we have more attempts left
                    if attempt + 1 >= policy.max_attempts {
                        error!(
                            operation = operation_name,
                            agent_type = agent_type_name,
                            total_attempts = attempt + 1,
                            error = %error,
                            "All retry attempts exhausted, failing"
                        );
                        return Err(error);
                    }
                    
                    // Calculate delay and wait
                    let delay = policy.calculate_delay(attempt);
                    warn!(
                        operation = operation_name,
                        agent_type = agent_type_name,
                        attempt = attempt + 1,
                        delay_ms = delay.as_millis(),
                        error = %error,
                        "Operation failed, retrying after delay"
                    );
                    
                    sleep(delay).await;
                }
            }
        }
        
        // This should never be reached, but just in case
        Err(last_error.unwrap_or_else(|| ProxyError::routing("Retry logic error".to_string())))
    }
}

impl Default for RetryExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for AgentType to get type name
impl AgentType {
    /// Get the string name of the agent type
    pub fn type_name(&self) -> String {
        match self {
            AgentType::Subprocess { .. } => "subprocess".to_string(),
            AgentType::Http { .. } => "http".to_string(),
            AgentType::Llm { .. } => "llm".to_string(),
            AgentType::WebSocket { .. } => "websocket".to_string(),
            AgentType::Database { .. } => "database".to_string(),
            AgentType::Grpc { .. } => "grpc".to_string(),
            AgentType::Sse { .. } => "sse".to_string(),
            AgentType::GraphQL { .. } => "graphql".to_string(),
            AgentType::ExternalMcp { .. } => "external_mcp".to_string(),
            AgentType::SmartDiscovery { .. } => "smart_discovery".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_retry_policy_delay_calculation() {
        let policy = RetryPolicy::new(3, 1000, 2.0);
        
        // Test exponential backoff
        let delay1 = policy.calculate_delay(0);
        let delay2 = policy.calculate_delay(1);
        let delay3 = policy.calculate_delay(2);
        
        // First attempt should be around initial delay
        assert!(delay1.as_millis() >= 1000 && delay1.as_millis() <= 1250);
        // Second attempt should be around 2x initial delay
        assert!(delay2.as_millis() >= 2000 && delay2.as_millis() <= 2500);
        // Third attempt should be around 4x initial delay
        assert!(delay3.as_millis() >= 4000 && delay3.as_millis() <= 5000);
    }

    #[test]
    fn test_should_retry_error() {
        // Should retry network errors
        assert!(should_retry_error(&ProxyError::routing("connection timeout".to_string())));
        assert!(should_retry_error(&ProxyError::routing("HTTP 500 error".to_string())));
        assert!(should_retry_error(&ProxyError::routing("HTTP 503 error".to_string())));

        // Should not retry auth errors
        assert!(!should_retry_error(&ProxyError::routing("HTTP 401 error".to_string())));
        assert!(!should_retry_error(&ProxyError::routing("authentication failed".to_string())));
        assert!(!should_retry_error(&ProxyError::auth("authentication failed".to_string())));

        // Should not retry validation errors
        assert!(!should_retry_error(&ProxyError::validation("invalid input".to_string())));
        assert!(!should_retry_error(&ProxyError::config("invalid config".to_string())));
    }

    #[tokio::test]
    async fn test_retry_executor_success_on_first_attempt() {
        let executor = RetryExecutor::new();
        let agent = AgentType::Http {
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            headers: None,
            timeout: None,
        };

        let result = executor.execute_with_retry(&agent, "test_operation", || async {
            Ok(AgentResult {
                success: true,
                data: Some(serde_json::json!({"result": "success"})),
                error: None,
                metadata: None,
            })
        }).await;

        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[tokio::test]
    async fn test_retry_executor_success_after_retry() {
        let executor = RetryExecutor::new();
        let agent = AgentType::Http {
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            headers: None,
            timeout: None,
        };

        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = executor.execute_with_retry(&agent, "test_operation", move || {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count == 0 {
                    // Fail on first attempt with retryable error
                    Err(ProxyError::routing("connection timeout".to_string()))
                } else {
                    // Succeed on second attempt
                    Ok(AgentResult {
                        success: true,
                        data: Some(serde_json::json!({"result": "success"})),
                        error: None,
                        metadata: None,
                    })
                }
            }
        }).await;

        assert!(result.is_ok());
        assert!(result.unwrap().success);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_executor_non_retryable_error() {
        let executor = RetryExecutor::new();
        let agent = AgentType::Http {
            method: "GET".to_string(),
            url: "http://example.com".to_string(),
            headers: None,
            timeout: None,
        };

        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = executor.execute_with_retry(&agent, "test_operation", move || {
            let _count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                // Always fail with non-retryable error
                Err(ProxyError::routing("HTTP 401 error".to_string()))
            }
        }).await;

        assert!(result.is_err());
        // Should only attempt once for non-retryable errors
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }
}
