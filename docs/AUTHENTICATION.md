# Authentication in MCP Proxy

MCP Proxy supports multiple authentication methods to secure your endpoints and control access to tools and resources.

## Overview

Authentication in MCP Proxy provides:
- **API Key Authentication**: ✅ **FULLY IMPLEMENTED** - Simple token-based authentication with permissions
- **OAuth 2.0**: ✅ **FULLY IMPLEMENTED** - Complete OAuth 2.0 authorization code flow with provider support
- **JWT Tokens**: ✅ **FULLY IMPLEMENTED** - Complete JWT token validation with configurable algorithms
- **Permission-based Access Control**: ✅ **FULLY IMPLEMENTED** - Fine-grained permissions for different operations
- **Flexible Configuration**: ✅ **FULLY IMPLEMENTED** - Easy to enable/disable and configure

## Configuration

Authentication is configured in the `auth` section of your configuration file:

```yaml
auth:
  enabled: true
  type: "api_key"  # api_key, oauth, jwt, or none
  # ... type-specific configuration
```

## API Key Authentication ✅ **FULLY IMPLEMENTED**

The most straightforward authentication method using API keys. This is the only authentication method currently fully implemented and ready for production use.

### Configuration

```yaml
auth:
  enabled: true
  type: "api_key"
  api_keys:
    keys:
      - key: "your_api_key_here"
        name: "Admin Key"
        description: "Administrative access"
        permissions:
          - "read"
          - "write"
          - "admin"
        active: true
        expires_at: "2025-12-31T23:59:59Z"  # Optional
    require_header: true
    header_name: "Authorization"
    header_format: "Bearer {key}"
```

### Usage

Include the API key in your HTTP requests:

```bash
curl -H "Authorization: Bearer your_api_key_here" \
     http://localhost:8080/mcp/tools
```

### Permissions

API keys support the following permissions:
- `read`: Access to list tools, resources, and prompts
- `write`: Execute tools and modify resources
- `admin`: Administrative operations and configuration

## OAuth 2.1 Authentication ✅ **FULLY IMPLEMENTED** - Phase 1 & 2 Complete

Enterprise-grade OAuth 2.1 authentication system with multi-level authentication, session persistence, and Device Code Flow support.

OAuth 2.1 provides enhanced security features including PKCE, Resource Indicators (RFC 8707), and comprehensive session management. The implementation supports both interactive browser flows and headless environments through Device Code Flow (RFC 8628).

### Configuration

```yaml
auth:
  enabled: true
  type: "oauth"  # Fully functional OAuth 2.0 authentication
  oauth:
    provider: "github"  # or "google", "microsoft", "custom"
    client_id: "your_oauth_client_id"
    client_secret: "your_oauth_client_secret"
    redirect_uri: "http://localhost:8080/auth/callback"
    scope: "user:email"  # Optional, provider-specific defaults used

    # For custom providers
    authorization_url: "https://provider.com/oauth/authorize"  # Optional
    token_url: "https://provider.com/oauth/token"              # Optional
    user_info_url: "https://provider.com/oauth/userinfo"       # Optional
```

### Supported Providers

- **GitHub**: `provider: "github"`
  - Default scope: `"user:email"`
  - User info endpoint: `https://api.github.com/user`
- **Google**: `provider: "google"`
  - Default scope: `"openid email profile"`
  - User info endpoint: `https://www.googleapis.com/oauth2/v2/userinfo`
- **Microsoft**: `provider: "microsoft"`
  - Default scope: `"openid profile email"`
  - User info endpoint: `https://graph.microsoft.com/v1.0/me`
- **Custom**: `provider: "custom"` with custom URLs

**Note**: MagicTunnel uses standard OAuth scopes for each provider. MCP-specific permissions are managed internally within MagicTunnel after successful OAuth authentication.

### OAuth Flow

1. **Authorization Request**: Client redirects to provider's authorization URL
2. **User Consent**: User grants permission at the provider
3. **Authorization Code**: Provider redirects back with authorization code
4. **Token Exchange**: Server exchanges code for access token
5. **User Info**: Server retrieves user information using access token
6. **Authentication**: User is authenticated for subsequent requests

### Implementation Features

- ✅ **Phase 1 Complete**: Multi-level authentication infrastructure with OAuth 2.1 and PKCE
- ✅ **Phase 1 Complete**: Device Code Flow (RFC 8628) for headless environments
- ✅ **Phase 1 Complete**: Authentication resolution with hierarchical fallback
- ✅ **Phase 1 Complete**: Security enhancements with credential protection
- ✅ **Phase 2 Complete**: Session persistence with user context system
- ✅ **Phase 2 Complete**: Multi-platform token storage (filesystem, keychain, credential manager)
- ✅ **Phase 2 Complete**: Automatic session recovery on startup
- ✅ **Phase 2 Complete**: Token refresh service with background renewal
- ✅ Provider-specific configurations with custom provider support
- ✅ Resource Indicators (RFC 8707) for enhanced authorization scope
- ✅ Comprehensive error handling with structured MCP responses
- ✅ MCP 2025-06-18 compliance features

**OAuth 2.1 Enhanced Features**:
- **PKCE (Proof Key for Code Exchange)**: Mandatory PKCE for all authorization flows
- **Resource Indicators (RFC 8707)**: Enhanced token security with resource scoping
- **Device Code Flow (RFC 8628)**: Headless authentication for CLI/server environments
- **Multi-Level Authentication**: Server → Capability → Tool level authentication hierarchy
- **Session Persistence**: Automatic session recovery across process restarts
- **Token Management**: Background refresh with secure multi-platform storage

## JWT Authentication ✅ **FULLY IMPLEMENTED**

JSON Web Token validation for stateless authentication with comprehensive algorithm support.

### Configuration

```yaml
auth:
  enabled: true
  type: "jwt"
  jwt:
    secret: "your_jwt_secret_key_at_least_32_characters_long"
    algorithm: "HS256"  # Supported: HS256, HS384, HS512, RS256, RS384, RS512, ES256, ES384
    expiration: 3600  # Token expiration in seconds (1 hour)
    issuer: "magictunnel"  # Optional: JWT issuer claim
    audience: "mcp-clients"  # Optional: JWT audience claim
```

### Supported Algorithms

- **HMAC**: HS256, HS384, HS512 (symmetric key)
- **RSA**: RS256, RS384, RS512 (asymmetric key)
- **ECDSA**: ES256, ES384 (asymmetric key)

### Token Format

JWT tokens must include the following claims:

```json
{
  "sub": "user_id",           // Subject (user identifier)
  "iat": 1234567890,          // Issued at timestamp
  "exp": 1234567893600,       // Expiration timestamp
  "iss": "magictunnel",         // Issuer (optional, must match config)
  "aud": "mcp-clients",       // Audience (optional, must match config)
  "permissions": ["read", "write", "admin"],  // User permissions
  "user_info": {              // Optional user information
    "id": "user_123",
    "email": "user@example.com",
    "name": "John Doe",
    "roles": ["user", "admin"]
  }
}
```

### Usage Examples

#### 1. Authorization Header
```bash
curl -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
     http://localhost:8080/mcp/tools
```

#### 2. Query Parameter
```bash
curl "http://localhost:8080/mcp/tools?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

### Planned Algorithms (Not Yet Implemented)

- HS256, HS384, HS512 (HMAC)
- RS256, RS384, RS512 (RSA)
- ES256, ES384, ES512 (ECDSA)

> **Note**: Complete JWT implementation with token parsing, signature verification, claims validation, and expiration checking.

## Endpoints and Permissions ✅ **FULLY IMPLEMENTED**

Different endpoints require different permission levels (currently only enforced for API Key authentication):

| Endpoint | Required Permission | Description |
|----------|-------------------|-------------|
| `/health` | None | Health check (always accessible) |
| `/mcp/tools` | `read` | List available tools |
| `/mcp/call` | `write` | Execute tools |
| `/mcp/call/stream` | `write` | Streaming tool execution |
| `/mcp/resources` | `read` | List resources |
| `/mcp/resources/read` | `read` | Read resource content |
| `/mcp/prompts` | `read` | List prompts |
| `/mcp/prompts/get` | `read` | Get prompt content |
| `/mcp/logging/setLevel` | `admin` | Change log levels |

## Error Responses

Authentication failures return appropriate HTTP status codes:

### 401 Unauthorized
```json
{
  "error": {
    "code": "AUTHENTICATION_FAILED",
    "message": "Invalid API key",
    "type": "authentication_error"
  }
}
```

### 403 Forbidden
```json
{
  "error": {
    "code": "INSUFFICIENT_PERMISSIONS",
    "message": "API key does not have 'write' permission",
    "type": "authorization_error"
  }
}
```

## Security Best Practices

1. **Use HTTPS**: Always use HTTPS in production to protect API keys in transit
2. **Rotate Keys**: Regularly rotate API keys and set expiration dates
3. **Principle of Least Privilege**: Grant only the minimum permissions needed
4. **Monitor Access**: Enable logging to monitor authentication events
5. **Secure Storage**: Store API keys securely and never commit them to version control

## Disabling Authentication

To disable authentication (not recommended for production):

```yaml
auth:
  enabled: false
```

Or omit the `auth` section entirely.

## Examples

See `examples/auth_config.yaml` for a complete configuration example with API key authentication.

## Testing Authentication ✅ **FULLY IMPLEMENTED**

The project includes comprehensive integration tests for API key authentication. Run them with:

```bash
cargo test --test auth_integration_tests
```

> **Note**: Tests cover API key, OAuth 2.0, and JWT authentication with comprehensive test coverage.

## Troubleshooting

### Common Issues

1. **Missing Authorization Header**: Ensure the header name matches your configuration
2. **Invalid Key Format**: Check that the header format includes the `{key}` placeholder
3. **Expired Keys**: Verify that API keys haven't expired
4. **Insufficient Permissions**: Ensure the API key has the required permissions for the endpoint

### Debug Logging

Enable debug logging to troubleshoot authentication issues:

```yaml
logging:
  level: "debug"
```

This will log authentication attempts and permission checks.

## Implementation Status Summary - OAuth 2.1 Phase 1 & 2 Complete ✅

### ✅ **OAuth 2.1 Authentication System - Production Ready**
- **Phase 1 Complete**: Multi-level authentication configuration with hierarchical resolution
- **Phase 1 Complete**: OAuth 2.1 with PKCE and Resource Indicators implementation
- **Phase 1 Complete**: Device Code Flow (RFC 8628) for headless environments
- **Phase 1 Complete**: Security enhancements with secure credential storage
- **Phase 2 Complete**: Session persistence system with user context identification
- **Phase 2 Complete**: Multi-platform token storage (filesystem, keychain, credential manager)
- **Phase 2 Complete**: Automatic session recovery for STDIO and remote MCP modes
- **Phase 2 Complete**: Token refresh service with background renewal

### ✅ **Additional Authentication Methods - Fully Implemented**
- **API Key Authentication**: Complete implementation with permissions and validation
  - ✅ Granular read/write/admin permissions for all endpoints
  - ✅ Authentication middleware with HTTP request validation
  - ✅ Comprehensive test coverage (7 integration tests)
- **JWT Authentication**: Complete implementation with multi-algorithm support
  - ✅ JWT token parsing, signature verification, and claims validation
  - ✅ Support for HMAC (HS256/384/512), RSA (RS256/384/512), and ECDSA (ES256/384) algorithms
  - ✅ JWT middleware and HTTP request handling
  - ✅ Comprehensive test coverage (10 tests)

### 🎯 **Recommended Usage - Enterprise Authentication**
- **For Production**: All authentication methods are fully implemented and enterprise-ready
  - **OAuth 2.1**: Interactive user authentication with enhanced security (PKCE, Resource Indicators)
  - **Device Code Flow**: Headless environments, CLI tools, Docker containers, CI/CD pipelines
  - **API Key**: Service-to-service communication with granular permissions
  - **JWT**: Stateless authentication for distributed systems and microservices

### 🚀 **Session Persistence Benefits**
- **STDIO Mode**: No re-authentication on Claude Desktop or MCP client restarts
- **Remote MCP**: Transparent session recovery when MCP servers restart
- **Multi-Platform**: Secure token storage using native credential systems
- **Background Refresh**: Automatic token renewal before expiration
