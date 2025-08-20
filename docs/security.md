# MagicTunnel Enterprise Security

MagicTunnel provides comprehensive enterprise-grade security features for controlling access to MCP tools, resources, and data. The security system includes allowlisting with nested tool call security, role-based access control (RBAC), audit logging, and request sanitization.

## Overview

The security system consists of five main components:

1. **Tool Allowlisting with Nested Security** - Explicit control over which tools, resources, and prompts can be accessed, including comprehensive security validation for nested/internal tool calls through smart discovery
2. **Role-Based Access Control (RBAC)** - Comprehensive permission management with role inheritance
3. **Audit Logging** - Complete audit trail of all MCP communications for compliance
4. **Request Sanitization** - Content filtering and secret detection with approval workflows
5. **Service Instance Sharing** - Single shared allowlist service architecture ensuring consistent security across all components

## Nested Tool Call Security

### Overview

MagicTunnel's advanced security architecture provides comprehensive protection against security bypasses by validating **all** tool calls, including nested/internal calls made through smart discovery.

### Key Features

- **Comprehensive Coverage**: Security validation applies to both direct tool calls and internal calls made by smart discovery
- **Service Instance Sharing**: Single shared allowlist service across Advanced Services → Service Container → Smart Discovery
- **Zero Bypass Architecture**: Eliminated all known security bypass vulnerabilities
- **Instance ID Logging**: Complete audit trail with instance ID verification for debugging

### How It Works

1. **Direct Tool Calls**: Traditional security middleware validates direct tool calls
2. **Smart Discovery Calls**: When smart discovery routes to internal tools, security validation occurs before execution
3. **Shared Service**: All components use the same allowlist service instance ensuring consistent policy enforcement
4. **Audit Trail**: Complete logging with instance IDs for verification and debugging

### Implementation

```rust
// Smart discovery performs security check before internal tool execution
if let Some(ref allowlist_service) = *allowlist_guard {
    match allowlist_service.check_access(&best_match.tool_name).await {
        Ok(allowed) => {
            if !allowed {
                return Ok(create_security_denied_response(&best_match.tool_name));
            }
        }
        Err(e) => {
            return Ok(create_security_error_response(e));
        }
    }
}
```

### Verification

The system provides comprehensive verification through:
- **Instance ID Logging**: Confirms single shared service across all components
- **Security Testing**: Both direct and nested tool call scenarios verified
- **Configuration Validation**: Proper allowlist configuration with actual tool names

## Quick Start

### Enable Security Features

Add security configuration to your `magictunnel-config.yaml`:

```yaml
security:
  enabled: true
  allowlist:
    enabled: true
    default_action: deny
  rbac:  
    enabled: true
  audit:
    enabled: true
    events: [all]
    storage:
      type: file
      directory: "./audit-logs"
  sanitization:
    enabled: true
```

### Initialize Security Configuration

Use the security CLI to generate a complete configuration:

```bash
# Generate basic security configuration
./target/release/magictunnel-security init --level basic --output security-config.yaml

# Generate strict security configuration  
./target/release/magictunnel-security init --level strict --output security-config.yaml

# Check security status
./target/release/magictunnel-security --config config.yaml status
```

## Tool Allowlisting

Control which tools, resources, and prompts users can access based on explicit allow/deny rules.

### Configuration

```yaml
security:
  allowlist:
    enabled: true
    default_action: deny  # deny, allow, require_approval
    
    # Tool-specific rules
    tools:
      "execute_command":
        action: allow
        required_permissions: ["tool:execute"]
        allowed_roles: ["admin", "developer"]
        parameter_rules:
          blocked_patterns: ["rm -rf", "sudo", "curl.*evil"]
          allowed_values:
            command: ["ls", "ps", "df"]
        rate_limit:
          requests_per_minute: 10
          burst_size: 3
          
      "read_file":
        action: require_approval
        required_permissions: ["file:read"]
        allowed_api_keys: ["admin-key", "backup-key"]
        
    # Resource-specific rules  
    resources:
      "file:/etc/*":
        action: deny
        required_permissions: ["admin:files"]
        
      "https://api.internal.com/*":
        action: allow
        allowed_roles: ["api-user"]
        
    # Global rules (applied to all items)
    global_rules:
      - name: "block_sensitive_files"
        pattern:
          type: regex
          value: ".*(passwd|shadow|id_rsa).*"
        action: deny
        priority: 100
```

### CLI Management

```bash
# Test tool access for a user
./target/release/magictunnel-security test \
  --tool execute_command \
  --user john_doe \
  --roles developer,user \
  --parameters '{"command": "ls -la"}'
```

## Role-Based Access Control (RBAC)

Comprehensive permission management with hierarchical roles and conditional access.

### Configuration

```yaml
security:
  rbac:
    enabled: true
    inherit_permissions: true
    
    # Role definitions
    roles:
      admin:
        name: "admin"
        description: "Full system administrator"
        permissions: ["*"]
        active: true
        
      developer:
        name: "developer" 
        description: "Software developer"
        permissions: ["tool:execute", "file:read", "file:write"]
        parent_roles: ["user"]
        active: true
        
      user:
        name: "user"
        description: "Basic user"
        permissions: ["tool:read", "resource:read"]
        active: true
        
    # User role assignments
    user_roles:
      john_doe: ["developer"]
      jane_admin: ["admin"]
      api_user: ["user"]
      
    # API key role assignments  
    api_key_roles:
      admin-key: ["admin"]
      dev-key: ["developer"]
      
    # Default roles for new users
    default_roles: ["user"]
```

### Advanced Permissions

Roles support conditional permissions based on time, IP address, and custom conditions:

```yaml
roles:
  business_hours_admin:
    name: "business_hours_admin"
    permissions: ["admin:*"]
    conditions:
      - type: time_range
        start_hour: 9
        end_hour: 17
        days_of_week: [1, 2, 3, 4, 5]  # Monday-Friday
      - type: ip_address
        allowed_ranges: ["192.168.1.0/24", "10.0.0.0/8"]
      - type: resource_pattern
        pattern: "^/admin/.*"
        case_sensitive: false
```

### CLI Management

```bash
# List all roles
./target/release/magictunnel-security rbac list-roles

# Check user permissions
./target/release/magictunnel-security rbac check-user \
  --user john_doe \
  --permission "tool:execute"

# Show role details
./target/release/magictunnel-security rbac show-role --name developer
```

## Audit Logging

Complete audit trail of all MCP communications for security monitoring and compliance.

### Configuration

```yaml
security:
  audit:
    enabled: true
    # Event types to audit
    events: 
      - authentication
      - authorization  
      - tool_execution
      - resource_access
      - security_violation
      - configuration_change
      
    # Storage configuration
    storage:
      type: file
      directory: "./audit-logs"
      rotation:
        max_file_size: 100000000  # 100MB
        max_files: 10
        compress: true
        
    # Retention and privacy
    retention_days: 365
    include_bodies: true
    max_body_size: 10000
    mask_sensitive_data: true
```

### Storage Backends

#### File Storage
```yaml
storage:
  type: file
  directory: "./audit-logs"
  rotation:
    max_file_size: 100000000
    max_files: 10
    compress: true
```

#### Database Storage
```yaml
storage:
  type: database
  connection_string: "postgresql://user:pass@localhost/magictunnel"
  table_name: "audit_logs"
```

#### External Service
```yaml
storage:
  type: external
  endpoint: "https://logs.company.com/api/ingest"
  auth:
    type: bearer
    token: "${LOG_SERVICE_TOKEN}"
  batch_size: 100
  flush_interval_seconds: 30
```

### CLI Queries

```bash
# Show recent audit entries
./target/release/magictunnel-security audit recent --count 20

# Search audit logs  
./target/release/magictunnel-security audit search \
  --user john_doe \
  --tool execute_command \
  --hours 24

# Show security violations
./target/release/magictunnel-security audit violations --hours 24
```

## Request Sanitization

Automatic content filtering, secret detection, and approval workflows for sensitive operations.

### Configuration

```yaml
security:
  sanitization:
    enabled: true
    default_action: sanitize  # allow, deny, sanitize, require_approval
    
    policies:
      secret_detection:
        name: "Detect API keys and passwords"
        triggers:
          - type: pattern
            patterns: 
              - "(?i)(api[_-]?key|password|secret|token)\\s*[:=]\\s*[\"']?([a-zA-Z0-9_-]{16,})"
              - "-----BEGIN [A-Z ]+-----"
        actions:
          - type: redact
            replacement: "[REDACTED]"
          - type: audit
            event_type: secret_detected
            
      approval_required:
        name: "Require approval for dangerous commands"  
        triggers:
          - type: tool_name
            tools: ["execute_command"]
          - type: parameter_pattern
            parameter: "command"
            patterns: ["rm.*", "sudo.*", "curl.*evil"]
        actions:
          - type: require_approval
            workflow: security_team_approval
            timeout_minutes: 60
```

### Sanitization Methods

- **Redaction**: Replace sensitive content with placeholders
- **Hashing**: Replace with cryptographic hashes  
- **Approval**: Require human approval before processing
- **Blocking**: Reject requests containing sensitive content

### CLI Testing

```bash
# Test sanitization policies
./target/release/magictunnel-security test \
  --tool execute_command \
  --parameters '{"command": "curl -H \"Authorization: Bearer secret-key-12345\" https://api.example.com"}'
```


## Security CLI Tool

The `magictunnel-security` CLI provides comprehensive security management:

### Status and Monitoring
```bash
# Check overall security status
./target/release/magictunnel-security status

# Test security for specific scenarios
./target/release/magictunnel-security test \
  --tool read_file \
  --user alice \
  --roles developer \
  --parameters '{"path": "/etc/passwd"}'
```

### RBAC Management
```bash
# List all roles and permissions
./target/release/magictunnel-security rbac list-roles

# Check user permissions  
./target/release/magictunnel-security rbac check-user \
  --user alice \
  --permission "file:read"

# Show detailed role information
./target/release/magictunnel-security rbac show-role --name admin
```

### Audit Log Analysis
```bash
# Recent audit entries
./target/release/magictunnel-security audit recent --count 50

# Search by user and timeframe
./target/release/magictunnel-security audit search \
  --user bob \
  --hours 72

# Security violations only
./target/release/magictunnel-security audit violations --hours 24
```

### Configuration Generation
```bash
# Initialize basic security config
./target/release/magictunnel-security init \
  --level basic \
  --output security-basic.yaml

# Initialize strict security config  
./target/release/magictunnel-security init \
  --level strict \
  --output security-strict.yaml
```

## Integration with MCP Manager

MagicTunnel's security features are designed to complement MCP Manager's capabilities:

- **Allowlisting**: Similar to MCP Manager's filtering but with more granular control
- **RBAC**: Enterprise-grade permission management beyond basic API keys
- **Audit**: Comprehensive logging for compliance requirements
- **Policies**: Organization-wide rules that can be centrally managed

## Best Practices

### Security Configuration
1. **Start with strict defaults**: Use `default_action: deny` for allowlists
2. **Principle of least privilege**: Grant minimal required permissions
3. **Regular audits**: Review logs and violations regularly
4. **Test configurations**: Use CLI testing before deploying changes

### Monitoring and Alerting  
1. **Monitor violations**: Set up alerts for security violations
2. **Track patterns**: Look for unusual access patterns in audit logs
3. **Regular reviews**: Periodically review roles and permissions
4. **Incident response**: Have procedures for security violations

### Compliance
1. **Data retention**: Configure appropriate retention periods
2. **Sensitive data**: Enable data masking for privacy
3. **Documentation**: Keep security configurations documented
4. **Change management**: Audit configuration changes

## Advanced Configuration Examples

### High-Security Environment
```yaml
security:
  enabled: true
  allowlist:
    enabled: true
    default_action: deny
    global_rules:
      - name: "block_system_files"
        pattern:
          type: regex  
          value: ".*(etc|sys|proc)/.*"
        action: deny
        priority: 100
  rbac:
    enabled: true
    inherit_permissions: false  # Explicit permissions only
  audit:
    enabled: true
    events: [all]
    include_bodies: true
    mask_sensitive_data: true
  sanitization:
    enabled: true
    default_action: require_approval
  policies:
    enabled: true
    default_action: deny
```

### Development Environment
```yaml
security:
  enabled: true
  allowlist:
    enabled: true
    default_action: allow
    tools:
      "execute_command":
        action: allow
        parameter_rules:
          blocked_patterns: ["rm -rf /", "sudo rm"]
  rbac:
    enabled: true
    default_roles: ["developer"]
  audit:
    enabled: true
    events: [security_violation, error]
    retention_days: 30
```

## Troubleshooting

### Common Issues

**Tool Access Denied**
```bash
# Check allowlist rules
./target/release/magictunnel-security test --tool TOOL_NAME --user USER

# Verify user roles
./target/release/magictunnel-security rbac check-user --user USER --permission PERMISSION
```

**Audit Logs Not Appearing**
- Check storage configuration and permissions
- Verify audit events are enabled for the operation type
- Check disk space for file storage

**Performance Impact**
- Reduce audit detail level if needed
- Use external storage for high-volume environments
- Optimize allowlist patterns for better performance

For more help, see the [troubleshooting guide](troubleshooting.md) or check audit logs for detailed error information.