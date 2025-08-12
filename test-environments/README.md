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
