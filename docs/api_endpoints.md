# MagicTunnel API Endpoints Reference

## Overview

MagicTunnel provides a comprehensive REST API for managing tools, resources, security, and system operations. The API is organized into logical groups with consistent HTTP methods and response formats.

**Base URL**: `http://localhost:3001` (default)
**API Version**: v0.3.20
**Content Type**: `application/json`

---

## Core MCP Protocol Endpoints

### Health Check
- **GET** `/health` - System health status

### MCP JSON-RPC 2.0
- **POST** `/mcp/jsonrpc` - Unified MCP protocol endpoint

### Tool Management
- **GET** `/mcp/tools` - List all available tools
- **POST** `/mcp/call` - Execute a tool call

### Streaming & Transport
- **GET** `/mcp/ws` - WebSocket connection for real-time communication
- **GET** `/mcp/sse` - Server-Sent Events endpoint (deprecated)
- **POST** `/mcp/sse/messages` - SSE messages endpoint (deprecated)
- **POST** `/mcp/call/stream` - Streaming tool execution
- **POST** `/mcp/streamable` - **MCP 2025-06-18 Streamable HTTP Transport (preferred)**
- **GET** `/mcp/streamable` - **MCP 2025-06-18 Streamable HTTP Transport (preferred)**

### Resource Management
- **GET** `/mcp/resources` - List available resources
- **POST** `/mcp/resources/read` - Read resource content

### Prompt Management
- **GET** `/mcp/prompts` - List prompt templates
- **POST** `/mcp/prompts/get` - Execute prompt template

### Logging
- **POST** `/mcp/logging/setLevel` - Set log level

---

## Authentication Endpoints

### OAuth 2.1 Authentication
- **GET** `/auth/oauth/authorize` - OAuth authorization endpoint
- **GET** `/auth/oauth/callback` - OAuth callback handler
- **POST** `/auth/oauth/token` - OAuth token exchange
- **GET** `/auth/callback/{server_name}` - Server-specific OAuth discovery callbacks

---

## Dashboard API Endpoints

All dashboard endpoints are prefixed with `/dashboard/api`

### System Status & Information
- **GET** `/dashboard/api/status` - System status overview
- **GET** `/dashboard/api/mode` - Runtime mode information
- **GET** `/dashboard/api/config` - System configuration
- **GET** `/dashboard/api/services` - Services status
- **GET** `/dashboard/api/services/status` - Detailed services status
- **GET** `/dashboard/api/system/services/status` - System services status
- **GET** `/dashboard/api/system/status` - Extended system status

### Tool Management
- **GET** `/dashboard/api/tools` - Tools catalog
- **GET** `/dashboard/api/capabilities` - Capabilities catalog
- **POST** `/dashboard/api/tools/{name}/execute` - Execute specific tool
- **GET** `/dashboard/api/tools/management/list` - Tool management list
- **GET** `/dashboard/api/tools/management/statistics` - Tool management statistics
- **POST** `/dashboard/api/tools/management/refresh` - Refresh tool management cache
- **POST** `/dashboard/api/tools/management/quick-action` - Quick tool actions
- **PUT** `/dashboard/api/tools/management/bulk` - Bulk tool updates
- **GET** `/dashboard/api/tools/management/{name}` - Get tool management state
- **PUT** `/dashboard/api/tools/management/{name}` - Update tool management state

### Service Management
- **POST** `/dashboard/api/services/{name}/restart` - Restart service
- **POST** `/dashboard/api/services/{name}/stop` - Stop service
- **POST** `/dashboard/api/services/{name}/start` - Start service

### Configuration Management
- **GET** `/dashboard/api/config/templates` - Configuration templates
- **POST** `/dashboard/api/config/validate` - Validate configuration
- **POST** `/dashboard/api/config/backup` - Create configuration backup
- **GET** `/dashboard/api/config/backups` - List configuration backups
- **POST** `/dashboard/api/config/restore` - Restore configuration from backup
- **POST** `/dashboard/api/config/save` - Save configuration

### Makefile Operations
- **GET** `/dashboard/api/makefile` - List Makefile commands
- **POST** `/dashboard/api/makefile/execute` - Execute Makefile command

### MCP Command Execution
- **POST** `/dashboard/api/mcp/execute` - Execute MCP command
- **POST** `/dashboard/api/mcp/execute/stdio` - Execute MCP command via stdio

### MCP Server Management
- **GET** `/dashboard/api/mcp-servers` - List MCP servers
- **GET** `/dashboard/api/mcp-servers/status` - MCP servers status summary
- **GET** `/dashboard/api/mcp-servers/{server_name}` - MCP server details
- **POST** `/dashboard/api/mcp-servers/{server_name}/oauth/initiate` - Initiate OAuth flow
- **GET** `/dashboard/api/mcp-servers/{server_name}/oauth/url` - Get OAuth URL

### System Control
- **POST** `/dashboard/api/system/restart` - Restart MagicTunnel
- **POST** `/dashboard/api/system/switch-mode` - Switch runtime mode
- **POST** `/dashboard/api/system/custom-restart` - Custom restart with options
- **POST** `/dashboard/api/system/execute-command` - Execute custom command

### Logging
- **GET** `/dashboard/api/logs` - Retrieve system logs (with query parameters)

### Environment Variables
- **GET** `/dashboard/api/env` - Get environment variables
- **POST** `/dashboard/api/env` - Set environment variables
- **DELETE** `/dashboard/api/env` - Delete environment variables

### Resource Management
- **GET** `/dashboard/api/resources` - List resources (with query parameters)
- **POST** `/dashboard/api/resources/read` - Read resource content
- **GET** `/dashboard/api/resources/management/status` - Resource management status
- **GET** `/dashboard/api/resources/management/resources` - Resource management listing
- **GET** `/dashboard/api/resources/management/resources/{uri:.*}` - Resource details
- **POST** `/dashboard/api/resources/management/resources/{uri:.*}/read` - Read resource with options
- **GET** `/dashboard/api/resources/management/providers` - Resource providers
- **POST** `/dashboard/api/resources/management/validate` - Validate resources
- **GET** `/dashboard/api/resources/management/statistics` - Resource statistics

### Prompt Management
- **GET** `/dashboard/api/prompts` - List prompts (with query parameters)
- **POST** `/dashboard/api/prompts/execute` - Execute prompt
- **GET** `/dashboard/api/prompts/management/status` - Prompt management status
- **GET** `/dashboard/api/prompts/management/templates` - Prompt templates management
- **POST** `/dashboard/api/prompts/management/templates` - Create prompt template

---

## Security API Endpoints

All security endpoints are mounted within the dashboard API scope at `/dashboard/api/security`

### Allowlist Management
- **GET** `/dashboard/api/security/allowlist/status` - Allowlist service status
- **GET** `/dashboard/api/security/allowlist/evaluate/{tool_name}` - Evaluate tool access
- **POST** `/dashboard/api/security/allowlist/evaluate/batch` - Batch evaluate tool access
- **GET** `/dashboard/api/security/allowlist/tree` - Get allowlist hierarchy tree
- **GET** `/dashboard/api/security/allowlist/rules` - List allowlist rules
- **GET** `/dashboard/api/security/allowlist/patterns` - List allowlist patterns
- **POST** `/dashboard/api/security/allowlist/patterns/test` - Test patterns against tools
- **POST** `/dashboard/api/security/allowlist/tool-rule` - Add/update tool rule
- **DELETE** `/dashboard/api/security/allowlist/tool-rule` - Remove tool rule
- **POST** `/dashboard/api/security/allowlist/capability-rule` - Add/update capability rule
- **DELETE** `/dashboard/api/security/allowlist/capability-rule` - Remove capability rule
- **GET** `/dashboard/api/security/allowlist/statistics` - Allowlist statistics
- **POST** `/dashboard/api/security/allowlist/emergency/lockdown` - Emergency lockdown
- **POST** `/dashboard/api/security/allowlist/emergency/unlock` - Emergency unlock
- **GET** `/dashboard/api/security/allowlist/emergency/status` - Emergency status

### RBAC (Role-Based Access Control)
- **GET** `/dashboard/api/security/rbac/status` - RBAC service status
- **GET** `/dashboard/api/security/rbac/roles` - List roles
- **POST** `/dashboard/api/security/rbac/roles` - Create role
- **GET** `/dashboard/api/security/rbac/roles/{role_id}` - Get role details
- **PUT** `/dashboard/api/security/rbac/roles/{role_id}` - Update role
- **DELETE** `/dashboard/api/security/rbac/roles/{role_id}` - Delete role
- **GET** `/dashboard/api/security/rbac/permissions` - List permissions
- **POST** `/dashboard/api/security/rbac/assign` - Assign role to user
- **DELETE** `/dashboard/api/security/rbac/revoke` - Revoke role from user

### Audit Logging
- **GET** `/dashboard/api/security/audit/status` - Audit service status
- **GET** `/dashboard/api/security/audit/events` - List audit events (with filters)
- **GET** `/dashboard/api/security/audit/events/{event_id}` - Get audit event details
- **GET** `/dashboard/api/security/audit/statistics` - Audit statistics
- **POST** `/dashboard/api/security/audit/export` - Export audit logs

### Sanitization
- **GET** `/dashboard/api/security/sanitization/status` - Sanitization service status
- **POST** `/dashboard/api/security/sanitization/test` - Test sanitization rules
- **GET** `/dashboard/api/security/sanitization/rules` - List sanitization rules
- **POST** `/dashboard/api/security/sanitization/rules` - Create sanitization rule
- **PUT** `/dashboard/api/security/sanitization/rules/{rule_id}` - Update sanitization rule
- **DELETE** `/dashboard/api/security/sanitization/rules/{rule_id}` - Delete sanitization rule

### Policy Management
- **GET** `/dashboard/api/security/policies/status` - Policy service status
- **GET** `/dashboard/api/security/policies` - List security policies
- **POST** `/dashboard/api/security/policies` - Create security policy
- **GET** `/dashboard/api/security/policies/{policy_id}` - Get policy details
- **PUT** `/dashboard/api/security/policies/{policy_id}` - Update policy
- **DELETE** `/dashboard/api/security/policies/{policy_id}` - Delete policy
- **POST** `/dashboard/api/security/policies/{policy_id}/activate` - Activate policy
- **POST** `/dashboard/api/security/policies/{policy_id}/deactivate` - Deactivate policy

### Configuration Change Tracking
- **GET** `/dashboard/api/security/changes/status` - Change tracker status
- **GET** `/dashboard/api/security/changes` - List configuration changes
- **GET** `/dashboard/api/security/changes/{change_id}` - Get change details
- **POST** `/dashboard/api/security/changes/{change_id}/approve` - Approve change
- **POST** `/dashboard/api/security/changes/{change_id}/reject` - Reject change

### Security Overview
- **GET** `/dashboard/api/security/status` - Overall security status
- **GET** `/dashboard/api/security/alerts` - Security alerts
- **GET** `/dashboard/api/security/metrics` - Security metrics
- **GET** `/dashboard/api/security/violations` - Security violations (with search/filter support)
- **GET** `/dashboard/api/security/violations/statistics` - Violation statistics

---

## Roots API Endpoints

All roots endpoints are configured when the roots service is available

### Root Management
- **GET** `/api/roots` - List filesystem/URI roots
- **POST** `/api/roots` - Add manual root
- **GET** `/api/roots/status` - Roots service status
- **GET** `/api/roots/discovery` - Root discovery status
- **POST** `/api/roots/discovery/refresh` - Refresh root discovery
- **PUT** `/api/roots/config` - Update roots configuration
- **DELETE** `/api/roots/{root_id}` - Remove root
- **GET** `/api/roots/{root_id}/permissions` - Get root permissions
- **PUT** `/api/roots/{root_id}/permissions` - Update root permissions

---

## Mode API Endpoints

Runtime mode detection and management

- **GET** `/api/mode` - Get current runtime mode and available features

---

## Common Request/Response Patterns

### Request Headers
```
Content-Type: application/json
Accept: application/json
```

### Error Response Format
```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable error message",
    "details": {}
  },
  "timestamp": "2025-01-21T10:30:00Z"
}
```

### Success Response Format
```json
{
  "data": {},
  "status": "success",
  "timestamp": "2025-01-21T10:30:00Z"
}
```

### Pagination (where applicable)
```json
{
  "data": [],
  "pagination": {
    "page": 1,
    "per_page": 50,
    "total": 150,
    "total_pages": 3
  }
}
```

### Query Parameters

**Common query parameters across endpoints:**
- `page` - Page number for pagination
- `per_page` - Items per page (default: 50)
- `filter` - Filter criteria
- `sort` - Sort field
- `order` - Sort order (asc/desc)

---

## Authentication

### API Key Authentication
Include API key in request headers:
```
Authorization: Bearer <api_key>
```

### OAuth 2.1 Authentication
1. Redirect to `/auth/oauth/authorize`
2. Handle callback at `/auth/oauth/callback`
3. Exchange code for token at `/auth/oauth/token`
4. Include token in subsequent requests

---

## Rate Limiting

- Default: 1000 requests per minute per IP
- Headers included in responses:
  - `X-RateLimit-Limit`
  - `X-RateLimit-Remaining`
  - `X-RateLimit-Reset`

---

## Transport Options

### HTTP/HTTPS
Standard REST API over HTTP/HTTPS

### WebSocket
Real-time bidirectional communication at `/mcp/ws`

### Server-Sent Events (Deprecated)
Unidirectional streaming at `/mcp/sse` (use Streamable HTTP instead)

### MCP 2025-06-18 Streamable HTTP (Recommended)
Enhanced streaming transport at `/mcp/streamable`

---

## Special Features

### Smart Tool Discovery
MagicTunnel provides intelligent tool routing through a single `smart_tool_discovery` tool that can:
- Analyze natural language requests
- Find the best tool using hybrid AI (semantic + rule-based + LLM)
- Map parameters with validation
- Proxy calls to actual tools
- Return results with metadata

### Multi-Mode Architecture
- **Proxy Mode**: Core MCP functionality only
- **Advanced Mode**: Full enterprise features including security suite

### Security Integration
All tool calls can be secured through:
- Allowlist evaluation (tool/capability patterns)
- RBAC authorization
- Request sanitization
- Audit logging
- Emergency lockdown

---

## Examples

### Execute a Tool
```bash
curl -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "name": "smart_tool_discovery",
    "arguments": {
      "request": "ping google.com"
    }
  }'
```

### List Available Tools
```bash
curl -X GET http://localhost:3001/dashboard/api/tools
```

### Check System Status
```bash
curl -X GET http://localhost:3001/dashboard/api/status
```

### Get Security Status
```bash
curl -X GET http://localhost:3001/dashboard/api/security/status
```

---

## Version History

- **v0.3.20**: Security violations fixes, comprehensive implementation analysis
- **v0.3.19**: Pattern management fixes, UI enhancements
- **v0.3.17**: Nested security validation, OAuth 2.1 production ready
- **v0.3.16**: Multi-mode architecture, security framework
- **v0.3.10**: Dashboard API expansion
- **v0.2.x**: Core MCP protocol implementation

---

For more information, see the [MagicTunnel Documentation](../README.md) and [Configuration Guide](../CLAUDE.md).