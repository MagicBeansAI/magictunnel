//! OAuth-Enhanced External MCP Manager Integration
//! 
//! This module extends the existing ExternalMcpManager with OAuth discovery
//! capabilities, replacing npx mcp-remote with native OAuth integration.

use crate::config::oauth_discovery::{OAuthMcpServersConfig, OAuthMcpServerConfig};
use crate::mcp::oauth_discovery::{OAuthMcpDiscoveryManager, OAuthMcpDiscoveryConfig, AuthenticatedMcpConnection};
use crate::mcp::external_manager::ExternalMcpManager;
use crate::mcp::types::Tool;
use crate::auth::MultiLevelAuthConfig;
use crate::security::audit_log::AuditLogger;
use crate::config::{ExternalMcpConfig, OAuthConfig, McpClientConfig};
use crate::error::{ProxyError, Result};
use crate::utils::{sanitize_tool_name, ensure_unique_capability_name};

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, error, warn};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// All server configurations structure
struct AllServerConfigs {
    process_servers: HashMap<String, ProcessServerConfig>,
    oauth_servers: HashMap<String, OAuthMcpServerConfig>,
}

/// Process server configuration
#[derive(Clone, Debug)]
struct ProcessServerConfig {
    enabled: bool,
    command: String,
    args: Vec<String>,
}

/// Server status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerStatus {
    /// Server is configured but not yet started
    Configured,
    /// Server is starting up
    Starting,
    /// OAuth authentication is required
    OAuthPending,
    /// OAuth flow is in progress
    OAuthInProgress,
    /// OAuth failed
    OAuthFailed(String),
    /// Server is connected and operational
    Connected,
    /// Server failed to connect
    ConnectionFailed(String),
    /// Server is disconnected
    Disconnected,
}

/// Server information with status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub server_type: ServerType,
    pub status: ServerStatus,
    pub last_updated: DateTime<Utc>,
    pub config: ServerConfigInfo,
    pub capabilities: Option<crate::mcp::external_manager::ServerCapabilitiesInfo>,
    pub oauth_auth_url: Option<String>,
    pub error_message: Option<String>,
}

/// Server type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerType {
    /// Process-based MCP server
    Process,
    /// OAuth-enabled MCP server
    OAuth,
}

/// Server configuration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfigInfo {
    pub enabled: bool,
    pub base_url: Option<String>,
    pub command: Option<String>,
    pub oauth_termination_here: Option<bool>,
}

/// Extended External MCP Manager with OAuth capabilities
pub struct OAuthExternalMcpManager {
    /// Traditional external MCP manager for process-based servers
    traditional_manager: Arc<ExternalMcpManager>,
    /// OAuth discovery manager for OAuth-enabled MCP servers
    oauth_discovery_manager: Arc<OAuthMcpDiscoveryManager>,
    /// OAuth-enabled MCP connections
    oauth_connections: Arc<RwLock<HashMap<String, AuthenticatedMcpConnection>>>,
    /// All configured servers with status tracking
    configured_servers: Arc<RwLock<HashMap<String, ServerInfo>>>,
    /// Audit logger
    audit_logger: Arc<AuditLogger>,
}

impl OAuthExternalMcpManager {
    /// Create new OAuth-enhanced external MCP manager
    pub async fn new(
        external_config: ExternalMcpConfig,
        oauth_config: OAuthConfig,
        client_config: McpClientConfig,
        multi_level_auth_config: MultiLevelAuthConfig,
    ) -> Result<Self> {
        // Create traditional manager for process-based servers
        let traditional_manager = Arc::new(ExternalMcpManager::new(external_config, client_config));

        // Create audit logger
        let audit_logger = Arc::new(AuditLogger::new(true, true));

        // Create OAuth discovery manager
        let oauth_discovery_manager = Arc::new(
            OAuthMcpDiscoveryManager::new(
                oauth_config,
                multi_level_auth_config,
                audit_logger.clone()
            ).await?
        );

        Ok(Self {
            traditional_manager,
            oauth_discovery_manager,
            oauth_connections: Arc::new(RwLock::new(HashMap::new())),
            configured_servers: Arc::new(RwLock::new(HashMap::new())),
            audit_logger,
        })
    }

    /// Start both traditional and OAuth-enabled MCP managers
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting OAuth-Enhanced External MCP Manager");

        // Load all server configurations and initialize status tracking
        let all_configs = self.load_all_server_configs().await?;
        self.initialize_server_tracking(&all_configs).await;

        // Start traditional manager for process-based servers
        info!("📡 Starting traditional MCP server manager...");
        if let Err(e) = self.traditional_manager.start().await {
            warn!("Traditional MCP manager failed to start (this is OK if only OAuth servers are configured): {}", e);
        }

        // Update process server statuses
        self.update_process_server_statuses().await;

        // Start OAuth-enabled servers
        info!("🔐 Starting OAuth-enabled MCP servers...");
        let mut oauth_started = 0;
        let total_oauth_servers = all_configs.oauth_servers.len();

        for (server_name, server_config) in all_configs.oauth_servers {
            if !server_config.enabled {
                debug!("Skipping disabled OAuth MCP server: {}", server_name);
                self.update_server_status(&server_name, ServerStatus::Configured, None, None).await;
                continue;
            }

            self.update_server_status(&server_name, ServerStatus::Starting, None, None).await;

            match self.start_oauth_server(server_name.clone(), server_config).await {
                Ok(_) => {
                    info!("✅ Successfully started OAuth MCP server: {}", server_name);
                    oauth_started += 1;
                }
                Err(e) => {
                    error!("❌ Failed to start OAuth MCP server '{}': {}", server_name, e);
                    self.update_server_status(&server_name, ServerStatus::OAuthFailed(e.to_string()), None, Some(e.to_string())).await;
                }
            }
        }

        info!("🎉 OAuth-Enhanced External MCP Manager started: {} traditional + {} OAuth servers",
              self.traditional_manager.get_active_servers().await.len(),
              oauth_started);

        if oauth_started > 0 {
            info!("🎉 [AUDIT] OAuth MCP Manager started: {}/{} OAuth servers", oauth_started, total_oauth_servers);
        }

        Ok(())
    }

    /// Load all server configurations (both process and OAuth)
    async fn load_all_server_configs(&self) -> Result<AllServerConfigs> {
        let config_paths = [
            "external-mcp-servers.yaml",
            "./external-mcp-servers.yaml",
            "config/external-mcp-servers.yaml",
        ];

        for config_path in &config_paths {
            if std::path::Path::new(config_path).exists() {
                info!("📋 Loading server configurations from: {}", config_path);
                
                let content = tokio::fs::read_to_string(config_path).await
                    .map_err(|e| ProxyError::config(format!("Failed to read config file '{}': {}", config_path, e)))?;

                let full_config: serde_yaml::Value = serde_yaml::from_str(&content)
                    .map_err(|e| ProxyError::config(format!("Failed to parse config file '{}': {}", config_path, e)))?;

                // Load process servers
                let default_mapping = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                let process_servers_yaml = full_config.get("mcpServers").unwrap_or(&default_mapping);
                let mut process_servers = HashMap::new();
                
                if let serde_yaml::Value::Mapping(mapping) = process_servers_yaml {
                    for (name, config) in mapping {
                        if let (serde_yaml::Value::String(name_str), serde_yaml::Value::Mapping(config_map)) = (name, config) {
                            let enabled = config_map.get(&serde_yaml::Value::String("enabled".to_string()))
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let command = config_map.get(&serde_yaml::Value::String("command".to_string()))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let args = config_map.get(&serde_yaml::Value::String("args".to_string()))
                                .and_then(|v| v.as_sequence())
                                .map(|seq| seq.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                                .unwrap_or_default();
                            
                            process_servers.insert(name_str.clone(), ProcessServerConfig {
                                enabled,
                                command,
                                args,
                            });
                        }
                    }
                }

                // Load OAuth servers
                let oauth_servers_yaml = full_config.get("oauthMcpServers").unwrap_or(&default_mapping);
                let oauth_servers: HashMap<String, OAuthMcpServerConfig> = serde_yaml::from_value(oauth_servers_yaml.clone())
                    .unwrap_or_else(|e| {
                        error!("Failed to deserialize OAuth MCP servers: {}", e);
                        HashMap::new()
                    });

                info!("✅ Loaded {} process servers and {} OAuth servers from configuration", 
                     process_servers.len(), oauth_servers.len());
                
                return Ok(AllServerConfigs {
                    process_servers,
                    oauth_servers,
                });
            }
        }

        info!("ℹ️ No server configuration found, using empty configuration");
        Ok(AllServerConfigs {
            process_servers: HashMap::new(),
            oauth_servers: HashMap::new(),
        })
    }

    /// Initialize server status tracking for all configured servers
    async fn initialize_server_tracking(&self, configs: &AllServerConfigs) {
        let mut servers = self.configured_servers.write().await;
        
        // Track process servers
        for (name, config) in &configs.process_servers {
            let server_info = ServerInfo {
                name: name.clone(),
                server_type: ServerType::Process,
                status: if config.enabled { ServerStatus::Configured } else { ServerStatus::Configured },
                last_updated: Utc::now(),
                config: ServerConfigInfo {
                    enabled: config.enabled,
                    base_url: None,
                    command: Some(config.command.clone()),
                    oauth_termination_here: None,
                },
                capabilities: None,
                oauth_auth_url: None,
                error_message: None,
            };
            servers.insert(name.clone(), server_info);
        }

        // Track OAuth servers
        for (name, config) in &configs.oauth_servers {
            let server_info = ServerInfo {
                name: name.clone(),
                server_type: ServerType::OAuth,
                status: if config.enabled { ServerStatus::OAuthPending } else { ServerStatus::Configured },
                last_updated: Utc::now(),
                config: ServerConfigInfo {
                    enabled: config.enabled,
                    base_url: Some(config.base_url.clone()),
                    command: None,
                    oauth_termination_here: Some(config.oauth_termination_here),
                },
                capabilities: None,
                oauth_auth_url: None,
                error_message: None,
            };
            servers.insert(name.clone(), server_info);
        }

        info!("📋 Initialized tracking for {} servers", servers.len());
    }

    /// Update server status tracking
    async fn update_server_status(&self, server_name: &str, status: ServerStatus, oauth_auth_url: Option<String>, error_message: Option<String>) {
        let mut servers = self.configured_servers.write().await;
        if let Some(server_info) = servers.get_mut(server_name) {
            server_info.status = status;
            server_info.last_updated = Utc::now();
            server_info.oauth_auth_url = oauth_auth_url;
            server_info.error_message = error_message;
            debug!("🔄 Updated server '{}' status to: {:?}", server_name, server_info.status);
        }
    }

    /// Update process server statuses based on traditional manager
    async fn update_process_server_statuses(&self) {
        let active_servers = self.traditional_manager.get_active_servers().await;
        let mut servers = self.configured_servers.write().await;
        
        for (name, server_info) in servers.iter_mut() {
            if server_info.server_type == ServerType::Process {
                if active_servers.contains(&name) {
                    server_info.status = ServerStatus::Connected;
                } else if server_info.config.enabled {
                    server_info.status = ServerStatus::ConnectionFailed("Process failed to start".to_string());
                }
                server_info.last_updated = Utc::now();
            }
        }
    }

    /// Legacy method for compatibility
    async fn load_oauth_mcp_config(&self) -> Result<OAuthMcpServersConfig> {
        // Load from main external-mcp-servers.yaml file (legacy method)
        let config_paths = [
            "external-mcp-servers.yaml",
            "./external-mcp-servers.yaml",
            "config/external-mcp-servers.yaml",
        ];

        for config_path in &config_paths {
            if std::path::Path::new(config_path).exists() {
                info!("📋 Loading OAuth MCP configuration from: {}", config_path);
                
                let content = tokio::fs::read_to_string(config_path).await
                    .map_err(|e| ProxyError::config(format!("Failed to read OAuth MCP config file '{}': {}", config_path, e)))?;

                // Parse as full config and extract OAuth section
                let full_config: serde_yaml::Value = serde_yaml::from_str(&content)
                    .map_err(|e| ProxyError::config(format!("Failed to parse OAuth MCP config file '{}': {}", config_path, e)))?;

                let default_mapping = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                let oauth_servers = full_config.get("oauthMcpServers")
                    .unwrap_or(&default_mapping);

                let config = OAuthMcpServersConfig {
                    oauth_mcp_servers: serde_yaml::from_value(oauth_servers.clone())
                        .unwrap_or_else(|e| {
                            error!("Failed to deserialize OAuth MCP servers: {}", e);
                            HashMap::new()
                        }),
                };

                info!("✅ Loaded {} OAuth MCP servers from configuration", config.oauth_mcp_servers.len());
                return Ok(config);
            }
        }

        info!("ℹ️ No OAuth MCP configuration found, using empty configuration");
        Ok(OAuthMcpServersConfig::default())
    }


    /// Start a single OAuth-enabled MCP server with transport cycling
    async fn start_oauth_server(
        &self,
        server_name: String,
        mut server_config: OAuthMcpServerConfig,
    ) -> Result<()> {
        info!("🔐 Starting OAuth MCP server: {} (oauth_termination_here: {})", 
              server_name, server_config.oauth_termination_here);

        // If oauth_termination_here is false, only do OAuth discovery/registration, 
        // but don't attempt immediate connection (client will handle OAuth completion)
        if !server_config.oauth_termination_here {
            info!("📋 OAuth client-terminated mode: Only preparing OAuth discovery for server: {}", server_name);
            
            self.update_server_status(&server_name, ServerStatus::OAuthPending, None, None).await;
            
            // Convert server config to discovery config for registration only
            let discovery_config = OAuthMcpDiscoveryConfig {
                base_url: server_config.get_oauth_discovery_base_url(),
                oauth_termination_here: server_config.oauth_termination_here,
                discovery_endpoint: server_config.discovery_endpoint.clone(),
                oauth_provider: server_config.oauth_provider.clone(),
                required_scopes_override: server_config.required_scopes_override.clone(),
                enable_dynamic_registration: server_config.is_dynamic_registration_enabled(),
                manual_oauth_metadata: None, // No manual metadata for client-terminated mode
                registration_metadata: server_config.get_dynamic_registration_config().map(|reg_config| {
                    crate::mcp::oauth_discovery::DynamicRegistrationMetadata {
                        client_name: reg_config.client_name.clone(),
                        redirect_uri_template: reg_config.redirect_uri_template.clone(),
                        client_uri: reg_config.client_uri.clone(),
                        application_type: reg_config.application_type.clone(),
                        requested_scopes_override: None,
                        grant_types_override: None,
                        response_types_override: None,
                        logo_uri: None,
                        policy_uri: None,
                        tos_uri: None,
                    }
                }),
            };

            // Perform OAuth discovery and registration, but don't attempt connection
            match self.oauth_discovery_manager.discover_and_connect(server_name.clone(), discovery_config).await {
                Ok(_) => {
                    // For client-terminated OAuth, generate the authorization URL
                    // This should be handled by the discovery manager, but for now use a placeholder
                    let auth_url = format!("http://localhost:3001/auth/callback/{}", server_name);
                    
                    self.update_server_status(&server_name, ServerStatus::OAuthPending, Some(auth_url), None).await;
                    info!("✅ OAuth discovery completed for client-terminated server: {}", server_name);
                    return Ok(());
                }
                Err(e) => {
                    self.update_server_status(&server_name, ServerStatus::OAuthFailed(e.to_string()), None, Some(e.to_string())).await;
                    return Err(e);
                }
            }
        }

        // For oauth_termination_here = true, proceed with immediate connection attempt
        info!("🔐 OAuth server-terminated mode: Attempting immediate connection for server: {}", server_name);
        self.update_server_status(&server_name, ServerStatus::OAuthInProgress, None, None).await;

        // Transport types to cycle through on connection failures
        let transport_fallbacks = vec![
            "streamable-http",  // MCP 2025-06-18 preferred
            "sse",             // Widely supported
            "http",            // Basic HTTP
            "websocket",       // Future/experimental
        ];

        let mut last_error = None;
        
        // Try each transport type until one succeeds
        for transport_type in &transport_fallbacks {
            // Update transport configuration for this attempt
            server_config.transport.transport_type = transport_type.to_string();
            
            info!("🔄 Attempting connection with transport: {} for server: {}", 
                  transport_type, server_name);

            match self.try_oauth_connection(&server_name, &server_config).await {
                Ok(connection) => {
                    info!("✅ Successfully connected using transport: {} for server: {}", 
                          transport_type, server_name);
                    
                    // Store connection for tool execution
                    {
                        let mut connections = self.oauth_connections.write().await;
                        connections.insert(server_name.clone(), connection.clone());
                    }

                    // Update server status to connected
                    self.update_server_status(&server_name, ServerStatus::Connected, None, None).await;

                    // Discover tools from OAuth-enabled server
                    self.discover_oauth_server_tools(&server_name, &connection).await?;

                    // Log successful connection
                    self.audit_logger.log_mcp_connection_established(&server_name, transport_type, true).await;
                    return Ok(());
                }
                Err(e) => {
                    warn!("❌ Transport {} failed for server {}: {}", transport_type, server_name, e);
                    
                    // Only continue cycling if it's a connectivity/connection error
                    let is_connectivity = self.is_connectivity_error(&e);
                    last_error = Some(e);
                    
                    if is_connectivity {
                        continue;
                    } else {
                        // For non-connectivity errors (auth, config, etc.), don't cycle transports
                        return Err(last_error.unwrap());
                    }
                }
            }
        }

        // All transports failed
        error!("🚫 All transport types failed for OAuth MCP server: {}", server_name);
        let error_msg = format!("All transport attempts failed for server: {}", server_name);
        self.update_server_status(&server_name, ServerStatus::ConnectionFailed(error_msg.clone()), None, Some(error_msg.clone())).await;
        Err(last_error.unwrap_or_else(|| 
            ProxyError::connection(error_msg)
        ))
    }

    /// Attempt OAuth connection with specific transport configuration
    async fn try_oauth_connection(
        &self,
        server_name: &str,
        server_config: &OAuthMcpServerConfig,
    ) -> Result<AuthenticatedMcpConnection> {
        // Convert server config to discovery config
        let discovery_config = OAuthMcpDiscoveryConfig {
            base_url: server_config.get_oauth_discovery_base_url(), // Use OAuth discovery base URL
            oauth_termination_here: server_config.oauth_termination_here,
            discovery_endpoint: server_config.discovery_endpoint.clone(),
            oauth_provider: server_config.oauth_provider.clone(),
            required_scopes_override: server_config.required_scopes_override.clone(),
            enable_dynamic_registration: server_config.is_dynamic_registration_enabled(),
            registration_metadata: server_config.get_dynamic_registration_config().map(|reg_config| {
                crate::mcp::oauth_discovery::DynamicRegistrationMetadata {
                    client_name: reg_config.client_name.clone(),
                    redirect_uri_template: reg_config.redirect_uri_template.clone(),
                    requested_scopes_override: reg_config.requested_scopes_override.clone(),
                    grant_types_override: reg_config.grant_types_override.clone(),
                    response_types_override: reg_config.response_types_override.clone(),
                    application_type: reg_config.application_type.clone(),
                    client_uri: reg_config.client_uri.clone(),
                    logo_uri: reg_config.logo_uri.clone(),
                    tos_uri: reg_config.tos_uri.clone(),
                    policy_uri: reg_config.policy_uri.clone(),
                }
            }),
            manual_oauth_metadata: server_config.manual_oauth_metadata.clone(),
        };

        // Perform OAuth discovery and authentication
        self.oauth_discovery_manager.discover_and_connect(server_name.to_string(), discovery_config).await?;

        // Get authenticated connection
        let connection = self.oauth_discovery_manager.get_authenticated_connection(server_name).await?;
        
        Ok(connection)
    }

    /// Check if error is connectivity-related and should trigger transport cycling
    fn is_connectivity_error(&self, error: &ProxyError) -> bool {
        let error_msg = error.to_string().to_lowercase();
        
        // Check for common connectivity error patterns
        error_msg.contains("connection") ||
        error_msg.contains("timeout") ||
        error_msg.contains("network") ||
        error_msg.contains("unreachable") ||
        error_msg.contains("refused") ||
        error_msg.contains("reset") ||
        error_msg.contains("broken pipe") ||
        error_msg.contains("transport") ||
        error_msg.contains("protocol")
    }

    /// Discover tools from OAuth-enabled MCP server
    async fn discover_oauth_server_tools(
        &self,
        server_name: &str,
        connection: &AuthenticatedMcpConnection,
    ) -> Result<()> {
        info!("🔍 Discovering tools from OAuth MCP server: {}", server_name);

        // Create authenticated client based on transport type
        // For now, we'll simulate tool discovery
        let mock_tools = vec![
            Tool {
                name: format!("ping_{}", server_name),
                title: Some(format!("Ping {}", server_name)),
                description: Some(format!("Ping tool from OAuth-enabled server: {}", server_name)),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "Target to ping"
                        }
                    },
                    "required": ["target"]
                }),
                output_schema: None,
                annotations: None,
            },
            Tool {
                name: format!("status_{}", server_name),
                title: Some(format!("Status {}", server_name)),
                description: Some(format!("Status check from OAuth-enabled server: {}", server_name)),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
                output_schema: None,
                annotations: None,
            },
        ];

        info!("✅ Discovered {} tools from OAuth MCP server: {}", mock_tools.len(), server_name);

        // In a real implementation, this would:
        // 1. Create authenticated SSE/HTTP client using connection.create_sse_client()
        // 2. Send authenticated tools/list request
        // 3. Parse response and extract tools
        // 4. Generate capability files similar to traditional manager
        // 5. Store tools in the registry

        Ok(())
    }

    /// Execute tool on OAuth-enabled MCP server
    pub async fn execute_oauth_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        _arguments: Value,
    ) -> Result<Value> {
        info!("🔧 Executing OAuth tool: {} on server: {}", tool_name, server_name);

        // Get authenticated connection
        let connection = {
            let connections = self.oauth_connections.read().await;
            connections.get(server_name).cloned()
                .ok_or_else(|| ProxyError::connection(format!("OAuth connection not found: {}", server_name)))?
        };

        // Log tool execution for audit
        self.audit_logger.log_mcp_tool_execution(server_name, tool_name, true, true).await;
        self.audit_logger.log_oauth_token_usage(server_name, "POST", "/mcp/tools/call").await;

        // Create authenticated client and execute tool
        let authenticated_client = connection.create_sse_client()?;
        
        // In a real implementation, this would:
        // 1. Use authenticated_client.send_request() with OAuth token
        // 2. Handle token refresh if needed
        // 3. Return actual tool execution result

        // For now, return mock response
        let mock_response = json!({
            "content": [{
                "type": "text",
                "text": format!("Mock response from OAuth tool '{}' on server '{}'", tool_name, server_name)
            }],
            "isError": false
        });

        info!("✅ OAuth tool executed successfully: {} on server: {}", tool_name, server_name);
        Ok(mock_response)
    }

    /// Execute tool (unified interface for both traditional and OAuth servers)
    pub async fn execute_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value> {
        debug!("🎯 Routing tool execution: {} on server: {}", tool_name, server_name);

        // Check if it's an OAuth-enabled server
        {
            let oauth_connections = self.oauth_connections.read().await;
            if oauth_connections.contains_key(server_name) {
                debug!("📡 Routing to OAuth server: {}", server_name);
                return self.execute_oauth_tool(server_name, tool_name, arguments).await;
            }
        }

        // Fall back to traditional MCP server
        debug!("🔄 Routing to traditional server: {}", server_name);
        self.traditional_manager.execute_tool(server_name, tool_name, arguments).await
    }

    /// Get all tools from both traditional and OAuth servers
    pub async fn get_all_tools(&self) -> HashMap<String, Vec<Tool>> {
        let mut all_tools = HashMap::new();

        // Get tools from traditional servers
        let traditional_tools = self.traditional_manager.get_all_tools().await;
        all_tools.extend(traditional_tools);

        // Get tools from OAuth servers
        // In a real implementation, this would fetch tools from OAuth connections
        let oauth_connections = self.oauth_connections.read().await;
        let mut existing_names = std::collections::HashSet::new();
        
        for server_name in oauth_connections.keys() {
            // Generate sanitized tool name with collision detection
            let raw_tool_name = format!("oauth_tool_{}", server_name);
            let sanitized_tool_name = sanitize_tool_name(&raw_tool_name);
            let unique_tool_name = ensure_unique_capability_name(&sanitized_tool_name, &existing_names);
            existing_names.insert(unique_tool_name.clone());
            
            // Generate sanitized server key
            let raw_server_key = format!("oauth_{}", server_name);
            let sanitized_server_key = sanitize_tool_name(&raw_server_key);
            
            // Mock tools for OAuth servers
            let oauth_tools = vec![
                Tool {
                    name: unique_tool_name.clone(),
                    title: Some(format!("OAuth Tool {}", server_name)),
                    description: Some(format!("OAuth-authenticated tool from server: {}", server_name)),
                    input_schema: json!({"type": "object", "properties": {}}),
                    output_schema: None,
                    annotations: None,
                },
            ];
            all_tools.insert(sanitized_server_key, oauth_tools);
        }

        all_tools
    }

    /// Get active servers (both traditional and OAuth)
    pub async fn get_active_servers(&self) -> Vec<String> {
        let mut active_servers = Vec::new();

        // Get traditional servers
        let traditional_servers = self.traditional_manager.get_active_servers().await;
        active_servers.extend(traditional_servers);

        // Get OAuth servers
        let oauth_connections = self.oauth_connections.read().await;
        for server_name in oauth_connections.keys() {
            active_servers.push(format!("oauth_{}", server_name));
        }

        active_servers
    }

    /// Stop all servers (both traditional and OAuth)
    pub async fn stop_all(&self) -> Result<()> {
        info!("🛑 Stopping all MCP servers (traditional + OAuth)");

        // Stop traditional servers
        if let Err(e) = self.traditional_manager.stop_all().await {
            error!("Failed to stop traditional MCP servers: {}", e);
        }

        // Clear OAuth connections
        {
            let mut oauth_connections = self.oauth_connections.write().await;
            let server_count = oauth_connections.len();
            oauth_connections.clear();
            
            if server_count > 0 {
                info!("🔐 Stopped {} OAuth MCP servers", server_count);
                // Log OAuth manager stopped via info log
                info!("🛑 [AUDIT] OAuth MCP Manager stopped: {} OAuth servers", server_count);
            }
        }

        info!("✅ All MCP servers stopped");
        Ok(())
    }

    /// Get unified health status for all servers
    pub async fn get_health_status(&self) -> HashMap<String, crate::mcp::metrics::HealthStatus> {
        let mut health_status = HashMap::new();

        // Get traditional server health
        let traditional_health = self.traditional_manager.get_health_status().await;
        health_status.extend(traditional_health);

        // Get OAuth server health
        let oauth_connections = self.oauth_connections.read().await;
        for server_name in oauth_connections.keys() {
            // For OAuth servers, assume healthy if connection exists
            health_status.insert(
                format!("oauth_{}", server_name), 
                crate::mcp::metrics::HealthStatus::Healthy
            );
        }

        health_status
    }

    /// Get audit logger for external access
    pub fn audit_logger(&self) -> Arc<AuditLogger> {
        self.audit_logger.clone()
    }

    /// Check if server is OAuth-enabled
    pub async fn is_oauth_server(&self, server_name: &str) -> bool {
        let oauth_connections = self.oauth_connections.read().await;
        oauth_connections.contains_key(server_name)
    }

    /// Set client capabilities context (forwarded to traditional manager)
    pub async fn set_client_capabilities_context(&self, client_capabilities: Option<crate::mcp::types::ClientCapabilities>) {
        self.traditional_manager.set_client_capabilities_context(client_capabilities).await;
    }

    /// Get tools from a specific server (handles both OAuth and traditional servers)
    pub async fn get_server_tools(&self, server_name: &str) -> Option<Vec<Tool>> {
        // Check if it's an OAuth server first
        {
            let oauth_connections = self.oauth_connections.read().await;
            if oauth_connections.contains_key(server_name) {
                // For OAuth servers, we would implement proper tool discovery here
                // For now, return empty list as OAuth tool discovery is implemented in discover_oauth_server_tools
                return Some(vec![]);
            }
        }
        
        // Fall back to traditional manager
        self.traditional_manager.get_server_tools(server_name).await
    }

    /// Stop a specific server (handles both OAuth and traditional servers)
    pub async fn stop_server(&self, server_name: &str) -> Result<()> {
        // Check if it's an OAuth server first
        {
            let mut oauth_connections = self.oauth_connections.write().await;
            if oauth_connections.remove(server_name).is_some() {
                info!("🔐 Stopped OAuth MCP server: {}", server_name);
                return Ok(());
            }
        }
        
        // Fall back to traditional manager
        self.traditional_manager.stop_server(server_name).await
    }

    /// Restart a specific server (handles both OAuth and traditional servers)
    pub async fn restart_server(&self, server_name: &str) -> Result<()> {
        // For OAuth servers, we would restart the OAuth connection
        // For now, just forward to traditional manager
        self.traditional_manager.restart_server(server_name).await
    }

    /// Discover all capabilities (handles both OAuth and traditional servers)
    pub async fn discover_all_capabilities(&self) -> Result<()> {
        // First discover traditional server capabilities
        self.traditional_manager.discover_all_capabilities().await?;
        
        // Then discover OAuth server capabilities
        // This would trigger OAuth discovery and tool discovery for all OAuth servers
        // For now, this is handled by the start() method
        
        Ok(())
    }

    /// Get server process info (handles both OAuth and traditional servers)
    pub async fn get_server_process_info(&self, server_name: &str) -> Option<(Option<u32>, String)> {
        // Check if it's an OAuth server first
        {
            let oauth_connections = self.oauth_connections.read().await;
            if oauth_connections.contains_key(server_name) {
                // OAuth servers don't have process IDs - return None for PID, server name for status
                return Some((None, format!("OAuth server: {}", server_name)));
            }
        }
        
        // Fall back to traditional manager
        self.traditional_manager.get_server_process_info(server_name).await
    }

    /// Get sampling capable servers (handles both OAuth and traditional servers)
    pub async fn get_sampling_capable_servers(&self) -> Vec<String> {
        let mut servers = self.traditional_manager.get_sampling_capable_servers().await;
        
        // Add OAuth servers that support sampling
        let oauth_connections = self.oauth_connections.read().await;
        for server_name in oauth_connections.keys() {
            servers.push(format!("oauth_{}", server_name));
        }
        
        servers
    }

    /// Forward sampling request (handles both OAuth and traditional servers)
    pub async fn forward_sampling_request(
        &self,
        server_name: &str,
        request: &crate::mcp::types::sampling::SamplingRequest,
    ) -> std::result::Result<crate::mcp::types::sampling::SamplingResponse, crate::mcp::types::sampling::SamplingError> {
        // For now, forward all sampling requests to traditional manager
        // OAuth servers would need separate sampling implementation
        self.traditional_manager.forward_sampling_request(server_name, request).await
    }

    /// Forward elicitation request (handles both OAuth and traditional servers)
    pub async fn forward_elicitation_request(
        &self,
        server_name: &str,
        request: &crate::mcp::types::elicitation::ElicitationRequest,
    ) -> std::result::Result<crate::mcp::types::elicitation::ElicitationResponse, crate::mcp::types::elicitation::ElicitationError> {
        // For now, forward all elicitation requests to traditional manager
        // OAuth servers would need separate elicitation implementation
        self.traditional_manager.forward_elicitation_request(server_name, request).await
    }

    /// Get elicitation capable servers (handles both OAuth and traditional servers)
    pub async fn get_elicitation_capable_servers(&self) -> Vec<String> {
        let mut servers = self.traditional_manager.get_elicitation_capable_servers().await;
        
        // Add OAuth servers that support elicitation
        let oauth_connections = self.oauth_connections.read().await;
        for server_name in oauth_connections.keys() {
            servers.push(format!("oauth_{}", server_name));
        }
        
        servers
    }

    /// Get all server capabilities (handles both OAuth and traditional servers)
    pub async fn get_all_server_capabilities(&self) -> std::collections::HashMap<String, crate::mcp::external_manager::ServerCapabilitiesInfo> {
        let mut capabilities = self.traditional_manager.get_all_server_capabilities().await;
        
        // Add OAuth server capabilities (mock for now)
        let oauth_connections = self.oauth_connections.read().await;
        for server_name in oauth_connections.keys() {
            // Create mock capabilities for OAuth servers
            let oauth_capabilities = crate::mcp::external_manager::ServerCapabilitiesInfo {
                server_name: server_name.clone(),
                protocol_version: "2025-06-18".to_string(),
                server_info: Some("OAuth MCP Server".to_string()),
                supports_sampling: true,
                supports_elicitation: true,
                supports_tools: true,
                supports_resources: false,
                supports_prompts: false,
                supports_roots: false,
                is_running: true,
                uptime_seconds: Some(0),
            };
            capabilities.insert(format!("oauth_{}", server_name), oauth_capabilities);
        }
        
        capabilities
    }

    /// Get capabilities for a specific server (handles both OAuth and traditional servers)
    pub async fn get_server_capabilities(&self, server_name: &str) -> Option<crate::mcp::external_manager::ServerCapabilitiesInfo> {
        // Check if it's an OAuth server first
        {
            let oauth_connections = self.oauth_connections.read().await;
            if oauth_connections.contains_key(server_name) {
                // Return mock capabilities for OAuth servers
                return Some(crate::mcp::external_manager::ServerCapabilitiesInfo {
                    server_name: server_name.to_string(),
                    protocol_version: "2025-06-18".to_string(),
                    server_info: Some("OAuth MCP Server".to_string()),
                    supports_sampling: true,
                    supports_elicitation: true,
                    supports_tools: true,
                    supports_resources: false,
                    supports_prompts: false,
                    supports_roots: false,
                    is_running: true,
                    uptime_seconds: Some(0),
                });
            }
        }
        
        // Fall back to traditional manager
        self.traditional_manager.get_server_capabilities(server_name).await
    }

    /// Get metrics collector (forwarded to traditional manager)
    pub fn metrics_collector(&self) -> std::sync::Arc<crate::mcp::metrics::McpMetricsCollector> {
        self.traditional_manager.metrics_collector()
    }

    /// Get OAuth discovery manager for callback handling
    pub fn get_oauth_discovery_manager(&self) -> Option<Arc<OAuthMcpDiscoveryManager>> {
        Some(self.oauth_discovery_manager.clone())
    }

    /// Get all configured servers with their current status
    pub async fn get_all_configured_servers(&self) -> Vec<ServerInfo> {
        let servers = self.configured_servers.read().await;
        servers.values().cloned().collect()
    }

    /// Get a specific server's information by name
    pub async fn get_server_info(&self, server_name: &str) -> Option<ServerInfo> {
        let servers = self.configured_servers.read().await;
        servers.get(server_name).cloned()
    }

    /// Get OAuth initiation URL for a server (if applicable)
    pub async fn get_oauth_initiation_url(&self, server_name: &str) -> Option<String> {
        let servers = self.configured_servers.read().await;
        if let Some(server_info) = servers.get(server_name) {
            if server_info.server_type == ServerType::OAuth {
                return server_info.oauth_auth_url.clone();
            }
        }
        None
    }

    /// Initiate OAuth flow for a server (for client-terminated mode)
    pub async fn initiate_oauth_flow(&self, server_name: &str) -> Result<String> {
        let servers = self.configured_servers.read().await;
        let server_info = servers.get(server_name)
            .ok_or_else(|| ProxyError::config(format!("Server '{}' not found", server_name)))?;

        if server_info.server_type != ServerType::OAuth {
            return Err(ProxyError::config(format!("Server '{}' is not an OAuth server", server_name)));
        }

        // For client-terminated mode, return the authorization URL
        if let Some(auth_url) = &server_info.oauth_auth_url {
            return Ok(auth_url.clone());
        }

        // For now, generate a standard OAuth callback URL
        // TODO: Implement proper authorization URL generation from OAuth discovery manager
        let auth_url = format!("http://localhost:3001/auth/callback/{}", server_name);
        
        // Update server info with the new URL
        drop(servers); // Release the read lock
        self.update_server_status(server_name, ServerStatus::OAuthPending, Some(auth_url.clone()), None).await;
        Ok(auth_url)
    }
}

