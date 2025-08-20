//! Configuration types for OAuth-enabled MCP discovery

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::config::secret_string;
use secrecy::Secret;

/// Configuration for OAuth-enabled MCP server discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthMcpServerConfig {
    /// Enable OAuth discovery for this server
    pub enabled: bool,
    /// MCP connection endpoint (e.g., https://mcp.globalping.dev/sse)
    pub base_url: String,
    /// OAuth termination preference - if true, MT handles OAuth; if false, forwards to client
    pub oauth_termination_here: bool,
    /// Discovery endpoint for OAuth metadata (RFC 8414)
    pub discovery_endpoint: Option<String>,
    /// OAuth provider configuration (for static credentials)
    pub oauth_provider: Option<String>,
    /// Required scopes override (optional - uses discovered scopes by default)
    pub required_scopes_override: Option<Vec<String>>,
    /// Manual OAuth metadata (fallback when RFC 8414/9728 discovery fails)
    pub manual_oauth_metadata: Option<ManualOAuthMetadata>,
    /// Dynamic registration configuration (RFC 7591)
    pub enable_dynamic_registration: Option<DynamicRegistrationConfig>,
    /// Static OAuth credentials (alternative to dynamic registration)
    pub static_credentials: Option<StaticOAuthCredentials>,
    /// Transport configuration
    pub transport: McpTransportConfig,
    /// Connection settings
    pub connection: ConnectionConfig,
}

/// Dynamic OAuth client registration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicRegistrationConfig {
    /// Enable dynamic registration (RFC 7591)
    pub enabled: bool,
    /// OAuth discovery base URL (e.g., https://mcp.globalping.dev/) - if not specified, derived from base_url
    pub oauth_discovery_base_url: Option<String>,
    /// Client name template (supports {{server_name}}, {{hostname}}, {{port}})
    pub client_name: String,
    /// Redirect URI template
    pub redirect_uri_template: String,
    /// Requested scopes override (optional - uses discovered scopes by default)
    pub requested_scopes_override: Option<Vec<String>>,
    /// Grant types override (optional - uses discovered grant types by default)
    pub grant_types_override: Option<Vec<String>>,
    /// Response types override (optional - uses discovered response types by default)
    pub response_types_override: Option<Vec<String>>,
    /// Application type (web, native)
    pub application_type: String,
    /// Client URI (optional)
    pub client_uri: Option<String>,
    /// Logo URI (optional)
    pub logo_uri: Option<String>,
    /// Terms of service URI (optional)
    pub tos_uri: Option<String>,
    /// Privacy policy URI (optional)
    pub policy_uri: Option<String>,
}

/// Static OAuth credentials configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticOAuthCredentials {
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret (stored securely)
    #[serde(with = "secret_string")]
    pub client_secret: Secret<String>,
    /// OAuth scopes
    pub scopes: Vec<String>,
    /// Authorization endpoint URL
    pub authorization_endpoint: String,
    /// Token endpoint URL
    pub token_endpoint: String,
}

/// MCP transport configuration for OAuth-enabled servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTransportConfig {
    /// Transport type (sse, http, websocket)
    pub transport_type: String,
    /// Connection timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Keep-alive settings
    pub keep_alive: Option<KeepAliveConfig>,
}

/// Keep-alive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepAliveConfig {
    /// Enable keep-alive
    pub enabled: bool,
    /// Keep-alive interval in seconds
    pub interval_seconds: u64,
    /// Keep-alive timeout in seconds
    pub timeout_seconds: u64,
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Connection pool size
    pub pool_size: u32,
    /// Idle connection timeout in seconds
    pub idle_timeout_seconds: u64,
    /// Enable connection reuse
    pub reuse_connections: bool,
    /// Enable HTTP/2 (for HTTP transport)
    pub enable_http2: bool,
    /// Custom headers
    pub headers: HashMap<String, String>,
}

impl Default for OAuthMcpServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "https://mcp.example.com/sse".to_string(),
            oauth_termination_here: true,
            discovery_endpoint: None,
            oauth_provider: None,
            required_scopes_override: None, // Uses discovered scopes by default
            manual_oauth_metadata: None, // Uses RFC 8414/9728 discovery by default
            enable_dynamic_registration: Some(DynamicRegistrationConfig::default()),
            static_credentials: None,
            transport: McpTransportConfig::default(),
            connection: ConnectionConfig::default(),
        }
    }
}

impl Default for DynamicRegistrationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            oauth_discovery_base_url: None, // Will be derived from base_url
            client_name: "MagicTunnel-{{hostname}}".to_string(),
            redirect_uri_template: "http://localhost:{{port}}/auth/callback/{{server_name}}".to_string(),
            requested_scopes_override: None, // Uses discovered scopes by default
            grant_types_override: None, // Uses discovered grant types by default
            response_types_override: None, // Uses discovered response types by default
            application_type: "web".to_string(),
            client_uri: Some("https://github.com/magictunnel/magictunnel".to_string()),
            logo_uri: None,
            tos_uri: None,
            policy_uri: None,
        }
    }
}

impl Default for McpTransportConfig {
    fn default() -> Self {
        Self {
            transport_type: "streamable-http".to_string(), // MCP 2025-06-18 preferred transport
            timeout_seconds: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
            keep_alive: Some(KeepAliveConfig::default()),
        }
    }
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 30,
            timeout_seconds: 10,
        }
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            pool_size: 5,
            idle_timeout_seconds: 300,
            reuse_connections: true,
            enable_http2: false,
            headers: HashMap::new(),
        }
    }
}

/// OAuth-enabled MCP servers configuration (extends external-mcp-servers.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthMcpServersConfig {
    /// OAuth-enabled MCP servers (each server is self-contained)
    pub oauth_mcp_servers: HashMap<String, OAuthMcpServerConfig>,
}


/// Manual OAuth metadata (fallback when RFC 8414/9728 discovery fails)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualOAuthMetadata {
    /// Authorization endpoint URL (required)
    pub authorization_endpoint: String,
    /// Token endpoint URL (required)
    pub token_endpoint: String,
    /// Registration endpoint URL (optional, for RFC 7591)
    pub registration_endpoint: Option<String>,
    /// Manually specified supported scopes (fallback)
    pub scopes_supported: Option<Vec<String>>,
    /// Manually specified supported grant types (fallback)
    pub grant_types_supported: Option<Vec<String>>,
    /// Manually specified supported response types (fallback)
    pub response_types_supported: Option<Vec<String>>,
    /// Manually specified PKCE methods (fallback)
    pub code_challenge_methods_supported: Option<Vec<String>>,
}

/// Token refresh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshConfig {
    /// Enable automatic token refresh
    pub enabled: bool,
    /// Refresh tokens before they expire (seconds)
    pub refresh_before_expiry_seconds: u64,
    /// Maximum retry attempts for token refresh
    pub max_refresh_attempts: u32,
    /// Retry delay for failed refresh attempts (milliseconds)
    pub refresh_retry_delay_ms: u64,
}

impl Default for OAuthMcpServersConfig {
    fn default() -> Self {
        Self {
            oauth_mcp_servers: HashMap::new(),
        }
    }
}


impl Default for TokenRefreshConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            refresh_before_expiry_seconds: 300, // 5 minutes
            max_refresh_attempts: 3,
            refresh_retry_delay_ms: 1000,
        }
    }
}

impl OAuthMcpServerConfig {
    /// Get the OAuth discovery base URL, deriving it from base_url if not explicitly set
    pub fn get_oauth_discovery_base_url(&self) -> String {
        // First check if it's set in dynamic registration config
        if let Some(dynamic_reg) = &self.enable_dynamic_registration {
            if dynamic_reg.enabled {
                if let Some(explicit_url) = &dynamic_reg.oauth_discovery_base_url {
                    return explicit_url.clone();
                }
            }
        }
        
        // Extract the base URL from the MCP connection endpoint
        // e.g., "https://mcp.globalping.dev/sse" -> "https://mcp.globalping.dev/"
        match self.base_url.parse::<url::Url>() {
            Ok(parsed_url) => {
                format!("{}://{}/", parsed_url.scheme(), parsed_url.host_str().unwrap_or(""))
            }
            Err(_) => {
                // Fallback: try to extract manually
                if let Some(scheme_end) = self.base_url.find("://") {
                    if let Some(path_start) = self.base_url[scheme_end + 3..].find('/') {
                        format!("{}/", &self.base_url[..scheme_end + 3 + path_start + 1])
                    } else {
                        format!("{}/", self.base_url)
                    }
                } else {
                    self.base_url.clone()
                }
            }
        }
    }
    
    /// Get the MCP connection endpoint URL
    pub fn get_mcp_connection_url(&self) -> String {
        self.base_url.clone()
    }
    
    /// Check if dynamic registration is enabled
    pub fn is_dynamic_registration_enabled(&self) -> bool {
        self.enable_dynamic_registration.as_ref().map(|config| config.enabled).unwrap_or(false)
    }
    
    /// Get dynamic registration config if enabled
    pub fn get_dynamic_registration_config(&self) -> Option<&DynamicRegistrationConfig> {
        self.enable_dynamic_registration.as_ref().filter(|config| config.enabled)
    }
}