//! Centralized timeout configuration for agent execution

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Timeout configuration for different agent types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Default timeout for all agent types (in seconds)
    pub default_timeout_secs: u64,
    /// Per-agent-type timeout configuration
    pub per_agent_type: HashMap<String, u64>,
    /// Global maximum timeout (safety limit)
    pub max_timeout_secs: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        let mut per_agent_type = HashMap::new();
        
        // Set reasonable defaults per agent type
        per_agent_type.insert("subprocess".to_string(), 30);  // 30 seconds for subprocess
        per_agent_type.insert("http".to_string(), 30);        // 30 seconds for HTTP requests
        per_agent_type.insert("llm".to_string(), 60);         // 60 seconds for LLM calls (can be slow)
        per_agent_type.insert("websocket".to_string(), 30);   // 30 seconds for WebSocket operations
        per_agent_type.insert("database".to_string(), 30);    // 30 seconds for database queries
        
        Self {
            default_timeout_secs: 30,
            per_agent_type,
            max_timeout_secs: 300, // 5 minutes maximum
        }
    }
}

impl TimeoutConfig {
    /// Create a new timeout configuration with custom defaults
    pub fn new(default_timeout_secs: u64, max_timeout_secs: u64) -> Self {
        Self {
            default_timeout_secs,
            per_agent_type: HashMap::new(),
            max_timeout_secs,
        }
    }

    /// Set timeout for a specific agent type
    pub fn set_agent_timeout(&mut self, agent_type: &str, timeout_secs: u64) {
        let capped_timeout = timeout_secs.min(self.max_timeout_secs);
        self.per_agent_type.insert(agent_type.to_string(), capped_timeout);
    }

    /// Get timeout for a specific agent type with optional override
    /// Priority: tool_override > agent_type_config > default_timeout
    pub fn get_timeout(&self, agent_type: &str, tool_override: Option<u64>) -> Duration {
        let timeout_secs = if let Some(override_timeout) = tool_override {
            // Tool-specific override has highest priority
            override_timeout.min(self.max_timeout_secs)
        } else if let Some(&agent_timeout) = self.per_agent_type.get(agent_type) {
            // Agent-type specific timeout
            agent_timeout
        } else {
            // Fall back to default
            self.default_timeout_secs
        };
        
        Duration::from_secs(timeout_secs)
    }

    /// Get timeout in seconds (for backward compatibility)
    pub fn get_timeout_secs(&self, agent_type: &str, tool_override: Option<u64>) -> u64 {
        self.get_timeout(agent_type, tool_override).as_secs()
    }

    /// Validate timeout configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.default_timeout_secs == 0 {
            return Err("Default timeout cannot be zero".to_string());
        }
        
        if self.max_timeout_secs == 0 {
            return Err("Maximum timeout cannot be zero".to_string());
        }
        
        if self.default_timeout_secs > self.max_timeout_secs {
            return Err("Default timeout cannot exceed maximum timeout".to_string());
        }
        
        for (agent_type, &timeout) in &self.per_agent_type {
            if timeout == 0 {
                return Err(format!("Timeout for agent type '{}' cannot be zero", agent_type));
            }
            
            if timeout > self.max_timeout_secs {
                return Err(format!(
                    "Timeout for agent type '{}' ({}) exceeds maximum timeout ({})",
                    agent_type, timeout, self.max_timeout_secs
                ));
            }
        }
        
        Ok(())
    }

    /// Create a builder for timeout configuration
    pub fn builder() -> TimeoutConfigBuilder {
        TimeoutConfigBuilder::new()
    }
}

/// Builder for timeout configuration
#[derive(Debug)]
pub struct TimeoutConfigBuilder {
    config: TimeoutConfig,
}

impl TimeoutConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: TimeoutConfig {
                default_timeout_secs: 30,
                per_agent_type: HashMap::new(), // Start with empty map
                max_timeout_secs: 300,
            },
        }
    }

    /// Set default timeout
    pub fn default_timeout(mut self, timeout_secs: u64) -> Self {
        self.config.default_timeout_secs = timeout_secs;
        self
    }

    /// Set maximum timeout
    pub fn max_timeout(mut self, timeout_secs: u64) -> Self {
        self.config.max_timeout_secs = timeout_secs;
        self
    }

    /// Set timeout for subprocess agents
    pub fn subprocess_timeout(mut self, timeout_secs: u64) -> Self {
        self.config.set_agent_timeout("subprocess", timeout_secs);
        self
    }

    /// Set timeout for HTTP agents
    pub fn http_timeout(mut self, timeout_secs: u64) -> Self {
        self.config.set_agent_timeout("http", timeout_secs);
        self
    }

    /// Set timeout for LLM agents
    pub fn llm_timeout(mut self, timeout_secs: u64) -> Self {
        self.config.set_agent_timeout("llm", timeout_secs);
        self
    }

    /// Set timeout for WebSocket agents
    pub fn websocket_timeout(mut self, timeout_secs: u64) -> Self {
        self.config.set_agent_timeout("websocket", timeout_secs);
        self
    }

    /// Set timeout for database agents
    pub fn database_timeout(mut self, timeout_secs: u64) -> Self {
        self.config.set_agent_timeout("database", timeout_secs);
        self
    }

    /// Set timeout for a custom agent type
    pub fn agent_timeout(mut self, agent_type: &str, timeout_secs: u64) -> Self {
        self.config.set_agent_timeout(agent_type, timeout_secs);
        self
    }

    /// Build the timeout configuration
    pub fn build(self) -> Result<TimeoutConfig, String> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for TimeoutConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_config_defaults() {
        let config = TimeoutConfig::default();
        
        assert_eq!(config.default_timeout_secs, 30);
        assert_eq!(config.max_timeout_secs, 300);
        assert_eq!(config.get_timeout_secs("subprocess", None), 30);
        assert_eq!(config.get_timeout_secs("http", None), 30);
        assert_eq!(config.get_timeout_secs("llm", None), 60);
        assert_eq!(config.get_timeout_secs("websocket", None), 30);
        assert_eq!(config.get_timeout_secs("database", None), 30);
    }

    #[test]
    fn test_timeout_config_tool_override() {
        let config = TimeoutConfig::default();
        
        // Tool override should take precedence
        assert_eq!(config.get_timeout_secs("subprocess", Some(45)), 45);
        assert_eq!(config.get_timeout_secs("llm", Some(120)), 120);
        
        // But should be capped by max_timeout
        assert_eq!(config.get_timeout_secs("subprocess", Some(400)), 300);
    }

    #[test]
    fn test_timeout_config_unknown_agent_type() {
        let config = TimeoutConfig::default();
        
        // Unknown agent type should use default
        assert_eq!(config.get_timeout_secs("unknown", None), 30);
        assert_eq!(config.get_timeout_secs("custom", Some(60)), 60);
    }

    #[test]
    fn test_timeout_config_builder() {
        let config = TimeoutConfig::builder()
            .default_timeout(45)
            .max_timeout(600)
            .subprocess_timeout(20)
            .http_timeout(40)
            .llm_timeout(90)
            .build()
            .unwrap();
        
        assert_eq!(config.default_timeout_secs, 45);
        assert_eq!(config.max_timeout_secs, 600);
        assert_eq!(config.get_timeout_secs("subprocess", None), 20);
        assert_eq!(config.get_timeout_secs("http", None), 40);
        assert_eq!(config.get_timeout_secs("llm", None), 90);
        assert_eq!(config.get_timeout_secs("websocket", None), 45); // Uses configured default
    }

    #[test]
    fn test_timeout_config_validation() {
        // Valid configuration
        let config = TimeoutConfig::builder()
            .default_timeout(30)
            .max_timeout(300)
            .build();
        assert!(config.is_ok());
        
        // Invalid: zero default timeout
        let config = TimeoutConfig::builder()
            .default_timeout(0)
            .build();
        assert!(config.is_err());
        
        // Invalid: default exceeds max
        let config = TimeoutConfig::builder()
            .default_timeout(400)
            .max_timeout(300)
            .build();
        assert!(config.is_err());
    }
}
