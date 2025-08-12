#!/bin/bash

set -e

echo "🔧 Setting up OAuth 2.1 Production Testing Environment"
echo "====================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# Check if we're in the MagicTunnel directory
if [ ! -f "Cargo.toml" ] || ! grep -q "magictunnel" Cargo.toml; then
    print_error "This script must be run from the MagicTunnel root directory"
    exit 1
fi

# Create necessary directories
print_status "Creating test directories..."
mkdir -p test-environments/{local,docker,staging}
mkdir -p test-environments/configs
mkdir -p test-environments/certs
mkdir -p test-environments/logs
mkdir -p test-environments/data

print_success "Test directories created"

# Install testing dependencies
print_status "Installing testing dependencies..."

# Check and install Node.js/npm for Artillery
if ! command -v node &> /dev/null; then
    print_warning "Node.js not found. Please install Node.js to use Artillery load testing."
    print_status "Visit: https://nodejs.org/ or use your package manager"
else
    NODE_VERSION=$(node --version)
    print_success "Node.js found: $NODE_VERSION"
    
    # Install Artillery if not present
    if ! command -v artillery &> /dev/null; then
        print_status "Installing Artillery for load testing..."
        npm install -g artillery || {
            print_warning "Failed to install Artillery globally. Installing locally..."
            npm install artillery
        }
    fi
    
    if command -v artillery &> /dev/null; then
        ARTILLERY_VERSION=$(artillery version)
        print_success "Artillery installed: $ARTILLERY_VERSION"
    fi
fi

# Check and install Docker
if ! command -v docker &> /dev/null; then
    print_warning "Docker not found. Docker testing will be skipped."
    print_status "Visit: https://docs.docker.com/get-docker/"
else
    DOCKER_VERSION=$(docker --version)
    print_success "Docker found: $DOCKER_VERSION"
fi

# Create local test environment configuration
print_status "Creating local test environment configuration..."

cat > test-environments/configs/local.yaml << 'EOF'
# Local Test Environment Configuration
deployment:
  runtime_mode: "advanced"
  environment: "test"

# Core MCP configuration
mcp:
  enabled: true
  stdio: false
  http:
    enabled: true
    host: "127.0.0.1"
    port: 3001
    cors_enabled: true
    cors_origins: ["http://localhost:3000", "http://localhost:5173"]

# Authentication configuration
auth:
  enabled: true
  
  # Default authentication for most tools
  server_level:
    type: oauth
    provider: github
    scopes: ["user:email", "repo:read"]
  
  # Capability-specific authentication
  capabilities:
    google_workspace:
      type: oauth
      provider: google
      scopes: ["https://www.googleapis.com/auth/spreadsheets.readonly"]
    
    headless_tools:
      type: device_code
      provider: github_device
      scopes: ["user:email", "repo:read"]
    
    admin_tools:
      type: service_account
      account_ref: github_admin_pat
  
  # Tool-specific overrides
  tools:
    emergency_admin:
      type: api_key
      key_ref: emergency_key

# OAuth Provider Configurations
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
    client_id: "${GOOGLE_CLIENT_ID:-test_placeholder}"
    client_secret: "${GOOGLE_CLIENT_SECRET:-test_placeholder}"
    oauth_enabled: true
    device_code_enabled: true
    authorization_endpoint: "https://accounts.google.com/o/oauth2/auth"
    device_authorization_endpoint: "https://oauth2.googleapis.com/device/code"
    token_endpoint: "https://oauth2.googleapis.com/token"
    user_info_endpoint: "https://www.googleapis.com/oauth2/v2/userinfo"
  
  microsoft:
    client_id: "${MICROSOFT_CLIENT_ID:-test_placeholder}"
    client_secret: "${MICROSOFT_CLIENT_SECRET:-test_placeholder}"
    oauth_enabled: true
    device_code_enabled: true
    authorization_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
    device_authorization_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/devicecode"
    token_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/token"
    user_info_endpoint: "https://graph.microsoft.com/v1.0/me"

# API Keys for testing
api_keys:
  emergency_key:
    key: "${EMERGENCY_API_KEY:-sk-test-emergency-key-123}"
    name: "Emergency Admin Key"
    permissions: ["read", "write", "admin", "emergency"]
    active: true

# Service Accounts
service_accounts:
  github_admin_pat:
    type: github_pat
    token: "${GITHUB_ADMIN_PAT:-$GITHUB_PERSONAL_ACCESS_TOKEN}"
    scopes: ["repo", "admin:org", "user:email"]
  
  google_service:
    type: google_service_account
    credentials_file: "${GOOGLE_SERVICE_ACCOUNT_FILE:-/dev/null}"
    scopes: ["https://www.googleapis.com/auth/spreadsheets"]

# Session persistence configuration
session_persistence:
  stdio:
    enabled: true
    storage_backend: "filesystem"
    storage_path: "./test-environments/data/sessions"
    encryption_key: "${SESSION_ENCRYPTION_KEY:-test_key_32_bytes_long_exactly!}"
  
  remote_mcp:
    enabled: true
    health_check_interval: "10s"
    session_recovery_timeout: "30s"
    distributed_storage:
      enabled: false  # Disable Redis for local testing
      backend: "filesystem"
  
  token_management:
    refresh_threshold: "2m"  # Short threshold for testing
    max_token_lifetime: "1h"
    cleanup_expired_sessions: "5m"

# Smart discovery configuration
smart_discovery:
  enabled: true
  mode: "hybrid"
  confidence_threshold: 0.7
  
  # LLM provider for smart discovery
  llm_provider:
    enabled: false  # Disable for basic testing
    provider: "ollama"
    model: "llama2"
    base_url: "http://localhost:11434"

# Registry configuration
registry:
  enabled: true
  scan_paths:
    - "./capabilities"
  auto_reload: true
  
  # Default visibility (all tools hidden, use smart discovery)
  default_hidden: true

# Logging configuration
logging:
  level: "debug"
  format: "json"
  file: "./test-environments/logs/magictunnel-local.log"
  max_size: "100MB"
  max_files: 5

# Metrics and monitoring
metrics:
  enabled: true
  port: 9090
  path: "/metrics"
  
  # Additional auth-specific metrics
  auth_metrics:
    enabled: true
    track_providers: true
    track_sessions: true
    track_errors: true

# Security configuration
security:
  # CORS for local development
  cors:
    enabled: true
    origins: ["http://localhost:3000", "http://localhost:5173"]
    methods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
    headers: ["Content-Type", "Authorization", "X-Session-ID"]
  
  # Rate limiting (lenient for testing)
  rate_limiting:
    enabled: true
    requests_per_minute: 1000
    burst_size: 100
  
  # Content security
  content_security:
    enabled: true
    max_request_size: "10MB"
    allowed_content_types: ["application/json", "text/plain"]

# Development features
development:
  # Hot reload capabilities
  hot_reload: true
  
  # Debug endpoints
  debug_endpoints: true
  
  # Detailed error responses
  verbose_errors: true
  
  # Performance profiling
  profiling: true
EOF

print_success "Local test configuration created"

# Create Docker test environment
print_status "Creating Docker test environment..."

cat > test-environments/docker/docker-compose.yml << 'EOF'
version: '3.8'

services:
  magictunnel:
    build:
      context: ../..
      dockerfile: Dockerfile
    ports:
      - "3001:3001"
      - "9090:9090"
    environment:
      - MAGICTUNNEL_CONFIG_PATH=/app/config/docker.yaml
      - GITHUB_CLIENT_ID=${GITHUB_CLIENT_ID}
      - GITHUB_CLIENT_SECRET=${GITHUB_CLIENT_SECRET}
      - GITHUB_DEVICE_CLIENT_ID=${GITHUB_DEVICE_CLIENT_ID}
      - GITHUB_DEVICE_CLIENT_SECRET=${GITHUB_DEVICE_CLIENT_SECRET}
      - GOOGLE_CLIENT_ID=${GOOGLE_CLIENT_ID}
      - GOOGLE_CLIENT_SECRET=${GOOGLE_CLIENT_SECRET}
      - GITHUB_PERSONAL_ACCESS_TOKEN=${GITHUB_PERSONAL_ACCESS_TOKEN}
      - SESSION_ENCRYPTION_KEY=${SESSION_ENCRYPTION_KEY:-docker_test_key_32_bytes_long!}
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=magictunnel::auth=debug
    volumes:
      - ../configs/docker.yaml:/app/config/docker.yaml:ro
      - ../data:/app/data
      - ../logs:/app/logs
    depends_on:
      - redis
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3001/health"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 30s

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 3

  # Load testing service
  artillery:
    image: node:18-alpine
    working_dir: /app
    volumes:
      - ../../scripts:/app/scripts
      - ../logs:/app/logs
    command: sh -c "npm install -g artillery && sleep infinity"
    depends_on:
      - magictunnel

volumes:
  redis_data:
EOF

# Create Docker-specific configuration
cat > test-environments/configs/docker.yaml << 'EOF'
# Docker Test Environment Configuration
deployment:
  runtime_mode: "advanced"
  environment: "docker-test"

mcp:
  enabled: true
  stdio: false
  http:
    enabled: true
    host: "0.0.0.0"  # Accept connections from any IP in container
    port: 3001
    cors_enabled: true

auth:
  enabled: true
  server_level:
    type: oauth
    provider: github
    scopes: ["user:email", "repo:read"]

oauth_providers:
  github:
    client_id: "${GITHUB_CLIENT_ID}"
    client_secret: "${GITHUB_CLIENT_SECRET}"
    oauth_enabled: true
    authorization_endpoint: "https://github.com/login/oauth/authorize"
    token_endpoint: "https://github.com/login/oauth/access_token"
    user_info_endpoint: "https://api.github.com/user"

session_persistence:
  stdio:
    enabled: true
    storage_backend: "filesystem"
    storage_path: "/app/data/sessions"
    encryption_key: "${SESSION_ENCRYPTION_KEY}"
  
  remote_mcp:
    enabled: true
    distributed_storage:
      enabled: true
      backend: "redis"
      redis_url: "${REDIS_URL}"

smart_discovery:
  enabled: true
  mode: "hybrid"

logging:
  level: "info"
  format: "json"
  file: "/app/logs/magictunnel-docker.log"

metrics:
  enabled: true
  port: 9090
  path: "/metrics"
EOF

print_success "Docker test environment created"

# Create staging environment configuration
print_status "Creating staging environment configuration..."

cat > test-environments/configs/staging.yaml << 'EOF'
# Staging Environment Configuration
deployment:
  runtime_mode: "advanced"
  environment: "staging"

mcp:
  enabled: true
  stdio: false
  http:
    enabled: true
    host: "0.0.0.0"
    port: 3001
    cors_enabled: false  # Disable CORS in staging

auth:
  enabled: true
  server_level:
    type: oauth
    provider: github
    scopes: ["user:email", "repo:read"]

oauth_providers:
  github:
    client_id: "${GITHUB_STAGING_CLIENT_ID:-$GITHUB_CLIENT_ID}"
    client_secret: "${GITHUB_STAGING_CLIENT_SECRET:-$GITHUB_CLIENT_SECRET}"
    oauth_enabled: true
    authorization_endpoint: "https://github.com/login/oauth/authorize"
    token_endpoint: "https://github.com/login/oauth/access_token"
    user_info_endpoint: "https://api.github.com/user"

session_persistence:
  stdio:
    enabled: true
    storage_backend: "filesystem"
    storage_path: "/var/lib/magictunnel/sessions"
    encryption_key: "${SESSION_ENCRYPTION_KEY}"
  
  token_management:
    refresh_threshold: "5m"
    max_token_lifetime: "24h"
    cleanup_expired_sessions: "1h"

smart_discovery:
  enabled: true
  mode: "hybrid"

logging:
  level: "info"
  format: "json"
  file: "/var/log/magictunnel/magictunnel.log"

security:
  rate_limiting:
    enabled: true
    requests_per_minute: 100
    burst_size: 20
  
  content_security:
    enabled: true
    max_request_size: "1MB"

metrics:
  enabled: true
  port: 9090
  path: "/metrics"
EOF

print_success "Staging environment configuration created"

# Create environment variables template
print_status "Creating environment variables template..."

cat > test-environments/.env.template << 'EOF'
# OAuth 2.1 Testing Environment Variables
# Copy this to .env and fill in your actual values

# GitHub OAuth Configuration
GITHUB_CLIENT_ID=your_github_client_id_here
GITHUB_CLIENT_SECRET=your_github_client_secret_here

# GitHub Device Flow (can be same as above or separate app)
GITHUB_DEVICE_CLIENT_ID=your_github_device_client_id_here
GITHUB_DEVICE_CLIENT_SECRET=your_github_device_client_secret_here

# Google OAuth Configuration (optional)
GOOGLE_CLIENT_ID=your_google_client_id_here
GOOGLE_CLIENT_SECRET=your_google_client_secret_here

# Microsoft OAuth Configuration (optional)
MICROSOFT_CLIENT_ID=your_microsoft_client_id_here
MICROSOFT_CLIENT_SECRET=your_microsoft_client_secret_here

# Service Account Tokens
GITHUB_PERSONAL_ACCESS_TOKEN=ghp_your_personal_access_token_here
GITHUB_ADMIN_PAT=ghp_your_admin_token_here

# Google Service Account (optional)
GOOGLE_SERVICE_ACCOUNT_FILE=/path/to/service-account.json

# Session Encryption
SESSION_ENCRYPTION_KEY=your_32_byte_encryption_key_here

# Emergency API Key for testing
EMERGENCY_API_KEY=sk-emergency-test-key-123

# Redis Configuration (for distributed testing)
REDIS_URL=redis://localhost:6379

# Staging Environment (optional)
GITHUB_STAGING_CLIENT_ID=your_staging_client_id
GITHUB_STAGING_CLIENT_SECRET=your_staging_client_secret

# Test Configuration
MAGICTUNNEL_TEST_MODE=true
RUST_LOG=magictunnel::auth=debug
EOF

print_success "Environment variables template created"

# Create test runner script
print_status "Creating test runner script..."

cat > test-environments/run-tests.sh << 'EOF'
#!/bin/bash

set -e

echo "🧪 MagicTunnel OAuth 2.1 Test Runner"
echo "==================================="

# Load environment variables if .env exists
if [ -f ".env" ]; then
    echo "📋 Loading environment variables from .env"
    export $(cat .env | grep -v '^#' | xargs)
else
    echo "⚠️  No .env file found. Using system environment variables."
fi

# Test environment selection
ENVIRONMENT=${1:-local}

case $ENVIRONMENT in
    "local")
        echo "🏠 Running local tests..."
        cd .. && ./scripts/test-oauth-production.sh
        ;;
    
    "docker")
        echo "🐳 Running Docker tests..."
        cd docker
        docker-compose up -d
        sleep 30  # Wait for services to start
        
        # Run tests against Docker container
        docker-compose exec artillery artillery run /app/scripts/oauth-load-test.yml --target http://magictunnel:3001
        
        # Cleanup
        docker-compose down
        ;;
    
    "staging")
        echo "🎭 Running staging tests..."
        echo "Staging tests require manual setup. See docs/OAUTH_2_1_TESTING_GUIDE.md"
        ;;
    
    "all")
        echo "🌐 Running all test environments..."
        $0 local
        $0 docker
        ;;
    
    *)
        echo "❌ Unknown environment: $ENVIRONMENT"
        echo "Usage: $0 [local|docker|staging|all]"
        exit 1
        ;;
esac

echo "✅ Tests completed for environment: $ENVIRONMENT"
EOF

chmod +x test-environments/run-tests.sh

print_success "Test runner script created"

# Create monitoring script
print_status "Creating monitoring script..."

cat > test-environments/monitor.sh << 'EOF'
#!/bin/bash

echo "📊 MagicTunnel OAuth 2.1 Monitoring Dashboard"
echo "============================================="

# Function to get metrics
get_metrics() {
    curl -s http://localhost:3001/metrics | grep -E "auth_|session_|oauth_" || echo "No auth metrics available"
}

# Function to get health status
get_health() {
    curl -s http://localhost:3001/health | jq '.' 2>/dev/null || echo "Health endpoint not available"
}

# Function to get session status
get_sessions() {
    curl -s http://localhost:3001/admin/sessions/status | jq '.' 2>/dev/null || echo "Session status not available"
}

# Main monitoring loop
while true; do
    clear
    echo "📊 MagicTunnel OAuth 2.1 Monitoring Dashboard"
    echo "============================================="
    echo "📅 $(date)"
    echo ""
    
    echo "🏥 Health Status:"
    get_health
    echo ""
    
    echo "👥 Session Status:"
    get_sessions
    echo ""
    
    echo "📈 Authentication Metrics:"
    get_metrics
    echo ""
    
    echo "🔄 Refreshing in 10 seconds... (Ctrl+C to exit)"
    sleep 10
done
EOF

chmod +x test-environments/monitor.sh

print_success "Monitoring script created"

# Create comprehensive README
print_status "Creating test environment README..."

cat > test-environments/README.md << 'EOF'
# OAuth 2.1 Testing Environments

This directory contains comprehensive testing environments for MagicTunnel's OAuth 2.1 authentication system.

## Quick Start

1. **Setup environment variables**:
   ```bash
   cp .env.template .env
   # Edit .env with your OAuth application credentials
   ```

2. **Run local tests**:
   ```bash
   ./run-tests.sh local
   ```

3. **Monitor system during tests**:
   ```bash
   ./monitor.sh
   ```

## Environments

### Local Environment
- **Purpose**: Development and basic testing
- **Configuration**: `configs/local.yaml`
- **Features**: 
  - Filesystem session storage
  - Debug logging
  - Hot reload
  - CORS enabled for development

### Docker Environment
- **Purpose**: Containerized testing with Redis
- **Configuration**: `docker/docker-compose.yml`
- **Features**:
  - Redis-backed session storage
  - Container isolation
  - Production-like networking
  - Load testing with Artillery

### Staging Environment
- **Purpose**: Pre-production testing
- **Configuration**: `configs/staging.yaml`
- **Features**:
  - Production security settings
  - Rate limiting enabled
  - Minimal CORS
  - Performance monitoring

## Test Types

### Integration Tests
```bash
# OAuth Phase 6 integration
cargo test --test oauth2_1_phase6_integration_test

# Device Code Flow
cargo test --test device_code_integration_test

# Service Accounts
cargo test --test service_account_integration_test
```

### Load Tests
```bash
# Install Artillery
npm install -g artillery

# Run load tests
artillery run ../scripts/oauth-load-test.yml
```

### Security Tests
```bash
# PKCE validation
./test-pkce-security.sh

# State parameter validation
./test-csrf-protection.sh

# Token storage security
./test-token-security.sh
```

## Monitoring

### Metrics Endpoints
- Health: `http://localhost:3001/health`
- Metrics: `http://localhost:3001/metrics`
- Sessions: `http://localhost:3001/admin/sessions/status`

### Key Metrics
- `auth_requests_total` - Total authentication requests
- `auth_requests_success` - Successful authentications
- `oauth_authorization_flow_total` - OAuth flows initiated
- `device_code_flow_total` - Device code flows initiated
- `session_recovery_total` - Session recovery events
- `token_refresh_total` - Token refresh operations

## Troubleshooting

### Common Issues

**Tests fail with "OAuth provider not configured"**:
- Check `.env` file has correct OAuth credentials
- Verify OAuth applications are configured correctly

**Docker tests fail**:
- Ensure Docker is running
- Check Docker Compose logs: `docker-compose logs`

**Load tests timeout**:
- Increase Artillery timeout settings
- Check system resources (CPU, memory)

### Debug Commands

```bash
# Check environment variables
env | grep -E "GITHUB|GOOGLE|OAUTH"

# View recent logs
tail -f logs/magictunnel-*.log

# Test OAuth provider connectivity
curl -v https://github.com/login/oauth/authorize

# Validate configuration
../target/release/magictunnel --config configs/local.yaml --validate-config
```

## Performance Benchmarks

### Expected Performance
- OAuth authorization: < 500ms
- Device code generation: < 100ms
- Token refresh: < 1s
- Session recovery: < 2s

### Load Test Targets
- Concurrent users: 50+
- Requests per second: 100+
- Error rate: < 5%
- Response time P95: < 2s

## Security Checklist

- [ ] PKCE challenge generation verified
- [ ] State parameter validation working
- [ ] Token storage encrypted
- [ ] Session isolation verified
- [ ] Cross-user access prevented
- [ ] Rate limiting functional
- [ ] HTTPS redirects enforced (staging/production)

## Next Steps

1. **Configure OAuth Applications**: Set up real OAuth apps with providers
2. **Run Full Test Suite**: Execute all test environments
3. **Performance Tune**: Optimize based on load test results
4. **Security Review**: Complete security audit
5. **Production Deploy**: Deploy to staging environment

## Support

- **Documentation**: `../docs/OAUTH_2_1_API_REFERENCE.md`
- **Testing Guide**: `../docs/OAUTH_2_1_TESTING_GUIDE.md`
- **Configuration**: `../docs/OAUTH_2_1_tasks.md`
EOF

print_success "Test environment README created"

# Create summary
echo ""
echo "🎉 OAuth 2.1 Testing Environment Setup Complete!"
echo "================================================"
echo ""
print_success "✅ Test directories created"
print_success "✅ Local test configuration ready"
print_success "✅ Docker environment configured"
print_success "✅ Staging environment prepared"
print_success "✅ Environment variables template created"
print_success "✅ Test runner script ready"
print_success "✅ Monitoring tools configured"
print_success "✅ Comprehensive documentation created"

echo ""
echo "📋 Next Steps:"
echo "   1. Copy test-environments/.env.template to test-environments/.env"
echo "   2. Fill in your OAuth application credentials in .env"
echo "   3. Run: cd test-environments && ./run-tests.sh local"
echo "   4. For Docker testing: ./run-tests.sh docker"
echo "   5. Monitor tests: ./monitor.sh"
echo ""
echo "📖 Documentation:"
echo "   - Setup guide: test-environments/README.md"
echo "   - API reference: docs/OAUTH_2_1_API_REFERENCE.md"
echo "   - Testing guide: docs/OAUTH_2_1_TESTING_GUIDE.md"
echo ""
echo "🔐 OAuth 2.1 Testing Environment: READY"