//! Configuration management for the MCP Proxy

use crate::error::{ProxyError, Result};

// Default functions for serde
fn default_protocol_version() -> String {
    "2025-06-18".to_string()
}

fn default_client_name() -> String {
    env!("CARGO_PKG_NAME").to_string()
}

fn default_client_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Tool visibility configuration for Smart Tool Discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibilityConfig {
    /// Hide individual tools when smart discovery is enabled (default: false)
    pub hide_individual_tools: bool,
    /// Only expose smart_tool_discovery tool (default: false)
    pub expose_smart_discovery_only: bool,
    /// Allow individual tools to override hidden setting (default: true)
    pub allow_override: bool,
    /// Default hidden state for new tools (default: false)
    pub default_hidden: bool,
}

impl Default for VisibilityConfig {
    fn default() -> Self {
        Self {
            hide_individual_tools: false,
            expose_smart_discovery_only: false,
            allow_override: true,
            default_hidden: false,
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Registry configuration
    pub registry: RegistryConfig,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
    /// Logging configuration
    pub logging: Option<LoggingConfig>,
    // Hybrid routing removed - use external_mcp instead
    /// External MCP discovery configuration (unified local/remote)
    pub external_mcp: Option<ExternalMcpConfig>,
    /// MCP client configuration
    pub mcp_client: Option<McpClientConfig>,
    /// Global conflict resolution configuration for tools from different sources
    pub conflict_resolution: Option<crate::routing::ConflictResolutionConfig>,
    /// Tool visibility configuration for Smart Tool Discovery
    pub visibility: Option<VisibilityConfig>,
    /// Smart Discovery configuration
    pub smart_discovery: Option<crate::discovery::SmartDiscoveryConfig>,
    /// Security configuration
    pub security: Option<crate::security::SecurityConfig>,
    /// Streamable HTTP Transport configuration (MCP 2025-06-18)
    pub streamable_http: Option<StreamableHttpTransportConfig>,
    /// MCP 2025-06-18 Sampling service configuration
    pub sampling: Option<SamplingConfig>,
    /// MCP 2025-06-18 Elicitation service configuration
    pub elicitation: Option<ElicitationConfig>,
    /// Prompt generation service configuration
    pub prompt_generation: Option<crate::mcp::PromptGenerationConfig>,
    /// Resource generation service configuration
    pub resource_generation: Option<crate::mcp::ResourceGenerationConfig>,
    /// Content storage service configuration
    pub content_storage: Option<crate::mcp::ContentStorageConfig>,
    /// External content management configuration
    pub external_content: Option<crate::mcp::ExternalContentConfig>,
    /// Enhancement storage configuration for persistent tool descriptions
    pub enhancement_storage: Option<crate::discovery::EnhancementStorageConfig>,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,
    /// Port to bind to
    pub port: u16,
    /// Enable WebSocket support
    pub websocket: bool,
    /// Request timeout in seconds
    pub timeout: u64,
    /// TLS configuration
    pub tls: Option<TlsConfig>,
}

/// TLS/SSL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// TLS mode (disabled, application, behind_proxy, auto)
    pub mode: TlsMode,
    /// Path to certificate file (PEM format)
    pub cert_file: Option<String>,
    /// Path to private key file (PEM format)
    pub key_file: Option<String>,
    /// Path to CA certificate file (optional)
    pub ca_file: Option<String>,
    /// Whether running behind a reverse proxy
    pub behind_proxy: bool,
    /// List of trusted proxy IP ranges (CIDR notation)
    pub trusted_proxies: Vec<String>,
    /// Minimum TLS version (1.2, 1.3)
    pub min_tls_version: String,
    /// Custom cipher suites (optional)
    pub cipher_suites: Option<Vec<String>>,
    /// Enable HTTP Strict Transport Security (HSTS)
    pub hsts_enabled: bool,
    /// HSTS max age in seconds
    pub hsts_max_age: u64,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// Enable HSTS preload
    pub hsts_preload: bool,
    /// Require X-Forwarded-Proto header when behind proxy
    pub require_forwarded_proto: bool,
    /// Require X-Forwarded-For header when behind proxy
    pub require_forwarded_for: bool,
    /// Auto-detection headers to check
    pub auto_detect_headers: Vec<String>,
    /// Fallback mode if auto-detection fails
    pub fallback_mode: TlsMode,
}

/// TLS operation mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TlsMode {
    /// TLS disabled (plain HTTP)
    Disabled,
    /// Application-level TLS (direct HTTPS)
    Application,
    /// Behind reverse proxy (proxy handles TLS)
    BehindProxy,
    /// Auto-detect based on headers
    Auto,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry type (file, database, etc.)
    pub r#type: String,
    /// Paths to capability files/directories
    pub paths: Vec<String>,
    /// Enable hot reloading
    pub hot_reload: bool,
    /// Validation settings
    pub validation: ValidationConfig,
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable strict validation
    pub strict: bool,
    /// Allow unknown fields
    pub allow_unknown_fields: bool,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication (default: false for backward compatibility)
    pub enabled: bool,
    /// Authentication type (api_key, oauth, jwt)
    pub r#type: AuthType,
    /// API key configuration (for api_key auth)
    pub api_keys: Option<ApiKeyConfig>,
    /// OAuth configuration (for oauth auth)
    pub oauth: Option<OAuthConfig>,
    /// JWT configuration (for jwt auth)
    pub jwt: Option<JwtConfig>,
}

/// Authentication type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    /// No authentication
    None,
    /// API key authentication
    ApiKey,
    /// OAuth 2.0 authentication
    OAuth,
    /// JWT token authentication
    Jwt,
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthType::None => write!(f, "none"),
            AuthType::ApiKey => write!(f, "api_key"),
            AuthType::OAuth => write!(f, "oauth"),
            AuthType::Jwt => write!(f, "jwt"),
        }
    }
}

impl PartialEq<&str> for AuthType {
    fn eq(&self, other: &&str) -> bool {
        self.to_string() == *other
    }
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// List of valid API keys with metadata
    pub keys: Vec<ApiKeyEntry>,
    /// Require API key in header (default: true)
    pub require_header: bool,
    /// Header name for API key (default: "Authorization")
    pub header_name: String,
    /// Expected header format (default: "Bearer {key}")
    pub header_format: String,
}

/// Individual API key entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    /// The API key value
    pub key: String,
    /// Human-readable name for this key
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Permissions for this key
    pub permissions: Vec<String>,
    /// Optional expiration timestamp (ISO 8601)
    pub expires_at: Option<String>,
    /// Whether this key is active
    pub active: bool,
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret key for validation
    pub secret: String,
    /// JWT algorithm (HS256, RS256, etc.)
    pub algorithm: String,
    /// Token expiration time in seconds
    pub expiration: u64,
    /// JWT issuer
    pub issuer: Option<String>,
    /// JWT audience
    pub audience: Option<String>,
}

/// OAuth 2.1 configuration with Resource Indicators (RFC 8707) support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// OAuth provider
    pub provider: String,
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Authorization URL
    pub auth_url: String,
    /// Token URL
    pub token_url: String,
    /// Enable OAuth 2.1 features (PKCE, Resource Indicators)
    #[serde(default = "default_oauth_2_1_enabled")]
    pub oauth_2_1_enabled: bool,
    /// Enable Resource Indicators (RFC 8707) support
    #[serde(default = "default_resource_indicators_enabled")]
    pub resource_indicators_enabled: bool,
    /// Default resource URIs for Resource Indicators
    #[serde(default = "default_oauth_resources")]
    pub default_resources: Vec<String>,
    /// Default audience for tokens
    #[serde(default = "default_oauth_audience")]
    pub default_audience: Vec<String>,
    /// Require explicit resource specification
    #[serde(default)]
    pub require_explicit_resources: bool,
}

/// Default value for OAuth 2.1 enabled (true for MCP 2025-06-18 compliance)
fn default_oauth_2_1_enabled() -> bool {
    true
}

/// Default value for Resource Indicators enabled (true for MCP 2025-06-18 compliance)
fn default_resource_indicators_enabled() -> bool {
    true
}

/// Default resources for OAuth Resource Indicators
fn default_oauth_resources() -> Vec<String> {
    vec![
        "https://api.magictunnel.io/mcp".to_string(),
        "urn:magictunnel:mcp:*".to_string(),
    ]
}

/// Default audience for OAuth tokens
fn default_oauth_audience() -> Vec<String> {
    vec!["magictunnel-mcp-server".to_string()]
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log format (json, text)
    pub format: String,
    /// Log file path (optional)
    pub file: Option<String>,
}

// Hybrid routing configuration removed - use external_mcp instead

/// Conflict resolution strategy for duplicate tool names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    /// Local tools take precedence over MCP proxy tools
    #[serde(rename = "local_first")]
    LocalFirst,
    /// MCP proxy tools take precedence over local tools
    #[serde(rename = "proxy_first")]
    ProxyFirst,
    /// Use the first tool found (discovery order dependent)
    #[serde(rename = "first_found")]
    FirstFound,
    /// Reject tools with conflicting names (error on conflict)
    #[serde(rename = "reject")]
    Reject,
    /// Create prefixed names for conflicting tools
    #[serde(rename = "prefix")]
    Prefix,
}

/// Tool aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    /// Whether aggregation is enabled
    pub enabled: bool,
    /// How often to refresh MCP server tools (seconds)
    pub refresh_interval: u64,
    /// Maximum age of cached MCP tools (seconds)
    pub cache_ttl: u64,
    /// Whether to include MCP server tools in aggregation
    pub include_mcp_tools: bool,
    /// Whether to include local tools in aggregation
    pub include_local_tools: bool,
}

// HybridRoutingConfig Default implementation removed

impl Default for ConflictResolutionStrategy {
    fn default() -> Self {
        ConflictResolutionStrategy::LocalFirst
    }
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            refresh_interval: 300, // 5 minutes
            cache_ttl: 600,        // 10 minutes
            include_mcp_tools: true,
            include_local_tools: true,
        }
    }
}

// HybridRoutingConfig implementation removed

impl ConflictResolutionStrategy {
    /// Convert to routing module's ConflictResolutionConfig
    pub fn to_routing_config(&self) -> crate::routing::ConflictResolutionConfig {
        crate::routing::ConflictResolutionConfig {
            strategy: match self {
                ConflictResolutionStrategy::LocalFirst => crate::routing::conflict_resolution::ConflictResolutionStrategy::LocalFirst,
                ConflictResolutionStrategy::ProxyFirst => crate::routing::conflict_resolution::ConflictResolutionStrategy::ProxyFirst,
                ConflictResolutionStrategy::FirstFound => crate::routing::conflict_resolution::ConflictResolutionStrategy::FirstFound,
                ConflictResolutionStrategy::Reject => crate::routing::conflict_resolution::ConflictResolutionStrategy::Reject,
                ConflictResolutionStrategy::Prefix => crate::routing::conflict_resolution::ConflictResolutionStrategy::Prefix,
            },
            local_prefix: "local".to_string(),
            proxy_prefix_format: "{server}".to_string(),
            log_conflicts: true,
            include_conflict_metadata: true,
        }
    }
}



/// MCP Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientConfig {
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay in seconds
    pub reconnect_delay_secs: u64,
    /// Enable automatic reconnection
    pub auto_reconnect: bool,
    /// MCP protocol version to use
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    /// Client name for MCP handshake
    #[serde(default = "default_client_name")]
    pub client_name: String,
    /// Client version for MCP handshake
    #[serde(default = "default_client_version")]
    pub client_version: String,
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 30,
            request_timeout_secs: 60,
            max_reconnect_attempts: 5,
            reconnect_delay_secs: 5,
            auto_reconnect: true,
            protocol_version: "2025-06-18".to_string(),
            client_name: env!("CARGO_PKG_NAME").to_string(),
            client_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}





/// External MCP Configuration (unified local/remote using Claude Desktop format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMcpConfig {
    /// Whether external MCP discovery is enabled
    pub enabled: bool,
    /// Path to external MCP servers configuration file (Claude Desktop format)
    pub config_file: String,
    /// Directory where capability files will be generated
    pub capabilities_output_dir: String,
    /// How often to refresh capabilities (in minutes)
    pub refresh_interval_minutes: u64,
    /// Container runtime configuration
    pub containers: Option<ContainerConfig>,
}



/// Container runtime configuration for External MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container runtime to use (docker, podman, etc.)
    pub runtime: String,
    /// Default container image for node-based MCP servers
    pub node_image: Option<String>,
    /// Default container image for python-based MCP servers
    pub python_image: Option<String>,
    /// Container network mode
    pub network_mode: Option<String>,
    /// Additional container run arguments
    pub run_args: Vec<String>,
}

/// Claude Desktop MCP Server Configuration (exact format compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Command to execute (e.g., "npx", "uv", "docker")
    pub command: String,
    /// Arguments to pass to the command
    pub args: Vec<String>,
    /// Environment variables for the process
    pub env: Option<std::collections::HashMap<String, String>>,
    /// Working directory for the process
    pub cwd: Option<String>,
}

/// External MCP Servers Configuration (Claude Desktop format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMcpServersConfig {
    /// MCP servers configuration (matches Claude Desktop format exactly)
    #[serde(rename = "mcpServers")]
    pub mcp_servers: Option<std::collections::HashMap<String, McpServerConfig>>,
    /// HTTP MCP services configuration
    #[serde(rename = "httpServices")]
    pub http_services: Option<std::collections::HashMap<String, HttpServiceConfig>>,
    /// SSE MCP services configuration
    #[serde(rename = "sseServices")]
    pub sse_services: Option<std::collections::HashMap<String, SseServiceConfig>>,
    /// WebSocket MCP services configuration (future)
    #[serde(rename = "websocketServices")]
    pub websocket_services: Option<std::collections::HashMap<String, WebSocketServiceConfig>>,
}

/// HTTP MCP Service Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServiceConfig {
    /// Whether this service is enabled
    pub enabled: bool,
    /// Base URL for the HTTP MCP service
    pub base_url: String,
    /// Authentication configuration
    pub auth: HttpAuthType,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// Maximum retry attempts
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    /// Retry delay in milliseconds
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,
    /// Connection pool max idle connections
    pub max_idle_connections: Option<usize>,
    /// Connection pool idle timeout in seconds
    pub idle_timeout: Option<u64>,
}

/// SSE MCP Service Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseServiceConfig {
    /// Whether this service is enabled
    pub enabled: bool,
    /// Base URL for the SSE MCP service
    pub base_url: String,
    /// Authentication configuration
    pub auth: SseAuthType,
    /// Whether this service supports only single session
    #[serde(default = "default_single_session")]
    pub single_session: bool,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub connection_timeout: u64,
    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,
    /// Maximum queue size for single-session services
    #[serde(default = "default_max_queue_size")]
    pub max_queue_size: usize,
    /// Heartbeat interval in seconds (0 to disable)
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: u64,
    /// Enable automatic reconnection
    #[serde(default = "default_reconnect")]
    pub reconnect: bool,
    /// Maximum reconnection attempts (0 for unlimited)
    #[serde(default = "default_max_reconnect_attempts")]
    pub max_reconnect_attempts: u32,
    /// Reconnection delay in milliseconds
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_ms: u64,
    /// Maximum reconnection delay in milliseconds
    #[serde(default = "default_max_reconnect_delay")]
    pub max_reconnect_delay_ms: u64,
}

/// WebSocket MCP Service Configuration (future)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketServiceConfig {
    /// Whether this service is enabled
    pub enabled: bool,
    /// Base URL for the WebSocket MCP service
    pub base_url: String,
    /// Authentication configuration
    pub auth: WebSocketAuthType,
    /// Ping interval in seconds
    #[serde(default = "default_timeout")]
    pub ping_interval: u64,
    /// Pong timeout in seconds
    #[serde(default = "default_pong_timeout")]
    pub pong_timeout: u64,
    /// Enable automatic reconnection
    #[serde(default = "default_reconnect")]
    pub reconnect: bool,
    /// Maximum reconnection attempts
    #[serde(default = "default_max_reconnect_attempts")]
    pub max_reconnect_attempts: u32,
}

/// HTTP Authentication Type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HttpAuthType {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "bearer")]
    Bearer { token: String },
    #[serde(rename = "api_key")]
    ApiKey { header: String, key: String },
    #[serde(rename = "basic")]
    Basic { username: String, password: String },
}

/// SSE Authentication Type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SseAuthType {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "bearer")]
    Bearer { token: String },
    #[serde(rename = "api_key")]
    ApiKey { header: String, key: String },
    #[serde(rename = "query_param")]
    QueryParam { param: String, value: String },
}

/// WebSocket Authentication Type (future)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketAuthType {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "bearer")]
    Bearer { token: String },
    #[serde(rename = "api_key")]
    ApiKey { header: String, key: String },
}

// Default value functions
fn default_timeout() -> u64 { 30 }
fn default_retry_attempts() -> u32 { 3 }
fn default_retry_delay() -> u64 { 1000 }
fn default_single_session() -> bool { true }
fn default_request_timeout() -> u64 { 60 }
fn default_max_queue_size() -> usize { 100 }
fn default_heartbeat_interval() -> u64 { 30 }
fn default_reconnect() -> bool { true }
fn default_max_reconnect_attempts() -> u32 { 10 }
fn default_reconnect_delay() -> u64 { 1000 }
fn default_max_reconnect_delay() -> u64 { 30000 }
fn default_pong_timeout() -> u64 { 10 }


impl Default for ExternalMcpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            config_file: "./external-mcp-servers.yaml".to_string(),
            capabilities_output_dir: "./capabilities/external-mcp".to_string(),
            refresh_interval_minutes: 60,
            containers: Some(ContainerConfig::default()),
        }
    }
}

impl Default for ExternalMcpServersConfig {
    fn default() -> Self {
        Self {
            mcp_servers: None,
            http_services: None,
            sse_services: None,
            websocket_services: None,
        }
    }
}

// Conversion implementations for HTTP services
impl From<&HttpServiceConfig> for crate::mcp::clients::HttpClientConfig {
    fn from(config: &HttpServiceConfig) -> Self {
        Self {
            base_url: config.base_url.clone(),
            auth: (&config.auth).into(),
            timeout: config.timeout,
            retry_attempts: config.retry_attempts,
            retry_delay_ms: config.retry_delay_ms,
            max_idle_connections: config.max_idle_connections,
            idle_timeout: config.idle_timeout,
        }
    }
}

impl From<&HttpAuthType> for crate::mcp::clients::HttpAuthConfig {
    fn from(auth: &HttpAuthType) -> Self {
        match auth {
            HttpAuthType::None => Self::None,
            HttpAuthType::Bearer { token } => Self::Bearer { token: token.clone() },
            HttpAuthType::ApiKey { header, key } => Self::ApiKey { 
                header: header.clone(), 
                key: key.clone() 
            },
            HttpAuthType::Basic { username, password } => Self::Basic { 
                username: username.clone(), 
                password: password.clone() 
            },
        }
    }
}

// Conversion implementations for SSE services
impl From<&SseServiceConfig> for crate::mcp::clients::SseClientConfig {
    fn from(config: &SseServiceConfig) -> Self {
        Self {
            base_url: config.base_url.clone(),
            auth: (&config.auth).into(),
            single_session: config.single_session,
            connection_timeout: config.connection_timeout,
            request_timeout: config.request_timeout,
            max_queue_size: config.max_queue_size,
            heartbeat_interval: config.heartbeat_interval,
            reconnect: config.reconnect,
            max_reconnect_attempts: config.max_reconnect_attempts,
            reconnect_delay_ms: config.reconnect_delay_ms,
            max_reconnect_delay_ms: config.max_reconnect_delay_ms,
        }
    }
}

impl From<&SseAuthType> for crate::mcp::clients::SseAuthConfig {
    fn from(auth: &SseAuthType) -> Self {
        match auth {
            SseAuthType::None => Self::None,
            SseAuthType::Bearer { token } => Self::Bearer { token: token.clone() },
            SseAuthType::ApiKey { header, key } => Self::ApiKey { 
                header: header.clone(), 
                key: key.clone() 
            },
            SseAuthType::QueryParam { param, value } => Self::QueryParam { 
                param: param.clone(), 
                value: value.clone() 
            },
        }
    }
}





impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            runtime: "docker".to_string(),
            node_image: Some("node:18-alpine".to_string()),
            python_image: Some("python:3.11-alpine".to_string()),
            network_mode: Some("bridge".to_string()),
            run_args: vec!["--rm".to_string(), "-i".to_string()],
        }
    }
}





impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            registry: RegistryConfig::default(),
            auth: None,
            logging: Some(LoggingConfig::default()),
            // hybrid_routing removed - use external_mcp instead
            external_mcp: None,
            mcp_client: None,
            conflict_resolution: None,
            visibility: None,
            smart_discovery: None,
            security: None,
            streamable_http: None,
            sampling: None,
            elicitation: None,
            prompt_generation: None,
            resource_generation: None,
            content_storage: None,
            external_content: None,
            enhancement_storage: None,
        }
    }
}

impl ServerConfig {
    /// Validate server configuration
    pub fn validate(&self) -> Result<()> {
        // Validate host
        if self.host.is_empty() {
            return Err(ProxyError::config("Server host cannot be empty"));
        }

        // Validate host format (basic check for valid characters)
        if !self.host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == ':' || c == '-') {
            return Err(ProxyError::config(format!(
                "Invalid host format: '{}'. Host must contain only alphanumeric characters, dots, colons, and hyphens",
                self.host
            )));
        }

        // Validate port range
        if self.port == 0 {
            return Err(ProxyError::config("Server port cannot be 0"));
        }

        if self.port < 1024 && !cfg!(test) {
            return Err(ProxyError::config(format!(
                "Port {} is in the reserved range (1-1023). Use a port >= 1024 for non-privileged operation",
                self.port
            )));
        }

        // Validate timeout
        if self.timeout == 0 {
            return Err(ProxyError::config("Server timeout cannot be 0"));
        }

        if self.timeout > 3600 {
            return Err(ProxyError::config(format!(
                "Server timeout {} seconds is too high. Maximum allowed is 3600 seconds (1 hour)",
                self.timeout
            )));
        }

        // Validate TLS configuration if present
        if let Some(ref tls_config) = self.tls {
            tls_config.validate()?;
        }

        Ok(())
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            websocket: true,
            timeout: 30,
            tls: None, // TLS disabled by default for backward compatibility
        }
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            mode: TlsMode::Disabled,
            cert_file: None,
            key_file: None,
            ca_file: None,
            behind_proxy: false,
            trusted_proxies: vec![
                "10.0.0.0/8".to_string(),
                "172.16.0.0/12".to_string(),
                "192.168.0.0/16".to_string(),
                "127.0.0.1/32".to_string(),
            ],
            min_tls_version: "1.2".to_string(),
            cipher_suites: None,
            hsts_enabled: true,
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: false,
            hsts_preload: false,
            require_forwarded_proto: false,
            require_forwarded_for: false,
            auto_detect_headers: vec![
                "X-Forwarded-Proto".to_string(),
                "X-Forwarded-For".to_string(),
                "X-Real-IP".to_string(),
            ],
            fallback_mode: TlsMode::Application,
        }
    }
}

impl Default for TlsMode {
    fn default() -> Self {
        TlsMode::Disabled
    }
}

impl TlsConfig {
    /// Validate TLS configuration
    pub fn validate(&self) -> Result<()> {
        match self.mode {
            TlsMode::Disabled => {
                // No validation needed for disabled mode
                Ok(())
            }
            TlsMode::Application => {
                self.validate_application_mode()
            }
            TlsMode::BehindProxy => {
                self.validate_behind_proxy_mode()
            }
            TlsMode::Auto => {
                self.validate_auto_mode()
            }
        }
    }

    /// Validate application mode configuration
    fn validate_application_mode(&self) -> Result<()> {
        // Certificate file is required for application mode
        let cert_file = self.cert_file.as_ref().ok_or_else(|| {
            ProxyError::config("TLS certificate file (cert_file) is required for application mode")
        })?;

        // Key file is required for application mode
        let key_file = self.key_file.as_ref().ok_or_else(|| {
            ProxyError::config("TLS private key file (key_file) is required for application mode")
        })?;

        // Validate certificate file exists and is readable
        if !std::path::Path::new(cert_file).exists() {
            return Err(ProxyError::config(format!(
                "TLS certificate file does not exist: {}",
                cert_file
            )));
        }

        // Validate key file exists and is readable
        if !std::path::Path::new(key_file).exists() {
            return Err(ProxyError::config(format!(
                "TLS private key file does not exist: {}",
                key_file
            )));
        }

        // Validate CA file if provided
        if let Some(ca_file) = &self.ca_file {
            if !std::path::Path::new(ca_file).exists() {
                return Err(ProxyError::config(format!(
                    "TLS CA certificate file does not exist: {}",
                    ca_file
                )));
            }
        }

        self.validate_common_settings()
    }

    /// Validate behind proxy mode configuration
    fn validate_behind_proxy_mode(&self) -> Result<()> {
        // Validate trusted proxies
        if self.trusted_proxies.is_empty() {
            return Err(ProxyError::config(
                "At least one trusted proxy must be specified for behind_proxy mode"
            ));
        }

        // Validate CIDR notation for trusted proxies
        for proxy in &self.trusted_proxies {
            if !self.is_valid_cidr(proxy) {
                return Err(ProxyError::config(format!(
                    "Invalid CIDR notation for trusted proxy: {}",
                    proxy
                )));
            }
        }

        self.validate_common_settings()
    }

    /// Validate auto mode configuration
    fn validate_auto_mode(&self) -> Result<()> {
        // Auto mode needs both application and proxy settings
        // Certificate files are optional (will use application mode if present)
        if self.cert_file.is_some() || self.key_file.is_some() {
            // If any cert files are provided, validate them like application mode
            if self.cert_file.is_none() || self.key_file.is_none() {
                return Err(ProxyError::config(
                    "Both cert_file and key_file must be provided together for auto mode"
                ));
            }

            // Validate certificate files exist
            if let (Some(cert_file), Some(key_file)) = (&self.cert_file, &self.key_file) {
                if !std::path::Path::new(cert_file).exists() {
                    return Err(ProxyError::config(format!(
                        "TLS certificate file does not exist: {}",
                        cert_file
                    )));
                }
                if !std::path::Path::new(key_file).exists() {
                    return Err(ProxyError::config(format!(
                        "TLS private key file does not exist: {}",
                        key_file
                    )));
                }
            }
        }

        // Validate trusted proxies for proxy detection
        for proxy in &self.trusted_proxies {
            if !self.is_valid_cidr(proxy) {
                return Err(ProxyError::config(format!(
                    "Invalid CIDR notation for trusted proxy: {}",
                    proxy
                )));
            }
        }

        // Validate auto-detection headers
        if self.auto_detect_headers.is_empty() {
            return Err(ProxyError::config(
                "At least one auto-detection header must be specified for auto mode"
            ));
        }

        self.validate_common_settings()
    }

    /// Validate common TLS settings
    fn validate_common_settings(&self) -> Result<()> {
        // Validate TLS version
        match self.min_tls_version.as_str() {
            "1.2" | "1.3" => {}
            _ => {
                return Err(ProxyError::config(format!(
                    "Invalid TLS version: {}. Supported versions: 1.2, 1.3",
                    self.min_tls_version
                )));
            }
        }

        // Validate HSTS settings
        if self.hsts_enabled {
            if self.hsts_max_age == 0 {
                return Err(ProxyError::config(
                    "HSTS max age must be greater than 0 when HSTS is enabled"
                ));
            }
            if self.hsts_max_age > 63072000 {
                // 2 years maximum
                return Err(ProxyError::config(
                    "HSTS max age cannot exceed 63072000 seconds (2 years)"
                ));
            }
        }

        Ok(())
    }

    /// Validate CIDR notation
    fn is_valid_cidr(&self, cidr: &str) -> bool {
        // Basic CIDR validation - check format
        if let Some((ip, prefix)) = cidr.split_once('/') {
            // Validate IP address format
            if std::net::IpAddr::from_str(ip).is_err() {
                return false;
            }

            // Validate prefix length
            if let Ok(prefix_len) = prefix.parse::<u8>() {
                // IPv4: 0-32, IPv6: 0-128
                if ip.contains(':') {
                    // IPv6
                    prefix_len <= 128
                } else {
                    // IPv4
                    prefix_len <= 32
                }
            } else {
                false
            }
        } else {
            // Single IP address without prefix
            std::net::IpAddr::from_str(cidr).is_ok()
        }
    }
}

impl RegistryConfig {
    /// Validate registry configuration
    pub fn validate(&self) -> Result<()> {
        // Validate registry type
        if self.r#type.is_empty() {
            return Err(ProxyError::config("Registry type cannot be empty"));
        }

        // Validate supported registry types
        match self.r#type.as_str() {
            "file" => {
                // File-based registry validation
                if self.paths.is_empty() {
                    return Err(ProxyError::config("Registry paths cannot be empty for file-based registry"));
                }

                // Validate each path
                for (index, path) in self.paths.iter().enumerate() {
                    if path.is_empty() {
                        return Err(ProxyError::config(format!(
                            "Registry path at index {} cannot be empty",
                            index
                        )));
                    }

                    // Check for potentially dangerous paths
                    if path.contains("..") {
                        return Err(ProxyError::config(format!(
                            "Registry path '{}' contains '..' which is not allowed for security reasons",
                            path
                        )));
                    }

                    // Validate file structure
                    self.validate_path_structure(path, index)?;
                }
            }
            "database" => {
                return Err(ProxyError::config(
                    "Database registry type is not yet implemented"
                ));
            }
            _ => {
                return Err(ProxyError::config(format!(
                    "Unsupported registry type: '{}'. Supported types: file",
                    self.r#type
                )));
            }
        }

        // Validate validation config
        self.validation.validate()?;

        Ok(())
    }

    /// Validate file path structure and accessibility
    fn validate_path_structure(&self, path: &str, index: usize) -> Result<()> {
        use std::path::Path;

        let path_obj = Path::new(path);

        // Check if path contains glob patterns
        let is_glob = path.contains('*') || path.contains('?') || path.contains('[');

        if is_glob {
            // For glob patterns, validate the base directory
            let base_path = if let Some(parent) = path_obj.parent() {
                // Find the first non-glob component
                let mut current = parent;
                while current.to_string_lossy().contains('*') ||
                      current.to_string_lossy().contains('?') ||
                      current.to_string_lossy().contains('[') {
                    if let Some(p) = current.parent() {
                        current = p;
                    } else {
                        break;
                    }
                }
                current
            } else {
                Path::new(".")
            };

            // Check if base directory exists and is readable
            if !base_path.exists() {
                return Err(ProxyError::config(format!(
                    "Registry path at index {}: base directory '{}' for glob pattern '{}' does not exist",
                    index, base_path.display(), path
                )));
            }

            if !base_path.is_dir() {
                return Err(ProxyError::config(format!(
                    "Registry path at index {}: base path '{}' for glob pattern '{}' is not a directory",
                    index, base_path.display(), path
                )));
            }

            // Check if directory is readable
            match std::fs::read_dir(base_path) {
                Ok(_) => {}
                Err(e) => {
                    return Err(ProxyError::config(format!(
                        "Registry path at index {}: cannot read base directory '{}' for glob pattern '{}': {}",
                        index, base_path.display(), path, e
                    )));
                }
            }
        } else {
            // For non-glob paths, check existence and accessibility
            if path_obj.exists() {
                if path_obj.is_file() {
                    // Check if file is readable
                    match std::fs::File::open(path_obj) {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(ProxyError::config(format!(
                                "Registry path at index {}: cannot read file '{}': {}",
                                index, path, e
                            )));
                        }
                    }

                    // Check file extension for capability files
                    if let Some(ext) = path_obj.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        if ext_str != "yaml" && ext_str != "yml" {
                            return Err(ProxyError::config(format!(
                                "Registry path at index {}: file '{}' must have .yaml or .yml extension",
                                index, path
                            )));
                        }
                    } else {
                        return Err(ProxyError::config(format!(
                            "Registry path at index {}: file '{}' must have .yaml or .yml extension",
                            index, path
                        )));
                    }
                } else if path_obj.is_dir() {
                    // Check if directory is readable
                    match std::fs::read_dir(path_obj) {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(ProxyError::config(format!(
                                "Registry path at index {}: cannot read directory '{}': {}",
                                index, path, e
                            )));
                        }
                    }
                } else {
                    return Err(ProxyError::config(format!(
                        "Registry path at index {}: '{}' is neither a file nor a directory",
                        index, path
                    )));
                }
            } else {
                // Path doesn't exist - this might be okay for some use cases
                // but we should warn about it
                tracing::warn!(
                    "Registry path at index {}: '{}' does not exist. It will be ignored during capability loading.",
                    index, path
                );
            }
        }

        Ok(())
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            r#type: "file".to_string(),
            paths: vec!["./data".to_string()],
            hot_reload: true,
            validation: ValidationConfig::default(),
        }
    }
}

impl ValidationConfig {
    /// Validate validation configuration
    pub fn validate(&self) -> Result<()> {
        // ValidationConfig is simple and doesn't need complex validation
        // All boolean values are valid
        Ok(())
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict: true,
            allow_unknown_fields: false,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default for backward compatibility
            r#type: AuthType::None,
            api_keys: None,
            oauth: None,
            jwt: None,
        }
    }
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::None
    }
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            require_header: true,
            header_name: "Authorization".to_string(),
            header_format: "Bearer {key}".to_string(),
        }
    }
}

impl ApiKeyConfig {
    /// Check if the API key configuration is empty (no keys defined)
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl ApiKeyEntry {
    /// Create a new API key entry with basic permissions
    pub fn new(key: String, name: String) -> Self {
        Self {
            key,
            name,
            description: None,
            permissions: vec!["read".to_string(), "write".to_string()],
            expires_at: None,
            active: true,
        }
    }

    /// Create a new API key entry with custom permissions
    pub fn with_permissions(key: String, name: String, permissions: Vec<String>) -> Self {
        Self {
            key,
            name,
            description: None,
            permissions,
            expires_at: None,
            active: true,
        }
    }

    /// Check if this API key has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.active && self.permissions.contains(&permission.to_string())
    }

    /// Check if this API key is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = &self.expires_at {
            // Parse ISO 8601 timestamp and compare with current time
            if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                return chrono::Utc::now() > expiry.with_timezone(&chrono::Utc);
            }
        }
        false
    }

    /// Check if this API key is valid (active and not expired)
    pub fn is_valid(&self) -> bool {
        self.active && !self.is_expired()
    }
}

impl AuthConfig {
    /// Validate authentication configuration
    pub fn validate(&self) -> Result<()> {
        // If authentication is disabled, no validation needed
        if !self.enabled {
            return Ok(());
        }

        match &self.r#type {
            AuthType::None => {
                // No authentication - no additional validation needed
                Ok(())
            }
            AuthType::ApiKey => {
                // Validate API key configuration
                match &self.api_keys {
                    Some(api_key_config) => {
                        if api_key_config.keys.is_empty() {
                            return Err(ProxyError::config(
                                "API key authentication requires at least one API key"
                            ));
                        }

                        // Validate each API key entry
                        for (index, key_entry) in api_key_config.keys.iter().enumerate() {
                            if key_entry.key.is_empty() {
                                return Err(ProxyError::config(format!(
                                    "API key at index {} cannot be empty",
                                    index
                                )));
                            }

                            if key_entry.key.len() < 16 {
                                return Err(ProxyError::config(format!(
                                    "API key '{}' at index {} is too short. Minimum length is 16 characters",
                                    key_entry.name, index
                                )));
                            }

                            if key_entry.name.is_empty() {
                                return Err(ProxyError::config(format!(
                                    "API key name at index {} cannot be empty",
                                    index
                                )));
                            }

                            // Validate expiration if provided
                            if let Some(expires_at) = &key_entry.expires_at {
                                if chrono::DateTime::parse_from_rfc3339(expires_at).is_err() {
                                    return Err(ProxyError::config(format!(
                                        "Invalid expiration date format for API key '{}'. Use ISO 8601 format",
                                        key_entry.name
                                    )));
                                }
                            }
                        }

                        // Validate header configuration
                        if api_key_config.header_name.is_empty() {
                            return Err(ProxyError::config("API key header name cannot be empty"));
                        }

                        if api_key_config.header_format.is_empty() {
                            return Err(ProxyError::config("API key header format cannot be empty"));
                        }

                        if !api_key_config.header_format.contains("{key}") {
                            return Err(ProxyError::config(
                                "API key header format must contain '{key}' placeholder"
                            ));
                        }

                        Ok(())
                    }
                    None => Err(ProxyError::config(
                        "API key authentication enabled but no API key configuration provided"
                    ))
                }
            }
            AuthType::OAuth => {
                // Validate OAuth configuration
                match &self.oauth {
                    Some(oauth_config) => oauth_config.validate(),
                    None => Err(ProxyError::config(
                        "OAuth authentication enabled but no OAuth configuration provided"
                    ))
                }
            }
            AuthType::Jwt => {
                // Validate JWT configuration
                match &self.jwt {
                    Some(jwt_config) => jwt_config.validate(),
                    None => Err(ProxyError::config(
                        "JWT authentication enabled but no JWT configuration provided"
                    ))
                }
            }
        }
    }

    /// Find an API key entry by key value
    pub fn find_api_key(&self, key: &str) -> Option<&ApiKeyEntry> {
        if let Some(api_key_config) = &self.api_keys {
            api_key_config.keys.iter().find(|entry| entry.key == key)
        } else {
            None
        }
    }

    /// Check if a given API key is valid and has the required permission
    pub fn validate_api_key(&self, key: &str, permission: &str) -> bool {
        if let Some(key_entry) = self.find_api_key(key) {
            key_entry.is_valid() && key_entry.has_permission(permission)
        } else {
            false
        }
    }
}

impl OAuthConfig {
    /// Validate OAuth configuration
    pub fn validate(&self) -> Result<()> {
        if self.provider.is_empty() {
            return Err(ProxyError::config("OAuth provider cannot be empty"));
        }

        if self.client_id.is_empty() {
            return Err(ProxyError::config("OAuth client ID cannot be empty"));
        }

        if self.client_secret.is_empty() {
            return Err(ProxyError::config("OAuth client secret cannot be empty"));
        }

        if self.auth_url.is_empty() {
            return Err(ProxyError::config("OAuth authorization URL cannot be empty"));
        }

        if self.token_url.is_empty() {
            return Err(ProxyError::config("OAuth token URL cannot be empty"));
        }

        // Basic URL validation
        if !self.auth_url.starts_with("http://") && !self.auth_url.starts_with("https://") {
            return Err(ProxyError::config(format!(
                "OAuth authorization URL must start with http:// or https://: '{}'",
                self.auth_url
            )));
        }

        if !self.token_url.starts_with("http://") && !self.token_url.starts_with("https://") {
            return Err(ProxyError::config(format!(
                "OAuth token URL must start with http:// or https://: '{}'",
                self.token_url
            )));
        }

        Ok(())
    }
}

impl JwtConfig {
    /// Validate JWT configuration
    pub fn validate(&self) -> Result<()> {
        if self.secret.is_empty() {
            return Err(ProxyError::config("JWT secret cannot be empty"));
        }

        if self.secret.len() < 32 {
            return Err(ProxyError::config(
                "JWT secret must be at least 32 characters long for security"
            ));
        }

        if self.algorithm.is_empty() {
            return Err(ProxyError::config("JWT algorithm cannot be empty"));
        }

        // Validate supported algorithms
        match self.algorithm.as_str() {
            "HS256" | "HS384" | "HS512" | "RS256" | "RS384" | "RS512" | "ES256" | "ES384" => {
                // Valid algorithm
            }
            _ => {
                return Err(ProxyError::config(format!(
                    "Unsupported JWT algorithm: '{}'. Supported: HS256, HS384, HS512, RS256, RS384, RS512, ES256, ES384",
                    self.algorithm
                )));
            }
        }

        if self.expiration == 0 {
            return Err(ProxyError::config("JWT expiration must be greater than 0"));
        }

        // Reasonable expiration limits (1 minute to 1 year)
        if self.expiration < 60 || self.expiration > 31_536_000 {
            return Err(ProxyError::config(
                "JWT expiration must be between 60 seconds and 1 year (31,536,000 seconds)"
            ));
        }

        Ok(())
    }
}

impl LoggingConfig {
    /// Validate logging configuration
    pub fn validate(&self) -> Result<()> {
        // Validate log level
        match self.level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => return Err(ProxyError::config(format!(
                "Invalid log level: '{}'. Valid levels: trace, debug, info, warn, error",
                self.level
            )))
        }

        // Validate log format
        match self.format.to_lowercase().as_str() {
            "json" | "text" | "pretty" => {}
            _ => return Err(ProxyError::config(format!(
                "Invalid log format: '{}'. Valid formats: json, text, pretty",
                self.format
            )))
        }

        // Validate log file path if provided
        if let Some(ref file_path) = self.file {
            if file_path.is_empty() {
                return Err(ProxyError::config("Log file path cannot be empty"));
            }

            // Check if the parent directory exists or can be created
            if let Some(parent) = std::path::Path::new(file_path).parent() {
                if !parent.exists() {
                    return Err(ProxyError::config(format!(
                        "Log file directory does not exist: '{}'",
                        parent.display()
                    )));
                }
            }
        }

        Ok(())
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "text".to_string(),
            file: None,
        }
    }
}

impl Config {
    /// Load .env files in order of precedence
    fn load_env_files() -> Result<()> {
        // Determine environment
        let env = std::env::var("MAGICTUNNEL_ENV")
            .or_else(|_| std::env::var("ENV"))
            .or_else(|_| std::env::var("NODE_ENV"))
            .unwrap_or_else(|_| "development".to_string());
        
        // Load .env files in order of precedence (each overrides the previous)
        let env_specific_file = format!(".env.{}", env);
        let env_files = vec![
            ".env",                    // Base environment file
            &env_specific_file,        // Environment-specific file
            ".env.local",              // Local overrides (highest precedence)
        ];
        
        for env_file in env_files {
            match dotenvy::from_filename(env_file) {
                Ok(_) => {
                    tracing::info!("Loaded environment variables from {}", env_file);
                }
                Err(e) if e.to_string().contains("not found") => {
                    tracing::debug!("No {} file found, skipping", env_file);
                }
                Err(e) => {
                    tracing::warn!("Failed to load {}: {}", env_file, e);
                }
            }
        }
        
        tracing::info!("Environment: {}", env);
        Ok(())
    }

    /// Load configuration from file with environment variables and CLI overrides
    pub fn load<P: AsRef<Path>>(
        path: P,
        host_override: Option<String>,
        port_override: Option<u16>,
    ) -> Result<Self> {
        // Load .env files in order of precedence: .env → .env.{environment} → .env.local
        Self::load_env_files()?;

        let mut config = if path.as_ref().exists() {
            let content = std::fs::read_to_string(&path).map_err(|e| {
                ProxyError::config(format!("Failed to read config file: {}", e))
            })?;

            serde_yaml::from_str(&content).map_err(|e| {
                ProxyError::config(format!("Failed to parse config file: {}", e))
            })?
        } else {
            tracing::warn!("Config file not found, using defaults");
            Self::default()
        };

        // Apply environment variable overrides (precedence: .env < file < env < CLI)
        config.apply_environment_overrides()?;

        // Apply CLI overrides (highest precedence)
        if let Some(host) = host_override {
            config.server.host = host;
        }
        if let Some(port) = port_override {
            config.server.port = port;
        }

        config.validate()?;
        Ok(config)
    }

    /// Apply environment variable overrides to configuration
    pub fn apply_environment_overrides(&mut self) -> Result<()> {
        // Server configuration environment variables
        if let Ok(host) = std::env::var("MCP_HOST") {
            if !host.is_empty() {
                self.server.host = host;
            }
        }

        if let Ok(port_str) = std::env::var("MCP_PORT") {
            if !port_str.is_empty() {
                self.server.port = port_str.parse().map_err(|e| {
                    ProxyError::config(format!("Invalid MCP_PORT environment variable: {}", e))
                })?;
            }
        }

        if let Ok(websocket_str) = std::env::var("MCP_WEBSOCKET") {
            if !websocket_str.is_empty() {
                self.server.websocket = websocket_str.parse().map_err(|e| {
                    ProxyError::config(format!("Invalid MCP_WEBSOCKET environment variable: {}", e))
                })?;
            }
        }

        if let Ok(timeout_str) = std::env::var("MCP_TIMEOUT") {
            if !timeout_str.is_empty() {
                self.server.timeout = timeout_str.parse().map_err(|e| {
                    ProxyError::config(format!("Invalid MCP_TIMEOUT environment variable: {}", e))
                })?;
            }
        }

        // TLS configuration environment variables
        self.apply_tls_environment_overrides()?;

        // Registry configuration environment variables
        if let Ok(registry_type) = std::env::var("MCP_REGISTRY_TYPE") {
            if !registry_type.is_empty() {
                self.registry.r#type = registry_type;
            }
        }

        if let Ok(registry_paths) = std::env::var("MCP_REGISTRY_PATHS") {
            if !registry_paths.is_empty() {
                // Split by comma and trim whitespace
                self.registry.paths = registry_paths
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        if let Ok(hot_reload_str) = std::env::var("MCP_HOT_RELOAD") {
            if !hot_reload_str.is_empty() {
                self.registry.hot_reload = hot_reload_str.parse().map_err(|e| {
                    ProxyError::config(format!("Invalid MCP_HOT_RELOAD environment variable: {}", e))
                })?;
            }
        }



        // Conflict resolution environment variables
        if let Ok(strategy) = std::env::var("CONFLICT_RESOLUTION_STRATEGY") {
            if !strategy.is_empty() {
                let conflict_strategy = match strategy.to_lowercase().as_str() {
                    "local_first" => crate::routing::conflict_resolution::ConflictResolutionStrategy::LocalFirst,
                    "proxy_first" => crate::routing::conflict_resolution::ConflictResolutionStrategy::ProxyFirst,
                    "first_found" => crate::routing::conflict_resolution::ConflictResolutionStrategy::FirstFound,
                    "reject" => crate::routing::conflict_resolution::ConflictResolutionStrategy::Reject,
                    "prefix" => crate::routing::conflict_resolution::ConflictResolutionStrategy::Prefix,
                    _ => return Err(ProxyError::config(format!(
                        "Invalid CONFLICT_RESOLUTION_STRATEGY value: {}. Must be one of: local_first, proxy_first, first_found, reject, prefix",
                        strategy
                    ))),
                };

                if let Some(ref mut conflict_resolution) = self.conflict_resolution {
                    conflict_resolution.strategy = conflict_strategy;
                } else {
                    self.conflict_resolution = Some(crate::routing::ConflictResolutionConfig {
                        strategy: conflict_strategy,
                        ..Default::default()
                    });
                }
            }
        }

        if let Ok(local_prefix) = std::env::var("CONFLICT_RESOLUTION_LOCAL_PREFIX") {
            if !local_prefix.is_empty() {
                if let Some(ref mut conflict_resolution) = self.conflict_resolution {
                    conflict_resolution.local_prefix = local_prefix;
                } else {
                    self.conflict_resolution = Some(crate::routing::ConflictResolutionConfig {
                        local_prefix,
                        ..Default::default()
                    });
                }
            }
        }

        if let Ok(proxy_prefix_format) = std::env::var("CONFLICT_RESOLUTION_PROXY_PREFIX_FORMAT") {
            if !proxy_prefix_format.is_empty() {
                if let Some(ref mut conflict_resolution) = self.conflict_resolution {
                    conflict_resolution.proxy_prefix_format = proxy_prefix_format;
                } else {
                    self.conflict_resolution = Some(crate::routing::ConflictResolutionConfig {
                        proxy_prefix_format,
                        ..Default::default()
                    });
                }
            }
        }

        if let Ok(log_conflicts) = std::env::var("CONFLICT_RESOLUTION_LOG_CONFLICTS") {
            if !log_conflicts.is_empty() {
                let log_conflicts_bool = log_conflicts.to_lowercase() == "true";
                if let Some(ref mut conflict_resolution) = self.conflict_resolution {
                    conflict_resolution.log_conflicts = log_conflicts_bool;
                } else {
                    self.conflict_resolution = Some(crate::routing::ConflictResolutionConfig {
                        log_conflicts: log_conflicts_bool,
                        ..Default::default()
                    });
                }
            }
        }

        if let Ok(include_metadata) = std::env::var("CONFLICT_RESOLUTION_INCLUDE_METADATA") {
            if !include_metadata.is_empty() {
                let include_metadata_bool = include_metadata.to_lowercase() == "true";
                if let Some(ref mut conflict_resolution) = self.conflict_resolution {
                    conflict_resolution.include_conflict_metadata = include_metadata_bool;
                } else {
                    self.conflict_resolution = Some(crate::routing::ConflictResolutionConfig {
                        include_conflict_metadata: include_metadata_bool,
                        ..Default::default()
                    });
                }
            }
        }

        // Logging configuration environment variables
        if let Ok(log_level) = std::env::var("MCP_LOG_LEVEL") {
            if !log_level.is_empty() {
                if self.logging.is_none() {
                    self.logging = Some(LoggingConfig::default());
                }
                if let Some(ref mut logging) = self.logging {
                    logging.level = log_level;
                }
            }
        }

        if let Ok(log_format) = std::env::var("MCP_LOG_FORMAT") {
            if !log_format.is_empty() {
                if self.logging.is_none() {
                    self.logging = Some(LoggingConfig::default());
                }
                if let Some(ref mut logging) = self.logging {
                    logging.format = log_format;
                }
            }
        }

        if let Ok(log_file) = std::env::var("MCP_LOG_FILE") {
            if !log_file.is_empty() {
                if self.logging.is_none() {
                    self.logging = Some(LoggingConfig::default());
                }
                if let Some(ref mut logging) = self.logging {
                    logging.file = Some(log_file);
                }
            }
        }



        // Note: Legacy MCP proxy environment variables removed - use remote_mcp instead

        // Semantic search configuration environment variables
        self.apply_semantic_search_environment_overrides()?;

        Ok(())
    }

    /// Apply TLS-specific environment variable overrides
    fn apply_tls_environment_overrides(&mut self) -> Result<()> {
        // Initialize TLS config if any TLS environment variables are set
        let has_tls_env = std::env::var("MCP_TLS_MODE").is_ok()
            || std::env::var("MCP_TLS_CERT_FILE").is_ok()
            || std::env::var("MCP_TLS_KEY_FILE").is_ok()
            || std::env::var("MCP_TLS_BEHIND_PROXY").is_ok();

        if has_tls_env && self.server.tls.is_none() {
            self.server.tls = Some(TlsConfig::default());
        }

        if let Some(ref mut tls_config) = self.server.tls {
            // TLS mode
            if let Ok(mode_str) = std::env::var("MCP_TLS_MODE") {
                if !mode_str.is_empty() {
                    tls_config.mode = match mode_str.to_lowercase().as_str() {
                        "disabled" => TlsMode::Disabled,
                        "application" => TlsMode::Application,
                        "behind_proxy" => TlsMode::BehindProxy,
                        "auto" => TlsMode::Auto,
                        _ => {
                            return Err(ProxyError::config(format!(
                                "Invalid MCP_TLS_MODE: {}. Valid values: disabled, application, behind_proxy, auto",
                                mode_str
                            )));
                        }
                    };
                }
            }

            // Certificate file
            if let Ok(cert_file) = std::env::var("MCP_TLS_CERT_FILE") {
                if !cert_file.is_empty() {
                    tls_config.cert_file = Some(cert_file);
                }
            }

            // Key file
            if let Ok(key_file) = std::env::var("MCP_TLS_KEY_FILE") {
                if !key_file.is_empty() {
                    tls_config.key_file = Some(key_file);
                }
            }

            // CA file
            if let Ok(ca_file) = std::env::var("MCP_TLS_CA_FILE") {
                if !ca_file.is_empty() {
                    tls_config.ca_file = Some(ca_file);
                }
            }

            // Behind proxy flag
            if let Ok(behind_proxy_str) = std::env::var("MCP_TLS_BEHIND_PROXY") {
                if !behind_proxy_str.is_empty() {
                    tls_config.behind_proxy = behind_proxy_str.parse().map_err(|e| {
                        ProxyError::config(format!("Invalid MCP_TLS_BEHIND_PROXY environment variable: {}", e))
                    })?;
                }
            }

            // Trusted proxies
            if let Ok(trusted_proxies) = std::env::var("MCP_TLS_TRUSTED_PROXIES") {
                if !trusted_proxies.is_empty() {
                    tls_config.trusted_proxies = trusted_proxies
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }

            // Minimum TLS version
            if let Ok(min_version) = std::env::var("MCP_TLS_MIN_VERSION") {
                if !min_version.is_empty() {
                    tls_config.min_tls_version = min_version;
                }
            }

            // HSTS enabled
            if let Ok(hsts_enabled_str) = std::env::var("MCP_TLS_HSTS_ENABLED") {
                if !hsts_enabled_str.is_empty() {
                    tls_config.hsts_enabled = hsts_enabled_str.parse().map_err(|e| {
                        ProxyError::config(format!("Invalid MCP_TLS_HSTS_ENABLED environment variable: {}", e))
                    })?;
                }
            }

            // HSTS max age
            if let Ok(hsts_max_age_str) = std::env::var("MCP_TLS_HSTS_MAX_AGE") {
                if !hsts_max_age_str.is_empty() {
                    tls_config.hsts_max_age = hsts_max_age_str.parse().map_err(|e| {
                        ProxyError::config(format!("Invalid MCP_TLS_HSTS_MAX_AGE environment variable: {}", e))
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Apply semantic search environment variable overrides
    fn apply_semantic_search_environment_overrides(&mut self) -> Result<()> {
        // Initialize smart_discovery config if semantic environment variables are set
        let has_semantic_env = std::env::var("MAGICTUNNEL_SEMANTIC_MODEL").is_ok()
            || std::env::var("MAGICTUNNEL_EMBEDDING_FILE").is_ok()
            || std::env::var("MAGICTUNNEL_DISABLE_SEMANTIC").is_ok();

        if has_semantic_env && self.smart_discovery.is_none() {
            self.smart_discovery = Some(crate::discovery::SmartDiscoveryConfig::default());
        }

        if let Some(ref mut smart_discovery) = self.smart_discovery {
            // Override semantic model
            if let Ok(model) = std::env::var("MAGICTUNNEL_SEMANTIC_MODEL") {
                if !model.is_empty() {
                    smart_discovery.semantic_search.model_name = model;
                }
            }

            // Override embedding file path
            if let Ok(embedding_file) = std::env::var("MAGICTUNNEL_EMBEDDING_FILE") {
                if !embedding_file.is_empty() {
                    smart_discovery.semantic_search.storage.embeddings_file = PathBuf::from(embedding_file);
                }
            }

            // Disable semantic search
            if let Ok(disable_str) = std::env::var("MAGICTUNNEL_DISABLE_SEMANTIC") {
                if !disable_str.is_empty() {
                    smart_discovery.semantic_search.enabled = !disable_str.parse().unwrap_or(false);
                }
            }
        }

        Ok(())
    }

    /// Validate the configuration with comprehensive checks
    pub fn validate(&self) -> Result<()> {
        // Validate server configuration
        self.server.validate()?;

        // Validate registry configuration
        self.registry.validate()?;

        // Validate authentication configuration if present
        if let Some(ref auth) = self.auth {
            auth.validate()?;
        }

        // Validate logging configuration if present
        if let Some(ref logging) = self.logging {
            logging.validate()?;
        }

        // Note: Legacy MCP proxy validation removed - use remote_mcp instead

        // Cross-validation checks
        self.validate_cross_dependencies()?;

        Ok(())
    }

    /// Validate cross-dependencies between configuration sections
    fn validate_cross_dependencies(&self) -> Result<()> {
        // Validate that gRPC port doesn't conflict with HTTP port
        let grpc_port = match self.server.port.checked_add(1000) {
            Some(port) => port,
            None => {
                return Err(ProxyError::config(format!(
                    "gRPC port calculation overflow: HTTP port {} + 1000 exceeds maximum value",
                    self.server.port
                )));
            }
        };
        if grpc_port > 65535 {
            return Err(ProxyError::config(format!(
                "gRPC port {} (HTTP port + 1000) exceeds maximum port number 65535",
                grpc_port
            )));
        }

        // Validate that authentication is properly configured if enabled
        if let Some(ref auth) = self.auth {
            if auth.enabled {
                match auth.r#type {
                    AuthType::ApiKey => {
                        if auth.api_keys.as_ref().map_or(true, |keys| keys.is_empty()) {
                            return Err(ProxyError::config(
                                "API key authentication enabled but no API keys provided"
                            ));
                        }
                    }
                    AuthType::OAuth => {
                        if auth.oauth.is_none() {
                            return Err(ProxyError::config(
                                "OAuth authentication enabled but no OAuth configuration provided"
                            ));
                        }
                    }
                    AuthType::Jwt => {
                        if auth.jwt.is_none() {
                            return Err(ProxyError::config(
                                "JWT authentication enabled but no JWT configuration provided"
                            ));
                        }
                    }
                    AuthType::None => {
                        // No additional validation needed for "none" type
                    }
                }
            }
        }

        Ok(())
    }
}

// Note: Legacy MCP proxy configuration validation removed - use remote_mcp instead

impl McpClientConfig {
    /// Validate MCP client configuration
    pub fn validate(&self) -> Result<()> {
        if self.connect_timeout_secs == 0 {
            return Err(ProxyError::config("Connect timeout must be greater than 0"));
        }

        if self.connect_timeout_secs > 300 {
            return Err(ProxyError::config("Connect timeout cannot exceed 300 seconds"));
        }

        if self.request_timeout_secs == 0 {
            return Err(ProxyError::config("Request timeout must be greater than 0"));
        }

        if self.request_timeout_secs > 600 {
            return Err(ProxyError::config("Request timeout cannot exceed 600 seconds"));
        }

        if self.max_reconnect_attempts > 10 {
            return Err(ProxyError::config("Maximum reconnect attempts cannot exceed 10"));
        }

        if self.reconnect_delay_secs == 0 {
            return Err(ProxyError::config("Reconnect delay must be greater than 0"));
        }

        if self.reconnect_delay_secs > 60 {
            return Err(ProxyError::config("Reconnect delay cannot exceed 60 seconds"));
        }

        Ok(())
    }
}

/// Streamable HTTP Transport configuration for MCP 2025-06-18 compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamableHttpTransportConfig {
    /// Enable the streamable HTTP transport
    pub enabled: bool,
    /// Enable enhanced batching capabilities
    pub enable_batching: bool,
    /// Maximum batch size for requests
    pub max_batch_size: usize,
    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,
    /// Enable compression for transport
    pub enable_compression: bool,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Enable HTTP keep-alive
    pub enable_keep_alive: bool,
    /// Enable NDJSON streaming support
    pub enable_ndjson_streaming: bool,
}

impl Default for StreamableHttpTransportConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_batching: true,
            max_batch_size: 100,
            batch_timeout_ms: 50,
            enable_compression: true,
            max_message_size: 10 * 1024 * 1024, // 10MB
            connection_timeout_seconds: 30,
            enable_keep_alive: true,
            enable_ndjson_streaming: true,
        }
    }
}

impl StreamableHttpTransportConfig {
    /// Validate the streamable HTTP transport configuration
    pub fn validate(&self) -> Result<()> {
        if self.max_batch_size == 0 {
            return Err(ProxyError::config("Streamable HTTP batch size must be greater than 0"));
        }

        if self.max_batch_size > 1000 {
            return Err(ProxyError::config("Streamable HTTP batch size cannot exceed 1000"));
        }

        if self.batch_timeout_ms == 0 {
            return Err(ProxyError::config("Streamable HTTP batch timeout must be greater than 0"));
        }

        if self.batch_timeout_ms > 60000 {
            return Err(ProxyError::config("Streamable HTTP batch timeout cannot exceed 60 seconds"));
        }

        if self.max_message_size < 1024 {
            return Err(ProxyError::config("Streamable HTTP message size must be at least 1KB"));
        }

        if self.max_message_size > 100 * 1024 * 1024 {
            return Err(ProxyError::config("Streamable HTTP message size cannot exceed 100MB"));
        }

        if self.connection_timeout_seconds == 0 {
            return Err(ProxyError::config("Streamable HTTP connection timeout must be greater than 0"));
        }

        if self.connection_timeout_seconds > 300 {
            return Err(ProxyError::config("Streamable HTTP connection timeout cannot exceed 5 minutes"));
        }

        Ok(())
    }
}

/// MCP 2025-06-18 Sampling service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingConfig {
    pub enabled: bool,
    pub default_model: String,
    pub max_tokens_limit: u32,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_model: "gpt-4o-mini".to_string(),
            max_tokens_limit: 4000,
        }
    }
}

/// MCP 2025-06-18 Elicitation service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationConfig {
    pub enabled: bool,
    pub max_schema_complexity: String,
    pub default_timeout_seconds: u32,
}

impl Default for ElicitationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_schema_complexity: "WithArrays".to_string(),  
            default_timeout_seconds: 300,
        }
    }
}
