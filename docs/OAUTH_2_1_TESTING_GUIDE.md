# OAuth 2.1 End-to-End Testing Guide

## Overview

This guide provides comprehensive testing procedures for MagicTunnel's OAuth 2.1 authentication system in production-like environments.

## Table of Contents

1. [Test Environment Setup](#test-environment-setup)
2. [OAuth 2.1 Flow Testing](#oauth-21-flow-testing)
3. [Device Code Flow Testing](#device-code-flow-testing)
4. [Service Account Testing](#service-account-testing)
5. [Load Testing](#load-testing)
6. [Security Testing](#security-testing)
7. [Cross-Platform Testing](#cross-platform-testing)
8. [Production Readiness Checklist](#production-readiness-checklist)

## Test Environment Setup

### Prerequisites

1. **Real OAuth Providers**: GitHub, Google, Microsoft OAuth applications
2. **Test Accounts**: Dedicated test accounts for each provider
3. **Environment Variables**: Production-like configuration
4. **Network Access**: Internet connectivity for OAuth flows
5. **Multiple Platforms**: macOS, Linux, Windows test environments

### GitHub OAuth Application Setup

1. **Create GitHub OAuth App**:
   - Go to GitHub Settings → Developer settings → OAuth Apps
   - Create new OAuth App with these settings:
     - Application name: "MagicTunnel Test"
     - Homepage URL: "https://localhost:3001"
     - Authorization callback URL: "https://localhost:3001/oauth/callback"

2. **Create GitHub Device Flow App**:
   - Create second OAuth App for device flow testing
   - Enable device flow in application settings

3. **Generate Personal Access Token**:
   - Go to Settings → Personal access tokens → Fine-grained tokens
   - Create token with repo and user:email scopes

### Google OAuth Application Setup

1. **Create Google Cloud Project**:
   - Go to Google Cloud Console
   - Create new project: "magictunnel-test"
   - Enable APIs: Google Sheets API, Google Drive API

2. **Configure OAuth Consent Screen**:
   - Set application type: External
   - Add test users
   - Configure scopes: email, profile, spreadsheets

3. **Create OAuth 2.0 Credentials**:
   - Create OAuth 2.0 client ID
   - Application type: Web application
   - Authorized redirect URIs: "https://localhost:3001/oauth/callback"

### Environment Configuration

Create `.env.test` file:

```bash
# GitHub OAuth
GITHUB_CLIENT_ID="your_github_client_id"
GITHUB_CLIENT_SECRET="your_github_client_secret"

# GitHub Device Flow
GITHUB_DEVICE_CLIENT_ID="your_device_client_id"
GITHUB_DEVICE_CLIENT_SECRET="your_device_client_secret"

# Google OAuth
GOOGLE_CLIENT_ID="your_google_client_id"
GOOGLE_CLIENT_SECRET="your_google_client_secret"

# Service Accounts
GITHUB_PERSONAL_ACCESS_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"

# Test Configuration
MAGICTUNNEL_TEST_MODE="true"
MAGICTUNNEL_LOG_LEVEL="debug"
RUST_LOG="magictunnel::auth=debug"

# Session Storage
SESSION_ENCRYPTION_KEY="test_key_32_bytes_long_exactly!"
```

### Test Configuration File

Create `test-config.yaml`:

```yaml
deployment:
  runtime_mode: "advanced"

auth:
  enabled: true
  
  server_level:
    type: oauth
    provider: github
    scopes: ["user:email", "repo:read"]
  
  capabilities:
    google_workspace:
      type: oauth
      provider: google
      scopes: ["https://www.googleapis.com/auth/spreadsheets.readonly"]
    
    headless_tools:
      type: device_code
      provider: github_device
      scopes: ["user:email", "repo:read"]
  
  tools:
    admin_test:
      type: service_account
      account_ref: github_pat

oauth_providers:
  github:
    client_id: "${GITHUB_CLIENT_ID}"
    client_secret: "${GITHUB_CLIENT_SECRET}"
    oauth_enabled: true
    device_code_enabled: false
    scopes: ["user:email", "repo:read"]
    authorization_endpoint: "https://github.com/login/oauth/authorize"
    token_endpoint: "https://github.com/login/oauth/access_token"
    user_info_endpoint: "https://api.github.com/user"
  
  github_device:
    client_id: "${GITHUB_DEVICE_CLIENT_ID}"
    client_secret: "${GITHUB_DEVICE_CLIENT_SECRET}"
    oauth_enabled: false
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

service_accounts:
  github_pat:
    type: github_pat
    token: "${GITHUB_PERSONAL_ACCESS_TOKEN}"
    scopes: ["repo", "user:email"]

session_persistence:
  stdio:
    enabled: true
    storage_backend: "filesystem"
    storage_path: "./test-sessions"
    encryption_key: "${SESSION_ENCRYPTION_KEY}"
  
  token_management:
    refresh_threshold: "5m"
    max_token_lifetime: "1h"
    cleanup_expired_sessions: "10m"

smart_discovery:
  enabled: true
  mode: "hybrid"
```

## OAuth 2.1 Flow Testing

### Test Case 1: Complete OAuth Flow

**Objective**: Test full OAuth 2.1 authorization flow with PKCE

**Steps**:

1. **Start MagicTunnel**:
   ```bash
   source .env.test
   ./magictunnel --config test-config.yaml
   ```

2. **Trigger OAuth Flow**:
   ```bash
   curl -X POST http://localhost:3001/mcp/call \
     -H "Content-Type: application/json" \
     -d '{
       "name": "github_get_user",
       "arguments": {}
     }'
   ```

3. **Expected Response** (OAuth required):
   ```json
   {
     "jsonrpc": "2.0",
     "error": {
       "code": -32001,
       "message": "OAuth authorization required",
       "data": {
         "auth_type": "oauth",
         "provider": "github",
         "authorization_url": "https://github.com/login/oauth/authorize?client_id=...",
         "state": "...",
         "expires_in": 600
       }
     }
   }
   ```

4. **Complete Authorization**:
   - Visit authorization URL
   - Grant permissions
   - Get redirected with authorization code

5. **Retry Tool Call**:
   ```bash
   curl -X POST http://localhost:3001/mcp/call \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer {oauth_token}" \
     -d '{
       "name": "github_get_user",
       "arguments": {}
     }'
   ```

6. **Expected Result**: Successful API call with user data

**Verification Points**:
- ✅ PKCE challenge generated correctly
- ✅ State parameter prevents CSRF
- ✅ Authorization URL contains all required parameters
- ✅ Token exchange succeeds
- ✅ User info retrieved correctly
- ✅ Authentication context flows to tool execution

### Test Case 2: Token Refresh

**Objective**: Test automatic token refresh before expiry

**Steps**:

1. **Use Short-Lived Token** (modify provider config):
   ```yaml
   oauth_providers:
     github:
       # Override for testing - use short expiry
       token_expiry_override: 300 # 5 minutes
   ```

2. **Monitor Token Refresh**:
   ```bash
   # Watch logs for refresh events
   tail -f magictunnel.log | grep "token_refresh"
   ```

3. **Verify Refresh Behavior**:
   - Token refreshes automatically 5 minutes before expiry
   - No interruption to ongoing requests
   - New tokens stored correctly

**Verification Points**:
- ✅ Background refresh triggers correctly
- ✅ New tokens replace expired tokens
- ✅ Sessions persist through refresh
- ✅ No service interruption

### Test Case 3: Multiple Provider Flow

**Objective**: Test simultaneous OAuth with multiple providers

**Steps**:

1. **Configure Multiple Tools**:
   ```yaml
   tools:
     github_tool:
       type: oauth
       provider: github
     google_tool:
       type: oauth
       provider: google
   ```

2. **Trigger Both Flows**:
   ```bash
   # GitHub tool
   curl -X POST http://localhost:3001/mcp/call \
     -d '{"name": "github_tool", "arguments": {}}'
   
   # Google tool
   curl -X POST http://localhost:3001/mcp/call \
     -d '{"name": "google_tool", "arguments": {}}'
   ```

3. **Complete Both Authorizations**:
   - Authorize GitHub access
   - Authorize Google access

4. **Verify Independent Operation**:
   - Each provider maintains separate tokens
   - No cross-provider interference

**Verification Points**:
- ✅ Multiple OAuth flows work simultaneously
- ✅ Provider tokens stored independently
- ✅ Correct provider selected for each tool

## Device Code Flow Testing

### Test Case 1: Headless Environment

**Objective**: Test Device Code Flow in server environment

**Setup**:
```bash
# Simulate headless environment (no browser)
export DISPLAY=""
export MAGICTUNNEL_HEADLESS=true
```

**Steps**:

1. **Trigger Device Code Flow**:
   ```bash
   curl -X POST http://localhost:3001/mcp/call \
     -d '{
       "name": "headless_github_tool",
       "arguments": {}
     }'
   ```

2. **Expected Response**:
   ```json
   {
     "jsonrpc": "2.0",
     "error": {
       "code": -32001,
       "message": "Device authorization required",
       "data": {
         "auth_type": "device_code",
         "provider": "github_device",
         "user_code": "WDJB-MJHT",
         "verification_uri": "https://github.com/login/device",
         "expires_in": 1800,
         "interval": 5
       }
     }
   }
   ```

3. **Complete Device Authorization**:
   - On separate device, visit verification URI
   - Enter user code: WDJB-MJHT
   - Grant permissions

4. **Monitor Polling**:
   ```bash
   # Watch polling in logs
   grep "device_code_poll" magictunnel.log
   ```

5. **Verify Success**:
   - Polling continues until user authorization
   - Token retrieved successfully
   - Subsequent tool calls work without authorization

**Verification Points**:
- ✅ Device code generated correctly
- ✅ Polling interval respected
- ✅ User instructions clear and accurate
- ✅ Automatic token retrieval after authorization
- ✅ Works without browser on server

### Test Case 2: Device Code Timeout

**Objective**: Test device code expiration handling

**Steps**:

1. **Trigger Device Code Flow**:
   ```bash
   curl -X POST http://localhost:3001/mcp/call \
     -d '{"name": "headless_github_tool", "arguments": {}}'
   ```

2. **Do NOT Complete Authorization**:
   - Wait for device code to expire (15-30 minutes)
   - Monitor logs for expiration

3. **Verify Timeout Handling**:
   ```json
   {
     "jsonrpc": "2.0",
     "error": {
       "code": -32004,
       "message": "Device code expired",
       "data": {
         "provider": "github_device",
         "instructions": "Please restart the device authorization process"
       }
     }
   }
   ```

**Verification Points**:
- ✅ Polling stops after expiration
- ✅ Clear error message provided
- ✅ New device code flow can be initiated

### Test Case 3: Multiple Device Flows

**Objective**: Test concurrent device code flows

**Steps**:

1. **Start Multiple Flows Simultaneously**:
   ```bash
   # Terminal 1
   curl -X POST http://localhost:3001/mcp/call \
     -d '{"name": "headless_tool_1", "arguments": {}}'
   
   # Terminal 2
   curl -X POST http://localhost:3001/mcp/call \
     -d '{"name": "headless_tool_2", "arguments": {}}'
   ```

2. **Complete Each Flow Independently**:
   - Use different browser sessions
   - Complete each authorization separately

3. **Verify Independent Operation**:
   - Each flow maintains separate device codes
   - No interference between flows

**Verification Points**:
- ✅ Multiple device flows work concurrently
- ✅ Device codes remain unique
- ✅ Independent completion possible

## Service Account Testing

### Test Case 1: GitHub Personal Access Token

**Objective**: Test GitHub PAT validation and usage

**Steps**:

1. **Configure Service Account**:
   ```yaml
   service_accounts:
     github_pat:
       type: github_pat
       token: "${GITHUB_PERSONAL_ACCESS_TOKEN}"
       scopes: ["repo", "user:email"]
   ```

2. **Test Token Validation**:
   ```bash
   curl -X POST http://localhost:3001/mcp/auth/service-account/validate \
     -H "Content-Type: application/json" \
     -d '{
       "account_type": "github_pat",
       "token": "ghp_xxxxxxxxxxxxxxxxxxxx"
     }'
   ```

3. **Expected Response**:
   ```json
   {
     "valid": true,
     "user_info": {
       "id": "123456",
       "login": "testuser",
       "name": "Test User"
     },
     "permissions": ["repo", "user:email"],
     "account_type": "GitHubPAT"
   }
   ```

4. **Test Tool Execution**:
   ```bash
   curl -X POST http://localhost:3001/mcp/call \
     -d '{"name": "admin_test", "arguments": {"repo": "test/repo"}}'
   ```

**Verification Points**:
- ✅ PAT validation works correctly
- ✅ User information retrieved
- ✅ Permissions mapped correctly
- ✅ Tool execution uses PAT automatically

### Test Case 2: Invalid Service Account

**Objective**: Test handling of invalid service account credentials

**Steps**:

1. **Use Invalid Token**:
   ```bash
   curl -X POST http://localhost:3001/mcp/auth/service-account/validate \
     -d '{
       "account_type": "github_pat",
       "token": "invalid_token"
     }'
   ```

2. **Expected Error Response**:
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

**Verification Points**:
- ✅ Invalid tokens rejected
- ✅ Clear error messages
- ✅ No service disruption

## Load Testing

### Test Case 1: OAuth Flow Load

**Objective**: Test system under OAuth load

**Setup**:
```bash
# Install load testing tool
npm install -g artillery
```

**Artillery Configuration** (`oauth-load-test.yml`):
```yaml
config:
  target: 'http://localhost:3001'
  phases:
    - duration: 60
      arrivalRate: 10
  defaults:
    headers:
      Content-Type: 'application/json'

scenarios:
  - name: 'OAuth Authorization Flow'
    flow:
      - post:
          url: '/mcp/call'
          json:
            name: 'github_get_user'
            arguments: {}
      - think: 1
```

**Run Load Test**:
```bash
artillery run oauth-load-test.yml
```

**Expected Results**:
- 99% requests under 2 seconds
- No memory leaks
- Graceful error handling under load

### Test Case 2: Token Refresh Load

**Objective**: Test token refresh under load

**Steps**:

1. **Create Multiple Sessions**:
   ```bash
   # Script to create 100 sessions
   for i in {1..100}; do
     curl -X POST http://localhost:3001/mcp/call \
       -H "Session-ID: session_$i" \
       -d '{"name": "github_get_user", "arguments": {}}'
   done
   ```

2. **Trigger Mass Token Refresh**:
   ```bash
   # Simulate token expiry
   curl -X POST http://localhost:3001/admin/expire-tokens
   ```

3. **Monitor Refresh Performance**:
   ```bash
   # Watch refresh metrics
   curl http://localhost:3001/metrics | grep token_refresh
   ```

**Verification Points**:
- ✅ All tokens refresh successfully
- ✅ No performance degradation
- ✅ Resource usage remains stable

## Security Testing

### Test Case 1: PKCE Security

**Objective**: Verify PKCE implementation security

**Steps**:

1. **Intercept OAuth Flow**:
   ```bash
   # Start proxy to intercept requests
   mitmproxy -p 8080
   ```

2. **Verify PKCE Parameters**:
   - Authorization URL contains `code_challenge`
   - `code_challenge_method=S256`
   - Token exchange includes `code_verifier`

3. **Test PKCE Validation**:
   - Attempt token exchange with wrong verifier
   - Should fail with invalid_grant error

**Verification Points**:
- ✅ PKCE challenge generated securely
- ✅ Code verifier properly validated
- ✅ Attacks against PKCE prevented

### Test Case 2: State Parameter Security

**Objective**: Test CSRF protection via state parameter

**Steps**:

1. **Capture State Parameter**:
   ```bash
   # Extract state from authorization URL
   state=$(curl -s -X POST http://localhost:3001/mcp/call \
     -d '{"name": "github_get_user", "arguments": {}}' \
     | jq -r '.error.data.state')
   ```

2. **Test State Validation**:
   ```bash
   # Attempt callback with wrong state
   curl -X POST http://localhost:3001/oauth/callback \
     -d '{
       "code": "valid_auth_code",
       "state": "wrong_state"
     }'
   ```

3. **Expected Result**: Request rejected due to state mismatch

**Verification Points**:
- ✅ State parameter required
- ✅ State mismatch rejected
- ✅ CSRF attacks prevented

### Test Case 3: Token Storage Security

**Objective**: Verify secure token storage

**Steps**:

1. **Check Session File Permissions**:
   ```bash
   ls -la ~/.magictunnel/sessions/
   # Should show 0700 permissions (owner only)
   ```

2. **Verify Encryption**:
   ```bash
   # Session files should be encrypted
   hexdump -C ~/.magictunnel/sessions/session_*.json
   # Should show encrypted binary data, not plaintext
   ```

3. **Test Cross-User Access**:
   ```bash
   # Switch to different user
   sudo -u testuser ls ~/.magictunnel/sessions/
   # Should fail with permission denied
   ```

**Verification Points**:
- ✅ Session files encrypted
- ✅ Proper file permissions
- ✅ Cross-user access prevented

## Cross-Platform Testing

### macOS Testing

**Platform-Specific Features**:
- Keychain integration
- Security framework usage

**Test Steps**:

1. **Test Keychain Storage**:
   ```yaml
   session_persistence:
     stdio:
       storage_backend: "keychain"
   ```

2. **Verify Keychain Integration**:
   ```bash
   # Check Keychain entries
   security find-generic-password -s "magictunnel"
   ```

**Verification Points**:
- ✅ Tokens stored in Keychain
- ✅ Proper Keychain permissions
- ✅ Secure access control

### Windows Testing

**Platform-Specific Features**:
- Credential Manager integration
- Windows-specific storage

**Test Steps**:

1. **Test Credential Manager**:
   ```yaml
   session_persistence:
     stdio:
       storage_backend: "credential_manager"
   ```

2. **Verify Credential Storage**:
   ```powershell
   # Check Credential Manager
   cmdkey /list | findstr magictunnel
   ```

**Verification Points**:
- ✅ Credentials stored securely
- ✅ Windows integration working
- ✅ User isolation maintained

### Linux Testing

**Platform-Specific Features**:
- Secret Service API
- D-Bus integration

**Test Steps**:

1. **Test Secret Service**:
   ```yaml
   session_persistence:
     stdio:
       storage_backend: "secret_service"
   ```

2. **Verify Secret Storage**:
   ```bash
   # Check secret service
   secret-tool search service magictunnel
   ```

**Verification Points**:
- ✅ Secrets stored via D-Bus
- ✅ Proper GNOME/KDE integration
- ✅ User session isolation

## Production Readiness Checklist

### Security ✅
- [ ] HTTPS enforcement
- [ ] PKCE implementation verified
- [ ] State parameter validation
- [ ] Token encryption working
- [ ] Cross-user isolation tested
- [ ] Audit logging enabled

### Performance ✅
- [ ] Load testing passed (>100 concurrent users)
- [ ] Memory usage stable under load
- [ ] Token refresh performance acceptable
- [ ] Session recovery time < 5 seconds
- [ ] No resource leaks detected

### Reliability ✅
- [ ] OAuth flows work with all providers
- [ ] Device code flows work in headless environments
- [ ] Session persistence across restarts
- [ ] Graceful error handling
- [ ] Automatic token refresh
- [ ] Health monitoring operational

### Compatibility ✅
- [ ] macOS Keychain integration
- [ ] Windows Credential Manager integration
- [ ] Linux Secret Service integration
- [ ] Cross-platform session sharing
- [ ] MCP client compatibility

### Documentation ✅
- [ ] API documentation complete
- [ ] Configuration examples provided
- [ ] Troubleshooting guide available
- [ ] Security best practices documented
- [ ] Migration guide prepared

### Monitoring ✅
- [ ] Authentication metrics exposed
- [ ] Error rates monitored
- [ ] Performance metrics tracked
- [ ] Health check endpoints working
- [ ] Audit logs collected

## Test Automation

### Continuous Integration

**GitHub Actions Workflow** (`.github/workflows/oauth-test.yml`):

```yaml
name: OAuth Integration Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  oauth-tests:
    runs-on: ubuntu-latest
    
    services:
      redis:
        image: redis:latest
        ports:
          - 6379:6379
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    
    - name: Setup test environment
      env:
        GITHUB_CLIENT_ID: ${{ secrets.TEST_GITHUB_CLIENT_ID }}
        GITHUB_CLIENT_SECRET: ${{ secrets.TEST_GITHUB_CLIENT_SECRET }}
        GOOGLE_CLIENT_ID: ${{ secrets.TEST_GOOGLE_CLIENT_ID }}
        GOOGLE_CLIENT_SECRET: ${{ secrets.TEST_GOOGLE_CLIENT_SECRET }}
      run: |
        cp test-config.yaml.template test-config.yaml
        envsubst < test-config.yaml.template > test-config.yaml
    
    - name: Run OAuth integration tests
      run: |
        cargo test --test oauth2_1_phase6_integration_test -- --nocapture
        cargo test --test device_code_integration_test -- --nocapture
        cargo test --test service_account_integration_test -- --nocapture
    
    - name: Run load tests
      run: |
        npm install -g artillery
        artillery run oauth-load-test.yml
```

### Local Test Script

**`scripts/test-oauth-production.sh`**:

```bash
#!/bin/bash

set -e

echo "🔐 OAuth 2.1 Production Testing Suite"
echo "======================================"

# Check prerequisites
echo "📋 Checking prerequisites..."
if [ -z "$GITHUB_CLIENT_ID" ]; then
  echo "❌ GITHUB_CLIENT_ID not set"
  exit 1
fi

if [ -z "$GOOGLE_CLIENT_ID" ]; then
  echo "❌ GOOGLE_CLIENT_ID not set"
  exit 1
fi

echo "✅ Prerequisites met"

# Build in release mode
echo "🔨 Building MagicTunnel..."
cargo build --release

# Run integration tests
echo "🧪 Running integration tests..."
cargo test --test oauth2_1_phase6_integration_test -- --nocapture
cargo test --test device_code_integration_test -- --nocapture
cargo test --test service_account_integration_test -- --nocapture

# Start MagicTunnel in background
echo "🚀 Starting MagicTunnel..."
./target/release/magictunnel --config test-config.yaml &
MAGICTUNNEL_PID=$!

# Wait for startup
sleep 5

# Test OAuth flow
echo "🔐 Testing OAuth flow..."
curl -f -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"name": "github_get_user", "arguments": {}}'

# Test device code flow
echo "📱 Testing device code flow..."
curl -f -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"name": "headless_github_tool", "arguments": {}}'

# Run load tests
echo "⚡ Running load tests..."
artillery run oauth-load-test.yml

# Cleanup
echo "🧹 Cleaning up..."
kill $MAGICTUNNEL_PID

echo "✅ All tests passed!"
echo "🎉 OAuth 2.1 system is production ready!"
```

## Troubleshooting

### Common Issues

**OAuth Authorization Fails**:
```bash
# Check logs
tail -f magictunnel.log | grep oauth

# Verify configuration
./magictunnel --config test-config.yaml --validate-config

# Test provider connectivity
curl -v https://github.com/login/oauth/authorize
```

**Device Code Times Out**:
```bash
# Check device flow configuration
curl -X POST localhost:3001/admin/device-codes

# Monitor polling
tail -f magictunnel.log | grep device_code
```

**Session Recovery Fails**:
```bash
# Check session storage
ls -la ~/.magictunnel/sessions/

# Verify encryption key
echo $SESSION_ENCRYPTION_KEY | wc -c  # Should be 32 bytes
```

### Debug Commands

```bash
# Enable debug logging
export RUST_LOG=magictunnel::auth=debug

# Check authentication status
curl http://localhost:3001/health/auth

# View session information
curl http://localhost:3001/admin/sessions

# Test token refresh
curl -X POST http://localhost:3001/admin/refresh-tokens
```

## Performance Benchmarks

### Expected Performance Metrics

**OAuth Authorization Flow**:
- Time to authorization URL generation: < 100ms
- Token exchange time: < 500ms
- Session creation time: < 200ms

**Device Code Flow**:
- Device code generation: < 100ms
- Polling interval: 5 seconds
- Token retrieval after authorization: < 1 second

**Token Refresh**:
- Background refresh time: < 1 second
- Refresh failure recovery: < 5 seconds
- Mass refresh (100 tokens): < 10 seconds

**Session Recovery**:
- Startup session recovery: < 2 seconds
- Session validation: < 100ms per session
- Cross-platform session access: < 500ms

These benchmarks ensure MagicTunnel's OAuth system performs well in production environments.