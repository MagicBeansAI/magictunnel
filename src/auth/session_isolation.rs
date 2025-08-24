//! Enhanced Session Isolation System for Remote Client Binding
//!
//! This module provides comprehensive session isolation to prevent cross-user
//! session interference in multi-deployment scenarios. It creates secure
//! session boundaries for each remote client connecting to MagicTunnel instances.

use crate::auth::{RemoteUserContext, ClientIdentity, TokenStorage, AuthenticationResult};
use crate::error::{Result, ProxyError};
use actix_web::HttpRequest;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tracing::{debug, error, info, trace, warn};
use std::collections::HashMap;
use uuid::Uuid;

/// Enhanced session manager with remote client isolation
#[derive(Debug)]
pub struct IsolatedSessionManager {
    /// Active remote user sessions
    active_sessions: Arc<RwLock<HashMap<String, IsolatedSession>>>,
    
    /// Session configuration
    config: SessionIsolationConfig,
    
    /// Token storage factory for creating client-specific storage
    token_storage_factory: Arc<dyn TokenStorageFactory>,
    
    /// Session cleanup tracker
    cleanup_tracker: Arc<RwLock<SessionCleanupTracker>>,
}

/// Configuration for session isolation
#[derive(Debug, Clone)]
pub struct SessionIsolationConfig {
    /// Maximum session duration in hours
    pub max_session_duration_hours: u64,
    
    /// Maximum inactivity period in hours
    pub max_inactivity_hours: u64,
    
    /// Enable strict client validation
    pub strict_client_validation: bool,
    
    /// Require MCP client identity
    pub require_mcp_identity: bool,
    
    /// Maximum concurrent sessions per client
    pub max_sessions_per_client: usize,
    
    /// Session cleanup interval in minutes
    pub cleanup_interval_minutes: u64,
    
    /// Enable session recovery
    pub enable_session_recovery: bool,
    
    /// Session recovery timeout in minutes
    pub recovery_timeout_minutes: u64,
}

/// Isolated session with remote client binding
#[derive(Debug, Clone)]
pub struct IsolatedSession {
    /// Unique session identifier
    pub session_id: String,
    
    /// Remote user context for this session
    pub remote_context: RemoteUserContext,
    
    /// Session metadata
    pub metadata: SessionMetadata,
    
    /// Session state
    pub state: IsolationSessionState,
    
    /// Client-specific token storage
    pub token_storage: Option<Arc<TokenStorage>>,
    
    /// Authentication result if authenticated
    pub auth_result: Option<AuthenticationResult>,
    
    /// Session isolation boundaries
    pub isolation_boundaries: IsolationBoundaries,
}

/// Session metadata for tracking and validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Session creation timestamp
    pub created_at: SystemTime,
    
    /// Last activity timestamp
    pub last_activity: SystemTime,
    
    /// Session expiry timestamp
    pub expires_at: Option<SystemTime>,
    
    /// Number of requests processed
    pub request_count: u64,
    
    /// Session tags for categorization
    pub tags: HashMap<String, String>,
    
    /// Client connection metadata
    pub connection_metadata: ConnectionMetadata,
}

/// Client connection metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetadata {
    /// Original client IP
    pub client_ip: String,
    
    /// Connection type (HTTP, WebSocket, etc.)
    pub connection_type: String,
    
    /// User agent string
    pub user_agent: Option<String>,
    
    /// Connection establishment time
    pub connected_at: SystemTime,
    
    /// TLS information if applicable
    pub tls_info: Option<TlsConnectionInfo>,
    
    /// Proxy chain information
    pub proxy_chain: Option<Vec<String>>,
}

/// TLS connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConnectionInfo {
    /// TLS version
    pub version: String,
    
    /// Cipher suite
    pub cipher_suite: Option<String>,
    
    /// Client certificate fingerprint
    pub client_cert_fingerprint: Option<String>,
}

/// Session state tracking for isolation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IsolationSessionState {
    /// Session is initializing
    Initializing,
    
    /// Session is active and ready
    Active,
    
    /// Session is authenticated
    Authenticated,
    
    /// Session is suspended (temporary)
    Suspended,
    
    /// Session is being recovered
    Recovering,
    
    /// Session is expired
    Expired,
    
    /// Session is terminated
    Terminated,
}

/// Isolation boundaries for session security
#[derive(Debug, Clone)]
pub struct IsolationBoundaries {
    /// Network isolation key
    pub network_isolation: String,
    
    /// Storage isolation key
    pub storage_isolation: String,
    
    /// Authentication isolation key
    pub auth_isolation: String,
    
    /// Resource access boundaries
    pub resource_boundaries: Vec<String>,
    
    /// Allowed operations
    pub allowed_operations: Vec<String>,
}

/// Session cleanup tracking
#[derive(Debug, Default)]
pub struct SessionCleanupTracker {
    /// Last cleanup timestamp
    pub last_cleanup: Option<SystemTime>,
    
    /// Cleanup statistics
    pub cleanup_stats: CleanupStats,
    
    /// Sessions pending cleanup
    pub pending_cleanup: Vec<String>,
}

/// Cleanup statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CleanupStats {
    /// Total cleanups performed
    pub total_cleanups: u64,
    
    /// Expired sessions cleaned
    pub expired_sessions: u64,
    
    /// Inactive sessions cleaned
    pub inactive_sessions: u64,
    
    /// Failed cleanups
    pub failed_cleanups: u64,
    
    /// Last cleanup duration in milliseconds
    pub last_cleanup_duration_ms: u64,
}

/// Token storage factory trait
pub trait TokenStorageFactory: Send + Sync + std::fmt::Debug {
    /// Create token storage for remote user context
    fn create_token_storage(&self, remote_context: &RemoteUserContext) -> Result<Arc<TokenStorage>>;
}

/// Default token storage factory implementation
#[derive(Debug)]
pub struct DefaultTokenStorageFactory;

impl TokenStorageFactory for DefaultTokenStorageFactory {
    fn create_token_storage(&self, remote_context: &RemoteUserContext) -> Result<Arc<TokenStorage>> {
        // Use the remote context's user context for token storage
        let user_context = remote_context.to_user_context();
        
        // Create token storage with the modified user context
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                TokenStorage::new(user_context).await.map(Arc::new)
            })
        })
    }
}

impl Default for SessionIsolationConfig {
    fn default() -> Self {
        Self {
            max_session_duration_hours: 24,
            max_inactivity_hours: 2,
            strict_client_validation: true,
            require_mcp_identity: false,
            max_sessions_per_client: 5,
            cleanup_interval_minutes: 15,
            enable_session_recovery: true,
            recovery_timeout_minutes: 10,
        }
    }
}

impl IsolatedSessionManager {
    /// Create a new isolated session manager
    pub fn new(config: SessionIsolationConfig) -> Self {
        let token_storage_factory = Arc::new(DefaultTokenStorageFactory) as Arc<dyn TokenStorageFactory>;
        
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
            token_storage_factory,
            cleanup_tracker: Arc::new(RwLock::new(SessionCleanupTracker::default())),
        }
    }
    
    /// Create a new isolated session manager with custom token storage factory
    pub fn with_token_storage_factory(
        config: SessionIsolationConfig,
        token_storage_factory: Arc<dyn TokenStorageFactory>,
    ) -> Self {
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
            token_storage_factory,
            cleanup_tracker: Arc::new(RwLock::new(SessionCleanupTracker::default())),
        }
    }

    /// Create a new isolated session for remote client
    pub async fn create_session(
        &self,
        req: &HttpRequest,
        remote_context: RemoteUserContext,
    ) -> Result<String> {
        debug!("Creating isolated session for client: {}", remote_context.display_name());

        // Check if client has reached maximum session limit
        self.enforce_session_limits(&remote_context)?;

        // Generate unique session ID
        let session_id = self.generate_session_id(&remote_context)?;

        // Create connection metadata
        let connection_metadata = self.create_connection_metadata(req)?;

        // Create session metadata
        let metadata = SessionMetadata {
            created_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            expires_at: Some(SystemTime::now() + Duration::from_secs(self.config.max_session_duration_hours * 3600)),
            request_count: 0,
            tags: HashMap::new(),
            connection_metadata,
        };

        // Create isolation boundaries
        let isolation_boundaries = self.create_isolation_boundaries(&remote_context)?;

        // Create client-specific token storage
        let token_storage = if self.config.strict_client_validation {
            Some(self.token_storage_factory.create_token_storage(&remote_context)?)
        } else {
            None
        };

        // Create isolated session
        let session = IsolatedSession {
            session_id: session_id.clone(),
            remote_context,
            metadata,
            state: IsolationSessionState::Initializing,
            token_storage,
            auth_result: None,
            isolation_boundaries,
        };

        // Store session
        {
            let mut sessions = self.active_sessions.write().unwrap();
            sessions.insert(session_id.clone(), session);
        }

        info!("Created isolated session: {}", session_id);
        Ok(session_id)
    }

    /// Get isolated session by ID
    pub fn get_session(&self, session_id: &str) -> Option<IsolatedSession> {
        let sessions = self.active_sessions.read().unwrap();
        sessions.get(session_id).cloned()
    }

    /// Update session activity
    pub fn update_activity(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.active_sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.metadata.last_activity = SystemTime::now();
            session.metadata.request_count += 1;
            
            // Update remote context activity
            session.remote_context.update_activity();
            
            trace!("Updated activity for session: {}", session_id);
            Ok(())
        } else {
            Err(ProxyError::auth(format!("Session not found: {}", session_id)))
        }
    }

    /// Authenticate session
    pub fn authenticate_session(
        &self,
        session_id: &str,
        auth_result: AuthenticationResult,
    ) -> Result<()> {
        let mut sessions = self.active_sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.auth_result = Some(auth_result);
            session.state = IsolationSessionState::Authenticated;
            
            // Update remote context authentication
            session.remote_context.update_auth_state(
                true,
                Some("mcp".to_string()),
                None,
            );
            
            info!("Authenticated session: {}", session_id);
            Ok(())
        } else {
            Err(ProxyError::auth(format!("Session not found: {}", session_id)))
        }
    }

    /// Suspend session temporarily
    pub fn suspend_session(&self, session_id: &str, reason: &str) -> Result<()> {
        let mut sessions = self.active_sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.state = IsolationSessionState::Suspended;
            session.metadata.tags.insert("suspend_reason".to_string(), reason.to_string());
            session.metadata.tags.insert("suspended_at".to_string(), 
                SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string());
            
            warn!("Suspended session {}: {}", session_id, reason);
            Ok(())
        } else {
            Err(ProxyError::auth(format!("Session not found: {}", session_id)))
        }
    }

    /// Resume suspended session
    pub fn resume_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.active_sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            if session.state == IsolationSessionState::Suspended {
                session.state = IsolationSessionState::Active;
                session.metadata.tags.remove("suspend_reason");
                session.metadata.tags.remove("suspended_at");
                session.metadata.last_activity = SystemTime::now();
                
                info!("Resumed session: {}", session_id);
                Ok(())
            } else {
                Err(ProxyError::auth(format!("Session {} is not suspended", session_id)))
            }
        } else {
            Err(ProxyError::auth(format!("Session not found: {}", session_id)))
        }
    }

    /// Terminate session
    pub async fn terminate_session(&self, session_id: &str) -> Result<()> {
        let session = {
            let mut sessions = self.active_sessions.write().unwrap();
            sessions.remove(session_id)
        };

        if let Some(mut session) = session {
            session.state = IsolationSessionState::Terminated;
            
            // Clean up token storage if needed
            if let Some(token_storage) = &session.token_storage {
                if let Err(e) = token_storage.cleanup_expired_tokens().await {
                    warn!("Failed to cleanup tokens for session {}: {}", session_id, e);
                }
            }
            
            info!("Terminated session: {}", session_id);
            Ok(())
        } else {
            Err(ProxyError::auth(format!("Session not found: {}", session_id)))
        }
    }

    /// Get all sessions for a specific client
    pub fn get_client_sessions(&self, client_identity: &ClientIdentity) -> Vec<IsolatedSession> {
        let sessions = self.active_sessions.read().unwrap();
        let client_ip = client_identity.client_ip.to_string();
        
        sessions.values()
            .filter(|session| {
                session.remote_context.client_identity.client_ip.to_string() == client_ip &&
                session.remote_context.client_identity.client_hostname == client_identity.client_hostname
            })
            .cloned()
            .collect()
    }

    /// Get session statistics
    pub fn get_session_stats(&self) -> SessionStats {
        let sessions = self.active_sessions.read().unwrap();
        let cleanup_tracker = self.cleanup_tracker.read().unwrap();
        
        let mut stats = SessionStats {
            total_sessions: sessions.len(),
            active_sessions: 0,
            authenticated_sessions: 0,
            suspended_sessions: 0,
            expired_sessions: 0,
            unique_clients: std::collections::HashSet::new(),
            cleanup_stats: cleanup_tracker.cleanup_stats.clone(),
        };

        for session in sessions.values() {
            match session.state {
                IsolationSessionState::Active | IsolationSessionState::Initializing => stats.active_sessions += 1,
                IsolationSessionState::Authenticated => stats.authenticated_sessions += 1,
                IsolationSessionState::Suspended => stats.suspended_sessions += 1,
                IsolationSessionState::Expired => stats.expired_sessions += 1,
                _ => {}
            }
            
            stats.unique_clients.insert(session.remote_context.client_identity.client_ip.to_string());
        }

        stats
    }

    /// Perform session cleanup
    pub async fn cleanup_sessions(&self) -> Result<CleanupStats> {
        let cleanup_start = SystemTime::now();
        let mut stats = CleanupStats::default();
        
        debug!("Starting session cleanup");

        let expired_sessions: Vec<String> = {
            let sessions = self.active_sessions.read().unwrap();
            let now = SystemTime::now();
            
            sessions.iter()
                .filter_map(|(id, session)| {
                    // Check if session is expired by time
                    if let Some(expires_at) = session.metadata.expires_at {
                        if now > expires_at {
                            return Some(id.clone());
                        }
                    }
                    
                    // Check if session is expired by inactivity
                    let max_inactivity = Duration::from_secs(self.config.max_inactivity_hours * 3600);
                    let inactivity = now.duration_since(session.metadata.last_activity)
                        .unwrap_or(Duration::from_secs(0));
                    
                    if inactivity > max_inactivity {
                        return Some(id.clone());
                    }
                    
                    None
                })
                .collect()
        };

        // Clean up expired sessions
        for session_id in expired_sessions {
            match self.terminate_session(&session_id).await {
                Ok(()) => {
                    stats.expired_sessions += 1;
                    trace!("Cleaned up expired session: {}", session_id);
                }
                Err(e) => {
                    error!("Failed to cleanup session {}: {}", session_id, e);
                    stats.failed_cleanups += 1;
                }
            }
        }

        // Update cleanup tracker
        let cleanup_duration = cleanup_start.elapsed().unwrap_or(Duration::from_secs(0));
        stats.last_cleanup_duration_ms = cleanup_duration.as_millis() as u64;
        stats.total_cleanups = 1;

        {
            let mut cleanup_tracker = self.cleanup_tracker.write().unwrap();
            cleanup_tracker.last_cleanup = Some(SystemTime::now());
            cleanup_tracker.cleanup_stats.total_cleanups += 1;
            cleanup_tracker.cleanup_stats.expired_sessions += stats.expired_sessions;
            cleanup_tracker.cleanup_stats.failed_cleanups += stats.failed_cleanups;
            cleanup_tracker.cleanup_stats.last_cleanup_duration_ms = stats.last_cleanup_duration_ms;
        }

        info!("Session cleanup completed: {} expired, {} failed, {}ms", 
               stats.expired_sessions, stats.failed_cleanups, stats.last_cleanup_duration_ms);

        Ok(stats)
    }

    /// Generate unique session ID
    fn generate_session_id(&self, remote_context: &RemoteUserContext) -> Result<String> {
        let uuid = Uuid::new_v4();
        let client_hash = &remote_context.isolation_key[..8]; // Use part of isolation key
        let uuid_str = uuid.simple().to_string();
        Ok(format!("iso_{}_{}", client_hash, &uuid_str[..16]))
    }

    /// Create connection metadata from HTTP request
    fn create_connection_metadata(&self, req: &HttpRequest) -> Result<ConnectionMetadata> {
        let connection_info = req.connection_info();
        let client_ip = connection_info
            .realip_remote_addr()
            .unwrap_or_else(|| connection_info.peer_addr().unwrap_or("unknown"))
            .to_string();

        let user_agent = req.headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(String::from);

        // Extract proxy chain from forwarded headers
        let proxy_chain = if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded_for.to_str() {
                Some(forwarded_str.split(',').map(|s| s.trim().to_string()).collect())
            } else {
                None
            }
        } else {
            None
        };

        // Extract TLS information if available
        let tls_info = extract_tls_info(req);
        
        // Detect actual connection type based on request properties
        let connection_type = detect_connection_type(req);

        Ok(ConnectionMetadata {
            client_ip,
            connection_type,
            user_agent,
            connected_at: SystemTime::now(),
            tls_info,
            proxy_chain,
        })
    }

    /// Create isolation boundaries for session
    fn create_isolation_boundaries(&self, remote_context: &RemoteUserContext) -> Result<IsolationBoundaries> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(remote_context.isolation_key.as_bytes());
        hasher.update(b"network");
        let network_isolation = format!("{:x}", hasher.finalize())[..16].to_string();

        let mut hasher = Sha256::new();
        hasher.update(remote_context.isolation_key.as_bytes());
        hasher.update(b"storage");
        let storage_isolation = format!("{:x}", hasher.finalize())[..16].to_string();

        let mut hasher = Sha256::new();
        hasher.update(remote_context.isolation_key.as_bytes());
        hasher.update(b"auth");
        let auth_isolation = format!("{:x}", hasher.finalize())[..16].to_string();

        Ok(IsolationBoundaries {
            network_isolation,
            storage_isolation,
            auth_isolation,
            resource_boundaries: vec![
                format!("session:{}", remote_context.remote_session_id),
                format!("client:{}", remote_context.client_identity.client_ip),
            ],
            allowed_operations: vec![
                "read".to_string(),
                "write".to_string(),
                "execute".to_string(),
            ],
        })
    }

    /// Enforce session limits per client
    fn enforce_session_limits(&self, remote_context: &RemoteUserContext) -> Result<()> {
        let client_sessions = self.get_client_sessions(&remote_context.client_identity);
        
        if client_sessions.len() >= self.config.max_sessions_per_client {
            warn!("Client {} has reached maximum session limit: {}", 
                  remote_context.display_name(), self.config.max_sessions_per_client);
            
            return Err(ProxyError::auth(format!(
                "Maximum session limit reached: {}/{}", 
                client_sessions.len(), 
                self.config.max_sessions_per_client
            )));
        }

        Ok(())
    }
}

/// Session statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub authenticated_sessions: usize,
    pub suspended_sessions: usize,
    pub expired_sessions: usize,
    #[serde(skip)]
    pub unique_clients: std::collections::HashSet<String>,
    pub cleanup_stats: CleanupStats,
}

impl SessionStats {
    pub fn unique_client_count(&self) -> usize {
        self.unique_clients.len()
    }
}

/// Extract TLS information from HTTP request if available
fn extract_tls_info(req: &HttpRequest) -> Option<TlsConnectionInfo> {
    // Check for TLS termination at reverse proxy level
    let is_secure = req.connection_info().scheme() == "https" ||
                   req.headers().get("x-forwarded-proto").map(|v| v.to_str().unwrap_or("")) == Some("https") ||
                   req.headers().get("x-forwarded-ssl").map(|v| v.to_str().unwrap_or("")) == Some("on");
    
    if !is_secure {
        return None;
    }
    
    let mut tls_info = TlsConnectionInfo {
        version: "unknown".to_string(),
        cipher_suite: None,
        client_cert_fingerprint: None,
    };
    
    // Extract TLS version from headers if provided by reverse proxy
    if let Some(tls_version) = req.headers().get("x-forwarded-tls-version") {
        if let Ok(version_str) = tls_version.to_str() {
            tls_info.version = version_str.to_string();
        }
    } else if let Some(ssl_protocol) = req.headers().get("x-forwarded-ssl-protocol") {
        if let Ok(protocol_str) = ssl_protocol.to_str() {
            tls_info.version = protocol_str.to_string();
        }
    }
    
    // Extract cipher suite if available
    if let Some(cipher) = req.headers().get("x-forwarded-tls-cipher") {
        if let Ok(cipher_str) = cipher.to_str() {
            tls_info.cipher_suite = Some(cipher_str.to_string());
        }
    } else if let Some(ssl_cipher) = req.headers().get("x-forwarded-ssl-cipher") {
        if let Ok(cipher_str) = ssl_cipher.to_str() {
            tls_info.cipher_suite = Some(cipher_str.to_string());
        }
    }
    
    // Extract client certificate fingerprint if available
    if let Some(cert_fingerprint) = req.headers().get("x-forwarded-tls-client-cert-fingerprint") {
        if let Ok(fingerprint_str) = cert_fingerprint.to_str() {
            tls_info.client_cert_fingerprint = Some(fingerprint_str.to_string());
        }
    } else if let Some(ssl_client_cert) = req.headers().get("x-ssl-client-fingerprint") {
        if let Ok(fingerprint_str) = ssl_client_cert.to_str() {
            tls_info.client_cert_fingerprint = Some(fingerprint_str.to_string());
        }
    }
    
    Some(tls_info)
}

/// Detect actual connection type from HTTP request
fn detect_connection_type(req: &HttpRequest) -> String {
    // Check if this is a WebSocket upgrade request
    if let Some(upgrade) = req.headers().get("upgrade") {
        if let Ok(upgrade_str) = upgrade.to_str() {
            if upgrade_str.to_lowercase() == "websocket" {
                return "websocket".to_string();
            }
        }
    }
    
    // Check for Server-Sent Events
    if let Some(accept) = req.headers().get("accept") {
        if let Ok(accept_str) = accept.to_str() {
            if accept_str.contains("text/event-stream") {
                return "sse".to_string();
            }
        }
    }
    
    // Check for HTTP/2 or HTTP/3
    let version = req.head().version;
    match version {
        actix_web::http::Version::HTTP_2 => return "http2".to_string(),
        actix_web::http::Version::HTTP_3 => return "http3".to_string(),
        _ => {}
    }
    
    // Check if connection is secure (HTTPS)
    let is_secure = req.connection_info().scheme() == "https" ||
                   req.headers().get("x-forwarded-proto").map(|v| v.to_str().unwrap_or("")) == Some("https");
    
    if is_secure {
        "https".to_string()
    } else {
        "http".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::UserContext;
    use actix_web::test::TestRequest;

    fn create_test_remote_context() -> RemoteUserContext {
        let local_context = UserContext::default();
        let req = TestRequest::default()
            .peer_addr("192.168.1.100:12345".parse().unwrap())
            .to_http_request();
        
        RemoteUserContext::new(local_context, &req, None).unwrap()
    }

    #[tokio::test]
    async fn test_session_creation() {
        let config = SessionIsolationConfig::default();
        let manager = IsolatedSessionManager::new(config);
        
        let remote_context = create_test_remote_context();
        let req = TestRequest::default().to_http_request();
        
        let session_id = manager.create_session(&req, remote_context).await.unwrap();
        assert!(!session_id.is_empty());
        
        let session = manager.get_session(&session_id);
        assert!(session.is_some());
        
        let session = session.unwrap();
        assert_eq!(session.state, IsolationSessionState::Initializing);
    }

    #[tokio::test]
    async fn test_session_activity_tracking() {
        let config = SessionIsolationConfig::default();
        let manager = IsolatedSessionManager::new(config);
        
        let remote_context = create_test_remote_context();
        let req = TestRequest::default().to_http_request();
        
        let session_id = manager.create_session(&req, remote_context).await.unwrap();
        
        // Test activity update
        let result = manager.update_activity(&session_id);
        assert!(result.is_ok());
        
        let session = manager.get_session(&session_id).unwrap();
        assert_eq!(session.metadata.request_count, 1);
    }

    #[tokio::test]
    async fn test_session_limits() {
        let mut config = SessionIsolationConfig::default();
        config.max_sessions_per_client = 2;
        let manager = IsolatedSessionManager::new(config);
        
        let remote_context = create_test_remote_context();
        let req = TestRequest::default().to_http_request();
        
        // Create first session - should succeed
        let session1 = manager.create_session(&req, remote_context.clone()).await;
        assert!(session1.is_ok());
        
        // Create second session - should succeed
        let session2 = manager.create_session(&req, remote_context.clone()).await;
        assert!(session2.is_ok());
        
        // Create third session - should fail due to limit
        let session3 = manager.create_session(&req, remote_context).await;
        assert!(session3.is_err());
    }

    #[tokio::test]
    async fn test_session_cleanup() {
        let mut config = SessionIsolationConfig::default();
        config.max_inactivity_hours = 0; // Immediate expiry for testing
        let manager = IsolatedSessionManager::new(config);
        
        let remote_context = create_test_remote_context();
        let req = TestRequest::default().to_http_request();
        
        let session_id = manager.create_session(&req, remote_context).await.unwrap();
        
        // Session should exist
        assert!(manager.get_session(&session_id).is_some());
        
        // Run cleanup
        let cleanup_stats = manager.cleanup_sessions().await.unwrap();
        assert!(cleanup_stats.expired_sessions > 0);
        
        // Session should be removed
        assert!(manager.get_session(&session_id).is_none());
    }

    #[tokio::test]
    async fn test_session_suspension_and_resume() {
        let config = SessionIsolationConfig::default();
        let manager = IsolatedSessionManager::new(config);
        
        let remote_context = create_test_remote_context();
        let req = TestRequest::default().to_http_request();
        
        let session_id = manager.create_session(&req, remote_context).await.unwrap();
        
        // Suspend session
        let result = manager.suspend_session(&session_id, "test suspension");
        assert!(result.is_ok());
        
        let session = manager.get_session(&session_id).unwrap();
        assert_eq!(session.state, IsolationSessionState::Suspended);
        
        // Resume session
        let result = manager.resume_session(&session_id);
        assert!(result.is_ok());
        
        let session = manager.get_session(&session_id).unwrap();
        assert_eq!(session.state, IsolationSessionState::Active);
    }
}