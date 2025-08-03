//! MCP Session Management System
//! 
//! Provides session tracking for WebSocket connections with request ID uniqueness validation
//! and protocol version negotiation according to the MCP specification.

use crate::error::{Result, ProxyError};
use crate::mcp::types::McpRequest;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Supported MCP protocol versions in order of preference (newest first)
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[
    "2025-06-18",
    "2025-03-26",
    "2024-11-05",
    "2024-10-07", 
    "2024-09-25",
];

/// Default protocol version to use
pub const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";

/// Maximum number of active sessions
pub const MAX_ACTIVE_SESSIONS: usize = 1000;

/// Session timeout duration (30 minutes)
pub const SESSION_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Maximum number of request IDs to track per session
pub const MAX_REQUEST_IDS_PER_SESSION: usize = 10000;

/// MCP Session information
#[derive(Debug, Clone)]
pub struct McpSession {
    /// Unique session identifier
    pub id: String,
    /// Client information from initialize request
    pub client_info: Option<ClientInfo>,
    /// Negotiated protocol version
    pub protocol_version: String,
    /// Set of used request IDs in this session
    pub used_request_ids: HashSet<String>,
    /// Session creation time
    pub created_at: Instant,
    /// Last activity time
    pub last_activity: Instant,
    /// Whether the session has been initialized
    pub initialized: bool,
}

/// Client information from MCP initialize request
#[derive(Debug, Clone)]
pub struct ClientInfo {
    /// Client name
    pub name: String,
    /// Client version
    pub version: String,
    /// Supported protocol version
    pub protocol_version: Option<String>,
}

/// MCP Session Manager for tracking WebSocket connections and validating requests
#[derive(Debug)]
pub struct McpSessionManager {
    /// Active sessions indexed by session ID
    sessions: Arc<RwLock<HashMap<String, McpSession>>>,
    /// Configuration
    config: SessionConfig,
}

/// Configuration for session management
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum number of active sessions
    pub max_sessions: usize,
    /// Session timeout duration
    pub session_timeout: Duration,
    /// Maximum request IDs per session
    pub max_request_ids_per_session: usize,
    /// Enable strict protocol version validation
    pub strict_version_validation: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_sessions: MAX_ACTIVE_SESSIONS,
            session_timeout: SESSION_TIMEOUT,
            max_request_ids_per_session: MAX_REQUEST_IDS_PER_SESSION,
            strict_version_validation: true,
        }
    }
}

impl McpSessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self::with_config(SessionConfig::default())
    }

    /// Create a new session manager with custom configuration
    pub fn with_config(config: SessionConfig) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Create a new session
    pub fn create_session(&self) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        
        // Check session limit
        {
            let sessions = self.sessions.read().unwrap();
            if sessions.len() >= self.config.max_sessions {
                return Err(ProxyError::mcp("Maximum number of sessions reached".to_string()));
            }
        }

        let session = McpSession {
            id: session_id.clone(),
            client_info: None,
            protocol_version: DEFAULT_PROTOCOL_VERSION.to_string(),
            used_request_ids: HashSet::new(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
            initialized: false,
        };

        // Add session
        {
            let mut sessions = self.sessions.write().unwrap();
            sessions.insert(session_id.clone(), session);
        }

        info!("Created new MCP session: {}", session_id);
        Ok(session_id)
    }

    /// Remove a session
    pub fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        if sessions.remove(session_id).is_some() {
            info!("Removed MCP session: {}", session_id);
            Ok(())
        } else {
            Err(ProxyError::mcp(format!("Session not found: {}", session_id)))
        }
    }

    /// Get session information
    pub fn get_session(&self, session_id: &str) -> Option<McpSession> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(session_id).cloned()
    }

    /// Update session activity
    pub fn update_activity(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = Instant::now();
            Ok(())
        } else {
            Err(ProxyError::mcp(format!("Session not found: {}", session_id)))
        }
    }

    /// Validate request ID uniqueness within session
    pub fn validate_request_id(&self, session_id: &str, request_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            // Check if request ID is already used
            if session.used_request_ids.contains(request_id) {
                return Err(ProxyError::mcp(format!(
                    "Duplicate request ID '{}' in session '{}'", 
                    request_id, session_id
                )));
            }

            // Check request ID limit
            if session.used_request_ids.len() >= self.config.max_request_ids_per_session {
                // Remove oldest request IDs (simple cleanup - in production might want LRU)
                session.used_request_ids.clear();
                warn!("Cleared request ID cache for session '{}' due to limit", session_id);
            }

            // Add request ID to used set
            session.used_request_ids.insert(request_id.to_string());
            session.last_activity = Instant::now();
            
            debug!("Validated request ID '{}' for session '{}'", request_id, session_id);
            Ok(())
        } else {
            Err(ProxyError::mcp(format!("Session not found: {}", session_id)))
        }
    }

    /// Handle initialize request and negotiate protocol version
    pub fn handle_initialize(&self, session_id: &str, request: &McpRequest) -> Result<String> {
        // Extract client info and protocol version from initialize request
        let client_info = self.extract_client_info(request)?;
        let negotiated_version = self.negotiate_protocol_version(&client_info)?;

        // Update session with initialization info
        {
            let mut sessions = self.sessions.write().unwrap();
            if let Some(session) = sessions.get_mut(session_id) {
                session.client_info = Some(client_info);
                session.protocol_version = negotiated_version.clone();
                session.initialized = true;
                session.last_activity = Instant::now();
            } else {
                return Err(ProxyError::mcp(format!("Session not found: {}", session_id)));
            }
        }

        info!("Initialized session '{}' with protocol version '{}'", session_id, negotiated_version);
        Ok(negotiated_version)
    }

    /// Extract client information from initialize request
    fn extract_client_info(&self, request: &McpRequest) -> Result<ClientInfo> {
        let params = request.params.as_ref()
            .ok_or_else(|| ProxyError::mcp("Initialize request missing parameters".to_string()))?;

        let client_info = params.get("clientInfo")
            .ok_or_else(|| ProxyError::mcp("Initialize request missing clientInfo".to_string()))?;

        let name = client_info.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let version = client_info.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let protocol_version = params.get("protocolVersion")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(ClientInfo {
            name,
            version,
            protocol_version,
        })
    }

    /// Negotiate protocol version with client
    fn negotiate_protocol_version(&self, client_info: &ClientInfo) -> Result<String> {
        let client_version = client_info.protocol_version.as_deref()
            .unwrap_or(DEFAULT_PROTOCOL_VERSION);

        // Check if client version is supported
        if SUPPORTED_PROTOCOL_VERSIONS.contains(&client_version) {
            debug!("Using client requested protocol version: {}", client_version);
            return Ok(client_version.to_string());
        }

        // If strict validation is enabled, reject unsupported versions
        if self.config.strict_version_validation {
            return Err(ProxyError::mcp(format!(
                "Unsupported protocol version '{}'. Supported versions: {:?}",
                client_version, SUPPORTED_PROTOCOL_VERSIONS
            )));
        }

        // Otherwise, use default version
        warn!("Client requested unsupported protocol version '{}', using default '{}'", 
              client_version, DEFAULT_PROTOCOL_VERSION);
        Ok(DEFAULT_PROTOCOL_VERSION.to_string())
    }

    /// Clean up expired sessions
    pub fn cleanup_expired_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().unwrap();
        let now = Instant::now();
        let initial_count = sessions.len();

        sessions.retain(|session_id, session| {
            let expired = now.duration_since(session.last_activity) > self.config.session_timeout;
            if expired {
                info!("Removing expired session: {}", session_id);
            }
            !expired
        });

        let removed_count = initial_count - sessions.len();
        if removed_count > 0 {
            info!("Cleaned up {} expired sessions", removed_count);
        }
        removed_count
    }

    /// Get session statistics
    pub fn get_stats(&self) -> SessionStats {
        let sessions = self.sessions.read().unwrap();
        let now = Instant::now();
        
        let mut initialized_count = 0;
        let mut total_request_ids = 0;
        let mut oldest_session_age = Duration::ZERO;

        for session in sessions.values() {
            if session.initialized {
                initialized_count += 1;
            }
            total_request_ids += session.used_request_ids.len();
            
            let age = now.duration_since(session.created_at);
            if age > oldest_session_age {
                oldest_session_age = age;
            }
        }

        SessionStats {
            total_sessions: sessions.len(),
            initialized_sessions: initialized_count,
            total_request_ids,
            oldest_session_age,
        }
    }
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub initialized_sessions: usize,
    pub total_request_ids: usize,
    pub oldest_session_age: Duration,
}

impl Default for McpSessionManager {
    fn default() -> Self {
        Self::new()
    }
}
