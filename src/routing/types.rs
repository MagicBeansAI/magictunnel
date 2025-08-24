//! Routing types and agent definitions

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request context that carries information through the routing pipeline
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Client ID from the session manager
    pub client_id: Option<String>,
    /// Session ID for tracking
    pub session_id: Option<String>,
    /// Authentication context if available
    pub auth_context: Option<crate::auth::AuthenticationContext>,
}

/// Agent types supported by the router
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentType {
    /// Subprocess agent (execute commands)
    #[serde(rename = "subprocess")]
    Subprocess {
        command: String,
        args: Vec<String>,
        timeout: Option<u64>,
        env: Option<std::collections::HashMap<String, String>>,
    },
    
    /// HTTP agent (make HTTP requests)
    #[serde(rename = "http")]
    Http {
        method: String,
        url: String,
        headers: Option<std::collections::HashMap<String, String>>,
        timeout: Option<u64>,
    },
    
    /// LLM agent (call language models)
    #[serde(rename = "llm")]
    Llm {
        provider: String,
        model: String,
        api_key: Option<String>,
        base_url: Option<String>,
        timeout: Option<u64>,
    },
    
    /// WebSocket agent (persistent connections)
    #[serde(rename = "websocket")]
    WebSocket {
        url: String,
        headers: Option<std::collections::HashMap<String, String>>,
        timeout: Option<u64>,
    },

    /// Database agent (SQL queries)
    #[serde(rename = "database")]
    Database {
        db_type: String,
        connection_string: String,
        query: String,
        timeout: Option<u64>,
    },

    /// gRPC agent (call gRPC services)
    #[serde(rename = "grpc")]
    Grpc {
        endpoint: String,
        service: String,
        method: String,
        headers: Option<std::collections::HashMap<String, String>>,
        timeout: Option<u64>,
        request_body: Option<String>,
    },

    /// SSE agent (subscribe to Server-Sent Events)
    #[serde(rename = "sse")]
    Sse {
        url: String,
        headers: Option<std::collections::HashMap<String, String>>,
        timeout: Option<u64>,
        max_events: Option<u32>,
        event_filter: Option<String>,
    },

    /// GraphQL agent (execute GraphQL queries and mutations)
    #[serde(rename = "graphql")]
    GraphQL {
        endpoint: String,
        query: Option<String>,
        variables: Option<serde_json::Value>,
        headers: Option<std::collections::HashMap<String, String>>,
        timeout: Option<u64>,
        operation_name: Option<String>,
    },

    /// External MCP agent (route to external MCP servers via external MCP integration)
    #[serde(rename = "external_mcp")]
    ExternalMcp {
        server_name: String,
        tool_name: String,
        timeout: Option<u64>,
        mapping_metadata: Option<std::collections::HashMap<String, String>>,
    },
    
    /// Smart Discovery agent (intelligent tool discovery and execution)
    #[serde(rename = "smart_discovery")]
    SmartDiscovery {
        enabled: bool,
    },
}

/// Smart Discovery LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartDiscoveryLlmConfig {
    pub provider: String,
    pub model: String,
    pub api_key_env: Option<String>,
    pub base_url: Option<String>,
    pub timeout: u64,
    pub max_retries: u32,
    pub enabled: bool,
}

/// Agent execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Whether execution was successful
    pub success: bool,
    /// Result data
    pub data: Option<Value>,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution metadata (timing, etc.)
    pub metadata: Option<Value>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new() -> Self {
        Self {
            client_id: None,
            session_id: None,
            auth_context: None,
        }
    }

    /// Create a request context with client ID
    pub fn with_client_id(client_id: String) -> Self {
        Self {
            client_id: Some(client_id),
            session_id: None,
            auth_context: None,
        }
    }

    /// Create a request context with session ID and client ID
    pub fn with_session(session_id: String, client_id: Option<String>) -> Self {
        Self {
            client_id,
            session_id: Some(session_id),
            auth_context: None,
        }
    }

    /// Add authentication context
    pub fn with_auth_context(mut self, auth_context: crate::auth::AuthenticationContext) -> Self {
        self.auth_context = Some(auth_context);
        self
    }
}
