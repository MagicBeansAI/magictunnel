# OAuth 2.1 API Reference

## Overview

MagicTunnel implements a comprehensive OAuth 2.1 authentication system with full MCP (Model Context Protocol) integration. This document provides complete API reference for all OAuth endpoints, authentication flows, and error responses.

## Table of Contents

1. [Authentication Methods](#authentication-methods)
2. [OAuth 2.1 Flow](#oauth-21-flow)
3. [Device Code Flow](#device-code-flow)
4. [API Key Authentication](#api-key-authentication)
5. [Service Account Authentication](#service-account-authentication)
6. [MCP Integration](#mcp-integration)
7. [Error Responses](#error-responses)
8. [Configuration](#configuration)
9. [Session Management](#session-management)
10. [Production Deployment](#production-deployment)

## Authentication Methods

MagicTunnel supports 4 enterprise-grade authentication methods:

### 1. OAuth 2.1 with PKCE
- **Use Case**: Interactive browser-based authentication
- **Providers**: GitHub, Google, Microsoft, Custom
- **Features**: PKCE, Resource Indicators (RFC 8707), automatic token refresh

### 2. Device Code Flow (RFC 8628)
- **Use Case**: Headless environments (servers, CLI, Docker)
- **Providers**: GitHub, Google, Microsoft, Custom
- **Features**: No browser required, automatic polling, user instructions

### 3. API Key Authentication
- **Use Case**: Service-to-service authentication
- **Features**: Static keys, permission scoping, rate limiting

### 4. Service Account Authentication
- **Use Case**: Machine authentication with provider credentials
- **Providers**: GitHub PAT, GitLab PAT, Google Service Account Keys
- **Features**: Non-interactive, credential validation, automatic refresh

## OAuth 2.1 Flow

### Initiate Authorization

**Endpoint**: `POST /mcp/auth/oauth/authorize`

**Request Body**:
```json
{
  "provider": "github",
  "scopes": ["user:email", "repo:read"],
  "state": "optional_state_parameter",
  "resource": "https://api.github.com" // Optional Resource Indicator
}
```

**Response**:
```json
{
  "authorization_url": "https://github.com/login/oauth/authorize?client_id=...&code_challenge=...",
  "state": "generated_state_value",
  "code_verifier": "stored_internally",
  "expires_in": 600
}
```

**MCP Error Response** (if authentication required):
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "OAuth authorization required",
    "data": {
      "auth_type": "oauth",
      "provider": "github",
      "authorization_url": "https://github.com/login/oauth/authorize?client_id=...&code_challenge=...",
      "state": "abc123",
      "expires_in": 600,
      "instructions": "Visit the authorization URL to grant access. MagicTunnel will automatically continue once authorized."
    }
  }
}
```

### Complete Authorization

**Endpoint**: `POST /mcp/auth/oauth/callback`

**Request Body**:
```json
{
  "code": "authorization_code_from_provider",
  "state": "state_from_initial_request"
}
```

**Response**:
```json
{
  "access_token": "oauth_access_token",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "oauth_refresh_token",
  "scope": "user:email repo:read",
  "user_info": {
    "id": "user_123",
    "login": "username",
    "name": "Full Name",
    "email": "user@example.com"
  }
}
```

### Token Refresh

**Endpoint**: `POST /mcp/auth/oauth/refresh`

**Request Body**:
```json
{
  "refresh_token": "oauth_refresh_token",
  "provider": "github"
}
```

**Response**:
```json
{
  "access_token": "new_oauth_access_token",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "new_oauth_refresh_token",
  "scope": "user:email repo:read"
}
```

## Device Code Flow

### Initiate Device Authorization

**Endpoint**: `POST /mcp/auth/device/authorize`

**Request Body**:
```json
{
  "provider": "github",
  "scopes": ["user:email", "repo:read"]
}
```

**Response**:
```json
{
  "device_code": "3584d83297b6ac1c7335a452b5c5f1a1",
  "user_code": "WDJB-MJHT",
  "verification_uri": "https://github.com/login/device",
  "verification_uri_complete": "https://github.com/login/device?user_code=WDJB-MJHT",
  "expires_in": 1800,
  "interval": 5
}
```

**MCP Error Response** (if device authorization required):
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Device authorization required",
    "data": {
      "auth_type": "device_code",
      "provider": "github",
      "user_code": "WDJB-MJHT",
      "verification_uri": "https://github.com/login/device",
      "verification_uri_complete": "https://github.com/login/device?user_code=WDJB-MJHT",
      "expires_in": 1800,
      "interval": 5,
      "instructions": "Go to https://github.com/login/device and enter code: WDJB-MJHT. MagicTunnel will automatically continue once authorized."
    }
  }
}
```

### Poll for Token

**Endpoint**: `POST /mcp/auth/device/token`

**Request Body**:
```json
{
  "device_code": "3584d83297b6ac1c7335a452b5c5f1a1",
  "provider": "github"
}
```

**Response** (Success):
```json
{
  "access_token": "oauth_access_token",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "oauth_refresh_token",
  "scope": "user:email repo:read",
  "user_info": {
    "id": "user_123",
    "login": "username",
    "name": "Full Name",
    "email": "user@example.com"
  }
}
```

**Response** (Pending):
```json
{
  "error": "authorization_pending",
  "error_description": "The authorization request is still pending as the end user hasn't yet completed the user interaction steps."
}
```

**Response** (Slow Down):
```json
{
  "error": "slow_down",
  "error_description": "The interval was exceeded. Reduce polling frequency."
}
```

## API Key Authentication

### Validate API Key

**Endpoint**: `POST /mcp/auth/apikey/validate`

**Request Headers**:
```
Authorization: ApiKey your_api_key_here
```

**Response**:
```json
{
  "valid": true,
  "key_name": "admin_key",
  "permissions": ["read", "write", "admin"],
  "expires_at": null
}
```

### MCP Integration

API keys are passed via HTTP Authorization header:
```
Authorization: ApiKey sk-1234567890abcdef
```

## Service Account Authentication

### GitHub Personal Access Token

**Configuration**:
```yaml
service_accounts:
  github_pat:
    type: github_pat
    token: "${GITHUB_PERSONAL_ACCESS_TOKEN}"
    scopes: ["repo", "user:email"]
```

**Validation Endpoint**: `POST /mcp/auth/service-account/validate`

**Request Body**:
```json
{
  "account_type": "github_pat",
  "token": "ghp_xxxxxxxxxxxxxxxxxxxx"
}
```

**Response**:
```json
{
  "valid": true,
  "user_info": {
    "id": "123456",
    "login": "username",
    "name": "Full Name",
    "email": "user@example.com"
  },
  "permissions": ["repo", "user:email"],
  "account_type": "GitHubPAT",
  "expires_at": null
}
```

### Google Service Account Key

**Configuration**:
```yaml
service_accounts:
  google_service:
    type: google_service_account
    credentials_file: "/path/to/service-account.json"
    scopes: ["https://www.googleapis.com/auth/spreadsheets"]
```

**Validation Response**:
```json
{
  "valid": true,
  "user_info": {
    "id": "service-account@project.iam.gserviceaccount.com",
    "name": "Service Account Name",
    "email": "service-account@project.iam.gserviceaccount.com"
  },
  "permissions": ["https://www.googleapis.com/auth/spreadsheets"],
  "account_type": "GoogleServiceAccount",
  "expires_at": 1640995200
}
```

## MCP Integration

### Authentication Context Flow

1. **HTTP Request with Bearer Token**:
   ```
   POST /mcp/call
   Authorization: Bearer oauth_access_token
   Content-Type: application/json
   
   {
     "name": "github_create_issue",
     "arguments": {
       "repo": "user/repo",
       "title": "Test Issue"
     }
   }
   ```

2. **Authentication Context Creation**:
   ```rust
   AuthenticationContext {
       user_id: "github_user_123",
       provider_tokens: {
           "github": ProviderToken {
               access_token: "oauth_access_token",
               token_type: "Bearer",
               scopes: ["user:email", "repo:read"],
               expires_at: 1640995200
           }
       },
       session_id: "session_abc123",
       auth_method: OAuth { provider: "github", scopes: [...] }
   }
   ```

3. **Tool Execution with Authentication**:
   ```rust
   ToolExecutionContext {
       tool_name: "github_create_issue",
       arguments: { "repo": "user/repo", "title": "Test Issue" },
       auth_context: Some(authentication_context)
   }
   ```

4. **External API Call with Token**:
   ```rust
   // Tool implementation automatically uses OAuth token
   let headers = auth_context.get_auth_headers(Some("github"));
   // Headers include: Authorization: Bearer oauth_access_token
   ```

### Session Persistence

MagicTunnel automatically persists authentication sessions across:

- **STDIO Mode**: Sessions stored in `~/.magictunnel/sessions/`
- **Remote MCP**: Distributed session storage with Redis support
- **Multi-Platform**: Native credential storage (Keychain, Credential Manager, Secret Service)

### Session Recovery

Sessions are automatically recovered on:
- Process restarts
- Server restarts
- Connection failures
- Token expiration (with automatic refresh)

## Error Responses

### OAuth Errors

**Invalid Provider**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32002,
    "message": "Invalid OAuth provider",
    "data": {
      "provider": "unknown_provider",
      "available_providers": ["github", "google", "microsoft"]
    }
  }
}
```

**Token Expired**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32003,
    "message": "OAuth token expired",
    "data": {
      "provider": "github",
      "expired_at": "2024-01-01T12:00:00Z",
      "refresh_available": true
    }
  }
}
```

### Device Code Errors

**Device Code Expired**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32004,
    "message": "Device code expired",
    "data": {
      "provider": "github",
      "expired_at": "2024-01-01T12:30:00Z",
      "instructions": "Please restart the device authorization process"
    }
  }
}
```

**Access Denied**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32005,
    "message": "User denied authorization",
    "data": {
      "provider": "github",
      "user_code": "WDJB-MJHT"
    }
  }
}
```

### API Key Errors

**Invalid API Key**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32006,
    "message": "Invalid API key",
    "data": {
      "key_prefix": "sk-1234...",
      "reason": "Key not found or expired"
    }
  }
}
```

### Service Account Errors

**Invalid Service Account**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32007,
    "message": "Service account validation failed",
    "data": {
      "account_type": "github_pat",
      "reason": "Token invalid or insufficient permissions"
    }
  }
}
```

## Configuration

### Complete Configuration Example

```yaml
# Multi-level authentication configuration
auth:
  enabled: true
  
  # Server-level default (applies to all tools unless overridden)
  server_level:
    type: oauth
    provider: github
    scopes: ["user:email"]
  
  # Capability-level overrides
  capabilities:
    google_workspace:
      type: oauth
      provider: google
      scopes: ["https://www.googleapis.com/auth/spreadsheets"]
    
    headless_tools:
      type: device_code
      provider: github_headless
      scopes: ["repo:read", "user:email"]
  
  # Tool-level overrides (highest priority)
  tools:
    admin_tool:
      type: api_key
      key_ref: admin_key
    
    github_deploy_tool:
      type: service_account
      account_ref: github_deploy_pat

# OAuth Provider Configurations
oauth_providers:
  github:
    client_id: "${GITHUB_CLIENT_ID}"
    client_secret: "${GITHUB_CLIENT_SECRET}"
    oauth_enabled: true
    device_code_enabled: true
    scopes: ["user:email", "repo:read"]
    authorization_endpoint: "https://github.com/login/oauth/authorize"
    device_authorization_endpoint: "https://github.com/login/device/code"
    token_endpoint: "https://github.com/login/oauth/access_token"
    user_info_endpoint: "https://api.github.com/user"
  
  # Headless-only provider
  github_headless:
    client_id: "${GITHUB_DEVICE_CLIENT_ID}"
    client_secret: "${GITHUB_DEVICE_CLIENT_SECRET}"
    oauth_enabled: false # Force device code flow
    device_code_enabled: true
    device_authorization_endpoint: "https://github.com/login/device/code"
    token_endpoint: "https://github.com/login/oauth/access_token"
    user_info_endpoint: "https://api.github.com/user"
  
  google:
    client_id: "${GOOGLE_CLIENT_ID}"
    client_secret: "${GOOGLE_CLIENT_SECRET}"
    oauth_enabled: true
    device_code_enabled: true
    authorization_endpoint: "https://accounts.google.com/o/oauth2/auth"
    device_authorization_endpoint: "https://oauth2.googleapis.com/device/code"
    token_endpoint: "https://oauth2.googleapis.com/token"
    user_info_endpoint: "https://www.googleapis.com/oauth2/v2/userinfo"

# API Keys
api_keys:
  admin_key:
    key: "${ADMIN_API_KEY}"
    name: "Admin Key"
    permissions: ["read", "write", "admin"]
    active: true

# Service Accounts
service_accounts:
  github_deploy_pat:
    type: github_pat
    token: "${GITHUB_DEPLOY_PAT}"
    scopes: ["repo", "workflow"]
  
  google_service:
    type: google_service_account
    credentials_file: "/path/to/service-account.json"
    scopes: ["https://www.googleapis.com/auth/spreadsheets"]

# Session Persistence
session_persistence:
  stdio:
    enabled: true
    storage_backend: "filesystem" # filesystem | keychain | credential_manager
    storage_path: "~/.magictunnel/sessions"
    encryption_key: "${SESSION_ENCRYPTION_KEY}"
  
  remote_mcp:
    enabled: true
    health_check_interval: "30s"
    session_recovery_timeout: "5m"
    distributed_storage:
      enabled: true
      backend: "redis"
      redis_url: "${REDIS_URL}"
  
  token_management:
    refresh_threshold: "5m" # Refresh tokens 5 minutes before expiry
    max_token_lifetime: "24h"
    cleanup_expired_sessions: "1h"
```

### Environment Variables

```bash
# OAuth Provider Credentials
export GITHUB_CLIENT_ID="your_github_client_id"
export GITHUB_CLIENT_SECRET="your_github_client_secret"
export GOOGLE_CLIENT_ID="your_google_client_id"
export GOOGLE_CLIENT_SECRET="your_google_client_secret"

# Device Code Flow (Headless)
export GITHUB_DEVICE_CLIENT_ID="your_device_client_id"
export GITHUB_DEVICE_CLIENT_SECRET="your_device_client_secret"

# API Keys
export ADMIN_API_KEY="sk-1234567890abcdef"

# Service Accounts
export GITHUB_DEPLOY_PAT="ghp_xxxxxxxxxxxxxxxxxxxx"

# Session Encryption
export SESSION_ENCRYPTION_KEY="your_32_byte_encryption_key"

# Distributed Storage
export REDIS_URL="redis://localhost:6379"
```

## Session Management

### Session Storage Backends

#### Filesystem Storage
- **Location**: `~/.magictunnel/sessions/`
- **Encryption**: AES-256-GCM
- **Permissions**: 0700 (owner only)
- **Cross-Platform**: All platforms

#### Native Credential Storage
- **macOS**: Keychain integration
- **Windows**: Credential Manager
- **Linux**: Secret Service API
- **Fallback**: Encrypted filesystem

#### Distributed Storage (Redis)
- **Use Case**: Multi-instance deployments
- **Encryption**: AES-256-GCM in Redis
- **TTL**: Automatic session expiration
- **Failover**: Falls back to filesystem

### Session Lifecycle

1. **Session Creation**: On successful authentication
2. **Session Persistence**: Automatic storage across restarts
3. **Session Recovery**: Automatic on startup
4. **Token Refresh**: Background refresh before expiry
5. **Session Cleanup**: Automatic cleanup of expired sessions

### Session Isolation

MagicTunnel implements mathematical session isolation:

```rust
session_key = SHA256(username + hostname + deployment_id + process_id)
```

This prevents:
- Cross-user session leakage
- Cross-deployment session conflicts
- Process isolation violations

## Production Deployment

### Security Best Practices

1. **Use HTTPS**: All OAuth redirects must use HTTPS
2. **Secure Storage**: Use native credential storage when available
3. **Environment Variables**: Store secrets in environment, not config files
4. **Session Encryption**: Always encrypt session storage
5. **Token Rotation**: Enable automatic token refresh
6. **Audit Logging**: Enable authentication audit logs

### Health Monitoring

```bash
# Check authentication system health
curl -X GET http://localhost:3001/health/auth

# Response
{
  "healthy": true,
  "providers": {
    "github": "healthy",
    "google": "healthy"
  },
  "session_storage": "healthy",
  "token_refresh": "healthy"
}
```

### Metrics and Monitoring

Key metrics to monitor:

- `auth_requests_total` - Total authentication requests
- `auth_requests_success` - Successful authentications
- `auth_token_refresh_total` - Token refresh attempts
- `auth_session_recovery_total` - Session recovery events
- `auth_error_total` - Authentication errors by type

### Load Testing

Example load test scenarios:

1. **OAuth Flow Load**: 100 concurrent OAuth flows
2. **Token Refresh Load**: 1000 concurrent token refreshes
3. **Session Recovery Load**: Simulate server restarts with active sessions
4. **Device Code Flow Load**: Multiple device code flows in parallel

### Troubleshooting

#### Common Issues

**OAuth Authorization Fails**:
- Check provider configuration
- Verify client ID/secret
- Ensure HTTPS redirect URIs
- Check OAuth scopes

**Device Code Times Out**:
- Check device authorization endpoint
- Verify polling interval
- Ensure user completes authorization

**Session Recovery Fails**:
- Check session storage permissions
- Verify encryption key
- Check session expiration

**Token Refresh Fails**:
- Check refresh token validity
- Verify provider token endpoint
- Check network connectivity

#### Debug Logging

Enable debug logging:
```bash
export RUST_LOG=magictunnel::auth=debug
./magictunnel
```

## API Rate Limits

### Provider Rate Limits

- **GitHub**: 5,000 requests/hour (authenticated)
- **Google**: Varies by API
- **Custom**: Configurable per provider

### MagicTunnel Rate Limits

- **OAuth Authorization**: 10 requests/minute per IP
- **Token Refresh**: 60 requests/minute per session
- **API Key Validation**: 1000 requests/minute per key

## Security Considerations

### PKCE Implementation

MagicTunnel implements PKCE (RFC 7636) for all OAuth flows:

```rust
// Code challenge generation
let code_verifier = generate_random_string(128);
let code_challenge = base64url(sha256(code_verifier));
let code_challenge_method = "S256";
```

### Resource Indicators (RFC 8707)

Resource Indicators provide enhanced security:

```yaml
oauth_providers:
  github:
    resource_indicators:
      - "https://api.github.com"
      - "https://uploads.github.com"
```

### Token Security

- **Storage**: Encrypted with AES-256-GCM
- **Transit**: Always HTTPS
- **Memory**: Zeroized on drop
- **Logging**: Tokens never logged

## SDK Integration

### MCP Client Libraries

MagicTunnel authentication works seamlessly with MCP client libraries:

#### Python
```python
from mcp import ClientSession

# OAuth authentication
session = ClientSession()
session.authenticate_oauth("github", ["user:email", "repo:read"])

# Device code authentication (headless)
session.authenticate_device_code("github", ["user:email", "repo:read"])

# API key authentication
session.authenticate_api_key("sk-1234567890abcdef")
```

#### TypeScript
```typescript
import { ClientSession } from '@modelcontextprotocol/sdk';

// OAuth authentication
const session = new ClientSession();
await session.authenticateOAuth('github', ['user:email', 'repo:read']);

// Device code authentication
await session.authenticateDeviceCode('github', ['user:email', 'repo:read']);
```

### Claude Desktop Integration

MagicTunnel works with Claude Desktop's MCP integration:

```json
{
  "mcpServers": {
    "magictunnel": {
      "command": "magictunnel",
      "args": ["--stdio", "--config", "config.yaml"],
      "env": {
        "GITHUB_CLIENT_ID": "your_client_id",
        "GITHUB_CLIENT_SECRET": "your_client_secret"
      }
    }
  }
}
```

## Migration Guide

### From v0.2.x to v0.3.x

1. **Update Configuration Format**:
   ```yaml
   # Old format
   auth:
     github_token: "token"
   
   # New format
   auth:
     server_level:
       type: oauth
       provider: github
   ```

2. **Update Environment Variables**:
   ```bash
   # Old
   export GITHUB_TOKEN="token"
   
   # New
   export GITHUB_CLIENT_ID="client_id"
   export GITHUB_CLIENT_SECRET="client_secret"
   ```

3. **Session Migration**:
   Sessions will be automatically migrated on first startup.

## Support and Resources

- **Documentation**: [docs/OAUTH_2_1_tasks.md](./OAUTH_2_1_tasks.md)
- **Configuration Examples**: [config.yaml.template](../config.yaml.template)
- **Troubleshooting**: [docs/TROUBLESHOOTING.md](./TROUBLESHOOTING.md)
- **GitHub Issues**: [GitHub Repository](https://github.com/your-org/magictunnel/issues)

## Changelog

### v0.3.12 (Current)
- ✅ Complete OAuth 2.1 implementation with PKCE
- ✅ Device Code Flow (RFC 8628) support
- ✅ Multi-platform session persistence
- ✅ Service account authentication
- ✅ MCP Protocol integration (Phase 6)
- ✅ Comprehensive integration testing

### v0.3.11
- ✅ Core authentication infrastructure
- ✅ Multi-level authentication resolution
- ✅ Session recovery system

### Previous Versions
See [CHANGELOG.md](../CHANGELOG.md) for complete version history.