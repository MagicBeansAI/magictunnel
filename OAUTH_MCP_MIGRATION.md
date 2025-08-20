# OAuth-Enabled MCP Discovery Migration Guide

## Overview

This guide covers migrating from `npx mcp-remote` to native OAuth-enabled MCP discovery in MagicTunnel. The new system provides:

- **Native OAuth 2.1 integration** with PKCE and resource indicators
- **Dynamic client registration** (RFC 7591) for automatic OAuth setup  
- **Comprehensive audit logging** for all OAuth flows and API calls
- **`oauth_termination_here` flag** to control where OAuth browser spawning happens
- **Secure credential storage** using MagicTunnel's existing auth infrastructure

## Quick Migration: Globalping Example

### Before (npx mcp-remote)
```yaml
# external-mcp-servers.yaml
mcpServers:
  globalping:
    command: "npx"
    args: ["mcp-remote", "https://mcp.globalping.dev/sse"]
    env:
      PATH: "${PATH}"
```

### After (RFC 8414/9728 Discovery-First OAuth)
```yaml
# external-mcp-servers-oauth.yaml
oauthMcpServers:
  globalping:
    enabled: true
    base_url: "https://mcp.globalping.dev"  # Only URL needed!
    oauth_termination_here: true  # MT spawns browser
    enable_dynamic_registration: true
    # ✅ All scopes, grant types, endpoints discovered automatically via RFC 8414/9728!
    # ✅ No manual scope configuration required!
```

**Key Improvement**: Scopes, grant types, and OAuth endpoints are now **automatically discovered** from the server's `/.well-known/oauth-authorization-server` endpoint per RFC 8414, eliminating manual configuration guesswork.

## Key Features

### 1. RFC-Compliant Automatic Discovery

**RFC 8414 Authorization Server Metadata**:
- ✅ Automatically discovers `authorization_endpoint`, `token_endpoint`, `registration_endpoint`
- ✅ Fetches `scopes_supported`, `grant_types_supported`, `response_types_supported`
- ✅ Detects PKCE support via `code_challenge_methods_supported`
- ✅ No manual endpoint or capability configuration needed

**RFC 9728 Protected Resource Metadata** (Optional):
- ✅ Discovers resource-specific scopes from `/.well-known/oauth-protected-resource`
- ✅ Validates scope intersection between auth server and resource server
- ✅ Provides fine-grained access control for MCP resources

### 2. OAuth Termination Control

**`oauth_termination_here: true`** (MagicTunnel handles OAuth):
- ✅ Uses **discovered** authorization endpoint for browser redirect
- ✅ Requests **discovered** scopes during authorization
- ✅ Stores and manages access tokens with **discovered** capabilities
- ✅ Automatic token refresh using **discovered** token endpoint
- ✅ Full visibility in MagicTunnel audit logs
- 🎯 **Use case**: Development, testing, or when MT has UI access

**`oauth_termination_here: false`** (Forward to client):
- ✅ Forward **discovered** OAuth metadata to MCP client (Claude Desktop, etc.)
- ✅ Client receives RFC-compliant server capabilities and endpoints
- ✅ Client handles authorization using **discovered** authorization endpoint
- ✅ Validates token scopes against **discovered** server capabilities
- ✅ Audit logs show token usage with scope validation
- 🎯 **Use case**: Production deployments, headless environments

### 3. Dynamic vs Static Registration (Both RFC-Compliant)

**Dynamic Registration** (RFC 7591 + RFC 8414 Discovery):
```yaml
enable_dynamic_registration: true
registration_metadata:
  client_name: "MagicTunnel-{{hostname}}"
  # ✅ Uses discovered scopes, grant types, response types automatically!
  # Optional overrides only if you need to restrict discovered capabilities:
  # requested_scopes_override: ["subset", "of", "discovered", "scopes"]
```
- ✅ **Discovery-First**: Uses RFC 8414 discovered registration endpoint
- ✅ **Smart Defaults**: Automatically uses discovered scopes and grant types
- ✅ **Override Option**: Can restrict discovered capabilities if needed
- ✅ **Validation**: Warns if overrides conflict with server capabilities
- ✅ Ideal for development and auto-deployment

**Static Credentials** (RFC 8414 Discovery + Static Auth):
```yaml
enable_dynamic_registration: false
static_credentials:
  client_id: "${OAUTH_CLIENT_ID}"
  client_secret: "${OAUTH_CLIENT_SECRET}"
  # ✅ Scopes validated against discovered server capabilities!
```
- ✅ **Discovery-First**: Still uses RFC 8414 for endpoints and capabilities
- ✅ **Static Auth**: Uses pre-configured client credentials
- ✅ **Capability Validation**: Verifies static config against discovered metadata
- ✅ **Fallback Support**: Manual metadata when discovery completely fails
- ✅ Ideal for production environments with approved OAuth applications

## Configuration Templates

### Complete Configuration Example

```yaml
# external-mcp-servers-oauth.yaml

# Global OAuth settings
globalOAuthConfig:
  default_oauth_termination_here: true
  default_scopes: ["mcp:read", "mcp:write"]
  enable_audit_logging: true
  token_refresh:
    enabled: true
    refresh_before_expiry_seconds: 300

# OAuth-enabled MCP servers
oauthMcpServers:
  # Production example with static credentials
  production_mcp:
    enabled: true
    base_url: "https://mcp.production.com"
    oauth_termination_here: false  # Forward to client
    
    enable_dynamic_registration: false
    static_credentials:
      client_id: "${PROD_MCP_CLIENT_ID}"
      client_secret: "${PROD_MCP_CLIENT_SECRET}"
      scopes: ["mcp:production", "mcp:audit"]
      authorization_endpoint: "https://auth.production.com/oauth/authorize"
      token_endpoint: "https://auth.production.com/oauth/token"
    
    transport:
      transport_type: "http"
      timeout_seconds: 60
    
    connection:
      pool_size: 20
      enable_http2: true
      headers:
        "X-Environment": "production"

  # Development example with dynamic registration  
  dev_mcp:
    enabled: true
    base_url: "https://mcp.dev.com"
    oauth_termination_here: true  # MT spawns browser
    
    enable_dynamic_registration: true
    registration_metadata:
      client_name: "MagicTunnel-Dev-{{hostname}}"
      requested_scopes: ["mcp:dev", "mcp:debug"]
      application_type: "web"
    
    transport:
      transport_type: "sse"
      timeout_seconds: 30
```

### Environment Variables

```bash
# Static OAuth credentials
export PROD_MCP_CLIENT_ID="your_production_client_id"
export PROD_MCP_CLIENT_SECRET="your_production_client_secret"

# MagicTunnel OAuth configuration
export MAGICTUNNEL_OAUTH_AUDIT_LOGGING=true
export MAGICTUNNEL_OAUTH_DEFAULT_TERMINATION=true
```

## Implementation Details

### 1. RFC-Compliant Discovery Flow

1. **Authorization Server Discovery** (RFC 8414):
   ```
   GET https://mcp.server.com/.well-known/oauth-authorization-server
   Response: {
     "authorization_endpoint": "...",
     "token_endpoint": "...",
     "scopes_supported": ["mcp:read", "mcp:write", "globalping:measurements"],
     "grant_types_supported": ["authorization_code", "refresh_token"],
     "response_types_supported": ["code"]
   }
   ```

2. **Protected Resource Discovery** (RFC 9728, optional):
   ```
   GET https://mcp.server.com/.well-known/oauth-protected-resource
   Response: {
     "resource": "https://mcp.server.com",
     "scopes_supported": ["mcp:read", "mcp:write"]
   }
   ```

3. **Scope Resolution Logic**:
   - Uses intersection of auth server + resource scopes (if both available)
   - Falls back to auth server scopes only
   - Applies manual overrides if specified
   - Validates overrides against discovered capabilities

4. **Dynamic Registration** (RFC 7591, if enabled):
   ```
   POST https://discovered-registration-endpoint/oauth/register
   Request: {
     "scopes": "discovered scopes joined by space",
     "grant_types": [discovered grant types],
     "response_types": [discovered response types]
   }
   ```

5. **Authorization Flow**:
   - Uses **discovered** authorization endpoint for browser redirect
   - Requests **resolved** scopes during authorization
   - If `oauth_termination_here: true`: MT spawns browser with discovered endpoint
   - If `oauth_termination_here: false`: Forward discovered metadata to client

6. **Token Exchange**:
   - Uses **discovered** token endpoint
   - PKCE-enhanced if supported by **discovered** `code_challenge_methods_supported`
   - Automatic refresh using **discovered** capabilities

### 2. Security Features

- **RFC 8414/9728 Compliance**: Automatic discovery prevents configuration errors
- **PKCE (RFC 7636)**: Enabled automatically when `code_challenge_methods_supported` includes `S256`
- **Resource Indicators (RFC 8707)**: Tokens bound to discovered resource servers
- **Scope Validation**: Manual overrides validated against discovered server capabilities
- **Secure Storage**: Credentials stored using MagicTunnel's platform-native auth infrastructure
- **Discovery Caching**: 1-hour TTL prevents excessive discovery requests
- **Configuration Hashing**: Detects server capability changes for cache invalidation
- **Comprehensive Audit Logging**: All discovery, registration, and token events logged

### 3. Audit Log Examples

```json
{
  "event": "oauth_discovery_attempt",
  "timestamp": "2024-01-01T12:00:00Z",
  "server_name": "globalping",
  "base_url": "https://mcp.globalping.dev",
  "action": "discovery_start"
}

{
  "event": "oauth_authorization_success", 
  "timestamp": "2024-01-01T12:05:00Z",
  "server_name": "globalping",
  "action": "authorization_success"
}

{
  "event": "oauth_token_usage",
  "timestamp": "2024-01-01T12:10:00Z", 
  "server_name": "globalping",
  "method": "POST",
  "endpoint": "/mcp/tools/call",
  "action": "api_call_authenticated"
}
```

## Migration Steps

### Step 1: Install Dependencies

Ensure you have the required dependencies:
```toml
# Cargo.toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
webbrowser = "0.8"
base64 = "0.21"
sha2 = "0.10"
rand = "0.8"
dirs = "5.0"
```

### Step 2: Create OAuth Configuration

1. Copy `external-mcp-servers.yaml` to `external-mcp-servers-oauth.yaml`
2. Add OAuth servers under `oauthMcpServers` section
3. Configure global OAuth settings in `globalOAuthConfig`

### Step 3: Update MagicTunnel Startup

Replace the traditional external manager with the OAuth-enhanced version:

```rust
use crate::mcp::OAuthExternalMcpManager;

// Replace ExternalMcpManager with OAuthExternalMcpManager
let oauth_mcp_manager = OAuthExternalMcpManager::new(
    external_config,
    oauth_config,
    client_config,
    multi_level_auth_config
).await?;

oauth_mcp_manager.start().await?;
```

### Step 4: Test OAuth Flow

1. Start MagicTunnel with OAuth configuration
2. For `oauth_termination_here: true`: Browser should open for authorization
3. For `oauth_termination_here: false`: OAuth details forwarded to client
4. Check audit logs for OAuth events
5. Verify tool execution with authenticated requests

## Troubleshooting

### Common Issues

1. **"Browser failed to open"**:
   - Ensure `webbrowser` crate can access system browser
   - For headless environments, use `oauth_termination_here: false`

2. **"Dynamic registration failed"**:
   - Check if server supports RFC 7591 dynamic registration
   - Verify `registration_endpoint` in server metadata
   - Use static credentials as fallback

3. **"Token validation failed"**:
   - Verify required scopes match server expectations
   - Check token expiration and refresh settings
   - Ensure correct audience binding for MCP server

4. **"Audit logs not appearing"**:
   - Enable `enable_audit_logging: true` in global config
   - Check tracing configuration for audit target
   - Verify structured logging settings

### Debug Commands

```bash
# Enable debug logging for OAuth flows
export RUST_LOG=magictunnel::mcp::oauth_discovery=debug

# Run MagicTunnel with OAuth debugging
./magictunnel-supervisor --config external-mcp-servers-oauth.yaml

# Check OAuth credential storage
ls ~/.magictunnel/sessions/oauth_*
```

## Benefits of Migration

### RFC Compliance & Configuration Simplification
- ✅ **RFC 8414/9728 Automatic Discovery** eliminates manual OAuth configuration guesswork
- ✅ **Zero-Config OAuth** - only server URL needed, all capabilities discovered
- ✅ **Server Capability Validation** prevents mismatched configuration errors
- ✅ **Smart Fallback** to manual config when discovery fails
- ✅ **Configuration Caching** with change detection for improved performance

### Security Improvements
- ✅ **Native OAuth 2.1** with automatically discovered PKCE support
- ✅ **Scope Intersection Logic** between auth server and resource server
- ✅ **Override Validation** ensures manual configs don't conflict with server capabilities
- ✅ **Secure credential storage** using platform-native keychain
- ✅ **Automatic token lifecycle** management with discovered refresh endpoints
- ✅ **Comprehensive audit logging** for all discovery and OAuth events

### Operational Benefits  
- ✅ **No external npm dependencies** (eliminates `npx mcp-remote` completely)
- ✅ **Unified authentication** with MagicTunnel's existing auth infrastructure
- ✅ **Flexible deployment options** via termination flag
- ✅ **Enterprise-ready** with static credential support + discovery validation
- ✅ **Automatic capability updates** when servers change their OAuth configuration

### Developer Experience
- ✅ **One-Line Configuration** - just specify `base_url` and let RFC discovery handle the rest
- ✅ **Automatic OAuth app registration** via RFC 7591 with discovered capabilities
- ✅ **Intelligent error messages** when discovery or validation fails
- ✅ **Rich audit logs** for debugging OAuth discovery and flow issues
- ✅ **Drop-in replacement** for existing MCP remote connections
- ✅ **Cache management tools** for testing and troubleshooting

## Next Steps

1. **Backup current configuration** before migration
2. **Test in development** environment first
3. **Configure OAuth applications** for production servers
4. **Set up monitoring** for OAuth audit logs  
5. **Train team** on new OAuth configuration options

## Troubleshooting Discovery Issues

### Discovery Validation
```bash
# Test RFC 8414 discovery endpoint
curl -H "Accept: application/json" https://mcp.server.com/.well-known/oauth-authorization-server

# Test RFC 9728 resource discovery (optional)
curl -H "Accept: application/json" https://mcp.server.com/.well-known/oauth-protected-resource

# Clear MagicTunnel OAuth discovery cache
# (Available programmatically via OAuthMcpDiscoveryManager::clear_oauth_cache())
```

### Common Discovery Issues
1. **"RFC 8414 discovery failed"**: Server doesn't support well-known endpoint - configure `manual_oauth_metadata` as fallback
2. **"No OAuth scopes resolved"**: Server metadata missing `scopes_supported` - use `required_scopes_override`
3. **"Manual override not in discovered scopes"**: Your override includes scopes the server doesn't support
4. **"Cache validation failed"**: Clear cache or wait for 1-hour TTL expiration

For questions or issues, refer to:
- OAuth enhancements document: `oauth_enhancements.md`
- MagicTunnel auth documentation: `docs/auth/`  
- Audit logging guide: `docs/security/audit-logging.md`
- RFC 8414 specification: https://tools.ietf.org/rfc/rfc8414.txt
- RFC 9728 specification: https://tools.ietf.org/rfc/rfc9728.txt