//! OAuth-Enabled MCP Server Discovery and Connection Manager
//! 
//! This module implements dynamic discovery and OAuth credential management
//! for MCP servers, replacing npx mcp-remote with native OAuth integration.

use crate::config::{OAuthConfig, oauth_discovery::ManualOAuthMetadata};
use crate::error::{ProxyError, Result};
use crate::auth::{AuthResolver, UserContext, MultiLevelAuthConfig, TokenStorage, TokenData};
use secrecy::Secret;
use std::time::SystemTime;
use std::collections::HashMap;
use crate::mcp::clients::sse_client::{SseMcpClient, SseClientConfig, SseAuthConfig};
use crate::mcp::clients::http_client::{HttpMcpClient, HttpClientConfig, HttpAuthConfig};
use crate::security::audit_log::AuditLogger;
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, oneshot, Mutex};
use tracing::{debug, info, error, warn};
use url::Url;
use reqwest::Client;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use base64::{Engine as _, engine::general_purpose};
use regex::Regex;

/// OAuth-enabled MCP server discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthMcpDiscoveryConfig {
    /// Base URL of the MCP server
    pub base_url: String,
    /// OAuth termination preference - if true, MT handles OAuth; if false, forwards to client
    pub oauth_termination_here: bool,
    /// Discovery endpoint for OAuth metadata (RFC 8414)
    pub discovery_endpoint: Option<String>,
    /// OAuth provider configuration (for static credentials fallback)
    pub oauth_provider: Option<String>,
    /// Required scopes override (optional - defaults to discovered scopes)
    pub required_scopes_override: Option<Vec<String>>,
    /// Whether to enable dynamic registration (RFC 7591)
    pub enable_dynamic_registration: bool,
    /// Client registration metadata for dynamic registration
    pub registration_metadata: Option<DynamicRegistrationMetadata>,
    /// Manual OAuth metadata override (only used when discovery fails)
    pub manual_oauth_metadata: Option<ManualOAuthMetadata>,
}

/// Dynamic OAuth client registration metadata (RFC 7591)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicRegistrationMetadata {
    pub client_name: String,
    pub redirect_uri_template: String,
    /// Requested scopes override (optional - uses discovered scopes by default)
    pub requested_scopes_override: Option<Vec<String>>,
    /// Grant types override (optional - uses discovered grant types by default)
    pub grant_types_override: Option<Vec<String>>,
    /// Response types override (optional - uses discovered response types by default)
    pub response_types_override: Option<Vec<String>>,
    pub application_type: String,
    pub client_uri: Option<String>,
    pub logo_uri: Option<String>,
    pub tos_uri: Option<String>,
    pub policy_uri: Option<String>,
}

// Use ManualOAuthMetadata from config module instead of duplicating

/// OAuth server metadata from RFC 8414 discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub registration_endpoint: Option<String>,
    /// Discovered scopes supported by the authorization server (RFC 8414)
    pub scopes_supported: Option<Vec<String>>,
    /// Discovered response types supported (RFC 8414 - REQUIRED)
    pub response_types_supported: Vec<String>,
    /// Discovered grant types supported (RFC 8414 - defaults to ["authorization_code"])
    pub grant_types_supported: Option<Vec<String>>,
    pub code_challenge_methods_supported: Option<Vec<String>>,
}

/// Protected resource metadata from RFC 9728 discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedResourceMetadata {
    /// URL of the protected resource (REQUIRED)
    pub resource: String,
    /// Authorization servers that can issue tokens for this resource
    pub authorization_servers: Option<Vec<String>>,
    /// Resource-specific scopes supported
    pub scopes_supported: Option<Vec<String>>,
    /// Bearer token methods supported
    pub bearer_methods_supported: Option<Vec<String>>,
}

/// Combined discovered OAuth configuration
#[derive(Debug, Clone)]
pub struct DiscoveredOAuthConfig {
    /// Authorization server metadata (RFC 8414)
    pub auth_server_metadata: OAuthServerMetadata,
    /// Protected resource metadata (RFC 9728, optional)
    pub resource_metadata: Option<ProtectedResourceMetadata>,
    /// Final resolved scopes (intersection of auth server and resource scopes)
    pub resolved_scopes: Vec<String>,
    /// Final resolved grant types
    pub resolved_grant_types: Vec<String>,
    /// Final resolved response types
    pub resolved_response_types: Vec<String>,
    /// Discovery timestamp for caching
    pub discovered_at: DateTime<Utc>,
}

/// Dynamic OAuth credentials from registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicOAuthCredentials {
    pub client_id: String,
    #[serde(skip_serializing)]
    pub client_secret: String,
    pub registration_token: Option<String>,
    pub registration_client_uri: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub server_endpoint: String,
    pub granted_scopes: Vec<String>,
    pub metadata: HashMap<String, Value>,
}

/// OAuth-enabled MCP connection state
#[derive(Debug, Clone)]
pub struct OAuthMcpConnection {
    pub server_name: String,
    pub base_url: String,
    pub credentials: Option<DynamicOAuthCredentials>,
    pub access_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub connection_established: bool,
    pub last_auth_attempt: Option<DateTime<Utc>>,
}

/// OAuth callback data received from the callback endpoint
#[derive(Debug, Clone)]
pub struct OAuthCallbackData {
    pub authorization_code: String,
    pub state: String,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// Main OAuth-enabled MCP discovery manager
pub struct OAuthMcpDiscoveryManager {
    /// OAuth configuration
    oauth_config: OAuthConfig,
    /// Auth resolver for OAuth flows
    auth_resolver: Arc<AuthResolver>,
    /// Active OAuth-enabled MCP connections
    connections: Arc<RwLock<HashMap<String, OAuthMcpConnection>>>,
    /// Cached discovered OAuth configurations (server_name -> config)
    discovery_cache: Arc<RwLock<HashMap<String, CachedDiscoveredOAuthConfig>>>,
    /// HTTP client for discovery and API calls
    http_client: Client,
    /// Audit logger for OAuth events
    audit_logger: Arc<AuditLogger>,
    /// User context for credential storage
    user_context: Arc<UserContext>,
    /// OAuth callback coordination - maps server_name to callback sender
    callback_coordinators: Arc<Mutex<HashMap<String, oneshot::Sender<OAuthCallbackData>>>>,
}

/// Cached discovered OAuth configuration with TTL
#[derive(Debug, Clone)]
pub struct CachedDiscoveredOAuthConfig {
    /// The discovered configuration
    pub config: DiscoveredOAuthConfig,
    /// Cache expiration time (for revalidation)
    pub expires_at: DateTime<Utc>,
    /// Configuration hash for change detection
    pub config_hash: String,
}

impl OAuthMcpDiscoveryManager {
    /// Create new OAuth-enabled MCP discovery manager
    pub async fn new(
        oauth_config: OAuthConfig,
        multi_level_config: MultiLevelAuthConfig,
        audit_logger: Arc<AuditLogger>,
    ) -> Result<Self> {
        let user_context = UserContext::new()?;
        let auth_resolver = Arc::new(AuthResolver::with_user_context(multi_level_config, user_context)?);
        let user_context = Arc::new(UserContext::new()?);
        
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProxyError::connection(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            oauth_config,
            auth_resolver,
            connections: Arc::new(RwLock::new(HashMap::new())),
            discovery_cache: Arc::new(RwLock::new(HashMap::new())),
            http_client,
            audit_logger,
            user_context,
            callback_coordinators: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Discover and connect to OAuth-enabled MCP server
    pub async fn discover_and_connect(
        &self,
        server_name: String,
        config: OAuthMcpDiscoveryConfig,
    ) -> Result<()> {
        info!("🔍 Starting OAuth-enabled MCP discovery for server: {}", server_name);
        
        // Log discovery attempt
        self.audit_logger.log_oauth_discovery_attempt(&server_name, &config.base_url).await;

        // Step 1: Discover and resolve OAuth configuration (RFC 8414/9728)
        let oauth_config = self.discover_oauth_configuration(&config).await?;
        info!("✅ Discovered and resolved OAuth configuration for server: {}", server_name);

        // Step 2: Handle dynamic registration if enabled
        let credentials = if config.enable_dynamic_registration && oauth_config.auth_server_metadata.registration_endpoint.is_some() {
            self.perform_dynamic_registration(&server_name, &config, &oauth_config).await?
        } else {
            // Use static credentials from configuration
            self.load_static_credentials(&server_name, &config).await?
        };

        // Step 3: Initialize connection state
        let connection = OAuthMcpConnection {
            server_name: server_name.clone(),
            base_url: config.base_url.clone(),
            credentials: Some(credentials),
            access_token: None,
            token_expires_at: None,
            connection_established: false,
            last_auth_attempt: None,
        };

        // Store connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(server_name.clone(), connection);
        }

        // Step 4: Perform OAuth authorization flow based on termination preference
        if config.oauth_termination_here {
            // Spawn OAuth authorization as a background task to avoid blocking the main thread
            self.spawn_oauth_authorization(server_name.clone(), config.clone(), oauth_config.clone()).await?;
        } else {
            self.forward_oauth_to_client(&server_name, &config, &oauth_config).await?;
        }

        info!("🎉 Successfully established OAuth-enabled MCP connection: {}", server_name);
        Ok(())
    }

    /// Discover and resolve OAuth configuration using RFC 8414/9728 with caching
    async fn discover_oauth_configuration(
        &self,
        config: &OAuthMcpDiscoveryConfig,
    ) -> Result<DiscoveredOAuthConfig> {
        let server_name = self.extract_server_name_from_url(&config.base_url);
        
        // Check cache first
        if let Some(cached_config) = self.get_cached_oauth_config(&server_name).await? {
            info!("💾 Using cached OAuth configuration for: {}", server_name);
            return Ok(cached_config.config);
        }

        info!("🔍 Starting RFC-compliant OAuth discovery for: {}", config.base_url);

        // Step 1: Discover authorization server metadata (RFC 8414)
        let auth_server_metadata = self.discover_authorization_server_metadata(config).await
            .or_else(|e| {
                warn!("RFC 8414 discovery failed: {}, trying manual fallback", e);
                self.use_manual_oauth_metadata(config)
            })?;

        // Step 2: Discover protected resource metadata (RFC 9728, optional)
        let resource_metadata = self.discover_protected_resource_metadata(config).await
            .map_err(|e| {
                debug!("RFC 9728 discovery failed (optional): {}", e);
                e
            })
            .ok();

        // Step 3: Resolve final scopes from discovery results
        let resolved_scopes = self.resolve_scopes(
            &auth_server_metadata,
            &resource_metadata,
            config
        )?;

        // Step 4: Resolve grant types and response types
        let resolved_grant_types = self.resolve_grant_types(&auth_server_metadata, config)?;
        let resolved_response_types = self.resolve_response_types(&auth_server_metadata, config)?;

        let discovered_config = DiscoveredOAuthConfig {
            auth_server_metadata,
            resource_metadata,
            resolved_scopes,
            resolved_grant_types,
            resolved_response_types,
            discovered_at: Utc::now(),
        };

        info!("✅ OAuth discovery completed: scopes={:?}, grant_types={:?}", 
              discovered_config.resolved_scopes, discovered_config.resolved_grant_types);

        // Cache the discovered configuration
        self.cache_oauth_config(&server_name, &discovered_config).await?;
        
        // Validate the discovered configuration
        self.validate_discovered_config(&discovered_config, config)?;

        Ok(discovered_config)
    }

    /// Discover authorization server metadata using RFC 8414
    async fn discover_authorization_server_metadata(
        &self,
        config: &OAuthMcpDiscoveryConfig,
    ) -> Result<OAuthServerMetadata> {
        let discovery_url = if let Some(ref endpoint) = config.discovery_endpoint {
            endpoint.clone()
        } else {
            // Default to RFC 8414 well-known endpoint
            let base_url = Url::parse(&config.base_url)
                .map_err(|e| ProxyError::config(format!("Invalid base URL: {}", e)))?;
            format!("{}/.well-known/oauth-authorization-server", base_url.origin().ascii_serialization())
        };

        debug!("🔍 RFC 8414: Discovering authorization server metadata from: {}", discovery_url);

        let response = self.http_client
            .get(&discovery_url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("Failed to fetch OAuth metadata: {}", e)))?;

        if !response.status().is_success() {
            return Err(ProxyError::auth(format!(
                "OAuth discovery failed with status {}: {}", 
                response.status(), 
                discovery_url
            )));
        }

        let mut metadata: OAuthServerMetadata = response
            .json()
            .await
            .map_err(|e| ProxyError::auth(format!("Failed to parse OAuth metadata: {}", e)))?;

        // Validate required fields per RFC 8414
        if metadata.authorization_endpoint.is_empty() {
            return Err(ProxyError::auth("Invalid OAuth metadata: missing authorization_endpoint (RFC 8414 required)".to_string()));
        }
        if metadata.response_types_supported.is_empty() {
            return Err(ProxyError::auth("Invalid OAuth metadata: missing response_types_supported (RFC 8414 required)".to_string()));
        }

        // Apply RFC 8414 defaults
        if metadata.grant_types_supported.is_none() {
            metadata.grant_types_supported = Some(vec!["authorization_code".to_string()]);
        }
        if metadata.token_endpoint.is_empty() && metadata.grant_types_supported.as_ref().unwrap().contains(&"authorization_code".to_string()) {
            return Err(ProxyError::auth("Invalid OAuth metadata: token_endpoint required for authorization_code grant".to_string()));
        }

        debug!("✅ RFC 8414: Discovered metadata - issuer={}, scopes_supported={:?}, grant_types_supported={:?}", 
               metadata.issuer, metadata.scopes_supported, metadata.grant_types_supported);

        Ok(metadata)
    }

    /// Discover protected resource metadata using RFC 9728 (optional)
    async fn discover_protected_resource_metadata(
        &self,
        config: &OAuthMcpDiscoveryConfig,
    ) -> Result<ProtectedResourceMetadata> {
        let base_url = Url::parse(&config.base_url)
            .map_err(|e| ProxyError::config(format!("Invalid base URL: {}", e)))?;
        let discovery_url = format!("{}/.well-known/oauth-protected-resource", base_url.origin().ascii_serialization());

        debug!("🔍 RFC 9728: Discovering protected resource metadata from: {}", discovery_url);

        let response = self.http_client
            .get(&discovery_url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("Failed to fetch resource metadata: {}", e)))?;

        if !response.status().is_success() {
            return Err(ProxyError::auth(format!(
                "Resource discovery failed with status {}: {}", 
                response.status(), 
                discovery_url
            )));
        }

        let metadata: ProtectedResourceMetadata = response
            .json()
            .await
            .map_err(|e| ProxyError::auth(format!("Failed to parse resource metadata: {}", e)))?;

        // Validate required field per RFC 9728
        if metadata.resource.is_empty() {
            return Err(ProxyError::auth("Invalid resource metadata: missing resource field (RFC 9728 required)".to_string()));
        }

        debug!("✅ RFC 9728: Discovered resource metadata - resource={}, scopes_supported={:?}", 
               metadata.resource, metadata.scopes_supported);

        Ok(metadata)
    }

    /// Fallback to manual OAuth metadata when discovery fails
    fn use_manual_oauth_metadata(
        &self,
        config: &OAuthMcpDiscoveryConfig,
    ) -> Result<OAuthServerMetadata> {
        let manual_metadata = config.manual_oauth_metadata.as_ref()
            .ok_or_else(|| ProxyError::auth("OAuth discovery failed and no manual fallback configured".to_string()))?;

        warn!("⚠️  Using manual OAuth metadata fallback (RFC discovery failed)");

        let metadata = OAuthServerMetadata {
            issuer: config.base_url.clone(),
            authorization_endpoint: manual_metadata.authorization_endpoint.clone(),
            token_endpoint: manual_metadata.token_endpoint.clone(),
            registration_endpoint: manual_metadata.registration_endpoint.clone(),
            scopes_supported: manual_metadata.scopes_supported.clone(),
            response_types_supported: manual_metadata.response_types_supported.clone()
                .unwrap_or_else(|| vec!["code".to_string()]),
            grant_types_supported: manual_metadata.grant_types_supported.clone(),
            code_challenge_methods_supported: manual_metadata.code_challenge_methods_supported.clone(),
        };

        debug!("⚙️  Manual fallback metadata: scopes={:?}, grant_types={:?}", 
               metadata.scopes_supported, metadata.grant_types_supported);

        Ok(metadata)
    }

    /// Resolve final scopes from discovery results and configuration
    fn resolve_scopes(
        &self,
        auth_server_metadata: &OAuthServerMetadata,
        resource_metadata: &Option<ProtectedResourceMetadata>,
        config: &OAuthMcpDiscoveryConfig,
    ) -> Result<Vec<String>> {
        // Priority order:
        // 1. Manual override from config (if specified)
        // 2. Intersection of auth server + resource scopes (if both available)
        // 3. Resource scopes (if available)
        // 4. Auth server scopes (if available)
        // 5. Default MCP scopes

        // Check for manual override first
        if let Some(ref override_scopes) = config.required_scopes_override {
            self.validate_scope_override(override_scopes, auth_server_metadata, resource_metadata)?;
            info!("🔧 Using manual scope override: {:?}", override_scopes);
            return Ok(override_scopes.clone());
        }

        // Try to use discovered scopes
        let auth_scopes = auth_server_metadata.scopes_supported.as_ref();
        let resource_scopes = resource_metadata.as_ref().and_then(|rm| rm.scopes_supported.as_ref());

        let resolved_scopes = match (auth_scopes, resource_scopes) {
            (Some(auth), Some(resource)) => {
                // Use intersection of both
                let intersection: Vec<String> = auth.iter()
                    .filter(|scope| resource.contains(scope))
                    .cloned()
                    .collect();
                if intersection.is_empty() {
                    warn!("⚠️  No scope intersection found, using auth server scopes");
                    auth.clone()
                } else {
                    info!("🔗 Using scope intersection: {:?}", intersection);
                    intersection
                }
            },
            (Some(auth), None) => {
                info!("🔑 Using auth server scopes: {:?}", auth);
                auth.clone()
            },
            (None, Some(resource)) => {
                info!("📦 Using resource scopes: {:?}", resource);
                resource.clone()
            },
            (None, None) => {
                info!("🎯 Using default MCP scopes (no discovery available)");
                vec!["mcp:read".to_string(), "mcp:write".to_string(), "mcp:tools".to_string()]
            }
        };

        if resolved_scopes.is_empty() {
            return Err(ProxyError::config("No valid scopes could be resolved".to_string()));
        }

        Ok(resolved_scopes)
    }

    /// Resolve grant types from discovery results and configuration
    fn resolve_grant_types(
        &self,
        auth_server_metadata: &OAuthServerMetadata,
        config: &OAuthMcpDiscoveryConfig,
    ) -> Result<Vec<String>> {
        // Check for manual override in dynamic registration metadata
        if let Some(ref reg_metadata) = config.registration_metadata {
            if let Some(ref override_grants) = reg_metadata.grant_types_override {
                self.validate_grant_type_override(override_grants, auth_server_metadata)?;
                info!("🔧 Using manual grant type override: {:?}", override_grants);
                return Ok(override_grants.clone());
            }
        }

        // Use discovered grant types or RFC 8414 default
        let resolved_grants = auth_server_metadata.grant_types_supported.clone()
            .unwrap_or_else(|| {
                info!("🎯 Using RFC 8414 default grant type: [authorization_code]");
                vec!["authorization_code".to_string()]
            });

        if resolved_grants.is_empty() {
            return Err(ProxyError::config("No valid grant types could be resolved".to_string()));
        }

        info!("⚙️  Resolved grant types: {:?}", resolved_grants);
        Ok(resolved_grants)
    }

    /// Resolve response types from discovery results and configuration
    fn resolve_response_types(
        &self,
        auth_server_metadata: &OAuthServerMetadata,
        config: &OAuthMcpDiscoveryConfig,
    ) -> Result<Vec<String>> {
        // Check for manual override in dynamic registration metadata
        if let Some(ref reg_metadata) = config.registration_metadata {
            if let Some(ref override_responses) = reg_metadata.response_types_override {
                self.validate_response_type_override(override_responses, auth_server_metadata)?;
                info!("🔧 Using manual response type override: {:?}", override_responses);
                return Ok(override_responses.clone());
            }
        }

        // Use discovered response types (RFC 8414 required field)
        let resolved_responses = auth_server_metadata.response_types_supported.clone();

        if resolved_responses.is_empty() {
            return Err(ProxyError::config("No valid response types available (RFC 8414 violation)".to_string()));
        }

        info!("⚙️  Resolved response types: {:?}", resolved_responses);
        Ok(resolved_responses)
    }

    /// Validate that manual scope override is subset of discovered scopes
    fn validate_scope_override(
        &self,
        override_scopes: &[String],
        auth_server_metadata: &OAuthServerMetadata,
        resource_metadata: &Option<ProtectedResourceMetadata>,
    ) -> Result<()> {
        // Collect all discovered scopes
        let mut discovered_scopes = Vec::new();
        
        if let Some(ref auth_scopes) = auth_server_metadata.scopes_supported {
            discovered_scopes.extend(auth_scopes.iter().cloned());
        }
        
        if let Some(ref resource_meta) = resource_metadata {
            if let Some(ref resource_scopes) = resource_meta.scopes_supported {
                discovered_scopes.extend(resource_scopes.iter().cloned());
            }
        }

        // If no scopes were discovered, allow override (discovery may have failed)
        if discovered_scopes.is_empty() {
            warn!("⚠️  No scopes discovered, allowing manual override");
            return Ok(());
        }

        // Check that all override scopes are supported
        for scope in override_scopes {
            if !discovered_scopes.contains(scope) {
                warn!("⚠️  Manual scope '{}' not in discovered scopes: {:?}", scope, discovered_scopes);
            }
        }

        Ok(())
    }

    /// Validate that manual grant type override is subset of discovered grant types
    fn validate_grant_type_override(
        &self,
        override_grants: &[String],
        auth_server_metadata: &OAuthServerMetadata,
    ) -> Result<()> {
        if let Some(ref discovered_grants) = auth_server_metadata.grant_types_supported {
            for grant in override_grants {
                if !discovered_grants.contains(grant) {
                    warn!("⚠️  Manual grant type '{}' not in discovered grant types: {:?}", grant, discovered_grants);
                }
            }
        }
        Ok(())
    }

    /// Validate that manual response type override is subset of discovered response types
    fn validate_response_type_override(
        &self,
        override_responses: &[String],
        auth_server_metadata: &OAuthServerMetadata,
    ) -> Result<()> {
        for response in override_responses {
            if !auth_server_metadata.response_types_supported.contains(response) {
                warn!("⚠️  Manual response type '{}' not in discovered response types: {:?}", 
                      response, auth_server_metadata.response_types_supported);
            }
        }
        Ok(())
    }

    /// Perform dynamic OAuth client registration (RFC 7591)
    async fn perform_dynamic_registration(
        &self,
        server_name: &str,
        config: &OAuthMcpDiscoveryConfig,
        oauth_config: &DiscoveredOAuthConfig,
    ) -> Result<DynamicOAuthCredentials> {
        let registration_endpoint = oauth_config.auth_server_metadata.registration_endpoint.as_ref()
            .ok_or_else(|| ProxyError::auth("Server does not support dynamic registration".to_string()))?;

        let reg_metadata = config.registration_metadata.as_ref()
            .ok_or_else(|| ProxyError::config("Dynamic registration enabled but no metadata provided".to_string()))?;

        info!("🔐 Performing dynamic OAuth registration for server: {}", server_name);

        // Generate dynamic redirect URI
        let redirect_uri = reg_metadata.redirect_uri_template
            .replace("{{server_name}}", server_name)
            .replace("{{port}}", "3001"); // Default MagicTunnel port

        // Build registration request with only non-null optional fields
        let mut registration_request = serde_json::Map::new();
        
        // Required fields - replace template variables in client_name
        let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string());
        let client_name = reg_metadata.client_name
            .replace("{{server_name}}", server_name)
            .replace("{{hostname}}", &hostname);
        registration_request.insert("client_name".to_string(), json!(client_name));
        registration_request.insert("redirect_uris".to_string(), json!([redirect_uri]));
        registration_request.insert("scope".to_string(), json!(oauth_config.resolved_scopes.join(" "))); // RFC-discovered scopes
        registration_request.insert("grant_types".to_string(), json!(oauth_config.resolved_grant_types)); // RFC-discovered grant types
        registration_request.insert("response_types".to_string(), json!(oauth_config.resolved_response_types)); // RFC-discovered response types
        registration_request.insert("token_endpoint_auth_method".to_string(), json!("client_secret_basic"));
        registration_request.insert("application_type".to_string(), json!(reg_metadata.application_type));
        
        // client_uri is also optional
        if let Some(ref client_uri) = reg_metadata.client_uri {
            registration_request.insert("client_uri".to_string(), json!(client_uri));
        }
        
        // Optional fields - only include if they have actual values (not None)
        if let Some(ref logo_uri) = reg_metadata.logo_uri {
            registration_request.insert("logo_uri".to_string(), json!(logo_uri));
        }
        
        if let Some(ref tos_uri) = reg_metadata.tos_uri {
            registration_request.insert("tos_uri".to_string(), json!(tos_uri));
        }
        
        if let Some(ref policy_uri) = reg_metadata.policy_uri {
            registration_request.insert("policy_uri".to_string(), json!(policy_uri));
        }
        
        let registration_request = json!(registration_request);

        // Debug each field individually to avoid truncation
        info!("📋 Registration request fields:");
        info!("   client_name: {:?}", registration_request.get("client_name"));
        info!("   redirect_uris: {:?}", registration_request.get("redirect_uris"));
        info!("   scope: {:?}", registration_request.get("scope"));
        info!("   grant_types: {:?}", registration_request.get("grant_types"));
        info!("   response_types: {:?}", registration_request.get("response_types"));
        info!("   application_type: {:?}", registration_request.get("application_type"));
        info!("   client_uri: {:?}", registration_request.get("client_uri"));
        info!("   logo_uri: {:?}", registration_request.get("logo_uri"));
        info!("   tos_uri: {:?}", registration_request.get("tos_uri"));
        info!("   policy_uri: {:?}", registration_request.get("policy_uri"));
        info!("📋 Full registration request JSON: {}", serde_json::to_string_pretty(&registration_request).unwrap_or_default());

        let response = self.http_client
            .post(registration_endpoint)
            .header("Content-Type", "application/json")
            .json(&registration_request)
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("Failed to perform dynamic registration: {}", e)))?;

        if !response.status().is_success() {
            let status_code = response.status();
            let error_text = response.text().await.unwrap_or_default();
            self.audit_logger.log_oauth_registration_failure(server_name, &format!("Status {}: {}", status_code, error_text)).await;
            return Err(ProxyError::auth(format!("Dynamic registration failed: {}", error_text)));
        }

        let registration_response: Value = response
            .json()
            .await
            .map_err(|e| ProxyError::auth(format!("Failed to parse registration response: {}", e)))?;

        let credentials = DynamicOAuthCredentials {
            client_id: registration_response.get("client_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ProxyError::auth("Missing client_id in registration response".to_string()))?
                .to_string(),
            client_secret: registration_response.get("client_secret")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ProxyError::auth("Missing client_secret in registration response".to_string()))?
                .to_string(),
            registration_token: registration_response.get("registration_access_token")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            registration_client_uri: registration_response.get("registration_client_uri")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            expires_at: registration_response.get("client_secret_expires_at")
                .and_then(|v| v.as_u64())
                .filter(|&t| t != 0)
                .map(|t| DateTime::from_timestamp(t as i64, 0))
                .flatten(),
            server_endpoint: config.base_url.clone(),
            granted_scopes: registration_response.get("scope")
                .and_then(|v| v.as_str())
                .map(|s| s.split_whitespace().map(|scope| scope.to_string()).collect())
                .unwrap_or_else(|| oauth_config.resolved_scopes.clone()), // Fallback to discovered scopes
            metadata: registration_response.as_object()
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
        };

        // Store credentials securely
        self.store_dynamic_credentials(server_name, &credentials).await?;

        // Log successful registration
        self.audit_logger.log_oauth_registration_success(
            server_name, 
            &credentials.client_id, 
            &credentials.granted_scopes
        ).await;

        info!("✅ Dynamic OAuth registration successful for server: {} (client_id: {})", 
              server_name, credentials.client_id);

        Ok(credentials)
    }

    /// Load static OAuth credentials from configuration
    async fn load_static_credentials(
        &self,
        server_name: &str,
        config: &OAuthMcpDiscoveryConfig,
    ) -> Result<DynamicOAuthCredentials> {
        // Load from existing OAuth configuration or environment variables
        let provider = config.oauth_provider.as_ref()
            .ok_or_else(|| ProxyError::config("No OAuth provider specified for static credentials".to_string()))?;

        info!("🔑 Loading static OAuth credentials for server: {} (provider: {})", server_name, provider);

        // This would integrate with existing OAuth provider configurations
        // For now, return a placeholder that would be filled from actual config
        let credentials = DynamicOAuthCredentials {
            client_id: std::env::var(&format!("{}_CLIENT_ID", provider.to_uppercase()))
                .map_err(|_| ProxyError::config(format!("Missing client ID for provider: {}", provider)))?,
            client_secret: std::env::var(&format!("{}_CLIENT_SECRET", provider.to_uppercase()))
                .map_err(|_| ProxyError::config(format!("Missing client secret for provider: {}", provider)))?,
            registration_token: None,
            registration_client_uri: None,
            expires_at: None,
            server_endpoint: config.base_url.clone(),
            granted_scopes: vec![], // Will be filled from actual config or discovered scopes
            metadata: HashMap::new(),
        };

        Ok(credentials)
    }

    /// Store dynamic OAuth credentials securely
    async fn store_dynamic_credentials(
        &self,
        server_name: &str,
        credentials: &DynamicOAuthCredentials,
    ) -> Result<()> {
        debug!("💾 Storing dynamic OAuth credentials for server: {}", server_name);

        // Use existing token storage infrastructure
        let storage_key = format!("mcp_oauth_dynamic_{}", server_name);
        
        // Create token storage from user context
        let token_storage = TokenStorage::new((*self.user_context).clone()).await
            .map_err(|e| ProxyError::config(format!("Failed to create token storage: {}", e)))?;
            
        // Create token data
        let token_data = TokenData {
            access_token: Secret::new(credentials.client_secret.clone()),
            refresh_token: None,
            expires_at: credentials.expires_at.map(|dt| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64)),
            scopes: vec![], // No scopes for client credentials
            provider: "mcp_dynamic".to_string(),
            token_type: "client_credentials".to_string(),
            audience: None,
            resource: None,
            created_at: SystemTime::now(),
            last_refreshed: None,
            user_id: None,
            metadata: HashMap::new(),
        };
        
        // Store using existing secure storage mechanisms
        // This integrates with the existing token storage system
        token_storage.store_token(&storage_key, &token_data).await
            .map_err(|e| ProxyError::config(format!("Failed to store OAuth credentials: {}", e)))?;

        info!("✅ Stored dynamic OAuth credentials for server: {}", server_name);
        Ok(())
    }

    /// Spawn OAuth authorization as a background task (when oauth_termination_here = true)
    async fn spawn_oauth_authorization(
        &self,
        server_name: String,
        config: OAuthMcpDiscoveryConfig,
        oauth_config: DiscoveredOAuthConfig,
    ) -> Result<()> {
        info!("🚀 Spawning background OAuth authorization for server: {} (MT-terminated)", server_name);
        
        // Clone necessary references for the background task
        let connections = self.connections.clone();
        let audit_logger = self.audit_logger.clone();
        let http_client = self.http_client.clone();
        let callback_coordinators = self.callback_coordinators.clone();
        
        // Clone server_name for use after the spawn
        let server_name_for_spawn = server_name.clone();
        
        // Spawn the OAuth flow as a background task
        tokio::spawn(async move {
            if let Err(e) = Self::perform_oauth_authorization_task(
                server_name_for_spawn.clone(),
                config,
                oauth_config,
                connections,
                audit_logger,
                http_client,
                callback_coordinators,
            ).await {
                error!("❌ Background OAuth authorization failed for server '{}': {}", server_name_for_spawn, e);
            }
        });
        
        info!("✅ OAuth authorization task spawned for server: {}", server_name);
        Ok(())
    }

    /// Perform OAuth authorization flow task (static method for background execution)
    async fn perform_oauth_authorization_task(
        server_name: String,
        config: OAuthMcpDiscoveryConfig,
        oauth_config: DiscoveredOAuthConfig,
        connections: Arc<RwLock<HashMap<String, OAuthMcpConnection>>>,
        audit_logger: Arc<AuditLogger>,
        http_client: Client,
        callback_coordinators: Arc<Mutex<HashMap<String, oneshot::Sender<OAuthCallbackData>>>>,
    ) -> Result<()> {
        info!("🔐 Performing OAuth authorization for server: {} (MT-terminated)", server_name);

        let mut connection = {
            let connections = connections.read().await;
            connections.get(&server_name).cloned()
                .ok_or_else(|| ProxyError::connection(format!("Connection not found: {}", server_name)))?
        };

        let credentials = connection.credentials.as_ref()
            .ok_or_else(|| ProxyError::auth("No credentials available".to_string()))?;

        // Generate PKCE challenge
        let code_verifier = Self::generate_pkce_verifier();
        let code_challenge = Self::generate_pkce_challenge(&code_verifier);

        // Build authorization URL with discovered scopes
        let auth_url = Self::build_authorization_url(
            &oauth_config.auth_server_metadata.authorization_endpoint,
            &credentials.client_id,
            &oauth_config.resolved_scopes,
            &code_challenge,
            &server_name,
        )?;

        info!("🌐 Opening browser for OAuth authorization: {}", server_name);
        audit_logger.log_oauth_authorization_start(&server_name, &auth_url).await;

        // Open browser for authorization (MT handles the OAuth flow)
        Self::open_browser_for_authorization(&auth_url).await?;

        // Set up callback handler and wait for authorization code
        let auth_code = Self::wait_for_authorization_callback_static(&server_name, callback_coordinators).await?;

        // Exchange authorization code for access token
        let token_response = Self::exchange_authorization_code_static(
            &oauth_config.auth_server_metadata.token_endpoint,
            &credentials.client_id,
            &credentials.client_secret,
            &auth_code,
            &code_verifier,
            &server_name,
            &http_client,
            &audit_logger,
        ).await?;

        // Update connection with token
        connection.access_token = Some(token_response.access_token);
        connection.token_expires_at = token_response.expires_at;
        connection.connection_established = true;
        connection.last_auth_attempt = Some(Utc::now());

        // Store updated connection
        {
            let mut connections = connections.write().await;
            connections.insert(server_name.to_string(), connection);
        }

        audit_logger.log_oauth_authorization_success(&server_name).await;
        info!("✅ OAuth authorization completed for server: {}", server_name);

        Ok(())
    }

    /// Forward OAuth details to client (when oauth_termination_here = false)
    async fn forward_oauth_to_client(
        &self,
        server_name: &str,
        config: &OAuthMcpDiscoveryConfig,
        oauth_config: &DiscoveredOAuthConfig,
    ) -> Result<()> {
        info!("📤 Forwarding OAuth details to client for server: {} (client-terminated)", server_name);

        let connection = {
            let connections = self.connections.read().await;
            connections.get(server_name).cloned()
                .ok_or_else(|| ProxyError::connection(format!("Connection not found: {}", server_name)))?
        };

        let credentials = connection.credentials.as_ref()
            .ok_or_else(|| ProxyError::auth("No credentials available".to_string()))?;

        // Prepare OAuth details for client with discovered configuration
        let oauth_details = json!({
            "server_name": server_name,
            "oauth_termination_here": false,
            "authorization_endpoint": oauth_config.auth_server_metadata.authorization_endpoint,
            "token_endpoint": oauth_config.auth_server_metadata.token_endpoint,
            "client_id": credentials.client_id,
            "required_scopes": oauth_config.resolved_scopes, // RFC-discovered scopes
            "grant_types_supported": oauth_config.resolved_grant_types, // RFC-discovered grant types
            "response_types_supported": oauth_config.resolved_response_types, // RFC-discovered response types
            "pkce_required": oauth_config.auth_server_metadata.code_challenge_methods_supported
                .as_ref()
                .map(|methods| methods.contains(&"S256".to_string()))
                .unwrap_or(false),
            "discovery_timestamp": oauth_config.discovered_at.to_rfc3339(),
        });

        // Log OAuth forwarding
        self.audit_logger.log_oauth_forwarded_to_client(server_name, &oauth_details).await;

        // This would be sent to the MCP client through the existing protocol
        // The client would then handle the OAuth flow and send back the access token
        info!("📋 OAuth details prepared for client forwarding: {}", server_name);
        debug!("OAuth details: {}", serde_json::to_string_pretty(&oauth_details).unwrap_or_default());

        Ok(())
    }

    /// Generate PKCE verifier (static version)
    fn generate_pkce_verifier() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let verifier: String = (0..128)
            .map(|_| {
                let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
                chars[rng.gen_range(0..chars.len())] as char
            })
            .collect();
        verifier
    }

    /// Generate PKCE challenge from verifier (static version)
    fn generate_pkce_challenge(verifier: &str) -> String {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(verifier.as_bytes());
        general_purpose::URL_SAFE_NO_PAD.encode(digest)
    }

    /// Build OAuth authorization URL (static version)
    fn build_authorization_url(
        auth_endpoint: &str,
        client_id: &str,
        scopes: &[String],
        code_challenge: &str,
        server_name: &str,
    ) -> Result<String> {
        let mut url = Url::parse(auth_endpoint)
            .map_err(|e| ProxyError::config(format!("Invalid authorization endpoint: {}", e)))?;

        let redirect_uri = format!("http://localhost:3001/auth/callback/{}", server_name);
        let state = Self::generate_state();

        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("response_type", "code");
            query_pairs.append_pair("client_id", client_id);
            query_pairs.append_pair("redirect_uri", &redirect_uri);
            query_pairs.append_pair("scope", &scopes.join(" "));
            query_pairs.append_pair("state", &state);
            query_pairs.append_pair("code_challenge", code_challenge);
            query_pairs.append_pair("code_challenge_method", "S256");
        }

        Ok(url.to_string())
    }

    /// Generate cryptographically secure state parameter (static version)
    fn generate_state() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let state: String = (0..32)
            .map(|_| {
                let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
                chars[rng.gen_range(0..chars.len())] as char
            })
            .collect();
        state
    }

    /// Open browser for OAuth authorization (static version)
    async fn open_browser_for_authorization(auth_url: &str) -> Result<()> {
        info!("🌐 Opening browser for OAuth authorization: {}", auth_url);
        
        // Use system's default browser
        if let Err(e) = webbrowser::open(auth_url) {
            error!("Failed to open browser: {}", e);
            return Err(ProxyError::auth(format!("Failed to open browser for OAuth: {}", e)));
        }

        Ok(())
    }

    /// Wait for OAuth authorization callback
    async fn wait_for_authorization_callback(&self, server_name: &str) -> Result<String> {
        info!("⏳ Waiting for OAuth callback for server: {}", server_name);
        
        // Create a oneshot channel for this callback
        let (sender, receiver) = oneshot::channel::<OAuthCallbackData>();
        
        // Register the callback coordinator
        {
            let mut coordinators = self.callback_coordinators.lock().await;
            coordinators.insert(server_name.to_string(), sender);
        }
        
        // Wait for the callback with a timeout
        let timeout_duration = std::time::Duration::from_secs(300); // 5 minutes
        let callback_data = tokio::time::timeout(timeout_duration, receiver)
            .await
            .map_err(|_| ProxyError::auth(format!("OAuth callback timeout for server: {}", server_name)))?
            .map_err(|_| ProxyError::auth(format!("OAuth callback channel closed for server: {}", server_name)))?;
        
        // Check for OAuth errors
        if let Some(error) = callback_data.error {
            let error_desc = callback_data.error_description
                .unwrap_or_else(|| "Unknown OAuth error".to_string());
            return Err(ProxyError::auth(format!("OAuth error: {} - {}", error, error_desc)));
        }
        
        info!("✅ Received OAuth callback for server: {}", server_name);
        Ok(callback_data.authorization_code)
    }

    /// Wait for OAuth authorization callback (static version for background tasks)
    async fn wait_for_authorization_callback_static(
        server_name: &str,
        callback_coordinators: Arc<Mutex<HashMap<String, oneshot::Sender<OAuthCallbackData>>>>,
    ) -> Result<String> {
        info!("⏳ Waiting for OAuth callback for server: {}", server_name);
        
        // Create a oneshot channel for this callback
        let (sender, receiver) = oneshot::channel::<OAuthCallbackData>();
        
        // Register the callback coordinator
        {
            let mut coordinators = callback_coordinators.lock().await;
            coordinators.insert(server_name.to_string(), sender);
        }
        
        // Wait for the callback with a timeout
        let timeout_duration = std::time::Duration::from_secs(300); // 5 minutes
        let callback_data = tokio::time::timeout(timeout_duration, receiver)
            .await
            .map_err(|_| ProxyError::auth(format!("OAuth callback timeout for server: {}", server_name)))?
            .map_err(|_| ProxyError::auth(format!("OAuth callback channel closed for server: {}", server_name)))?;
        
        // Check for OAuth errors
        if let Some(error) = callback_data.error {
            let error_desc = callback_data.error_description
                .unwrap_or_else(|| "Unknown OAuth error".to_string());
            return Err(ProxyError::auth(format!("OAuth error: {} - {}", error, error_desc)));
        }
        
        info!("✅ Received OAuth callback for server: {}", server_name);
        Ok(callback_data.authorization_code)
    }

    /// Exchange authorization code for access token
    async fn exchange_authorization_code(
        &self,
        token_endpoint: &str,
        client_id: &str,
        client_secret: &str,
        auth_code: &str,
        code_verifier: &str,
        server_name: &str,
    ) -> Result<TokenResponse> {
        info!("🔄 Exchanging authorization code for access token: {}", server_name);

        let token_request = [
            ("grant_type", "authorization_code"),
            ("code", auth_code),
            ("redirect_uri", &format!("http://localhost:3001/auth/callback/{}", server_name)),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code_verifier", code_verifier),
        ];

        let response = self.http_client
            .post(token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&token_request)
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("Token exchange failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            self.audit_logger.log_oauth_token_exchange_failure(server_name, &error_text).await;
            return Err(ProxyError::auth(format!("Token exchange failed: {}", error_text)));
        }

        let token_data: Value = response
            .json()
            .await
            .map_err(|e| ProxyError::auth(format!("Failed to parse token response: {}", e)))?;

        let access_token = token_data.get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::auth("Missing access_token in response".to_string()))?
            .to_string();

        let expires_in = token_data.get("expires_in")
            .and_then(|v| v.as_u64());

        let expires_at = expires_in.map(|seconds| Utc::now() + chrono::Duration::seconds(seconds as i64));

        self.audit_logger.log_oauth_token_exchange_success(server_name).await;

        Ok(TokenResponse {
            access_token,
            expires_at,
        })
    }

    /// Exchange authorization code for access token (static version)
    async fn exchange_authorization_code_static(
        token_endpoint: &str,
        client_id: &str,
        client_secret: &str,
        auth_code: &str,
        code_verifier: &str,
        server_name: &str,
        http_client: &Client,
        audit_logger: &Arc<AuditLogger>,
    ) -> Result<TokenResponse> {
        info!("🔄 Exchanging authorization code for access token: {}", server_name);

        let token_request = [
            ("grant_type", "authorization_code"),
            ("code", auth_code),
            ("redirect_uri", &format!("http://localhost:3001/auth/callback/{}", server_name)),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code_verifier", code_verifier),
        ];

        let response = http_client
            .post(token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&token_request)
            .send()
            .await
            .map_err(|e| ProxyError::connection(format!("Token exchange failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            audit_logger.log_oauth_token_exchange_failure(server_name, &error_text).await;
            return Err(ProxyError::auth(format!("Token exchange failed: {}", error_text)));
        }

        let token_data: Value = response
            .json()
            .await
            .map_err(|e| ProxyError::auth(format!("Failed to parse token response: {}", e)))?;

        let access_token = token_data.get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProxyError::auth("Missing access_token in response".to_string()))?
            .to_string();

        let expires_in = token_data.get("expires_in")
            .and_then(|v| v.as_u64());

        let expires_at = expires_in.map(|seconds| Utc::now() + chrono::Duration::seconds(seconds as i64));

        audit_logger.log_oauth_token_exchange_success(server_name).await;

        Ok(TokenResponse {
            access_token,
            expires_at,
        })
    }

    /// Get authenticated MCP connection for making API calls
    pub async fn get_authenticated_connection(&self, server_name: &str) -> Result<AuthenticatedMcpConnection> {
        let connection = {
            let connections = self.connections.read().await;
            connections.get(server_name).cloned()
                .ok_or_else(|| ProxyError::connection(format!("Connection not found: {}", server_name)))?
        };

        if !connection.connection_established {
            return Err(ProxyError::auth(format!("OAuth not completed for server: {}", server_name)));
        }

        let access_token = connection.access_token
            .ok_or_else(|| ProxyError::auth(format!("No access token for server: {}", server_name)))?;

        // Check token expiration
        if let Some(expires_at) = connection.token_expires_at {
            if Utc::now() >= expires_at {
                // Token expired, need to refresh
                warn!("Access token expired for server: {}", server_name);
                return Err(ProxyError::auth(format!("Access token expired for server: {}", server_name)));
            }
        }

        Ok(AuthenticatedMcpConnection {
            server_name: server_name.to_string(),
            base_url: connection.base_url,
            access_token,
        })
    }
}

/// Token exchange response
#[derive(Debug)]
struct TokenResponse {
    access_token: String,
    expires_at: Option<DateTime<Utc>>,
}

/// Authenticated MCP connection ready for API calls
#[derive(Debug, Clone)]
pub struct AuthenticatedMcpConnection {
    pub server_name: String,
    pub base_url: String,
    pub access_token: String,
}

impl AuthenticatedMcpConnection {
    /// Create authenticated SSE client with OAuth bearer token
    pub fn create_sse_client(&self) -> Result<SseMcpClient> {
        let config = SseClientConfig {
            base_url: self.base_url.clone(),
            auth: SseAuthConfig::Bearer {
                token: self.access_token.clone(),
            },
            single_session: true,
            connection_timeout: 30,
            request_timeout: 60,
            max_queue_size: 100,
            heartbeat_interval: 30,
            reconnect: true,
            max_reconnect_attempts: 10,
            reconnect_delay_ms: 1000,
            max_reconnect_delay_ms: 30000,
        };

        SseMcpClient::new(config, format!("oauth-{}", self.server_name))
    }

    /// Create authenticated HTTP client with OAuth bearer token
    pub fn create_http_client(&self) -> Result<HttpMcpClient> {
        let config = HttpClientConfig {
            base_url: self.base_url.clone(),
            auth: HttpAuthConfig::Bearer {
                token: self.access_token.clone(),
            },
            timeout: 30,
            retry_attempts: 3,
            retry_delay_ms: 1000,
            max_idle_connections: Some(10),
            idle_timeout: Some(60),
        };

        HttpMcpClient::new(config, format!("oauth-{}", self.server_name))
    }
}

impl OAuthMcpDiscoveryManager {
    /// Extract server name from base URL for caching
    fn extract_server_name_from_url(&self, base_url: &str) -> String {
        // Use host and path as server identifier
        if let Ok(url) = Url::parse(base_url) {
            format!("{}{}", url.host_str().unwrap_or("unknown"), url.path())
                .replace('/', "_")
                .replace('.', "_")
        } else {
            base_url.replace('/', "_").replace('.', "_")
        }
    }

    /// Get cached OAuth configuration if valid and not expired
    async fn get_cached_oauth_config(&self, server_name: &str) -> Result<Option<CachedDiscoveredOAuthConfig>> {
        let cache = self.discovery_cache.read().await;
        
        if let Some(cached) = cache.get(server_name) {
            // Check if cache is still valid
            if Utc::now() < cached.expires_at {
                debug!("✨ Cache hit for OAuth config: {} (expires: {})", server_name, cached.expires_at);
                return Ok(Some(cached.clone()));
            } else {
                debug!("⏰ Cache expired for OAuth config: {} (expired: {})", server_name, cached.expires_at);
            }
        }
        
        Ok(None)
    }

    /// Cache discovered OAuth configuration
    async fn cache_oauth_config(
        &self,
        server_name: &str,
        discovered_config: &DiscoveredOAuthConfig,
    ) -> Result<()> {
        let config_hash = self.calculate_config_hash(discovered_config)?;
        let cache_ttl = chrono::Duration::hours(1); // Cache for 1 hour
        
        let cached_config = CachedDiscoveredOAuthConfig {
            config: discovered_config.clone(),
            expires_at: Utc::now() + cache_ttl,
            config_hash,
        };
        
        {
            let mut cache = self.discovery_cache.write().await;
            cache.insert(server_name.to_string(), cached_config.clone());
        }
        
        debug!("💾 Cached OAuth config for: {} (expires: {})", server_name, cached_config.expires_at);
        Ok(())
    }

    /// Calculate configuration hash for change detection
    fn calculate_config_hash(&self, config: &DiscoveredOAuthConfig) -> Result<String> {
        let config_data = format!(
            "{}{}{:?}{:?}{:?}",
            config.auth_server_metadata.authorization_endpoint,
            config.auth_server_metadata.token_endpoint,
            config.resolved_scopes,
            config.resolved_grant_types,
            config.resolved_response_types
        );
        
        let mut hasher = Sha256::new();
        hasher.update(config_data.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        
        Ok(hash[..16].to_string()) // Use first 16 chars
    }

    /// Validate discovered OAuth configuration against manual overrides
    fn validate_discovered_config(
        &self,
        discovered_config: &DiscoveredOAuthConfig,
        config: &OAuthMcpDiscoveryConfig,
    ) -> Result<()> {
        // Validate that we have required endpoints
        if discovered_config.auth_server_metadata.authorization_endpoint.is_empty() {
            return Err(ProxyError::config("Discovered OAuth config missing authorization_endpoint".to_string()));
        }
        
        if discovered_config.auth_server_metadata.token_endpoint.is_empty() {
            return Err(ProxyError::config("Discovered OAuth config missing token_endpoint".to_string()));
        }
        
        // Validate that we have at least one scope
        if discovered_config.resolved_scopes.is_empty() {
            return Err(ProxyError::config("No OAuth scopes resolved (check server capabilities)".to_string()));
        }
        
        // Validate that we have at least one grant type
        if discovered_config.resolved_grant_types.is_empty() {
            return Err(ProxyError::config("No OAuth grant types resolved (check server capabilities)".to_string()));
        }
        
        // Validate that we have at least one response type
        if discovered_config.resolved_response_types.is_empty() {
            return Err(ProxyError::config("No OAuth response types resolved (check server capabilities)".to_string()));
        }
        
        // Log validation success
        info!("✅ OAuth configuration validation passed for: {}", config.base_url);
        
        Ok(())
    }

    /// Clear cached OAuth configuration (for testing or manual refresh)
    pub async fn clear_oauth_cache(&self, server_name: Option<&str>) {
        let mut cache = self.discovery_cache.write().await;
        
        if let Some(name) = server_name {
            cache.remove(name);
            info!("🗺 Cleared OAuth cache for server: {}", name);
        } else {
            cache.clear();
            info!("🗺 Cleared all OAuth discovery cache");
        }
    }

    /// Handle incoming OAuth callback and notify waiting authorization flow
    pub async fn handle_oauth_callback(
        &self,
        server_name: &str,
        authorization_code: Option<String>,
        state: Option<String>,
        error: Option<String>,
        error_description: Option<String>,
    ) -> Result<()> {
        info!("📞 Handling OAuth callback for server: {}", server_name);
        
        // Extract the callback sender for this server
        let sender = {
            let mut coordinators = self.callback_coordinators.lock().await;
            coordinators.remove(server_name)
        };
        
        let sender = sender.ok_or_else(|| {
            ProxyError::auth(format!("No OAuth flow waiting for callback from server: {}", server_name))
        })?;
        
        // Create callback data
        let callback_data = OAuthCallbackData {
            authorization_code: authorization_code.unwrap_or_default(),
            state: state.unwrap_or_default(),
            error,
            error_description,
        };
        
        // Send the callback data to the waiting flow
        if let Err(_) = sender.send(callback_data) {
            warn!("Failed to send OAuth callback data for server: {} (receiver dropped)", server_name);
            return Err(ProxyError::auth(format!("OAuth callback receiver dropped for server: {}", server_name)));
        }
        
        info!("✅ OAuth callback handled successfully for server: {}", server_name);
        Ok(())
    }

    /// Register OAuth callback route with server-specific endpoint
    pub fn get_callback_route_pattern() -> &'static str {
        "/auth/callback/{server_name}"
    }

    /// Extract server name from callback URL path
    pub fn extract_server_name_from_callback_path(path: &str) -> Option<String> {
        // Match pattern /auth/callback/{server_name}
        if let Some(captures) = Regex::new(r"^/auth/callback/([^/]+)$")
            .ok()
            .and_then(|re| re.captures(path)) {
            captures.get(1).map(|m| m.as_str().to_string())
        } else {
            None
        }
    }
}