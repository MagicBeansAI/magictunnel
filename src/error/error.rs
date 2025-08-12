//! Error types and handling for the MCP Proxy

use thiserror::Error;

/// Result type alias for MCP Proxy operations
pub type Result<T> = std::result::Result<T, ProxyError>;

/// Main error type for the MCP Proxy
#[derive(Error, Debug)]
pub enum ProxyError {
    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Registry errors
    #[error("Registry error: {message}")]
    Registry { message: String },

    /// MCP protocol errors
    #[error("MCP protocol error: {message}")]
    Mcp { message: String },

    /// Routing errors
    #[error("Routing error: {message}")]
    Routing { message: String },

    /// Tool execution errors
    #[error("Tool execution error: {tool_name}: {message}")]
    ToolExecution { tool_name: String, message: String },

    /// Authentication errors
    #[error("Authentication error: {message}")]
    Auth { message: String },

    /// Security errors
    #[error("Security error: {message}")]
    Security { message: String },

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Connection errors (for MCP client connections)
    #[error("Connection error: {message}")]
    Connection { message: String },

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// YAML parsing errors
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// HTTP client errors
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON Schema validation errors
    #[error("JSON Schema validation error: {0}")]
    JsonSchema(#[from] jsonschema::ValidationError<'static>),

    /// Generic errors
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl ProxyError {
    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a registry error
    pub fn registry<S: Into<String>>(message: S) -> Self {
        Self::Registry {
            message: message.into(),
        }
    }

    /// Create an MCP protocol error
    pub fn mcp<S: Into<String>>(message: S) -> Self {
        Self::Mcp {
            message: message.into(),
        }
    }

    /// Create a routing error
    pub fn routing<S: Into<String>>(message: S) -> Self {
        Self::Routing {
            message: message.into(),
        }
    }

    /// Create a tool execution error
    pub fn tool_execution<S: Into<String>>(tool_name: S, message: S) -> Self {
        Self::ToolExecution {
            tool_name: tool_name.into(),
            message: message.into(),
        }
    }

    /// Create an authentication error
    pub fn auth<S: Into<String>>(message: S) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }

    /// Create a security error
    pub fn security<S: Into<String>>(message: S) -> Self {
        Self::Security {
            message: message.into(),
        }
    }

    /// Create a timeout error (using connection error type)
    pub fn timeout<S: Into<String>>(message: S) -> Self {
        Self::Connection {
            message: format!("Timeout: {}", message.into()),
        }
    }

    /// Create a connection error
    pub fn connection<S: Into<String>>(message: S) -> Self {
        Self::Connection {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a user context error (using config error type)
    pub fn user_context<E: std::fmt::Display>(error: E) -> Self {
        Self::Config {
            message: format!("User context error: {}", error),
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ProxyError::Http(_) | ProxyError::Io(_) | ProxyError::ToolExecution { .. }
        )
    }

    /// Get the error category for logging/metrics
    pub fn category(&self) -> &'static str {
        match self {
            ProxyError::Config { .. } => "config",
            ProxyError::Registry { .. } => "registry",
            ProxyError::Mcp { .. } => "mcp",
            ProxyError::Routing { .. } => "routing",
            ProxyError::ToolExecution { .. } => "tool_execution",
            ProxyError::Auth { .. } => "auth",
            ProxyError::Security { .. } => "security",
            ProxyError::Validation { .. } => "validation",
            ProxyError::Connection { .. } => "connection",
            ProxyError::Io(_) => "io",
            ProxyError::Serde(_) => "serialization",
            ProxyError::Yaml(_) => "yaml",
            ProxyError::Http(_) => "http",
            ProxyError::JsonSchema(_) => "json_schema",
            ProxyError::Internal(_) => "internal",
        }
    }
}

impl Clone for ProxyError {
    fn clone(&self) -> Self {
        match self {
            ProxyError::Config { message } => ProxyError::Config { message: message.clone() },
            ProxyError::Registry { message } => ProxyError::Registry { message: message.clone() },
            ProxyError::Mcp { message } => ProxyError::Mcp { message: message.clone() },
            ProxyError::Routing { message } => ProxyError::Routing { message: message.clone() },
            ProxyError::ToolExecution { tool_name, message } => ProxyError::ToolExecution {
                tool_name: tool_name.clone(),
                message: message.clone()
            },
            ProxyError::Auth { message } => ProxyError::Auth { message: message.clone() },
            ProxyError::Validation { message } => ProxyError::Validation { message: message.clone() },
            ProxyError::Connection { message } => ProxyError::Connection { message: message.clone() },
            ProxyError::Security { message } => ProxyError::Security { message: message.clone() },

            // For non-cloneable types, convert to string representation
            ProxyError::Io(e) => ProxyError::routing(format!("IO error: {}", e)),
            ProxyError::Serde(e) => ProxyError::routing(format!("Serialization error: {}", e)),
            ProxyError::Yaml(e) => ProxyError::routing(format!("YAML error: {}", e)),
            ProxyError::Http(e) => ProxyError::routing(format!("HTTP error: {}", e)),
            ProxyError::JsonSchema(e) => ProxyError::routing(format!("JSON Schema error: {}", e)),
            ProxyError::Internal(e) => ProxyError::routing(format!("Internal error: {}", e)),
        }
    }
}
