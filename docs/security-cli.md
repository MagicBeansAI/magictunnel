# MagicTunnel Security CLI Reference

The `magictunnel-security` CLI tool provides comprehensive security management for MagicTunnel. This reference covers all commands, options, and usage examples.

## Installation & Setup

The security CLI is built as part of the main MagicTunnel build:

```bash
# Build the security CLI
cargo build --release --bin magictunnel-security

# The binary will be available at:
./target/release/magictunnel-security
```

## Global Options

All commands support these global options:

```bash
magictunnel-security [OPTIONS] <COMMAND>

OPTIONS:
  -c, --config <PATH>    Configuration file path [default: config.yaml]
  -h, --help            Print help information
  -V, --version         Print version information
```

## Commands Overview

- [`status`](#status) - Show security status and configuration
- [`test`](#test) - Test security configuration for specific scenarios
- [`rbac`](#rbac) - Manage roles and permissions
- [`audit`](#audit) - View and search audit logs
- [`init`](#init) - Initialize security configuration

---

## `status`

Display the current security configuration status and feature availability.

### Usage
```bash
magictunnel-security status
```

### Example Output
```
🔒 MagicTunnel Security Status
=============================
Security: ✅ ENABLED
Tool Allowlisting: ✅ ENABLED
  - Default Action: Deny
  - Tool Rules: 15
  - Resource Rules: 8
  - Global Rules: 3
Request Sanitization: ✅ ENABLED
  - Policies: 4
  - Default Action: Sanitize
RBAC: ✅ ENABLED
  - Roles: 5
  - User Assignments: 12
  - API Key Assignments: 3
  - Default Roles: ["user"]
Organization Policies: ✅ ENABLED
  - Policies: 7
  - Default Action: Allow
Audit Logging: ✅ ENABLED
  - Events: [All]
  - Storage: File
  - Retention: 365 days
```

### Use Cases
- Quick health check of security configuration
- Verifying security features are properly enabled
- Getting overview of security rules and assignments

---

## `test`

Test security configuration against specific scenarios to verify allowlist and access control rules.

### Usage
```bash
magictunnel-security test [OPTIONS]

OPTIONS:
  -t, --tool <TOOL>           Tool name to test [required]
  -u, --user <USER>           User ID for testing
  -r, --roles <ROLES>         User roles (comma-separated)
  -p, --parameters <JSON>     Tool parameters as JSON string
```

### Examples

#### Test Tool Access
```bash
# Test if user can access a tool
magictunnel-security test \
  --tool execute_command \
  --user john_doe \
  --roles developer,user

# Test with specific parameters
magictunnel-security test \
  --tool read_file \
  --user alice \
  --roles admin \
  --parameters '{"path": "/etc/passwd"}'

# Test dangerous command
magictunnel-security test \
  --tool execute_command \
  --user bob \
  --roles user \
  --parameters '{"command": "rm -rf /"}'
```

### Example Output
```
🧪 Testing Security for Tool: execute_command
=====================================
Testing individual security components...

🔍 Testing Tool Allowlist:
  - Allowed: ❌ NO
  - Action: Deny
  - Reason: Tool blocked by global rule 'dangerous_commands'
  - Matched Rule: block_dangerous_patterns

🔍 Testing RBAC:
  - Permission Granted: ✅ YES
  - Reason: User has required permission 'tool:execute'
  - Granting Roles: ["developer"]

✅ Security test completed
```

### Use Cases
- Validate security configuration before deployment
- Debug access issues for specific users
- Test policy changes in development
- Verify security rules work as expected

---

## `rbac`

Manage roles, permissions, and user assignments.

### Subcommands
- [`list-roles`](#rbac-list-roles) - List all available roles
- [`show-role`](#rbac-show-role) - Show detailed role information
- [`add-role`](#rbac-add-role) - Add a new role (config file editing required)
- [`assign-user`](#rbac-assign-user) - Assign role to user (config file editing required)
- [`remove-user`](#rbac-remove-user) - Remove role from user (config file editing required)
- [`check-user`](#rbac-check-user) - Check user permissions

### `rbac list-roles`

List all configured roles with basic information.

#### Usage
```bash
magictunnel-security rbac list-roles
```

#### Example Output
```
👥 RBAC Roles
============
  - admin: Full system administrator
    Permissions: ["*"]
    Parents: []
    Active: true

  - developer: Software developer with tool access
    Permissions: ["tool:execute", "file:read", "file:write"]
    Parents: ["user"]
    Active: true

  - user: Basic user with read permissions
    Permissions: ["tool:read", "resource:read"]
    Parents: []
    Active: true
```

### `rbac show-role`

Show detailed information about a specific role.

#### Usage
```bash
magictunnel-security rbac show-role --name <ROLE_NAME>
```

#### Examples
```bash
# Show admin role details
magictunnel-security rbac show-role --name admin

# Show developer role details
magictunnel-security rbac show-role --name developer
```

#### Example Output
```
👤 Role: developer
===========
Description: Software developer with tool access
Permissions: ["tool:execute", "file:read", "file:write", "resource:read"]
Parent Roles: ["user"]
Active: true
Created: 2024-01-15T10:30:00Z
Modified: 2024-01-20T14:45:00Z
```

### `rbac check-user`

Check what permissions a user has and which roles grant them.

#### Usage
```bash
magictunnel-security rbac check-user --user <USER> --permission <PERMISSION>
```

#### Examples
```bash
# Check if user can execute tools
magictunnel-security rbac check-user \
  --user john_doe \
  --permission "tool:execute"

# Check file read permissions
magictunnel-security rbac check-user \
  --user alice \
  --permission "file:read"
```

#### Example Output
```
🔍 Permission Check: tool:execute for user john_doe
==================================
Granted: ✅ YES
Reason: Permission granted through role inheritance
Granting Roles: ["developer", "user"]
```

### Role Management Notes

Currently, role creation and user assignment require direct configuration file editing. The CLI provides read-only access for security. Future versions will include write operations.

---

## `audit`

View and search audit logs for security monitoring and compliance.

### Subcommands
- [`recent`](#audit-recent) - Show recent audit entries
- [`search`](#audit-search) - Search audit logs with filters
- [`violations`](#audit-violations) - Show security violations only

### `audit recent`

Show the most recent audit log entries.

#### Usage
```bash
magictunnel-security audit recent [OPTIONS]

OPTIONS:
  -n, --count <COUNT>    Number of entries to show [default: 10]
```

#### Examples
```bash
# Show last 10 entries
magictunnel-security audit recent

# Show last 50 entries
magictunnel-security audit recent --count 50
```

#### Example Output
```
📋 Recent Audit Entries (10)
========================
1. 2024-01-20 14:30:15 - Tool Execution - execute_command success
   User: "john_doe"
   Outcome: Success

2. 2024-01-20 14:29:45 - Security Violation - Allowlist blocked
   User: "bob"
   Outcome: Blocked
   Error: Tool 'dangerous_command' blocked by allowlist

3. 2024-01-20 14:28:30 - Authentication - API key validation
   User: None
   Outcome: Success
```

### `audit search`

Search audit logs with specific filters.

#### Usage
```bash
magictunnel-security audit search [OPTIONS]

OPTIONS:
  -u, --user <USER>         Filter by user ID
  -t, --tool <TOOL>         Filter by tool name
      --hours <HOURS>       Hours to look back [default: 24]
```

#### Examples
```bash
# Search for specific user activity
magictunnel-security audit search --user john_doe --hours 48

# Search for specific tool usage
magictunnel-security audit search --tool execute_command --hours 12

# Search for user activity with specific tool
magictunnel-security audit search \
  --user alice \
  --tool read_file \
  --hours 72
```

#### Example Output
```
🔍 Audit Search Results (last 24 hours)
===============================
Found 5 entries

1. 2024-01-20 14:30:15 - Tool Execution - execute_command
2. 2024-01-20 13:45:22 - Resource Access - file:///data/config.yaml
3. 2024-01-20 12:15:30 - Authorization - Permission check passed
4. 2024-01-20 11:30:45 - Tool Execution - read_file
5. 2024-01-20 10:22:15 - Authentication - User login
```

### `audit violations`

Show only security violations for incident response.

#### Usage
```bash
magictunnel-security audit violations [OPTIONS]

OPTIONS:
      --hours <HOURS>    Hours to look back [default: 24]
```

#### Examples
```bash
# Show violations in last 24 hours
magictunnel-security audit violations

# Show violations in last week
magictunnel-security audit violations --hours 168
```

#### Example Output
```
🚨 Security Violations (last 24 hours)
==============================
Found 3 violations

1. 2024-01-20 14:29:45 - Allowlist violation
   🚫 Tool 'rm_command' blocked by allowlist policy
   👤 User: "bob" (Roles: ["user"])

2. 2024-01-20 13:15:30 - RBAC violation
   🚫 Insufficient permissions for 'admin:config'
   👤 User: "alice" (Roles: ["developer"])

3. 2024-01-20 11:45:15 - Sanitization violation
   🚫 Secret detected in tool parameters
   👤 User: "john_doe" (Roles: ["developer", "user"])
```

---

## `init`

Initialize security configuration files with predefined security levels.

### Usage
```bash
magictunnel-security init [OPTIONS]

OPTIONS:
  -o, --output <PATH>     Output file path [default: security-config.yaml]
  -l, --level <LEVEL>     Security level: basic, standard, strict [default: standard]
```

### Security Levels

#### Basic
- Allowlisting enabled with permissive defaults
- Basic RBAC with simple roles
- Audit logging for violations only
- Minimal sanitization

#### Standard (Default)
- Allowlisting with deny-by-default
- Comprehensive RBAC with role inheritance
- Full audit logging
- Content sanitization enabled
- Tool allowlisting with pattern support

#### Strict
- Maximum security settings
- Deny-by-default for all features
- Comprehensive audit logging with body content
- Aggressive sanitization and approval workflows
- Advanced allowlisting with strict pattern matching

### Examples

```bash
# Generate basic security config
magictunnel-security init \
  --level basic \
  --output security-basic.yaml

# Generate strict security config
magictunnel-security init \
  --level strict \
  --output security-strict.yaml

# Generate standard config (default)
magictunnel-security init
```

### Example Output
```
🔧 Initializing Security Configuration
======================================
✅ Security configuration written to: security-config.yaml
📝 Review and customize the configuration, then add to your main config.yaml:
   security:
     # Copy the generated configuration here
```

---

## Common Workflows

### Daily Security Monitoring

```bash
# Check overall security status
magictunnel-security status

# Review recent violations
magictunnel-security audit violations --hours 24

# Check specific user activity if needed
magictunnel-security audit search --user suspicious_user --hours 48
```

### New User Setup

```bash
# Check what roles are available
magictunnel-security rbac list-roles

# Test access for the new user
magictunnel-security test \
  --tool common_tool \
  --user new_user \
  --roles user

# Verify permissions
magictunnel-security rbac check-user \
  --user new_user \
  --permission "tool:read"
```

### Policy Testing

```bash
# Test policy changes before deployment
magictunnel-security test \
  --tool execute_command \
  --user test_user \
  --roles developer \
  --parameters '{"command": "ls -la"}'

# Test edge cases
magictunnel-security test \
  --tool dangerous_tool \
  --user admin_user \
  --roles admin \
  --parameters '{"action": "delete_all"}'
```

### Incident Response

```bash
# Check recent violations
magictunnel-security audit violations --hours 4

# Search for specific user activity
magictunnel-security audit search \
  --user compromised_user \
  --hours 72

# Verify current security status
magictunnel-security status
```

### Compliance Reporting

```bash
# Generate recent activity report
magictunnel-security audit recent --count 100 > security-report.txt

# Search for specific compliance events
magictunnel-security audit search \
  --tool sensitive_data_access \
  --hours 720  # 30 days
```

## Integration Examples

### CI/CD Pipeline

```bash
#!/bin/bash
# Security validation script for CI/CD

echo "Validating security configuration..."
magictunnel-security status

if [ $? -ne 0 ]; then
    echo "Security configuration validation failed"
    exit 1
fi

echo "Testing critical tool access..."
magictunnel-security test \
  --tool execute_command \
  --user ci_user \
  --roles automation

echo "Security validation completed successfully"
```

### Monitoring Script

```bash
#!/bin/bash
# Daily security monitoring

VIOLATIONS=$(magictunnel-security audit violations --hours 24 2>/dev/null | grep "Found" | awk '{print $2}')

if [ "$VIOLATIONS" -gt 0 ]; then
    echo "ALERT: $VIOLATIONS security violations in the last 24 hours"
    magictunnel-security audit violations --hours 24
    # Send alert to security team
fi
```

### Health Check

```bash
#!/bin/bash
# Security health check for monitoring systems

magictunnel-security status --config /etc/magictunnel/config.yaml
STATUS=$?

if [ $STATUS -eq 0 ]; then
    echo "OK: Security system operational"
    exit 0
else
    echo "CRITICAL: Security system issues detected"
    exit 2
fi
```

## Troubleshooting

### Common Issues

**Command not found**
```bash
# Ensure the binary is built and in PATH
cargo build --release --bin magictunnel-security
export PATH=$PATH:/path/to/target/release
```

**Configuration file not found**
```bash
# Specify config file explicitly
magictunnel-security --config /path/to/config.yaml status
```

**Permission denied**
```bash
# Check file permissions and run with appropriate user
ls -la target/release/magictunnel-security
chmod +x target/release/magictunnel-security
```

**No audit logs visible**
```bash
# Check audit configuration and storage location
magictunnel-security status
# Verify audit storage directory exists and is writable
```

### Debug Mode

For detailed troubleshooting, enable debug logging:

```bash
RUST_LOG=debug magictunnel-security status
```

---

## Security Considerations

- **CLI Access**: Limit access to the security CLI to authorized administrators
- **Configuration Files**: Protect configuration files with appropriate file permissions
- **Audit Logs**: Ensure audit log storage is secure and tamper-evident
- **Regular Reviews**: Regularly review security configurations and audit logs
- **Backup**: Keep backups of security configurations

## Related Documentation

- [Security Overview](security.md) - Complete security system documentation
- [Configuration Guide](config.md) - General configuration options
- [Deployment Guide](deploy.md) - Security considerations for production
- [API Reference](api.md) - Programmatic security management

For additional help or questions, check the audit logs for detailed error information or consult the main security documentation.