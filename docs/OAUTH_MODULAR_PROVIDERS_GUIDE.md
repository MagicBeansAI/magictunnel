# OAuth Modular Providers Guide for MagicTunnel

## Overview

MagicTunnel implements a comprehensive **Modular OAuth Provider System** that supports 9+ major authentication providers with a unified interface. This system combines **OAuth 2.1 compliance**, **OIDC standards**, and **provider-specific optimizations** under a single, type-safe API.

**Status**: ✅ **FUNCTIONALLY COMPLETE & PRODUCTION-READY** (Version 0.3.16+)

## Table of Contents

1. [Supported Providers](#supported-providers)
2. [Architecture Overview](#architecture-overview)
3. [Configuration Examples](#configuration-examples)
4. [Provider-Specific Features](#provider-specific-features)
5. [Migration from Legacy OAuth](#migration-from-legacy-oauth)
6. [API Reference](#api-reference)
7. [Production Deployment](#production-deployment)
8. [Troubleshooting](#troubleshooting)

## Supported Providers

### ✅ **Enterprise Identity Providers**
| Provider | Protocol | Auto-Discovery | PKCE | Refresh | Custom Features |
|----------|----------|----------------|------|---------|-----------------|
| **Auth0** | OIDC | ✅ | ✅ | ✅ | Audience, Connection, Namespace |
| **Clerk** | OIDC | ✅ | ✅ | ✅ | Organizations, Sessions |
| **SuperTokens** | OIDC | ✅ | ✅ | ✅ | Recipes, API Paths |
| **Keycloak** | OIDC | ✅ | ✅ | ✅ | Realms, Admin API, Roles |

### ✅ **Major Cloud Providers**
| Provider | Protocol | Auto-Discovery | PKCE | Refresh | Custom Features |
|----------|----------|----------------|------|---------|-----------------|
| **Google** | OIDC | ✅ | ✅ | ✅ | Workspace Domains, API Scopes |
| **Microsoft** | OIDC | ✅ | ✅ | ✅ | Graph API, Tenants |
| **Apple** | OIDC | ✅ | ❌ | ✅ | JWT Client Assertions, P8 Keys |
| **GitHub** | OAuth 2.0 | ❌ | ❌ | ❌ | Repository Scopes, Enterprise |

### ✅ **Generic Support**
| Provider | Protocol | Auto-Discovery | PKCE | Refresh | Custom Features |
|----------|----------|----------------|------|---------|-----------------|
| **Generic OIDC** | OIDC | ✅ | ✅ | ✅ | Any OIDC-compliant provider |

## Architecture Overview

### **Three-Tier Protocol Support**

1. **Pure OAuth 2.1 Providers**
   - GitHub: Simple OAuth 2.0 with repository scopes
   - Custom endpoints, no auto-discovery

2. **Standard OIDC Providers**
   - Auth0, Clerk, SuperTokens, Keycloak, Google, Microsoft
   - Auto-discovery via `/.well-known/openid-configuration`
   - JWT ID tokens with standard claims

3. **OIDC with Custom Authentication**
   - Apple: OIDC discovery + JWT client assertions
   - P8 private key signing instead of client secrets

### **Unified OAuth Interface**

```rust
// Same interface for all providers
trait OAuthProvider {
    async fn get_authorization_url(&self, scopes: &[String], redirect_uri: &str) -> Result<AuthorizationUrl>;
    async fn exchange_code_for_token(&self, code: &str, redirect_uri: &str, state: &str, code_verifier: Option<&str>) -> Result<TokenSet>;
    async fn get_user_info(&self, access_token: &str) -> Result<UserInfo>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenSet>;
    async fn validate_token(&self, access_token: &str) -> Result<TokenValidation>;
    async fn revoke_token(&self, access_token: &str) -> Result<()>;
}
```

## Configuration Examples

### **Google Workspace Integration**

```yaml
oauth_providers:
  google:
    type: google
    client_id: "your-app.apps.googleusercontent.com"
    client_secret: "GOCSPX-your-secret"
    hosted_domain: "company.com"  # Restrict to Workspace domain
    enable_offline_access: true
    prompt: "consent"
    access_type: "offline"
    scopes:
      - "openid"
      - "profile" 
      - "email"
      - "https://www.googleapis.com/auth/drive"
      - "https://www.googleapis.com/auth/calendar"
      - "https://www.googleapis.com/auth/gmail.readonly"
```

### **Microsoft Azure AD / Office 365**

```yaml
oauth_providers:
  microsoft:
    type: microsoft
    tenant_id: "common"  # or specific tenant ID
    client_id: "your-azure-app-id"
    client_secret: "your-azure-secret"
    domain_hint: "contoso.com"  # Optional domain hint
    prompt: "select_account"
    response_mode: "query"
    scopes:
      - "openid"
      - "profile"
      - "email"
      - "https://graph.microsoft.com/User.Read"
      - "https://graph.microsoft.com/Mail.Read"
      - "https://graph.microsoft.com/Calendars.Read"
      - "https://graph.microsoft.com/Files.Read"
```

### **Apple Sign In**

```yaml
oauth_providers:
  apple:
    type: apple
    client_id: "com.yourcompany.yourapp"
    team_id: "TEAM123456"
    key_id: "KEY123456"
    private_key: |
      -----BEGIN PRIVATE KEY-----
      MIGTAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBHkwdwIBAQQg...
      -----END PRIVATE KEY-----
    response_mode: "form_post"
    request_user_info: true
    scopes:
      - "name"
      - "email"
```

### **GitHub Enterprise**

```yaml
oauth_providers:
  github:
    type: github
    client_id: "your-github-app-id"
    client_secret: "your-github-secret"
    enterprise_url: "https://github.company.com"  # Optional for Enterprise
    scopes:
      - "user:email"
      - "read:user"
      - "repo"
      - "read:org"
```

### **Auth0 with Custom Domain**

```yaml
oauth_providers:
  auth0:
    type: auth0
    domain: "your-tenant.auth0.com"
    client_id: "your-auth0-client-id"
    client_secret: "your-auth0-secret"
    audience: "https://api.yourcompany.com"
    connection: "Username-Password-Authentication"
    namespace: "https://yourcompany.com/claims/"
    scopes:
      - "openid"
      - "profile"
      - "email"
      - "read:current_user"
```

### **Keycloak with Realm**

```yaml
oauth_providers:
  keycloak:
    type: keycloak
    server_url: "https://auth.company.com"
    realm: "company-realm"
    client_id: "magictunnel-client"
    client_secret: "keycloak-secret"
    enable_role_mapping: true
    admin_client_id: "admin-client"  # Optional for admin API
    admin_client_secret: "admin-secret"
    scopes:
      - "openid"
      - "profile"
      - "email"
      - "roles"
```

## Provider-Specific Features

### **Google-Specific Features**

- **Workspace Domain Restriction**: `hosted_domain: "company.com"`
- **Comprehensive API Scopes**: Drive, Gmail, Calendar, Sheets, Docs, Cloud Platform
- **Offline Access**: Automatic refresh token handling
- **Token Validation**: Google's tokeninfo endpoint

```yaml
google_advanced_scopes:
  - "https://www.googleapis.com/auth/cloud-platform"
  - "https://www.googleapis.com/auth/spreadsheets"
  - "https://www.googleapis.com/auth/documents"
  - "https://www.googleapis.com/auth/presentations"
```

### **Microsoft-Specific Features**

- **Multi-Tenant Support**: Tenant ID configuration
- **Graph API Integration**: Full Microsoft 365 access
- **Domain Hints**: Faster authentication for known domains
- **Enhanced User Info**: Rich profile data from Graph API

```yaml
microsoft_graph_scopes:
  - "https://graph.microsoft.com/Sites.Read.All"
  - "https://graph.microsoft.com/TeamSettings.Read.All"
  - "https://graph.microsoft.com/Directory.Read.All"
```

### **Apple-Specific Features**

- **JWT Client Assertions**: P8 private key authentication
- **Minimal Data Model**: Only name and email scopes
- **Form POST Response**: Secure token delivery
- **Team/Key Management**: Enterprise developer account integration

### **GitHub-Specific Features**

- **Repository Scopes**: Fine-grained repository access
- **Organization Support**: Enterprise organization management
- **Enterprise Server**: Support for GitHub Enterprise deployments
- **No Token Expiry**: GitHub tokens don't expire (until revoked)

## Migration from Legacy OAuth

### **Automatic Migration**

The system automatically converts legacy OAuth configurations:

```yaml
# Legacy OAuth (still supported)
auth:
  type: oauth
  oauth:
    provider: "google"
    client_id: "your-client-id"
    client_secret: "your-secret"
    auth_url: "https://accounts.google.com/o/oauth2/auth"
    token_url: "https://oauth2.googleapis.com/token"

# ↓ Automatically converts to ↓

# New Modular Provider (recommended)
oauth_providers:
  google:
    type: google
    client_id: "your-client-id"
    client_secret: "your-secret"
    # Plus all Google-specific optimizations
```

### **Migration Benefits**

1. **Provider-Specific Optimizations**: Access to enhanced features
2. **Better Error Handling**: Provider-specific error messages
3. **Enhanced Security**: OAuth 2.1 compliance, PKCE support
4. **Improved User Experience**: Provider-specific UX optimizations

## API Reference

### **Provider Manager API**

```rust
// Initialize provider manager
let manager = ProviderManager::new(config).await?;

// Start OAuth flow
let auth_url = manager.start_authorization("google", &scopes, None).await?;

// Complete OAuth flow
let token_set = manager.complete_authorization("google", code, state, code_verifier).await?;

// Get user information
let user_info = manager.get_user_info("google", &access_token).await?;

// Refresh tokens
let new_tokens = manager.refresh_token("google", &refresh_token).await?;

// Validate tokens
let validation = manager.validate_token("google", &access_token).await?;

// Revoke tokens
manager.revoke_token("google", &access_token).await?;
```

### **Session Management API**

```rust
// Create OAuth session
let session = OAuthSession::new(provider_id, token_set, user_info);

// Check expiration
if session.is_expired() || session.needs_refresh() {
    // Handle refresh
}

// Update activity
session.update_activity();
```

### **Unified OAuth System API**

```rust
// Create unified system
let oauth_system = UnifiedOAuthSystem::new(auth_config, user_context).await?;

// Validate HTTP requests
let result = oauth_system.validate_request(&http_request).await?;

// Get active sessions
let sessions = oauth_system.get_active_sessions().await;

// Revoke session
oauth_system.revoke_session(&access_token).await?;
```

## Production Deployment

### **Environment Variables**

```bash
# Provider credentials
export GOOGLE_CLIENT_ID="your-google-client-id"
export GOOGLE_CLIENT_SECRET="your-google-secret"
export MICROSOFT_CLIENT_ID="your-azure-app-id"
export MICROSOFT_CLIENT_SECRET="your-azure-secret"
export APPLE_PRIVATE_KEY="$(cat apple-key.p8)"

# Configuration
export MAGICTUNNEL_OAUTH_REDIRECT_BASE="https://your-domain.com"
export MAGICTUNNEL_OAUTH_DEFAULT_PROVIDER="google"
```

### **Security Considerations**

1. **HTTPS Required**: All OAuth flows require HTTPS in production
2. **Redirect URI Validation**: Strict validation of redirect URIs
3. **State Parameter**: CSRF protection via state parameter validation
4. **PKCE**: Required for public clients, recommended for confidential
5. **Token Storage**: Secure token storage with encryption
6. **Session Isolation**: Per-deployment session isolation

### **Load Balancing**

```yaml
# Session persistence across instances
session_storage:
  type: "redis"  # or "database"
  connection: "redis://localhost:6379"
  encryption_key: "your-session-encryption-key"
```

### **Monitoring**

```yaml
# OAuth metrics and logging
oauth_monitoring:
  enable_metrics: true
  log_successful_auths: true
  log_failed_auths: true
  track_provider_performance: true
  alert_on_high_failure_rate: true
```

## Troubleshooting

### **Common Issues**

#### **Invalid Client Error**
```
Error: Invalid client credentials
```
**Solution**: Verify `client_id` and `client_secret` in provider dashboard

#### **Redirect URI Mismatch**
```
Error: Redirect URI mismatch
```
**Solution**: Ensure redirect URI in config matches provider settings

#### **Scope Not Granted**
```
Error: Insufficient permissions for requested scope
```
**Solution**: Check scope availability and user consent

#### **Token Expired**
```
Error: Access token has expired
```
**Solution**: Implement automatic token refresh

### **Provider-Specific Issues**

#### **Google Workspace Domain Restriction**
```yaml
# Ensure domain restriction is properly configured
google:
  hosted_domain: "company.com"  # Must match user's domain
```

#### **Apple P8 Key Format**
```yaml
# Ensure P8 key is properly formatted
apple:
  private_key: |
    -----BEGIN PRIVATE KEY-----
    [Base64 encoded key content]
    -----END PRIVATE KEY-----
```

#### **Microsoft Tenant Configuration**
```yaml
# Use specific tenant ID for single-tenant apps
microsoft:
  tenant_id: "12345678-1234-1234-1234-123456789012"  # Not "common"
```

### **Debug Mode**

```bash
# Enable OAuth debug logging
export RUST_LOG=magictunnel::auth::oauth_providers=debug
./magictunnel-supervisor
```

### **Provider Health Checks**

```rust
// Check provider availability
let health = manager.check_provider_health("google").await?;
if !health.is_healthy {
    println!("Provider issue: {}", health.error_message);
}
```

## Conclusion

The MagicTunnel Modular OAuth Provider System offers **enterprise-grade authentication** with support for major cloud providers while maintaining **flexibility** for custom deployments. The unified interface ensures **consistent behavior** across all providers while leveraging **provider-specific optimizations** for the best user experience.

For additional support, see:
- [OAuth 2.1 Production Readiness Guide](./OAUTH_2_1_PRODUCTION_READINESS.md)
- [OAuth 2.1 Testing Guide](./OAUTH_2_1_TESTING_GUIDE.md)
- [Authentication Configuration](./AUTHENTICATION.md)