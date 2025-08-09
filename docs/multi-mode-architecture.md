# Multi-Mode Architecture Guide (v0.3.11)

MagicTunnel implements a sophisticated multi-mode architecture that provides two distinct runtime modes to address different deployment scenarios and use cases.

## Overview

The multi-mode architecture allows MagicTunnel to function as either:
- **Proxy Mode**: Lightweight, zero-config MCP proxy for development and simple deployments
- **Advanced Mode**: Full-featured enterprise platform with comprehensive management capabilities

## Architecture Principles

### Pure Config-Driven Design
All runtime behavior is controlled through:
1. **Environment Variables** (highest priority)
2. **Configuration Files** (default priority) 
3. **Built-in Defaults** (fallback)

### Conditional Service Loading
Services are loaded conditionally based on runtime mode:
- **ProxyServices**: Core MCP functionality, basic routing, minimal web UI
- **AdvancedServices**: Security, authentication, LLM services, enterprise features

## Runtime Modes

### 🚀 Proxy Mode (Default)

**Use Cases:**
- Development and testing
- Simple MCP proxy deployments
- Resource-constrained environments
- Quick setup scenarios

**Features:**
- ✅ Core MCP server with tool routing
- ✅ Smart tool discovery (if configured)
- ✅ Basic web dashboard
- ✅ External MCP integration
- ✅ Fast startup (<2 seconds)
- ✅ Low memory footprint (<50MB)
- ❌ No enterprise security features
- ❌ No LLM enhancement services
- ❌ No audit logging
- ❌ No OAuth authentication

**Configuration:**
```yaml
# magictunnel-config.yaml
deployment:
  runtime_mode: "proxy"

smart_discovery:
  enabled: true  # Optional but recommended
```

**Startup:**
```bash
# Environment variable override
export MAGICTUNNEL_RUNTIME_MODE=proxy
./magictunnel

# Or use config file
./magictunnel  # Reads from magictunnel-config.yaml
```

### 🏢 Advanced Mode

**Use Cases:**
- Production deployments
- Enterprise environments
- Multi-user systems
- Security-conscious deployments

**Features:**
- ✅ All Proxy Mode features
- ✅ Enterprise security management
- ✅ Role-based access control (RBAC)
- ✅ LLM services and enhancement pipeline
- ✅ Complete web dashboard with security UI
- ✅ Audit logging and monitoring
- ✅ OAuth 2.1 authentication
- ✅ Request sanitization and filtering
- ✅ Tool allowlisting and security policies
- ⚠️ Longer startup time (~5-10 seconds)
- ⚠️ Higher memory usage (100-200MB)

**Configuration:**
```yaml
# magictunnel-config.yaml
deployment:
  runtime_mode: "advanced"

smart_discovery:
  enabled: true

# Additional enterprise features
auth:
  enabled: true
  type: "oauth"

security:
  enterprise_features:
    enabled: true

sampling:
  enabled: true

elicitation:
  enabled: true
```

**Startup:**
```bash
# Environment variable override
export MAGICTUNNEL_RUNTIME_MODE=advanced
./magictunnel

# Or use config file with advanced mode
./magictunnel
```

## Environment Variable System

### Priority Order
1. **Environment Variables** (highest)
2. **Config File Settings** (medium)
3. **Built-in Defaults** (lowest)

### Core Environment Variables

```bash
# Runtime mode control
export MAGICTUNNEL_RUNTIME_MODE=proxy        # proxy | advanced

# Config file resolution
export MAGICTUNNEL_CONFIG_PATH=./my-config.yaml  # Custom config path

# Feature overrides
export MAGICTUNNEL_SMART_DISCOVERY=true      # true | false

# Smart discovery provider override  
export SMART_DISCOVERY_LLM_PROVIDER=openai   # openai | anthropic | ollama

# API keys for LLM services (Advanced mode)
export OPENAI_API_KEY=your_key_here
export ANTHROPIC_API_KEY=your_key_here
```

### Environment Variable Validation

The system validates environment variables at startup:
```bash
🚀 MagicTunnel v0.3.10 starting...

📋 Configuration Resolution:
   ├─ Runtime Mode: proxy (environment override)  
   ├─ Config File: ./magictunnel-config.yaml (auto-detected)
   ├─ Smart Discovery: enabled (environment override)
   └─ Environment Variables: 3 active overrides

✅ Configuration validated successfully
```

## Configuration File Resolution

### Default Config File Detection

1. **magictunnel-config.yaml** (preferred, new default)
2. **config.yaml** (legacy, still supported)
3. **Built-in defaults** (if no config found)

### Custom Config Path

```bash
# Via environment variable
export MAGICTUNNEL_CONFIG_PATH=./configs/production.yaml
./magictunnel

# Via command line (still supported)
./magictunnel --config ./configs/production.yaml
```

## Service Loading Strategy

### ServiceContainer Architecture

The **ServiceContainer** is the core architectural pattern that enables multi-mode functionality:

```rust
pub struct ServiceContainer {
    /// Core proxy services (always present)
    pub proxy_services: Option<ProxyServices>,
    /// Advanced enterprise services (only in advanced mode)
    pub advanced_services: Option<AdvancedServices>,
    /// Runtime mode for this container
    pub runtime_mode: RuntimeMode,
    /// Total number of loaded services
    pub service_count: usize,
}

// Configuration-driven instantiation via ServiceLoader
let service_container = match runtime_mode {
    RuntimeMode::Proxy => {
        let proxy_services = ProxyServices::new(config.clone()).await?;
        let service_count = proxy_services.service_count();
        ServiceContainer {
            proxy_services: Some(proxy_services),
            advanced_services: None,
            runtime_mode: RuntimeMode::Proxy,
            service_count,
        }
    }
    RuntimeMode::Advanced => {
        let proxy_services = ProxyServices::new(config.clone()).await?;
        let advanced_services = AdvancedServices::new(config.clone(), &proxy_services).await?;
        let total_services = proxy_services.service_count() + advanced_services.service_count();
        ServiceContainer {
            proxy_services: Some(proxy_services),
            advanced_services: Some(advanced_services),
            runtime_mode: RuntimeMode::Advanced,
            service_count: total_services,
        }
    }
};
```

### ProxyServices Container

Loaded in **Proxy Mode**:
- Registry (tool management)
- MCP Server (protocol handling)
- Tool Enhancement (optional, core LLM service)
- Smart Discovery (optional, intelligent tool routing)
- Health Monitoring (built-in)
- Web Dashboard (via MCP server)

### AdvancedServices Container  

Loaded in **Advanced Mode** (includes all ProxyServices plus):
- Tool Allowlisting (enterprise tool control)
- RBAC (Role-Based Access Control) 
- Request Sanitization (content filtering)
- Audit Logging (compliance tracking)
- Security Policies (organization-wide rules)
- Emergency Lockdown (security response)
- Advanced Web Dashboard (security UI)
- Enterprise Monitoring (security metrics)

### Service Dependencies

Services are loaded with proper dependency validation:
```bash
🚀 Loading services for advanced mode

📊 Service Container Details:
   Runtime Mode: advanced
   Total Services: 10
   📦 Proxy Services (4 services):
      1. Registry
      2. MCP Server
      3. Tool Enhancement
      4. Smart Discovery
   🏢 Advanced Services (6 services):
      1. Tool Allowlisting
      2. RBAC (Role-Based Access Control)
      3. Request Sanitization
      4. Audit Logging
      5. Security Policies
      6. Emergency Lockdown

   ✅ All services are healthy
✅ Service loading completed for advanced mode (10 total services)
```

## Configuration Validation System

### Mode-Specific Validators

**Proxy Mode Validation:**
- Verifies core MCP server configuration
- Validates tool registry paths
- Checks smart discovery dependencies (if enabled)
- Ensures minimal resource requirements

**Advanced Mode Validation:**
- All proxy mode validations
- Verifies enterprise feature configuration
- Validates authentication setup
- Checks LLM service dependencies
- Ensures security policy configuration

### Helpful Error Messages

```bash
❌ Configuration Error: Advanced mode requires authentication configuration

Suggestion: Add authentication configuration to your config file:

auth:
  enabled: true
  type: "api_key"  # or "oauth" or "jwt"
  api_keys:
    keys:
      - key: "your_secure_api_key_here"
        permissions: ["read", "write"]

Run with MAGICTUNNEL_RUNTIME_MODE=proxy for minimal setup.
```

## Frontend Mode Awareness

### Mode Detection API

The web dashboard automatically detects the current runtime mode:

```javascript
// Frontend mode detection
const response = await fetch('/api/mode');
const { mode } = await response.json();

if (mode === 'proxy') {
  // Hide advanced features
  hideSecurityNavigation();
  hideLLMServices();
} else if (mode === 'advanced') {
  // Show all features
  showEnterpriseDashboard();
}
```

### Progressive Enhancement

The UI adapts based on available services:
- **Proxy Mode**: Basic navigation, core tool management
- **Advanced Mode**: Full navigation with security, LLM services, enterprise features

### Complete System Integration Flow (v0.3.11)

**Unified Status Banner System Integration:**

```
1. Configuration Resolution (main.rs)
   ├─ Environment Variable Priority
   ├─ Config File Detection
   └─ Runtime Mode Determination

2. Service Container Loading (ServiceLoader)
   ├─ ProxyServices (always loaded)
   └─ AdvancedServices (conditional)

3. Supervisor Integration (TCP 8081)
   ├─ Process Management
   ├─ Custom Restart Workflows
   └─ Environment Variable Passing

4. Frontend Status Integration
   ├─ Banner Store System (/lib/stores/banner.ts)
   ├─ Real-time Status Updates
   ├─ Mode Switch Integration
   └─ Auto-refresh & Page Reload

5. Mode Switch Complete Flow
   Frontend Request → Custom Restart → Environment Override → 
   Service Container Reload → Status Monitoring → Success Banner → 
   Page Reload → Complete Integration
```

**Key Integration Points:**
- **ServiceContainer**: Core architectural pattern enabling mode switching
- **Environment Variables**: `MAGICTUNNEL_RUNTIME_MODE` persists across restarts
- **Banner System**: Provides real-time user feedback during transitions
- **Supervisor TCP**: Handles complex restart workflows with pre/post commands
- **Progressive UI**: Frontend adapts automatically based on available services

## Migration Guide

### From Legacy Configuration

**Old approach (v0.3.9):**
```bash
./magictunnel --config config.yaml
```

**New approach (v0.3.10):**
```bash
# Rename config.yaml to magictunnel-config.yaml
mv config.yaml magictunnel-config.yaml

# Add deployment section
echo "deployment:" >> magictunnel-config.yaml  
echo "  runtime_mode: \"advanced\"" >> magictunnel-config.yaml

# Run with auto-detection
./magictunnel
```

### Environment Variable Migration

**Old environment variables (still work):**
```bash
export MCP_HOST=127.0.0.1
export MCP_PORT=3001
export SMART_DISCOVERY_ENABLED=true
```

**New environment variables (recommended):**
```bash
export MAGICTUNNEL_RUNTIME_MODE=advanced
export MAGICTUNNEL_CONFIG_PATH=./magictunnel-config.yaml  
export MAGICTUNNEL_SMART_DISCOVERY=true
```

## Best Practices

### Development Setup
```bash
# Quick development with proxy mode
export MAGICTUNNEL_RUNTIME_MODE=proxy
export MAGICTUNNEL_SMART_DISCOVERY=true
export RUST_LOG=debug
./magictunnel
```

### Production Setup
```bash
# Production with advanced mode
export MAGICTUNNEL_RUNTIME_MODE=advanced
export MAGICTUNNEL_CONFIG_PATH=/etc/magictunnel/config.yaml
export OPENAI_API_KEY=your_production_key
./magictunnel
```

### Container Deployment
```dockerfile
# Dockerfile
FROM rust:1.75-slim

COPY target/release/magictunnel /usr/local/bin/
COPY magictunnel-config.yaml /etc/magictunnel/

ENV MAGICTUNNEL_RUNTIME_MODE=advanced
ENV MAGICTUNNEL_CONFIG_PATH=/etc/magictunnel/magictunnel-config.yaml

CMD ["magictunnel"]
```

## Troubleshooting

### Common Issues

**Issue**: "Configuration file not found"
```bash
❌ Error: Configuration file not found

✅ Solution: Create magictunnel-config.yaml or set MAGICTUNNEL_CONFIG_PATH
```

**Issue**: "Advanced mode missing dependencies"
```bash  
❌ Error: Advanced mode requires API keys for LLM services

✅ Solution: Set environment variables or switch to proxy mode:
export OPENAI_API_KEY=your_key
# or
export MAGICTUNNEL_RUNTIME_MODE=proxy
```

**Issue**: "Service loading failed"
```bash
❌ Error: Failed to load AdvancedServices

✅ Solution: Check service dependencies and configuration:
RUST_LOG=debug ./magictunnel
```

### Debug Mode

Enable comprehensive debug logging:
```bash
export RUST_LOG=magictunnel::config=debug,magictunnel::services=debug
./magictunnel
```

This provides detailed information about:
- Configuration resolution process
- Environment variable processing  
- Service loading and dependencies
- Mode-specific validations

## API Endpoints

### Mode Information
```bash
# Get current runtime mode
curl http://localhost:3001/api/mode

# Get configuration information  
curl http://localhost:3001/api/config

# Get service status
curl http://localhost:3001/api/services/status
```

### Response Examples

**Proxy Mode:**
```json
{
  "mode": "proxy",
  "services": ["mcp_server", "registry", "smart_discovery", "web_dashboard"],
  "features": {
    "smart_discovery": true,
    "security": false,
    "llm_services": false
  }
}
```

**Advanced Mode:**
```json
{
  "mode": "advanced", 
  "services": ["mcp_server", "registry", "smart_discovery", "security", "auth", "llm_services", "audit", "web_dashboard"],
  "features": {
    "smart_discovery": true,
    "security": true,
    "llm_services": true,
    "audit_logging": true,
    "oauth": true
  }
}
```

## Performance Considerations

### Startup Times
- **Proxy Mode**: ~1-2 seconds
- **Advanced Mode**: ~5-10 seconds (includes LLM service initialization)

### Memory Usage
- **Proxy Mode**: ~30-50MB
- **Advanced Mode**: ~100-200MB (includes security services and LLM caching)

### Resource Scaling
- **Proxy Mode**: Suitable for single-user, development
- **Advanced Mode**: Designed for multi-user, enterprise scale

## Security Implications

### Proxy Mode Security
- Basic input validation
- No authentication required
- Limited audit logging
- Suitable for trusted environments

### Advanced Mode Security  
- Comprehensive security framework
- Authentication and authorization
- Full audit logging
- Request sanitization
- Security policy enforcement
- Suitable for production/enterprise

---

This multi-mode architecture provides the flexibility to deploy MagicTunnel in various environments while maintaining a clean separation between core functionality and enterprise features.