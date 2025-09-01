# MCP Client Authentication Integration

## Overview

This document provides comprehensive documentation for MagicTunnel's **MCP Client Authentication Integration** system, which enables authenticated tool calls through Smart Discovery and external MCP proxying. This system allows OAuth tokens, API keys, and other authentication contexts to flow seamlessly from MCP clients through the entire request pipeline to external APIs.

## Architecture Overview

The authentication integration follows a multi-layered approach that preserves authentication context throughout the entire request lifecycle:

```
MCP Client Request → MCP Server → Smart Discovery → Agent Router → External MCP Client → External API
       ↓                ↓              ↓               ↓                ↓                ↓
  Auth Headers    Auth Context   Auth Context    Route w/ Auth    Auth Headers    API Call w/ Auth
```

## Core Components

### 1. Authentication Context (`AuthenticationContext`)

The `AuthenticationContext` is the central data structure that carries authentication information throughout the system:

```rust
pub struct AuthenticationContext {
    pub user_id: String,
    pub session_id: String,
    pub provider_tokens: HashMap<String, ProviderToken>,
    pub auth_method: String,
    pub created_at: SystemTime,
    pub last_used: SystemTime,
}
```

**Key Features:**
- **Multi-Provider Support**: Supports OAuth, API keys, JWT tokens, service accounts
- **Token Management**: Automatic expiration detection and refresh handling
- **Session Persistence**: Cross-platform secure storage with automatic recovery
- **Header Generation**: Converts auth context to HTTP headers for external API calls

### 2. Smart Discovery Authentication Integration

Smart Discovery now supports authentication context propagation:

```rust
pub struct SmartDiscoveryRequest {
    pub request: String,
    pub context: Option<String>,
    pub preferred_tools: Option<Vec<String>>,
    pub confidence_threshold: Option<f32>,
    pub include_error_details: Option<bool>,
    pub sequential_mode: Option<bool>,
    pub auth_context: Option<Arc<AuthenticationContext>>, // 🆕 Auth context field
}
```

**Integration Points:**
- MCP Server extracts auth context from incoming requests
- Smart Discovery Service receives auth context with requests
- Agent Router propagates auth context to tool execution
- External MCP clients apply auth headers to outbound requests

### 3. External MCP Client Authentication

All external MCP clients now support authentication:

#### HTTP Client Authentication
```rust
// New methods for authenticated requests
pub async fn call_tool_with_auth(
    &self,
    tool_call: ToolCall,
    auth_context: Option<&AuthenticationContext>
) -> Result<ToolResult>

pub async fn send_request_with_auth(
    &self,
    method: &str,
    params: Option<Value>,
    auth_context: Option<&AuthenticationContext>
) -> Result<Value>
```

#### WebSocket Client Authentication
```rust
// Authentication during handshake
pub async fn new_with_auth(
    url: String,
    auth_context: Option<AuthenticationContext>
) -> Result<Self>

// Runtime auth context updates
pub fn set_authentication_context(&mut self, auth_context: AuthenticationContext)
```

#### SSE Client Authentication
```rust
// Auth-enabled SSE client
pub async fn new_with_auth(
    url: String,
    auth_context: Option<AuthenticationContext>
) -> Result<Self>

// Tool calls with authentication
pub async fn call_tool_with_auth(
    &self,
    tool_call: ToolCall,
    auth_context: Option<&AuthenticationContext>
) -> Result<ToolResult>
```

## Authentication Flow

### Complete Request Flow

1. **MCP Client Request**
   - Client sends authenticated request to MCP server
   - Headers include: `Authorization`, `mcp-session-id`, `mcp-user-id`

2. **MCP Server Processing**
   - Server extracts authentication context from headers
   - Validates session and creates `AuthenticationContext`
   - Calls `call_tool_with_auth()` instead of `call_tool()`

3. **Smart Discovery Integration**
   - MCP server creates `SmartDiscoveryRequest` with auth context
   - Smart Discovery service receives request with authentication
   - Service uses `route_with_auth()` for authenticated tool routing

4. **Agent Router Processing**
   - Router receives auth context from Smart Discovery
   - Selects appropriate tool and execution method
   - Calls `execute_with_auth()` on selected agent

5. **External MCP Client Execution**
   - External manager receives auth context
   - HTTP/WebSocket/SSE client applies authentication headers
   - Headers generated: `Authorization: Bearer <token>`, `X-Session-ID`, `X-User-ID`

6. **External API Call**
   - External API receives authenticated request
   - API validates token and processes request
   - Response flows back through the chain

### Authentication Context Propagation

```mermaid
graph TD
    A[MCP Client] -->|Auth Headers| B[MCP Server]
    B -->|AuthenticationContext| C[Smart Discovery]
    C -->|AuthenticationContext| D[Agent Router]
    D -->|AuthenticationContext| E[External MCP Manager]
    E -->|Auth Headers| F[External API]
    
    B --> B1[extract_auth_context]
    C --> C1[route_with_auth]
    D --> D1[execute_with_auth]
    E --> E1[add_authentication_headers]
```

## Supported Authentication Methods

### 1. OAuth 2.1 with PKCE

**Setup:**
```rust
let user_info = OAuthUserInfo {
    id: "github_user_123".to_string(),
    email: Some("user@example.com".to_string()),
    name: Some("GitHub User".to_string()),
    login: Some("github_user".to_string()),
};

let oauth_result = OAuthValidationResult {
    user_info,
    expires_at: Some(expires_timestamp),
    scopes: vec!["repo".to_string(), "user:email".to_string()],
    access_token: Some("github_token_abc123".to_string()),
    refresh_token: Some("github_refresh_def456".to_string()),
    issuer: Some("https://github.com".to_string()),
};

let auth_context = AuthenticationContext::from_auth_result(
    &AuthenticationResult::OAuth(oauth_result), 
    session_id
)?;
```

**Generated Headers:**
```
Authorization: Bearer github_token_abc123
X-Session-ID: user_session_123
X-User-ID: github_user_123
X-Auth-Method: oauth2.1
X-Token-Scope: repo,user:email
```

### 2. API Key Authentication

**Setup:**
```rust
let api_key_result = AuthenticationResult::ApiKey {
    key: "api_key_xyz789".to_string(),
    user_id: "api_user_456".to_string(),
    scopes: vec!["read".to_string(), "write".to_string()],
    expires_at: Some(expires_timestamp),
};

let auth_context = AuthenticationContext::from_auth_result(&api_key_result, session_id)?;
```

**Generated Headers:**
```
Authorization: Bearer api_key_xyz789
X-Session-ID: api_session_456
X-User-ID: api_user_456
X-Auth-Method: api_key
```

### 3. JWT Token Authentication

**Setup:**
```rust
let jwt_result = AuthenticationResult::JWT {
    token: "jwt_token_abc123".to_string(),
    claims: jwt_claims,
    expires_at: Some(expires_timestamp),
};

let auth_context = AuthenticationContext::from_auth_result(&jwt_result, session_id)?;
```

### 4. Service Account Authentication

**Setup:**
```rust
let service_result = AuthenticationResult::ServiceAccount {
    service_id: "service_789".to_string(),
    credentials: service_credentials,
    scopes: vec!["admin".to_string()],
};

let auth_context = AuthenticationContext::from_auth_result(&service_result, session_id)?;
```

## Authentication Header Generation

### Provider-Specific Headers

The authentication context generates different headers based on the provider and authentication method:

```rust
impl AuthenticationContext {
    pub fn get_auth_headers(&self, provider: Option<&str>) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        
        // Base headers (always included)
        headers.insert("X-Session-ID".to_string(), self.session_id.clone());
        headers.insert("X-User-ID".to_string(), self.user_id.clone());
        headers.insert("X-Auth-Method".to_string(), self.auth_method.clone());
        
        // Provider-specific token
        if let Some(token) = self.get_provider_token(provider.unwrap_or("oauth")) {
            headers.insert("Authorization".to_string(), 
                          format!("Bearer {}", token.access_token));
            
            if !token.scopes.is_empty() {
                headers.insert("X-Token-Scope".to_string(), token.scopes.join(","));
            }
            
            if let Some(expires_at) = token.expires_at {
                headers.insert("X-Token-Expires".to_string(), expires_at.to_string());
            }
        }
        
        headers
    }
}
```

### External API Integration

Different external APIs require different authentication patterns:

#### GitHub API
```rust
// Headers for GitHub API calls
Authorization: Bearer github_token_abc123
Accept: application/vnd.github.v3+json
User-Agent: MagicTunnel/0.3.22
X-Session-ID: github_session_123
```

#### Google Drive API
```rust
// Headers for Google Drive API calls
Authorization: Bearer google_token_def456
Accept: application/json
X-Session-ID: google_session_456
X-Goog-Request-Reason: automated-tool-access
```

#### Custom API Integration
```rust
// Generic API authentication
Authorization: Bearer custom_api_key_789
X-Session-ID: custom_session_789
X-User-ID: custom_user_123
X-API-Version: v1
```

## Error Handling and Recovery

### Token Expiration Detection

```rust
impl ProviderToken {
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            expires_at <= now
        } else {
            false // No expiration set
        }
    }
}
```

### Automatic Token Refresh

The system supports automatic token refresh for OAuth providers:

```rust
pub async fn refresh_token_if_needed(
    &mut self,
    provider: &str
) -> Result<bool> {
    if let Some(token) = self.provider_tokens.get_mut(provider) {
        if token.is_expired() {
            if let Some(refresh_token) = &token.refresh_token {
                // Perform token refresh
                let new_token = oauth_client.refresh_token(refresh_token).await?;
                token.access_token = new_token.access_token;
                token.expires_at = new_token.expires_at;
                return Ok(true);
            }
        }
    }
    Ok(false)
}
```

### Error Recovery Strategies

1. **Token Refresh**: Automatic refresh for expired OAuth tokens
2. **Graceful Degradation**: Non-auth tools continue to work when auth fails
3. **Error Propagation**: Clear error messages for authentication failures
4. **Session Recovery**: Automatic session restoration on service restart

### Common Error Scenarios

#### Expired Token
```rust
// Error response
{
    "error": "authentication_failed",
    "message": "OAuth token has expired",
    "details": {
        "provider": "github",
        "expires_at": 1640995200,
        "current_time": 1640998800
    },
    "recovery": "token_refresh_required"
}
```

#### Invalid Token
```rust
// Error response
{
    "error": "invalid_token",
    "message": "The provided token is invalid or malformed",
    "details": {
        "token_type": "bearer",
        "validation_error": "signature_verification_failed"
    },
    "recovery": "re_authentication_required"
}
```

#### Missing Authentication
```rust
// Error response
{
    "error": "authentication_required",
    "message": "This tool requires authentication",
    "details": {
        "tool": "github_create_repo",
        "required_scopes": ["repo"],
        "available_providers": ["oauth", "api_key"]
    },
    "recovery": "provide_authentication"
}
```

## Session Management

### Cross-Platform Session Storage

The authentication system provides secure session persistence across different platforms:

#### macOS (Keychain)
```rust
use security_framework::passwords::*;

pub async fn store_session_keychain(
    session_id: &str,
    auth_data: &AuthenticationContext
) -> Result<()> {
    let service = "magictunnel-auth";
    let account = session_id;
    let password = serde_json::to_string(auth_data)?;
    
    set_generic_password(service, account, password.as_bytes())?;
    Ok(())
}
```

#### Windows (Credential Manager)
```rust
use windows_sys::Win32::Security::Credentials::*;

pub async fn store_session_windows(
    session_id: &str,
    auth_data: &AuthenticationContext
) -> Result<()> {
    let target_name = format!("magictunnel-auth:{}", session_id);
    let credential_blob = serde_json::to_string(auth_data)?;
    
    // Store in Windows Credential Manager
    unsafe {
        CredWriteW(&credential, 0);
    }
    Ok(())
}
```

#### Linux (Secret Service)
```rust
use secret_service::*;

pub async fn store_session_linux(
    session_id: &str,
    auth_data: &AuthenticationContext
) -> Result<()> {
    let ss = SecretService::new(EncryptionType::Dh)?;
    let collection = ss.get_default_collection()?;
    
    let attributes = vec![
        ("service", "magictunnel-auth"),
        ("session_id", session_id),
    ];
    
    let credential_data = serde_json::to_string(auth_data)?;
    collection.create_item(
        "MagicTunnel Session",
        attributes,
        credential_data.as_bytes(),
        true, // replace
        "text/plain"
    )?;
    
    Ok(())
}
```

#### Filesystem Fallback
```rust
pub async fn store_session_filesystem(
    session_id: &str,
    auth_data: &AuthenticationContext
) -> Result<()> {
    let session_dir = get_secure_session_directory()?;
    let session_file = session_dir.join(format!("{}.json", session_id));
    
    // Create encrypted session data
    let encrypted_data = encrypt_session_data(auth_data)?;
    
    // Write with secure permissions (0600)
    let mut file = tokio::fs::File::create(&session_file).await?;
    file.write_all(&encrypted_data).await?;
    
    // Set secure file permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata().await?.permissions();
        perms.set_mode(0o600); // Read/write for owner only
        file.set_permissions(perms).await?;
    }
    
    Ok(())
}
```

### Session Recovery

Automatic session recovery ensures authentication context persists across service restarts:

```rust
pub async fn recover_authentication_context(
    session_id: &str
) -> Result<Option<AuthenticationContext>> {
    // Try platform-specific storage first
    if let Ok(Some(context)) = try_platform_storage(session_id).await {
        return Ok(Some(context));
    }
    
    // Fallback to filesystem storage
    if let Ok(Some(context)) = try_filesystem_storage(session_id).await {
        return Ok(Some(context));
    }
    
    // No session found
    Ok(None)
}

pub async fn try_platform_storage(session_id: &str) -> Result<Option<AuthenticationContext>> {
    #[cfg(target_os = "macos")]
    return try_keychain_storage(session_id).await;
    
    #[cfg(target_os = "windows")]
    return try_credential_manager_storage(session_id).await;
    
    #[cfg(target_os = "linux")]
    return try_secret_service_storage(session_id).await;
    
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return Ok(None);
}
```

## Testing and Validation

### Test Coverage

The authentication integration includes comprehensive test coverage:

#### Unit Tests
- `AuthenticationContext` creation and validation
- Token expiration detection and refresh logic
- Header generation for different providers
- Session serialization and deserialization

#### Integration Tests (`tests/auth_injection_integration_test.rs`)
- **9 comprehensive tests** covering all Phase 3.1 requirements:
  1. `test_http_agent_oauth_token_injection` - OAuth token propagation
  2. `test_multi_provider_oauth_token_injection` - Multiple providers
  3. `test_authentication_injection_failure_cases` - Error handling
  4. `test_router_level_authentication_propagation` - Router integration
  5. `test_session_recovery_with_tool_execution` - Session persistence
  6. `test_smart_discovery_auth_propagation` - **Smart Discovery integration**
  7. `test_mixed_auth_and_non_auth_tool_execution` - **Mixed auth scenarios**
  8. `test_oauth_token_refresh_scenarios` - **Token refresh flows**
  9. `test_end_to_end_authentication_pipeline` - **Complete pipeline**

#### Mock External APIs
The tests include mock servers for testing external API integration:

```rust
struct MockExternalApiServer {
    server: MockServer,
}

impl MockExternalApiServer {
    async fn setup_github_api_mock(&self) {
        // Mock GitHub API with Bearer token validation
        Mock::given(method("GET"))
            .and(path("/user"))
            .and(header("Authorization", "Bearer github_test_token_123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "login": "test_user",
                "id": 12345,
                "name": "Test User"
            })))
            .mount(&self.server)
            .await;
    }
    
    async fn setup_google_api_mock(&self) {
        // Mock Google Drive API with Bearer token validation
        Mock::given(method("GET"))
            .and(path("/drive/v3/about"))
            .and(header("Authorization", "Bearer google_test_token_456"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "user": {"displayName": "Test User"},
                "storageQuota": {"limit": "17179869184"}
            })))
            .mount(&self.server)
            .await;
    }
}
```

### Manual Testing

#### Smart Discovery Authentication Test
```bash
# Test Smart Discovery with OAuth authentication
curl -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer github_token_123" \
  -H "mcp-session-id: test_session_456" \
  -H "mcp-user-id: github_user_789" \
  -d '{
    "name": "smart_tool_discovery",
    "arguments": {
      "request": "check my GitHub repositories",
      "context": "User wants to see their GitHub repos",
      "confidence_threshold": 0.8
    }
  }'
```

#### Direct Tool Authentication Test
```bash
# Test direct tool call with authentication
curl -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer github_token_123" \
  -H "mcp-session-id: test_session_456" \
  -d '{
    "name": "github_get_user",
    "arguments": {
      "username": "octocat"
    }
  }'
```

## Troubleshooting Guide

### Common Issues and Solutions

#### 1. Authentication Headers Not Propagating

**Symptoms:**
- External API calls receive 401 Unauthorized
- Tools work without auth but fail with auth context

**Diagnosis:**
```rust
// Add debug logging to verify auth context propagation
log::debug!("Auth context in Smart Discovery: {:?}", request.auth_context);
log::debug!("Generated auth headers: {:?}", auth_headers);
```

**Solutions:**
- Verify `auth_context` field is properly set in `SmartDiscoveryRequest`
- Check that `route_with_auth()` is called instead of `route()`
- Ensure external MCP clients use auth-enabled methods

#### 2. Token Expiration Issues

**Symptoms:**
- Intermittent authentication failures
- Tools work initially but fail after some time

**Diagnosis:**
```rust
// Check token expiration
if let Some(token) = auth_context.get_provider_token("oauth") {
    if token.is_expired() {
        log::warn!("Token expired at: {:?}", token.expires_at);
    }
}
```

**Solutions:**
- Implement automatic token refresh
- Add expiration buffer (refresh 5 minutes early)
- Provide clear expiration error messages

#### 3. Session Recovery Failures

**Symptoms:**
- Authentication lost after service restart
- Users need to re-authenticate frequently

**Diagnosis:**
```rust
// Check session storage
match recover_authentication_context(&session_id).await {
    Ok(Some(context)) => log::info!("Session recovered successfully"),
    Ok(None) => log::warn!("No session found for ID: {}", session_id),
    Err(e) => log::error!("Session recovery failed: {}", e),
}
```

**Solutions:**
- Verify platform-specific storage permissions
- Check filesystem fallback directory permissions
- Ensure encryption keys are available

#### 4. Multi-Provider Token Conflicts

**Symptoms:**
- Wrong provider token used for API calls
- Authentication works for some providers but not others

**Diagnosis:**
```rust
// Check provider token mapping
for (provider, token) in &auth_context.provider_tokens {
    log::debug!("Provider '{}': token_type={}, scopes={:?}", 
               provider, token.token_type, token.scopes);
}
```

**Solutions:**
- Use provider-specific token selection
- Implement provider priority ordering
- Add provider validation for tool requirements

### Debug Logging

Enable detailed authentication logging:

```bash
# Enable auth-specific debug logging
export RUST_LOG="magictunnel::auth=debug,magictunnel::mcp::clients=debug,magictunnel::discovery=debug"

# Start MagicTunnel with debug logging
./magictunnel-supervisor
```

### Health Checks

The system includes authentication health checks:

```rust
pub async fn health_check_authentication() -> Result<AuthHealthStatus> {
    let mut status = AuthHealthStatus::new();
    
    // Check session storage availability
    status.session_storage = check_session_storage().await?;
    
    // Check token refresh capabilities
    status.token_refresh = check_token_refresh_endpoints().await?;
    
    // Check external API connectivity
    status.external_apis = check_external_api_connectivity().await?;
    
    Ok(status)
}
```

## Performance Considerations

### Authentication Overhead

The authentication integration is designed for minimal performance impact:

#### Caching
- **Auth Context Caching**: Contexts cached in memory for session duration
- **Header Generation Caching**: Generated headers cached for 5 minutes
- **Token Validation Caching**: Validation results cached to avoid repeated checks

#### Async Processing
- **Non-Blocking Auth**: All authentication operations are async
- **Parallel Processing**: Multiple auth contexts processed concurrently
- **Background Refresh**: Token refresh happens in background

#### Memory Management
- **Arc<AuthenticationContext>**: Shared ownership prevents cloning
- **Lazy Header Generation**: Headers generated only when needed
- **Session Cleanup**: Expired sessions automatically cleaned up

### Performance Metrics

Monitor authentication performance with built-in metrics:

```rust
pub struct AuthMetrics {
    pub total_auth_requests: u64,
    pub successful_authentications: u64,
    pub failed_authentications: u64,
    pub token_refresh_count: u64,
    pub average_auth_time_ms: f64,
    pub session_recovery_count: u64,
}
```

## Production Deployment

### Security Best Practices

1. **Token Storage**: Use platform-specific secure storage
2. **Encryption**: Encrypt session data at rest
3. **Permissions**: Restrict session file access (0600)
4. **Rotation**: Implement regular token rotation
5. **Monitoring**: Monitor for suspicious authentication patterns

### Configuration

#### OAuth 2.1 Configuration
```yaml
oauth:
  providers:
    github:
      client_id: "${GITHUB_CLIENT_ID}"
      client_secret: "${GITHUB_CLIENT_SECRET}"
      scopes: ["repo", "user:email"]
      token_refresh: true
      
    google:
      client_id: "${GOOGLE_CLIENT_ID}"
      client_secret: "${GOOGLE_CLIENT_SECRET}"
      scopes: ["https://www.googleapis.com/auth/drive.readonly"]
      token_refresh: true

session:
  storage_method: "platform_specific" # keychain, credential_manager, secret_service
  fallback_storage: "filesystem"
  session_timeout: 86400 # 24 hours
  cleanup_interval: 3600 # 1 hour
```

#### API Key Configuration
```yaml
api_keys:
  default_expiration: 2592000 # 30 days
  rotation_enabled: true
  storage_encrypted: true
```

### Monitoring and Alerting

Set up monitoring for authentication metrics:

```yaml
monitoring:
  auth_failure_threshold: 10 # failures per minute
  token_expiration_warning: 300 # 5 minutes before expiration
  session_cleanup_alerts: true
  external_api_connectivity: true
```

## Conclusion

The MCP Client Authentication Integration system provides comprehensive authentication support for MagicTunnel, enabling secure and seamless tool execution with external APIs. The system supports multiple authentication methods, automatic token management, and robust error recovery, making it suitable for production deployments requiring enterprise-grade authentication.

### Key Benefits

1. **Seamless Integration**: Authentication flows transparently through Smart Discovery
2. **Multi-Provider Support**: Works with OAuth, API keys, JWT, and service accounts
3. **Automatic Management**: Token refresh, expiration detection, and session recovery
4. **Cross-Platform**: Secure storage on macOS, Windows, and Linux
5. **Production Ready**: Comprehensive testing, monitoring, and error handling

### Future Enhancements

- **SAML Integration**: Support for SAML-based authentication
- **Certificate Authentication**: Client certificate authentication support  
- **Advanced Token Management**: Token rotation policies and lifecycle management
- **Audit Logging**: Comprehensive authentication audit trails
- **Rate Limiting**: Authentication-aware rate limiting and throttling

This authentication integration represents a **critical milestone** in MagicTunnel's evolution, enabling **authenticated tool calls through smart discovery and external MCP proxying** while maintaining security, performance, and reliability standards required for enterprise deployments.