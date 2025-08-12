//! Device Code Flow implementation for OAuth 2.1 (RFC 8628)
//! 
//! This module implements the OAuth 2.1 Device Code Flow for headless environments
//! where browser-based authentication is not available (servers, CLI tools, containers).
//! 
//! The Device Code Flow allows users to authorize an application on a different device
//! with a web browser while the application polls for authorization completion.

use crate::{auth::{config::OAuthProviderConfig, OAuthTokenResponse, TokenStorage}};
use crate::error::ProxyError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration, collections::HashMap};
use tracing::{debug, error, info, warn};
use url::Url;
use secrecy::{Secret, ExposeSecret};
use crate::error::Result;
use actix_web::HttpRequest;

/// Device Code Flow handler for OAuth 2.1 Device Authorization Grant
#[derive(Debug, Clone)]
pub struct DeviceCodeFlow {
    /// OAuth provider configuration
    provider_config: OAuthProviderConfig,
    /// HTTP client for API requests
    client: Client,
    /// Polling interval in seconds (default from provider or 5 seconds)
    polling_interval: Duration,
    /// Maximum polling attempts before giving up (default: 360 = 30 minutes at 5s interval)
    max_polling_attempts: u32,
    /// Token storage for automatic token persistence (optional)
    token_storage: Option<Arc<TokenStorage>>,
}

/// Device authorization request to start the Device Code Flow
#[derive(Debug, Serialize)]
pub struct DeviceAuthorizationRequest {
    /// OAuth client ID
    pub client_id: String,
    /// Requested scopes (space-separated)
    pub scope: String,
    /// Resource indicators (RFC 8707) - audience for the token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
}

/// Device authorization response from the authorization server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAuthorizationResponse {
    /// Device verification code for polling
    pub device_code: String,
    /// Human-readable user code for authorization
    pub user_code: String,
    /// URI where user should go to authorize
    pub verification_uri: String,
    /// Optional complete URI with user code embedded
    pub verification_uri_complete: Option<String>,
    /// Device code expiration time in seconds (typically 1800 = 30 minutes)
    pub expires_in: u64,
    /// Polling interval in seconds (typically 5 seconds)
    pub interval: Option<u64>,
}

/// Token polling request for Device Code Flow
#[derive(Debug, Serialize)]
pub struct TokenPollRequest {
    /// Grant type (always "urn:ietf:params:oauth:grant-type:device_code")
    pub grant_type: String,
    /// Device code from authorization response
    pub device_code: String,
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    pub client_secret: String,
}

/// Result of token polling operation
#[derive(Debug, Clone, PartialEq)]
pub enum TokenPollResult {
    /// Authorization successful - contains access token response
    Success(DeviceTokenResponse),
    /// Authorization still pending - continue polling
    Pending,
    /// Server requests slower polling - increase interval
    SlowDown,
    /// User denied authorization request
    Denied,
    /// Device code has expired
    Expired,
    /// Unknown error occurred
    Error(String),
}

/// Device Code Flow token response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceTokenResponse {
    /// Access token
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Token expiration time in seconds
    pub expires_in: Option<u64>,
    /// Refresh token for getting new access tokens
    pub refresh_token: Option<String>,
    /// Scope of the access token
    pub scope: Option<String>,
    /// Resource indicators (RFC 8707) - audience for the token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<String>>,
    /// Resource indicators (RFC 8707) - resources this token is valid for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<Vec<String>>,
}

impl From<DeviceTokenResponse> for OAuthTokenResponse {
    /// Convert DeviceTokenResponse to standard OAuthTokenResponse for storage
    fn from(device_response: DeviceTokenResponse) -> Self {
        OAuthTokenResponse {
            access_token: Secret::new(device_response.access_token),
            token_type: device_response.token_type,
            expires_in: device_response.expires_in,
            refresh_token: device_response.refresh_token.map(Secret::new),
            scope: device_response.scope,
            audience: device_response.audience,
            resource: device_response.resource,
        }
    }
}

/// Device Code Flow error response from token endpoint
#[derive(Debug, Deserialize)]
pub struct DeviceCodeErrorResponse {
    /// Error code
    pub error: String,
    /// Human-readable error description
    pub error_description: Option<String>,
    /// URI with more information about the error
    pub error_uri: Option<String>,
}

/// Device Code Flow validation result for middleware integration
#[derive(Debug, Clone)]
pub struct DeviceCodeValidationResult {
    /// Device authorization response containing user code and verification URI
    pub device_authorization: DeviceAuthorizationResponse,
    /// Device code for polling
    pub device_code: String,
    /// User information (if available, typically empty for device flow)
    pub user_info: Option<DeviceCodeUserInfo>,
    /// Granted scopes
    pub scopes: Vec<String>,
    /// Additional metadata from the device authorization
    pub metadata: HashMap<String, String>,
    /// Expiration time for the device code
    pub expires_at: u64,
}

/// User information from Device Code Flow (minimal, as device flow typically doesn't provide full user info initially)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeUserInfo {
    /// User identifier (typically empty until token is obtained)
    pub id: String,
    /// User display name
    pub name: Option<String>,
    /// User email address
    pub email: Option<String>,
    /// User login/username
    pub login: Option<String>,
}

/// Device Code Flow validator for middleware integration
#[derive(Debug, Clone)]
pub struct DeviceCodeValidator {
    /// HTTP client for OAuth requests
    client: Arc<Client>,
    /// Device Code Flow configurations mapped by provider name
    configs: HashMap<String, OAuthProviderConfig>,
    /// Token storage for automatic token persistence (optional)
    token_storage: Option<Arc<TokenStorage>>,
}

impl DeviceCodeFlow {
    /// Create a new Device Code Flow handler
    pub fn new(provider_config: OAuthProviderConfig) -> Result<Self> {
        // Validate that device code flow is enabled
        if !provider_config.device_code_enabled {
            return Err(ProxyError::config(
                "Device Code Flow is not enabled for this provider".to_string()
            ));
        }

        // Validate required endpoints
        if provider_config.device_authorization_endpoint.is_none() {
            return Err(ProxyError::config(
                "Device authorization endpoint is required for Device Code Flow".to_string()
            ));
        }

        if provider_config.token_endpoint.is_none() {
            return Err(ProxyError::config(
                "Token endpoint is required for Device Code Flow".to_string()
            ));
        }

        // Validate URLs
        Self::validate_endpoint_urls(&provider_config)?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ProxyError::config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            provider_config,
            client,
            polling_interval: Duration::from_secs(5), // Default 5 seconds
            max_polling_attempts: 360, // 30 minutes at 5-second intervals
            token_storage: None,
        })
    }

    /// Create a Device Code Flow handler with custom polling settings
    pub fn with_polling_config(
        provider_config: OAuthProviderConfig,
        polling_interval: Duration,
        max_polling_attempts: u32,
    ) -> Result<Self> {
        let mut flow = Self::new(provider_config)?;
        flow.polling_interval = polling_interval;
        flow.max_polling_attempts = max_polling_attempts;
        Ok(flow)
    }

    /// Create a Device Code Flow handler with token storage support
    pub async fn with_token_storage(
        provider_config: OAuthProviderConfig, 
        token_storage: Arc<TokenStorage>
    ) -> Result<Self> {
        let mut flow = Self::new(provider_config)?;
        flow.token_storage = Some(token_storage);
        Ok(flow)
    }

    /// Create a Device Code Flow handler with both custom polling and token storage
    pub async fn with_polling_and_storage(
        provider_config: OAuthProviderConfig,
        polling_interval: Duration,
        max_polling_attempts: u32,
        token_storage: Arc<TokenStorage>,
    ) -> Result<Self> {
        let mut flow = Self::with_polling_config(provider_config, polling_interval, max_polling_attempts)?;
        flow.token_storage = Some(token_storage);
        Ok(flow)
    }

    /// Initiate device authorization flow
    /// 
    /// Step 1 of Device Code Flow:
    /// 1. POST to device authorization endpoint
    /// 2. Get device code and user code
    /// 3. Return authorization response with user instructions
    pub async fn initiate_device_authorization(
        &self,
        scopes: &[String],
    ) -> Result<DeviceAuthorizationResponse> {
        let device_auth_endpoint = self.provider_config
            .device_authorization_endpoint
            .as_ref()
            .unwrap(); // Already validated in constructor

        debug!(
            "Initiating device authorization flow with provider: {} for scopes: {:?}",
            self.provider_config.client_id, scopes
        );

        let request = DeviceAuthorizationRequest {
            client_id: self.provider_config.client_id.clone(),
            scope: scopes.join(" "),
            audience: None, // TODO: Add Resource Indicators support
        };

        let response = self
            .client
            .post(device_auth_endpoint)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", format!("{}/{} OAuth2.1-DeviceCode", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
            .form(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send device authorization request: {}", e);
                ProxyError::auth("Failed to initiate device authorization")
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Device authorization request failed with status {}: {}", status, error_text);
            return Err(ProxyError::auth("Device authorization request failed"));
        }

        let auth_response: DeviceAuthorizationResponse = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse device authorization response: {}", e);
                ProxyError::auth("Invalid device authorization response")
            })?;

        // Update polling interval from response if provided
        if let Some(interval) = auth_response.interval {
            self.update_polling_interval(Duration::from_secs(interval));
        }

        info!(
            "Device authorization initiated successfully. User code: {}, expires in: {}s",
            auth_response.user_code, auth_response.expires_in
        );

        Ok(auth_response)
    }

    /// Poll for token using device code
    /// 
    /// Step 2 of Device Code Flow:
    /// 1. POST to token endpoint with device code
    /// 2. Handle various response types (pending, slow_down, success, errors)
    /// 3. Return appropriate poll result
    pub async fn poll_for_token(&self, device_code: &str) -> Result<TokenPollResult> {
        let token_endpoint = self.provider_config
            .token_endpoint
            .as_ref()
            .unwrap(); // Already validated in constructor

        debug!("Polling for token with device code");

        let request = TokenPollRequest {
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
            device_code: device_code.to_string(),
            client_id: self.provider_config.client_id.clone(),
            client_secret: self.provider_config.client_secret.expose_secret().clone(),
        };

        let response = self
            .client
            .post(token_endpoint)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", format!("{}/{} OAuth2.1-DeviceCode", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
            .form(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to poll for token: {}", e);
                ProxyError::auth("Failed to poll for device token")
            })?;

        let status = response.status();
        
        if status.is_success() {
            // Success - parse token response
            let token_response: DeviceTokenResponse = response
                .json()
                .await
                .map_err(|e| {
                    error!("Failed to parse successful token response: {}", e);
                    ProxyError::auth("Invalid token response")
                })?;

            info!("Device authorization completed successfully");
            return Ok(TokenPollResult::Success(token_response));
        }

        // Handle error responses  
        if status == 400 {
            match response.json::<DeviceCodeErrorResponse>().await {
                Ok(error) => {
                    match error.error.as_str() {
                        "authorization_pending" => {
                            debug!("Authorization still pending - continuing to poll");
                            return Ok(TokenPollResult::Pending);
                        }
                        "slow_down" => {
                            debug!("Server requests slower polling");
                            return Ok(TokenPollResult::SlowDown);
                        }
                        "access_denied" => {
                            info!("User denied authorization request");
                            return Ok(TokenPollResult::Denied);
                        }
                        "expired_token" => {
                            info!("Device code has expired");
                            return Ok(TokenPollResult::Expired);
                        }
                        _ => {
                            error!("Unknown device code error: {}", error.error);
                            return Ok(TokenPollResult::Error(format!(
                                "Unknown error: {} - {}", 
                                error.error, 
                                error.error_description.unwrap_or_default()
                            )));
                        }
                    }
                }
                Err(_) => {
                    // Failed to parse error response - return generic error
                    error!("Failed to parse error response for status 400");
                    return Ok(TokenPollResult::Error("Bad Request - Failed to parse error details".to_string()));
                }
            }
        } else {
            // Unknown error - try to get response text
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Unexpected token polling error (status {}): {}", status, error_text);
            return Ok(TokenPollResult::Error(format!("HTTP {}: {}", status, error_text)));
        }
    }

    /// Complete the device code flow with automatic polling
    /// 
    /// This method handles the complete flow:
    /// 1. Polls for token at configured intervals
    /// 2. Handles rate limiting (slow_down responses)
    /// 3. Returns final result (success, denied, expired, or error)
    pub async fn complete_device_flow(&self, device_code: &str) -> Result<DeviceTokenResponse> {
        let mut attempts = 0;
        let mut current_interval = self.polling_interval;

        info!(
            "Starting device code polling (max attempts: {}, interval: {}s)",
            self.max_polling_attempts,
            current_interval.as_secs()
        );

        loop {
            if attempts >= self.max_polling_attempts {
                error!("Device code polling timeout after {} attempts", attempts);
                return Err(ProxyError::auth("Device code flow timeout - maximum polling attempts reached"));
            }

            // Wait before polling (except first attempt)
            if attempts > 0 {
                tokio::time::sleep(current_interval).await;
            }

            match self.poll_for_token(device_code).await? {
                TokenPollResult::Success(token) => {
                    info!("Device code flow completed successfully after {} attempts", attempts + 1);
                    
                    // Automatically store token if storage is available
                    if let Some(ref token_storage) = self.token_storage {
                        let oauth_token: OAuthTokenResponse = token.clone().into();
                        let provider = "device_code"; // Use a default provider name for device code flow
                        
                        // For device code flow, we typically don't have user info immediately
                        // The token might need to be used to get user info first
                        match token_storage.store_oauth_token(provider, None, &oauth_token).await {
                            Ok(key) => {
                                info!("Device code token automatically stored with key: {}", key);
                            },
                            Err(e) => {
                                warn!("Failed to store device code token: {}", e);
                                // Don't fail the whole operation if storage fails
                            }
                        }
                    }
                    
                    return Ok(token);
                }
                TokenPollResult::Pending => {
                    attempts += 1;
                    debug!("Authorization pending (attempt {})", attempts);
                    continue;
                }
                TokenPollResult::SlowDown => {
                    // Increase polling interval by 5 seconds as per RFC 8628
                    current_interval += Duration::from_secs(5);
                    attempts += 1;
                    debug!("Server requested slow down - new interval: {}s (attempt {})", current_interval.as_secs(), attempts);
                    continue;
                }
                TokenPollResult::Denied => {
                    error!("User denied device authorization");
                    return Err(ProxyError::auth("User denied device authorization"));
                }
                TokenPollResult::Expired => {
                    error!("Device code expired during polling");
                    return Err(ProxyError::auth("Device code expired"));
                }
                TokenPollResult::Error(error_msg) => {
                    error!("Device code polling error: {}", error_msg);
                    return Err(ProxyError::auth(&format!("Device code polling error: {}", error_msg)));
                }
            }
        }
    }

    /// Get user-friendly instructions for device authorization
    pub fn get_user_instructions(&self, auth_response: &DeviceAuthorizationResponse) -> String {
        let verification_uri = if let Some(ref complete_uri) = auth_response.verification_uri_complete {
            format!("Go to: {}", complete_uri)
        } else {
            format!("Go to: {} and enter code: {}", 
                   auth_response.verification_uri, 
                   auth_response.user_code)
        };

        format!(
            "🔐 Device Authorization Required\n\n\
             {} \n\n\
             Code expires in {} minutes.\n\
             MagicTunnel will automatically continue once you complete authorization.",
            verification_uri,
            auth_response.expires_in / 60
        )
    }

    /// Create MCP error response for device authorization
    pub fn create_mcp_error_response(
        &self,
        provider: &str,
        auth_response: &DeviceAuthorizationResponse,
    ) -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32001,
                "message": "Device authorization required",
                "data": {
                    "auth_type": "device_code",
                    "provider": provider,
                    "user_code": auth_response.user_code,
                    "verification_uri": auth_response.verification_uri,
                    "verification_uri_complete": auth_response.verification_uri_complete,
                    "expires_in": auth_response.expires_in,
                    "interval": auth_response.interval.unwrap_or(5),
                    "instructions": self.get_user_instructions(auth_response)
                }
            }
        })
    }

    /// Validate OAuth endpoint URLs
    fn validate_endpoint_urls(config: &OAuthProviderConfig) -> Result<()> {
        if let Some(ref url) = config.device_authorization_endpoint {
            Url::parse(url).map_err(|_| {
                ProxyError::config(format!("Invalid device_authorization_endpoint URL: {}", url))
            })?;
        }

        if let Some(ref url) = config.token_endpoint {
            Url::parse(url).map_err(|_| {
                ProxyError::config(format!("Invalid token_endpoint URL: {}", url))
            })?;
        }

        Ok(())
    }

    /// Update polling interval (used when server requests slower polling)
    fn update_polling_interval(&self, new_interval: Duration) {
        // Note: This is a read-only update for logging purposes
        // In a real implementation, you might want to make polling_interval mutable
        debug!("Server recommended polling interval: {}s", new_interval.as_secs());
    }

    /// Get provider configuration
    pub fn provider_config(&self) -> &OAuthProviderConfig {
        &self.provider_config
    }

    /// Get current polling configuration
    pub fn polling_config(&self) -> (Duration, u32) {
        (self.polling_interval, self.max_polling_attempts)
    }

    /// Store a device token manually (requires token storage)
    pub async fn store_device_token(&self, token: &DeviceTokenResponse, user_id: Option<&str>) -> Result<Option<String>> {
        match &self.token_storage {
            Some(storage) => {
                let oauth_token: OAuthTokenResponse = token.clone().into();
                let provider = "device_code";
                
                match storage.store_oauth_token(provider, user_id, &oauth_token).await {
                    Ok(key) => {
                        info!("Device token manually stored with key: {}", key);
                        Ok(Some(key))
                    },
                    Err(e) => Err(e),
                }
            },
            None => {
                warn!("Token storage not available for device token storage");
                Ok(None)
            }
        }
    }

    /// Retrieve stored device token (requires token storage)
    pub async fn retrieve_stored_token(&self, user_id: Option<&str>) -> Result<Option<crate::auth::TokenData>> {
        match &self.token_storage {
            Some(storage) => {
                let provider = "device_code";
                storage.retrieve_oauth_token(provider, user_id).await
            },
            None => {
                warn!("Token storage not available for device token retrieval");
                Ok(None)
            }
        }
    }

    /// Delete stored device token (requires token storage)
    pub async fn delete_stored_token(&self, user_id: Option<&str>) -> Result<bool> {
        match &self.token_storage {
            Some(storage) => {
                let provider = "device_code";
                match storage.delete_oauth_token(provider, user_id).await {
                    Ok(()) => {
                        info!("Device token deleted for provider: {}, user: {:?}", provider, user_id);
                        Ok(true)
                    },
                    Err(e) => {
                        warn!("Failed to delete device token: {}", e);
                        Err(e)
                    }
                }
            },
            None => {
                warn!("Token storage not available for device token deletion");
                Ok(false)
            }
        }
    }

    /// Check if token storage is available
    pub fn has_token_storage(&self) -> bool {
        self.token_storage.is_some()
    }
}

impl DeviceCodeValidator {
    /// Create a new Device Code Flow validator
    pub fn new(configs: HashMap<String, OAuthProviderConfig>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client: Arc::new(client),
            configs,
            token_storage: None,
        }
    }

    /// Create a Device Code Flow validator with token storage
    pub fn with_token_storage(
        configs: HashMap<String, OAuthProviderConfig>,
        token_storage: Arc<TokenStorage>,
    ) -> Self {
        let mut validator = Self::new(configs);
        validator.token_storage = Some(token_storage);
        validator
    }

    /// Check if Device Code Flow validation is enabled
    pub fn is_enabled(&self) -> bool {
        !self.configs.is_empty() && self.configs.values().any(|config| config.device_code_enabled)
    }

    /// Validate HTTP request for Device Code Flow authentication
    /// 
    /// Device Code Flow uses different headers than standard OAuth:
    /// - X-Device-Code-Provider: Provider name for device authorization
    /// - X-Device-Code-Scopes: Requested scopes (comma-separated)
    /// 
    /// Or Authorization header format:
    /// - Authorization: DeviceCode <provider>:<scopes>
    pub async fn validate_request(&self, req: &HttpRequest) -> Result<Option<DeviceCodeValidationResult>> {
        if !self.is_enabled() {
            debug!("Device Code Flow validation disabled");
            return Ok(None);
        }

        // Try to extract device code request from headers
        let device_request = match self.extract_device_code_request(req) {
            Ok(Some(request)) => request,
            Ok(None) => {
                debug!("No Device Code Flow credentials found in request");
                return Ok(None);
            }
            Err(e) => {
                warn!("Failed to extract Device Code Flow request: {}", e);
                return Ok(None);
            }
        };

        // Get provider configuration
        let provider_config = self.configs.get(&device_request.provider)
            .ok_or_else(|| ProxyError::auth(format!("Device Code provider '{}' not configured", device_request.provider)))?;

        // Validate that device code flow is enabled for this provider
        if !provider_config.device_code_enabled {
            warn!("Device Code Flow not enabled for provider: {}", device_request.provider);
            return Err(ProxyError::auth(format!("Device Code Flow not enabled for provider: {}", device_request.provider)));
        }

        // Create DeviceCodeFlow instance and initiate authorization
        let flow = DeviceCodeFlow::new(provider_config.clone())?;
        
        debug!(
            "Initiating Device Code Flow for provider: {} with scopes: {:?}",
            device_request.provider, device_request.scopes
        );

        // Initiate device authorization
        let auth_response = flow.initiate_device_authorization(&device_request.scopes).await?;

        // Create validation result
        let validation_result = DeviceCodeValidationResult {
            device_code: auth_response.device_code.clone(),
            device_authorization: auth_response.clone(),
            user_info: None, // Device flow typically doesn't have user info initially
            scopes: device_request.scopes.clone(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("provider".to_string(), device_request.provider.clone());
                metadata.insert("verification_uri".to_string(), auth_response.verification_uri.clone());
                metadata.insert("user_code".to_string(), auth_response.user_code.clone());
                metadata.insert("expires_in".to_string(), auth_response.expires_in.to_string());
                metadata
            },
            expires_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() + auth_response.expires_in,
        };

        info!(
            "Device Code Flow initiated successfully for provider: {}, user code: {}, expires in: {}s",
            device_request.provider, auth_response.user_code, auth_response.expires_in
        );

        Ok(Some(validation_result))
    }

    /// Check if a device code validation result has a specific permission
    pub fn check_permission(&self, result: &DeviceCodeValidationResult, permission: &str) -> bool {
        result.scopes.contains(&permission.to_string())
    }

    /// Extract device code request from HTTP headers
    fn extract_device_code_request(&self, req: &HttpRequest) -> Result<Option<DeviceCodeRequest>> {
        // Try X-Device-Code-Provider and X-Device-Code-Scopes headers first
        if let (Some(provider_header), Some(scopes_header)) = (
            req.headers().get("X-Device-Code-Provider"),
            req.headers().get("X-Device-Code-Scopes")
        ) {
            let provider = provider_header.to_str()
                .map_err(|_| ProxyError::auth("Invalid X-Device-Code-Provider header encoding"))?
                .to_string();

            let scopes_str = scopes_header.to_str()
                .map_err(|_| ProxyError::auth("Invalid X-Device-Code-Scopes header encoding"))?;

            let scopes = scopes_str.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            return Ok(Some(DeviceCodeRequest { provider, scopes }));
        }

        // Try Authorization header with DeviceCode scheme
        if let Some(auth_header) = req.headers().get("Authorization") {
            let auth_str = auth_header.to_str()
                .map_err(|_| ProxyError::auth("Invalid Authorization header encoding"))?;

            if auth_str.starts_with("DeviceCode ") {
                let auth_data = auth_str.strip_prefix("DeviceCode ").unwrap();
                
                // Parse format: "provider:scope1,scope2,scope3"
                if let Some((provider, scopes_str)) = auth_data.split_once(':') {
                    let scopes = scopes_str.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    return Ok(Some(DeviceCodeRequest {
                        provider: provider.to_string(),
                        scopes,
                    }));
                } else {
                    // If no scopes specified, use provider with default scopes
                    return Ok(Some(DeviceCodeRequest {
                        provider: auth_data.to_string(),
                        scopes: vec![], // Will use provider's default scopes
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Get the configured providers
    pub fn get_providers(&self) -> Vec<&String> {
        self.configs.keys().collect()
    }

    /// Check if a specific provider is configured
    pub fn has_provider(&self, provider: &str) -> bool {
        self.configs.contains_key(provider)
    }
}

/// Device Code Flow request extracted from HTTP headers
#[derive(Debug, Clone)]
struct DeviceCodeRequest {
    /// OAuth provider name
    provider: String,
    /// Requested scopes
    scopes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::config::OAuthProviderConfig;

    fn create_test_provider_config() -> OAuthProviderConfig {
        OAuthProviderConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string().into(),
            scopes: vec!["user:email".to_string()],
            oauth_enabled: false,
            device_code_enabled: true,
            authorization_endpoint: None,
            device_authorization_endpoint: Some("https://github.com/login/device/code".to_string()),
            token_endpoint: Some("https://github.com/login/oauth/access_token".to_string()),
            user_info_endpoint: Some("https://api.github.com/user".to_string()),
            resource_indicators: None,
            extra_params: None,
        }
    }

    #[test]
    fn test_device_code_flow_creation() {
        let config = create_test_provider_config();
        let flow = DeviceCodeFlow::new(config);
        assert!(flow.is_ok());
    }

    #[test]
    fn test_device_code_flow_requires_enabled() {
        let mut config = create_test_provider_config();
        config.device_code_enabled = false;
        
        let flow = DeviceCodeFlow::new(config);
        assert!(flow.is_err());
        assert!(flow.unwrap_err().to_string().contains("Device Code Flow is not enabled"));
    }

    #[test]
    fn test_device_code_flow_requires_endpoints() {
        let mut config = create_test_provider_config();
        config.device_authorization_endpoint = None;
        
        let flow = DeviceCodeFlow::new(config);
        assert!(flow.is_err());
        assert!(flow.unwrap_err().to_string().contains("Device authorization endpoint is required"));
    }

    #[test]
    fn test_invalid_endpoint_urls() {
        let mut config = create_test_provider_config();
        config.device_authorization_endpoint = Some("invalid-url".to_string());
        
        let flow = DeviceCodeFlow::new(config);
        assert!(flow.is_err());
        assert!(flow.unwrap_err().to_string().contains("Invalid device_authorization_endpoint URL"));
    }

    #[test]
    fn test_custom_polling_config() {
        let config = create_test_provider_config();
        let flow = DeviceCodeFlow::with_polling_config(
            config,
            Duration::from_secs(10),
            100,
        );
        
        assert!(flow.is_ok());
        let flow = flow.unwrap();
        let (interval, max_attempts) = flow.polling_config();
        assert_eq!(interval, Duration::from_secs(10));
        assert_eq!(max_attempts, 100);
    }

    #[test]
    fn test_user_instructions() {
        let config = create_test_provider_config();
        let flow = DeviceCodeFlow::new(config).unwrap();
        
        let auth_response = DeviceAuthorizationResponse {
            device_code: "test_device_code".to_string(),
            user_code: "ABCD-EFGH".to_string(),
            verification_uri: "https://github.com/login/device".to_string(),
            verification_uri_complete: Some("https://github.com/login/device?user_code=ABCD-EFGH".to_string()),
            expires_in: 1800,
            interval: Some(5),
        };
        
        let instructions = flow.get_user_instructions(&auth_response);
        assert!(instructions.contains("Device Authorization Required"));
        assert!(instructions.contains("https://github.com/login/device?user_code=ABCD-EFGH"));
        assert!(instructions.contains("30 minutes")); // 1800 seconds / 60
    }

    #[test]
    fn test_mcp_error_response() {
        let config = create_test_provider_config();
        let flow = DeviceCodeFlow::new(config).unwrap();
        
        let auth_response = DeviceAuthorizationResponse {
            device_code: "test_device_code".to_string(),
            user_code: "ABCD-EFGH".to_string(),
            verification_uri: "https://github.com/login/device".to_string(),
            verification_uri_complete: None,
            expires_in: 1800,
            interval: Some(5),
        };
        
        let error_response = flow.create_mcp_error_response("github", &auth_response);
        
        assert_eq!(error_response["jsonrpc"], "2.0");
        assert_eq!(error_response["error"]["code"], -32001);
        assert_eq!(error_response["error"]["message"], "Device authorization required");
        assert_eq!(error_response["error"]["data"]["auth_type"], "device_code");
        assert_eq!(error_response["error"]["data"]["provider"], "github");
        assert_eq!(error_response["error"]["data"]["user_code"], "ABCD-EFGH");
    }
}