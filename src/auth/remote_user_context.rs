//! Remote User Context System for Multi-Deployment Session Isolation
//!
//! This module addresses critical security vulnerabilities in multi-user remote deployments
//! by providing proper client identity isolation. It prevents session collisions and token
//! conflicts when multiple users connect to remote MagicTunnel instances.
//!
//! ## Security Issues Addressed:
//! - Session overwrites between different users connecting to same remote instance
//! - Token storage collisions causing authentication conflicts
//! - Identity loss in proxy → remote MagicTunnel chains
//! - Cross-user session hijacking in multi-user deployments

use crate::{auth::UserContext};
use crate::error::ProxyError;
use actix_web::HttpRequest;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::{IpAddr, SocketAddr}, path::PathBuf};
use tracing::{debug, info, trace};
use crate::error::Result;

/// Network-aware remote user context that identifies clients across network boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteUserContext {
    /// Local server context (where MagicTunnel is running)
    pub local_context: UserContext,
    /// Remote client identity information
    pub client_identity: ClientIdentity,
    /// Unique session identifier for this remote client
    pub remote_session_id: String,
    /// Combined unique identifier for isolation
    pub isolation_key: String,
    /// Session-specific storage directory
    pub remote_session_dir: PathBuf,
    /// Client authentication state
    pub client_auth_state: ClientAuthState,
}

/// Client identity extracted from network connection and MCP protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIdentity {
    /// Client IP address
    pub client_ip: IpAddr,
    /// Client port (if available)
    pub client_port: Option<u16>,
    /// Client hostname/machine name (from MCP initialization)
    pub client_hostname: Option<String>,
    /// Client username (from MCP client metadata)
    pub client_username: Option<String>,
    /// Client application/process info
    pub client_process_info: Option<ClientProcessInfo>,
    /// Custom client headers for identity validation
    pub client_headers: HashMap<String, String>,
    /// Client user agent
    pub user_agent: Option<String>,
    /// Client forwarded information (for proxy chains)
    pub forwarded_info: Option<ForwardedInfo>,
    /// MCP client capability fingerprint for validation
    pub capability_fingerprint: Option<String>,
}

/// Client process and application information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientProcessInfo {
    /// Client application name (e.g., "Claude Desktop", "VSCode")
    pub app_name: String,
    /// Client application version
    pub app_version: String,
    /// Client platform/OS
    pub platform: Option<String>,
    /// Process ID if available
    pub pid: Option<u32>,
    /// Working directory
    pub working_dir: Option<String>,
}

/// Forwarded client information for proxy chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardedInfo {
    /// Original client IP (X-Forwarded-For)
    pub original_ip: Option<IpAddr>,
    /// Proxy chain information
    pub proxy_chain: Vec<String>,
    /// Original client headers
    pub original_headers: HashMap<String, String>,
}

/// Client authentication state for session validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAuthState {
    /// Whether client is authenticated
    pub authenticated: bool,
    /// Authentication method used
    pub auth_method: Option<String>,
    /// Authentication timestamp
    pub auth_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// Session creation time
    pub session_created: chrono::DateTime<chrono::Utc>,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Client-provided identity token (optional)
    pub client_token: Option<String>,
}

/// Client identity validation result
#[derive(Debug, Clone)]
pub struct ClientValidationResult {
    pub valid: bool,
    pub client_identity: ClientIdentity,
    pub security_score: f64,
    pub validation_issues: Vec<String>,
    pub recommended_actions: Vec<String>,
}

impl RemoteUserContext {
    /// Create a new remote user context from HTTP request and MCP initialization
    pub fn new(
        local_context: UserContext,
        req: &HttpRequest,
        mcp_client_info: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<Self> {
        debug!("Creating remote user context for client connection");

        // Extract client identity from request
        let client_identity = Self::extract_client_identity(req, mcp_client_info)?;
        
        // Generate unique session ID for this remote client
        let remote_session_id = Self::generate_remote_session_id(&client_identity)?;
        
        // Create isolation key combining local and remote identity
        let isolation_key = Self::create_isolation_key(&local_context, &client_identity, &remote_session_id)?;
        
        // Create session-specific storage directory
        let remote_session_dir = Self::create_remote_session_dir(&local_context, &isolation_key)?;
        
        // Initialize client authentication state
        let client_auth_state = ClientAuthState {
            authenticated: false,
            auth_method: None,
            auth_timestamp: None,
            session_created: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            client_token: None,
        };

        let remote_context = RemoteUserContext {
            local_context,
            client_identity,
            remote_session_id,
            isolation_key,
            remote_session_dir,
            client_auth_state,
        };

        info!("Created remote user context with isolation key: {}", remote_context.isolation_key);
        Ok(remote_context)
    }

    /// Extract client identity from HTTP request
    fn extract_client_identity(
        req: &HttpRequest,
        mcp_client_info: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<ClientIdentity> {
        // Get client IP and port from connection info
        let connection_info = req.connection_info();
        let client_addr: SocketAddr = connection_info
            .realip_remote_addr()
            .unwrap_or_else(|| connection_info.peer_addr().unwrap_or("127.0.0.1:0"))
            .parse()
            .map_err(|e| ProxyError::auth(format!("Invalid client address: {}", e)))?;
            
        let client_ip = client_addr.ip();
        let client_port = if client_addr.port() == 0 { None } else { Some(client_addr.port()) };

        // Extract forwarded information
        let forwarded_info = Self::extract_forwarded_info(req);

        // Get user agent
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(String::from);

        // Extract custom client headers for identity
        let mut client_headers = HashMap::new();
        
        // Standard identity headers
        const IDENTITY_HEADERS: &[&str] = &[
            "x-client-id",
            "x-client-hostname", 
            "x-client-username",
            "x-client-app",
            "x-client-version",
            "x-client-platform",
            "x-session-token",
            "authorization",
        ];
        
        for header_name in IDENTITY_HEADERS {
            if let Some(header_value) = req.headers().get(*header_name) {
                if let Ok(value_str) = header_value.to_str() {
                    client_headers.insert(header_name.to_string(), value_str.to_string());
                }
            }
        }

        // Extract MCP client information
        let (client_hostname, client_username, client_process_info) = 
            Self::extract_mcp_client_info(mcp_client_info);

        // Generate capability fingerprint if MCP info is available
        let capability_fingerprint = mcp_client_info
            .and_then(|info| Self::generate_capability_fingerprint(info));

        let client_identity = ClientIdentity {
            client_ip,
            client_port,
            client_hostname,
            client_username,
            client_process_info,
            client_headers,
            user_agent,
            forwarded_info,
            capability_fingerprint,
        };

        trace!("Extracted client identity: {:?}", client_identity);
        Ok(client_identity)
    }

    /// Extract forwarded client information from proxy headers
    fn extract_forwarded_info(req: &HttpRequest) -> Option<ForwardedInfo> {
        let mut original_ip = None;
        let mut proxy_chain = Vec::new();
        let mut original_headers = HashMap::new();

        // Check X-Forwarded-For header
        if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded_for.to_str() {
                // Parse comma-separated list of IPs
                let ips: Vec<&str> = forwarded_str.split(',').map(|s| s.trim()).collect();
                if let Some(first_ip) = ips.first() {
                    if let Ok(ip) = first_ip.parse::<IpAddr>() {
                        original_ip = Some(ip);
                    }
                }
                proxy_chain.extend(ips.into_iter().map(String::from));
            }
        }

        // Check other forwarded headers
        const FORWARDED_HEADERS: &[&str] = &[
            "x-forwarded-proto",
            "x-forwarded-host",
            "x-forwarded-port",
            "x-real-ip",
            "x-original-forwarded-for",
        ];

        for header_name in FORWARDED_HEADERS {
            if let Some(header_value) = req.headers().get(*header_name) {
                if let Ok(value_str) = header_value.to_str() {
                    original_headers.insert(header_name.to_string(), value_str.to_string());
                }
            }
        }

        if original_ip.is_some() || !proxy_chain.is_empty() || !original_headers.is_empty() {
            Some(ForwardedInfo {
                original_ip,
                proxy_chain,
                original_headers,
            })
        } else {
            None
        }
    }

    /// Extract client information from MCP initialization data
    fn extract_mcp_client_info(
        mcp_client_info: Option<&HashMap<String, serde_json::Value>>,
    ) -> (Option<String>, Option<String>, Option<ClientProcessInfo>) {
        let client_info = match mcp_client_info {
            Some(info) => info,
            None => return (None, None, None),
        };

        // Extract hostname
        let client_hostname = client_info
            .get("hostname")
            .or_else(|| client_info.get("clientHostname"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Extract username
        let client_username = client_info
            .get("username")
            .or_else(|| client_info.get("clientUsername"))
            .or_else(|| client_info.get("user"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Extract process information
        let client_process_info = Self::extract_process_info(client_info);

        (client_hostname, client_username, client_process_info)
    }

    /// Extract client process information from MCP data
    fn extract_process_info(client_info: &HashMap<String, serde_json::Value>) -> Option<ClientProcessInfo> {
        let app_name = client_info
            .get("clientName")
            .or_else(|| client_info.get("appName"))
            .or_else(|| client_info.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from)?;

        let app_version = client_info
            .get("clientVersion")
            .or_else(|| client_info.get("appVersion"))
            .or_else(|| client_info.get("version"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| "unknown".to_string());

        let platform = client_info
            .get("platform")
            .or_else(|| client_info.get("os"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let pid = client_info
            .get("pid")
            .or_else(|| client_info.get("processId"))
            .and_then(|v| v.as_u64())
            .map(|p| p as u32);

        let working_dir = client_info
            .get("workingDirectory")
            .or_else(|| client_info.get("cwd"))
            .and_then(|v| v.as_str())
            .map(String::from);

        Some(ClientProcessInfo {
            app_name,
            app_version,
            platform,
            pid,
            working_dir,
        })
    }

    /// Generate capability fingerprint for client validation
    fn generate_capability_fingerprint(client_info: &HashMap<String, serde_json::Value>) -> Option<String> {
        use sha2::{Sha256, Digest};
        
        // Collect capability information for fingerprinting
        let mut capability_data = Vec::new();
        
        if let Some(capabilities) = client_info.get("capabilities") {
            capability_data.push(serde_json::to_string(capabilities).ok()?);
        }
        
        if let Some(client_name) = client_info.get("clientName") {
            capability_data.push(client_name.to_string());
        }
        
        if let Some(client_version) = client_info.get("clientVersion") {
            capability_data.push(client_version.to_string());
        }

        if capability_data.is_empty() {
            return None;
        }

        // Generate SHA-256 fingerprint
        let mut hasher = Sha256::new();
        for data in capability_data {
            hasher.update(data.as_bytes());
        }
        
        let fingerprint = format!("{:x}", hasher.finalize());
        Some(fingerprint[..16].to_string()) // Use first 16 chars for readability
    }

    /// Generate unique session ID for remote client
    fn generate_remote_session_id(client_identity: &ClientIdentity) -> Result<String> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        // Include client IP and current timestamp for uniqueness
        hasher.update(client_identity.client_ip.to_string().as_bytes());
        hasher.update(chrono::Utc::now().timestamp().to_string().as_bytes());
        
        // Include client-specific identifiers if available
        if let Some(hostname) = &client_identity.client_hostname {
            hasher.update(hostname.as_bytes());
        }
        
        if let Some(username) = &client_identity.client_username {
            hasher.update(username.as_bytes());
        }
        
        if let Some(process_info) = &client_identity.client_process_info {
            hasher.update(process_info.app_name.as_bytes());
            hasher.update(process_info.app_version.as_bytes());
        }
        
        // Include a random component
        hasher.update(uuid::Uuid::new_v4().to_string().as_bytes());
        
        let session_id = format!("rmt_{:x}", hasher.finalize());
        Ok(session_id[..32].to_string()) // Limit to 32 chars for readability
    }

    /// Create isolation key for session and storage isolation
    fn create_isolation_key(
        local_context: &UserContext,
        client_identity: &ClientIdentity,
        remote_session_id: &str,
    ) -> Result<String> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        // Include local context for server-side isolation
        hasher.update(local_context.get_unique_user_id().as_bytes());
        
        // Include client identity components
        hasher.update(client_identity.client_ip.to_string().as_bytes());
        
        if let Some(hostname) = &client_identity.client_hostname {
            hasher.update(hostname.as_bytes());
        }
        
        if let Some(username) = &client_identity.client_username {
            hasher.update(username.as_bytes());
        }
        
        // Include session ID for temporal isolation
        hasher.update(remote_session_id.as_bytes());
        
        let isolation_key = format!("iso_{:x}", hasher.finalize());
        Ok(isolation_key[..48].to_string()) // Use 48 chars for strong isolation
    }

    /// Create session-specific storage directory
    fn create_remote_session_dir(
        local_context: &UserContext,
        isolation_key: &str,
    ) -> Result<PathBuf> {
        let remote_sessions_dir = local_context.session_dir.join("remote_sessions");
        let specific_session_dir = remote_sessions_dir.join(isolation_key);
        
        // Create directory if it doesn't exist
        if !specific_session_dir.exists() {
            std::fs::create_dir_all(&specific_session_dir)
                .map_err(|e| ProxyError::auth(format!("Failed to create remote session directory: {}", e)))?;
                
            // Set secure permissions (0700 - owner only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut permissions = std::fs::metadata(&specific_session_dir)
                    .map_err(|e| ProxyError::auth(format!("Failed to get directory metadata: {}", e)))?
                    .permissions();
                permissions.set_mode(0o700);
                std::fs::set_permissions(&specific_session_dir, permissions)
                    .map_err(|e| ProxyError::auth(format!("Failed to set directory permissions: {}", e)))?;
            }
        }
        
        debug!("Created remote session directory: {}", specific_session_dir.display());
        Ok(specific_session_dir)
    }

    /// Validate client identity and detect potential security issues
    pub fn validate_client_identity(&self) -> ClientValidationResult {
        let mut validation_issues = Vec::new();
        let mut recommended_actions = Vec::new();
        let mut security_score: f64 = 1.0;

        // Check for missing client information
        if self.client_identity.client_hostname.is_none() {
            validation_issues.push("Missing client hostname".to_string());
            security_score -= 0.1;
            recommended_actions.push("Include client hostname in MCP initialization".to_string());
        }

        if self.client_identity.client_username.is_none() {
            validation_issues.push("Missing client username".to_string());
            security_score -= 0.1;
            recommended_actions.push("Include client username in MCP initialization".to_string());
        }

        // Check for localhost connections (potential security risk in production)
        if self.client_identity.client_ip.is_loopback() {
            if std::env::var("MAGICTUNNEL_PRODUCTION").is_ok() {
                validation_issues.push("Localhost connection in production mode".to_string());
                security_score -= 0.2;
                recommended_actions.push("Review localhost access in production".to_string());
            }
        }

        // Check for missing process information
        if self.client_identity.client_process_info.is_none() {
            validation_issues.push("Missing client process information".to_string());
            security_score -= 0.05;
        }

        // Check capability fingerprint
        if self.client_identity.capability_fingerprint.is_none() {
            validation_issues.push("Missing capability fingerprint".to_string());
            security_score -= 0.05;
            recommended_actions.push("Include client capabilities in MCP initialization".to_string());
        }

        // Check authentication state
        if !self.client_auth_state.authenticated {
            validation_issues.push("Client not authenticated".to_string());
            security_score -= 0.3;
            recommended_actions.push("Complete client authentication".to_string());
        }

        // Check session age
        let session_age = chrono::Utc::now() - self.client_auth_state.session_created;
        if session_age > chrono::Duration::hours(24) {
            validation_issues.push("Long-running session".to_string());
            security_score -= 0.1;
            recommended_actions.push("Consider session refresh or re-authentication".to_string());
        }

        // Ensure security score is between 0 and 1
        security_score = security_score.max(0.0).min(1.0);

        ClientValidationResult {
            valid: security_score >= 0.5 && validation_issues.len() < 3,
            client_identity: self.client_identity.clone(),
            security_score,
            validation_issues,
            recommended_actions,
        }
    }

    /// Get a unique identifier for this remote client session
    pub fn get_remote_client_id(&self) -> String {
        format!("{}::{}", self.isolation_key, self.remote_session_id)
    }

    /// Get session file path for remote client-specific data
    pub fn get_remote_session_file(&self, filename: &str) -> PathBuf {
        self.remote_session_dir.join(filename)
    }

    /// Update client authentication state
    pub fn update_auth_state(&mut self, authenticated: bool, auth_method: Option<String>, client_token: Option<String>) {
        self.client_auth_state.authenticated = authenticated;
        self.client_auth_state.auth_method = auth_method;
        self.client_auth_state.client_token = client_token;
        
        if authenticated {
            self.client_auth_state.auth_timestamp = Some(chrono::Utc::now());
        }
        
        self.client_auth_state.last_activity = chrono::Utc::now();
    }

    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.client_auth_state.last_activity = chrono::Utc::now();
    }

    /// Check if session is expired based on inactivity
    pub fn is_session_expired(&self, max_inactivity_hours: u64) -> bool {
        let max_inactivity = chrono::Duration::hours(max_inactivity_hours as i64);
        let inactivity = chrono::Utc::now() - self.client_auth_state.last_activity;
        inactivity > max_inactivity
    }

    /// Convert to UserContext for backward compatibility
    pub fn to_user_context(&self) -> UserContext {
        // Create a modified UserContext that includes remote client identity
        let mut modified_context = self.local_context.clone();
        
        // Override session directory to use remote-specific directory
        modified_context.session_dir = self.remote_session_dir.clone();
        
        // Override hostname to include client information for uniqueness
        if let Some(client_hostname) = &self.client_identity.client_hostname {
            modified_context.hostname = format!("{}+{}", 
                self.local_context.hostname, 
                client_hostname
            );
        }
        
        // Override username to include client information if available
        if let Some(client_username) = &self.client_identity.client_username {
            modified_context.username = format!("{}+{}", 
                self.local_context.username, 
                client_username
            );
        }
        
        modified_context
    }

    /// Get display name for logging and debugging
    pub fn display_name(&self) -> String {
        let client_info = match (&self.client_identity.client_username, &self.client_identity.client_hostname) {
            (Some(username), Some(hostname)) => format!("{}@{}", username, hostname),
            (Some(username), None) => username.clone(),
            (None, Some(hostname)) => hostname.clone(),
            (None, None) => "unknown".to_string(),
        };
        
        format!("{}[{}]", client_info, self.client_identity.client_ip)
    }
}

impl std::fmt::Display for RemoteUserContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RemoteUserContext({})", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    use std::collections::HashMap;

    fn create_test_local_context() -> UserContext {
        UserContext::default()
    }

    #[test]
    fn test_remote_user_context_creation() {
        let local_context = create_test_local_context();
        let req = TestRequest::default()
            .insert_header(("user-agent", "test-client/1.0"))
            .insert_header(("x-client-hostname", "test-machine"))
            .insert_header(("x-client-username", "testuser"))
            .peer_addr("192.168.1.100:12345".parse().unwrap())
            .to_http_request();

        let mut mcp_info = HashMap::new();
        mcp_info.insert("clientName".to_string(), serde_json::Value::String("TestClient".to_string()));
        mcp_info.insert("clientVersion".to_string(), serde_json::Value::String("1.0.0".to_string()));

        let remote_context = RemoteUserContext::new(local_context, &req, Some(&mcp_info));
        assert!(remote_context.is_ok());

        let remote_context = remote_context.unwrap();
        assert_eq!(remote_context.client_identity.client_ip.to_string(), "192.168.1.100");
        assert_eq!(remote_context.client_identity.client_port, Some(12345));
        assert!(remote_context.client_identity.client_hostname.is_some());
        assert!(remote_context.client_identity.client_username.is_some());
    }

    #[test]
    fn test_client_identity_extraction() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.1, 198.51.100.1"))
            .insert_header(("x-client-hostname", "remote-machine"))
            .peer_addr("10.0.0.1:8080".parse().unwrap())
            .to_http_request();

        let client_identity = RemoteUserContext::extract_client_identity(&req, None);
        assert!(client_identity.is_ok());

        let client_identity = client_identity.unwrap();
        assert!(client_identity.forwarded_info.is_some());
        
        let forwarded = client_identity.forwarded_info.unwrap();
        assert_eq!(forwarded.original_ip.unwrap().to_string(), "203.0.113.1");
        assert_eq!(forwarded.proxy_chain.len(), 2);
    }

    #[test]
    fn test_isolation_key_generation() {
        let local_context = create_test_local_context();
        
        let mut client_identity = ClientIdentity {
            client_ip: "192.168.1.100".parse().unwrap(),
            client_port: Some(12345),
            client_hostname: Some("test-machine".to_string()),
            client_username: Some("testuser".to_string()),
            client_process_info: None,
            client_headers: HashMap::new(),
            user_agent: None,
            forwarded_info: None,
            capability_fingerprint: None,
        };

        let session_id = RemoteUserContext::generate_remote_session_id(&client_identity).unwrap();
        let isolation_key1 = RemoteUserContext::create_isolation_key(&local_context, &client_identity, &session_id).unwrap();

        // Change client identity slightly
        client_identity.client_username = Some("different-user".to_string());
        let isolation_key2 = RemoteUserContext::create_isolation_key(&local_context, &client_identity, &session_id).unwrap();

        // Keys should be different for different users
        assert_ne!(isolation_key1, isolation_key2);
    }

    #[test]
    fn test_session_validation() {
        let local_context = create_test_local_context();
        let req = TestRequest::default()
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .to_http_request();

        let mut remote_context = RemoteUserContext::new(local_context, &req, None).unwrap();
        
        // Test initial validation (should have issues due to missing info)
        let validation = remote_context.validate_client_identity();
        assert!(!validation.valid);
        assert!(!validation.validation_issues.is_empty());

        // Update authentication state
        remote_context.update_auth_state(true, Some("oauth".to_string()), None);
        
        // Should still have issues due to missing client info, but be better
        let validation2 = remote_context.validate_client_identity();
        assert!(validation2.security_score > validation.security_score);
    }

    #[test]
    fn test_session_expiration() {
        let local_context = create_test_local_context();
        let req = TestRequest::default()
            .peer_addr("192.168.1.100:12345".parse().unwrap())
            .to_http_request();

        let mut remote_context = RemoteUserContext::new(local_context, &req, None).unwrap();
        
        // Fresh session should not be expired
        assert!(!remote_context.is_session_expired(24));
        
        // Manually set old last activity
        remote_context.client_auth_state.last_activity = chrono::Utc::now() - chrono::Duration::hours(25);
        
        // Should now be expired
        assert!(remote_context.is_session_expired(24));
    }

    #[test]
    fn test_display_formatting() {
        let local_context = create_test_local_context();
        let req = TestRequest::default()
            .insert_header(("x-client-hostname", "test-machine"))
            .insert_header(("x-client-username", "testuser"))
            .peer_addr("192.168.1.100:12345".parse().unwrap())
            .to_http_request();

        let remote_context = RemoteUserContext::new(local_context, &req, None).unwrap();
        let display = remote_context.display_name();
        
        assert!(display.contains("testuser@test-machine"));
        assert!(display.contains("192.168.1.100"));
    }
}