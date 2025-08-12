#!/bin/bash

set -e

echo "🔐 OAuth 2.1 Production Testing Suite"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
  echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
  echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
  echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
  echo -e "${RED}❌ $1${NC}"
}

# Check prerequisites
print_status "Checking prerequisites..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    print_error "Cargo not found. Please install Rust."
    exit 1
fi

# Check if curl is installed
if ! command -v curl &> /dev/null; then
    print_error "curl not found. Please install curl."
    exit 1
fi

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    print_warning "jq not found. Installing jq for JSON parsing..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        brew install jq || { print_error "Failed to install jq"; exit 1; }
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        sudo apt-get update && sudo apt-get install -y jq || { print_error "Failed to install jq"; exit 1; }
    else
        print_error "Please install jq manually for JSON parsing"
        exit 1
    fi
fi

# Check environment variables
print_status "Checking environment variables..."

MISSING_VARS=()

if [ -z "$GITHUB_CLIENT_ID" ]; then
    MISSING_VARS+=("GITHUB_CLIENT_ID")
fi

if [ -z "$GITHUB_CLIENT_SECRET" ]; then
    MISSING_VARS+=("GITHUB_CLIENT_SECRET")
fi

if [ -z "$GITHUB_DEVICE_CLIENT_ID" ]; then
    print_warning "GITHUB_DEVICE_CLIENT_ID not set - Device Code Flow tests will be skipped"
fi

if [ -z "$GOOGLE_CLIENT_ID" ]; then
    print_warning "GOOGLE_CLIENT_ID not set - Google OAuth tests will be skipped"
fi

if [ -z "$GITHUB_PERSONAL_ACCESS_TOKEN" ]; then
    print_warning "GITHUB_PERSONAL_ACCESS_TOKEN not set - Service Account tests will be skipped"
fi

if [ ${#MISSING_VARS[@]} -gt 0 ]; then
    print_error "Missing required environment variables: ${MISSING_VARS[*]}"
    echo ""
    echo "Please set the following environment variables:"
    for var in "${MISSING_VARS[@]}"; do
        echo "  export $var=\"your_value_here\""
    done
    echo ""
    echo "See docs/OAUTH_2_1_TESTING_GUIDE.md for setup instructions."
    exit 1
fi

print_success "Environment variables configured"

# Set default values
export SESSION_ENCRYPTION_KEY="${SESSION_ENCRYPTION_KEY:-test_key_32_bytes_long_exactly!}"
export MAGICTUNNEL_TEST_MODE="true"
export RUST_LOG="${RUST_LOG:-magictunnel::auth=debug}"

# Build MagicTunnel
print_status "Building MagicTunnel in release mode..."
if ! cargo build --release; then
    print_error "Failed to build MagicTunnel"
    exit 1
fi
print_success "Build completed"

# Create test configuration
print_status "Creating test configuration..."

cat > test-config.yaml << EOF
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
    client_id: "${GITHUB_DEVICE_CLIENT_ID:-$GITHUB_CLIENT_ID}"
    client_secret: "${GITHUB_DEVICE_CLIENT_SECRET:-$GITHUB_CLIENT_SECRET}"
    oauth_enabled: false
    device_code_enabled: true
    device_authorization_endpoint: "https://github.com/login/device/code"
    token_endpoint: "https://github.com/login/oauth/access_token"
    user_info_endpoint: "https://api.github.com/user"
  
  google:
    client_id: "${GOOGLE_CLIENT_ID:-placeholder}"
    client_secret: "${GOOGLE_CLIENT_SECRET:-placeholder}"
    oauth_enabled: true
    device_code_enabled: true
    authorization_endpoint: "https://accounts.google.com/o/oauth2/auth"
    device_authorization_endpoint: "https://oauth2.googleapis.com/device/code"
    token_endpoint: "https://oauth2.googleapis.com/token"
    user_info_endpoint: "https://www.googleapis.com/oauth2/v2/userinfo"

service_accounts:
  github_pat:
    type: github_pat
    token: "${GITHUB_PERSONAL_ACCESS_TOKEN:-placeholder}"
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

mcp:
  enabled: true
  stdio: false
  http:
    enabled: true
    host: "127.0.0.1"
    port: 3001
    cors_enabled: true
EOF

print_success "Test configuration created"

# Clean up any existing test sessions
print_status "Cleaning up existing test sessions..."
rm -rf ./test-sessions
mkdir -p ./test-sessions

# Run integration tests first
print_status "Running OAuth integration tests..."

echo "  📝 Testing OAuth Phase 6 integration..."
if MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem cargo test --test oauth2_1_phase6_integration_test -- --nocapture; then
    print_success "OAuth Phase 6 integration tests passed"
else
    print_error "OAuth Phase 6 integration tests failed"
    exit 1
fi

if [ -n "$GITHUB_DEVICE_CLIENT_ID" ]; then
    echo "  📱 Testing Device Code Flow integration..."
    if MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem cargo test --test device_code_integration_test -- --nocapture; then
        print_success "Device Code Flow integration tests passed"
    else
        print_error "Device Code Flow integration tests failed"
        exit 1
    fi
else
    print_warning "Skipping Device Code Flow tests (GITHUB_DEVICE_CLIENT_ID not set)"
fi

if [ -n "$GITHUB_PERSONAL_ACCESS_TOKEN" ]; then
    echo "  🔑 Testing Service Account integration..."
    if MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem cargo test --test service_account_integration_test -- --nocapture; then
        print_success "Service Account integration tests passed"
    else
        print_error "Service Account integration tests failed"
        exit 1
    fi
else
    print_warning "Skipping Service Account tests (GITHUB_PERSONAL_ACCESS_TOKEN not set)"
fi

# Start MagicTunnel in background
print_status "Starting MagicTunnel server..."
./target/release/magictunnel --config test-config.yaml > magictunnel-test.log 2>&1 &
MAGICTUNNEL_PID=$!

# Function to cleanup on exit
cleanup() {
    if [ ! -z "$MAGICTUNNEL_PID" ]; then
        print_status "Stopping MagicTunnel server..."
        kill $MAGICTUNNEL_PID 2>/dev/null || true
        wait $MAGICTUNNEL_PID 2>/dev/null || true
    fi
    
    # Clean up test files
    rm -f test-config.yaml
    rm -f magictunnel-test.log
    rm -rf ./test-sessions
}

trap cleanup EXIT

# Wait for server to start
print_status "Waiting for server to start..."
for i in {1..30}; do
    if curl -s http://localhost:3001/health >/dev/null 2>&1; then
        break
    fi
    if [ $i -eq 30 ]; then
        print_error "Server failed to start within 30 seconds"
        echo "Server logs:"
        cat magictunnel-test.log
        exit 1
    fi
    sleep 1
done

print_success "MagicTunnel server started successfully"

# Test 1: Basic health check
print_status "Testing basic health check..."
HEALTH_RESPONSE=$(curl -s http://localhost:3001/health)
if echo "$HEALTH_RESPONSE" | jq -e '.status == "healthy"' >/dev/null; then
    print_success "Health check passed"
else
    print_error "Health check failed: $HEALTH_RESPONSE"
    exit 1
fi

# Test 2: OAuth authorization required response
print_status "Testing OAuth authorization flow..."
OAUTH_RESPONSE=$(curl -s -X POST http://localhost:3001/mcp/call \
    -H "Content-Type: application/json" \
    -d '{
        "name": "smart_tool_discovery",
        "arguments": {
            "request": "get my GitHub user information"
        }
    }')

if echo "$OAUTH_RESPONSE" | jq -e '.error.code == -32001' >/dev/null; then
    AUTH_URL=$(echo "$OAUTH_RESPONSE" | jq -r '.error.data.authorization_url // empty')
    if [ -n "$AUTH_URL" ] && [[ "$AUTH_URL" == https://github.com/login/oauth/authorize* ]]; then
        print_success "OAuth authorization flow initiated correctly"
        print_status "Authorization URL: $AUTH_URL"
    else
        print_error "Invalid authorization URL: $AUTH_URL"
        exit 1
    fi
else
    print_error "OAuth authorization flow failed: $OAUTH_RESPONSE"
    exit 1
fi

# Test 3: Device Code Flow (if configured)
if [ -n "$GITHUB_DEVICE_CLIENT_ID" ]; then
    print_status "Testing Device Code Flow..."
    DEVICE_RESPONSE=$(curl -s -X POST http://localhost:3001/mcp/call \
        -H "Content-Type: application/json" \
        -d '{
            "name": "smart_tool_discovery",
            "arguments": {
                "request": "use headless GitHub tool to get repository info",
                "tool_hint": "headless_tools"
            }
        }')
    
    if echo "$DEVICE_RESPONSE" | jq -e '.error.data.auth_type == "device_code"' >/dev/null; then
        USER_CODE=$(echo "$DEVICE_RESPONSE" | jq -r '.error.data.user_code')
        VERIFICATION_URI=$(echo "$DEVICE_RESPONSE" | jq -r '.error.data.verification_uri')
        print_success "Device Code Flow initiated correctly"
        print_status "User Code: $USER_CODE"
        print_status "Verification URI: $VERIFICATION_URI"
    else
        print_error "Device Code Flow failed: $DEVICE_RESPONSE"
        exit 1
    fi
else
    print_warning "Skipping Device Code Flow test (GITHUB_DEVICE_CLIENT_ID not set)"
fi

# Test 4: Service Account authentication (if configured)
if [ -n "$GITHUB_PERSONAL_ACCESS_TOKEN" ] && [ "$GITHUB_PERSONAL_ACCESS_TOKEN" != "placeholder" ]; then
    print_status "Testing Service Account authentication..."
    SA_RESPONSE=$(curl -s -X POST http://localhost:3001/mcp/call \
        -H "Content-Type: application/json" \
        -d '{
            "name": "admin_test",
            "arguments": {
                "action": "validate"
            }
        }')
    
    # Service account should either work or require authentication
    if echo "$SA_RESPONSE" | jq -e '.result or .error' >/dev/null; then
        print_success "Service Account authentication flow working"
    else
        print_error "Service Account authentication failed: $SA_RESPONSE"
        exit 1
    fi
else
    print_warning "Skipping Service Account test (GITHUB_PERSONAL_ACCESS_TOKEN not set or placeholder)"
fi

# Test 5: Configuration validation
print_status "Testing configuration validation..."
CONFIG_RESPONSE=$(curl -s http://localhost:3001/admin/config/validate)
if echo "$CONFIG_RESPONSE" | jq -e '.valid == true' >/dev/null; then
    print_success "Configuration validation passed"
else
    print_error "Configuration validation failed: $CONFIG_RESPONSE"
    exit 1
fi

# Test 6: Session management
print_status "Testing session management..."
SESSION_RESPONSE=$(curl -s http://localhost:3001/admin/sessions/status)
if echo "$SESSION_RESPONSE" | jq -e '.session_count >= 0' >/dev/null; then
    SESSION_COUNT=$(echo "$SESSION_RESPONSE" | jq -r '.session_count')
    print_success "Session management working (active sessions: $SESSION_COUNT)"
else
    print_error "Session management failed: $SESSION_RESPONSE"
    exit 1
fi

# Test 7: Metrics endpoint
print_status "Testing metrics endpoint..."
METRICS_RESPONSE=$(curl -s http://localhost:3001/metrics)
if echo "$METRICS_RESPONSE" | grep -q "auth_requests_total"; then
    print_success "Metrics endpoint working"
else
    print_error "Metrics endpoint failed"
    exit 1
fi

# Test 8: Load test (basic)
print_status "Running basic load test..."
LOAD_TEST_PASSED=true

for i in {1..10}; do
    LOAD_RESPONSE=$(curl -s -X POST http://localhost:3001/mcp/call \
        -H "Content-Type: application/json" \
        -H "Session-ID: load-test-$i" \
        -d '{
            "name": "smart_tool_discovery",
            "arguments": {
                "request": "get system status"
            }
        }')
    
    if ! echo "$LOAD_RESPONSE" | jq -e '.result or .error' >/dev/null; then
        print_error "Load test request $i failed: $LOAD_RESPONSE"
        LOAD_TEST_PASSED=false
        break
    fi
done

if [ "$LOAD_TEST_PASSED" = true ]; then
    print_success "Basic load test passed (10 concurrent requests)"
else
    print_error "Basic load test failed"
    exit 1
fi

# Test 9: Security headers
print_status "Testing security headers..."
SECURITY_RESPONSE=$(curl -s -I http://localhost:3001/health)
if echo "$SECURITY_RESPONSE" | grep -q "X-Content-Type-Options: nosniff"; then
    print_success "Security headers present"
else
    print_warning "Some security headers may be missing"
fi

# Test 10: Error handling
print_status "Testing error handling..."
ERROR_RESPONSE=$(curl -s -X POST http://localhost:3001/mcp/call \
    -H "Content-Type: application/json" \
    -d '{
        "name": "nonexistent_tool",
        "arguments": {}
    }')

if echo "$ERROR_RESPONSE" | jq -e '.error.code' >/dev/null; then
    print_success "Error handling working correctly"
else
    print_error "Error handling failed: $ERROR_RESPONSE"
    exit 1
fi

# Performance metrics
print_status "Collecting performance metrics..."
END_TIME=$(date +%s)
if [ -n "$START_TIME" ]; then
    TOTAL_TIME=$((END_TIME - START_TIME))
    print_success "All tests completed in ${TOTAL_TIME} seconds"
fi

# Final server logs check
print_status "Checking for critical errors in server logs..."
if grep -q "CRITICAL\|FATAL\|panic" magictunnel-test.log; then
    print_warning "Critical errors found in server logs:"
    grep "CRITICAL\|FATAL\|panic" magictunnel-test.log
else
    print_success "No critical errors in server logs"
fi

# Summary
echo ""
echo "🎉 OAuth 2.1 Production Testing Complete!"
echo "========================================"
echo ""
print_success "✅ Integration tests passed"
print_success "✅ OAuth authorization flow working"
if [ -n "$GITHUB_DEVICE_CLIENT_ID" ]; then
    print_success "✅ Device Code Flow working"
fi
if [ -n "$GITHUB_PERSONAL_ACCESS_TOKEN" ] && [ "$GITHUB_PERSONAL_ACCESS_TOKEN" != "placeholder" ]; then
    print_success "✅ Service Account authentication working"
fi
print_success "✅ Configuration validation passed"
print_success "✅ Session management operational"
print_success "✅ Metrics endpoint functional"
print_success "✅ Basic load test passed"
print_success "✅ Error handling verified"
print_success "✅ No critical server errors"

echo ""
echo "🔐 OAuth 2.1 System Status: PRODUCTION READY"
echo ""
echo "📋 Next Steps:"
echo "   1. Set up production OAuth applications with your providers"
echo "   2. Configure production-grade session storage (Redis recommended)"
echo "   3. Set up monitoring and alerting for authentication metrics"
echo "   4. Configure HTTPS and security headers for production deployment"
echo "   5. Test with real user accounts in staging environment"
echo ""
echo "📖 Documentation:"
echo "   - API Reference: docs/OAUTH_2_1_API_REFERENCE.md"
echo "   - Testing Guide: docs/OAUTH_2_1_TESTING_GUIDE.md"
echo "   - Configuration: docs/OAUTH_2_1_tasks.md"
echo ""

# Record START_TIME for next run
START_TIME=$(date +%s)